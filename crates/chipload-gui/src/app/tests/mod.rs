// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use chipload_job::{HoleMethod, Lead};
use chipload_toolpath::model::MoveKind;

mod browser;
mod browser_drawing_folder;
mod chip_load;
mod cycle_time;
mod disk_changes;
mod drawing;
mod generating;
mod handles;
mod header;
mod holding;
mod images;
mod leave_out_shapes;
mod live_problems;
mod parts;
mod posts;
mod profile_form;
mod properties_sources;
mod recovery_copies;
mod texts;
mod three_d_view;
mod tool_library;
mod view_bar;
mod workflows;

fn corpus(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-gui-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn select(app: &mut ChiploadApp, ids: &[usize]) {
    app.selection = CanvasSelection::of_contours(&ids.iter().copied().collect(), &app.contours);
}

/// messy_2 imported: a pocket on the crossing bars, then the lower outline.
fn lower_part() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
    select(&mut app, &[4, 6]);
    app.add_operation(OpKind::Pocket).unwrap();
    select(&mut app, &[2]);
    app.add_operation(OpKind::Profile).unwrap();
    app
}

fn first_cut(app: &ChiploadApp) -> (f64, f64) {
    let moves = &app.result.as_ref().unwrap().program.moves;
    let m = moves
        .iter()
        .find(|m| m.kind == MoveKind::Rapid && m.x.is_some())
        .unwrap();
    (m.x.unwrap(), m.y.unwrap())
}

fn drawn_part() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
    app
}

fn contour_of(app: &ChiploadApp, shape: u64) -> usize {
    app.contours
        .iter()
        .find(|c| c.parts.iter().any(|p| p.segment.shape == Some(shape)))
        .unwrap()
        .id
}

/// Run the keyboard shortcuts for one frame with `key` pressed.
fn press(app: &mut ChiploadApp, key: egui::Key) {
    press_with(app, egui::Modifiers::NONE, key);
}

fn press_with(app: &mut ChiploadApp, modifiers: egui::Modifiers, key: egui::Key) {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
        ..Default::default()
    };
    ctx.begin_pass(input);
    app.shortcuts(&ctx);
    // The font atlas a first frame makes isn't painted anywhere here.
    ctx.end_pass().textures_delta.clear();
}

/// Draw the canvas and the status bar for one frame.
fn draw_canvas(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.status_bar(ui);
        app.canvas(ui);
    });
    output.textures_delta.clear();
}

/// Draw the left sidebar for one frame.
fn draw_sidebar(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.sidebar(ui));
    // The font atlas a first frame makes isn't painted anywhere here.
    output.textures_delta.clear();
}

/// Change operation 1's feed and record it as an edit.
fn edit_feed(app: &mut ChiploadApp, feed: f64) {
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = feed,
        _ => panic!("operation 1 is a pocket"),
    }
    app.commit();
}

/// An app with its own settings folder (and so a recovery session).
fn with_settings(name: &str) -> (ChiploadApp, PathBuf) {
    let dir = temp_dir(name);
    let _ = std::fs::remove_dir_all(dir.join("recovery"));
    let mut app = ChiploadApp::default();
    app.set_settings_dir(Some(dir.clone()));
    (app, dir)
}

/// Draw the properties panel for one frame.
fn draw_properties(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.properties_panel(ui));
    output.textures_delta.clear();
}

/// The two-part sample: part 1 with operations 1–3, part 2 ("part B") with operation 4.
fn two_parts() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v22/two_parts.chipload")));
    app
}

fn op_parts(app: &ChiploadApp) -> Vec<u64> {
    app.edit
        .as_ref()
        .unwrap()
        .ops
        .iter()
        .map(|e| e.op.part())
        .collect()
}

#[test]
fn import_create_operations_generate_save_and_run_without_the_dxf() {
    let mut app = lower_part();
    assert!(app.dirty(), "a new project is unsaved");
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let dir = temp_dir("save");
    let project = dir.join("lower.chipload");
    let gcode_path = dir.join("lower.nc");
    assert!(app.save_project(&project));
    assert!(!app.dirty());
    assert!(app.export_gcode(&gcode_path));
    // The saved project is self-contained: run_job on it gives the exported G-code.
    let job = load_job(&project).unwrap();
    assert!(!job.drawing.shapes.is_empty());
    let again = chipload_pipeline::run_job(&job).unwrap();
    assert_eq!(
        std::fs::read_to_string(&gcode_path).unwrap(),
        again.files[0].gcode
    );
    // Reopening it shows the same contours.
    let mut reopened = ChiploadApp::default();
    assert!(reopened.open(&project));
    assert_eq!(reopened.contours, app.contours);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_simulation_is_on_from_the_start_and_shows_what_the_tools_leave() {
    // docs/specs/material-left-merge.md: the simulation, not a separate shading, shows the
    // material left, and the report has no "can't reach" lines any more.
    let mut app = lower_part();
    assert!(app.show_simulation);
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let ctx = egui::Context::default();
    assert!(app.wait_for_simulation(&ctx));
    assert!(app.sim.simulation.is_some());
    assert!(
        !app.report
            .iter()
            .any(|l| l.contains("can't reach material"))
    );
}
