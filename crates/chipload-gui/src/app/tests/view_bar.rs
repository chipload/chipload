// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::app::generation::Scope;

#[test]
fn the_canvas_bar_header_and_job_settings_draw_with_the_moved_settings() {
    let mut app = lower_part();
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.top_bar(ui);
        app.ribbon(ui);
        let rect = ui.max_rect();
        app.navigation_bar(ui, rect);
    });
    output.textures_delta.clear();
    app.focus = Focus::Job;
    draw_properties(&mut app);
}

/// User report 2026-10-14: "I still can't create a V-carve from a traced image". A
/// selector naming only the picture, {"image": 1}, was taken for "no geometry", and the new
/// V-carve took the endmill though the project had a V-bit.
#[test]
fn a_traced_picture_is_v_carved_end_to_end() {
    for (file, trace) in [
        ("images/logo.png", chipload_job::ImageTrace::Outline),
        ("images/hand_outline.png", chipload_job::ImageTrace::Outline),
        (
            "images/hand_outline.png",
            chipload_job::ImageTrace::CentreLine,
        ),
    ] {
        let mut app = ChiploadApp::default();
        assert!(app.import_image(&corpus(file)));
        app.image_dialog.as_mut().unwrap().draft.trace = trace;
        assert!(app.apply_image());
        // A V-bit in the project, after the default endmill.
        app.edit
            .as_mut()
            .unwrap()
            .job
            .tools
            .push(chipload_job::Tool {
                id: 2,
                name: None,
                library_uid: None,
                kind: ToolType::Vbit,
                diameter: 12.7,
                flutes: Some(2),
                center_cutting: Some(true),
                rpm: Some(18000.0),
                angle: Some(60.0_f64.to_radians()),
                tip_diameter: Some(0.2),
                corner_radius: None,
            });
        app.commit();
        // The picture is selected after importing: V-carve from the ribbon.
        let index = app.add_operation(OpKind::Vcarve).unwrap();
        let op = &app.edit.as_ref().unwrap().job.operations[index];
        assert_eq!(op.select().image(), Some(1), "{file}");
        assert_eq!(op.tool(), 2, "{file}: the V-bit");
        assert!(
            app.problems.is_empty(),
            "{file} {trace:?}: {:?}",
            app.problems
                .iter()
                .map(|p| problems::plain(&app, app.prefs.units, p))
                .collect::<Vec<_>>()
        );
        assert!(
            app.generate_and_wait(),
            "{file} {trace:?}: {:?}",
            app.report
        );
        assert!(app.toolpath.result.is_some());
        // A profile after it goes back to the endmill.
        let profile = app.add_operation(OpKind::Profile).unwrap();
        assert_eq!(app.edit.as_ref().unwrap().job.operations[profile].tool(), 1);
    }
}

/// docs/specs/parts-and-setups.md §8a: a part selected is shown alone, where it is
/// drawn; the setup selected shows every part on it; export always writes the setup.
#[test]
fn a_part_selected_is_shown_alone_and_export_is_of_the_setup() {
    let mut app = two_parts();
    let ops_of = |app: &ChiploadApp| -> std::collections::BTreeSet<usize> {
        app.toolpath
            .result
            .as_ref()
            .unwrap()
            .op_spans
            .iter()
            .filter(|s| !s.moves.is_empty())
            .map(|s| s.operation)
            .collect()
    };
    let of = |app: &ChiploadApp, part: u64| -> std::collections::BTreeSet<usize> {
        op_parts(app)
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == part)
            .map(|(i, _)| i)
            .collect()
    };
    // At start the setup is shown: every operation.
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert_eq!(app.toolpath.scope, Scope::Setup);
    assert_eq!(ops_of(&app).len(), op_parts(&app).len());
    // A part selected in the Browser: only its operations.
    app.focus = Focus::Part(2);
    app.follow_focus();
    assert!(app.wait_for_generation());
    assert_eq!(app.toolpath.scope, Scope::View(2));
    assert_eq!(ops_of(&app), of(&app, 2));
    // One of the other part's operations: that part instead.
    let first = *of(&app, 1).iter().next().unwrap();
    app.focus = Focus::Op(first);
    app.follow_focus();
    assert!(app.wait_for_generation());
    assert_eq!(app.toolpath.scope, Scope::View(1));
    assert_eq!(ops_of(&app), of(&app, 1));
    // Tools or a shape leave it as it is.
    app.focus = Focus::Tools;
    app.follow_focus();
    assert!(!app.generating());
    // Export: the setup is planned first, then written.
    let dir = temp_dir("part-preview-export");
    let path = dir.join("two.nc");
    assert!(
        !app.export_gcode(&path),
        "not yet: the setup is planned first"
    );
    assert!(app.generating());
    assert!(app.wait_for_generation());
    assert_eq!(app.toolpath.scope, Scope::Setup);
    let whole = chipload_pipeline::run_job(&app.edit.as_ref().unwrap().to_job().unwrap()).unwrap();
    let written: Vec<String> = whole
        .files
        .iter()
        .map(|f| std::fs::read_to_string(f.path(&path)).unwrap())
        .collect();
    let expected: Vec<String> = whole.files.iter().map(|f| f.gcode.clone()).collect();
    assert_eq!(written, expected);
    // The setup selected: the setup again (docs/specs/drawing-and-setup-spaces.md §2).
    show_setup(&mut app);
    assert_eq!(app.display.part, None);
}

