// SPDX-License-Identifier: GPL-3.0-or-later
//! Fill pattern (docs/specs/fill-pattern.md): grooves of lines, cross-hatch, rings or waves
//! inside closed shapes, their selected holes left alone. Lines and waves are laid out from
//! a fixed anchor (the drawing's origin, moved with a copy) and clipped to the region, shrunk
//! by half the groove's width when they stay inside; rings are the region's offsets.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Direction, FillPattern, PatternOp, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::cavalier::{Area, shrunk_farther};
use crate::flute::Flute;
use crate::offset::{flatten, flatten_tagged};
use crate::path::FollowedPath;
use crate::pocket::build_region;
use crate::profile::{cut_clockwise, orient};

/// The most grooves one operation cuts (docs/specs/fill-pattern.md §6).
pub const MAX_GROOVES: usize = 50_000;

/// The most points one wave is drawn with, whatever its length.
const MAX_WAVE_POINTS: usize = 200_000;

/// What a pattern cuts: open grooves (lines, waves) or closed rings, and the notes.
#[derive(Debug, Clone, Default)]
pub struct Grooves {
    pub open: Vec<Flute>,
    pub rings: Vec<FollowedPath>,
    pub arcs: ArcTable,
    pub notes: Vec<Message>,
}

/// Everything the grooves need besides the shapes.
#[derive(Debug, Clone, Copy)]
pub struct PatternSetup<'a> {
    /// The groove's width at the top, mm.
    pub width: f64,
    /// How deep the grooves go below their start depth, mm.
    pub depth: f64,
    /// The depth of each pass below the start depth, mm.
    pub levels: &'a [f64],
    /// Where lines and waves are laid out from: the drawing's origin, moved with a copy.
    pub anchor: Point,
    pub spindle: Spindle,
}

