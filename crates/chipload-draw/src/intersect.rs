// SPDX-License-Identifier: GPL-3.0-or-later
//! Where lines and arcs cross or touch, and the special points of a segment as seen from a
//! point (nearest, perpendicular foot, tangent points). Plain analytic geometry: a line
//! against a line by its two parameters, a line against a circle by the quadratic, two
//! circles by their radical line. Points are kept only where they lie on both segments,
//! within `CHAIN_TOL_MM`.

use std::f64::consts::{FRAC_PI_2, TAU};

use chipload_core::Point;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;

/// Every point where `a` and `b` cross or touch, without repeats. Overlapping collinear
/// lines and arcs on the same circle give none here: where they meet, their end points do.
pub fn intersections(a: &Primitive, b: &Primitive) -> Vec<Point> {
    let found = match (a, b) {
        (Primitive::Line(p), Primitive::Line(q)) => line_line(p, q).into_iter().collect(),
        (Primitive::Line(l), Primitive::Arc(c)) | (Primitive::Arc(c), Primitive::Line(l)) => {
            line_circle(l.start, l.end, c.center, c.radius)
        }
        (Primitive::Arc(p), Primitive::Arc(q)) => {
            circle_circle(p.center, p.radius, q.center, q.radius)
        }
    };
    let mut points: Vec<Point> = Vec::new();
    for p in found {
        if lies_on(a, p) && lies_on(b, p) && points.iter().all(|&q| distance(p, q) > CHAIN_TOL_MM) {
            points.push(p);
        }
    }
    points
}

/// Whether `p`, a point on the segment's line or circle, is within its extent.
pub fn lies_on(prim: &Primitive, p: Point) -> bool {
    match prim {
        Primitive::Line(l) => {
            let length = distance(l.start, l.end);
            if length <= CHAIN_TOL_MM {
                return distance(l.start, p) <= CHAIN_TOL_MM;
            }
            let t = along_line(l, p);
            let slack = CHAIN_TOL_MM / length;
            (-slack..=1.0 + slack).contains(&t)
        }
        Primitive::Arc(a) => within_sweep(a, (p.1 - a.center.1).atan2(p.0 - a.center.0)),
    }
}

/// Whether the arc passes the angle `angle` (radians, about its centre), within
/// `CHAIN_TOL_MM` along the arc.
pub fn within_sweep(arc: &Arc, angle: f64) -> bool {
    if arc.sweep.abs() >= TAU {
        return true;
    }
    let slack = if arc.radius > 0.0 {
        CHAIN_TOL_MM / arc.radius
    } else {
        0.0
    };
    let along = if arc.sweep >= 0.0 {
        (angle - arc.start_angle).rem_euclid(TAU)
    } else {
        (arc.start_angle - angle).rem_euclid(TAU)
    };
    along <= arc.sweep.abs() + slack || along >= TAU - slack
}

/// The point of the segment nearest `p`.
pub fn nearest_on(prim: &Primitive, p: Point) -> Point {
    match prim {
        Primitive::Line(l) => {
            let t = along_line(l, p).clamp(0.0, 1.0);
            lerp(l.start, l.end, t)
        }
        Primitive::Arc(a) => {
            let angle = (p.1 - a.center.1).atan2(p.0 - a.center.0);
            if distance(p, a.center) > 0.0 && within_sweep(a, angle) {
                a.point_at(angle)
            } else {
                // Off the sweep (or at the centre): the nearer end.
                let (s, e) = (prim.start(), prim.end());
                if distance(p, s) <= distance(p, e) {
                    s
                } else {
                    e
                }
            }
        }
    }
}

