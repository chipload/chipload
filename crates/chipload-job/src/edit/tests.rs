// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::models::{PathSegment, Shape};
use std::f64::consts::FRAC_PI_2;

fn close(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

fn rect(angle: f64) -> Geometry {
    Geometry::Rectangle {
        center: [10.0, 20.0],
        length: 8.0,
        width: 4.0,
        angle,
        corner_radius: 0.0,
        radii: Vec::new(),
    }
}

fn corners(g: &Geometry) -> [[f64; 2]; 4] {
    let Geometry::Rectangle {
        center,
        length,
        width,
        angle,
        ..
    } = *g
    else {
        panic!("rectangle");
    };
    rectangle_corners(center, length, width, angle)
}

fn triangle() -> Geometry {
    Geometry::Polyline {
        closed: true,
        points: vec![[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]],
        bulges: vec![0.0, 0.4, 0.0],
        radii: Vec::new(),
    }
}

#[test]
fn circle_handles_move_it_and_set_the_radius() {
    let circle = Geometry::Circle {
        center: [1.0, 2.0],
        radius: 5.0,
    };
    let list = handles(&circle, 3.0);
    assert_eq!(
        list,
        [(Handle::Center, [1.0, 2.0]), (Handle::Radius, [6.0, 2.0])]
    );
    assert_eq!(
        drag(&circle, Handle::Center, [7.0, 8.0], SegmentDrag::Move),
        Some(Geometry::Circle {
            center: [7.0, 8.0],
            radius: 5.0
        })
    );
    assert_eq!(
        drag(&circle, Handle::Radius, [1.0, 5.0], SegmentDrag::Move),
        Some(Geometry::Circle {
            center: [1.0, 2.0],
            radius: 3.0
        })
    );
    // Radius 0 is invalid: the drag stops (the caller keeps the last valid shape).
    assert_eq!(
        drag(&circle, Handle::Radius, [1.0, 2.0], SegmentDrag::Move),
        None
    );
    // Handles of other kinds don't apply.
    assert_eq!(
        drag(&circle, Handle::Rotate, [0.0, 0.0], SegmentDrag::Move),
        None
    );
}

#[test]
fn a_slot_stays_a_slot_when_its_size_changes() {
    let rect = |width: f64, corner_radius: f64| Geometry::Rectangle {
        center: [0.0, 0.0],
        length: 40.0,
        width,
        angle: 0.0,
        corner_radius,
        radii: Vec::new(),
    };
    let slot = rect(10.0, 5.0);
    assert!(is_slot(&slot));
    assert!(!is_slot(&rect(10.0, 2.0)));
    // Wider through the form: the radius follows.
    assert_eq!(kept_slot(&slot, rect(16.0, 5.0)), rect(16.0, 8.0));
    // A radius typed with it wins; a plain rounded rectangle is left alone.
    assert_eq!(kept_slot(&slot, rect(16.0, 3.0)), rect(16.0, 3.0));
    assert_eq!(
        kept_slot(&rect(10.0, 2.0), rect(16.0, 2.0)),
        rect(16.0, 2.0)
    );
    // A corner drag narrows it: still a slot (a plain rectangle with that radius
    // would refuse to get narrower than its corners).
    let narrower = try_drag(&slot, Handle::Corner(2), [20.0, 2.0], SegmentDrag::Move).unwrap();
    assert!(is_slot(&narrower));
}

#[test]
fn a_polygon_has_a_centre_and_a_corner_handle_and_mirrors_and_turns() {
    let hexagon = Geometry::Polygon {
        center: [0.0, 0.0],
        radius: 10.0,
        sides: 6,
        angle: 0.0,
        radii: Vec::new(),
    };
    let list = handles(&hexagon, 5.0);
    assert_eq!(
        list,
        [(Handle::Center, [0.0, 0.0]), (Handle::Radius, [10.0, 0.0])]
    );
    // Dragging the corner sets the radius and the turn.
    let Ok(Geometry::Polygon { radius, angle, .. }) =
        try_drag(&hexagon, Handle::Radius, [0.0, 20.0], SegmentDrag::Move)
    else {
        panic!()
    };
    assert_eq!(radius, 20.0);
    assert!((angle - FRAC_PI_2).abs() < 1e-12);
    // Mirrored left to right, its first corner points the other way.
    let turned = Geometry::Polygon {
        center: [5.0, 0.0],
        radius: 10.0,
        sides: 5,
        angle: 0.3,
        radii: Vec::new(),
    };
    let Some(Geometry::Polygon { center, angle, .. }) = mirrored(&turned, Mirror::LeftRight, 0.0)
    else {
        panic!()
    };
    assert_eq!(center, [-5.0, 0.0]);
    assert!((angle - (PI - 0.3)).abs() < 1e-12);
    // Turned by 90° and scaled by 2 about the origin.
    let Some(Geometry::Polygon {
        center,
        radius,
        angle,
        ..
    }) = transformed(&turned, [0.0, 0.0], [0.0, 0.0], 2.0, 90.0)
    else {
        panic!()
    };
    assert_eq!((center, radius), ([0.0, 10.0], 20.0));
    assert!((angle - (0.3 + FRAC_PI_2)).abs() < 1e-12);
    // A point on its first corner follows the corner.
    let bigger = Geometry::Polygon {
        center: [0.0, 0.0],
        radius: 20.0,
        sides: 6,
        angle: FRAC_PI_2,
        radii: Vec::new(),
    };
    let moved = followed_point([10.0, 0.0], &hexagon, &bigger).unwrap();
    assert!((moved[0]).abs() < 1e-9 && (moved[1] - 20.0).abs() < 1e-9);
}

#[test]
fn lengths_show_and_can_be_typed() {
    let l_shape = Geometry::Polyline {
        closed: false,
        points: vec![[0.0, 0.0], [30.0, 0.0], [30.0, 10.0], [40.0, 10.0]],
        bulges: vec![0.0, 0.0, 0.5],
        radii: Vec::new(),
    };
    // The arc segment has no label; an open polyline's stand to the left.
    let shown = dimensions(&l_shape);
    let brief: Vec<_> = shown
        .iter()
        .map(|s| (s.dimension, s.at, s.value, s.outward))
        .collect();
    assert_eq!(
        brief,
        [
            (Dimension::Segment(0), [15.0, 0.0], 30.0, [0.0, 1.0]),
            (Dimension::Segment(1), [30.0, 5.0], 10.0, [-1.0, 0.0])
        ]
    );
    // Segment 0 made 50 long: its end moves along it, the rest stays.
    let Ok(Geometry::Polyline { points, .. }) =
        with_dimension(&l_shape, Dimension::Segment(0), 50.0)
    else {
        panic!()
    };
    assert_eq!(
        points,
        [[0.0, 0.0], [50.0, 0.0], [30.0, 10.0], [40.0, 10.0]]
    );
    assert!(with_dimension(&l_shape, Dimension::Segment(2), 5.0).is_err());
    assert!(with_dimension(&l_shape, Dimension::Segment(0), 0.0).is_err());
    let rect = Geometry::Rectangle {
        center: [0.0, 0.0],
        length: 40.0,
        width: 20.0,
        angle: 0.0,
        corner_radius: 0.0,
        radii: Vec::new(),
    };
    // On the bottom and the right side, standing outward.
    let brief: Vec<_> = dimensions(&rect)
        .iter()
        .map(|s| (s.dimension, s.at, s.value, s.outward))
        .collect();
    assert_eq!(
        brief,
        [
            (Dimension::Length, [0.0, -10.0], 40.0, [0.0, -1.0]),
            (Dimension::Width, [20.0, 0.0], 20.0, [1.0, 0.0])
        ]
    );
    let Ok(Geometry::Rectangle { width, center, .. }) =
        with_dimension(&rect, Dimension::Width, 30.0)
    else {
        panic!()
    };
    assert_eq!((width, center), (30.0, [0.0, 0.0]));
    assert!(with_dimension(&rect, Dimension::Radius, 3.0).is_err());
}

#[test]
fn a_dragged_corner_keeps_the_opposite_corner_and_the_angle() {
    for angle in [0.0, 0.5, -2.0] {
        let start = rect(angle);
        let before = corners(&start);
        for k in 0..4 {
            let (sin, cos) = math::sin_cos(angle);
            // 3 mm further out along the local X and 1 mm along local Y.
            let to = [
                before[k][0] + 3.0 * cos - 1.0 * sin,
                before[k][1] + 3.0 * sin + 1.0 * cos,
            ];
            let moved = drag(&start, Handle::Corner(k), to, SegmentDrag::Move).unwrap();
            let after = corners(&moved);
            let Geometry::Rectangle { angle: a, .. } = moved else {
                panic!()
            };
            assert_eq!(a, angle);
            assert!(close(after[(k + 2) % 4], before[(k + 2) % 4]), "{k}");
            // The dragged corner is where the cursor is (perhaps as another index
            // when the drag flipped a side).
            assert!(after.iter().any(|&c| close(c, to)), "{k} {after:?}");
        }
    }
    // Onto the opposite corner's line: no width left, refused.
    let start = rect(0.0);
    let opposite = corners(&start)[2];
    assert_eq!(
        drag(
            &start,
            Handle::Corner(0),
            [0.0, opposite[1]],
            SegmentDrag::Move
        ),
        None
    );
}

#[test]
fn the_rotate_handle_changes_only_the_angle() {
    let start = rect(0.0);
    let rotate = handles(&start, 5.0)
        .into_iter()
        .find(|(h, _)| *h == Handle::Rotate)
        .unwrap();
    assert!(close(rotate.1, [10.0 + 4.0 + 5.0, 20.0]));
    let turned = drag(&start, Handle::Rotate, [10.0, 30.0], SegmentDrag::Move).unwrap();
    assert_eq!(
        turned,
        Geometry::Rectangle {
            center: [10.0, 20.0],
            length: 8.0,
            width: 4.0,
            angle: std::f64::consts::FRAC_PI_2,
            corner_radius: 0.0,
            radii: Vec::new(),
        }
    );
    let moved = drag(&start, Handle::Center, [0.0, 0.0], SegmentDrag::Move).unwrap();
    assert!(matches!(
        moved,
        Geometry::Rectangle {
            center: [0.0, 0.0],
            length: 8.0,
            ..
        }
    ));
}

#[test]
fn polyline_points_move_and_middles_insert_or_bend() {
    let start = triangle();
    let list = handles(&start, 0.0);
    assert_eq!(list.len(), 6);
    // The middle of the bulged segment is on its arc.
    let shape = Shape {
        construction: false,
        id: 1,
        layer: String::new(),
        name: None,
        geometry: start.clone(),
    };
    let PathSegment::Arc(arc) = shape.segments()[1] else {
        panic!("arc")
    };
    let mid = list[4].1;
    let r = math::hypot(mid[0] - arc.center[0], mid[1] - arc.center[1]);
    assert!((r - arc.radius).abs() < 1e-9);

    let moved = drag(&start, Handle::Point(2), [20.0, 40.0], SegmentDrag::Move).unwrap();
    assert!(matches!(&moved, Geometry::Polyline { points, .. } if points[2] == [20.0, 40.0]));
    // On top of its neighbour: refused.
    assert_eq!(
        drag(&start, Handle::Point(2), [40.0, 0.0], SegmentDrag::Move),
        None
    );

    let inserted = drag(&start, Handle::Middle(0), [20.0, -5.0], SegmentDrag::Insert).unwrap();
    assert_eq!(
        inserted,
        Geometry::Polyline {
            closed: true,
            points: vec![[0.0, 0.0], [20.0, -5.0], [40.0, 0.0], [20.0, 30.0]],
            bulges: vec![0.0, 0.0, 0.4, 0.0],
            radii: Vec::new(),
        }
    );
    // Shift: bend the first segment 10 mm below its chord: bulge 10 / 20.
    let bent = drag(&start, Handle::Middle(0), [20.0, -10.0], SegmentDrag::Bend).unwrap();
    assert!(matches!(&bent, Geometry::Polyline { bulges, .. } if bulges[0] == 0.5));
    // Further than a half circle: held at one.
    let full = drag(&start, Handle::Middle(0), [20.0, -50.0], SegmentDrag::Bend).unwrap();
    assert!(matches!(&full, Geometry::Polyline { bulges, .. } if bulges[0] == 1.0));
    let other = drag(&start, Handle::Middle(0), [20.0, 50.0], SegmentDrag::Bend).unwrap();
    assert!(matches!(&other, Geometry::Polyline { bulges, .. } if bulges[0] == -1.0));
}

#[test]
fn a_plain_segment_drag_moves_the_segment_and_its_neighbours_follow() {
    let start = triangle();
    // Segment 0 (0,0)-(40,0), its middle (20,0) dragged 10 mm down.
    let moved = drag(&start, Handle::Middle(0), [20.0, -10.0], SegmentDrag::Move).unwrap();
    assert_eq!(
        moved,
        Geometry::Polyline {
            closed: true,
            points: vec![[0.0, -10.0], [40.0, -10.0], [20.0, 30.0]],
            bulges: vec![0.0, 0.4, 0.0],
            radii: Vec::new(),
        }
    );
    // Its end onto the far point: a neighbour of no length, refused.
    assert_eq!(
        drag(&start, Handle::Middle(0), [0.0, 30.0], SegmentDrag::Move),
        None
    );
}

#[test]
fn inserting_splits_an_arc_into_two_halves_of_it() {
    let mut g = triangle();
    let before = Shape {
        construction: false,
        id: 1,
        layer: String::new(),
        name: None,
        geometry: g.clone(),
    }
    .segments();
    assert!(insert_point_after(&mut g, 1));
    let after = Shape {
        construction: false,
        id: 1,
        layer: String::new(),
        name: None,
        geometry: g.clone(),
    }
    .segments();
    let (PathSegment::Arc(whole), PathSegment::Arc(h1), PathSegment::Arc(h2)) =
        (&before[1], &after[1], &after[2])
    else {
        panic!("arcs")
    };
    for half in [*h1, *h2] {
        assert!(close(half.center, whole.center));
        assert!((half.radius - whole.radius).abs() < 1e-9);
        assert!((half.sweep - whole.sweep / 2.0).abs() < 1e-9);
    }
    // After the last point of an open polyline: 10 mm on, straight.
    let mut open = Geometry::straight_polyline(false, vec![[0.0, 0.0], [5.0, 0.0]]);
    assert!(insert_point_after(&mut open, 1));
    assert_eq!(
        open,
        Geometry::straight_polyline(false, vec![[0.0, 0.0], [5.0, 0.0], [15.0, 0.0]])
    );
}

#[test]
fn removing_and_closing_keep_one_bulge_per_segment() {
    let mut g = Geometry::Polyline {
        closed: true,
        points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
        bulges: vec![0.1, 0.2, 0.3, 0.4],
        radii: Vec::new(),
    };
    // Point 2 joins segments 1 and 2 into one straight one.
    assert!(remove_point(&mut g, 2));
    assert!(matches!(&g, Geometry::Polyline { bulges, .. } if *bulges == [0.1, 0.0, 0.4]));
    // Point 0 of a closed one joins the last segment and the first.
    assert!(remove_point(&mut g, 0));
    assert!(matches!(&g, Geometry::Polyline { bulges, .. } if *bulges == [0.0, 0.0]));

    let mut open = Geometry::Polyline {
        closed: false,
        points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]],
        bulges: vec![0.1, 0.2],
        radii: Vec::new(),
    };
    set_closed(&mut open, true);
    assert!(matches!(&open, Geometry::Polyline { bulges, .. } if *bulges == [0.1, 0.2, 0.0]));
    set_closed(&mut open, false);
    assert!(remove_point(&mut open, 0));
    assert!(matches!(&open, Geometry::Polyline { bulges, .. } if *bulges == [0.2]));
    assert!(!remove_point(&mut open, 5));
}

