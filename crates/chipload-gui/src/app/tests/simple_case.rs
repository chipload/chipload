// SPDX-License-Identifier: GPL-3.0-or-later
//! The simple case (docs/specs/parts-and-setups.md §2a, changed 2026-11-18): the setups are
//! always listed, one board as *Setups › Setup 1* with its stock, work zero and instances;
//! the project's row and the Browser's empty space have the project's menu.
use super::*;
use egui_kittest::kittest::Queryable;

fn depth_of(rows: &[(usize, String)], label: &str) -> Option<usize> {
    rows.iter().find(|(_, l)| l == label).map(|(d, _)| *d)
}

#[test]
fn one_board_is_listed_as_setup_1_with_its_stock_zero_and_parts() {
    // Imported, a pocket and a profile: the usual simple job.
    let mut app = lower_part();
    let rows = browser_rows(&mut app);
    assert_eq!(depth_of(&rows, "Parts"), Some(1), "{rows:?}");
    assert_eq!(depth_of(&rows, "Part 1"), Some(2));
    // Its operations right under it.
    let part = rows.iter().position(|(_, l)| l == "Part 1").unwrap();
    assert_eq!(rows[part + 1].0, 3, "{rows:?}");
    assert_eq!(depth_of(&rows, "Setups"), Some(1));
    assert_eq!(depth_of(&rows, "Setup 1"), Some(2));
    assert_eq!(depth_of(&rows, "Stock"), Some(3));
    assert_eq!(depth_of(&rows, "Work zero"), Some(3));
    assert_eq!(depth_of(&rows, "Part 1 #1"), Some(3));

    // A second board, then removed again: the same shape, one setup fewer.
    let one = rows.len();
    let added = app.add_setup().unwrap();
    // + Setup 2, its stock and zero.
    assert_eq!(browser_rows(&mut app).len(), one + 3);
    app.remove_setup(added);
    assert_eq!(browser_rows(&mut app), rows);
}

/// *Place…* selects the part's instance under its setup.
#[test]
fn place_selects_the_instance_under_its_setup() {
    let mut app = drawn_part();
    app.place_part(1);
    assert_eq!(app.focus, Focus::Instance(1, 1));
    let rows = browser_rows(&mut app);
    assert_eq!(depth_of(&rows, "Part 1 #1"), Some(3), "{rows:?}");
}

/// The project's menu (the user, 2026-11-18: "the mechanism for creating a new part is
/// kind of hidden"): on the empty space below the Browser's rows, and on the project's row.
#[test]
fn right_clicking_the_browsers_empty_space_offers_a_new_part() {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(|_| drawn_part());
    h.run_steps(4);
    // Below the last row: the bottom of the Browser, on the left.
    let below = egui::pos2(60.0, 940.0);
    h.input_mut().events.push(egui::Event::PointerButton {
        pos: below,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.input_mut().events.push(egui::Event::PointerButton {
        pos: below,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(4);
    assert!(h.query_by_label("Add a setup").is_some());
    h.get_by_label("New part").click();
    h.run_steps(4);
    assert_eq!(h.state().edit.as_ref().unwrap().job.parts.len(), 2);
}

#[test]
fn the_nest_window_names_no_setup_with_one_board() {
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = drawn_part();
            app.start_nest(None);
            app.wait_for_nest();
            app
        });
    harness.run_steps(4);
    assert!(harness.query_by_label("Nest").is_some());
    assert!(harness.query_by_label("Nest: Setup 1").is_none());
}
