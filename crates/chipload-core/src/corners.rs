// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounded corners of a polyline kept as radii (docs/specs/polyline-radii.md §3,
//! fillet-between-shapes.md §4): the points stay at the sharp corners, and a corner with a
//! radius r is cut by the arc of radius r tangent to both its sides. Between two straight
//! sides it touches each r · tan(θ / 2) back from the corner, θ the angle the path turns
//! there; beside an arc it is the round of [`round`], the arc cut back along its own circle.

pub mod round;

use crate::Point;
use crate::math;
use crate::primitives::{Arc, Line, Primitive, distance};
use crate::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};
use round::{Curve, most_that_fits, nearest_round};
use std::f64::consts::PI;

/// A polyline: its points, one bulge per segment (0 straight), and whether it closes.
#[derive(Debug, Clone, Copy)]
pub struct Path<'a> {
    pub points: &'a [Point],
    pub bulges: &'a [f64],
    pub closed: bool,
}

/// A corner's round: where it touches the incoming and the outgoing side, how far back
/// along each (along an arc side, the arc's length), and its arc.
#[derive(Debug, Clone, Copy)]
struct Corner {
    in_at: Point,
    out_at: Point,
    back_in: f64,
    back_out: f64,
    arc: Option<Arc>,
}

impl Path<'_> {
    fn n(&self) -> usize {
        self.points.len()
    }

    /// How many segments it has.
    pub fn segment_count(&self) -> usize {
        match (self.n(), self.closed) {
            (0 | 1, _) => 0,
            (n, true) => n,
            (n, false) => n - 1,
        }
    }

    fn bulge(&self, segment: usize) -> f64 {
        self.bulges.get(segment).copied().unwrap_or(0.0)
    }

    fn straight(&self, segment: usize) -> bool {
        self.bulge(segment) == 0.0
    }

    /// The segments before and after point `k`, when it has both (an open path's ends
    /// don't).
    fn sides(&self, k: usize) -> Option<(usize, usize)> {
        let n = self.n();
        if n < 2 || k >= n {
            return None;
        }
        if self.closed {
            if n < 3 {
                return None;
            }
            Some(((k + n - 1) % n, k))
        } else if k == 0 || k + 1 == n {
            None
        } else {
            Some((k - 1, k))
        }
    }

    /// Segment `i` as drawn, before any corner is rounded; None without length.
    fn segment(&self, i: usize) -> Option<Primitive> {
        let n = self.n();
        let (a, b) = (*self.points.get(i)?, *self.points.get((i + 1) % n)?);
        if distance(a, b) < CHAIN_TOL_MM {
            return None;
        }
        let bulge = self.bulge(i);
        if bulge == 0.0 {
            Some(Primitive::Line(Line { start: a, end: b }))
        } else {
            Arc::from_bulge(a, b, bulge).map(Primitive::Arc)
        }
    }

    /// The direction segment `i` runs at its start (or end), as an angle: an arc turning
    /// counter-clockwise leaves its chord half its sweep to the right and meets it again
    /// as far to the left.
    fn direction(&self, i: usize, at_end: bool) -> f64 {
        let n = self.n();
        let (a, b) = (self.points[i], self.points[(i + 1) % n]);
        let chord = math::atan2(b.1 - a.1, b.0 - a.0);
        let half = 2.0 * math::atan(self.bulge(i));
        if at_end { chord + half } else { chord - half }
    }

    /// The angle the path turns at point `k` (counter-clockwise positive), when it is a
    /// corner that turns (not tangent, not straight back); None otherwise. Beside an arc
    /// the arc's direction at the point counts.
    pub fn turn(&self, k: usize) -> Option<f64> {
        let (before, after) = self.sides(k)?;
        self.segment(before)?;
        self.segment(after)?;
        let turn = if self.straight(before) && self.straight(after) {
            let n = self.n();
            let (a, p, b) = (
                self.points[(k + n - 1) % n],
                self.points[k],
                self.points[(k + 1) % n],
            );
            let (ux, uy) = (p.0 - a.0, p.1 - a.1);
            let (vx, vy) = (b.0 - p.0, b.1 - p.1);
            math::atan2(ux * vy - uy * vx, ux * vx + uy * vy)
        } else {
            wrap(self.direction(after, false) - self.direction(before, true))
        };
        (turn.abs() > ANGLE_TOL_RAD && turn.abs() < PI - ANGLE_TOL_RAD).then_some(turn)
    }

    /// The round of radius `r` at point `k`; None where there is no corner or no round
    /// touches both sides within them.
    fn corner(&self, k: usize, r: f64) -> Option<Corner> {
        let turn = self.turn(k)?;
        let (before, after) = self.sides(k)?;
        let n = self.n();
        let p = self.points[k];
        if self.straight(before) && self.straight(after) {
            let d = r * math::tan(turn.abs() / 2.0);
            let (a, b) = (self.points[(k + n - 1) % n], self.points[(k + 1) % n]);
            let back = |q: Point| {
                let len = distance(p, q);
                (p.0 + (q.0 - p.0) / len * d, p.1 + (q.1 - p.1) / len * d)
            };
            let (in_at, out_at) = (back(a), back(b));
            return Some(Corner {
                in_at,
                out_at,
                back_in: d,
                back_out: d,
                arc: Arc::from_bulge(in_at, out_at, math::tan(turn / 4.0)),
            });
        }
        let (incoming, outgoing) = (self.segment(before)?, self.segment(after)?);
        // The round lies on the inside of the turn: left of both sides' travel for a left
        // turn. Left of an arc's travel is its inside when it runs counter-clockwise.
        let side = |segment: &Primitive| match segment {
            Primitive::Line(_) => turn.signum(),
            Primitive::Arc(arc) => -turn.signum() * arc.sweep.signum(),
        };
        let round = nearest_round(
            &Curve::of(&incoming),
            side(&incoming),
            &Curve::of(&outgoing),
            side(&outgoing),
            r,
            p,
        )?;
        let back_in = incoming.length() - along(&incoming, round.first)?;
        let back_out = along(&outgoing, round.second)?;
        let angle = |q: Point| math::atan2(q.1 - round.center.1, q.0 - round.center.0);
        let (a0, a1) = (angle(round.first), angle(round.second));
        let sweep = if turn > 0.0 {
            (a1 - a0).rem_euclid(2.0 * PI)
        } else {
            -(a0 - a1).rem_euclid(2.0 * PI)
        };
        Some(Corner {
            in_at: round.first,
            out_at: round.second,
            back_in,
            back_out,
            arc: (sweep.abs() > ANGLE_TOL_RAD && r >= CHAIN_TOL_MM).then_some(Arc {
                center: round.center,
                radius: r,
                start_angle: a0,
                sweep,
            }),
        })
    }

    /// How far back along each side the round of radius `r` at point `k` reaches; 0 where
    /// there is none.
    fn setbacks(&self, k: usize, r: f64) -> (f64, f64) {
        if r <= 0.0 {
            return (0.0, 0.0);
        }
        self.corner(k, r)
            .map_or((0.0, 0.0), |c| (c.back_in, c.back_out))
    }
}