#[test]
fn transformed_moves_turns_and_scales_every_kind() {
    let circle = Geometry::Circle {
        center: [10.0, 0.0],
        radius: 2.0,
    };
    assert_eq!(
        transformed(&circle, [0.0, 0.0], [5.0, 5.0], 2.0, 90.0),
        Some(Geometry::Circle {
            center: [5.0, 25.0],
            radius: 4.0
        })
    );
    let rectangle = Geometry::Rectangle {
        center: [1.0, 1.0],
        length: 4.0,
        width: 2.0,
        angle: 0.0,
        corner_radius: 0.5,
        radii: Vec::new(),
    };
    let Some(Geometry::Rectangle {
        center,
        length,
        width,
        angle,
        corner_radius,
        ..
    }) = transformed(&rectangle, [1.0, 1.0], [1.0, 1.0], 1.5, 30.0)
    else {
        panic!();
    };
    assert_eq!(
        (center, length, width, corner_radius),
        ([1.0, 1.0], 6.0, 3.0, 0.75)
    );
    assert!((angle - 30f64.to_radians()).abs() < 1e-12);
    let polyline = Geometry::Polyline {
        closed: false,
        points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]],
        bulges: vec![0.0, 0.4],
        radii: Vec::new(),
    };
    // A quarter turn about (10, 0) is exact; bulges stay.
    assert_eq!(
        transformed(&polyline, [10.0, 0.0], [10.0, 0.0], 1.0, 90.0),
        Some(Geometry::Polyline {
            closed: false,
            points: vec![[10.0, -10.0], [10.0, 0.0], [5.0, 0.0]],
            bulges: vec![0.0, 0.4],
            radii: Vec::new(),
        })
    );
    // A whole turn gives the shape back, within rounding.
    let Some(Geometry::Polyline { points, .. }) =
        transformed(&polyline, [3.0, 4.0], [3.0, 4.0], 1.0, 360.0)
    else {
        panic!();
    };
    assert_eq!(points, vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]]);
    let Some(Geometry::Polyline { points, .. }) =
        transformed(&polyline, [3.0, 4.0], [3.0, 4.0], 1.0, 33.0)
            .and_then(|g| transformed(&g, [3.0, 4.0], [3.0, 4.0], 1.0, -33.0))
    else {
        panic!();
    };
    for (p, q) in points.iter().zip([[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]]) {
        assert!(math::hypot(p[0] - q[0], p[1] - q[1]) < 1e-12);
    }
    assert_eq!(transformed(&polyline, [0.0; 2], [0.0; 2], 0.0, 0.0), None);
    assert_eq!(transformed(&polyline, [0.0; 2], [0.0; 2], -1.0, 0.0), None);
}

