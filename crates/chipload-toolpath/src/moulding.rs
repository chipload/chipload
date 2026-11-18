// SPDX-License-Identifier: GPL-3.0-or-later
//! Moulding (docs/specs/moulding.md): a profile swept along a path. The section is sampled
//! across the path; for each distance from the path the tool's tip goes as deep as dropping
//! its own shape onto the section allows (one dimension). Each pass is the path offset by
//! that distance at that depth, with round ends turning round the path's end points, and
//! tapered ends scaling the section's depth to the surface along the path (3D moves).

use cavalier_contours::polyline::{PlineSource, PlineSourceMut, Polyline};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::cutter::Cutter;
use chipload_core::math;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{MouldEnds, MouldProfile, MouldSide, MouldingOp, RampShape, Side};

use crate::ToolpathError;
use crate::arcs::{ArcTable, Circle};
use crate::cavalier::guarded;
use crate::flute::Profile;
use crate::model::Move;
use crate::offset::{offset_rings_tagged, point_distance};

/// The section is sampled this far apart across the path, mm.
pub const SECTION_STEP_MM: f64 = 0.01;
/// The widest section, mm (docs/specs/moulding.md §3).
pub const MAX_WIDTH_MM: f64 = 200.0;
/// Deepest recursion when adding points along a tapered pass.
const MAX_SPLITS: u32 = 12;
/// A bridge the tool leaves above the profile counts from this, mm (below: the stepover's
/// ridges and the sampling).
const BRIDGE_NOTE_MM: f64 = 0.05;
/// The pass distances for the bridging note are tried this far apart, mm.
const BRIDGE_STEP_MM: f64 = 0.05;

/// The section across the path: its depth (negative Z below the start depth) at
/// `d = i × SECTION_STEP_MM` from the path, `width` wide. Beyond it the board is untouched;
/// before the path (`d` < 0) it is waste when the section starts below the surface (an
/// edge moulding), else untouched as well (docs/specs/moulding.md §5, decision 4).
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub width: f64,
    pub z: Vec<f64>,
    /// `d` < 0 is waste: the tool may cut there.
    pub waste_before: bool,
}

impl Section {
    /// The section of `profile`; a drawn shape's from the project `job` (construction
    /// geometry, never among the contours cut).
    pub fn of(profile: &MouldProfile, job: &chipload_job::Job) -> Result<Section, ToolpathError> {
        match *profile {
            MouldProfile::Shape { shape } => drawn_section(shape, job),
            _ => Section::built_in(profile),
        }
    }

    /// The section of a built-in profile, from its size alone (a drawn one is refused here).
    fn built_in(profile: &MouldProfile) -> Result<Section, ToolpathError> {
        let curve: Box<dyn Fn(f64) -> f64> = match *profile {
            MouldProfile::Cove { radius: r } => {
                Box::new(move |d: f64| -(r * r - d * d).max(0.0).sqrt())
            }
            MouldProfile::Roundover { radius: r } => {
                Box::new(move |d: f64| -(r - (r * r - (r - d) * (r - d)).max(0.0).sqrt()))
            }
            MouldProfile::Ogee { width, depth } => {
                let (half_w, half_d) = (width / 2.0, depth / 2.0);
                Box::new(move |d: f64| {
                    if d <= half_w {
                        // A cove: down at the path, easing to the middle.
                        let u = (d / half_w).clamp(0.0, 1.0);
                        -half_d - half_d * (1.0 - u * u).max(0.0).sqrt()
                    } else {
                        // A round-over up to the surface.
                        let u = ((width - d) / half_w).clamp(0.0, 1.0);
                        -half_d * (1.0 - (1.0 - u * u).max(0.0).sqrt())
                    }
                })
            }
            MouldProfile::Bead { radius: r } => {
                Box::new(move |d: f64| -r + (r * r - (d - r) * (d - r)).max(0.0).sqrt())
            }
            MouldProfile::Flute { radius: r } => {
                Box::new(move |d: f64| -(r * r - (d - r) * (d - r)).max(0.0).sqrt())
            }
            MouldProfile::Shape { shape } => {
                return Err(ToolpathError::Job(
                    Message::new("toolpath-moulding-profile-shape").arg("shape", shape),
                ));
            }
        };
        let width = match *profile {
            MouldProfile::Cove { radius } | MouldProfile::Roundover { radius } => radius,
            MouldProfile::Ogee { width, .. } => width,
            MouldProfile::Bead { radius } | MouldProfile::Flute { radius } => 2.0 * radius,
            MouldProfile::Shape { .. } => 0.0,
        };
        Section::sampled(width, curve.as_ref())
    }

