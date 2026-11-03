// SPDX-License-Identifier: GPL-3.0-or-later
//! The stock, an item of its own under its setup beside the work zero (the user,
//! 2026-11-03).
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness() -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
            app
        });
    harness.run_steps(6);
    harness
}

fn click_at(harness: &mut Harness<'_, ChiploadApp>, pos: egui::Pos2) {
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run_steps(3);
}

/// The Browser's row says the thickness and opens the stock.
#[test]
fn the_browser_row_shows_the_thickness_and_opens_the_stock() {
    let mut h = harness();
    let thickness = h.state().edit.as_ref().unwrap().job.stock().thickness;
    let row = h.get_by_label("Stock");
    row.click();
    h.run_steps(3);
    assert_eq!(h.state().focus, Focus::Stock);
    let summary = format!("{} mm", super::super::forms::len_in(Units::Mm, thickness));
    assert!(h.query_by_label_contains(&summary).is_some(), "{summary}");
}

/// A click on the stock's edge, where no shape is, opens it; inside the board it doesn't.
#[test]
fn a_click_on_the_board_edge_opens_the_stock() {
    let mut h = harness();
    h.state_mut().focus = Focus::Job;
    h.run_steps(2);
    let ((x0, y0), (x1, _)) = h.state().edit.as_ref().unwrap().job.stock_bounds();
    let edge = h.state().view.to_screen((0.5 * (x0 + x1), y0));
    let inside = h.state().view.to_screen((0.5 * (x0 + x1), y0 + 1.0)) + egui::vec2(0.0, -20.0);
    click_at(&mut h, inside);
    assert_ne!(h.state().focus, Focus::Stock, "inside the board");
    click_at(&mut h, edge);
    assert_eq!(h.state().focus, Focus::Stock);
}
