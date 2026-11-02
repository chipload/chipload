// SPDX-License-Identifier: GPL-3.0-or-later
//! One canvas tool at a time (the review of 2026-11-02, point 3): starting one ends any
//! other, and Esc and the 3D view end whichever is in use.

use super::*;
use crate::app::modify::ModifyTool;

fn project() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
    app
}

/// Which tools are in use: drawing, editing, placing a text, measuring.
fn tools(app: &ChiploadApp) -> [bool; 4] {
    [
        app.sketch().is_some(),
        app.modify().is_some(),
        app.placing_text(),
        app.measure().is_some(),
    ]
}

#[test]
fn starting_a_tool_ends_the_one_in_use() {
    let mut app = project();
    app.start_modify(ModifyTool::Offset);
    // Measuring used to leave the offset tool on, which took the clicks.
    app.toggle_measure();
    assert_eq!(tools(&app), [false, false, false, true]);
    app.start_modify(ModifyTool::Trim);
    assert_eq!(tools(&app), [false, true, false, false]);
    // Placing a text used to leave the editing tool on.
    app.start_text();
    assert_eq!(tools(&app), [false, false, true, false]);
    app.start_drawing(Tool::Circle);
    assert_eq!(tools(&app), [true, false, false, false]);
    // M again: measuring, then off.
    app.toggle_measure();
    app.toggle_measure();
    assert_eq!(tools(&app), [false; 4]);
}

#[test]
fn the_3d_view_ends_every_tool() {
    let mut app = project();
    for start in [
        |a: &mut ChiploadApp| a.start_modify(ModifyTool::Fillet),
        |a: &mut ChiploadApp| a.start_text(),
        |a: &mut ChiploadApp| a.toggle_measure(),
        |a: &mut ChiploadApp| a.start_drawing(Tool::Rectangle),
    ] {
        app.view_top();
        start(&mut app);
        assert!(app.tool_active());
        app.view_3d();
        assert_eq!(tools(&app), [false; 4]);
    }
}

#[test]
fn escape_ends_the_tool_then_the_selection() {
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = project();
            select(&mut app, &[2]);
            app.start_modify(ModifyTool::Offset);
            app
        });
    harness.run_steps(2);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(1);
    assert_eq!(tools(harness.state()), [false; 4]);
    assert_ne!(harness.state().selection, CanvasSelection::Nothing);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(1);
    assert_eq!(harness.state().selection, CanvasSelection::Nothing);
}
