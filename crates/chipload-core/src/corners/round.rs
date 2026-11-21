// SPDX-License-Identifier: GPL-3.0-or-later
//! A round of radius r tangent to two curves, each a line or a circle (an arc's whole
//! circle), from first principles (docs/specs/fillet-between-shapes.md §2, §3): its centre
//! is where the two curves, offset by r towards it, cross; it touches each curve at the
//! centre's foot on it. Radius 0 gives the point where the curves meet.

use crate::Point;
use crate::math;
use crate::primitives::{Arc, Primitive, distance};
use crate::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM, SAME_CIRCLE_TOL_MM};
use std::f64::consts::PI;

/// How far the search for the most that fits goes, mm: a round larger than a kilometre is
/// no round a router cuts.
const SEARCH_LIMIT_MM: f64 = 1.0e6;

/// A curve a round can touch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    /// The line through `a` and `b` (apart), running from `a` to `b`.
    Line { a: Point, b: Point },
    /// A circle.
    Circle { center: Point, radius: f64 },
}

impl Curve {
    /// The curve a segment lies on: a line's line, an arc's circle.
    pub fn of(segment: &Primitive) -> Curve {
        match *segment {
            Primitive::Line(l) => Curve::Line {
                a: l.start,
                b: l.end,
            },
            Primitive::Arc(a) => Curve::Circle {
                center: a.center,
                radius: a.radius,
            },
        }
    }

    /// Which side of the curve `p` lies, as a signed distance: left of a line (as it runs
    /// from `a` to `b`) and outside a circle are positive.
    pub fn side(&self, p: Point) -> f64 {
        match *self {
            Curve::Line { a, b } => {
                let len = distance(a, b);
                if len < CHAIN_TOL_MM {
                    return 0.0;
                }
                ((b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)) / len
            }
            Curve::Circle { center, radius } => distance(center, p) - radius,
        }
    }

    /// The point of the curve nearest `p`; None for a circle's own centre (every point is)
    /// or a line without length.
    pub fn foot(&self, p: Point) -> Option<Point> {
        match *self {
            Curve::Line { a, b } => {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len2 = dx * dx + dy * dy;
                if len2 < CHAIN_TOL_MM * CHAIN_TOL_MM {
                    return None;
                }
                let t = ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2;
                Some((a.0 + t * dx, a.1 + t * dy))
            }
            Curve::Circle { center, radius } => {
                let d = distance(center, p);
                if d < SAME_CIRCLE_TOL_MM {
                    return None;
                }
                Some((
                    center.0 + (p.0 - center.0) / d * radius,
                    center.1 + (p.1 - center.1) / d * radius,
                ))
            }
        }
    }

    /// The curve moved `d` to its positive side (left of a line, outward for a circle);
    /// None when a circle shrinks to nothing or a line has no length.
    fn offset(&self, d: f64) -> Option<Curve> {
        match *self {
            Curve::Line { a, b } => {
                let len = distance(a, b);
                if len < CHAIN_TOL_MM {
                    return None;
                }
                let n = (-(b.1 - a.1) / len * d, (b.0 - a.0) / len * d);
                Some(Curve::Line {
                    a: (a.0 + n.0, a.1 + n.1),
                    b: (b.0 + n.0, b.1 + n.1),
                })
            }
            Curve::Circle { center, radius } => {
                let r = radius + d;
                (r > CHAIN_TOL_MM).then_some(Curve::Circle { center, radius: r })
            }
        }
    }
}

/// A round: its centre and radius, and where it touches the first and the second curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Round {
    pub center: Point,
    pub radius: f64,
    pub first: Point,
    pub second: Point,
}

impl Round {
    /// The arc from where it touches the first curve to where it touches the second, the
    /// short way round; None for radius 0 or when the two points are one.
    pub fn arc(&self) -> Option<Arc> {
        if self.radius < CHAIN_TOL_MM || distance(self.first, self.second) < CHAIN_TOL_MM {
            return None;
        }
        let angle = |p: Point| math::atan2(p.1 - self.center.1, p.0 - self.center.0);
        let start_angle = angle(self.first);
        let mut sweep = angle(self.second) - start_angle;
        while sweep > PI {
            sweep -= 2.0 * PI;
        }
        while sweep <= -PI {
            sweep += 2.0 * PI;
        }
        (sweep.abs() > ANGLE_TOL_RAD).then_some(Arc {
            center: self.center,
            radius: self.radius,
            start_angle,
            sweep,
        })
    }
}

