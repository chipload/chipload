// SPDX-License-Identifier: GPL-3.0-or-later
//! Exploding a picture or a text from the Browser or the canvas
//! (docs/specs/explode-pictures-and-texts.md §6).

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

fn window(project: &str) -> Harness<'static, ChiploadApp> {
    let project = corpus(project);
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

fn press(harness: &mut Harness<'_, ChiploadApp>, pos: egui::Pos2, button: egui::PointerButton) {
    harness.hover_at(pos);
    harness.run_steps(1);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

/// Click `label` (a menu item or button), the last one shown with it.
fn click_label(harness: &mut Harness<'_, ChiploadApp>, label: &str) {
    let at = harness
        .get_all_by_label(label)
        .last()
        .unwrap_or_else(|| panic!("{label}"))
        .rect()
        .center();
    press(harness, at, egui::PointerButton::Primary);
}

fn drawing<'a>(harness: &'a Harness<'_, ChiploadApp>) -> &'a chipload_job::Drawing {
    &harness.state().edit.as_ref().unwrap().job.drawing
}

#[test]
fn a_picture_explodes_from_its_browser_row_and_undo_brings_it_back() {
    let mut harness = window("jobs/v35/vcarve_step_down.chipload");
    let traced = drawing(&harness).shapes.len();
    let row = harness.get_by_label_contains("logo.png").rect().center();
    press(&mut harness, row, egui::PointerButton::Secondary);
    click_label(&mut harness, "Explode into shapes…");
    assert!(harness.state().dialogs.prompt.is_some());
    // A picture's dialog has no join field.
    assert!(harness.query_by_label("Join ends closer than").is_none());
    click_label(&mut harness, "Explode");
    let after = drawing(&harness);
    assert!(after.images.is_empty());
    assert_eq!(after.shapes.len(), traced);
    assert!(after.shapes.iter().all(|s| s.is_drawn()));
    harness.state_mut().undo();
    harness.run_steps(2);
    assert_eq!(drawing(&harness).images.len(), 1);
}

#[test]
fn a_selected_text_explodes_from_the_canvas_menu() {
    let mut harness = window("jobs/v35/sign_frame.chipload");
    let letter = harness
        .state()
        .contours
        .iter()
        .find(|c| c.layers().contains(&"text".to_string()))
        .unwrap()
        .parts[0]
        .segment
        .prim
        .start();
    let at = harness.state().view.to_screen(letter);
    press(&mut harness, at, egui::PointerButton::Primary);
    assert!(matches!(
        harness.state().selection,
        CanvasSelection::Whole {
            of: Source::Text(1),
            ..
        }
    ));
    press(&mut harness, at, egui::PointerButton::Secondary);
    click_label(&mut harness, "Explode into shapes…");
    click_label(&mut harness, "Explode");
    let after = drawing(&harness);
    assert!(after.texts.is_empty());
    assert!(after.shapes.iter().all(|s| s.is_drawn()));
}
