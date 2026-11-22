// SPDX-License-Identifier: GPL-3.0-or-later
//! The Scale tool (docs/specs/scale-and-stretch.md §2).

use super::*;
use crate::app::modify::ModifyTool;
use chipload_job::Geometry;

fn circle(app: &ChiploadApp, id: u64) -> ([f64; 2], f64) {
    match app.shape(id).unwrap().geometry {
        Geometry::Circle { center, radius } => (center, radius),
        _ => panic!("a circle"),
    }
}

fn points(app: &ChiploadApp, id: u64) -> Vec<[f64; 2]> {
    match &app.shape(id).unwrap().geometry {
        Geometry::Polyline { points, .. } => points.clone(),
        other => panic!("a polyline, got {other:?}"),
    }
}

#[test]
fn a_typed_length_scales_a_circle_about_the_first_point() {
    // S2 is the circle at (75, 80), r 12.
    let mut app = drawn_part();
    let (center, radius) = circle(&app, 2);
    app.start_modify(ModifyTool::Scale);
    // Nothing selected: a click picks the shape.
    let on = contour_of(&app, 2);
    app.scale_click(
        Some(on),
        (center[0] + radius, center[1]),
        false,
        (true, false),
    );
    assert_eq!(app.selected_drawn(), [2]);
    // Its centre, then its edge: the entry opens with the length measured.
    app.scale_click(None, (center[0], center[1]), false, (true, false));
    app.scale_click(None, (center[0] + radius, center[1]), false, (true, false));
    let typed = app.canvas_tools.typed.clone().unwrap();
    assert_eq!(typed.text, "12");
    // Esc goes back to the second point, and the entry closes.
    assert!(app.scale_back());
    assert!(app.canvas_tools.typed.is_none());
    assert_eq!(app.modify(), Some(ModifyTool::Scale));
    app.scale_click(None, (center[0] + radius, center[1]), false, (true, false));
    app.canvas_tools.typed.as_mut().unwrap().text = "2 * 12".into();
    app.apply_typed((0.0, 0.0));
    assert_eq!(circle(&app, 2), (center, 2.0 * radius));
    assert!(app.modify().is_none(), "the tool ends after scaling");
    app.undo();
    assert_eq!(circle(&app, 2), (center, radius));
}

#[test]
fn a_third_click_gives_the_new_length() {
    // S3 is the triangle (110, 65) (140, 65) (125, 95): its base, 30, made 60.
    let mut app = drawn_part();
    app.selection = CanvasSelection::Shapes([3].into_iter().collect());
    app.start_modify(ModifyTool::Scale);
    app.scale_click(None, (110.0, 65.0), false, (true, false));
    app.scale_click(None, (140.0, 65.0), false, (true, false));
    app.scale_click(None, (110.0, 125.0), false, (true, false));
    assert_eq!(
        points(&app, 3),
        vec![[110.0, 65.0], [170.0, 65.0], [140.0, 125.0]]
    );
    assert!(app.modify().is_none());
}

#[test]
fn a_length_of_zero_is_refused_and_the_entry_stays() {
    let mut app = drawn_part();
    app.selection = CanvasSelection::Shapes([3].into_iter().collect());
    app.start_modify(ModifyTool::Scale);
    app.scale_click(None, (110.0, 65.0), false, (true, false));
    app.scale_click(None, (140.0, 65.0), false, (true, false));
    app.canvas_tools.typed.as_mut().unwrap().text = "0".into();
    app.apply_typed((0.0, 0.0));
    assert!(app.canvas_tools.typed.as_ref().unwrap().refused);
    assert_eq!(app.modify(), Some(ModifyTool::Scale));
    assert_eq!(points(&app, 3)[1], [140.0, 65.0]);
}
