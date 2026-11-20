// SPDX-License-Identifier: GPL-3.0-or-later
//! Placing in the window (docs/specs/parts-and-setups.md §8): with two setups the Browser
//! shows the parts as definitions and each setup's instances; a part dropped on a setup
//! moves there (with Ctrl it is placed there too), an instance moved to another keeps its number; the part's form lists its
//! instances by setup.
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn two_setups() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    app
}

fn on(app: &ChiploadApp, part: u64) -> Vec<(u64, u32)> {
    app.edit
        .as_ref()
        .unwrap()
        .job
        .instances_of(part)
        .into_iter()
        .map(|(s, i)| (s, i.id))
        .collect()
}

#[test]
fn the_browser_shows_parts_and_each_setups_instances() {
    let mut app = two_setups();
    let rows = browser_rows(&mut app);
    let at = |label: &str| rows.iter().position(|(_, l)| l == label);
    let depth = |label: &str| rows.iter().find(|(_, l)| l == label).map(|(d, _)| *d);
    assert_eq!(depth("Parts"), Some(1));
    assert_eq!(depth("simple part"), Some(2));
    assert_eq!(depth("part B"), Some(2));
    // Under setup 2: part B's instances, named with their part.
    assert!(
        at("6 mm bottom").unwrap() < at("part B #1").unwrap(),
        "{rows:?}"
    );
    assert_eq!(depth("part B #1"), Some(3));
    assert!(at("12 mm sides").unwrap() < at("simple part #1").unwrap());
}

#[test]
fn a_part_is_placed_on_a_setup_and_an_instance_moved_as_one_undo_step_each() {
    let mut app = two_setups();
    assert_eq!(on(&app, 1), [(1, 1)]);
    // *Place on setup ▸ 6 mm bottom*: a new instance there, where drawn, selected.
    app.place_part_on(1, 2);
    assert_eq!(on(&app, 1), [(1, 1), (2, 2)]);
    assert_eq!(app.focus, Focus::Instance(1, 2));
    let fresh = app.instance_value(1, 2).unwrap();
    assert!(fresh.is_drawn());
    app.undo();
    assert_eq!(on(&app, 1), [(1, 1)]);
    // *Move to setup ▸*: the same number and place on the other board.
    let before = app.instance_value(2, 1).unwrap();
    app.instance_to_setup(2, 1, 1);
    let moved: Vec<(u64, u32)> = on(&app, 2);
    assert!(moved.contains(&(1, 1)), "{moved:?}");
    assert_eq!(app.instance_value(2, 1).unwrap(), before);
    // The part's form lists its instances by setup.
    app.focus = Focus::Part(2);
    draw_properties(&mut app);
    app.undo();
    assert!(on(&app, 2).iter().all(|(s, _)| *s == 2));
}

/// Dropped, a part moves; with Ctrl (⌘) it is placed there too (the user, 2026-11-17).
#[test]
fn a_parts_row_dropped_on_a_setup_moves_it_there() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|_| two_setups());
    h.run_steps(4);
    let from = h.get_by_label("simple part").rect().center();
    // The Browser's row, left of the canvas's tab of the same name.
    let to = h
        .get_all_by_label("6 mm bottom")
        .map(|n| n.rect().center())
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(2);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run_steps(2);
    }
    h.drop_at(to);
    h.run_steps(4);
    // Every instance of it on the other board, its number kept.
    assert_eq!(on(h.state(), 1), [(2, 1)]);
    use crate::app::browser_setups::{SetupAction, part_dropped};
    assert_eq!(part_dropped(1, 2, false), SetupAction::MovePart(1, 2));
    assert_eq!(part_dropped(1, 2, true), SetupAction::PlacePart(1, 2));
}

/// A part moved to a thinner board: its cuts through the stock go through that board, in
/// the same undo step.
#[test]
fn a_moved_parts_cuts_through_follow_its_new_board() {
    let mut app = two_setups();
    if let Some(t) = app.edit.as_mut().unwrap().job.operations[1].through_mut() {
        *t = Some(0.0);
    }
    app.commit();
    app.move_part_to_setup(1, 2);
    let depth = |app: &ChiploadApp| app.edit.as_ref().unwrap().job.operations[1].depths(&[]).1;
    assert!((depth(&app) - 6.0).abs() < 1e-9, "{}", depth(&app));
    app.undo();
    assert!((depth(&app) - 12.0).abs() < 1e-9, "{}", depth(&app));
}
