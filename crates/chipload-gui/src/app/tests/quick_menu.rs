// SPDX-License-Identifier: GPL-3.0-or-later
//! The Q menu (docs/specs/fewer-clicks.md §2).
use super::super::quick_menu::QuickChoice;
use super::styles::app_with_library;
use super::*;

fn ops(app: &ChiploadApp) -> Vec<chipload_job::Operation> {
    app.edit.as_ref().unwrap().job.operations.clone()
}

fn window(app: ChiploadApp) -> egui_kittest::Harness<'static, ChiploadApp> {
    let mut app = Some(app);
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            app.take().unwrap()
        });
    harness.run_steps(2);
    // A new project opens with the cursor in the board's thickness: out of it, as a click
    // on the canvas would.
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.run_steps(1);
    harness
}

#[test]
fn a_letter_adds_what_the_ribbon_adds() {
    // By the ribbon.
    let mut by_hand = app_with_library("quick-ribbon");
    select(&mut by_hand, &[2, 3, 4]);
    by_hand.add_operation(OpKind::Pocket).unwrap();
    // By Q, K.
    let mut app = app_with_library("quick-keys");
    select(&mut app, &[2, 3, 4]);
    let mut harness = window(app);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    assert!(harness.state().quick_menu_open());
    harness.key_press(egui::Key::K);
    harness.run_steps(2);
    assert!(!harness.state().quick_menu_open());
    assert_eq!(ops(harness.state()), ops(&by_hand));
    // P is a profile while the menu is open, not Panels.
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    harness.key_press(egui::Key::P);
    harness.run_steps(2);
    let last = ops(harness.state()).last().cloned().unwrap();
    assert!(matches!(last, chipload_job::Operation::Profile(_)));
}

#[test]
fn escape_closes_and_nothing_selected_says_so() {
    let mut app = app_with_library("quick-escape");
    app.selection = CanvasSelection::Nothing;
    let mut harness = window(app);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    assert!(!harness.state().quick_menu_open());
    let status = harness.state().status.clone().unwrap().0;
    assert!(status.contains("then Q"), "{status}");
    select(harness.state_mut(), &[1]);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    assert!(harness.state().quick_menu_open());
    harness.key_press(egui::Key::Escape);
    harness.run_steps(2);
    assert!(!harness.state().quick_menu_open());
    assert!(ops(harness.state()).is_empty());
    // Escape closed the menu only: the selection stays.
    assert_ne!(harness.state().selection, CanvasSelection::Nothing);
}

#[test]
fn a_kind_the_selection_cant_take_is_greyed_and_refused() {
    let mut app = app_with_library("quick-greyed");
    // The outline alone: no circle to drill.
    select(&mut app, &[1]);
    assert!(app.quick_unfit(OpKind::Holes).is_some());
    assert!(app.quick_unfit(OpKind::Pocket).is_none());
    app.quick_menu = Some(egui::pos2(100.0, 100.0));
    app.quick_choose(QuickChoice::Kind(OpKind::Holes));
    assert!(ops(&app).is_empty());
    assert!(app.quick_menu_open(), "stays open for another choice");
    // The holes are circles.
    select(&mut app, &[2, 3, 4]);
    assert!(app.quick_unfit(OpKind::Holes).is_none());
}

#[test]
fn a_digit_applies_the_style_in_the_list_order() {
    let mut app = app_with_library("quick-style");
    select(&mut app, &[1]);
    let i = app.add_operation(OpKind::Profile).unwrap();
    assert!(app.save_style(i, "Cut out"));
    app.undo();
    assert!(ops(&app).is_empty());
    select(&mut app, &[1]);
    let mut harness = window(app);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    harness.key_press(egui::Key::Num1);
    harness.run_steps(2);
    let names: Vec<String> = ops(harness.state())
        .iter()
        .map(|op| op.name().to_string())
        .collect();
    assert_eq!(names, ["Cut out"]);
    // A digit with no style: nothing.
    select(harness.state_mut(), &[2]);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    harness.key_press(egui::Key::Num2);
    harness.run_steps(2);
    assert_eq!(ops(harness.state()).len(), 1);
    assert!(harness.state().quick_menu_open());
}

#[test]
fn the_menu_opens_in_every_language() {
    for language in crate::i18n::Language::ALL {
        let mut app = app_with_library("quick-languages");
        app.set_language(language.code());
        select(&mut app, &[1, 2]);
        let mut harness = window(app);
        harness.key_press(egui::Key::Q);
        harness.run_steps(3);
        assert!(harness.state().quick_menu_open(), "{language:?}");
    }
}

#[test]
#[ignore = "a picture for looking at the menu"]
fn menu_picture() {
    let mut app = app_with_library("quick-picture");
    select(&mut app, &[1]);
    let i = app.add_operation(OpKind::Profile).unwrap();
    assert!(app.save_style(i, "Cut out"));
    app.undo();
    select(&mut app, &[1]);
    let mut app = Some(app);
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .wgpu()
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            crate::style::apply(&cc.egui_ctx);
            app.take().unwrap()
        });
    harness.run_steps(2);
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.state_mut().quick_menu = Some(egui::pos2(600.0, 400.0));
    harness.run_steps(4);
    let out = std::env::var("CHIPLOAD_SHOTS_OUT").unwrap_or_else(|_| ".".into());
    harness
        .render()
        .unwrap()
        .save(PathBuf::from(out).join("quick-menu.png"))
        .unwrap();
}