    /// `curve` sampled across `width`.
    fn sampled(width: f64, curve: &dyn Fn(f64) -> f64) -> Result<Section, ToolpathError> {
        if width.is_nan() || width <= CHAIN_TOL_MM {
            return Err(ToolpathError::Job(Message::new(
                "toolpath-moulding-no-width",
            )));
        }
        if width > MAX_WIDTH_MM {
            return Err(ToolpathError::Job(
                Message::new("toolpath-moulding-too-wide")
                    .arg("width", Value::Length(width, Digits::Trim(2)))
                    .arg("most", Value::Length(MAX_WIDTH_MM, Digits::Trim(0))),
            ));
        }
        let n = (width / SECTION_STEP_MM).ceil() as usize;
        let z: Vec<f64> = (0..=n)
            .map(|i| curve((i as f64 * SECTION_STEP_MM).min(width)).min(0.0))
            .collect();
        let waste_before = z.first().is_some_and(|&z0| z0 < -DEPTH_TOL_MM);
        Ok(Section {
            width,
            z,
            waste_before,
        })
    }

    /// How deep it goes, mm (positive).
    pub fn deepest(&self) -> f64 {
        -self.z.iter().copied().fold(0.0, f64::min)
    }

    /// The section's Z at `d` from the path, scaled by `k`, straight between samples; None
    /// where it is waste.
    fn at(&self, d: f64, k: f64) -> Option<f64> {
        if d < -CHAIN_TOL_MM {
            return if self.waste_before { None } else { Some(0.0) };
        }
        // Past its end the board is untouched: the section stops at `width` exactly, not
        // at the next sample of its grid (the property test's gouge, 2026-11-16).
        if d > self.width {
            return Some(0.0);
        }
        let last = self.z.len() - 1;
        let x = (d / SECTION_STEP_MM).clamp(0.0, last as f64);
        let i = (x.floor() as usize).min(last.saturating_sub(1));
        // The last sample lies at `width`, which may come before the grid's next step.
        let start = i as f64 * SECTION_STEP_MM;
        let end = ((i + 1) as f64 * SECTION_STEP_MM).min(self.width);
        let t = if end > start {
            ((d - start) / (end - start)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let z = match self.z.get(i + 1) {
            Some(&next) => self.z[i] + (next - self.z[i]) * t,
            None => self.z[i],
        };
        Some(z * k)
    }

    /// The tip's Z with its axis `c` from the path, the section scaled by `k`: as deep as
    /// the cutter can go touching the section and nowhere below it (a 1D drop).
    pub fn drop(&self, cutter: Cutter, c: f64, k: f64) -> f64 {
        let r = cutter.radius();
        // The section's own sample points under the tool (on its grid, before and beyond it
        // too), so nothing between samples is skipped.
        let first = ((c - r) / SECTION_STEP_MM).floor() as i64;
        let last = ((c + r) / SECTION_STEP_MM).ceil() as i64;
        let mut z = f64::NEG_INFINITY;
        // The rim's two edges too: a steep wall between two samples still stops it. And
        // the top of the wall where the section ends (the board beyond it is at 0), which
        // needn't lie on the grid.
        let end_wall = self.width + f64::EPSILON * self.width.max(1.0);
        let rims = [(c - r, -r), (c + r, r), (end_wall, end_wall - c)];
        let grid = (first..=last).map(|i| {
            let d = i as f64 * SECTION_STEP_MM;
            (d, d - c)
        });
        for (d, off) in grid.chain(rims) {
            let Some(h) = cutter.height_at(off) else {
                continue;
            };
            let Some(surface) = self.at(d, k) else {
                continue;
            };
            z = z.max(surface - h);
        }
        // Nothing under the tool but waste: nothing to cut against, it stays at the surface.
        if z == f64::NEG_INFINITY {
            return 0.0;
        }
        z.min(0.0)
    }
}

/// A drawn open shape read as a section: across to the right, up the page; its leftmost
/// point at the path, its highest point the surface; where it doubles back, the lowest.
fn drawn_section(shape: u64, job: &chipload_job::Job) -> Result<Section, ToolpathError> {
    let parts: Vec<Primitive> = job
        .drawing
        .shapes
        .iter()
        .find(|s| s.id == shape)
        .map(|s| s.segments().iter().map(|seg| seg.to_primitive()).collect())
        .unwrap_or_default();
    section_of_parts(shape, &parts)
}

/// The section a drawn shape's lines and arcs `parts` make (shape `shape`, for the error).
fn section_of_parts(shape: u64, parts: &[Primitive]) -> Result<Section, ToolpathError> {
    let closed = match (parts.first(), parts.last()) {
        (Some(first), Some(last)) => distance(first.start(), last.end()) <= CHAIN_TOL_MM,
        _ => false,
    };
    if parts.is_empty() || closed {
        return Err(ToolpathError::Job(
            Message::new("toolpath-moulding-profile-shape").arg("shape", shape),
        ));
    }
    let chords: Vec<(Point, Point)> = parts
        .iter()
        .flat_map(|prim| {
            let points = prim.points();
            points.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>()
        })
        .collect();
    let (x0, x1, top) = chords.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        |(lo, hi, top), (a, b)| {
            (
                lo.min(a.0).min(b.0),
                hi.max(a.0).max(b.0),
                top.max(a.1).max(b.1),
            )
        },
    );
    let curve = move |d: f64| {
        let x = x0 + d;
        let mut lowest = f64::INFINITY;
        for (a, b) in &chords {
            let (lo, hi) = (a.0.min(b.0), a.0.max(b.0));
            if x < lo - CHAIN_TOL_MM || x > hi + CHAIN_TOL_MM {
                continue;
            }
            let y = if (b.0 - a.0).abs() <= CHAIN_TOL_MM {
                a.1.min(b.1)
            } else {
                a.1 + (b.1 - a.1) * ((x - a.0) / (b.0 - a.0)).clamp(0.0, 1.0)
            };
            lowest = lowest.min(y);
        }
        if lowest.is_finite() {
            lowest - top
        } else {
            0.0
        }
    };
    Section::sampled(x1 - x0, &curve)
}

/// The most points a preview of a section is drawn with.
const PREVIEW_POINTS: usize = 160;

/// The section of `profile` as points (distance across, Z below the surface) for a small
/// drawing of it in the window (docs/specs/moulding.md §9): a drawn profile's from its
/// shape among `contours` (the drawing's, construction geometry included). None when it
/// can't be read (a drawn shape missing or closed).
pub fn preview(profile: &MouldProfile, contours: &[Contour]) -> Option<Vec<Point>> {
    let section = match *profile {
        MouldProfile::Shape { shape } => {
            let parts: Vec<Primitive> = contours
                .iter()
                .filter(|c| !c.closed)
                .flat_map(|c| &c.parts)
                .filter(|p| p.segment.shape == Some(shape))
                .map(|p| p.segment.prim)
                .collect();
            section_of_parts(shape, &parts).ok()?
        }
        _ => Section::built_in(profile).ok()?,
    };
    let n = section.z.len();
    let stride = n.div_ceil(PREVIEW_POINTS).max(1);
    let mut points: Vec<Point> = (0..n)
        .step_by(stride)
        .map(|i| {
            (
                (i as f64 * SECTION_STEP_MM).min(section.width),
                section.z[i],
            )
        })
        .collect();
    if let Some(&last) = section.z.last() {
        points.push((section.width, last));
    }
    Some(points)
}

/// The tool in a preview of the section: each pass's tip (distance across, Z) and the
/// tool's outline at the deepest pass, its shank going up past the surface.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewTool {
    pub tips: Vec<Point>,
    pub outline: Vec<Point>,
}

