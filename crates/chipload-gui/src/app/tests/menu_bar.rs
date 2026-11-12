// SPDX-License-Identifier: GPL-3.0-or-later
//! The menu bar in the top bar (docs/specs/menu-bar.md).
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

fn harness() -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_eframe(|_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
            app
        });
    harness.run_steps(6);
    harness
}

/// The topmost node with this label: the menu bar's title, above the window's own.
fn title<'a>(h: &'a Harness<'_, ChiploadApp>, label: &'a str) -> egui_kittest::Node<'a> {
    h.get_all_by_label(label)
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .unwrap()
}

#[test]
fn the_menus_are_in_the_top_bar_in_order() {
    let h = harness();
    let titles = [
        "File",
        "Edit",
        "View",
        "Draw",
        "Modify",
        "Operation",
        "Wizards",
        "Library",
        "Help",
    ];
    let rects: Vec<egui::Rect> = titles.iter().map(|t| title(&h, t).rect()).collect();
    for pair in rects.windows(2) {
        assert!(pair[0].right() <= pair[1].left(), "{rects:?}");
        assert!((pair[0].center().y - pair[1].center().y).abs() < 1.0);
    }
    assert!(
        rects[0].bottom() <= crate::app::header::TOP_BAR_HEIGHT + 1.0,
        "{rects:?}"
    );
}

#[test]
fn edit_preferences_opens_this_computers_preferences() {
    let mut h = harness();
    title(&h, "Edit").click();
    h.run_steps(2);
    h.get_by_label("Preferences…").click();
    h.run_steps(3);
    assert_eq!(h.state().focus, Focus::Preferences);
}

#[test]
fn a_menu_opens_and_its_item_works() {
    let mut h = harness();
    title(&h, "View").click();
    h.run_steps(2);
    h.get_by_label("Reverse left-right orbit in 3D").click();
    h.run_steps(2);
    assert!(h.state().prefs.reverse_orbit);
}
