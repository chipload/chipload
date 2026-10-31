// SPDX-License-Identifier: GPL-3.0-or-later
//! The Delete key on the canvas: drawn shapes, and a text, a picture or a DXF as a whole
//! (the user, 2026-10-31: "all shapes should be deletable via canvas").
use chipload_core::chain::ShapeFrom;

use super::*;

fn drawing(app: &ChiploadApp) -> &chipload_job::Drawing {
    &app.edit.as_ref().unwrap().settings.drawing
}

#[test]
fn delete_removes_a_selected_text_and_a_box_round_a_sign() {
    let mut app = ChiploadApp::default();
    app.new_project();
    app.start_text();
    let id = app.place_text_at((20.0, 30.0)).unwrap();
    // The text selected, as a click on a letter selects it.
    assert_eq!(app.selection, CanvasSelection::Source(ShapeFrom::Text(id)));
    app.delete_selected_shapes();
    assert!(drawing(&app).texts.is_empty());
    assert!(drawing(&app).shapes.is_empty());
    app.undo();
    assert_eq!(drawing(&app).texts.len(), 1);

    // A box round the text and a frame: its letters and the frame, as one undo step.
    app.start_frame();
    assert!(app.create_frame());
    // A frame of the window between the two (undo steps are taken per frame).
    app.commit();
    let all: std::collections::BTreeSet<u64> = drawing(&app).shapes.iter().map(|s| s.id).collect();
    app.selection = CanvasSelection::Shapes(all);
    app.delete_selected_shapes();
    assert!(drawing(&app).texts.is_empty());
    assert!(
        drawing(&app).shapes.is_empty(),
        "{:?}",
        drawing(&app).shapes
    );
    app.undo();
    assert_eq!(drawing(&app).texts.len(), 1);
    assert!(drawing(&app).shapes.iter().any(|s| s.is_drawn()));
}

#[test]
fn delete_removes_a_selected_picture_or_dxf_but_not_one_imported_shape() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v19/image_vcarve.chipload")));
    let image = drawing(&app).images[0].id;
    app.selection = CanvasSelection::Source(ShapeFrom::Image(image));
    app.delete_selected_shapes();
    assert!(drawing(&app).images.is_empty());
    app.undo();
    assert_eq!(drawing(&app).images.len(), 1);

    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v21/two_dxfs.chipload")));
    let dxfs = drawing(&app).dxfs.len();
    let first = drawing(&app).dxfs[0].id;
    // One shape of a DXF: it stays.
    let one = drawing(&app)
        .shapes
        .iter()
        .find(|s| !s.is_drawn())
        .unwrap()
        .id;
    app.selection = CanvasSelection::Shapes([one].into());
    app.delete_selected_shapes();
    assert!(drawing(&app).shapes.iter().any(|s| s.id == one));
    // The DXF as a whole: removed.
    app.selection = CanvasSelection::Source(ShapeFrom::Dxf(first));
    app.delete_selected_shapes();
    assert_eq!(drawing(&app).dxfs.len(), dxfs - 1);
}
