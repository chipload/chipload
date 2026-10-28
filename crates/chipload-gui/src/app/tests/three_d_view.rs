// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn the_3d_view_can_be_entered_and_left_and_drawing_returns_to_top_view() {
    let mut app = drawn_part();
    app.view_3d();
    assert!(app.three_d && !app.view.is_top());
    app.view_top();
    assert!(!app.three_d && app.view.is_top());
    app.view_3d();
    app.start_drawing(Tool::Circle);
    assert!(
        !app.three_d && app.view.is_top(),
        "drawing works in top view only"
    );
    assert!(app.sketch.is_some());
}

#[test]
fn contours_can_be_picked_in_3d() {
    let mut app = drawn_part();
    app.view_3d();
    let circle = contour_of(&app, 2);
    let on_it = app
        .contours
        .iter()
        .find(|c| c.id == circle)
        .unwrap()
        .points()[0];
    let at = app.view.to_screen(on_it);
    assert_eq!(app.scene.pick(&app.view, at), Some(circle));
}

/// docs/specs/rectangle-corner-radius.md §2, §5: the form's radius reaches the project, a
/// radius too large is refused, and a corner drag that would leave too little room stops.
#[test]
fn a_rectangle_gets_rounded_corners_from_its_form() {
    let mut app = drawn_part();
    app.commit();
    // S1: 120 × 80, so at most 40.
    app.click(Some(contour_of(&app, 1)), false);
    assert_eq!(app.selected_shape(), Some(1));
    let mut geometry = app.shape(1).unwrap().geometry.clone();
    let Geometry::Rectangle { corner_radius, .. } = &mut geometry else {
        panic!("rectangle");
    };
    assert_eq!(*corner_radius, 0.0);
    *corner_radius = 10.0;
    app.set_selected_shape("drawn", geometry.clone()).unwrap();
    assert!(matches!(
        app.shape(1).unwrap().geometry,
        Geometry::Rectangle { corner_radius, .. } if corner_radius == 10.0
    ));
    // Eight segments now, four of them arcs.
    let arcs = app
        .shape(1)
        .unwrap()
        .segments()
        .iter()
        .filter(|s| matches!(s, chipload_job::PathSegment::Arc(_)))
        .count();
    assert_eq!(arcs, 4);
    let Geometry::Rectangle { corner_radius, .. } = &mut geometry else {
        panic!("rectangle");
    };
    *corner_radius = 41.0;
    let why = app.set_selected_shape("drawn", geometry).unwrap_err();
    assert!(why.contains("corner_radius: at most 40 mm"), "{why}");
    // A corner dragged to 15 mm from the opposite one: no room for 10 mm corners.
    app.commit();
    let corners: Vec<_> = app
        .shape_handles()
        .into_iter()
        .filter(|(h, _)| matches!(h, Handle::Corner(_)))
        .collect();
    let (opposite, far) = (corners[2].1, corners[0].1);
    app.start_handle_drag(corners[0].0);
    let toward = |t: f64| {
        (
            opposite.0 + (far.0 - opposite.0) * t,
            opposite.1 + (far.1 - opposite.1) * t,
        )
    };
    app.drag_handle_to(toward(0.9), SegmentDrag::Move);
    app.drag_handle_to(toward(0.1), SegmentDrag::Move);
    let why = app.shape_draft.as_ref().unwrap().error.clone().unwrap();
    assert!(why.contains("corner_radius"), "{why}");
    app.end_handle_drag();
    let Geometry::Rectangle {
        length,
        width,
        corner_radius,
        ..
    } = app.shape(1).unwrap().geometry
    else {
        panic!("rectangle");
    };
    assert!(
        (length - 108.0).abs() < 1e-6 && (width - 72.0).abs() < 1e-6,
        "{length} {width}"
    );
    assert_eq!(corner_radius, 10.0);
}
