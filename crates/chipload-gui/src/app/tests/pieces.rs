// SPDX-License-Identifier: GPL-3.0-or-later
//! A setup cut in pieces in the window (docs/specs/pieces.md §9): its pieces drawn in the
//! view, the preview the pieces put together, export a program per piece with one sheet,
//! and the setup's form turning it on and off.
use super::*;

#[test]
fn a_setup_in_pieces_is_drawn_previewed_and_exported_piece_by_piece() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v37/pieces.chipload")));
    let names: Vec<&str> = app.scene.pieces.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["A1", "A2"]);
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let result = app.toolpath.result.as_ref().unwrap();
    // Every piece's files, named after it.
    let suffixes: Vec<String> = result
        .files
        .iter()
        .map(|f| f.suffix.clone().unwrap_or_default())
        .collect();
    assert_eq!(suffixes, ["-A1-1-T1", "-A1-2-T2", "-A2-1-T1", "-A2-2-T2"]);
    let dir = temp_dir("pieces-export");
    assert!(app.export_gcode(&dir.join("table.nc")));
    for name in ["table-A1-1-T1.nc", "table-A2-2-T2.nc", "table-sheet.html"] {
        assert!(dir.join(name).exists(), "{name}");
    }
    let sheet = std::fs::read_to_string(dir.join("table-sheet.html")).unwrap();
    assert!(sheet.contains("Piece A2"), "the sheet's pieces");
    draw_properties(&mut app);
}

#[test]
fn the_setup_form_turns_pieces_on_and_off() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v37/pieces.chipload")));
            app.focus = Focus::Setup(1);
            app
        });
    h.run_steps(6);
    if h.query_by_label("Cut in pieces").is_none() {
        h.get_by_label("Pieces").click();
        h.run_steps(3);
    }
    h.get_by_label("Cut in pieces").click();
    h.run_steps(6);
    assert!(h.state().edited_job().unwrap().setups[0].pieces.is_none());
    assert!(h.state().scene.pieces.is_empty());
    // On again: the joint's defaults on boards of the machine's travel or 600 × 400.
    h.get_by_label("Cut in pieces").click();
    h.run_steps(6);
    let pieces = h.state().edited_job().unwrap().setups[0]
        .pieces
        .clone()
        .unwrap();
    assert_eq!(pieces.joint, chipload_job::Joint::Knobs);
    assert_eq!(pieces.size, 30.0);
}
