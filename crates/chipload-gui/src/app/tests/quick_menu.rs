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

#[test]
fn select_inside_and_similar_replace_or_add_to_the_selection() {
    let mut app = app_with_library("select-by-kind");
    select(&mut app, &[1]);
    // The outline's holes, without it.
    app.select_inside(1, false);
    assert_eq!(app.selected_ids(), [2, 3, 4].into());
    assert_eq!(app.focus, Focus::Selection);
    // The circle is the only one of its diameter: replaces the selection.
    let circle = app
        .contours
        .iter()
        .find(|c| {
            c.parts
                .iter()
                .all(|p| matches!(p.segment.prim, chipload_core::primitives::Primitive::Arc(_)))
        })
        .unwrap()
        .id;
    app.select_similar(circle, false);
    assert_eq!(app.selected_ids(), [circle].into());
    // With Ctrl: added.
    app.select_similar(1, true);
    assert_eq!(app.selected_ids(), [1, circle].into());
    // No inside to an open contour or a hole with nothing in it.
    assert!(app.has_inside(1));
    app.select_inside(circle, false);
    assert_eq!(app.selection, CanvasSelection::Nothing);
}

/// Layer rules (docs/specs/fewer-clicks.md §4): a style with a pattern adds its operation on
/// import, as one undo step; *Apply layer rules* then adds nothing more.
#[test]
fn layer_rules_add_operations_on_import_in_one_undo_step() {
    let mut app = app_with_library("layer-rules");
    select(&mut app, &[1]);
    let i = app.add_operation(OpKind::Profile).unwrap();
    assert!(app.save_style(i, "Cut out"));
    let uid = app.user.library.styles.styles[0].uid.clone();
    assert!(app.set_style_layer(&uid, " cut_* "));
    assert_eq!(
        app.user.library.styles.styles[0].layer.as_deref(),
        Some("cut_*")
    );
    // A new project from the same drawing: the rule cuts its outline layer.
    assert!(app.import_dxf(&corpus("dxf/simple_part.dxf"), None));
    let names: Vec<(String, Option<String>)> = ops(&app)
        .iter()
        .map(|op| (op.name().to_string(), op.select().layer.clone()))
        .collect();
    assert_eq!(
        names,
        [("Cut out".to_string(), Some("CUT_OUTSIDE".to_string()))]
    );
    // Still on the stock, as a new project opens; the report says what the rule did.
    assert_ne!(app.focus, Focus::Op(0));
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("style \"Cut out\" on layer CUT_OUTSIDE")),
        "{:?}",
        app.report.iter().collect::<Vec<_>>()
    );
    // Applied again: the layer is cut already.
    assert_eq!(app.apply_layer_rules(), 0);
    assert_eq!(ops(&app).len(), 1);
    // One Undo takes the rule's operations away and keeps the project.
    app.undo();
    assert!(ops(&app).is_empty());
    assert!(app.edit.is_some());
    // No rule: nothing is added on import.
    assert!(app.set_style_layer(&uid, ""));
    assert!(app.import_dxf(&corpus("dxf/simple_part.dxf"), None));
    assert!(ops(&app).is_empty());
}
