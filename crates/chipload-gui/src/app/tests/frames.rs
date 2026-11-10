// SPDX-License-Identifier: GPL-3.0-or-later
//! Draw > Frame… (docs/specs/shape-library.md §5, §9).
use chipload_core::chain::ShapeFrom;
use chipload_job::Geometry;

use super::*;
use crate::app::modify::ModifyTool;

fn shapes(app: &ChiploadApp) -> &[chipload_job::Shape] {
    &app.edit.as_ref().unwrap().job.drawing.shapes
}

#[test]
fn a_frame_round_a_selected_text_is_one_undo_step() {
    let mut app = ChiploadApp::default();
    app.new_project();
    app.start_text();
    let id = app.place_text_at((20.0, 30.0)).unwrap();
    let letters = shapes(&app).len();
    app.start_frame();
    assert_eq!(app.modify(), Some(ModifyTool::Frame));
    assert_eq!(app.focus, Focus::Selection);
    // The text stays selected: the frame goes round it, its form under Selection.
    assert_eq!(app.selection, CanvasSelection::whole(ShapeFrom::Text(id)));
    // A click on empty space (where the preview is drawn) keeps the selection; a click on
    // a letter keeps the frame's values open (the user, 2026-10-31).
    app.modify_click(None, (500.0, 500.0), false, (false, false));
    assert_eq!(app.selection, CanvasSelection::whole(ShapeFrom::Text(id)));
    let letter = app
        .contours
        .iter()
        .find(|c| {
            c.parts
                .iter()
                .all(|p| p.segment.from == Some(ShapeFrom::Text(id)))
        })
        .unwrap()
        .id;
    app.modify_click(Some(letter), (20.0, 30.0), false, (false, false));
    assert_eq!(app.selection, CanvasSelection::whole(ShapeFrom::Text(id)));
    assert_eq!(app.focus, Focus::Selection);
    assert_eq!(app.modify(), Some(ModifyTool::Frame));
    app.canvas_tools.frame.kind = chipload_pipeline::frames::FrameKind::Oval;
    app.canvas_tools.frame.borders = 2;
    draw_properties(&mut app);
    assert!(!app.frame_outlines().is_empty(), "the preview");
    assert!(app.create_frame());
    assert_eq!(app.modify(), None, "the tool ends");
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
    app.canvas_tools.frame.kind = chipload_pipeline::frames::FrameKind::Rounded;
    (app.canvas_tools.frame.width, app.canvas_tools.frame.height) = (200.0, 120.0);
    app.canvas_tools.frame.corner_set = true;
    app.canvas_tools.frame.corner = 12.0;
    app.canvas_tools.frame.borders = 0;
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
    app.canvas_tools.frame.corner = 90.0;
    draw_properties(&mut app);
    assert!(matches!(app.canvas_tools.frame_preview.place, Some(Err(_))));
    assert!(!app.create_frame());
    assert_eq!(shapes(&app).len(), 1);
}

fn outline_width(app: &ChiploadApp, id: u64) -> f64 {
    match &shapes(app).iter().find(|s| s.id == id).unwrap().geometry {
        Geometry::Rectangle { length, .. } => *length,
        other => panic!("{other:?}"),
    }
}

/// docs/specs/sign-maker.md §4: new words draw the frame again round them, in the same
/// undo step; a frame moved alone is let go of.
#[test]
fn a_frame_follows_its_text_in_the_same_undo_step() {
    let mut app = ChiploadApp::default();
    app.new_project();
    app.start_text();
    let id = app.place_text_at((20.0, 30.0)).unwrap();
    app.start_frame();
    app.canvas_tools.frame.kind = chipload_pipeline::frames::FrameKind::Rounded;
    app.canvas_tools.frame.borders = 1;
    assert!(app.create_frame());
    app.commit();
    let job = &app.edit.as_ref().unwrap().job;
    let frame = job.drawing.frames[0].clone();
    let before = outline_width(&app, frame.outline);

    let mut text = job.drawing.texts[0].clone();
    text.text = format!("{} AND MORE WORDS", text.text);
    assert!(app.set_text(id, &text));
    let after = outline_width(&app, frame.outline);
    assert!(after > before + 20.0, "{before} → {after}");
    assert_eq!(app.edit.as_ref().unwrap().job.drawing.frames.len(), 1);
    // Its values under Selection when the frame is selected.
    app.selection = CanvasSelection::Shapes([frame.outline].into_iter().collect());
    app.focus = Focus::Selection;
    draw_properties(&mut app);
    app.undo();
    assert!((outline_width(&app, frame.outline) - before).abs() < 1e-9);

    // Moved alone: plain shapes from then on.
    app.transform_shapes(&[frame.outline], (0.0, 0.0), (5.0, 0.0), 1.0, 0.0);
    app.commit();
    assert!(app.edit.as_ref().unwrap().job.drawing.frames.is_empty());
}

/// docs/specs/sign-maker.md §5: a stepped frame with a double border of its own corners;
/// changed under Selection afterwards.
#[test]
fn a_frame_with_corners_and_double_borders_from_the_window() {
    use chipload_pipeline::frames::{CornerStyle, FrameKind};
    let mut app = ChiploadApp::default();
    app.new_project();
    app.start_frame();
    let f = &mut app.canvas_tools.frame;
    f.kind = FrameKind::Stepped;
    (f.width, f.height) = (240.0, 120.0);
    f.borders = 1;
    f.double = true;
    f.border_width = 2.0;
    f.border_corners = Some(CornerStyle::Round);
    draw_properties(&mut app);
    assert!(app.create_frame());
    app.commit();
    let frame = app.edit.as_ref().unwrap().job.drawing.frames[0].clone();
    assert_eq!(frame.kind, chipload_job::FrameShape::Stepped);
    assert_eq!(frame.borders.len(), 2);
    assert_eq!(
        frame.border_corners,
        Some(chipload_job::FrameCorners::Round)
    );
    // Its card shows under Selection.
    app.selection = CanvasSelection::Shapes([frame.outline].into_iter().collect());
    app.focus = Focus::Selection;
    draw_properties(&mut app);
    // Another frame that isn't rectangular: its borders follow its shape again.
    app.canvas_tools.frame.kind = FrameKind::Shield;
    let change = chipload_pipeline::frames::FrameChange {
        kind: Some(FrameKind::Shield),
        ..Default::default()
    };
    let job = app.edit.as_ref().unwrap().job.clone();
    let result = chipload_pipeline::frames::set_frame(&job, frame.outline, &change);
    assert!(app.take_edit(result, &[]));
    app.commit();
    let frame = &app.edit.as_ref().unwrap().job.drawing.frames[0];
    assert_eq!(frame.border_corners, None);
}

/// The ribbon's Frame button went (the user, 2026-11-10: WIZARDS > Sign makes a sign); a
/// frame alone is the canvas' right-click on a selection, or Draw > Frame….
#[test]
fn right_click_on_a_selection_offers_a_frame_round_it() {
    use egui_kittest::kittest::Queryable;
    let mut app = ChiploadApp::default();
    app.new_project();
    app.start_text();
    app.place_text_at((20.0, 30.0)).unwrap();
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(400.0, 600.0))
        .build_ui_state(|ui, app: &mut ChiploadApp| app.shape_menu(ui), app);
    harness.run();
    harness.get_by_label("Frame round the selection…").click();
    harness.run();
    assert_eq!(harness.state().modify(), Some(ModifyTool::Frame));
}
