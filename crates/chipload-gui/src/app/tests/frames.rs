// SPDX-License-Identifier: GPL-3.0-or-later
//! Draw > Frame… (docs/specs/shape-library.md §5, §9).
use chipload_core::chain::ShapeFrom;
use chipload_job::Geometry;

use super::*;
use crate::app::modify::ModifyTool;

fn shapes(app: &ChiploadApp) -> &[chipload_job::Shape] {
    &app.edit.as_ref().unwrap().settings.drawing.shapes
}

#[test]
fn a_frame_round_a_selected_text_is_one_undo_step() {
    let mut app = ChiploadApp::default();
    app.new_project();
    app.start_text();
    let id = app.place_text_at((20.0, 30.0)).unwrap();
    let letters = shapes(&app).len();
    app.start_frame();
    assert_eq!(app.modify, Some(ModifyTool::Frame));
    assert_eq!(app.focus, Focus::Selection);
    // The text stays selected: the frame goes round it, its form under Selection.
    assert_eq!(app.selection, CanvasSelection::Source(ShapeFrom::Text(id)));
    app.frame.kind = chipload_pipeline::frames::FrameKind::Oval;
    app.frame.borders = 2;
    draw_properties(&mut app);
    assert!(!app.frame_outlines().is_empty(), "the preview");
    assert!(app.create_frame());
    assert_eq!(app.modify, None, "the tool ends");
    let added: Vec<_> = shapes(&app)[letters..].to_vec();
    assert_eq!(added.len(), 3);
    assert_eq!(added[0].layer, "frame");
    assert!(added[1..].iter().all(|s| s.layer == "border"));
    app.undo();
    assert_eq!(shapes(&app).len(), letters);
}

#[test]
fn with_nothing_selected_a_frame_is_drawn_to_a_size() {
    let mut app = ChiploadApp::default();
    app.new_project();
    app.start_frame();
    assert_eq!(app.selection, CanvasSelection::Nothing);
    app.frame.kind = chipload_pipeline::frames::FrameKind::Rounded;
    (app.frame.width, app.frame.height) = (200.0, 120.0);
    app.frame.corner_set = true;
    app.frame.corner = 12.0;
    app.frame.borders = 0;
    draw_properties(&mut app);
    assert!(app.create_frame());
    let [shape] = shapes(&app) else {
        panic!("{:?}", shapes(&app));
    };
    let shape = shape.clone();
    let Geometry::Rectangle {
        length,
        width,
        corner_radius,
        ..
    } = shape.geometry
    else {
        panic!("{:?}", shape.geometry);
    };
    assert_eq!((length, width, corner_radius), (200.0, 120.0, 12.0));

    // The new frame is selected: a frame now would go round it.
    assert_eq!(
        app.selection,
        CanvasSelection::Shapes([shape.id].into_iter().collect())
    );
    // A corner too large: no Add, the reason in the form, nothing added.
    app.selection = CanvasSelection::Nothing;
    app.start_frame();
    app.frame.corner = 90.0;
    draw_properties(&mut app);
    assert!(matches!(app.frame_preview.place, Some(Err(_))));
    assert!(!app.create_frame());
    assert_eq!(shapes(&app).len(), 1);
}