/// The feet of the perpendiculars from `from` onto the segment that lie on it: on a line
/// the one foot; on an arc the points where the line through its centre and `from` meets it.
pub fn perpendicular_feet(prim: &Primitive, from: Point) -> Vec<Point> {
    match prim {
        Primitive::Line(l) => {
            if distance(l.start, l.end) <= CHAIN_TOL_MM {
                return Vec::new();
            }
            let foot = lerp(l.start, l.end, along_line(l, from));
            if lies_on(prim, foot) {
                vec![foot]
            } else {
                Vec::new()
            }
        }
        Primitive::Arc(a) => {
            if distance(from, a.center) <= CHAIN_TOL_MM {
                return Vec::new();
            }
            let angle = (from.1 - a.center.1).atan2(from.0 - a.center.0);
            [angle, angle + std::f64::consts::PI]
                .into_iter()
                .filter(|&t| within_sweep(a, t))
                .map(|t| a.point_at(t))
                .collect()
        }
    }
}

/// The points of an arc where a line from `from` touches it (none from inside its circle).
pub fn tangent_points(arc: &Arc, from: Point) -> Vec<Point> {
    let d = distance(from, arc.center);
    if d <= arc.radius + CHAIN_TOL_MM {
        return Vec::new();
    }
    let towards = (from.1 - arc.center.1).atan2(from.0 - arc.center.0);
    let spread = (arc.radius / d).acos();
    [towards + spread, towards - spread]
        .into_iter()
        .filter(|&t| within_sweep(arc, t))
        .map(|t| arc.point_at(t))
        .collect()
}

/// The points of an arc at 0°, 90°, 180° and 270° about its centre that it passes.
pub fn quadrants(arc: &Arc) -> Vec<Point> {
    (0..4)
        .map(|k| f64::from(k) * FRAC_PI_2)
        .filter(|&t| within_sweep(arc, t))
        .map(|t| arc.point_at(t))
        .collect()
}

