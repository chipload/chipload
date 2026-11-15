// SPDX-License-Identifier: GPL-3.0-or-later
//! The fill pattern in the window (docs/specs/fill-pattern.md §8): S then E adds one for
//! closed shapes with the V-bit, and its properties show the pattern's settings.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

#[test]
fn s_then_e_adds_a_pattern_with_the_v_bit_and_it_generates() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/pattern.chipload")));
    let edit = app.edit.as_mut().unwrap();
    while !edit.job.operations.is_empty() {
        edit.remove(0);
    }
    // The board and its two holes.
    let ids: Vec<usize> = app.contours.iter().take(3).map(|c| c.id).collect();
    select(&mut app, &ids);
    assert!(app.quick_unfit(OpKind::Pattern).is_none());
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
    harness.key_press(egui::Key::E);
    harness.run_steps(2);
    let app = harness.state_mut();
    let Operation::Pattern(f) = app.edit.as_ref().unwrap().job.operations[0].clone() else {
        panic!("a pattern");
    };
    assert_eq!(f.name, "Pattern 1");
    assert_eq!(f.tool, 2, "the V-bit");
    assert_eq!(f.end_depth, 0.5);
    assert!(f.inside);
    assert!(matches!(f.pattern, chipload_job::FillPattern::Lines { .. }));
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn the_form_shows_the_pattern_and_its_settings() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v36/pattern.chipload")));
            app.focus = Focus::Op(0);
            app
        });
    harness.run_steps(4);
    harness.get_by_label("Pattern   cross-hatch, 4 mm apart");
    harness.get_by_label("Angle between");
    harness.get_by_label("Inside the edge");
    harness.get_by_label("Ends");
    harness.get_by_label("each groove 1.4 mm wide at the top");
}

/// The window with the pattern sample's first operation open and the S menu, rendered
/// (wgpu) to `CHIPLOAD_SHOTS_OUT`; looked at by hand.
#[test]
#[ignore = "a picture for looking at the pattern"]
fn pattern_picture() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/pattern.chipload")));
    let ids: Vec<usize> = app.contours.iter().take(3).map(|c| c.id).collect();
    select(&mut app, &ids);
    app.focus = Focus::Op(0);
    let mut app = Some(app);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            crate::style::apply(&cc.egui_ctx);
            open_sections(&cc.egui_ctx);
            app.take().unwrap()
        });
    harness.run_steps(4);
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.state_mut().quick_menu = Some(egui::pos2(700.0, 450.0));
    harness.run_steps(4);
    let out = std::env::var("CHIPLOAD_SHOTS_OUT").unwrap_or_else(|_| ".".into());
    harness
        .render()
        .unwrap()
        .save(std::path::PathBuf::from(out).join("pattern.png"))
        .unwrap();
}
