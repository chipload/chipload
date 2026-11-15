// SPDX-License-Identifier: GPL-3.0-or-later
//! Flute (docs/specs/fluting.md): a groove on the line of open shapes whose depth ramps in
//! from the shape's first end, runs at full depth, and ramps out to its last end. Each pass
//! is that profile cut off at its own level, back and forth along the shape.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{FluteOp, RampShape, Tool};

use crate::arcs::{ArcTable, Circle};
use crate::model::Move;
use crate::path::open_points;

/// The most points a curved ramp is cut into, whatever its depth and length.
const MAX_RAMP_PIECES: usize = 4096;

/// Steps of the bisection for a flute's width at the top.
const WIDTH_STEPS: usize = 60;

/// A flute's depth along one shape, mm below its start depth, by the distance `s` from the
/// shape's first end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Profile {
    pub length: f64,
    pub depth: f64,
    /// The ramps as cut: shortened in proportion when they don't fit together.
    pub ramp_start: f64,
    pub ramp_end: f64,
    pub shape: RampShape,
}

impl Profile {
    /// The profile of a shape `length` long; and whether its ramps were shortened to fit.
    pub fn new(op: &FluteOp, depth: f64, length: f64) -> (Profile, bool) {
        Profile::with(depth, length, (op.ramp_start, op.ramp_end), op.ramp_shape)
    }

    /// The profile of a shape `length` long with (start, end) `ramps` of `shape`; and
    /// whether the ramps were shortened to fit.
    pub fn with(depth: f64, length: f64, ramps: (f64, f64), shape: RampShape) -> (Profile, bool) {
        let (mut start, mut end) = (ramps.0.max(0.0), ramps.1.max(0.0));
        let total = start + end;
        let shortened = total > length + CHAIN_TOL_MM;
        if shortened {
            (start, end) = (length * start / total, length * end / total);
        }
        let profile = Profile {
            length,
            depth,
            ramp_start: start,
            ramp_end: end,
            shape,
        };
        (profile, shortened)
    }

    /// The fraction of the depth a ramp reaches at `u` of its length (0 at the end, 1 at
    /// full depth).
    fn ramp(&self, u: f64) -> f64 {
        let u = u.clamp(0.0, 1.0);
        match self.shape {
            RampShape::Straight => u,
            RampShape::Curved => math::sin(std::f64::consts::FRAC_PI_2 * u),
        }
    }

    /// Where a ramp reaches `fraction` of the depth, as `u` of its length.
    fn ramp_inverse(&self, fraction: f64) -> f64 {
        let f = fraction.clamp(0.0, 1.0);
        match self.shape {
            RampShape::Straight => f,
            RampShape::Curved => math::asin(f) / std::f64::consts::FRAC_PI_2,
        }
    }

    /// The depth at `s` mm from the first end, below the start depth.
    pub fn depth_at(&self, s: f64) -> f64 {
        let into_start = if self.ramp_start > 0.0 {
            self.ramp(s / self.ramp_start)
        } else {
            1.0
        };
        let into_end = if self.ramp_end > 0.0 {
            self.ramp((self.length - s) / self.ramp_end)
        } else {
            1.0
        };
        self.depth * into_start.min(into_end)
    }

    /// The distances along the shape where the profile needs a point besides the shape's
    /// own: the ramps' ends, where each level cuts a ramp, and (curved) enough points that
    /// the depth between them is within the arc tolerance of the curve.
    fn breaks(&self, levels: &[f64]) -> Vec<f64> {
        let mut at = vec![self.ramp_start, self.length - self.ramp_end];
        let ramps = [(self.ramp_start, false), (self.ramp_end, true)];
        for (ramp, from_end) in ramps {
            if ramp <= 0.0 {
                continue;
            }
            let place = |u: f64| {
                if from_end {
                    self.length - u * ramp
                } else {
                    u * ramp
                }
            };
            for &level in levels {
                if level < self.depth {
                    at.push(place(self.ramp_inverse(level / self.depth)));
                }
            }
            if self.shape == RampShape::Curved {
                // Linear pieces under the curve: the error is at most h²/8 times its
                // steepest bend, depth × (π / 2 / ramp)².
                let bend = self.depth * (std::f64::consts::FRAC_PI_2 / ramp).powi(2);
                let piece = (8.0 * ARC_APPROX_TOL_MM / bend).sqrt();
                let count = ((ramp / piece).ceil() as usize).clamp(1, MAX_RAMP_PIECES);
                for k in 1..count {
                    at.push(place(k as f64 / count as f64));
                }
            }
        }
        at.retain(|s| *s > CHAIN_TOL_MM && *s < self.length - CHAIN_TOL_MM);
        at.sort_by(f64::total_cmp);
        at
    }
}