/// The passes of `profile` with `cutter` `stepover` apart across it, for the window's
/// drawing (docs/specs/moulding.md §9); None when the section can't be read or the
/// stepover is no use.
pub fn preview_tool(
    profile: &MouldProfile,
    contours: &[Contour],
    cutter: Cutter,
    stepover: f64,
) -> Option<PreviewTool> {
    let section = match *profile {
        MouldProfile::Shape { shape } => {
            let parts: Vec<Primitive> = contours
                .iter()
                .filter(|c| !c.closed)
                .flat_map(|c| &c.parts)
                .filter(|p| p.segment.shape == Some(shape))
                .map(|p| p.segment.prim)
                .collect();
            section_of_parts(shape, &parts).ok()?
        }
        _ => Section::built_in(profile).ok()?,
    };
    if stepover.is_nan()
        || stepover <= CHAIN_TOL_MM
        || section.width / stepover > PREVIEW_POINTS as f64 * 4.0
    {
        return None;
    }
    let mut tips = Vec::new();
    let mut c: f64 = 0.0;
    loop {
        let at = c.min(section.width);
        let z = section.drop(cutter, at, 1.0);
        if z < -DEPTH_TOL_MM {
            tips.push((at, z));
        }
        if c >= section.width - CHAIN_TOL_MM {
            break;
        }
        c += stepover;
    }
    let &(cx, cz) = tips.iter().min_by(|a, b| a.1.total_cmp(&b.1))?;
    let r = cutter.radius();
    let steps = 24;
    let mut outline: Vec<Point> = Vec::with_capacity(steps + 3);
    let shank = cz + cutter.height_at(r).unwrap_or(0.0) + section.deepest() + 2.0 * r;
    outline.push((cx - r, shank));
    for i in 0..=steps {
        let x = -r + 2.0 * r * i as f64 / steps as f64;
        outline.push((cx + x, cz + cutter.height_at(x).unwrap_or(0.0)));
    }
    outline.push((cx + r, shank));
    Some(PreviewTool { tips, outline })
}

