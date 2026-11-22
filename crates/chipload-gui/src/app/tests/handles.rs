// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn dragging_a_handle_edits_the_shape_in_one_undo_step() {
    let mut app = drawn_part();
    app.commit();
    // S3 is the triangle (110, 65) (140, 65) (125, 95).
    app.click(Some(contour_of(&app, 3)), false);
    assert_eq!(app.selected_shape(), Some(3));
    assert_eq!(app.shape_handles().len(), 3 + 3, "3 points, 3 middles");
    // The middles are drawn apart, and the status bar says how to use them.
    draw_canvas(&mut app);
    // 3 points, and the box round it: move, turn, 4 corners (drawn-shapes-box.md §2) and
    // 4 sides (scale-and-stretch.md §3).
    assert_eq!(app.scene.handles.len(), 3 + 10);
    assert_eq!(app.scene.middle_handles.len(), 3);
    assert!(app.polyline_selected());
    app.start_handle_drag(Handle::Point(2));
    app.drag_handle_to((125.0, 100.0), SegmentDrag::Move);
    app.drag_handle_to((126.0, 105.0), SegmentDrag::Move);
    // Onto its neighbour: refused, the last valid shape stays and the form says why.
    app.drag_handle_to((140.0, 65.0), SegmentDrag::Move);
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points[2], [126.0, 105.0]);
    let why = app.shape_draft.as_ref().unwrap().error.clone().unwrap();
    assert!(why.contains("on top of the previous point"), "{why}");
    app.end_handle_drag();
    assert!(app.shape_draft.as_ref().unwrap().error.is_none());
    app.commit();
    // One undo step for the whole drag.
    app.undo();
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points[2], [125.0, 95.0]);

    // Shift on a middle bends the segment: 7.5 mm below the 30 mm bottom side.
    app.click(Some(contour_of(&app, 3)), false);
    app.start_handle_drag(Handle::Middle(0));
    app.drag_handle_to((125.0, 57.5), SegmentDrag::Bend);
    app.end_handle_drag();
    assert!(matches!(
        &app.shape(3).unwrap().geometry,
        Geometry::Polyline { bulges, .. } if bulges[0] == 0.5
    ));
}

#[test]
fn handles_only_in_top_view_and_only_for_drawn_shapes() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 1)), false);
    // The rectangle: centre, 4 corners, rotation.
    assert_eq!(app.shape_handles().len(), 6);
    assert!(!app.polyline_selected());
    app.view_3d();
    assert!(app.shape_handles().is_empty());
    app.view_top();
    app.start_drawing(Tool::Circle);
    assert!(app.shape_handles().is_empty(), "not while drawing");
    app.cancel_drawing();
    // Imported shapes have none.
    let mut imported = lower_part();
    select(&mut imported, &[2]);
    assert!(imported.shape_handles().is_empty());
}

#[test]
fn drawing_snaps_to_the_drawing_unless_alt_is_held() {
    let mut app = drawn_part();
    app.view.scale = 10.0; // 8 px = 0.8 mm
    // Near the triangle's corner (110, 65): it snaps there and says so.
    let p = app.snap_world((110.3, 65.4), false, false);
    assert_eq!(p, (110.0, 65.0));
    let (_, hit) = app.drag.snapped.unwrap();
    assert!(
        crate::draw::snap_name(&hit.unwrap(), &crate::i18n::Language::En).starts_with("end of #")
    );
    // Alt: exactly the cursor.
    assert_eq!(app.snap_world((110.3, 65.4), true, false), (110.3, 65.4));
    // Far from everything: the grid (1 mm at 10 px/mm).
    assert_eq!(app.snap_world((20.3, 20.6), false, false), (20.0, 21.0));
    assert!(app.drag.snapped.unwrap().1.is_none());
}

