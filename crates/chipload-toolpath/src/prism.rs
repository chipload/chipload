// SPDX-License-Identifier: GPL-3.0-or-later
//! Prism (docs/specs/prism.md): closed shapes left standing as roofs, a ridge along their
//! centre line. The V-bit's tip runs along each outline, its centre half the tip into the
//! waste, as deep as the largest circle inside the shape touching the outline there allows
//! (its radius over the tangent of the half angle), so the inner flank reaches the surface
//! on the centre line.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_core::primitives::{distance, signed_area};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, OFFSET_CHECK_SLACK_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{PrismOp, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::{ArcTable, Circle};
use crate::cavalier::{Area, encloses};
use crate::medial::Outline;
use crate::model::Move;
use crate::offset::offset_rings_tagged;
use crate::pocket::build_region;
use crate::profile::{contour_context, cut_clockwise};

/// Halvings when finding the largest touching circle: far below the chaining tolerance.
const RADIUS_STEPS: usize = 48;

/// Deepest recursion when adding points along a ring edge.
const MAX_SPLITS: u32 = 12;

/// Points along the outward reach checked for another shape in the trench.
const REACH_SAMPLES: usize = 8;

/// The V-bit as the prism needs it.
#[derive(Debug, Clone, Copy)]
pub struct PrismBit {
    /// tan of the half angle.
    pub tan: f64,
    /// The flat tip's diameter, 0 for a pointed bit.
    pub tip: f64,
    /// The largest touching circle the cone reaches, mm: half the diameter less the tip.
    pub reach: f64,
}

/// One ring: the tip's centre points (closed, no repeated start), each with its depth below
/// the start depth.
#[derive(Debug, Clone, PartialEq)]
pub struct PrismRing {
    pub contour_id: usize,
    pub points: Vec<Point>,
    pub depths: Vec<f64>,
}

/// The rings of a prism operation, its notes, and its deepest edge.
#[derive(Debug, Clone, Default)]
pub struct PrismRings {
    pub rings: Vec<PrismRing>,
    pub arcs: ArcTable,
    pub notes: Vec<Message>,
    pub deepest: f64,
}

/// One region, for the depths and the trench check.
struct Region {
    area: Area,
    outline: Outline,
    first_id: usize,
}

/// The rings for `contours` (the selected shapes inside another are its holes).
pub fn prism_rings(
    op: &PrismOp,
    label: &Message,
    contours: &[&Contour],
    bit: PrismBit,
    spindle: Spindle,
) -> Result<PrismRings, ToolpathError> {
    let mut out = PrismRings::default();
    let half_tip = bit.tip.max(0.0) / 2.0;
    // The largest circle the depth allows: the cone, or the max depth.
    let cap_by_depth = op.max_depth.map(|d| d * bit.tan);
    let cap = cap_by_depth.map_or(bit.reach, |c| c.min(bit.reach));
    let pieces = build_region(contours)?;
    let regions: Vec<Region> = pieces
        .iter()
        .map(|p| Region {
            outline: Outline::new(&p.area),
            area: p.area.clone(),
            first_id: p.contour_ids.first().copied().unwrap_or(0),
        })
        .collect();
    let by_id = |id: usize| contours.iter().find(|c| c.id == id).copied();
    let mut close: Vec<(usize, usize, f64)> = Vec::new();
    for (k, piece) in pieces.iter().enumerate() {
        let region = &regions[k];
        let mut flat_top = false;
        for (n, &id) in piece.contour_ids.iter().enumerate() {
            let Some(contour) = by_id(id) else {
                continue;
            };
            // The boundary's waste is outside it, a hole's inside it.
            let side = if n == 0 { Side::Outside } else { Side::Inside };
            let found = if half_tip > CHAIN_TOL_MM {
                offset_rings_tagged(contour, side, half_tip, &mut out.arcs)
            } else {
                offset_rings_tagged(contour, Side::On, 0.0, &mut out.arcs)
            }
            .map_err(|err| err.with_context(contour_context(label, id)))?;
            for ring in found {
                let ring = ccw(&ring);
                if ring.len() < 3 {
                    continue;
                }
                // Into the region: left of a counter-clockwise outline, right of a hole.
                let sign = if n == 0 { 1.0 } else { -1.0 };
                let probe = Probe {
                    region,
                    half_tip,
                    cap,
                    bit,
                };
                let (points, depths, capped) = along(&ring, sign, &probe, &mut out.arcs);
                flat_top |= capped && cap_by_depth.is_none_or(|c| c > bit.reach);
                for (i, (&o, &h)) in points.iter().zip(&depths).enumerate() {
                    let normal = normal_at(&points, i, sign, &out.arcs);
                    if let Some((j, deeper)) = trench_hit(o, normal, h, &probe, &regions) {
                        record(&mut close, id, regions[j].first_id, deeper);
                    }
                }
                let clockwise = cut_clockwise(side, op.direction, spindle);
                let (points, depths) = turned(points, depths, clockwise);
                out.deepest = depths.iter().copied().fold(out.deepest, f64::max);
                out.rings.push(PrismRing {
                    contour_id: id,
                    points,
                    depths,
                });
            }
        }
        if flat_top {
            out.notes.push(
                Message::new("toolpath-prism-flat-top")
                    .arg("contour", region.first_id)
                    .arg("deepest", Value::Length(cap / bit.tan, Digits::Trim(2))),
            );
        }
    }
    for (a, b, deeper) in close {
        out.notes.push(
            Message::new("toolpath-prism-close")
                .arg("contour", a)
                .arg("other", b)
                .arg("deeper", Value::Length(deeper, Digits::Trim(2))),
        );
    }
    Ok(out)
}

