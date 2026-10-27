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
    // 3 points, and the box round it: move, turn, 4 corners (drawn-shapes-box.md §2).
    assert_eq!(app.scene.handles.len(), 3 + 6);
    assert_eq!(app.scene.middle_handles.len(), 3);
    assert!(app.polyline_selected());
    app.start_handle_drag(Handle::Point(2));
    app.drag_handle_to((125.0, 100.0), false);
    app.drag_handle_to((126.0, 105.0), false);
    // Onto its neighbour: refused, the last valid shape stays and the form says why.
    app.drag_handle_to((140.0, 65.0), false);
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
    app.drag_handle_to((125.0, 57.5), true);
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
