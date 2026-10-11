// SPDX-License-Identifier: GPL-3.0-or-later
//! Adaptive clearing as a morphed spiral (docs/specs/adaptive-spiral.md §3.2), designed and
//! written for Chipload.
//!
//! One continuous spiral from the centre of the area's largest circle out to its boundary
//! (with its corners rounded): each turn is a blend of a circle round the start and the
//! boundary, as seen from the start (the area must be star-shaped from there). Turns are
//! never farther apart than the stepover, and closer where the path curves so tightly that
//! the stepover would put more of the tool's edge in material than a straight cut does (the
//! engagement angle, §2). The points are fitted with arcs.

use std::f64::consts::{PI, SQRT_2, TAU};

use chipload_core::Point;
use chipload_core::primitives::{Line, Primitive, distance, max_segment_angle};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};

use crate::ToolpathError;
use crate::adaptive::Grid;
use crate::arcs::{ArcTable, Circle};
use crate::fronts::largest_circle;
use crate::offset::{pline_segments, point_distance};
use crate::pocket::Area;

/// An area is cleared by one spiral when its farthest boundary point (seen from the
/// start, corners rounded) is at most this many times as far as its nearest. Measured
/// (stage 1): a circle cuts a median 0.9 of the target angle, a square only 0.42 to 0.66,
/// a 3:1 rectangle about 0.35.
const ROUND_ENOUGH: f64 = 1.25;
/// Directions sampled round the start when spacing a turn.
const DIRECTIONS: usize = 360;
/// Directions per turn; more points where they would lie farther apart than
/// `POINT_SPACING_MM`.
const MIN_POINTS_PER_TURN: usize = 360;
/// The spiral's points are at most this far apart, mm (before fitting arcs).
const POINT_SPACING_MM: f64 = 0.25;
/// A turn is never closer than this share of the stepover to the last (the very first
/// turns round the plunge would otherwise need ever smaller steps).
const MIN_STEP_SHARE: f64 = 0.1;
/// Most turns, against runaway loops on absurd input.
const MAX_TURNS: usize = 100_000;
/// Points round the tool's edge sampled when measuring the engagement angle.
const EDGE_SAMPLES: usize = 120;

/// What the spiral needs.
pub struct Spiral<'a> {
    /// The area the tool centre may be in (one part of the wall-pass area).
    pub area: &'a Area,
    pub tool_radius: f64,
    /// The stepover, mm.
    pub width: f64,
    /// true: material on the right of the direction of travel (the spiral turns
    /// counter-clockwise).
    pub material_right: bool,
}

/// The spiral's points (from the start, arcs recorded in `arcs`) and how many of them
/// belong to the opening round the plunge (the first turn: an entry, not measured).
#[derive(Debug, Clone, PartialEq)]
pub struct SpiralPath {
    pub points: Vec<Point>,
    pub opening: usize,
}

/// The engagement angle a straight cut of `width` takes with a tool of `tool_radius`.
pub fn target_angle(width: f64, tool_radius: f64) -> f64 {
    (1.0 - width / tool_radius).clamp(-1.0, 1.0).acos()
}

/// The spiral for the area, or None when the area doesn't suit one (it has islands, has no
/// room for a turn, or isn't star-shaped from its largest circle once its corners are
/// rounded): the caller clears it another way.
pub fn plan(spiral: &Spiral<'_>, arcs: &mut ArcTable) -> Result<Option<SpiralPath>, ToolpathError> {
    let Spiral {
        area,
        tool_radius,
        width,
        material_right,
    } = *spiral;
    if !area.holes.is_empty() || width.is_nan() || width <= 0.0 {
        return Ok(None);
    }
    let Some((start, largest)) = largest_circle(area)? else {
        return Ok(None);
    };
    // Corners rounded so the corner a turn leaves to the wall pass is about one stepover
    // deep (a 90° corner of radius ρ leaves (√2 − 1) ρ).
    let corner = (width / (SQRT_2 - 1.0)).min(0.5 * largest);
    let Some(rounded) = rounded(area, corner)? else {
        return Ok(None);
    };
    let Some(polar) = Polar::new(&rounded, start) else {
        return Ok(None);
    };
    // One spiral has as many turns in every direction: the farthest point sets them, and
    // nearer ones get thinner cuts. Only a roundish area is cleared well by one (§3.1).
    let farthest = (0..DIRECTIONS)
        .map(|k| polar.radius(TAU * k as f64 / DIRECTIONS as f64))
        .fold(0.0, f64::max);
    if farthest > ROUND_ENOUGH * polar.nearest {
        return Ok(None);
    }
    let morph = Morph {
        polar: &polar,
        inner: polar.nearest,
    };
    let theta0 = target_angle(width, tool_radius);
    let raw = turns(&morph, width, tool_radius, theta0, material_right);
    let opening = raw.opening;
    let (points, opening) = fit_arcs(&raw.points, opening, arcs);
    Ok(Some(SpiralPath { points, opening }))
}