/// Keep the deepest trench cut for each pair of shapes.
fn record(close: &mut Vec<(usize, usize, f64)>, a: usize, b: usize, deeper: f64) {
    if let Some(found) = close.iter_mut().find(|(x, y, _)| *x == a && *y == b) {
        found.2 = found.2.max(deeper);
    } else {
        close.push((a, b, deeper));
    }
}

/// The ring counter-clockwise.
fn ccw(ring: &[Point]) -> Vec<Point> {
    let area: f64 = (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    if area < 0.0 {
        ring.iter().rev().copied().collect()
    } else {
        ring.to_vec()
    }
}

/// What finding a depth needs.
struct Probe<'a> {
    region: &'a Region,
    half_tip: f64,
    cap: f64,
    bit: PrismBit,
}

impl Probe<'_> {
    /// The depth at the tip's centre `o` with the inward `normal`, and whether the cap
    /// limited it.
    fn depth(&self, o: Point, normal: Point) -> (f64, bool) {
        let foot = (
            o.0 + normal.0 * self.half_tip,
            o.1 + normal.1 * self.half_tip,
        );
        let fits = |r: f64| {
            let c = (foot.0 + normal.0 * r, foot.1 + normal.1 * r);
            self.region.outline.distance(c) >= r - OFFSET_CHECK_SLACK_MM
        };
        if fits(self.cap) {
            return (self.cap / self.bit.tan, true);
        }
        let (mut low, mut high) = (0.0, self.cap);
        for _ in 0..RADIUS_STEPS {
            if high - low <= CHAIN_TOL_MM / 16.0 {
                break;
            }
            let middle = (low + high) / 2.0;
            if fits(middle) {
                low = middle;
            } else {
                high = middle;
            }
        }
        (low / self.bit.tan, false)
    }
}

/// The unit normal at point `i` of the closed ring, into the region (`sign` +1: left of the
/// counter-clockwise ring): the bisector of its two edges' normals, an edge on an arc giving
/// the arc's radius at the point (exact, where two chords of unequal length would not be).
fn normal_at(ring: &[Point], i: usize, sign: f64, arcs: &ArcTable) -> Point {
    let n = ring.len();
    let p = ring[i];
    let edge = |a: Point, b: Point| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = math::hypot(dx, dy).max(f64::MIN_POSITIVE);
        let line = (-dy / len * sign, dx / len * sign);
        match arcs.get(a, b) {
            Some(c) => {
                let (rx, ry) = (p.0 - c.center.0, p.1 - c.center.1);
                let d = math::hypot(rx, ry);
                if d <= 0.0 {
                    return line;
                }
                let radial = (rx / d, ry / d);
                if radial.0 * line.0 + radial.1 * line.1 >= 0.0 {
                    radial
                } else {
                    (-radial.0, -radial.1)
                }
            }
            None => line,
        }
    };
    let before = edge(ring[(i + n - 1) % n], p);
    let after = edge(p, ring[(i + 1) % n]);
    let (x, y) = (before.0 + after.0, before.1 + after.1);
    let len = math::hypot(x, y);
    if len <= 0.0 {
        after
    } else {
        (x / len, y / len)
    }
}