#[test]
fn a_rectangle_turned_keeps_a_small_angle() {
    let rectangle = Geometry::Rectangle {
        center: [0.0, 0.0],
        length: 4.0,
        width: 2.0,
        angle: 3.0,
        corner_radius: 0.0,
        radii: Vec::new(),
    };
    let Some(Geometry::Rectangle { angle, .. }) =
        transformed(&rectangle, [0.0; 2], [0.0; 2], 1.0, 90.0)
    else {
        panic!();
    };
    assert!((angle - (3.0 + std::f64::consts::FRAC_PI_2 - TAU)).abs() < 1e-12);
}

/// docs/specs/drawing-improvements.md §1.
#[test]
fn shapes_reflect_about_any_line() {
    // About the 45° line through the origin, x and y swap.
    let (a, b) = ([0.0, 0.0], [1.0, 1.0]);
    let close =
        |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).abs() < 1e-12 && (p[1] - q[1]).abs() < 1e-12;
    assert!(close(reflected_point([3.0, 1.0], a, b), [1.0, 3.0]));
    // A rectangle along X becomes one along Y: angle 90°.
    let r = Geometry::Rectangle {
        center: [10.0, 0.0],
        length: 20.0,
        width: 4.0,
        angle: 0.0,
        corner_radius: 1.0,
        radii: Vec::new(),
    };
    let Some(Geometry::Rectangle { center, angle, .. }) = mirrored_about(&r, a, b) else {
        panic!("a rectangle");
    };
    assert!(close(center, [0.0, 10.0]));
    assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    // A polygon's first corner goes where its image is.
    let hex = Geometry::Polygon {
        center: [0.0, 0.0],
        radius: 5.0,
        sides: 6,
        angle: 0.3,
        radii: Vec::new(),
    };
    let Some(Geometry::Polygon { angle, .. }) = mirrored_about(&hex, a, b) else {
        panic!("a polygon");
    };
    let corner = |t: f64| [5.0 * math::cos(t), 5.0 * math::sin(t)];
    assert!(close(corner(angle), reflected_point(corner(0.3), a, b)));
    // A polyline's arcs bend the other way; about a horizontal line exactly.
    let arc = Geometry::Polyline {
        closed: false,
        points: vec![[0.0, 1.5], [10.0, 2.25]],
        bulges: vec![0.5],
        radii: Vec::new(),
    };
    assert_eq!(
        mirrored_about(&arc, [0.0, 1.0], [5.0, 1.0]),
        Some(Geometry::Polyline {
            closed: false,
            points: vec![[0.0, 0.5], [10.0, -0.25]],
            bulges: vec![-0.5],
            radii: Vec::new(),
        })
    );
    // An imported path, or a line of one point: none.
    assert_eq!(mirrored_about(&r, a, a), None);
}