/// Everything the passes need.
#[derive(Debug, Clone, Copy)]
pub struct MouldCut<'a> {
    pub cutter: Cutter,
    /// Each level's depth below the start depth, going down.
    pub levels: &'a [f64],
    pub top_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    /// Open passes run the way the path is drawn.
    pub forward: bool,
    /// Closed passes run clockwise.
    pub clockwise: bool,
}

/// One pass: its points and each one's Z below the start depth (negative), and whether it
/// is closed.
#[derive(Debug, Clone, PartialEq)]
pub struct Pass {
    pub points: Vec<Point>,
    pub z: Vec<f64>,
    pub closed: bool,
    /// The deepest of it, for the order (shallow first).
    pub low: f64,
}

/// The passes of `op` along `paths`, the section `section`, the cutter `cutter`, and the
/// notes (a bridge the tool leaves).
pub fn passes(
    op: &MouldingOp,
    section: &Section,
    cutter: Cutter,
    paths: &[&Contour],
    arcs: &mut ArcTable,
) -> Result<(Vec<Pass>, Vec<Message>), ToolpathError> {
    let mut notes = Vec::new();
    let diameter = 2.0 * cutter.radius();
    if op.stepover > diameter + CHAIN_TOL_MM {
        return Err(ToolpathError::Job(
            Message::new("toolpath-moulding-stepover")
                .arg("stepover", Value::Length(op.stepover, Digits::Trim(3)))
                .arg("diameter", Value::Length(diameter, Digits::Trim(3))),
        ));
    }
    if let Some(gap) = bridge(section, cutter) {
        notes.push(
            Message::new("toolpath-moulding-bridge")
                .arg("gap", Value::Length(gap, Digits::Trim(2))),
        );
    }
    // The distances across the profile: every stepover, and the far edge.
    let mut distances: Vec<f64> = Vec::new();
    let mut c: f64 = 0.0;
    while c < section.width - CHAIN_TOL_MM {
        distances.push(c);
        c += op.stepover;
    }
    distances.push(section.width);
    let mut out = Vec::new();
    for &c in &distances {
        let full = section.drop(cutter, c, 1.0);
        if full >= -DEPTH_TOL_MM {
            continue;
        }
        for path in paths {
            for (points, closed, along) in pass_lines(op, path, c, arcs)? {
                let z_of = |s: Option<f64>| match s {
                    Some(s) => section.drop(cutter, c, along.k(s)),
                    None => full,
                };
                let (points, z) = refined(&points, &along, closed, &z_of, arcs);
                let (points, z) = simplified(points, z, arcs);
                let (points, z) = trimmed(points, z, closed);
                if points.len() < 2 {
                    continue;
                }
                let low = z.iter().copied().fold(0.0, f64::min);
                out.push(Pass {
                    points,
                    z,
                    closed,
                    low,
                });
            }
        }
    }
    Ok((out, notes))
}

