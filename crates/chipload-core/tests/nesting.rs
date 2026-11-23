// SPDX-License-Identifier: GPL-3.0-or-later
//! The box test before `contains` never turns away a ring that `contains` accepts.

use chipload_core::Point;
use chipload_core::nesting::{Ring, contains, may_contain, ring_box};
use proptest::prelude::*;

/// A convex quadrilateral (or triangle) round a centre, on a coarse grid so that rings
/// often share edges and corners, where `contains` decides with its tolerance.
fn quad() -> impl Strategy<Value = Vec<Point>> {
    (0..8i32, 0..8i32, 1..6i32, 1..6i32, 1..6i32, 1..6i32).prop_map(|(cx, cy, r, u, l, d)| {
        let (cx, cy) = (f64::from(cx), f64::from(cy));
        vec![
            (cx + f64::from(r), cy),
            (cx, cy + f64::from(u)),
            (cx - f64::from(l), cy),
            (cx, cy - f64::from(d)),
        ]
    })
}

fn rect() -> impl Strategy<Value = Vec<Point>> {
    (0..8i32, 0..8i32, 1..8i32, 1..8i32).prop_map(|(x, y, w, h)| {
        let (x, y, w, h) = (f64::from(x), f64::from(y), f64::from(w), f64::from(h));
        vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
    })
}

fn shape() -> impl Strategy<Value = Vec<Point>> {
    prop_oneof![quad(), rect()]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn what_contains_accepts_the_boxes_allow(a in shape(), b in shape()) {
        let (outer, inner) = (Ring::from_points(a), Ring::from_points(b));
        if contains(&outer, &inner) {
            prop_assert!(may_contain(ring_box(&outer.points), ring_box(&inner.points)));
        }
    }
}
