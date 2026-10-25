// SPDX-License-Identifier: GPL-3.0-or-later
//! The box round selected drawn shapes (docs/specs/drawn-shapes-box.md §2).
use super::*;
use chipload_pipeline::box_handles::BoxHandle;

fn polyline_points(app: &ChiploadApp, id: u64) -> Vec<[f64; 2]> {
    let Geometry::Polyline { points, .. } = &app.shape(id).unwrap().geometry else {
        panic!("polyline");
    };
    points.clone()
}

#[test]
fn a_polyline_and_several_shapes_get_a_box_but_one_circle_does_not() {
    let mut app = drawn_part();
    // S3, the triangle: a box with move, turn and four corners.
    app.click(Some(contour_of(&app, 3)), false);
    assert_eq!(app.box_handle_points().len(), 6);
    // The circle alone has its own handles.
    app.click(Some(contour_of(&app, 2)), false);
    assert!(app.box_handle_points().is_empty());
    // The circle and the triangle together: a box.
    app.click(Some(contour_of(&app, 3)), true);
    assert_eq!(app.box_handle_points().len(), 6);
}

#[test]
fn moving_the_box_moves_the_shapes_in_one_undo_step() {
    let mut app = drawn_part();
    app.commit();
    app.click(Some(contour_of(&app, 3)), false);
    app.click(Some(contour_of(&app, 2)), true);
    let before = polyline_points(&app, 3);
    let center = app.box_handle_points()[0].1;
    app.start_box_drag(BoxHandle::Move);
    app.drag_box_to((center.0 + 10.0, center.1 - 5.0), false);
    // Nothing changes until the drag ends; the outline shows the shapes where they go.
    assert_eq!(polyline_points(&app, 3), before);
    assert!(app.box_outline().len() > 2);
    app.end_box_drag();
    let after = polyline_points(&app, 3);
    for (a, b) in before.iter().zip(&after) {
        assert!((b[0] - a[0] - 10.0).abs() < 1e-9 && (b[1] - a[1] + 5.0).abs() < 1e-9);
    }
    let Geometry::Circle { center: c, .. } = app.shape(2).unwrap().geometry else {
        panic!("circle");
    };
    assert!((c[0] - 85.0).abs() < 1e-9 && (c[1] - 75.0).abs() < 1e-9);
    // Still selected, with the same ids.
    assert_eq!(app.deletable_shapes().0, vec![2, 3]);
    app.commit();
    app.undo();
    assert_eq!(polyline_points(&app, 3), before);
}

#[test]
fn turning_and_sizing_the_box_turns_and_scales_about_its_centre() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 3)), false);
    let center = app.box_handle_points()[0].1;
    // Straight up from the centre: a quarter turn.
    app.start_box_drag(BoxHandle::Turn);
    app.drag_box_to((center.0, center.1 + 40.0), false);
    app.end_box_drag();
    let turned = polyline_points(&app, 3);
    // (110, 65) is 15 left and 15 below the box's centre (125, 80): a quarter turn takes
    // it 15 right and 15 below.
    assert!((turned[0][0] - 140.0).abs() < 1e-9 && (turned[0][1] - 65.0).abs() < 1e-9);
    // Corner 2 (upper right) dragged out: the shapes grow, the lower-left corner stays.
    let handles = app.box_handle_points();
    let lower_left = handles[2].1;
    let upper_right = handles[4].1;
    app.start_box_drag(BoxHandle::Size(2));
    let far = (
        lower_left.0 + 2.0 * (upper_right.0 - lower_left.0),
        lower_left.1 + 2.0 * (upper_right.1 - lower_left.1),
    );
    app.drag_box_to(far, false);
    app.end_box_drag();
    let handles = app.box_handle_points();
    assert!((handles[2].1.0 - lower_left.0).abs() < 1.0);
    let grown = polyline_points(&app, 3);
    let side = |p: &[[f64; 2]]| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]);
    assert!(side(&grown) > 1.9 * side(&turned), "{grown:?}");
}

#[test]
fn imported_shapes_get_no_box() {
    let mut app = two_parts();
    let first = app.edit.as_ref().unwrap().settings.drawing.shapes[0].id;
    app.selection = CanvasSelection::Shapes(BTreeSet::from([first]));
    assert!(app.box_handle_points().is_empty());
}

/// Position and Turn in Properties (docs/specs/drawn-shapes-box.md §3).
#[test]
fn position_and_turn_move_and_turn_the_box() {
    use crate::shape_form::{PlaceEdit, geometry_box};
    let mut app = drawn_part();
    // The triangle's box is (110, 65) to (140, 95).
    let triangle = app.shape(3).unwrap().geometry.clone();
    let bounds = geometry_box(&triangle).unwrap();
    assert_eq!(bounds, ((110.0, 65.0), (140.0, 95.0)));
    let (from, to, scale, degrees) = PlaceEdit::CornerTo(0.0, 0.0).transform(bounds);
    assert!(app.transform_shapes(&[3], from, to, scale, degrees));
    assert_eq!(
        polyline_points(&app, 3),
        vec![[0.0, 0.0], [30.0, 0.0], [15.0, 30.0]]
    );
    // A quarter turn about the middle of its box (15, 15).
    let bounds = geometry_box(&app.shape(3).unwrap().geometry).unwrap();
    let (from, to, scale, degrees) = PlaceEdit::TurnBy(90.0).transform(bounds);
    assert!(app.transform_shapes(&[3], from, to, scale, degrees));
    assert_eq!(
        polyline_points(&app, 3),
        vec![[30.0, 0.0], [30.0, 30.0], [0.0, 15.0]]
    );
    // Several shapes turn together about the box round them all.
    let (from, to, scale, degrees) =
        PlaceEdit::TurnBy(180.0).transform(((0.0, 0.0), (100.0, 100.0)));
    assert!(app.transform_shapes(&[2, 3], from, to, scale, degrees));
    let Geometry::Circle { center, .. } = app.shape(2).unwrap().geometry else {
        panic!("circle");
    };
    assert_eq!(center, [25.0, 20.0]);
}

/// Drawn shapes in the Browser, by layer, with names (docs/specs/drawn-shapes-box.md §4).
#[test]
fn drawn_groups_list_their_shapes_by_layer_with_names() {
    let mut app = drawn_part();
    app.name_shape(3, "  Lid ").unwrap();
    assert_eq!(app.shape(3).unwrap().name.as_deref(), Some("Lid"));
    let groups = app.drawing_groups();
    let polylines = groups.iter().find(|g| g.key == "drawn:polyline").unwrap();
    assert_eq!(
        polylines.members,
        vec![("drawn".to_string(), vec![(3, Some("Lid".to_string()))])]
    );
    // An empty name takes it away.
    app.name_shape(3, "").unwrap();
    assert_eq!(app.shape(3).unwrap().name, None);
    // DXFs keep their layer rows, without shape rows.
    let app = two_parts();
    assert!(app.drawing_groups().iter().all(|g| match g.kind {
        super::super::sources::GroupKind::Dxf(_) => g.members.is_empty(),
        _ => true,
    }));
}