/// Where `p` projects onto the line through `l`, as a parameter: 0 at its start, 1 at its end.
fn along_line(l: &Line, p: Point) -> f64 {
    let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
    let squared = dx * dx + dy * dy;
    ((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / squared
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Where the lines through `p` and `q` cross; None when they are parallel.
fn line_line(p: &Line, q: &Line) -> Option<Point> {
    let r = (p.end.0 - p.start.0, p.end.1 - p.start.1);
    let s = (q.end.0 - q.start.0, q.end.1 - q.start.1);
    let cross = r.0 * s.1 - r.1 * s.0;
    let (p_len, q_len) = (r.0.hypot(r.1), s.0.hypot(s.1));
    if p_len <= CHAIN_TOL_MM || q_len <= CHAIN_TOL_MM {
        return None;
    }
    // Parallel when, along the shorter line, the directions part by no more than the
    // chaining tolerance: such lines meet, if at all, at their ends.
    let sine = cross / (p_len * q_len);
    if sine.abs() * p_len.min(q_len) <= CHAIN_TOL_MM {
        return None;
    }
    let w = (q.start.0 - p.start.0, q.start.1 - p.start.1);
    let t = (w.0 * s.1 - w.1 * s.0) / cross;
    Some(lerp(p.start, p.end, t))
}

/// Where the line through `a` and `b` meets the circle; a line that passes within
/// `CHAIN_TOL_MM` of touching gives the touching point.
fn line_circle(a: Point, b: Point, center: Point, radius: f64) -> Vec<Point> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    if length <= CHAIN_TOL_MM {
        return Vec::new();
    }
    let (ux, uy) = (dx / length, dy / length);
    // The foot of the perpendicular from the centre, and the centre's distance to the line.
    let t = (center.0 - a.0) * ux + (center.1 - a.1) * uy;
    let foot = (a.0 + ux * t, a.1 + uy * t);
    let off = distance(foot, center);
    if off > radius + CHAIN_TOL_MM {
        return Vec::new();
    }
    if off >= radius - CHAIN_TOL_MM {
        return vec![foot];
    }
    let half = (radius * radius - off * off).sqrt();
    vec![
        (foot.0 - ux * half, foot.1 - uy * half),
        (foot.0 + ux * half, foot.1 + uy * half),
    ]
}

/// Where two circles meet; circles that come within `CHAIN_TOL_MM` of touching give the
/// touching point. Circles with one centre give none.
fn circle_circle(c1: Point, r1: f64, c2: Point, r2: f64) -> Vec<Point> {
    let d = distance(c1, c2);
    if d <= CHAIN_TOL_MM || d > r1 + r2 + CHAIN_TOL_MM || d < (r1 - r2).abs() - CHAIN_TOL_MM {
        return Vec::new();
    }
    let (ux, uy) = ((c2.0 - c1.0) / d, (c2.1 - c1.1) / d);
    // Distance from c1 to the radical line, along the centre line.
    let a = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
    let base = (c1.0 + ux * a, c1.1 + uy * a);
    let squared = r1 * r1 - a * a;
    let h = squared.max(0.0).sqrt();
    if h <= CHAIN_TOL_MM {
        return vec![base];
    }
    vec![
        (base.0 - uy * h, base.1 + ux * h),
        (base.0 + uy * h, base.1 - ux * h),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn line(a: Point, b: Point) -> Primitive {
        Primitive::Line(Line { start: a, end: b })
    }

    fn arc(center: Point, radius: f64, start_angle: f64, sweep: f64) -> Primitive {
        Primitive::Arc(Arc {
            center,
            radius,
            start_angle,
            sweep,
        })
    }

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-9
    }

    #[test]
    fn crossing_lines_meet_once_and_only_on_both() {
        let a = line((0.0, 0.0), (10.0, 10.0));
        let b = line((0.0, 10.0), (10.0, 0.0));
        let found = intersections(&a, &b);
        assert_eq!(found.len(), 1);
        assert!(close(found[0], (5.0, 5.0)));
        // The extension would cross, the segments don't.
        let short = line((0.0, 10.0), (4.0, 6.0));
        assert!(intersections(&a, &short).is_empty());
        // Parallel and collinear lines: none.
        assert!(intersections(&a, &line((1.0, 0.0), (11.0, 10.0))).is_empty());
        assert!(intersections(&a, &line((2.0, 2.0), (12.0, 12.0))).is_empty());
        // A T: one line ends on the other.
        let t = intersections(
            &line((0.0, 0.0), (10.0, 0.0)),
            &line((5.0, 0.0), (5.0, 5.0)),
        );
        assert_eq!(t.len(), 1);
        assert!(close(t[0], (5.0, 0.0)));
    }

    #[test]
    fn a_line_meets_an_arc_twice_once_or_not() {
        let circle = arc((0.0, 0.0), 5.0, 0.0, TAU);
        let through = intersections(&line((-10.0, 0.0), (10.0, 0.0)), &circle);
        assert_eq!(through.len(), 2);
        assert!(close(through[0], (-5.0, 0.0)) && close(through[1], (5.0, 0.0)));
        // Touching at the top: one point.
        let touch = intersections(&line((-10.0, 5.0), (10.0, 5.0)), &circle);
        assert_eq!(touch.len(), 1);
        assert!(close(touch[0], (0.0, 5.0)));
        assert!(intersections(&line((-10.0, 6.0), (10.0, 6.0)), &circle).is_empty());
        // The upper half only: the line at y = -3 misses it.
        let upper = arc((0.0, 0.0), 5.0, 0.0, PI);
        assert!(intersections(&line((-10.0, -3.0), (10.0, -3.0)), &upper).is_empty());
        assert_eq!(
            intersections(&line((-10.0, 3.0), (10.0, 3.0)), &upper).len(),
            2
        );
        // A clockwise arc over the same half.
        let clockwise = arc((0.0, 0.0), 5.0, PI, -PI);
        assert_eq!(
            intersections(&line((-10.0, 3.0), (10.0, 3.0)), &clockwise).len(),
            2
        );
    }

    #[test]
    fn two_arcs_meet_where_their_circles_do() {
        let a = arc((0.0, 0.0), 5.0, 0.0, TAU);
        let b = arc((8.0, 0.0), 5.0, 0.0, TAU);
        let found = intersections(&a, &b);
        assert_eq!(found.len(), 2);
        assert!(close(found[0], (4.0, 3.0)) && close(found[1], (4.0, -3.0)));
        // Touching from outside, and one inside the other.
        let touching = intersections(&a, &arc((10.0, 0.0), 5.0, 0.0, TAU));
        assert_eq!(touching.len(), 1);
        assert!(close(touching[0], (5.0, 0.0)));
        assert!(intersections(&a, &arc((1.0, 0.0), 2.0, 0.0, TAU)).is_empty());
        // Same circle: none (their ends are end points).
        assert!(intersections(&a, &arc((0.0, 0.0), 5.0, 0.0, PI)).is_empty());
        // b's left half only reaches the upper crossing if it sweeps there.
        let upper_left = arc((8.0, 0.0), 5.0, FRAC_PI_2, FRAC_PI_2);
        let one = intersections(&a, &upper_left);
        assert_eq!(one.len(), 1);
        assert!(close(one[0], (4.0, 3.0)));
    }

    #[test]
    fn nearest_perpendicular_and_tangent_points() {
        let l = line((0.0, 0.0), (10.0, 0.0));
        assert!(close(nearest_on(&l, (3.0, 4.0)), (3.0, 0.0)));
        assert!(close(nearest_on(&l, (-3.0, 4.0)), (0.0, 0.0)));
        assert_eq!(perpendicular_feet(&l, (3.0, 4.0)).len(), 1);
        assert!(perpendicular_feet(&l, (-3.0, 4.0)).is_empty());
        let upper = arc((0.0, 0.0), 5.0, 0.0, PI);
        let Primitive::Arc(upper_arc) = upper else {
            unreachable!()
        };
        assert!(close(nearest_on(&upper, (0.0, 10.0)), (0.0, 5.0)));
        // Below the half circle: its nearer end.
        assert!(close(nearest_on(&upper, (4.0, -1.0)), (5.0, 0.0)));
        // From above, the near foot is on the arc, the far one isn't.
        let feet = perpendicular_feet(&upper, (0.0, 10.0));
        assert_eq!(feet.len(), 1);
        assert!(close(feet[0], (0.0, 5.0)));
        // Tangents from (10, 0) touch the full circle at 60° either side; the upper half
        // has only the upper one.
        let full = Arc {
            sweep: TAU,
            ..upper_arc
        };
        assert_eq!(tangent_points(&full, (10.0, 0.0)).len(), 2);
        let up = tangent_points(&upper_arc, (10.0, 0.0));
        assert_eq!(up.len(), 1);
        assert!(close(up[0], (2.5, 5.0 * (PI / 3.0).sin())));
        assert!(tangent_points(&full, (1.0, 0.0)).is_empty());
        // Quadrants: the half circle passes 0°, 90°, 180°.
        assert_eq!(quadrants(&upper_arc).len(), 3);
        assert_eq!(quadrants(&full).len(), 4);
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        fn coord() -> impl Strategy<Value = f64> {
            -50.0..50.0f64
        }

        fn prim() -> impl Strategy<Value = Primitive> {
            prop_oneof![
                (coord(), coord(), coord(), coord()).prop_map(|(a, b, c, d)| line((a, b), (c, d))),
                (coord(), coord(), 0.5..30.0f64, -PI..PI, -TAU..TAU)
                    .prop_map(|(x, y, r, s, w)| arc((x, y), r, s, w)),
            ]
        }

        proptest! {
            /// Every point found lies on both segments.
            #[test]
            fn crossings_lie_on_both(a in prim(), b in prim()) {
                for p in intersections(&a, &b) {
                    for q in [&a, &b] {
                        let off = distance(nearest_on(q, p), p);
                        prop_assert!(off <= 2.0 * CHAIN_TOL_MM, "{p:?} is {off} off {q:?}");
                    }
                }
            }
        }
    }
}
