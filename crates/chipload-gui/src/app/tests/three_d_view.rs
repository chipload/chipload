// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn the_3d_view_can_be_entered_and_left_and_drawing_returns_to_top_view() {
    let mut app = drawn_part();
    app.view_3d();
    assert!(app.display.three_d && !app.view.is_top());
    app.view_top();
    assert!(!app.display.three_d && app.view.is_top());
    app.view_3d();
    app.start_drawing(Tool::Circle);
    assert!(
        !app.display.three_d && app.view.is_top(),
        "drawing works in top view only"
    );
    assert!(app.sketch().is_some());
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

#[test]
fn edit_and_simulate_each_keep_their_own_view() {
    // The user, 2026-11-03: 3D or top, the toolpath, the drawing and the pictures
    // remembered for each mode.
    let mut app = lower_part();
    assert!(!app.display.three_d && app.display.toolpath);
    app.set_show_simulation(true);
    // Simulate starts as Edit was, then is set up its own way.
    assert!(!app.display.three_d && app.display.toolpath && app.show_drawing());
    app.view_3d();
    app.display.toolpath = false;
    app.display.images = false;
    app.set_show_drawing(false);
    app.set_show_simulation(false);
    // Edit as it was left, the drawing shown.
    assert!(!app.display.three_d && app.display.toolpath && app.display.images);
    assert!(app.show_drawing());
    app.set_show_simulation(true);
    assert!(app.display.three_d && !app.display.toolpath && !app.display.images);
    assert!(!app.show_drawing());
}

/// Draw the canvas alone for one frame in a screen of `size` points.
fn draw_canvas_sized(app: &mut ChiploadApp, size: egui::Vec2) -> egui::Rect {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..Default::default()
    };
    let mut rect = egui::Rect::NOTHING;
    let mut output = ctx.run_ui(input, |ui| {
        rect = ui.available_rect_before_wrap();
        app.canvas(ui);
    });
    output.textures_delta.clear();
    rect
}

/// docs/specs/canvas-resize.md: a fitted view stays fitted when the canvas changes size.
#[test]
fn a_fitted_view_is_fitted_again_in_a_resized_canvas() {
    let mut app = drawn_part();
    app.fit_pending = true;
    draw_canvas_sized(&mut app, egui::vec2(1200.0, 800.0));
    let big = app.view;
    let small = draw_canvas_sized(&mut app, egui::vec2(600.0, 400.0));
    assert!(
        app.view.scale < big.scale * 0.6,
        "zoomed out with the canvas"
    );
    for p in app.scene.fit_points(app.view.top_z) {
        let at = app.view.to_screen3(p);
        assert!(small.expand(1.0).contains(at), "{at:?} outside {small:?}");
    }
    // Back to the first size: as the fit left it.
    draw_canvas_sized(&mut app, egui::vec2(1200.0, 800.0));
    assert!((app.view.scale - big.scale).abs() < 1e-9);
}

/// docs/specs/canvas-resize.md: a view zoomed in by hand keeps showing its area, and a
/// shrink and a grow back give the same view.
#[test]
fn a_zoomed_view_keeps_its_area_in_a_resized_canvas() {
    let mut app = drawn_part();
    app.fit_pending = true;
    draw_canvas_sized(&mut app, egui::vec2(1200.0, 800.0));
    app.view.zoom_at(egui::pos2(300.0, 300.0), 4.0);
    draw_canvas_sized(&mut app, egui::vec2(1200.0, 800.0));
    let zoomed = app.view;
    let middle = zoomed.to_world(egui::pos2(600.0, 400.0));
    // Half as wide: the same area, at half the scale, its middle in the middle.
    draw_canvas_sized(&mut app, egui::vec2(600.0, 800.0));
    assert!((app.view.scale - zoomed.scale / 2.0).abs() < 1e-9);
    let now = app.view.to_world(egui::pos2(300.0, 400.0));
    assert!((now.0 - middle.0).abs() < 1e-6 && (now.1 - middle.1).abs() < 1e-6);
    // Not a fit: the zoom stays.
    assert!(app.view.scale > zoomed.scale / 4.0);
    draw_canvas_sized(&mut app, egui::vec2(1200.0, 800.0));
    assert!((app.view.scale - zoomed.scale).abs() < 1e-9);
    assert!((app.view.offset.0 - zoomed.offset.0).abs() < 1e-6);
    assert!((app.view.offset.1 - zoomed.offset.1).abs() < 1e-6);
}
