// SPDX-License-Identifier: GPL-3.0-or-later
//! The part view (docs/specs/parts-and-setups.md §8a): a part, or one of its operations,
//! selected shows that part alone, once, where it is drawn; the setup, the project, the
//! stock or the work zero selected shows the active setup with every instance on it.
use super::*;
use crate::app::generation::Scope;

/// Two setups, part 2 on the second.
fn two_setups() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    app
}

fn select(app: &mut ChiploadApp, focus: Focus) {
    app.focus = focus;
    app.follow_focus();
}

#[test]
fn a_part_selected_is_shown_alone_and_the_setup_brings_the_board_back() {
    let mut app = two_setups();
    app.activate_setup(1);
    select(&mut app, Focus::Setup(1));
    assert!(!app.scene.sheets.is_empty(), "the other setup beside it");
    let job = app.edited_job().unwrap();
    // Part B, cut twice from the second setup.
    let part = 2;
    select(&mut app, Focus::Part(part));
    assert_eq!(app.display.part, Some(part));
    // Its board, no sheets, the other parts' contours ghosts.
    let view = chipload_pipeline::part_view::part_view(&job, part).unwrap();
    assert_eq!(app.scene.stock, Some(view.stock_bounds()));
    assert!(app.scene.sheets.is_empty());
    assert!(!app.scene.ghost_contours.is_empty());
    // Generated: the part view, only its operations.
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert_eq!(app.toolpath.scope, Scope::View(part));
    let result = app.toolpath.result.as_ref().unwrap();
    assert!(
        result
            .op_spans
            .iter()
            .all(|s| job.operations[s.operation].part() == part)
    );
    // A setup selected: its board and the sheets come back.
    select(&mut app, Focus::Setup(1));
    assert_eq!(app.display.part, None);
    assert!(!app.scene.sheets.is_empty());
    assert!(app.wait_for_generation());
    assert_eq!(app.toolpath.scope, Scope::Setup);
}

#[test]
fn the_simple_case_keeps_its_board_when_a_part_is_selected() {
    let mut app = two_parts();
    let before = app.scene.stock;
    select(&mut app, Focus::Part(2));
    assert_eq!(app.display.part, Some(2));
    assert_eq!(
        app.scene.stock, before,
        "drawn in place: the board doesn't jump"
    );
}

#[test]
fn a_part_that_is_removed_is_no_longer_shown() {
    let mut app = two_parts();
    let added = app.add_part().unwrap();
    app.follow_focus();
    assert_eq!(app.display.part, Some(added));
    app.delete_part(added);
    assert_eq!(app.preview_scope(), Scope::Setup);
}