/// Where two curves cross: none, one (touching) or two points. Parallel lines and circles
/// on one centre have none.
pub fn crossings(c1: &Curve, c2: &Curve) -> Vec<Point> {
    match (*c1, *c2) {
        (Curve::Line { a, b }, Curve::Line { a: c, b: d }) => line_line(a, b, c, d),
        (Curve::Line { a, b }, Curve::Circle { center, radius })
        | (Curve::Circle { center, radius }, Curve::Line { a, b }) => {
            line_circle(a, b, center, radius)
        }
        (
            Curve::Circle {
                center: c1,
                radius: r1,
            },
            Curve::Circle {
                center: c2,
                radius: r2,
            },
        ) => circle_circle(c1, r1, c2, r2),
    }
}

fn line_line(a: Point, b: Point, c: Point, d: Point) -> Vec<Point> {
    let (ux, uy) = (b.0 - a.0, b.1 - a.1);
    let (vx, vy) = (d.0 - c.0, d.1 - c.1);
    let den = ux * vy - uy * vx;
    let lens = math::hypot(ux, uy) * math::hypot(vx, vy);
    // Parallel (the sine of the angle between them as good as 0).
    if lens == 0.0 || (den / lens).abs() < ANGLE_TOL_RAD {
        return Vec::new();
    }
    let t = ((c.0 - a.0) * vy - (c.1 - a.1) * vx) / den;
    vec![(a.0 + t * ux, a.1 + t * uy)]
}