/// The area with its convex corners rounded to `radius`, if it stays one piece; also
/// shrunk by `ARC_APPROX_TOL_MM`, so the last turn stays inside the area although its
/// points come from chords and fitted arcs (each within a fraction of that tolerance).
fn rounded(area: &Area, radius: f64) -> Result<Option<Area>, ToolpathError> {
    let inner = area.shrunk(radius)?;
    let [inner] = inner.as_slice() else {
        return Ok(None);
    };
    let outer = inner.grown(radius - ARC_APPROX_TOL_MM)?;
    match outer.as_slice() {
        [one] if one.holes.is_empty() => Ok(Some(one.clone())),
        _ => Ok(None),
    }
}

/// The area's boundary as seen from `centre`: one boundary point in every direction.
struct Polar {
    centre: Point,
    /// Boundary points counter-clockwise, and their directions, increasing (unwrapped).
    points: Vec<Point>,
    angles: Vec<f64>,
    /// Distance to the nearest point of the boundary.
    nearest: f64,
}

impl Polar {
    /// None unless every direction meets the boundary once (star-shaped from `centre`).
    fn new(area: &Area, centre: Point) -> Option<Self> {
        let mut points: Vec<Point> = Vec::new();
        for prim in pline_segments(&area.outer) {
            let piece = match prim {
                Primitive::Line(l) => vec![l.start, l.end],
                Primitive::Arc(a) => {
                    let step = max_segment_angle(a.radius, ARC_APPROX_TOL_MM / 4.0);
                    let n = (a.sweep.abs() / step).ceil().max(1.0) as usize;
                    (0..=n)
                        .map(|k| a.point_at(a.start_angle + a.sweep * k as f64 / n as f64))
                        .collect()
                }
            };
            for p in piece {
                if points.last().is_none_or(|&q| distance(p, q) > CHAIN_TOL_MM) {
                    points.push(p);
                }
            }
        }
        while points.len() > 1
            && points
                .first()
                .zip(points.last())
                .is_some_and(|(a, b)| distance(*a, *b) <= CHAIN_TOL_MM)
        {
            points.pop();
        }
        if points.len() < 3 {
            return None;
        }
        // Counter-clockwise as seen from the centre: every step turns the same way, a little.
        let mut angles = Vec::with_capacity(points.len());
        let mut previous = (points[0].1 - centre.1).atan2(points[0].0 - centre.0);
        angles.push(previous);
        for p in &points[1..] {
            let raw = (p.1 - centre.1).atan2(p.0 - centre.0);
            let mut delta = raw - previous.rem_euclid(TAU);
            delta = (delta + PI).rem_euclid(TAU) - PI;
            if delta <= 0.0 {
                return None;
            }
            previous += delta;
            angles.push(previous);
        }
        // Closing back to the first point: once round, exactly.
        let first = angles[0];
        let last = angles[angles.len() - 1];
        // The closing step, from the last point back to the first, must turn the same way
        // too: then the points went round exactly once.
        let back = (first + TAU) - last;
        if back <= 0.0 || back >= PI {
            return None;
        }
        // The nearest point of the boundary (on a chord, not only at the points).
        let n = points.len();
        let nearest = (0..n)
            .map(|i| {
                let chord = Primitive::Line(Line {
                    start: points[i],
                    end: points[(i + 1) % n],
                });
                point_distance(&chord, centre)
            })
            .fold(f64::INFINITY, f64::min);
        if nearest <= CHAIN_TOL_MM {
            return None;
        }
        Some(Polar {
            centre,
            points,
            angles,
            nearest,
        })
    }

