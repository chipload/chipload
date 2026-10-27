// SPDX-License-Identifier: GPL-3.0-or-later
//! Box selection and moving drawn shapes with the mouse (docs/specs/drawing-input.md §3).
use super::*;

fn selected(app: &ChiploadApp) -> BTreeSet<u64> {
    match &app.selection {
        CanvasSelection::Shapes(s) => s.clone(),
        CanvasSelection::Nothing => BTreeSet::new(),
        other => panic!("{other:?}"),
    }
}

fn box_select(app: &mut ChiploadApp, from: Point, to: Point, add: bool, toggle: bool) {
    app.start_select_drag(None, from, add, false);
    app.drag_select_to(to);
    app.end_select_drag(add, toggle);
}

#[test]
fn a_box_selects_what_is_inside_or_what_it_touches() {
    // Circle S2 at (75, 80) r 12, triangle S3 (110, 65), (140, 65), (125, 95), both inside
    // the turned rectangle S1.
    let mut app = drawn_part();
    assert!(app.drag_selects());
    // Left to right round the circle: only the circle.
    box_select(&mut app, (60.0, 60.0), (90.0, 100.0), false, false);
    assert_eq!(selected(&app), [2].into());
    // Right to left over the triangle's left side only: it touches the triangle.
    box_select(&mut app, (118.0, 70.0), (100.0, 75.0), false, false);
    assert_eq!(selected(&app), [3].into());
    // The same box left to right catches nothing, and clears the selection.
    box_select(&mut app, (100.0, 70.0), (118.0, 75.0), false, false);
    assert!(selected(&app).is_empty());
    // Shift adds, Ctrl toggles.
    box_select(&mut app, (60.0, 60.0), (90.0, 100.0), false, false);
    box_select(&mut app, (105.0, 60.0), (145.0, 100.0), true, false);
    assert_eq!(selected(&app), [2, 3].into());
    box_select(&mut app, (60.0, 60.0), (90.0, 100.0), false, true);
    assert_eq!(selected(&app), [3].into());
    // Round everything.
    box_select(&mut app, (0.0, 0.0), (300.0, 200.0), false, false);
    assert_eq!(selected(&app), [1, 2, 3].into());
}

#[test]
fn dragging_a_drawn_shape_moves_it_keeping_its_id() {
    let mut app = drawn_part();
    app.view.scale = 10.0;
    app.click(None, false);
    let circle = contour_of(&app, 2);
    // Grab the circle's right quadrant (87, 80) and drop it on the grid at (100, 50).
    app.start_select_drag(Some(circle), (87.1, 80.2), false, false);
    assert_eq!(
        selected(&app),
        [2].into(),
        "an unselected shape is selected first"
    );
    app.drag_move_to((100.3, 49.8), false, false);
    app.end_move_drag();
    assert_eq!(
        app.shape(2).unwrap().geometry,
        Geometry::Circle {
            center: [88.0, 50.0],
            radius: 12.0
        }
    );
    app.commit();
    // With both drawn shapes selected, dragging one moves both; Shift holds the move to
    // 15° steps (straight up here).
    box_select(&mut app, (60.0, 30.0), (145.0, 100.0), false, false);
    assert_eq!(selected(&app), [2, 3].into());
    let before = app.shape(3).unwrap().geometry.clone();
    app.start_select_drag(Some(contour_of(&app, 3)), (140.0, 65.0), false, false);
    app.drag_move_to((140.4, 75.2), false, true);
    app.end_move_drag();
    assert_eq!(
        app.shape(2).unwrap().geometry,
        Geometry::Circle {
            center: [88.0, 60.0],
            radius: 12.0
        }
    );
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!()
    };
    let Geometry::Polyline { points: was, .. } = &before else {
        panic!()
    };
    assert_eq!(points[0], [was[0][0], was[0][1] + 10.0]);
    // Ids are kept, so the selection still holds them.
    assert_eq!(selected(&app), [2, 3].into());
}

#[test]
fn a_drag_on_an_imported_contour_selects_with_a_box() {
    let mut app = lower_part();
    let picked = Some(app.contours[0].id);
    app.start_select_drag(picked, (0.0, 0.0), false, false);
    assert!(app.drag.moving.is_none());
    assert!(app.drag.select_box.is_some());
}