/// How far the tool stays above the section at most where it can't reach (a concave curve
/// tighter than it, a corner), passes taken at every distance; None below the note's mark.
fn bridge(section: &Section, cutter: Cutter) -> Option<f64> {
    let r = cutter.radius();
    let count = ((section.width + 2.0 * r) / BRIDGE_STEP_MM).ceil() as usize;
    let tips: Vec<(f64, f64)> = (0..=count)
        .map(|i| {
            let c = -r + i as f64 * BRIDGE_STEP_MM;
            (c, section.drop(cutter, c, 1.0))
        })
        .collect();
    let mut worst: f64 = 0.0;
    let n = section.z.len();
    let stride = ((BRIDGE_STEP_MM / SECTION_STEP_MM) as usize).max(1);
    for i in (0..n).step_by(stride) {
        let d = i as f64 * SECTION_STEP_MM;
        let reached = tips
            .iter()
            .filter_map(|&(c, z)| cutter.height_at(d - c).map(|h| z + h))
            .fold(f64::INFINITY, f64::min);
        if reached.is_finite() {
            worst = worst.max(reached - section.z[i]);
        }
    }
    (worst > BRIDGE_NOTE_MM).then_some(worst)
}

/// Where a pass point lies along the path, for a taper: the path's points and their
/// distances, and the taper's profile (None: no taper).
struct Along {
    path: Vec<Point>,
    at: Vec<f64>,
    taper: Option<Profile>,
}

impl Along {
    /// The section's scale at `s` along the path: 1 away from a taper, down to 0 at the end.
    fn k(&self, s: f64) -> f64 {
        self.taper.map_or(1.0, |t| t.depth_at(s))
    }

    /// The distance along the path of the point nearest `p`; None without a taper.
    fn s_of(&self, p: Point) -> Option<f64> {
        self.taper?;
        let mut best = (f64::INFINITY, 0.0);
        for (i, w) in self.path.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let q = (a.0 + dx * t, a.1 + dy * t);
            let d = distance(p, q);
            if d < best.0 {
                best = (d, self.at[i] + t * len2.sqrt());
            }
        }
        Some(best.1)
    }
}

/// The lines of one pass `c` from `path` on the profile's side: closed rings for a closed
/// path; for an open one its offset, with round ends turning round the end points.
fn pass_lines(
    op: &MouldingOp,
    path: &Contour,
    c: f64,
    arcs: &mut ArcTable,
) -> Result<Vec<(Vec<Point>, bool, Along)>, ToolpathError> {
    let no_taper = || Along {
        path: Vec::new(),
        at: Vec::new(),
        taper: None,
    };
    if path.closed {
        let side = match op.side {
            MouldSide::Inside | MouldSide::Left => Side::Inside,
            MouldSide::Outside | MouldSide::Right => Side::Outside,
        };
        let rings = if c > CHAIN_TOL_MM {
            offset_rings_tagged(path, side, c, arcs)?
        } else {
            offset_rings_tagged(path, Side::On, 0.0, arcs)?
        };
        return Ok(rings.into_iter().map(|r| (r, true, no_taper())).collect());
    }
    let pline = open_pline(path);
    let path_points = flatten_open(&pline, &mut ArcTable::default());
    let mut at = vec![0.0];
    for w in path_points.windows(2) {
        let last = at.last().copied().unwrap_or(0.0);
        at.push(last + distance(w[0], w[1]));
    }
    let length = at.last().copied().unwrap_or(0.0);
    if length <= CHAIN_TOL_MM {
        return Ok(Vec::new());
    }
    // Left of the path as drawn: a positive offset.
    let sign = match op.side {
        MouldSide::Left | MouldSide::Inside => 1.0,
        MouldSide::Right | MouldSide::Outside => -1.0,
    };
    let offsets: Vec<Polyline<f64>> = if c > CHAIN_TOL_MM {
        guarded(Message::new("toolpath-moulding-offset"), || {
            pline.parallel_offset(sign * c)
        })?
    } else {
        vec![pline.clone()]
    };
    let taper = match op.ends {
        MouldEnds::Taper { length: t, shape } => Some(Profile::with(1.0, length, (t, t), shape).0),
        _ => None,
    };
    let mut out = Vec::new();
    for offset in offsets {
        let mut points = flatten_open(&offset, arcs);
        verify(&points, path, c)?;
        if op.ends == MouldEnds::Round && c > CHAIN_TOL_MM {
            points = rounded(points, &path_points, sign, c, arcs);
        }
        let along = Along {
            path: path_points.clone(),
            at: at.clone(),
            taper,
        };
        out.push((points, false, along));
    }
    Ok(out)
}

