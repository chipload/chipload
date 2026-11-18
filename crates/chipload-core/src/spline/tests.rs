// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::math;
use crate::primitives::Arc;
use crate::tolerances::ARC_APPROX_TOL_MM;
use proptest::prelude::*;
use std::f64::consts::TAU;

fn close(a: Point, b: Point) -> bool {
    distance(a, b) < 1e-9
}

/// The direction from `a` to `b`, as a unit vector.
fn unit(a: Point, b: Point) -> Point {
    let d = distance(a, b);
    ((b.0 - a.0) / d, (b.1 - a.1) / d)
}

/// How far `p` lies from the primitive.
fn distance_to(p: Point, prim: &Primitive) -> f64 {
    match prim {
        Primitive::Line(l) => {
            let len = distance(l.start, l.end);
            if len == 0.0 {
                return distance(p, l.start);
            }
            let (ux, uy) = unit(l.start, l.end);
            let t = ((p.0 - l.start.0) * ux + (p.1 - l.start.1) * uy).clamp(0.0, len);
            distance(p, (l.start.0 + ux * t, l.start.1 + uy * t))
        }
        Primitive::Arc(a) => {
            let angle = math::atan2(p.1 - a.center.1, p.0 - a.center.0);
            let from_start = if a.sweep >= 0.0 {
                (angle - a.start_angle).rem_euclid(TAU)
            } else {
                (a.start_angle - angle).rem_euclid(TAU)
            };
            if from_start <= a.sweep.abs() {
                (distance(p, a.center) - a.radius).abs()
            } else {
                let end = a.point_at(a.start_angle + a.sweep);
                distance(p, a.point_at(a.start_angle)).min(distance(p, end))
            }
        }
    }
}

#[test]
fn the_curve_passes_through_every_point_and_bends_smoothly() {
    let points = [
        (0.0, 0.0),
        (20.0, 15.0),
        (45.0, 5.0),
        (60.0, 20.0),
        (61.0, 21.0),
    ];
    let pieces = beziers(&points, false);
    assert_eq!(pieces.len(), 4);
    for (i, b) in pieces.iter().enumerate() {
        assert!(close(b[0], points[i]) && close(b[3], points[i + 1]));
    }
    // Where two pieces meet, they leave and arrive in the same direction (G1).
    for pair in pieces.windows(2) {
        let (a, b) = (unit(pair[0][2], pair[0][3]), unit(pair[1][0], pair[1][1]));
        assert!(close(a, b), "{a:?} {b:?}");
    }
}

#[test]
fn two_points_are_a_straight_piece() {
    let pieces = beziers(&[(0.0, 0.0), (30.0, 10.0)], false);
    assert_eq!(pieces.len(), 1);
    for p in &pieces[0] {
        // Every control point on the line y = x / 3.
        assert!((p.1 - p.0 / 3.0).abs() < 1e-12);
    }
    let segs = segments(&[(0.0, 0.0), (30.0, 10.0)], false, ARC_APPROX_TOL_MM);
    assert_eq!(segs.len(), 1);
    assert!(matches!(segs[0], Primitive::Line(_)));
}

#[test]
fn a_closed_curve_is_smooth_across_its_first_point() {
    let points = [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)];
    let pieces = beziers(&points, true);
    assert_eq!(pieces.len(), 4);
    assert!(close(pieces[3][3], points[0]));
    let (a, b) = (
        unit(pieces[3][2], pieces[3][3]),
        unit(pieces[0][0], pieces[0][1]),
    );
    assert!(close(a, b));
    // Too few points, or a repeated one, are left out or give nothing.
    assert!(beziers(&points[..2], true).is_empty());
    assert!(beziers(&[(1.0, 1.0)], false).is_empty());
    assert_eq!(
        beziers(&[(0.0, 0.0), (0.0, 0.0), (5.0, 0.0)], false).len(),
        1
    );
}

#[test]
fn close_points_beside_far_ones_make_no_loop() {
    // The uniform form loops here; the centripetal one turns once, without a cusp.
    let points = [(0.0, 0.0), (50.0, 0.0), (50.5, 0.5), (51.0, 50.0)];
    for b in beziers(&points, false) {
        let samples: Vec<Point> = (0..=200)
            .map(|k| point_at(&b, f64::from(k) / 200.0))
            .collect();
        // Turning never adds up past a half turn in one piece: no loop.
        let mut turned = 0.0f64;
        for w in samples.windows(3) {
            let (a, c) = (unit(w[0], w[1]), unit(w[1], w[2]));
            let cross = a.0 * c.1 - a.1 * c.0;
            let dot = a.0 * c.0 + a.1 * c.1;
            turned += math::atan2(cross, dot).abs();
        }
        assert!(turned < std::f64::consts::PI, "turned {turned}");
    }
}

#[test]
fn moving_turning_mirroring_and_scaling_the_points_does_the_same_to_the_curve() {
    let points = [(0.0, 0.0), (20.0, 15.0), (45.0, 5.0), (60.0, 20.0)];
    let (sin, cos) = math::sin_cos(0.7);
    let map = |p: Point| {
        (
            2.0 * (p.0 * cos - p.1 * sin) + 3.0,
            -2.0 * (p.0 * sin + p.1 * cos),
        )
    };
    let moved: Vec<Point> = points.iter().map(|&p| map(p)).collect();
    for (b, m) in beziers(&points, false).iter().zip(beziers(&moved, false)) {
        for (p, q) in b.iter().zip(m) {
            assert!(distance(map(*p), q) < 1e-9);
        }
    }
}

proptest! {
    /// The lines and arcs stay within the tolerance of the exact curve, start at its first
    /// point and end at its last, joined end to end.
    #[test]
    fn the_segments_follow_the_curve_within_the_tolerance(
        raw in prop::collection::vec((-200.0f64..200.0, -200.0f64..200.0), 2..8),
        closed in any::<bool>(),
    ) {
        let points: Vec<Point> = raw;
        let pieces = beziers(&points, closed);
        prop_assume!(!pieces.is_empty());
        let segs = segments(&points, closed, ARC_APPROX_TOL_MM);
        prop_assert!(!segs.is_empty());
        for pair in segs.windows(2) {
            prop_assert!(distance(pair[0].end(), pair[1].start()) < 1e-6);
        }
        if !closed {
            prop_assert!(distance(segs[0].start(), pieces[0][0]) < 1e-9);
            let last = pieces[pieces.len() - 1][3];
            prop_assert!(distance(segs[segs.len() - 1].end(), last) < 1e-9);
        }
        for b in &pieces {
            for k in 0..=64 {
                let p = point_at(b, f64::from(k) / 64.0);
                let d = segs.iter().map(|s| distance_to(p, s)).fold(f64::INFINITY, f64::min);
                prop_assert!(d <= ARC_APPROX_TOL_MM + 1e-9, "{p:?} is {d} off");
            }
        }
        for s in &segs {
            if let Primitive::Arc(Arc { radius, .. }) = s {
                prop_assert!(radius.is_finite() && *radius > 0.0);
            }
        }
    }
}
