// SPDX-License-Identifier: GPL-3.0-or-later
//! The shape library's tab (docs/specs/shape-library.md §5.1): saved from the selection,
//! dragged onto the canvas, selected shapes carried onto the tab.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::super::shape_library::RightTab;
use super::*;

/// A settings folder of its own for each test.
fn settings(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "chipload-gui-shape-library-{}-{test}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The sign (a text framed, S5 the outline, S6 its border), everything selected.
fn sign(settings: PathBuf) -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v35/sign_frame.chipload")));
            app.settings_dir = Some(settings);
            let all = app
                .edit
                .as_ref()
                .unwrap()
                .job
                .drawing
                .shapes
                .iter()
                .map(|s| s.id)
                .collect();
            app.selection = CanvasSelection::Shapes(all);
            app.focus = Focus::Selection;
            app
        });
    harness.run_steps(4);
    harness
}

/// The panel's own button `label` (the top bar has a Save too).
fn panel_button(harness: &Harness<'_, ChiploadApp>, label: &str) -> egui::Pos2 {
    harness
        .get_all_by_label(label)
        .map(|n| n.rect().center())
        .find(|p| p.x > 700.0)
        .expect(label)
}

fn click_at(harness: &mut Harness<'_, ChiploadApp>, pos: egui::Pos2) {
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run_steps(3);
}

fn shape_count(harness: &Harness<'_, ChiploadApp>) -> usize {
    harness
        .state()
        .edit
        .as_ref()
        .unwrap()
        .job
        .drawing
        .shapes
        .len()
}

#[test]
fn the_selection_is_saved_then_dragged_onto_the_canvas() {
    let dir = settings("save-drag");
    let mut harness = sign(dir.clone());
    harness.get_by_label("SHAPES").click();
    harness.run_steps(2);
    assert_eq!(harness.state().right_tab, RightTab::Shapes);
    harness.get_by_label("Save selection…").click();
    harness.run_steps(3);
    // The text's words are the name suggested.
    let save = panel_button(&harness, "Save");
    click_at(&mut harness, save);
    let file = dir.join("shapes/SIGN.chipload-shapes");
    assert!(file.is_file(), "saved");
    // Saved again under the same name: asked first.
    harness.get_by_label("Save selection…").click();
    harness.run_steps(3);
    let save = panel_button(&harness, "Save");
    click_at(&mut harness, save);
    harness.get_by_label("Replace");
    harness.get_by_label("Cancel").click();
    harness.run_steps(2);

    // Dragged from its cell onto the canvas: inserted there, one undo step, selected.
    let before = shape_count(&harness);
    let cell = harness.get_by_label("SIGN").rect().center();
    let canvas = egui::pos2(500.0, 450.0);
    harness.drag_at(cell);
    harness.run_steps(2);
    harness.hover_at(cell + egui::vec2(10.0, 10.0));
    harness.run_steps(2);
    harness.hover_at(canvas);
    harness.run_steps(2);
    harness.event(egui::Event::PointerButton {
        pos: canvas,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(3);
    let added = shape_count(&harness) - before;
    assert_eq!(added, 6, "the four letters, the frame and its border");
    let app = harness.state();
    let at = app.view.to_world(canvas);
    let new: Vec<u64> = app.edit.as_ref().unwrap().job.drawing.shapes[before..]
        .iter()
        .map(|s| s.id)
        .collect();
    // More than one thing: it comes in as a group, selected whole
    // (docs/specs/shape-groups.md, decision 5).
    let group = app
        .selected_group()
        .unwrap_or_else(|| panic!("{:?}", app.selection));
    let members = app.edit.as_ref().unwrap().job.drawing.group_shapes(group);
    assert_eq!(members, new);
    let ((x0, y0), (x1, y1)) =
        chipload_pipeline::shapes::drawn_box(&app.edit.as_ref().unwrap().job, &new).unwrap();
    // Snapped to the grid near the pointer.
    assert!(((x0 + x1) / 2.0 - at.0).abs() < 5.0 && ((y0 + y1) / 2.0 - at.1).abs() < 5.0);
    assert_eq!(app.edit.as_ref().unwrap().job.drawing.texts.len(), 2);
}

#[test]
fn selected_shapes_carried_onto_the_tab_are_saved_not_moved() {
    let dir = settings("carry");
    let mut harness = sign(dir.clone());
    let before = harness.state().edit.as_ref().unwrap().job.drawing.clone();
    // The frame's left side (S5: 132.3 wide round (150, 80)), dragged onto the tab's name.
    let grab = harness
        .state()
        .view
        .to_screen((150.0 - 132.314 / 2.0, 80.0));
    let tab = harness.get_by_label("SHAPES").rect().center();
    harness.drag_at(grab);
    harness.run_steps(2);
    for step in 1..=4 {
        let t = step as f32 / 4.0;
        harness.hover_at(grab + (tab - grab) * t);
        harness.run_steps(2);
    }
    harness.drop_at(tab);
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.edit.as_ref().unwrap().job.drawing, before, "not moved");
    assert_eq!(app.right_tab, RightTab::Shapes);
    assert!(matches!(
        app.shape_library.asking,
        Some(super::super::shape_library::Asking::Save { .. })
    ));
}

#[test]
fn a_browser_item_brings_properties_back_and_the_canvas_selection_does_not() {
    let dir = settings("tabs");
    let mut harness = sign(dir);
    harness.get_by_label("SHAPES").click();
    harness.run_steps(2);
    harness.state_mut().selection = CanvasSelection::Shapes([5].into());
    harness.run_steps(2);
    assert_eq!(harness.state().right_tab, RightTab::Shapes);
    harness.state_mut().focus = Focus::Op(0);
    harness.run_steps(2);
    assert_eq!(harness.state().right_tab, RightTab::Properties);
}

#[test]
fn an_entry_dragged_onto_a_group_moves_into_it() {
    let dir = settings("move");
    let job = chipload_job::load_job(&corpus("jobs/v35/sign_frame.chipload")).unwrap();
    let entry = chipload_pipeline::shape_library::entry_from(&job, &[5], &[]).unwrap();
    let shapes = dir.join("shapes");
    chipload_pipeline::shape_library::save(&shapes, "Plate", &entry, false).unwrap();
    chipload_pipeline::shape_library::save(&shapes, "Frames/Oval", &entry, false).unwrap();
    let mut harness = sign(dir);
    harness.get_by_label("SHAPES").click();
    harness.run_steps(3);
    let from = harness.get_by_label("Plate").rect().center();
    let to = harness.get_by_label("Oval").rect().center();
    harness.drag_at(from);
    harness.run_steps(2);
    for step in 1..=4 {
        let t = step as f32 / 4.0;
        harness.hover_at(from + (to - from) * t);
        harness.run_steps(2);
    }
    harness.drop_at(to);
    harness.run_steps(3);
    assert!(shapes.join("Frames/Plate.chipload-shapes").is_file());
    assert!(!shapes.join("Plate.chipload-shapes").exists());
}