    /// Distance from the centre to the boundary in direction `theta`.
    fn radius(&self, theta: f64) -> f64 {
        let first = self.angles[0];
        let t = first + (theta - first).rem_euclid(TAU);
        let n = self.points.len();
        // The chord from points[i] to points[i + 1] (the last one closes the loop).
        let i = match self.angles.binary_search_by(|a| a.total_cmp(&t)) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        };
        let (a, b) = (self.points[i], self.points[(i + 1) % n]);
        let u = (theta.cos(), theta.sin());
        let d = (b.0 - a.0, b.1 - a.1);
        let w = (a.0 - self.centre.0, a.1 - self.centre.1);
        let denominator = u.0 * d.1 - u.1 * d.0;
        if denominator.abs() < f64::EPSILON {
            return distance(a, self.centre);
        }
        (w.0 * d.1 - w.1 * d.0) / denominator
    }
}

/// A turn at "time" t (0 at the start, 1 on the boundary): in each direction, t of the way
/// out along a blend of the inner circle (radius `inner`) and the boundary, the boundary's
/// share growing with t. Inner turns are round, the last ones follow the boundary.
struct Morph<'a> {
    polar: &'a Polar,
    inner: f64,
}

impl Morph<'_> {
    fn radius(&self, theta: f64, t: f64) -> f64 {
        t * ((1.0 - t) * self.inner + t * self.polar.radius(theta))
    }

    fn point(&self, theta: f64, t: f64) -> Point {
        let r = self.radius(theta, t);
        let c = self.polar.centre;
        (c.0 + r * theta.cos(), c.1 + r * theta.sin())
    }
}

/// The spiral's raw points and how many make up the opening (first turn).
struct Raw {
    points: Vec<Point>,
    opening: usize,
}

/// How far apart two turns may be where the tool centre runs on a curve of radius `bend`
/// with the material outside it, so that the engagement angle stays `theta0`: the material
/// edge is the last turn's cut, radius bend − a + r, and the tool's edge meets it at
/// cos θ = ((bend − a + r)² − bend² − r²) / (2 · bend · r).
fn allowed_step(bend: f64, tool_radius: f64, theta0: f64) -> f64 {
    let r = tool_radius;
    let reach = (bend * bend + r * r + 2.0 * bend * r * theta0.cos()).sqrt();
    (bend + r - reach).max(0.0)
}

/// Radius of the circle through three points; infinite if they are in line. Positive when
/// they turn left (counter-clockwise).
fn bend_radius(a: Point, b: Point, c: Point) -> f64 {
    let cross = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    if cross.abs() < f64::EPSILON {
        return f64::INFINITY;
    }
    distance(a, b) * distance(b, c) * distance(c, a) / (2.0 * cross)
}

/// The spiral's points, turn by turn, each turn as far from the last as the engagement
/// angle allows in its tightest direction (at most the stepover), then once round the
/// boundary.
fn turns(morph: &Morph<'_>, width: f64, tool_radius: f64, theta0: f64, ccw: bool) -> Raw {
    let sign = if ccw { 1.0 } else { -1.0 };
    let per_turn = MIN_POINTS_PER_TURN;
    let centre = morph.polar.centre;
    // Start one stepover out (or less in a small area).
    let mut t = (width / morph.inner).min(1.0);
    let mut points = vec![centre];
    let mut opening = 0;
    for turn in 0..MAX_TURNS {
        let next = if t >= 1.0 {
            1.0
        } else {
            next_time(morph, t, width, tool_radius, theta0)
        };
        let at = |f: f64| morph.point(sign * TAU * f, t + (next - t) * f);
        for k in 0..per_turn {
            let (f0, f1) = (k as f64 / per_turn as f64, (k + 1) as f64 / per_turn as f64);
            // Where the turn runs fast (along a side seen at a grazing angle), more points.
            let pieces = (distance(at(f0), at(f1)) / POINT_SPACING_MM)
                .ceil()
                .max(1.0) as usize;
            for j in 0..pieces {
                points.push(at(f0 + (f1 - f0) * j as f64 / pieces as f64));
            }
        }
        if turn == 0 {
            opening = points.len();
        }
        if t >= 1.0 {
            // Back to where the last turn started: the boundary is closed.
            points.push(morph.point(0.0, 1.0));
            break;
        }
        t = next;
    }
    Raw { points, opening }
}

