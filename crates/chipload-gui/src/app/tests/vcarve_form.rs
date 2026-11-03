// SPDX-License-Identifier: GPL-3.0-or-later
//! The V-carve form's depth passes (docs/specs/vcarve-step-down.md §3).

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::*;

/// Open the Depth passes list, showing `chosen`.
fn open_choice(harness: &mut Harness<'_, ChiploadApp>, chosen: &str) {
    harness
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| n.accesskit_node().value().as_deref() == Some(chosen))
        .expect("the depth passes list")
        .click();
    harness.run_steps(3);
}

fn vcarve(harness: &Harness<'_, ChiploadApp>) -> chipload_job::VcarveOp {
    match &harness.state().edit.as_ref().unwrap().job.operations[0] {
        Operation::Vcarve(v) => v.clone(),
        _ => unreachable!(),
    }
}

#[test]
fn a_vcarve_switches_between_equal_load_and_a_step_down() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v34/vcarve_step_down.chipload")));
            app.focus = Focus::Op(0);
            app
        });
    harness.run_steps(4);
    // 3.5 mm in 4 passes of equal load: the first to 1.75 mm.
    assert_eq!(vcarve(&harness).step_down, None);
    harness.get_by_label("Cut   from 0 mm, at most 3.5 mm deep, 4 passes");
    open_choice(&mut harness, "Equal load");
    harness.get_by_label("Step down").click();
    harness.run_steps(3);
    let v = vcarve(&harness);
    // The first pass stays as deep as it was; the count is kept for going back.
    assert_eq!(v.step_down, Some(1.8));
    assert_eq!(v.passes, 4);
    assert_eq!(
        chipload_pipeline::vcarve_pass_depths(&v),
        [1.75, 3.5],
        "two equal steps of at most 1.8 mm"
    );
    harness.get_by_value("to 1.75, 3.5 mm");
    harness.get_by_label("Cut   from 0 mm, at most 3.5 mm deep, 2 passes");
    // And back.
    open_choice(&mut harness, "Step down");
    harness.get_by_label("Equal load").click();
    harness.run_steps(2);
    let v = vcarve(&harness);
    assert_eq!(v.step_down, None);
    assert_eq!(v.passes, 4);
}
