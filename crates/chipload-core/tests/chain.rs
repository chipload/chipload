// SPDX-License-Identifier: GPL-3.0-or-later

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::f64::consts::{PI, TAU};

use chipload_core::chain::{Segment, chain_segments};
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_core::tolerances::CHAIN_TOL_MM;

fn seg(prim: Primitive) -> Segment {
    Segment {
        construction: false,
        prim,
        layer: "L".into(),
        handle: "H".into(),
        shape: None,
        source_type: "LINE".into(),
        from: None,
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

#[test]
fn coordinates_at_the_edge_of_the_grid_dont_overflow() {
    // The grid's cell index saturates; its neighbours mustn't overflow (fuzzing, 2026-10-13).
    let far = 1.0e300;
    let result = chain_segments(&[line((far, far), (far, 0.0)), line((0.0, 0.0), (1.0, 0.0))]);
    assert_eq!(result.contours.len(), 2);
}

/// A square drawn as one shape (a closed polyline) on `layer`, its lower-left corner at
/// (x, 0).
fn shape_square(id: u64, layer: &str, x: f64, size: f64) -> Vec<Segment> {
    let p = [(x, 0.0), (x + size, 0.0), (x + size, size), (x, size)];
    (0..4)
        .map(|i| Segment {
            layer: layer.into(),
            shape: Some(id),
            ..line(p[i], p[(i + 1) % 4])
        })
        .collect()
}

fn on(layer: &str, segment: Segment) -> Segment {
    Segment {
        layer: layer.into(),
        ..segment
    }
}

#[test]
fn closed_shapes_sharing_an_edge_stay_closed_on_any_layers() {
    // The user's drawing (2026-10-25): a pocket drawn against the outline.
    for second in ["outline", "pocket"] {
        let mut s = shape_square(1, "outline", 0.0, 10.0);
        s.extend(shape_square(2, second, 10.0, 5.0));
        let result = chain_segments(&s);
        assert_eq!(closed_flags(&s), [true, true], "{second}");
        assert!(result.duplicates.is_empty(), "{second}");
        assert!(result.branch_points.is_empty(), "{second}");
    }
}

#[test]
fn a_closed_shape_on_top_of_another_on_the_same_layer_is_a_duplicate() {
    let mut s = shape_square(1, "a", 0.0, 10.0);
    s.extend(shape_square(2, "a", 0.0, 10.0));
    let result = chain_segments(&s);
    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.duplicates.len(), 4);

    let mut s = shape_square(1, "a", 0.0, 10.0);
    s.extend(shape_square(2, "b", 0.0, 10.0));
    let result = chain_segments(&s);
    assert_eq!(result.contours.len(), 2);
    assert!(result.duplicates.is_empty());
}

#[test]
fn a_line_touching_a_closed_shape_is_its_own_contour() {
    let mut s = vec![on("a", line((20.0, 20.0), (10.0, 10.0)))];
    s.extend(shape_square(1, "a", 0.0, 10.0));
    let result = chain_segments(&s);
    assert!(result.branch_points.is_empty());
    assert_eq!(
        result.contours.iter().map(|c| c.closed).collect::<Vec<_>>(),
        [false, true]
    );
    // A loose line on the square's edge is the duplicate; the square stays whole.
    let mut s = vec![on("a", line((10.0, 0.0), (0.0, 0.0)))];
    s.extend(shape_square(1, "a", 0.0, 10.0));
    let result = chain_segments(&s);
    assert_eq!(result.duplicates.len(), 1);
    assert_eq!(closed_flags(&s), [true]);
}

#[test]
fn loose_lines_chain_only_within_their_layer() {
    let s = [
        on("a", line((0.0, 0.0), (10.0, 0.0))),
        on("b", line((10.0, 0.0), (10.0, 10.0))),
        on("A", line((10.0, 0.0), (20.0, 5.0))),
    ];
    let result = chain_segments(&s);
    // Layers compare ignoring case: "a" and "A" chain, "b" stays apart.
    assert_eq!(result.contours.len(), 2);
    assert_eq!(result.contours[0].parts.len(), 2);
    assert_eq!(result.contours[1].layers(), ["b"]);
    assert!(result.duplicates.is_empty() && result.branch_points.is_empty());
}

#[test]
fn a_shape_that_goes_back_on_itself_chains_as_loose_lines() {
    let there = Segment {
        shape: Some(1),
        ..line((0.0, 0.0), (10.0, 0.0))
    };
    let back = Segment {
        shape: Some(1),
        ..line((10.0, 0.0), (0.0, 0.0))
    };
    let result = chain_segments(&[there, back.clone()]);
    assert_eq!(result.duplicates, [back]);
    assert_eq!(result.contours.len(), 1);
    assert!(!result.contours[0].closed);
}

mod properties {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Closed shapes stay closed contours, in drawing order, whatever touches them and in
        /// whatever order they come.
        #[test]
        fn closed_shapes_stay_closed(
            squares in prop::collection::vec((0..6i32, 1..4i32, 0..2usize), 1..8),
        ) {
            let mut s = Vec::new();
            for (i, (x, size, layer)) in squares.iter().enumerate() {
                s.extend(shape_square(
                    i as u64 + 1,
                    ["a", "b"][*layer],
                    f64::from(*x) * 2.0,
                    f64::from(*size) * 2.0,
                ));
            }
            let result = chain_segments(&s);
            prop_assert!(result.contours.iter().all(|c| c.closed));
            prop_assert_eq!(
                result.contours.len() + result.duplicates.len() / 4,
                squares.len()
            );
        }
    }
}

/// docs/specs/dxf-export.md §B3: a guide line whose end meets an ordinary line's end on the
/// same layer stays a contour of its own, and so does the line.
#[test]
fn construction_segments_chain_only_with_each_other() {
    let mut guide = line((10.0, 0.0), (20.0, 0.0));
    guide.construction = true;
    let segments = vec![line((0.0, 0.0), (10.0, 0.0)), guide];
    let result = chain_segments(&segments);
    assert_eq!(result.contours.len(), 2);
    let flags: Vec<bool> = result
        .contours
        .iter()
        .map(|c| c.is_construction())
        .collect();
    assert_eq!(flags.iter().filter(|&&f| f).count(), 1);
    // Two ordinary lines meeting there would chain into one.
    let joined = chain_segments(&[
        line((0.0, 0.0), (10.0, 0.0)),
        line((10.0, 0.0), (20.0, 0.0)),
    ]);
    assert_eq!(joined.contours.len(), 1);
}