/// The time of the next turn: the largest step (up to 1) whose distance from this turn, in
/// every direction, is within what the engagement angle allows there.
fn next_time(morph: &Morph<'_>, t: f64, width: f64, tool_radius: f64, theta0: f64) -> f64 {
    let directions: Vec<f64> = (0..DIRECTIONS)
        .map(|k| TAU * k as f64 / DIRECTIONS as f64)
        .collect();
    let small = TAU / DIRECTIONS as f64;
    // What may be cut in each direction: the stepover, less on tight convex bends.
    let allowed: Vec<f64> = directions
        .iter()
        .map(|&theta| {
            let bend = bend_radius(
                morph.point(theta - small, t),
                morph.point(theta, t),
                morph.point(theta + small, t),
            );
            let step = if bend > 0.0 && bend.is_finite() {
                allowed_step(bend, tool_radius, theta0).min(width)
            } else {
                width
            };
            step.max(MIN_STEP_SHARE * width)
        })
        .collect();
    let fits = |next: f64| {
        directions
            .iter()
            .zip(&allowed)
            .all(|(&theta, &a)| morph.radius(theta, next) - morph.radius(theta, t) <= a)
    };
    if fits(1.0) {
        return 1.0;
    }
    // Bisection on the step: fits(t) is true (no step).
    let (mut lo, mut hi) = (t, 1.0);
    for _ in 0..50 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    // Never stand still: at least a tiny step.
    lo.max(t + CHAIN_TOL_MM / morph.polar.nearest.max(1.0))
        .min(1.0)
}

/// The points fitted with arcs within `ARC_APPROX_TOL_MM / 2`: each arc runs through three
/// of the points (its ends exactly), and is written as chords on it within the arc
/// tolerance, tagged in `arcs`. Returns the new points and where the opening now ends.
fn fit_arcs(points: &[Point], opening: usize, arcs: &mut ArcTable) -> (Vec<Point>, usize) {
    let tol = ARC_APPROX_TOL_MM / 2.0;
    let n = points.len();
    let mut out = vec![points[0]];
    let mut new_opening = 1;
    let mut i = 0;
    while i + 1 < n {
        // The furthest j that one arc (or line) through points i..=j fits.
        let mut good = (i + 1).min(n - 1);
        let mut step = 2;
        while i + step < n && fits(&points[i..=i + step], tol) {
            good = i + step;
            step *= 2;
        }
        let (mut lo, mut hi) = (good, (i + step).min(n - 1));
        while hi > lo + 1 {
            let mid = (lo + hi) / 2;
            if fits(&points[i..=mid], tol) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let j = if hi < n && hi > lo && fits(&points[i..=hi], tol) {
            hi
        } else {
            lo
        };
        emit(&points[i..=j], &mut out, arcs);
        if j <= opening {
            new_opening = out.len();
        }
        i = j;
    }
    (out, new_opening)
}

/// The circle through the first, middle and last point, with the sweep from first to last
/// through the middle; None when they are (nearly) in line.
fn arc_through(run: &[Point]) -> Option<(Circle, f64, f64)> {
    let (a, b, c) = (run[0], run[run.len() / 2], run[run.len() - 1]);
    let d = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
    if d.abs() < f64::EPSILON {
        return None;
    }
    let sq = |p: Point| p.0 * p.0 + p.1 * p.1;
    let ux = (sq(a) * (b.1 - c.1) + sq(b) * (c.1 - a.1) + sq(c) * (a.1 - b.1)) / d;
    let uy = (sq(a) * (c.0 - b.0) + sq(b) * (a.0 - c.0) + sq(c) * (b.0 - a.0)) / d;
    let center = (ux, uy);
    let radius = distance(center, a);
    let angle = |p: Point| (p.1 - uy).atan2(p.0 - ux);
    let ccw = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0) > 0.0;
    let (sa, sc) = (angle(a), angle(c));
    let sweep = if ccw {
        (sc - sa).rem_euclid(TAU)
    } else {
        -(sa - sc).rem_euclid(TAU)
    };
    Some((Circle { center, radius }, sa, sweep))
}

