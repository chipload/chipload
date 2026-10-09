// SPDX-License-Identifier: GPL-3.0-or-later
//! Port of tests/unit/test_chain.py.

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::f64::consts::{PI, TAU};

use chipload_core::chain::{Segment, chain_segments};
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_core::tolerances::CHAIN_TOL_MM;

fn seg(prim: Primitive) -> Segment {
    Segment {
        prim,
        layer: "L".into(),
        handle: "H".into(),
        source_type: "LINE".into(),
    }
}

fn line(a: (f64, f64), b: (f64, f64)) -> Segment {
    seg(Primitive::Line(Line { start: a, end: b }))
}

fn square(size: f64) -> Vec<Segment> {
    let p = [(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)];
    (0..4).map(|i| line(p[i], p[(i + 1) % 4])).collect()
}

fn closed_flags(segments: &[Segment]) -> Vec<bool> {
    chain_segments(segments)
        .contours
        .iter()
        .map(|c| c.closed)
        .collect()
}

#[test]
fn square_is_one_closed_contour() {
    let result = chain_segments(&square(10.0));
    assert_eq!(result.contours.len(), 1);
    assert!(result.contours[0].closed);
    assert_eq!(result.contours[0].points().len(), 4);
}

#[test]
fn mixed_direction_and_order_still_chain() {
    let s = square(10.0);
    let flipped = line((0.0, 10.0), (10.0, 10.0));
    assert_eq!(
        closed_flags(&[s[3].clone(), s[0].clone(), flipped, s[1].clone()]),
        [true]
    );
}

#[test]
fn gap_within_tolerance_is_joined() {
    let s = square(10.0);
    let nudged = line((0.0, 10.0), (0.0, CHAIN_TOL_MM / 2.0));
    assert_eq!(
        closed_flags(&[s[0].clone(), s[1].clone(), s[2].clone(), nudged]),
        [true]
    );
}

#[test]
fn gap_above_tolerance_gives_open_contour() {
    let s = square(10.0);
    let short = line((0.0, 10.0), (0.0, CHAIN_TOL_MM * 5.0));
    assert_eq!(
        closed_flags(&[s[0].clone(), s[1].clone(), s[2].clone(), short]),
        [false]
    );
}

#[test]
fn zero_length_and_duplicates_are_removed_and_returned() {
    let mut s = square(10.0);
    let dup = line((10.0, 0.0), (0.0, 0.0));
    let tiny = line((5.0, 5.0), (5.0, 5.0 + CHAIN_TOL_MM / 10.0));
    s.push(dup.clone());
    s.push(tiny.clone());
    let result = chain_segments(&s);
    assert_eq!(result.duplicates, [dup]);
    assert_eq!(result.zero_length, [tiny]);
    assert_eq!(
        result.contours.iter().map(|c| c.closed).collect::<Vec<_>>(),
        [true]
    );
}

#[test]
fn two_different_arcs_between_same_points_are_not_duplicates() {
    let upper = seg(Primitive::Arc(Arc {
        center: (0.0, 0.0),
        radius: 5.0,
        start_angle: 0.0,
        sweep: PI,
    }));
    let lower = seg(Primitive::Arc(Arc {
        center: (0.0, 0.0),
        radius: 5.0,
        start_angle: PI,
        sweep: PI,
    }));
    let result = chain_segments(&[upper, lower]);
    assert!(result.duplicates.is_empty());
    assert_eq!(
        result.contours.iter().map(|c| c.closed).collect::<Vec<_>>(),
        [true]
    );
}

#[test]
fn full_circle_is_closed() {
    let circle = seg(Primitive::Arc(Arc {
        center: (3.0, 4.0),
        radius: 2.0,
        start_angle: 0.0,
        sweep: TAU,
    }));
    assert_eq!(closed_flags(&[circle]), [true]);
}

#[test]
fn branch_point_splits_into_open_contours() {
    let mut s = square(10.0);
    s.push(line((10.0, 10.0), (20.0, 20.0)));
    let result = chain_segments(&s);
    assert_eq!(result.branch_points, [(10.0, 10.0)]);
    assert!(result.contours.iter().all(|c| !c.closed));
}

#[test]
fn contour_ids_follow_drawing_order() {
    let mut s = square(10.0);
    s.extend(square(5.0).into_iter().map(|g| match g.prim {
        Primitive::Line(l) => line((l.start.0 + 50.0, l.start.1), (l.end.0 + 50.0, l.end.1)),
        Primitive::Arc(_) => unreachable!(),
    }));
    let result = chain_segments(&s);
    assert_eq!(
        result.contours.iter().map(|c| c.id).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(result.contours[0].points()[0], (0.0, 0.0));
}
