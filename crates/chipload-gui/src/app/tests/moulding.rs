// SPDX-License-Identifier: GPL-3.0-or-later
//! The moulding in the window (docs/specs/moulding.md §9): S then M adds one with the ball
//! nose, and its properties show the profile, side and ends.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

#[test]
fn s_then_m_adds_a_moulding_with_the_ball_nose_and_it_generates() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/moulding.chipload")));
    let edit = app.edit.as_mut().unwrap();
    while !edit.job.operations.is_empty() {
        edit.remove(0);
    }
    // The rectangle.
    let rect = app.contours.iter().find(|c| c.closed).unwrap().id;
    select(&mut app, &[rect]);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            app
        });
    harness.run_steps(2);
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.run_steps(1);
    harness.key_press(egui::Key::S);
    harness.run_steps(2);
    harness.key_press(egui::Key::M);
    harness.run_steps(2);
    let app = harness.state_mut();
    let Operation::Moulding(m) = app.edit.as_ref().unwrap().job.operations[0].clone() else {
        panic!("a moulding");
    };
    assert_eq!(m.name, "Moulding 1");
    assert_eq!(m.tool, 1, "the ball nose");
    assert_eq!(m.side, chipload_job::MouldSide::Outside);
    assert!(matches!(m.profile, chipload_job::MouldProfile::Cove { .. }));
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn the_form_shows_the_profile_side_and_ends() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v36/moulding.chipload")));
            app.focus = Focus::Op(2);
            app
        });
    harness.run_steps(4);
    harness.get_by_label("Profile   cove, left of the path");
    harness.get_by_label("  Taper length");
    harness.get_by_label("Stepover");
}