/// One shape to flute: its points from the first drawn end, each with its distance along
/// the shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Flute {
    pub contour_id: usize,
    pub points: Vec<Point>,
    pub along: Vec<f64>,
    pub profile: Profile,
}

/// The selected shapes as flutes, drawn order, with the profile's points added (on the
/// circle where they fall on an arc, so arcs stay arcs). Closed and too short shapes are
/// left out with a note; shortened ramps get a note.
pub fn flutes(
    op: &FluteOp,
    depth: f64,
    levels: &[f64],
    contours: &[&Contour],
    arcs: &mut ArcTable,
    notes: &mut Vec<Message>,
) -> Vec<Flute> {
    let mut found = Vec::new();
    for contour in contours {
        if contour.closed {
            notes.push(Message::new("toolpath-flute-closed").arg("contour", contour.id));
            continue;
        }
        let points = open_points(contour, arcs);
        let along = distances(&points);
        let length = along.last().copied().unwrap_or(0.0);
        if points.len() < 2 || length <= CHAIN_TOL_MM {
            notes.push(
                Message::new("toolpath-path-too-short")
                    .arg("contour", contour.id)
                    .arg("length", Value::Length(length, Digits::Fixed(4))),
            );
            continue;
        }
        let (profile, shortened) = Profile::new(op, depth, length);
        if shortened {
            notes.push(
                Message::new("toolpath-flute-ramps-shortened")
                    .arg("contour", contour.id)
                    .arg("length", Value::Length(length, Digits::Trim(2))),
            );
        }
        let (points, along) = with_breaks(&points, &along, &profile.breaks(levels), arcs);
        found.push(Flute {
            contour_id: contour.id,
            points,
            along,
            profile,
        });
    }
    found
}

impl Flute {
    /// A groove along `points` (straight segments, none on an arc) with straight ramps of
    /// `ramp` at both ends, the profile's points added: what a pattern cuts
    /// (docs/specs/fill-pattern.md §4).
    pub fn of_points(
        contour_id: usize,
        points: Vec<Point>,
        depth: f64,
        ramp: f64,
        levels: &[f64],
    ) -> Flute {
        let along = distances(&points);
        let length = along.last().copied().unwrap_or(0.0);
        let (profile, _) = Profile::with(depth, length, (ramp, ramp), RampShape::Straight);
        let mut none = ArcTable::default();
        let (points, along) = with_breaks(&points, &along, &profile.breaks(levels), &mut none);
        Flute {
            contour_id,
            points,
            along,
            profile,
        }
    }
}

/// The distance along `points` at each point.
fn distances(points: &[Point]) -> Vec<f64> {
    let mut along = Vec::with_capacity(points.len());
    let mut total = 0.0;
    let mut last = points.first().copied();
    for &p in points {
        if let Some(l) = last {
            total += distance(l, p);
        }
        along.push(total);
        last = Some(p);
    }
    along
}

/// `points` with a point added at each of `breaks` (distances along it), further than the
/// chaining tolerance from the points there. A point on an arc's chord goes onto the
/// circle, and both halves are recorded as chords of it.
fn with_breaks(
    points: &[Point],
    along: &[f64],
    breaks: &[f64],
    arcs: &mut ArcTable,
) -> (Vec<Point>, Vec<f64>) {
    let mut out_points = Vec::with_capacity(points.len() + breaks.len());
    let mut out_along = Vec::with_capacity(points.len() + breaks.len());
    let mut next = breaks.iter().copied().peekable();
    for k in 0..points.len() {
        if k > 0 {
            let (a, b) = (points[k - 1], points[k]);
            let (sa, sb) = (along[k - 1], along[k]);
            let circle = arcs.get(a, b);
            let mut from = a;
            while let Some(&s) = next.peek() {
                if s >= sb - CHAIN_TOL_MM {
                    break;
                }
                next.next();
                if s <= sa + CHAIN_TOL_MM
                    || out_along
                        .last()
                        .is_some_and(|l: &f64| s <= l + CHAIN_TOL_MM)
                {
                    continue;
                }
                let t = (s - sa) / (sb - sa);
                let mut p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                if let Some(c) = circle {
                    p = onto(c, p);
                    arcs.insert(from, p, c);
                }
                out_points.push(p);
                out_along.push(s);
                from = p;
            }
            if let Some(c) = circle
                && from != a
            {
                arcs.insert(from, b, c);
            }
            // Breaks at this point itself are covered by it.
            while next.peek().is_some_and(|&s| s < sb + CHAIN_TOL_MM) {
                next.next();
            }
        }
        out_points.push(points[k]);
        out_along.push(along[k]);
    }
    (out_points, out_along)
}

