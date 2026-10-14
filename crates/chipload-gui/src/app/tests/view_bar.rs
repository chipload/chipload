// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

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
            .settings
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
            });
        app.commit();
        // The picture is selected after importing: V-carve from the ribbon.
        let index = app.add_operation(OpKind::Vcarve).unwrap();
        let op = &app.edit.as_ref().unwrap().ops[index].op;
        assert_eq!(op.select().image, Some(1), "{file}");
        assert_eq!(op.tool(), 2, "{file}: the V-bit");
        assert!(
            app.problems.is_empty(),
            "{file} {trace:?}: {:?}",
            app.problems.iter().map(problems::plain).collect::<Vec<_>>()
        );
        assert!(
            app.generate_and_wait(),
            "{file} {trace:?}: {:?}",
            app.report
        );
        assert!(app.result.is_some());
        // A profile after it goes back to the endmill.
        let profile = app.add_operation(OpKind::Profile).unwrap();
        assert_eq!(app.edit.as_ref().unwrap().ops[profile].op.tool(), 1);
    }
}

/// docs/specs/active-part-preview.md: with several parts, the preview is of the active
/// part; All parts shows all; export always writes every part.
#[test]
fn the_preview_is_of_the_active_part_and_export_is_of_all() {
    let mut app = two_parts();
    let active = app
        .edit
        .as_ref()
        .unwrap()
        .settings
        .active_part_id()
        .unwrap();
    let ops_of = |app: &ChiploadApp| -> std::collections::BTreeSet<usize> {
        app.result
            .as_ref()
            .unwrap()
            .op_spans
            .iter()
            .filter(|s| !s.moves.is_empty())
            .map(|s| s.operation)
            .collect()
    };
    let mine: std::collections::BTreeSet<usize> = op_parts(&app)
        .iter()
        .enumerate()
        .filter(|(_, part)| **part == active)
        .map(|(i, _)| i)
        .collect();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert_eq!(app.result_part, Some(active));
    assert_eq!(ops_of(&app), mine, "only the active part's operations");
    // All parts: every operation.
    app.set_all_parts(true);
    assert!(app.wait_for_generation());
    assert_eq!(app.result_part, None);
    assert_eq!(ops_of(&app).len(), op_parts(&app).len());
    // Back to the part, then export: every part is planned again and written.
    app.set_all_parts(false);
    assert!(app.wait_for_generation());
    assert_eq!(app.result_part, Some(active));
    let dir = temp_dir("part-preview-export");
    let path = dir.join("two.nc");
    assert!(
        !app.export_gcode(&path),
        "not yet: every part is planned first"
    );
    assert!(app.generating());
    assert!(app.wait_for_generation());
    assert_eq!(app.result_part, None);
    let whole = chipload_pipeline::run_job(&app.edit.as_ref().unwrap().to_job().unwrap()).unwrap();
    let written: Vec<String> = whole
        .files
        .iter()
        .map(|f| std::fs::read_to_string(f.path(&path)).unwrap())
        .collect();
    let expected: Vec<String> = whole.files.iter().map(|f| f.gcode.clone()).collect();
    assert_eq!(written, expected);
}