/// The ring's points with their depths, points added along each edge until the depth
/// between two is within the arc tolerance of the depth there; whether any depth was capped.
fn along(
    ring: &[Point],
    sign: f64,
    probe: &Probe,
    arcs: &mut ArcTable,
) -> (Vec<Point>, Vec<f64>, bool) {
    let n = ring.len();
    let mut capped = false;
    let at_vertex: Vec<f64> = (0..n)
        .map(|i| {
            let (h, c) = probe.depth(ring[i], normal_at(ring, i, sign, arcs));
            capped |= c;
            h
        })
        .collect();
    let mut points = Vec::with_capacity(n);
    let mut depths = Vec::with_capacity(n);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let circle = arcs.get(a, b);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = math::hypot(dx, dy).max(f64::MIN_POSITIVE);
        let normal = (-dy / len * sign, dx / len * sign);
        points.push(a);
        depths.push(at_vertex[i]);
        let mut inner: Vec<(Point, f64)> = Vec::new();
        split(
            (a, at_vertex[i]),
            (b, at_vertex[(i + 1) % n]),
            normal,
            circle,
            probe,
            MAX_SPLITS,
            &mut inner,
            &mut capped,
        );
        if let Some(c) = circle
            && !inner.is_empty()
        {
            let mut from = a;
            for &(p, _) in &inner {
                arcs.insert(from, p, c);
                from = p;
            }
            arcs.insert(from, b, c);
        }
        for (p, h) in inner {
            points.push(p);
            depths.push(h);
        }
    }
    let (points, depths) = simplified(points, depths, arcs);
    (points, depths, capped)
}

/// The ring without the points that add nothing: on a straight line between their
/// neighbours, no arc either side, the depth there what the line between theirs gives.
fn simplified(points: Vec<Point>, depths: Vec<f64>, arcs: &ArcTable) -> (Vec<Point>, Vec<f64>) {
    let mut p: Vec<Point> = Vec::with_capacity(points.len());
    let mut d: Vec<f64> = Vec::with_capacity(depths.len());
    let n = points.len();
    for i in 0..n {
        let (q, h) = (points[i], depths[i]);
        let (Some(&a), Some(&ha)) = (p.last(), d.last()) else {
            p.push(q);
            d.push(h);
            continue;
        };
        let (b, hb) = (points[(i + 1) % n], depths[(i + 1) % n]);
        let straight = arcs.get(a, q).is_none() && arcs.get(q, b).is_none();
        let ab = distance(a, b);
        let off = if ab > 0.0 {
            ((b.0 - a.0) * (q.1 - a.1) - (b.1 - a.1) * (q.0 - a.0)).abs() / ab
        } else {
            f64::INFINITY
        };
        let t = if ab > 0.0 { distance(a, q) / ab } else { 0.0 };
        let even = (h - (ha + (hb - ha) * t)).abs() <= ARC_APPROX_TOL_MM / 4.0;
        if i + 1 < n && straight && off <= CHAIN_TOL_MM && even {
            continue;
        }
        p.push(q);
        d.push(h);
    }
    (p, d)
}

/// Points between `a` and `b` (not included), in order, where the depth strays from the
/// straight line between theirs by more than the arc tolerance.
#[allow(clippy::too_many_arguments)]
fn split(
    a: (Point, f64),
    b: (Point, f64),
    normal: Point,
    circle: Option<Circle>,
    probe: &Probe,
    splits: u32,
    out: &mut Vec<(Point, f64)>,
    capped: &mut bool,
) {
    if splits == 0 || distance(a.0, b.0) <= 4.0 * CHAIN_TOL_MM {
        return;
    }
    let mut m = ((a.0.0 + b.0.0) / 2.0, (a.0.1 + b.0.1) / 2.0);
    let mut n = normal;
    if let Some(c) = circle {
        let (dx, dy) = (m.0 - c.center.0, m.1 - c.center.1);
        let d = math::hypot(dx, dy);
        if d > 0.0 {
            m = (
                c.center.0 + dx * c.radius / d,
                c.center.1 + dy * c.radius / d,
            );
            // Along the radius, the way the chord's normal points.
            let radial = (dx / d, dy / d);
            let towards = radial.0 * normal.0 + radial.1 * normal.1;
            n = if towards >= 0.0 {
                radial
            } else {
                (-radial.0, -radial.1)
            };
        }
    }
    let (h, c) = probe.depth(m, n);
    *capped |= c;
    if (h - (a.1 + b.1) / 2.0).abs() <= ARC_APPROX_TOL_MM {
        return;
    }
    split(a, (m, h), normal, circle, probe, splits - 1, out, capped);
    out.push((m, h));
    split((m, h), b, normal, circle, probe, splits - 1, out, capped);
}