/// A selected polyline's line, grabbed anywhere, moves that segment alone; Ctrl inserts a
/// point instead (the user, 2026-10-28).
#[test]
fn dragging_a_polyline_line_moves_that_segment() {
    let mut app = drawn_part();
    app.commit();
    // S3 is the triangle (110, 65) (140, 65) (125, 95); its bottom side is segment 0.
    app.click(Some(contour_of(&app, 3)), false);
    draw_canvas(&mut app);
    let grab = app.view.to_screen((115.0, 65.0));
    let (i, at) = app.polyline_segment_at(grab).expect("on the bottom side");
    assert_eq!(i, 0);
    app.start_segment_drag(i, at);
    // Grabbed left of the middle, dragged 10 mm down: the side moves 10 mm down.
    app.drag_handle_to((at.0, at.1 - 10.0), SegmentDrag::Move);
    app.end_handle_drag();
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    let near = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).abs() < 1e-6 && (p[1] - q[1]).abs() < 1e-6;
    assert!(near(points[0], [110.0, 55.0]) && near(points[1], [140.0, 55.0]));
    assert!(near(points[2], [125.0, 95.0]), "the far point stays");
    app.commit();

    app.click(Some(contour_of(&app, 3)), false);
    app.start_handle_drag(Handle::Middle(0));
    app.drag_handle_to((125.0, 50.0), SegmentDrag::Insert);
    app.end_handle_drag();
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points.len(), 4);
}

#[test]
fn shift_keeps_a_dragged_point_to_15_degree_steps() {
    let mut app = drawn_part();
    // S3 is the triangle (110, 65) (140, 65) (125, 95): its top point dragged nearly
    // straight right with Shift stays level, at grid steps (Alt off: free distance).
    app.click(Some(contour_of(&app, 3)), false);
    app.start_handle_drag(Handle::Point(2));
    let to = app.handle_point((160.3, 97.0), true, true);
    assert_eq!(to, (160.3, 95.0));
    // Without Shift it goes where the pointer is (Alt: no snapping).
    assert_eq!(app.handle_point((160.3, 97.0), true, false), (160.3, 97.0));
    // A segment's middle: Shift bends, so the point isn't held.
    app.end_handle_drag();
    app.start_handle_drag(Handle::Middle(0));
    assert_eq!(app.handle_point((130.0, 60.0), true, true), (130.0, 60.0));
    app.end_handle_drag();
}

#[test]
fn picked_points_are_dragged_together_in_one_undo_step() {
    let mut app = drawn_part();
    app.commit();
    // S3 is the triangle (110, 65) (140, 65) (125, 95): its bottom two points picked.
    app.click(Some(contour_of(&app, 3)), false);
    assert!(app.toggle_point(0) && app.toggle_point(1));
    assert_eq!(app.picked_points().len(), 2);
    draw_canvas(&mut app);
    assert_eq!(app.scene.picked_handles.len(), 2);
    // Picked again: left out, and back in.
    app.toggle_point(1);
    assert_eq!(app.picked_points().len(), 1);
    app.toggle_point(1);
    // Dragging one moves both by as much; the third stays.
    app.start_handle_drag(Handle::Point(1));
    app.drag_handle_to((140.0, 55.0), SegmentDrag::Move);
    app.end_handle_drag();
    app.commit();
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points[..], [[110.0, 55.0], [140.0, 55.0], [125.0, 95.0]]);
    app.undo();
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points[0], [110.0, 65.0], "one undo step");
    // An unpicked point alone moves alone.
    app.click(Some(contour_of(&app, 3)), false);
    app.start_handle_drag(Handle::Point(2));
    app.drag_handle_to((125.0, 100.0), SegmentDrag::Move);
    app.end_handle_drag();
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points[0], [110.0, 65.0]);
    // Another shape selected: the picked points are gone.
    app.click(Some(contour_of(&app, 1)), false);
    assert!(app.picked_points().is_empty());
    // A rectangle's handles aren't points to pick.
    assert!(!app.toggle_point(0));
}

#[test]
fn ctrl_click_on_a_point_in_the_window_picks_it() {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = drawn_part();
            app.fit_pending = true;
            app.click(Some(contour_of(&app, 3)), false);
            app
        });
    h.run_steps(6);
    let at = h.state().view.to_screen((110.0, 65.0));
    let click = |h: &mut egui_kittest::Harness<'_, ChiploadApp>, modifiers| {
        h.hover_at(at);
        h.run_steps(1);
        h.event(egui::Event::ModifiersChanged(modifiers));
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            });
            h.run_steps(1);
        }
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(2);
    };
    click(&mut h, egui::Modifiers::COMMAND);
    assert_eq!(h.state().selected_shape(), Some(3), "still selected");
    assert_eq!(
        h.state().picked_points().into_iter().collect::<Vec<_>>(),
        [0]
    );
    // Escape lets go of the points first; the shape stays selected.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(h.state().picked_points().is_empty());
    assert_eq!(h.state().selected_shape(), Some(3));
}