fn line_circle(a: Point, b: Point, center: Point, radius: f64) -> Vec<Point> {
    let line = Curve::Line { a, b };
    let Some(foot) = line.foot(center) else {
        return Vec::new();
    };
    let d = distance(center, foot);
    if d > radius + CHAIN_TOL_MM {
        return Vec::new();
    }
    let half = (radius * radius - d * d).max(0.0).sqrt();
    if half < CHAIN_TOL_MM {
        return vec![foot];
    }
    let len = distance(a, b);
    let (ux, uy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
    vec![
        (foot.0 - ux * half, foot.1 - uy * half),
        (foot.0 + ux * half, foot.1 + uy * half),
    ]
}

fn circle_circle(c1: Point, r1: f64, c2: Point, r2: f64) -> Vec<Point> {
    let d = distance(c1, c2);
    if d < SAME_CIRCLE_TOL_MM || d > r1 + r2 + CHAIN_TOL_MM || d < (r1 - r2).abs() - CHAIN_TOL_MM {
        return Vec::new();
    }
    // Along the line of centres to the chord's middle, then across it.
    let along = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
    let half = (r1 * r1 - along * along).max(0.0).sqrt();
    let (ux, uy) = ((c2.0 - c1.0) / d, (c2.1 - c1.1) / d);
    let mid = (c1.0 + ux * along, c1.1 + uy * along);
    if half < CHAIN_TOL_MM {
        return vec![mid];
    }
    vec![
        (mid.0 - uy * half, mid.1 + ux * half),
        (mid.0 + uy * half, mid.1 - ux * half),
    ]
}

/// The rounds of radius `r` tangent to both curves with their centre on side `s1` of `c1`
/// and `s2` of `c2` (+1 or −1, as [`Curve::side`]): at most two.
pub fn rounds(c1: &Curve, s1: f64, c2: &Curve, s2: f64, r: f64) -> Vec<Round> {
    let (Some(o1), Some(o2)) = (c1.offset(s1 * r), c2.offset(s2 * r)) else {
        return Vec::new();
    };
    crossings(&o1, &o2)
        .into_iter()
        .filter_map(|center| {
            Some(Round {
                center,
                radius: r,
                first: c1.foot(center)?,
                second: c2.foot(center)?,
            })
        })
        .collect()
}

/// Of the rounds on sides `s1` and `s2`, the one touching the curves nearest `near` (a
/// polyline's corner: the round where the path turns, §3).
pub fn nearest_round(
    c1: &Curve,
    s1: f64,
    c2: &Curve,
    s2: f64,
    r: f64,
    near: Point,
) -> Option<Round> {
    rounds(c1, s1, c2, s2, r)
        .into_iter()
        .min_by(|x, y| reach(x, near, near).total_cmp(&reach(y, near, near)))
}

/// The round between the parts of `c1` and `c2` picked by clicks `k1` (on `c1`) and `k2`
/// (on `c2`): its centre on `k2`'s side of `c1` and on `k1`'s side of `c2`; of the two
/// left, the one touching nearest the clicks (§3). A click on the other curve itself
/// leaves that side open.
pub fn picked_round(c1: &Curve, k1: Point, c2: &Curve, k2: Point, r: f64) -> Option<Round> {
    let sides = |s: f64| -> Vec<f64> {
        if s.abs() < CHAIN_TOL_MM {
            vec![1.0, -1.0]
        } else {
            vec![s.signum()]
        }
    };
    let mut best: Option<Round> = None;
    for s1 in sides(c1.side(k2)) {
        for s2 in sides(c2.side(k1)) {
            for round in rounds(c1, s1, c2, s2, r) {
                if best.is_none_or(|b| reach(&round, k1, k2) < reach(&b, k1, k2)) {
                    best = Some(round);
                }
            }
        }
    }
    best
}

/// How far a round's tangent points are from `p1` and `p2`, together.
fn reach(round: &Round, p1: Point, p2: Point) -> f64 {
    distance(round.first, p1) + distance(round.second, p2)
}

/// The largest radius for which `fits` holds, assuming it holds from 0 up to a limit and
/// not beyond: doubled from `start` while it fits, then the interval halved down to the
/// chaining tolerance. 0 when not even the tolerance fits. Deterministic: the same steps
/// every time.
pub fn most_that_fits(start: f64, fits: impl Fn(f64) -> bool) -> f64 {
    if !fits(CHAIN_TOL_MM) {
        return 0.0;
    }
    let mut lo = CHAIN_TOL_MM;
    let mut hi = start.max(1.0);
    while fits(hi) {
        lo = hi;
        hi *= 2.0;
        if hi > SEARCH_LIMIT_MM {
            return lo;
        }
    }
    while hi - lo > CHAIN_TOL_MM / 2.0 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-9
    }

    const X_AXIS: Curve = Curve::Line {
        a: (-10.0, 0.0),
        b: (10.0, 0.0),
    };
    const Y_AXIS: Curve = Curve::Line {
        a: (0.0, -10.0),
        b: (0.0, 10.0),
    };

    #[test]
    fn two_crossing_lines_have_four_corners_each_picked_by_its_clicks() {
        for (k1, k2, center) in [
            ((8.0, 0.0), (0.0, 8.0), (5.0, 5.0)),
            ((-8.0, 0.0), (0.0, 8.0), (-5.0, 5.0)),
            ((-8.0, 0.0), (0.0, -8.0), (-5.0, -5.0)),
            ((8.0, 0.0), (0.0, -8.0), (5.0, -5.0)),
        ] {
            let round = picked_round(&X_AXIS, k1, &Y_AXIS, k2, 5.0).unwrap();
            assert!(close(round.center, center), "{round:?}");
            assert!(close(round.first, (center.0, 0.0)));
            assert!(close(round.second, (0.0, center.1)));
            let arc = round.arc().unwrap();
            assert!((arc.sweep.abs() - PI / 2.0).abs() < 1e-9);
        }
    }

    #[test]
    fn radius_zero_is_where_the_curves_meet() {
        let round = picked_round(&X_AXIS, (8.0, 0.0), &Y_AXIS, (0.0, 8.0), 0.0).unwrap();
        assert!(close(round.first, (0.0, 0.0)) && close(round.second, (0.0, 0.0)));
        assert!(round.arc().is_none());
        // Lines that fall short meet where they would, extended.
        let short = Curve::Line {
            a: (3.0, 5.0),
            b: (3.0, 9.0),
        };
        let round = picked_round(&X_AXIS, (8.0, 0.0), &short, (3.0, 8.0), 0.0).unwrap();
        assert!(close(round.first, (3.0, 0.0)));
    }

    #[test]
    fn a_line_and_a_circle_the_round_outside_and_inside() {
        let circle = Curve::Circle {
            center: (0.0, 12.0),
            radius: 10.0,
        };
        // Outside the circle, above the line: two places, the one nearer the clicks.
        let round = picked_round(&X_AXIS, (9.0, 0.0), &circle, (8.0, 6.0), 5.0).unwrap();
        assert!(round.center.0 > 0.0 && (round.center.1 - 5.0).abs() < 1e-9);
        assert!((distance(round.center, (0.0, 12.0)) - 15.0).abs() < 1e-9);
        let other = picked_round(&X_AXIS, (-9.0, 0.0), &circle, (-8.0, 6.0), 5.0).unwrap();
        assert!(other.center.0 < 0.0);
        // Inside the circle, against a line through it.
        let chord = Curve::Line {
            a: (-10.0, 12.0),
            b: (10.0, 12.0),
        };
        let inside = picked_round(&chord, (5.0, 12.0), &circle, (6.0, 20.0), 3.0).unwrap();
        assert!((distance(inside.center, (0.0, 12.0)) - 7.0).abs() < 1e-9);
        assert!((inside.center.1 - 15.0).abs() < 1e-9);
        // Inside, a radius as large as the circle's is no round.
        assert!(picked_round(&chord, (5.0, 12.0), &circle, (6.0, 20.0), 10.0).is_none());
    }

    #[test]
    fn two_circles_and_the_refusals() {
        let a = Curve::Circle {
            center: (0.0, 0.0),
            radius: 20.0,
        };
        let b = Curve::Circle {
            center: (30.0, 0.0),
            radius: 20.0,
        };
        // Outside both, in the notch above where they cross.
        let round = picked_round(&a, (10.0, 17.32), &b, (20.0, 17.32), 4.0).unwrap();
        assert!((distance(round.center, (0.0, 0.0)) - 24.0).abs() < 1e-9);
        assert!((distance(round.center, (30.0, 0.0)) - 24.0).abs() < 1e-9);
        assert!(round.center.1 > 0.0);
        // Circles on one centre, parallel lines: none.
        let c = Curve::Circle {
            center: (0.0, 0.0),
            radius: 5.0,
        };
        assert!(picked_round(&a, (20.0, 0.0), &c, (5.0, 0.0), 1.0).is_none());
        let parallel = Curve::Line {
            a: (-10.0, 4.0),
            b: (10.0, 4.0),
        };
        assert!(picked_round(&X_AXIS, (0.0, 0.0), &parallel, (0.0, 4.0), 2.0).is_none());
        // Too far apart for the radius.
        let far = Curve::Circle {
            center: (0.0, 100.0),
            radius: 5.0,
        };
        assert!(picked_round(&X_AXIS, (0.0, 0.0), &far, (0.0, 95.0), 10.0).is_none());
    }

    #[test]
    fn the_most_that_fits_is_found_within_the_tolerance() {
        let most = most_that_fits(1.0, |r| r <= 37.25);
        assert!((most - 37.25).abs() <= CHAIN_TOL_MM, "{most}");
        assert_eq!(most_that_fits(1.0, |_| false), 0.0);
        assert!(most_that_fits(1.0, |_| true) <= SEARCH_LIMIT_MM);
    }

    fn curve() -> impl Strategy<Value = Curve> {
        prop_oneof![
            (-50.0..50.0, -50.0..50.0, 0.0..(2.0 * PI)).prop_map(|(x, y, t)| Curve::Line {
                a: (x, y),
                b: (x + 20.0 * math::cos(t), y + 20.0 * math::sin(t)),
            }),
            (-50.0..50.0, -50.0..50.0, 1.0..40.0).prop_map(|(x, y, r)| Curve::Circle {
                center: (x, y),
                radius: r,
            }),
        ]
    }

    proptest! {
        /// Every round touches both curves (its tangent points on them, r from its centre,
        /// the centre on the asked sides) and leaves each tangentially.
        #[test]
        fn every_round_is_tangent_to_both_curves(
            c1 in curve(), c2 in curve(), r in 0.1..20.0,
            s1 in prop_oneof![Just(1.0), Just(-1.0)], s2 in prop_oneof![Just(1.0), Just(-1.0)],
        ) {
            for round in rounds(&c1, s1, &c2, s2, r) {
                for (curve, side, touch) in [(c1, s1, round.first), (c2, s2, round.second)] {
                    prop_assert!(curve.side(touch).abs() < 1e-6);
                    prop_assert!((distance(round.center, touch) - r).abs() < 1e-6);
                    prop_assert!((curve.side(round.center) - side * r).abs() < 1e-6);
                }
            }
        }
    }
}