/// The pass with a quarter arc of radius `c` round each of the path's end points, from the
/// profile's side to the path's own line continued (docs/specs/moulding.md §4).
fn rounded(
    points: Vec<Point>,
    path: &[Point],
    sign: f64,
    c: f64,
    arcs: &mut ArcTable,
) -> Vec<Point> {
    let (Some(&s0), Some(&s1), Some(&e0), Some(&e1)) = (
        path.first(),
        path.get(1),
        path.get(path.len().wrapping_sub(2)),
        path.last(),
    ) else {
        return points;
    };
    let unit = |a: Point, b: Point| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = math::hypot(dx, dy).max(f64::MIN_POSITIVE);
        (dx / len, dy / len)
    };
    let normal = |t: Point| (-t.1 * sign, t.0 * sign);
    // Start: from straight back along the path, round to the profile's side.
    let ts = unit(s0, s1);
    let ns = normal(ts);
    let back = (-ts.0, -ts.1);
    let start = quarter(s0, c, back, ns, arcs);
    // End: from the profile's side round to straight on.
    let te = unit(e0, e1);
    let ne = normal(te);
    let end = quarter(e1, c, ne, te, arcs);
    let mut out = start;
    out.extend(points.into_iter().skip(1));
    out.extend(end.into_iter().skip(1));
    out
}

/// A quarter arc of radius `r` round `centre` from direction `from` to direction `to`, as
/// points within the arc tolerance, its chords recorded on its circle.
fn quarter(centre: Point, r: f64, from: Point, to: Point, arcs: &mut ArcTable) -> Vec<Point> {
    let a0 = math::atan2(from.1, from.0);
    let mut sweep = math::atan2(to.1, to.0) - a0;
    while sweep > std::f64::consts::PI {
        sweep -= 2.0 * std::f64::consts::PI;
    }
    while sweep < -std::f64::consts::PI {
        sweep += 2.0 * std::f64::consts::PI;
    }
    let arc = Arc {
        center: centre,
        radius: r,
        start_angle: a0,
        sweep,
    };
    let points = Primitive::Arc(arc).points();
    let circle = Circle {
        center: centre,
        radius: r,
    };
    for w in points.windows(2) {
        arcs.insert(w[0], w[1], circle);
    }
    points
}

/// The open polyline of an open contour, with true arcs.
fn open_pline(contour: &Contour) -> Polyline<f64> {
    let mut pline = Polyline::new();
    let mut end = None;
    for part in &contour.parts {
        let prim = if part.reversed {
            match part.segment.prim {
                Primitive::Line(l) => Primitive::Line(Line {
                    start: l.end,
                    end: l.start,
                }),
                Primitive::Arc(a) => Primitive::Arc(Arc {
                    start_angle: a.start_angle + a.sweep,
                    sweep: -a.sweep,
                    ..a
                }),
            }
        } else {
            part.segment.prim
        };
        match prim {
            Primitive::Line(line) => pline.add(line.start.0, line.start.1, 0.0),
            Primitive::Arc(arc) => {
                let pieces = if arc.sweep.abs() > std::f64::consts::PI {
                    2
                } else {
                    1
                };
                let sweep = arc.sweep / f64::from(pieces);
                for i in 0..pieces {
                    let piece = Arc {
                        start_angle: arc.start_angle + sweep * f64::from(i),
                        sweep,
                        ..arc
                    };
                    let start = piece.point_at(piece.start_angle);
                    pline.add(start.0, start.1, piece.bulge());
                }
            }
        }
        end = Some(prim.end());
    }
    if let Some(e) = end {
        pline.add(e.0, e.1, 0.0);
    }
    pline
}

/// An open polyline as points within the arc tolerance, its arc chords recorded.
fn flatten_open(pline: &Polyline<f64>, arcs: &mut ArcTable) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::new();
    let n = pline.vertex_count();
    for i in 0..n.saturating_sub(1) {
        let (v1, v2) = (pline.at(i), pline.at(i + 1));
        let (a, b) = ((v1.x, v1.y), (v2.x, v2.y));
        let piece: Vec<Point> = match Arc::from_bulge(a, b, v1.bulge) {
            Some(arc) => {
                let pts = Primitive::Arc(arc).points();
                let circle = Circle {
                    center: arc.center,
                    radius: arc.radius,
                };
                for w in pts.windows(2) {
                    arcs.insert(w[0], w[1], circle);
                }
                pts
            }
            None => vec![a, b],
        };
        for p in piece {
            if points.last().is_none_or(|&l| distance(l, p) > CHAIN_TOL_MM) {
                points.push(p);
            }
        }
    }
    points
}

