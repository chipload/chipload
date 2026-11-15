// SPDX-License-Identifier: GPL-3.0-or-later
//! The prism in the window (docs/specs/prism.md §5): S then R adds one for closed shapes
//! with the V-bit, and its properties show the max depth and how deep the edges go.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

#[test]
fn s_then_r_adds_a_prism_with_the_v_bit_and_it_generates() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/prism.chipload")));
    let edit = app.edit.as_mut().unwrap();
    while !edit.job.operations.is_empty() {
        edit.remove(0);
    }
    let first = app.contours[0].id;
    select(&mut app, &[first]);
    assert!(app.quick_unfit(OpKind::Prism).is_none());
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
    harness.key_press(egui::Key::R);
    harness.run_steps(2);
    let app = harness.state_mut();
    let Operation::Prism(f) = app.edit.as_ref().unwrap().job.operations[0].clone() else {
        panic!("a prism");
    };
    assert_eq!(f.name, "Prism 1");
    assert_eq!(f.tool, 2, "the first V-bit");
    assert_eq!(f.max_depth, None);
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn the_form_shows_the_max_depth_and_the_edges() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v36/prism.chipload")));
            app.focus = Focus::Op(1);
            app
        });
    harness.run_steps(4);
    harness.get_by_label("Flat top");
    harness.get_by_label("edges at most 4 mm deep with the 60° bit");
    harness.get_by_label("Cut   edges at most 4 mm deep, climb");
}
