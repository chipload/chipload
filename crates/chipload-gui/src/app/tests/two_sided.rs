// SPDX-License-Identifier: GPL-3.0-or-later
//! *Two-sided…* (docs/specs/two-sided-wizard.md §4): the dialog makes the back side with its
//! dowel pins as one undo step, changes one there is, and widens a fitted board's margin
//! only when asked.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::*;
use crate::app::wizard_tools::WizardTool;

/// The two-sided sample with its back side taken away: one fitted board, nothing turned.
fn window() -> Harness<'static, ChiploadApp> {
    let project = corpus("jobs/v36/two_sided.chipload");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&project));
            let job = app.edited_job().unwrap();
            let job = chipload_pipeline::dowels::remove_dowels(&job, 2).unwrap();
            let (job, _) = chipload_pipeline::setups::remove_setup(&job, 2, true).unwrap();
            app.edit.as_mut().unwrap().adopt(job);
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

fn settings(harness: &Harness<'_, ChiploadApp>) -> super::super::two_sided::TwoSidedSettings {
    match harness.state().dialogs.prompt.clone() {
        Some(Prompt::TwoSided(settings)) => *settings,
        other => panic!("the dialog, not {:?}", other.is_some()),
    }
}

#[test]
fn two_sided_makes_the_back_and_its_pins_as_one_undo_step() {
    let mut harness = window();
    assert!(harness.state().edited_job().unwrap().back_of(1).is_none());
    click(&mut harness, "Two-sided");
    let s = settings(&harness);
    // The defaults: left to right, by pins, 8 mm pins 8 mm deep, the zero at the first pin,
    // the end mill that bores them (decision 8).
    assert_eq!(s.front, 1);
    assert_eq!((s.diameter, s.depth, s.zero), (8.0, 8.0, true));
    assert!(matches!(s.tool, Some(WizardTool::Project(_))));
    assert!(harness.query_by_label("Pin diameter").is_some());
    click(&mut harness, "Make the back side");
    assert!(
        harness.state().dialogs.prompt.is_none(),
        "{:?}",
        harness.state().report
    );
    let job = harness.state().edited_job().unwrap();
    let back = job.back_of(1).expect("the back").id;
    assert!(!job.missing_dowels(back));
    assert_eq!(job.active_setup_id(), Some(back));
    assert_eq!(harness.state().focus, Focus::Setup(back));
    assert!(matches!(
        job.setup(1).unwrap().work_zero,
        chipload_job::WorkZero::Point { .. }
    ));
    // Opened again from the front's form: it changes the back, the holes made again.
    harness.state_mut().start_two_sided_for(1);
    harness.run_steps(2);
    let mut s = settings(&harness);
    s.turn = chipload_job::Turn::TopBottom;
    s.diameter = 6.0;
    harness.state_mut().dialogs.prompt = Some(Prompt::TwoSided(Box::new(s)));
    harness.run_steps(3);
    click(&mut harness, "Change the back side");
    let job = harness.state().edited_job().unwrap();
    let link = job.setup(back).unwrap().back_of.unwrap();
    assert_eq!(link.turn, chipload_job::Turn::TopBottom);
    assert_eq!(job.operations.iter().filter(|op| op.is_dowels()).count(), 1);
    // Two undo steps: the change, then the back.
    harness.state_mut().undo();
    harness.state_mut().undo();
    assert!(harness.state().edited_job().unwrap().back_of(1).is_none());
}

#[test]
fn pins_too_wide_offer_to_widen_the_margin_and_only_then_the_board_grows() {
    let mut harness = window();
    let fit = harness
        .state()
        .edited_job()
        .unwrap()
        .setup(1)
        .unwrap()
        .stock
        .fit;
    harness.state_mut().start_two_sided();
    harness.run_steps(2);
    let mut s = settings(&harness);
    s.diameter = 20.0;
    harness.state_mut().dialogs.prompt = Some(Prompt::TwoSided(Box::new(s)));
    harness.run_steps(3);
    // No room: Make is off, the margin that holds them is offered.
    assert!(
        harness
            .query_by_label_contains("No room for the pins")
            .is_some()
    );
    assert_eq!(
        harness
            .state()
            .edited_job()
            .unwrap()
            .setup(1)
            .unwrap()
            .stock
            .fit,
        fit
    );
    click_contains(&mut harness, "Widen the margin to");
    let s = settings(&harness);
    let margin = s.margin.expect("widened in the dialog");
    // Not yet on the board: only when the back is made.
    assert_eq!(
        harness
            .state()
            .edited_job()
            .unwrap()
            .setup(1)
            .unwrap()
            .stock
            .fit,
        fit
    );
    assert!(
        harness
            .query_by_label_contains("saw it to the new size")
            .is_some()
    );
    click(&mut harness, "Make the back side");
    let job = harness.state().edited_job().unwrap();
    assert_eq!(job.setup(1).unwrap().stock.fit, Some(margin));
    assert!(job.back_of(1).is_some());
}

fn click_contains(harness: &mut Harness<'_, ChiploadApp>, text: &str) {
    let label = harness
        .get_all_by_label_contains(text)
        .next()
        .unwrap_or_else(|| panic!("{text}"))
        .accesskit_node()
        .label()
        .unwrap_or_default();
    click(harness, &label);
}