/// docs/specs/polyline-radii.md §4a: a rectangle's and a polygon's per-corner radii stay on
/// their corners when the shape is turned past half a turn, scaled or mirrored.
#[test]
fn corner_radii_stay_on_their_corners() {
    let rect = Geometry::Rectangle {
        center: [0.0, 0.0],
        length: 100.0,
        width: 60.0,
        angle: 3.0,
        corner_radius: 0.0,
        radii: vec![1.0, 2.0, 3.0, 4.0],
    };
    // The radius at each sharp corner, by where the corner is.
    let at = |g: &Geometry| -> Vec<([f64; 2], f64)> {
        let Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            radii,
            ..
        } = g
        else {
            panic!("a rectangle")
        };
        rectangle_corners(*center, *length, *width, *angle)
            .into_iter()
            .zip(radii.iter().copied())
            .collect()
    };
    let radius_near = |list: &[([f64; 2], f64)], p: [f64; 2]| {
        list.iter()
            .min_by(|a, b| {
                let da = math::hypot(a.0[0] - p[0], a.0[1] - p[1]);
                let db = math::hypot(b.0[0] - p[0], b.0[1] - p[1]);
                da.total_cmp(&db)
            })
            .unwrap()
            .1
    };
    let before = at(&rect);
    // Turned 20° (past half a turn from 3 rad, renumbering the corners) and doubled.
    let turned = transformed(&rect, [0.0, 0.0], [0.0, 0.0], 2.0, 20.0).unwrap();
    for (p, r) in &before {
        let q = transformed_point(*p, [0.0, 0.0], [0.0, 0.0], 2.0, 20.0);
        assert!((radius_near(&at(&turned), q) - 2.0 * r).abs() < 1e-9);
    }
    // Mirrored about a slanted line.
    let (a, b) = ([0.0, 5.0], [3.0, 7.0]);
    let flipped = mirrored_about(&rect, a, b).unwrap();
    for (p, r) in &before {
        assert_eq!(radius_near(&at(&flipped), reflected_point(*p, a, b)), *r);
    }
    // A polygon's too.
    let hex = Geometry::Polygon {
        center: [0.0, 0.0],
        radius: 20.0,
        sides: 6,
        angle: 0.0,
        radii: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
    };
    let Some(Geometry::Polygon { angle, radii, .. }) = mirrored(&hex, Mirror::LeftRight, 0.0)
    else {
        panic!("a polygon")
    };
    let corners = polygon_corners([0.0, 0.0], 20.0, 6, angle);
    // The first corner was at (20, 0); its image (-20, 0) keeps radius 1.
    let k = corners
        .iter()
        .position(|c| (c[0] + 20.0).abs() < 1e-9 && c[1].abs() < 1e-9)
        .unwrap();
    assert_eq!(radii[k], 1.0);
}