/// Whether one arc (under half a circle) or one line holds every point of the run within
/// `tol`.
fn fits(run: &[Point], tol: f64) -> bool {
    if run.len() <= 2 {
        return true;
    }
    if near_line(run, tol) {
        return true;
    }
    match arc_through(run) {
        Some((circle, _, sweep)) => {
            sweep.abs() < PI
                && run
                    .iter()
                    .all(|&p| (distance(p, circle.center) - circle.radius).abs() <= tol)
        }
        None => false,
    }
}

/// Whether every point of the run is within `tol` of the line from its first to its last.
fn near_line(run: &[Point], tol: f64) -> bool {
    let chord = Primitive::Line(Line {
        start: run[0],
        end: run[run.len() - 1],
    });
    run.iter().all(|&p| point_distance(&chord, p) <= tol)
}

/// Write the run as a line or as chords on its arc (tagged), after its first point.
fn emit(run: &[Point], out: &mut Vec<Point>, arcs: &mut ArcTable) {
    let (a, c) = (run[0], run[run.len() - 1]);
    let line = run.len() <= 2 || near_line(run, ARC_APPROX_TOL_MM / 2.0);
    let arc = if line { None } else { arc_through(run) };
    let Some((circle, start, sweep)) = arc else {
        out.push(c);
        return;
    };
    let pieces = (sweep.abs() / max_segment_angle(circle.radius, ARC_APPROX_TOL_MM))
        .ceil()
        .max(1.0) as usize;
    let mut previous = a;
    for k in 1..=pieces {
        let p = if k == pieces {
            c
        } else {
            let angle = start + sweep * k as f64 / pieces as f64;
            (
                circle.center.0 + circle.radius * angle.cos(),
                circle.center.1 + circle.radius * angle.sin(),
            )
        };
        arcs.insert(previous, p, circle);
        out.push(p);
        previous = p;
    }
}

/// The engagement angle of every step along `points` (after the first `skip`), cut on
/// `grid` with the model's `reach`, as a share of `theta0`; the grid is cut as it goes.
pub fn measure(
    grid: &mut Grid,
    points: &[Point],
    skip: usize,
    reach: f64,
    theta0: f64,
) -> Vec<f64> {
    let h = grid.cell_side();
    // The edge as the model cuts it: what the last step cleared lies inside it (except a
    // sliver), so what is uncut there is what this step's edge meets.
    let edge = reach;
    let mut shares = Vec::new();
    for (i, pair) in points.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        let n = (distance(a, b) / h).ceil().max(1.0) as usize;
        for k in 1..=n {
            let t = k as f64 / n as f64;
            let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            if i >= skip {
                let engaged = (0..EDGE_SAMPLES)
                    .filter(|&s| {
                        let angle = TAU * s as f64 / EDGE_SAMPLES as f64;
                        grid.is_set((p.0 + edge * angle.cos(), p.1 + edge * angle.sin()))
                    })
                    .count();
                shares.push(TAU * engaged as f64 / EDGE_SAMPLES as f64 / theta0);
            }
            grid.clear_disk(p, reach);
        }
    }
    shares
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_angle_of_a_straight_cut() {
        assert!((target_angle(0.6, 3.0).to_degrees() - 36.87).abs() < 0.01);
        assert!((target_angle(3.0, 3.0).to_degrees() - 90.0).abs() < 1e-9);
    }

    #[test]
    fn the_allowed_step_is_the_stepover_on_a_straight_cut_and_less_on_a_bend() {
        let theta0 = target_angle(0.6, 3.0);
        assert!((allowed_step(1e9, 3.0, theta0) - 0.6).abs() < 1e-4);
        assert!(allowed_step(3.0, 3.0, theta0) < 0.6);
        assert!(allowed_step(1.0, 3.0, theta0) < allowed_step(3.0, 3.0, theta0));
    }
}