/// docs/specs/view-and-settings.md §2: Drawing off hides the contours and their picking;
/// a new selection or a shape being drawn brings them back.
#[test]
fn the_drawing_can_be_hidden_and_comes_back_to_edit() {
    let mut app = lower_part();
    draw_canvas(&mut app);
    let contour = app.contours[0].clone();
    let shape = app
        .scene
        .contours
        .iter()
        .find(|c| c.id == contour.id)
        .unwrap();
    let at = app.view.to_screen(shape.points[0]);
    assert_eq!(app.scene.pick(&app.view, at), Some(contour.id));
    app.click(Some(contour.id), false);
    let selected = app.selection.clone();

    app.set_show_drawing(false);
    draw_canvas(&mut app);
    assert!(!app.show_drawing() && app.scene.hide_drawing);
    assert_eq!(app.scene.pick(&app.view, at), None);
    assert!(app.scene.handles.is_empty());
    // The selection it was hidden with, or none (Esc), keeps it hidden.
    assert_eq!(app.selection, selected);
    app.selection = CanvasSelection::Nothing;
    draw_canvas(&mut app);
    assert!(!app.show_drawing());
    // Selecting a layer in the Browser shows it again.
    let layer = contour.layers()[0].clone();
    app.select_layer(&layer);
    draw_canvas(&mut app);
    assert!(app.show_drawing() && !app.scene.hide_drawing);

    // So does starting to draw a shape.
    app.set_show_drawing(false);
    draw_canvas(&mut app);
    assert!(!app.show_drawing());
    app.start_drawing(Tool::Circle);
    draw_canvas(&mut app);
    assert!(app.show_drawing());
}

/// User report 2026-10-16: the Images button did nothing. The pictures were drawn under the
/// simulated stock (on by default) and only in top view; now over the stock, in 3D too.
#[test]
fn pictures_show_over_the_stock_and_in_3d() {
    let ctx = egui::Context::default();
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    assert!(app.apply_image());
    app.sync_pictures(&ctx);
    assert_eq!(app.scene.pictures.len(), 1);
    app.display.images = false;
    app.sync_pictures(&ctx);
    assert!(app.scene.pictures.is_empty());
    app.display.images = true;
    app.sync_pictures(&ctx);
    // Painted in 3D: the picture's mesh is on the canvas.
    app.view_3d();
    let meshes = |app: &mut ChiploadApp| {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.canvas(ui));
        output.textures_delta.clear();
        let textured = |s: &&egui::epaint::ClippedShape| match &s.shape {
            egui::Shape::Mesh(m) => m.texture_id != egui::TextureId::default(),
            _ => false,
        };
        output.shapes.iter().filter(textured).count()
    };
    assert_eq!(meshes(&mut app), 1);
    app.display.images = false;
    app.sync_pictures(&ctx);
    assert_eq!(meshes(&mut app), 0);
}
