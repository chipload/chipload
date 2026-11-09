// SPDX-License-Identifier: GPL-3.0-or-later
//! *New sign…* (docs/specs/sign-maker.md §3): the dialog shows the sign and makes it, the
//! words, the frame and the operations, as one undo step.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

/// The corpus's tools (1 an end mill, 2 a V-bit) in an empty project.
fn window() -> Harness<'static, ChiploadApp> {
    let project = corpus("jobs/v27/text.chipload");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&project));
            let edit = app.edit.as_mut().unwrap();
            edit.job.operations.clear();
            edit.job.drawing = chipload_pipeline::empty_project().drawing;
            app.commit();
            app
        });
    harness.run_steps(4);
    harness
}

fn click(harness: &mut Harness<'_, ChiploadApp>, label: &str) {
    let pos = harness
        .get_all_by_label(label)
        .last()
        .unwrap_or_else(|| panic!("{label}"))
        .rect()
        .center();
    harness.hover_at(pos);
    harness.run_steps(1);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

#[test]
fn new_sign_makes_the_whole_sign_as_one_undo_step() {
    let mut harness = window();
    harness.state_mut().start_sign();
    harness.run_steps(2);
    let Some(Prompt::Sign(settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    // The project's V-bit and end mill are chosen; the board is fitted (nothing drawn).
    assert_eq!(settings.vbit, Some(2));
    assert_eq!(settings.end_mill, Some(1));
    assert!(settings.fit_board);
    let mut settings = settings;
    settings.words = "OPEN".into();
    settings.edge = true;
    harness.state_mut().dialogs.prompt = Some(Prompt::Sign(settings));
    harness.run_steps(3);
    assert!(harness.get_all_by_label("Make sign").count() > 0);
    click(&mut harness, "Make sign");
    assert!(harness.state().dialogs.prompt.is_none());
    let job = &harness.state().edit.as_ref().unwrap().job;
    let names: Vec<&str> = job.operations.iter().map(|op| op.name()).collect();
    assert_eq!(names, ["Letters", "Borders", "Edge chamfer", "Cut out"]);
    assert_eq!(job.drawing.frames.len(), 1);
    harness.state_mut().undo();
    let job = &harness.state().edit.as_ref().unwrap().job;
    assert!(job.operations.is_empty() && job.drawing.shapes.is_empty());
}