/// Every point of an open pass at `c` from the path, no nearer (a library result is never
/// trusted blindly).
fn verify(points: &[Point], path: &Contour, c: f64) -> Result<(), ToolpathError> {
    for &p in points {
        let d = path
            .parts
            .iter()
            .map(|part| point_distance(&part.segment.prim, p))
            .fold(f64::INFINITY, f64::min);
        if d < c - ARC_APPROX_TOL_MM {
            return Err(ToolpathError::Geometry(
                Message::new("toolpath-moulding-offset-near")
                    .arg("x", Value::LengthNumber(p.0, Digits::Fixed(3)))
                    .arg("y", Value::LengthNumber(p.1, Digits::Fixed(3))),
            ));
        }
    }
    Ok(())
}

/// The pass's points with their Z from `z_of` (given the distance along the path for a
/// taper), points added where Z between two strays from the straight line by more than the
/// arc tolerance.
fn refined(
    points: &[Point],
    along: &Along,
    closed: bool,
    z_of: &dyn Fn(Option<f64>) -> f64,
    arcs: &mut ArcTable,
) -> (Vec<Point>, Vec<f64>) {
    let zs: Vec<f64> = points.iter().map(|&p| z_of(along.s_of(p))).collect();
    if along.taper.is_none() {
        return (points.to_vec(), zs);
    }
    let n = points.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    let mut out_p = Vec::with_capacity(n);
    let mut out_z = Vec::with_capacity(n);
    for i in 0..edges {
        let j = (i + 1) % n;
        out_p.push(points[i]);
        out_z.push(zs[i]);
        let circle = arcs.get(points[i], points[j]);
        let mut inner = Vec::new();
        split(
            (points[i], zs[i]),
            (points[j], zs[j]),
            circle,
            along,
            z_of,
            MAX_SPLITS,
            &mut inner,
        );
        if let Some(c) = circle
            && !inner.is_empty()
        {
            let mut from = points[i];
            for &(p, _) in &inner {
                arcs.insert(from, p, c);
                from = p;
            }
            arcs.insert(from, points[j], c);
        }
        for (p, z) in inner {
            out_p.push(p);
            out_z.push(z);
        }
    }
    if !closed && let (Some(&p), Some(&z)) = (points.last(), zs.last()) {
        out_p.push(p);
        out_z.push(z);
    }
    (out_p, out_z)
}

/// Points between `a` and `b` where Z strays from the straight line between theirs.
fn split(
    a: (Point, f64),
    b: (Point, f64),
    circle: Option<Circle>,
    along: &Along,
    z_of: &dyn Fn(Option<f64>) -> f64,
    splits: u32,
    out: &mut Vec<(Point, f64)>,
) {
    if splits == 0 || distance(a.0, b.0) <= 4.0 * CHAIN_TOL_MM {
        return;
    }
    let mut m = ((a.0.0 + b.0.0) / 2.0, (a.0.1 + b.0.1) / 2.0);
    if let Some(c) = circle {
        let (dx, dy) = (m.0 - c.center.0, m.1 - c.center.1);
        let d = math::hypot(dx, dy);
        if d > 0.0 {
            m = (
                c.center.0 + dx * c.radius / d,
                c.center.1 + dy * c.radius / d,
            );
        }
    }
    let z = z_of(along.s_of(m));
    if (z - (a.1 + b.1) / 2.0).abs() <= ARC_APPROX_TOL_MM {
        return;
    }
    split(a, (m, z), circle, along, z_of, splits - 1, out);
    out.push((m, z));
    split((m, z), b, circle, along, z_of, splits - 1, out);
}

