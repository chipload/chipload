// SPDX-License-Identifier: GPL-3.0-or-later
//! *Inlay…* (docs/specs/vcarve-inlay.md §4): the dialog makes the pocket and the plug's
//! setup as one undo step, and the plug's form has its *Plug* section.

use chipload_core::chain::ShapeFrom;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

fn window() -> Harness<'static, ChiploadApp> {
    let project = corpus("jobs/v27/text.chipload");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&project));
            app
        });
    harness.run_steps(4);
    harness
}

/// Click the last widget labelled `label`.
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

fn job<'a>(harness: &'a Harness<'_, ChiploadApp>) -> &'a chipload_job::Job {
    &harness.state().edit.as_ref().unwrap().job
}

#[test]
fn inlay_makes_the_pair_from_the_selection_as_one_undo_step() {
    let mut harness = window();
    let before = job(&harness).operations.len();
    // Nothing selected: the button is there but off.
    assert!(!harness.state().can_inlay());
    harness.state_mut().selection = CanvasSelection::whole(ShapeFrom::Text(2));
    harness.run_steps(2);
    assert!(harness.state().can_inlay());
    click(&mut harness, "Inlay…");
    // The project's V-bit and its smallest end mill are chosen.
    let Some(Prompt::Inlay(settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the inlay dialog is open");
    };
    assert_eq!(settings.tool, Some(2));
    assert_eq!(settings.floor, Some(1));
    assert!(harness.query_by_label("Glue gap").is_some());
    click(&mut harness, "Make inlay");
    assert!(harness.state().dialogs.prompt.is_none());
    let after = job(&harness);
    assert_eq!(
        after.operations.len(),
        before + 2,
        "{:?}",
        harness.state().report
    );
    assert_eq!(after.setups.len(), 2);
    let plug = after
        .operations
        .iter()
        .position(|op| matches!(op, Operation::Vcarve(v) if v.plug.is_some()))
        .unwrap();
    let part = after.part_of(&after.operations[plug]).unwrap();
    assert_eq!(part.mirror, Some(chipload_job::Mirror::LeftRight));
    // The plug's form shows its Plug section.
    harness.state_mut().focus = Focus::Op(plug);
    open_sections(&harness.ctx);
    harness.run_steps(3);
    assert!(harness.query_by_label("Plug start depth").is_some());
    // One undo step takes both away.
    harness.state_mut().undo();
    harness.run_steps(2);
    assert_eq!(job(&harness).operations.len(), before);
    assert_eq!(job(&harness).setups.len(), 1);
}
