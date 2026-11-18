// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounded corners of a polyline kept as radii (docs/specs/polyline-radii.md §3): the points
//! stay at the sharp corners, and a corner between two straight sides with a radius r is
//! cut by the arc of radius r tangent to both, touching each r · tan(θ / 2) back from the
//! corner, θ the angle the path turns there.

use crate::Point;
use crate::math;
use crate::primitives::{Arc, Line, Primitive, distance};
use crate::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};

/// A polyline: its points, one bulge per segment (0 straight), and whether it closes.
#[derive(Debug, Clone, Copy)]
pub struct Path<'a> {
    pub points: &'a [Point],
    pub bulges: &'a [f64],
    pub closed: bool,
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

    fn straight(&self, segment: usize) -> bool {
        self.bulges.get(segment).copied().unwrap_or(0.0) == 0.0
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

    /// The angle the path turns at point `k` (counter-clockwise positive), when it is a
    /// corner between two straight sides that turns; None otherwise.
    pub fn turn(&self, k: usize) -> Option<f64> {
        let (before, after) = self.sides(k)?;
        if !self.straight(before) || !self.straight(after) {
            return None;
        }
        let n = self.n();
        let (a, p, b) = (
            self.points[(k + n - 1) % n],
            self.points[k],
            self.points[(k + 1) % n],
        );
        if distance(a, p) < CHAIN_TOL_MM || distance(p, b) < CHAIN_TOL_MM {
            return None;
        }
        let (ux, uy) = (p.0 - a.0, p.1 - a.1);
        let (vx, vy) = (b.0 - p.0, b.1 - p.1);
        let turn = math::atan2(ux * vy - uy * vx, ux * vx + uy * vy);
        (turn.abs() > ANGLE_TOL_RAD && turn.abs() < std::f64::consts::PI - ANGLE_TOL_RAD)
            .then_some(turn)
    }

    /// How far back along each side the round of radius `r` at point `k` touches it.
    fn setback(&self, k: usize, r: f64) -> f64 {
        self.turn(k)
            .map_or(0.0, |turn| r * math::tan(turn.abs() / 2.0))
    }
}

/// Why a corner's radius can't be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RadiusError {
    /// Point `k` is an open path's end: it has no corner.
    End(usize),
    /// Point `k` has an arc beside it, or its sides run straight on.
    NotACorner(usize),
    /// The radius at point `k` needs more of a side than there is; the most that fits,
    /// with the other corners' radii as they are.
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
/// its sides, what the other corner leaves.
pub fn most_radius(path: Path, radii: &[f64], k: usize) -> f64 {
    let (Some((before, after)), Some(turn)) = (path.sides(k), path.turn(k)) else {
        return 0.0;
    };
    let n = path.n();
    let other = |i: usize| radii.get(i % n).copied().unwrap_or(0.0);
    let room = |segment: usize, neighbour: usize| {
        let a = path.points[segment];
        let b = path.points[(segment + 1) % n];
        distance(a, b) - path.setback(neighbour, other(neighbour))
    };
    let tan = math::tan(turn.abs() / 2.0);
    room(before, (k + n - 1) % n)
        .min(room(after, (k + 1) % n))
        .max(0.0)
        / tan
}

/// `path` as lines and arcs, each corner with a radius in `radii` rounded (one per point;
/// empty or all 0: none). A radius where there is no corner is left out; one too large is
/// cut as asked (validation refuses it first).
pub fn segments(path: Path, radii: &[f64]) -> Vec<Primitive> {
    let n = path.n();
    let count = path.segment_count();
    let radius = |k: usize| radii.get(k).copied().unwrap_or(0.0);
    // The round at each corner: where it touches the incoming and the outgoing side, and
    // the turn.
    let rounds: Vec<Option<(Point, Point, f64)>> = (0..n)
        .map(|k| {
            let r = radius(k);
            let turn = path.turn(k).filter(|_| r > 0.0)?;
            let d = r * math::tan(turn.abs() / 2.0);
            let p = path.points[k];
            let (a, b) = (path.points[(k + n - 1) % n], path.points[(k + 1) % n]);
            let back = |q: Point| {
                let len = distance(p, q);
                (p.0 + (q.0 - p.0) / len * d, p.1 + (q.1 - p.1) / len * d)
            };
            Some((back(a), back(b), turn))
        })
        .collect();
    let mut out = Vec::new();
    for i in 0..count {
        let j = (i + 1) % n;
        let start = rounds[i].map_or(path.points[i], |(_, out_at, _)| out_at);
        let end = rounds[j].map_or(path.points[j], |(in_at, _, _)| in_at);
        let bulge = path.bulges.get(i).copied().unwrap_or(0.0);
        let piece = if bulge == 0.0 {
            (distance(start, end) > CHAIN_TOL_MM).then_some(Primitive::Line(Line { start, end }))
        } else {
            Arc::from_bulge(start, end, bulge).map(Primitive::Arc)
        };
        out.extend(piece);
        // The round at the corner this segment runs into (an open path's last point has
        // none).
        if let Some((in_at, out_at, turn)) = rounds[j]
            && let Some(arc) = Arc::from_bulge(in_at, out_at, math::tan(turn / 4.0))
        {
            out.push(Primitive::Arc(arc));
        }
    }
    out
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
        assert_eq!(
            check(open, &[0.0, 1.0, 0.0, 0.0]),
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
}