/// Whether the trench round the tip at `o` (depth `h`, into the region along `normal`)
/// reaches into a selected shape: the region it reaches and how far below its edge the
/// cone goes there.
fn trench_hit(
    o: Point,
    normal: Point,
    h: f64,
    probe: &Probe,
    regions: &[Region],
) -> Option<(usize, f64)> {
    let reach = probe.half_tip + h * probe.bit.tan;
    // Start a little out, past the tip's own edge, so the shape itself isn't found.
    let first = probe.half_tip + ARC_APPROX_TOL_MM;
    if reach <= first {
        return None;
    }
    for s in 1..=REACH_SAMPLES {
        let u = first + (reach - first) * s as f64 / REACH_SAMPLES as f64;
        let q = (o.0 - normal.0 * u, o.1 - normal.1 * u);
        for (j, region) in regions.iter().enumerate() {
            let inside = encloses(&region.area.outer, q)
                && !region.area.holes.iter().any(|hole| encloses(hole, q));
            if !inside || region.outline.distance(q) <= ARC_APPROX_TOL_MM {
                continue;
            }
            // The cone's depth there, against the shape's edge depth (at most h).
            let deeper = (h - (u - probe.half_tip) / probe.bit.tan).max(0.0);
            return (deeper > ARC_APPROX_TOL_MM).then_some((j, deeper));
        }
    }
    None
}

/// The ring turned for the cutting direction and started at its shallowest point.
fn turned(points: Vec<Point>, depths: Vec<f64>, clockwise: bool) -> (Vec<Point>, Vec<f64>) {
    let (mut p, mut d) = (points, depths);
    if (signed_area(&p) < 0.0) != clockwise {
        p.reverse();
        d.reverse();
    }
    let start = d
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(i, _)| i);
    p.rotate_left(start);
    d.rotate_left(start);
    (p, d)
}

/// Everything the moves need besides the rings.
#[derive(Debug, Clone, Copy)]
pub struct PrismCut<'a> {
    /// Each pass's depth below the start depth, going down.
    pub levels: &'a [f64],
    /// The Z of the start depth.
    pub top_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
}

/// The moves for one ring: every pass round it, cut off at its level, then a retract.
pub fn prism_moves(ring: &PrismRing, cut: &PrismCut, arcs: &ArcTable) -> (Vec<Move>, Point) {
    let Some(&first) = ring.points.first() else {
        return (Vec::new(), (0.0, 0.0));
    };
    let mut moves = vec![Move::rapid_z(cut.safe_z), Move::rapid_xy(first)];
    let mut current = cut.safe_z;
    let n = ring.points.len();
    for &level in cut.levels {
        let z_at = |i: usize| cut.top_z - ring.depths[i % n].min(level);
        let start = z_at(0);
        if start != current {
            moves.push(Move::feed_z(start, cut.plunge_feed));
            current = start;
        }
        let mut here = first;
        for i in 1..=n {
            let p = ring.points[i % n];
            let z = z_at(i);
            let mut m = if z == current {
                Move::feed_xy(p, cut.feed)
            } else {
                Move::feed_xyz(p, z, cut.feed)
            };
            m.circle = arcs.get(here, p);
            moves.push(m);
            here = p;
            current = z;
        }
    }
    moves.push(Move::rapid_z(cut.safe_z));
    (moves, first)
}

/// The rings in cutting order from `start`: each time the one whose start is nearest.
pub fn order(rings: Vec<PrismRing>, start: Point) -> Vec<PrismRing> {
    let mut remaining = rings;
    let mut ordered = Vec::with_capacity(remaining.len());
    let mut here = start;
    while !remaining.is_empty() {
        let mut best = 0;
        let mut best_d = f64::INFINITY;
        for (i, ring) in remaining.iter().enumerate() {
            let d = ring
                .points
                .first()
                .map_or(f64::INFINITY, |&p| distance(here, p));
            if d < best_d {
                (best, best_d) = (i, d);
            }
        }
        let ring = remaining.remove(best);
        here = ring.points.first().copied().unwrap_or(here);
        ordered.push(ring);
    }
    ordered
}
