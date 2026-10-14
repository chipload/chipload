// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn an_imported_image_is_traced_selected_as_a_whole_and_v_carved() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    // No project was open: a new one was made for it.
    assert!(app.edit.is_some());
    let dialog = app.image_dialog.as_mut().unwrap();
    assert_eq!(dialog.draft.layer, "logo");
    dialog.draft.width = 80.0;
    assert!(app.apply_image());
    assert!(app.image_dialog.is_none());
    let job = &app.edit.as_ref().unwrap().settings;
    assert_eq!(job.drawing.images.len(), 1);
    assert_eq!(
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.image() == Some(1))
            .count(),
        4
    );
    assert_eq!(
        app.selection,
        CanvasSelection::Source(chipload_core::chain::ShapeFrom::Image(1))
    );
    let index = app.add_operation(OpKind::Vcarve).unwrap();
    let op = &app.edit.as_ref().unwrap().ops[index].op;
    assert_eq!(op.select().image, Some(1), "the image itself");
    // One undo step takes the image away again (and one more the operation).
    app.undo();
    app.undo();
    assert!(
        app.edit
            .as_ref()
            .unwrap()
            .settings
            .drawing
            .images
            .is_empty()
    );
}

#[test]
fn clicked_traced_shapes_become_their_image_in_a_new_operation() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    app.image_dialog.as_mut().unwrap().draft.width = 80.0;
    assert!(app.apply_image());
    let first = app.edit.as_ref().unwrap().settings.drawing.shapes[0].id;
    app.selection = CanvasSelection::Shapes([first].into());
    let index = app.add_operation(OpKind::Vcarve).unwrap();
    let select = app.edit.as_ref().unwrap().ops[index].op.select().clone();
    assert_eq!(
        (select.image, select.layer.as_deref(), select.shapes),
        (Some(1), None, None)
    );
}

#[test]
fn an_image_is_changed_and_removed() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    app.image_dialog.as_mut().unwrap().draft.width = 80.0;
    assert!(app.apply_image());
    assert!(app.edit_image(1));
    app.image_dialog.as_mut().unwrap().draft.invert = true;
    assert!(app.apply_image());
    let job = &app.edit.as_ref().unwrap().settings;
    assert!(job.drawing.images[0].invert);
    assert_eq!(
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.image() == Some(1))
            .count(),
        5
    );
    assert!(app.remove_image(1));
    let job = &app.edit.as_ref().unwrap().settings;
    assert!(job.drawing.images.is_empty() && job.drawing.shapes.is_empty());
    assert!(!app.edit_image(1));
}

#[test]
fn a_file_that_is_no_picture_is_refused() {
    let mut app = ChiploadApp::default();
    assert!(!app.import_image(&corpus("dxf/simple_part.dxf")));
    assert!(app.image_dialog.is_none());
}

#[test]
fn the_image_window_traces_its_preview_in_the_background() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    let ctx = egui::Context::default();
    app.wait_for_image_preview(&ctx);
    let dialog = app.image_dialog.as_mut().unwrap();
    let Some(Ok(preview)) = &dialog.preview else {
        panic!("a preview");
    };
    assert_eq!(preview.trace.paths.len(), 4);
    assert_eq!(
        (preview.black_white.width, preview.black_white.height),
        (80, 40)
    );
    // A changed setting is traced again.
    dialog.draft.invert = true;
    app.wait_for_image_preview(&ctx);
    let Some(Ok(preview)) = &app.image_dialog.as_ref().unwrap().preview else {
        panic!("a preview");
    };
    assert!(preview.trace.black_at_border);
}

#[test]
fn the_image_window_previews_centre_lines_and_imports_them() {
    // docs/specs/image-arcs-centrelines.md §6.
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/strokes.png")));
    let ctx = egui::Context::default();
    let dialog = app.image_dialog.as_mut().unwrap();
    assert!(dialog.draft.arcs, "new images fit arcs");
    dialog.draft.width = 60.0;
    dialog.draft.trace = chipload_job::ImageTrace::CentreLine;
    app.wait_for_image_preview(&ctx);
    let Some(Ok(preview)) = &app.image_dialog.as_ref().unwrap().preview else {
        panic!("a preview");
    };
    assert!(preview.trace.centre_lines);
    assert_eq!(preview.trace.paths.len(), 4);
    assert_eq!(preview.trace.dots, 1);
    assert!(app.apply_image());
    let job = &app.edit.as_ref().unwrap().settings;
    assert_eq!(
        job.drawing.images[0].trace,
        chipload_job::ImageTrace::CentreLine
    );
    assert_eq!(
        job.drawing.shapes.iter().filter(|s| !s.closed()).count(),
        3,
        "the T's strokes"
    );
}

#[test]
fn a_hidden_toolpath_leaves_the_legend_and_the_plunges() {
    // docs/specs/drawing-sources.md §5.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    let drawing_only = crate::view::legend(&app.scene).len();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let plunge = app.scene.toolpath.as_ref().unwrap().starts[0].at;
    let at = app.view.to_screen(plunge);
    assert!(!app.scene.plunges_at(&app.view, at).is_empty());
    app.show_toolpath = false;
    app.scene.hide_toolpath = !app.show_toolpath;
    assert_eq!(crate::view::legend(&app.scene).len(), drawing_only);
    assert!(app.scene.plunges_at(&app.view, at).is_empty());
    // The toolpath itself stays: showing it again needs no new generation.
    assert!(app.scene.toolpath.is_some() && app.result.is_some());
}