/// The grooves `op` cuts in the closed `contours` (holes selected inside them are holes).
pub fn grooves(
    op: &PatternOp,
    contours: &[&Contour],
    setup: &PatternSetup,
) -> Result<Grooves, ToolpathError> {
    let mut out = Grooves::default();
    if contours.is_empty() {
        return Ok(out);
    }
    let spacing = op.pattern.spacing();
    if spacing < setup.width - CHAIN_TOL_MM {
        out.notes.push(
            Message::new("toolpath-pattern-overlap")
                .arg("spacing", Value::Length(spacing, Digits::Trim(3)))
                .arg("width", Value::Length(setup.width, Digits::Trim(3))),
        );
    }
    let inset = if op.inside { setup.width / 2.0 } else { 0.0 };
    for region in build_region(contours)? {
        let id = region.contour_ids.first().copied().unwrap_or(0);
        match op.pattern {
            FillPattern::Rings { spacing } => {
                rings(&region.area, id, spacing, inset, op.inside, setup, &mut out)?;
            }
            pattern => {
                let areas = if inset > 0.0 {
                    shrunk_farther(&region.area, inset)?
                } else {
                    vec![region.area.clone()]
                };
                if areas.is_empty() {
                    out.notes
                        .push(Message::new("toolpath-pattern-too-narrow").arg("contour", id));
                    continue;
                }
                for area in &areas {
                    let mut loops = vec![flatten(&area.outer)];
                    loops.extend(area.holes.iter().map(flatten));
                    for line in lines(&pattern, &loops, setup.anchor)? {
                        for piece in line {
                            if crate::ramp::polyline_length(&piece) < setup.width.max(CHAIN_TOL_MM)
                            {
                                continue;
                            }
                            out.open.push(Flute::of_points(
                                id,
                                piece,
                                setup.depth,
                                op.ramp,
                                setup.levels,
                            ));
                            too_many(out.open.len())?;
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Refused beyond [`MAX_GROOVES`].
fn too_many(count: usize) -> Result<(), ToolpathError> {
    if count > MAX_GROOVES {
        return Err(ToolpathError::Job(
            Message::new("toolpath-pattern-too-many").arg("most", MAX_GROOVES),
        ));
    }
    Ok(())
}

/// The rings of one region: its offsets `spacing` apart, from one spacing in (half a
/// spacing when the grooves run up to the line), none nearer the line than `inset`.
fn rings(
    area: &Area,
    id: usize,
    spacing: f64,
    inset: f64,
    inside: bool,
    setup: &PatternSetup,
    out: &mut Grooves,
) -> Result<(), ToolpathError> {
    let clockwise = cut_clockwise(Side::On, Direction::Climb, setup.spindle);
    let mut found = false;
    for k in 1..=MAX_GROOVES {
        let d = if inside {
            k as f64 * spacing
        } else {
            (k as f64 - 0.5) * spacing
        };
        if d < inset - CHAIN_TOL_MM {
            continue;
        }
        let areas = shrunk_farther(area, d)?;
        if areas.is_empty() {
            break;
        }
        for a in &areas {
            for pline in std::iter::once(&a.outer).chain(a.holes.iter()) {
                let ring = flatten_tagged(pline, &mut out.arcs);
                if ring.len() < 3 {
                    continue;
                }
                out.rings.push(FollowedPath {
                    points: orient(&ring, clockwise),
                    closed: true,
                    contour_id: id,
                });
                found = true;
                too_many(out.rings.len())?;
            }
        }
    }
    if !found {
        out.notes
            .push(Message::new("toolpath-pattern-too-narrow").arg("contour", id));
    }
    Ok(())
}

/// The lines (or waves) of `pattern` clipped to `loops` (even-odd), each line as its pieces
/// inside, in the drawing's coordinates.
fn lines(
    pattern: &FillPattern,
    loops: &[Vec<Point>],
    anchor: Point,
) -> Result<Vec<Vec<Vec<Point>>>, ToolpathError> {
    let (directions, spacing, wave) = match *pattern {
        FillPattern::Lines { angle, spacing } => (vec![angle], spacing, None),
        FillPattern::Cross {
            angle,
            spacing,
            between,
        } => (vec![angle, angle + between], spacing, None),
        FillPattern::Waves {
            angle,
            spacing,
            wavelength,
            height,
        } => (
            vec![angle],
            spacing,
            (height > 0.0).then_some((wavelength, height / 2.0)),
        ),
        FillPattern::Rings { .. } => return Ok(Vec::new()),
    };
    let mut out = Vec::new();
    for angle in directions {
        let (sin, cos) = math::sin_cos(angle);
        // (u, v): u along the lines, v across them, from the anchor.
        let to_uv = |p: Point| {
            let (x, y) = (p.0 - anchor.0, p.1 - anchor.1);
            (x * cos + y * sin, -x * sin + y * cos)
        };
        let from_uv = |(u, v): Point| (anchor.0 + u * cos - v * sin, anchor.1 + u * sin + v * cos);
        let uv_loops: Vec<Vec<Point>> = loops
            .iter()
            .map(|l| l.iter().map(|&p| to_uv(p)).collect())
            .collect();
        let edges: Vec<(Point, Point)> = uv_loops
            .iter()
            .flat_map(|l| (0..l.len()).map(move |k| (l[k], l[(k + 1) % l.len()])))
            .filter(|(a, b)| distance(*a, *b) > 0.0)
            .collect();
        let Some((u0, u1, v0, v1)) = bounds(&uv_loops) else {
            continue;
        };
        let amplitude = wave.map_or(0.0, |(_, a)| a);
        let first = ((v0 - amplitude) / spacing).ceil() as i64;
        let last = ((v1 + amplitude) / spacing).floor() as i64;
        if last >= first && (last - first) as usize > MAX_GROOVES {
            too_many(MAX_GROOVES + 1)?;
        }
        for k in first..=last {
            let v = k as f64 * spacing;
            let line = match wave {
                None => vec![(u0 - 1.0, v), (u1 + 1.0, v)],
                Some((wavelength, a)) => wave_points(u0 - 1.0, u1 + 1.0, v, wavelength, a)?,
            };
            // Only edges that reach the line's band can cross it or lie beside it.
            let near: Vec<(Point, Point)> = edges
                .iter()
                .copied()
                .filter(|(a, b)| {
                    a.1.min(b.1) <= v + amplitude + CHAIN_TOL_MM
                        && a.1.max(b.1) >= v - amplitude - CHAIN_TOL_MM
                })
                .collect();
            let pieces: Vec<Vec<Point>> = clip(&line, &near)
                .into_iter()
                .map(|piece| piece.into_iter().map(from_uv).collect())
                .collect();
            if !pieces.is_empty() {
                out.push(pieces);
            }
        }
    }
    Ok(out)
}

/// (u0, u1, v0, v1): the box round the loops.
fn bounds(loops: &[Vec<Point>]) -> Option<(f64, f64, f64, f64)> {
    let mut points = loops.iter().flatten();
    let &(x, y) = points.next()?;
    Some(points.fold((x, x, y, y), |(u0, u1, v0, v1), &(u, v)| {
        (u0.min(u), u1.max(u), v0.min(v), v1.max(v))
    }))
}

/// A sine wave along u from `u0` to `u1` about `v`, `a` either side of it, in pieces short
/// enough that the wave is within the arc tolerance of them.
fn wave_points(
    u0: f64,
    u1: f64,
    v: f64,
    wavelength: f64,
    a: f64,
) -> Result<Vec<Point>, ToolpathError> {
    let k = 2.0 * std::f64::consts::PI / wavelength;
    // A chord h long under a curve bending at most a·k² strays h²·a·k²/8 from it.
    let piece = (8.0 * ARC_APPROX_TOL_MM / (a * k * k))
        .sqrt()
        .min(wavelength / 8.0);
    let count = ((u1 - u0) / piece).ceil().max(1.0) as usize;
    if count > MAX_WAVE_POINTS {
        return Err(ToolpathError::Job(
            Message::new("toolpath-pattern-too-many").arg("most", MAX_GROOVES),
        ));
    }
    Ok((0..=count)
        .map(|i| {
            let u = u0 + (u1 - u0) * i as f64 / count as f64;
            (u, v + a * math::sin(k * u))
        })
        .collect())
}

/// The pieces of the polyline `line` inside the loops whose `edges` are given (even-odd):
/// split where it crosses an edge, each piece kept when its middle is inside.
fn clip(line: &[Point], edges: &[(Point, Point)]) -> Vec<Vec<Point>> {
    // Where the line crosses an edge, as (segment index + t).
    let mut cuts: Vec<f64> = Vec::new();
    for (i, w) in line.windows(2).enumerate() {
        for &(a, b) in edges {
            if let Some(t) = crossing(w[0], w[1], a, b) {
                cuts.push(i as f64 + t);
            }
        }
    }
    let end = (line.len() - 1) as f64;
    cuts.push(0.0);
    cuts.push(end);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let at = |s: f64| {
        let i = (s.floor() as usize).min(line.len() - 2);
        let t = s - i as f64;
        let (p, q) = (line[i], line[i + 1]);
        (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t)
    };
    let mut pieces = Vec::new();
    for w in cuts.windows(2) {
        let (s0, s1) = (w[0], w[1]);
        let middle = at((s0 + s1) / 2.0);
        if s1 - s0 <= 0.0 || !inside(middle, edges) || on_edge(middle, edges) {
            continue;
        }
        let mut piece = vec![at(s0)];
        let mut i = s0.floor() as usize + 1;
        while (i as f64) < s1 {
            piece.push(line[i]);
            i += 1;
        }
        piece.push(at(s1));
        piece.dedup_by(|a, b| distance(*a, *b) <= CHAIN_TOL_MM);
        if piece.len() >= 2 {
            pieces.push(piece);
        }
    }
    pieces
}

/// Where the segment p→q crosses the edge a→b, as t along p→q in (0, 1); None when they
/// don't cross or are parallel. Half-open on the edge (its start counts, its end not), so a
/// line through a vertex crosses once.
fn crossing(p: Point, q: Point, a: Point, b: Point) -> Option<f64> {
    let r = (q.0 - p.0, q.1 - p.1);
    let s = (b.0 - a.0, b.1 - a.1);
    let denom = r.0 * s.1 - r.1 * s.0;
    if denom == 0.0 {
        return None;
    }
    let w = (a.0 - p.0, a.1 - p.1);
    let t = (w.0 * s.1 - w.1 * s.0) / denom;
    let e = (w.0 * r.1 - w.1 * r.0) / denom;
    ((0.0..1.0).contains(&e) && t > 0.0 && t < 1.0).then_some(t)
}

/// Whether `p` lies on one of `edges`, within the chaining tolerance: a line along the
/// edge itself is no groove inside.
fn on_edge(p: Point, edges: &[(Point, Point)]) -> bool {
    edges.iter().any(|&(a, b)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        math::hypot(p.0 - a.0 - t * dx, p.1 - a.1 - t * dy) <= CHAIN_TOL_MM
    })
}

/// Whether `p` is inside the loops of `edges` (even-odd), by a ray towards +u.
fn inside(p: Point, edges: &[(Point, Point)]) -> bool {
    let mut odd = false;
    for &(a, b) in edges {
        if (a.1 <= p.1) != (b.1 <= p.1) {
            let u = a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if u > p.0 {
                odd = !odd;
            }
        }
    }
    odd
}

/// The rings in cutting order from `start`: each time the one whose start is nearest.
pub fn order_rings(rings: Vec<FollowedPath>, start: Point) -> Vec<FollowedPath> {
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