/// An angle brought into (−π, π].
fn wrap(mut angle: f64) -> f64 {
    while angle > PI {
        angle -= 2.0 * PI;
    }
    while angle <= -PI {
        angle += 2.0 * PI;
    }
    angle
}

/// How far along `segment` from its start `p` lies (`p` on its line or circle); None when
/// it lies outside the segment by more than the chaining tolerance.
fn along(segment: &Primitive, p: Point) -> Option<f64> {
    let len = segment.length();
    let s = match segment {
        Primitive::Line(l) => {
            let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
            ((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / len
        }
        Primitive::Arc(arc) => {
            let angle = math::atan2(p.1 - arc.center.1, p.0 - arc.center.0);
            let u = ((angle - arc.start_angle) * arc.sweep.signum()).rem_euclid(2.0 * PI);
            let s = u * arc.radius;
            // Just before its start reads as almost a whole turn on.
            if s > len && (2.0 * PI - u) * arc.radius < CHAIN_TOL_MM {
                0.0
            } else {
                s
            }
        }
    };
    (-CHAIN_TOL_MM..=len + CHAIN_TOL_MM)
        .contains(&s)
        .then_some(s.clamp(0.0, len))
}

/// Why a corner's radius can't be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RadiusError {
    /// Point `k` is an open path's end: it has no corner.
    End(usize),
    /// Point `k`'s sides meet tangentially, or run straight on or straight back.
    NotACorner(usize),
    /// The radius at point `k` needs more of a side than there is (or no round of it
    /// touches both); the most that fits, with the other corners' radii as they are.
    TooLarge { at: usize, most: f64 },
}

/// The problems with `radii` (one per point, 0 sharp) on `path`, point by point.
pub fn check(path: Path, radii: &[f64]) -> Vec<RadiusError> {
    let mut problems = Vec::new();
    for (k, &r) in radii.iter().enumerate() {
        if r == 0.0 {
            continue;
        }
        match (path.sides(k), path.turn(k)) {
            (None, _) => problems.push(RadiusError::End(k)),
            (Some(_), None) => problems.push(RadiusError::NotACorner(k)),
            (Some(_), Some(_)) => {
                let most = most_radius(path, radii, k);
                if r > most + CHAIN_TOL_MM {
                    problems.push(RadiusError::TooLarge { at: k, most });
                }
            }
        }
    }
    problems
}

/// The largest radius corner `k` can take, the other corners keeping theirs: on each of
/// its sides, what the other corner leaves. Between straight sides it is worked out
/// directly; beside an arc it is searched for (fillet-between-shapes.md §2).
pub fn most_radius(path: Path, radii: &[f64], k: usize) -> f64 {
    let (Some((before, after)), Some(turn)) = (path.sides(k), path.turn(k)) else {
        return 0.0;
    };
    let n = path.n();
    let other = |i: usize| radii.get(i % n).copied().unwrap_or(0.0);
    let (prev, next) = ((k + n - 1) % n, (k + 1) % n);
    let length = |segment: usize| path.segment(segment).map_or(0.0, |s| s.length());
    // What the neighbours leave of each side.
    let room_before = length(before) - path.setbacks(prev, other(prev)).1;
    let room_after = length(after) - path.setbacks(next, other(next)).0;
    if path.straight(before) && path.straight(after) {
        let tan = math::tan(turn.abs() / 2.0);
        return room_before.min(room_after).max(0.0) / tan;
    }
    let fits = |r: f64| {
        path.corner(k, r).is_some_and(|c| {
            c.back_in <= room_before + CHAIN_TOL_MM && c.back_out <= room_after + CHAIN_TOL_MM
        })
    };
    most_that_fits(length(before).max(length(after)), fits)
}

/// `path` as lines and arcs, each corner with a radius in `radii` rounded (one per point;
/// empty or all 0: none). A radius where there is no corner, or no round fits, is left
/// out; one too large is cut as asked (validation refuses it first).
pub fn segments(path: Path, radii: &[f64]) -> Vec<Primitive> {
    let n = path.n();
    let count = path.segment_count();
    let radius = |k: usize| radii.get(k).copied().unwrap_or(0.0);
    let corners: Vec<Option<Corner>> = (0..n)
        .map(|k| {
            let r = radius(k);
            (r > 0.0).then(|| path.corner(k, r)).flatten()
        })
        .collect();
    let mut out = Vec::new();
    for i in 0..count {
        let j = (i + 1) % n;
        let (from, to) = (corners[i], corners[j]);
        let bulge = path.bulge(i);
        let piece = if bulge == 0.0 {
            let start = from.map_or(path.points[i], |c| c.out_at);
            let end = to.map_or(path.points[j], |c| c.in_at);
            (distance(start, end) > CHAIN_TOL_MM).then_some(Primitive::Line(Line { start, end }))
        } else {
            Arc::from_bulge(path.points[i], path.points[j], bulge)
                .and_then(|arc| cut_back(arc, from.map(|c| c.back_out), to.map(|c| c.back_in)))
                .map(Primitive::Arc)
        };
        out.extend(piece);
        // The round at the corner this segment runs into (an open path's last point has
        // none).
        if let Some(arc) = to.and_then(|c| c.arc) {
            out.push(Primitive::Arc(arc));
        }
    }
    out
}

/// An arc side cut back along its circle by the rounds at its start and end; the arc as
/// it is when neither is rounded, None when nothing is left.
fn cut_back(arc: Arc, start: Option<f64>, end: Option<f64>) -> Option<Arc> {
    if start.is_none() && end.is_none() {
        return Some(arc);
    }
    let (s, e) = (start.unwrap_or(0.0), end.unwrap_or(0.0));
    let left = arc.sweep.abs() * arc.radius - s - e;
    if left <= CHAIN_TOL_MM {
        return None;
    }
    let dir = arc.sweep.signum();
    Some(Arc {
        start_angle: arc.start_angle + dir * s / arc.radius,
        sweep: dir * left / arc.radius,
        ..arc
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-9
    }

    const SQUARE: [Point; 4] = [(0.0, 0.0), (100.0, 0.0), (100.0, 60.0), (0.0, 60.0)];

    #[test]
    fn a_rounded_corner_is_an_arc_tangent_to_both_sides() {
        let path = Path {
            points: &SQUARE,
            bulges: &[0.0; 4],
            closed: true,
        };
        let segs = segments(path, &[0.0, 5.0, 5.0, 0.0]);
        // Four sides and two rounds.
        assert_eq!(segs.len(), 6);
        assert!(close(segs[0].start(), (0.0, 0.0)) && close(segs[0].end(), (95.0, 0.0)));
        let Primitive::Arc(round) = segs[1] else {
            panic!("the round at (100, 0)");
        };
        assert!(close(round.center, (95.0, 5.0)) && (round.radius - 5.0).abs() < 1e-9);
        assert!((round.sweep - FRAC_PI_2).abs() < 1e-9);
        assert!(close(segs[1].end(), (100.0, 5.0)));
        // Joined end to end, back to the start.
        for pair in segs.windows(2) {
            assert!(close(pair[0].end(), pair[1].start()));
        }
        assert!(close(segs[5].end(), segs[0].start()));
        // All radii 0: the plain sides.
        assert_eq!(segments(path, &[]).len(), 4);
    }

    #[test]
    fn acute_and_obtuse_corners_touch_their_sides_where_the_angle_says() {
        // A right turn of 120° (an acute corner of 60°): r · tan(60°) back.
        let points = [(0.0, 0.0), (50.0, 0.0), (25.0, -25.0 * 3f64.sqrt())];
        let path = Path {
            points: &points,
            bulges: &[0.0, 0.0],
            closed: false,
        };
        let turn = path.turn(1).unwrap();
        assert!((turn + 2.0 * PI / 3.0).abs() < 1e-9, "clockwise");
        let segs = segments(path, &[0.0, 2.0, 0.0]);
        let d = 2.0 * 3f64.sqrt();
        assert!(close(segs[0].end(), (50.0 - d, 0.0)));
        let Primitive::Arc(round) = segs[1] else {
            panic!("a round");
        };
        assert!(round.sweep < 0.0 && (round.radius - 2.0).abs() < 1e-9);
        // The arc's centre is r from both sides.
        assert!((round.center.1 + 2.0).abs() < 1e-9);
    }

    #[test]
    fn radii_are_checked_at_ends_beside_arcs_and_for_room() {
        let open = Path {
            points: &SQUARE,
            bulges: &[0.0, 0.5, 0.0],
            closed: false,
        };
        assert_eq!(check(open, &[1.0, 0.0, 0.0, 0.0]), [RadiusError::End(0)]);
        // Beside an arc is a corner now (fillet-between-shapes.md §4).
        assert!(check(open, &[0.0, 1.0, 0.0, 0.0]).is_empty());
        // A line running on into a half circle tangentially has no corner.
        let tangent = Path {
            points: &[(0.0, 0.0), (40.0, 0.0), (40.0, 40.0)],
            bulges: &[0.0, 1.0],
            closed: false,
        };
        assert_eq!(
            check(tangent, &[0.0, 1.0, 0.0]),
            [RadiusError::NotACorner(1)]
        );
        let closed = Path {
            points: &SQUARE,
            bulges: &[0.0; 4],
            closed: true,
        };
        // Two corners on the 60 side: 30 each fits, 31 beside 30 doesn't.
        assert!(check(closed, &[0.0, 30.0, 30.0, 0.0]).is_empty());
        let most = most_radius(closed, &[0.0, 31.0, 30.0, 0.0], 1);
        assert!((most - 30.0).abs() < 1e-9);
        assert_eq!(
            check(closed, &[0.0, 31.0, 0.0, 0.0]),
            Vec::<RadiusError>::new(),
            "alone it fits the 60 side"
        );
        assert!(matches!(
            check(closed, &[0.0, 70.0, 0.0, 0.0])[..],
            [RadiusError::TooLarge { at: 1, .. }]
        ));
        // Two corners meeting exactly: the side between them vanishes, no zero-length line.
        let segs = segments(closed, &[0.0, 30.0, 30.0, 0.0]);
        assert_eq!(segs.len(), 5);
    }

    /// A line into an arc (bulge 0.5: the path turns left 37° at the corner), and an arc into a
    /// line: the round is tangent to both, the arc cut back along its own circle.
    fn bracket() -> ([Point; 4], [f64; 4]) {
        (
            [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)],
            [0.0, 0.5, 0.0, 0.0],
        )
    }

    #[test]
    fn a_corner_beside_an_arc_is_rounded_tangent_to_both() {
        let (points, bulges) = bracket();
        let path = Path {
            points: &points,
            bulges: &bulges,
            closed: true,
        };
        let turn = path.turn(1).unwrap();
        assert!(
            (turn - (PI / 2.0 - 2.0 * math::atan(0.5))).abs() < 1e-9,
            "{turn}"
        );
        let Some(Primitive::Arc(side)) = path.segment(1) else {
            panic!("an arc side");
        };
        let segs = segments(path, &[0.0, 3.0, 4.0, 0.0]);
        assert_eq!(segs.len(), 6, "{segs:#?}");
        for pair in segs.windows(2) {
            assert!(close(pair[0].end(), pair[1].start()), "{pair:?}");
        }
        assert!(close(segs[5].end(), segs[0].start()));
        let (Primitive::Arc(round), Primitive::Arc(cut)) = (segs[1], segs[2]) else {
            panic!("a round, then the arc side");
        };
        // The round lies inside the arc's circle, r from the line and R − r from its centre.
        assert!((round.radius - 3.0).abs() < 1e-9 && round.sweep > 0.0);
        assert!((round.center.1 - 3.0).abs() < 1e-9);
        assert!((distance(round.center, side.center) - (side.radius - 3.0)).abs() < 1e-9);
        // The side keeps its circle and is shorter at both ends.
        assert!(close(cut.center, side.center) && (cut.radius - side.radius).abs() < 1e-9);
        assert!(cut.sweep > 0.0 && cut.sweep < side.sweep);
        // Arc into line at (40, 40), turning left 37° too.
        let Primitive::Arc(second) = segs[3] else {
            panic!("the round at (40, 40)");
        };
        assert!((second.radius - 4.0).abs() < 1e-9);
        assert!((second.center.1 - 36.0).abs() < 1e-9);
    }

    #[test]
    fn beside_an_arc_the_most_that_fits_is_searched_for() {
        let (points, bulges) = bracket();
        let path = Path {
            points: &points,
            bulges: &bulges,
            closed: true,
        };
        let most = most_radius(path, &[0.0, 0.0, 0.0, 0.0], 1);
        assert!(most > 3.0, "{most}");
        assert!(check(path, &[0.0, most, 0.0, 0.0]).is_empty());
        assert!(matches!(
            check(path, &[0.0, most + 0.01, 0.0, 0.0])[..],
            [RadiusError::TooLarge { at: 1, .. }]
        ));
        // Both ends of the arc side rounded share it.
        let shared = most_radius(path, &[0.0, 0.0, most, 0.0], 1);
        assert!(shared < most, "{shared} < {most}");
    }

    #[test]
    fn between_straight_sides_the_general_round_is_the_same_arc() {
        let (a, p, b) = ((0.0, 0.0), (50.0, 0.0), (20.0, 30.0));
        let path = Path {
            points: &[a, p, b],
            bulges: &[0.0, 0.0],
            closed: false,
        };
        let Primitive::Arc(direct) = segments(path, &[0.0, 5.0, 0.0])[1] else {
            panic!("a round");
        };
        let general = nearest_round(
            &Curve::Line { a, b: p },
            1.0,
            &Curve::Line { a: p, b },
            1.0,
            5.0,
            p,
        )
        .unwrap();
        assert!(distance(general.center, direct.center) < 1e-9);
    }
}
