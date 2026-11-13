// SPDX-License-Identifier: GPL-3.0-or-later
//! *Relief…* (docs/specs/relief-wizard.md §6): six steps, each needing the ones before it;
//! *Make relief* adds the model's relief and what is round it as one undo step.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;
use crate::app::relief_wizard::wizard_of;
use chipload_pipeline::relief_wizard::Around;

fn window() -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v36/relief_dome.chipload")));
            app.start_relief();
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

fn step(harness: &mut Harness<'_, ChiploadApp>) -> usize {
    wizard_of(harness.state_mut()).map_or(0, |w| w.step())
}

#[test]
fn the_wizard_goes_step_by_step_and_makes_the_relief_as_one_undo_step() {
    let mut harness = window();
    let before = harness.state().edit.as_ref().unwrap().job.clone();
    // No model chosen: Next does nothing.
    click(&mut harness, "Next");
    assert_eq!(step(&mut harness), 1);
    click(&mut harness, "dome");
    click(&mut harness, "Next");
    assert_eq!(step(&mut harness), 2);
    click(&mut harness, "Next");
    assert_eq!(step(&mut harness), 3);
    click(&mut harness, "Next");
    assert_eq!(step(&mut harness), 4);
    wizard_of(harness.state_mut())
        .unwrap()
        .set_around(Around::Background);
    harness.run_steps(2);
    click(&mut harness, "Next");
    assert_eq!(step(&mut harness), 5);
    // Back keeps the answers.
    click(&mut harness, "Back");
    assert_eq!(step(&mut harness), 4);
    click(&mut harness, "Next");
    click(&mut harness, "Next");
    assert_eq!(step(&mut harness), 6);
    click(&mut harness, "Make relief");
    assert!(harness.state().dialogs.prompt.is_none());
    let job = harness.state().edit.as_ref().unwrap().job.clone();
    // A pocket and a relief in a new part, the relief selected.
    let added: Vec<&chipload_job::Operation> = job
        .operations
        .iter()
        .filter(|op| op.part() == job.active_part.unwrap())
        .collect();
    assert!(
        matches!(added[0], chipload_job::Operation::Pocket(_)),
        "{added:?}"
    );
    assert!(matches!(added[1], chipload_job::Operation::Relief(_)));
    assert!(matches!(harness.state().focus, Focus::Op(_)));
    harness.state_mut().undo();
    assert_eq!(harness.state().edit.as_ref().unwrap().job, before);
}

#[test]
fn cancel_changes_nothing() {
    let mut harness = window();
    let before = harness.state().edit.as_ref().unwrap().job.clone();
    let model = before.drawing.models[0].clone();
    wizard_of(harness.state_mut())
        .unwrap()
        .choose_existing(&model);
    harness.run_steps(2);
    click(&mut harness, "Cancel");
    assert!(harness.state().dialogs.prompt.is_none());
    assert_eq!(harness.state().edit.as_ref().unwrap().job, before);
}

/// The wizard's steps as pictures, to look at:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui relief_wizard_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; run by hand"]
fn relief_wizard_pictures() {
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_eframe(move |cc| {
            {
                let mut fonts = crate::fonts::ui_fonts();
                crate::fonts::add_text_fonts(&mut fonts);
                cc.egui_ctx.set_fonts(fonts);
            }
            crate::style::apply(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v36/relief_dome.chipload")));
            app.start_relief();
            let model = app.edit.as_ref().unwrap().job.drawing.models[0].clone();
            wizard_of(&mut app).unwrap().choose_existing(&model);
            wizard_of(&mut app).unwrap().set_around(Around::Background);
            app
        });
    harness.run_steps(4);
    for n in 1..=6 {
        harness.run_steps(4);
        let image = harness.render().unwrap();
        image
            .save(dir.join(format!("relief-wizard-{n}.png")))
            .unwrap();
        if n < 6 {
            click(&mut harness, "Next");
        }
    }
}