/// The pass without the points that add nothing: on a straight line between their
/// neighbours, no arc either side, their Z what the line between theirs gives. The first
/// and last stay.
fn simplified(points: Vec<Point>, z: Vec<f64>, arcs: &ArcTable) -> (Vec<Point>, Vec<f64>) {
    let n = points.len();
    if n < 3 {
        return (points, z);
    }
    let mut out_p = vec![points[0]];
    let mut out_z = vec![z[0]];
    for i in 1..n - 1 {
        let (Some(&a), Some(&za)) = (out_p.last(), out_z.last()) else {
            continue;
        };
        let (q, zq, b, zb) = (points[i], z[i], points[i + 1], z[i + 1]);
        let straight = arcs.get(a, q).is_none() && arcs.get(q, b).is_none();
        let ab = distance(a, b);
        let off = if ab > 0.0 {
            ((b.0 - a.0) * (q.1 - a.1) - (b.1 - a.1) * (q.0 - a.0)).abs() / ab
        } else {
            f64::INFINITY
        };
        let t = if ab > 0.0 { distance(a, q) / ab } else { 0.0 };
        let even = (zq - (za + (zb - za) * t)).abs() <= ARC_APPROX_TOL_MM / 4.0;
        if straight && off <= CHAIN_TOL_MM && even {
            continue;
        }
        out_p.push(q);
        out_z.push(zq);
    }
    out_p.push(points[n - 1]);
    out_z.push(z[n - 1]);
    (out_p, out_z)
}

/// An open pass without its ends at the surface (a taper's last stretch), one point at the
/// surface kept at each end so the pass starts and ends there.
fn trimmed(points: Vec<Point>, z: Vec<f64>, closed: bool) -> (Vec<Point>, Vec<f64>) {
    if closed {
        return (points, z);
    }
    let cut = |z: f64| z < -DEPTH_TOL_MM;
    let Some(first) = z.iter().position(|&v| cut(v)) else {
        return (Vec::new(), Vec::new());
    };
    let last = z.iter().rposition(|&v| cut(v)).unwrap_or(first);
    let lo = first.saturating_sub(1);
    let hi = (last + 1).min(points.len() - 1);
    (points[lo..=hi].to_vec(), z[lo..=hi].to_vec())
}

/// The moves of every pass, level by level, shallow passes first within a level, each
/// pass no deeper than its level; a retract between passes.
pub fn moulding_moves(
    passes: &[Pass],
    cut: &MouldCut,
    arcs: &ArcTable,
) -> (Vec<Move>, Option<Point>) {
    let mut order: Vec<&Pass> = passes.iter().collect();
    order.sort_by(|a, b| b.low.total_cmp(&a.low));
    let mut moves = Vec::new();
    let mut end = None;
    let mut previous = 0.0;
    for &level in cut.levels {
        for pass in &order {
            // Nothing new at this level: cut to its floor already.
            if pass.low >= -previous - DEPTH_TOL_MM {
                continue;
            }
            let (points, z) = directed(pass, cut);
            let n = points.len();
            let count = if pass.closed { n + 1 } else { n };
            let at = |i: usize| cut.top_z + z[i % n].max(-level);
            moves.push(Move::rapid_z(cut.safe_z));
            moves.push(Move::rapid_xy(points[0]));
            moves.push(Move::feed_z(at(0), cut.plunge_feed));
            let mut current = at(0);
            let mut here = points[0];
            for i in 1..count {
                let p = points[i % n];
                let z_here = at(i);
                let mut m = if z_here == current {
                    Move::feed_xy(p, cut.feed)
                } else {
                    Move::feed_xyz(p, z_here, cut.feed)
                };
                m.circle = arcs.get(here, p);
                moves.push(m);
                here = p;
                current = z_here;
            }
            moves.push(Move::rapid_z(cut.safe_z));
            end = Some(here);
        }
        previous = level;
    }
    (moves, end)
}

/// The pass's points and Z the way it is cut.
fn directed(pass: &Pass, cut: &MouldCut) -> (Vec<Point>, Vec<f64>) {
    let reverse = if pass.closed {
        (chipload_core::primitives::signed_area(&pass.points) < 0.0) != cut.clockwise
    } else {
        !cut.forward
    };
    if reverse {
        let mut p = pass.points.clone();
        let mut z = pass.z.clone();
        p.reverse();
        z.reverse();
        (p, z)
    } else {
        (pass.points.clone(), pass.z.clone())
    }
}

/// The ramp shape a taper takes, for the setup sheet.
pub fn taper_shape_key(shape: RampShape) -> &'static str {
    match shape {
        RampShape::Straight => "sheet-ramp-straight",
        RampShape::Curved => "sheet-ramp-curved",
    }
}