/// `p` moved onto the circle, along the ray from its centre.
fn onto(c: Circle, p: Point) -> Point {
    let (dx, dy) = (p.0 - c.center.0, p.1 - c.center.1);
    let d = math::hypot(dx, dy);
    if d <= 0.0 {
        return p;
    }
    (
        c.center.0 + dx * c.radius / d,
        c.center.1 + dy * c.radius / d,
    )
}

/// Everything the flutes' moves need besides their shapes.
#[derive(Debug, Clone, Copy)]
pub struct FluteCut<'a> {
    /// The Z of each pass's level, going down.
    pub pass_z: &'a [f64],
    /// The Z of the start depth.
    pub top_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
}

/// The flutes in cutting order from `start`: each time the nearest end of those left (the
/// lowest contour id on a tie); and whether each is cut from its last end.
pub fn order(flutes: Vec<Flute>, start: Point) -> Vec<(Flute, bool)> {
    let mut remaining = flutes;
    let mut ordered = Vec::with_capacity(remaining.len());
    let mut here = start;
    while !remaining.is_empty() {
        let mut best: Option<(f64, bool, usize)> = None;
        for (i, flute) in remaining.iter().enumerate() {
            let (Some(&first), Some(&last)) = (flute.points.first(), flute.points.last()) else {
                continue;
            };
            let (d_first, d_last) = (distance(here, first), distance(here, last));
            let (d, reverse) = if d_last < d_first {
                (d_last, true)
            } else {
                (d_first, false)
            };
            let better = match best {
                None => true,
                Some((bd, _, bi)) => {
                    d < bd || (d == bd && flute.contour_id < remaining[bi].contour_id)
                }
            };
            if better {
                best = Some((d, reverse, i));
            }
        }
        let Some((_, reverse, index)) = best else {
            break;
        };
        let flute = remaining.remove(index);
        let ends = (flute.points.first().copied(), flute.points.last().copied());
        let end = if reverse { ends.0 } else { ends.1 };
        here = end.unwrap_or(here);
        ordered.push((flute, reverse));
    }
    ordered
}

/// The moves for one flute, from a retract above the end it starts at to a retract above
/// the end it finishes at; and that end (docs/specs/fluting.md §4).
pub fn flute_moves(
    flute: &Flute,
    reverse: bool,
    cut: &FluteCut,
    arcs: &ArcTable,
) -> (Vec<Move>, Point) {
    let n = flute.points.len();
    let forward: Vec<usize> = if reverse {
        (0..n).rev().collect()
    } else {
        (0..n).collect()
    };
    let backward: Vec<usize> = forward.iter().rev().copied().collect();
    let first = flute.points[forward[0]];
    let mut moves = vec![Move::rapid_z(cut.safe_z), Move::rapid_xy(first)];
    let mut current = cut.safe_z;
    let mut here = first;
    for (pass, &level_z) in cut.pass_z.iter().enumerate() {
        let level = cut.top_z - level_z;
        let order = if pass % 2 == 0 { &forward } else { &backward };
        for (k, &i) in order.iter().enumerate() {
            let p = flute.points[i];
            let z = cut.top_z - flute.profile.depth_at(flute.along[i]).min(level);
            if k == 0 {
                if z != current {
                    // Down at the end: to the start depth, or a step at a square end.
                    moves.push(Move::feed_z(z, cut.plunge_feed));
                    current = z;
                }
                continue;
            }
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
    (moves, here)
}

/// How wide a flute `depth` deep is at the top with `tool`, mm (docs/specs/fluting.md §5):
/// twice the distance from the axis where the tool's surface rises to `depth`, at most its
/// diameter. None for a tool whose shape isn't known.
pub fn flute_width_mm(tool: &Tool, depth: f64) -> Option<f64> {
    let cutter = tool.cutter()?;
    let radius = cutter.radius();
    if cutter.height_at(radius)? <= depth {
        return Some(2.0 * radius);
    }
    let (mut low, mut high) = (0.0, radius);
    for _ in 0..WIDTH_STEPS {
        let middle = (low + high) / 2.0;
        if cutter.height_at(middle)? <= depth {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(2.0 * low)
}
