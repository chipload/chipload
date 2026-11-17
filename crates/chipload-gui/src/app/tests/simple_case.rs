// SPDX-License-Identifier: GPL-3.0-or-later
//! The simple case first (docs/specs/parts-and-setups.md §2a): one board shows no Setups
//! branch, its stock, work zero and parts are the project's rows; a part cut once where it
//! is drawn shows no instance rows, ghost or badge. A second board brings the branch,
//! removing it takes the branch away again.
use super::*;

fn depth_of(rows: &[(usize, String)], label: &str) -> Option<usize> {
    rows.iter().find(|(_, l)| l == label).map(|(d, _)| *d)
}

fn has_setups_branch(rows: &[(usize, String)]) -> bool {
    rows.iter()
        .any(|(_, l)| l == "Setups" || l.starts_with("Setup "))
}

#[test]
fn one_board_shows_no_setups_and_a_part_cut_once_no_instances() {
    // Imported, a pocket and a profile: the usual simple job.
    let mut app = lower_part();
    let rows = browser_rows(&mut app);
    assert!(!has_setups_branch(&rows), "{rows:?}");
    assert_eq!(depth_of(&rows, "Stock"), Some(1), "{rows:?}");
    assert_eq!(depth_of(&rows, "Work zero"), Some(1));
    assert_eq!(depth_of(&rows, "Part 1"), Some(1));
    // Its operations right under it.
    let part = rows.iter().position(|(_, l)| l == "Part 1").unwrap();
    assert_eq!(rows[part + 1].0, 2, "{rows:?}");
    assert!(!rows.iter().any(|(_, l)| l.starts_with("Instance ")));
    draw_canvas(&mut app);
    assert!(app.scene.instances.is_empty() && app.scene.elsewhere.is_empty());

    // Two parts on the one board, then a second board (found by the user, 2026-11-17: a new
    // setup made a part of its own): Parts and Setups (docs/specs/parts-and-setups.md §8),
    // the new board empty. The parts are definitions with their operations; each setup
    // lists its stock, zero and instances.
    app.add_part().unwrap();
    let simple = browser_rows(&mut app).len();
    let added = app.add_setup().unwrap();
    assert_eq!(app.edit.as_ref().unwrap().job.parts.len(), 2);
    let rows = browser_rows(&mut app);
    assert!(has_setups_branch(&rows), "{rows:?}");
    assert_eq!(depth_of(&rows, "Parts"), Some(1));
    assert_eq!(depth_of(&rows, "Setups"), Some(1));
    assert_eq!(depth_of(&rows, "Part 1"), Some(2));
    assert_eq!(depth_of(&rows, "Stock"), Some(3));
    assert_eq!(depth_of(&rows, "Part 1 #1"), Some(3));
    assert_eq!(depth_of(&rows, "Part 2 #1"), Some(3));
    assert!(
        !rows.iter().any(|(_, l)| l.starts_with("Part 3")),
        "{rows:?}"
    );
    // + Parts; + Setups, Setup 1, Part 1 #1, Part 2 #1; + Setup 2, its stock and zero.
    assert_eq!(rows.len(), simple + 1 + 4 + 3, "{rows:?}");
    // Part 2 moves to the new board, and back.
    app.instance_to_setup(2, 1, added);
    assert_eq!(app.edit.as_ref().unwrap().job.setups_of_part(2), [added]);
    app.instance_to_setup(2, 1, 1);
    assert_eq!(app.edit.as_ref().unwrap().job.setups_of_part(2), [1]);

    // Removed again: Setup 1 stays listed (the user, 2026-11-17: only gone when asked).
    app.remove_setup(added);
    assert_eq!(app.edit.as_ref().unwrap().job.setups.len(), 1);
    let rows = browser_rows(&mut app);
    assert!(has_setups_branch(&rows), "{rows:?}");
    assert_eq!(depth_of(&rows, "Setup 1"), Some(2));
    assert_eq!(depth_of(&rows, "Part 2 #1"), Some(3));
    // Simple view, asked for: the project's rows again, one undo step.
    app.show_setups(false);
    let rows = browser_rows(&mut app);
    assert!(!has_setups_branch(&rows), "{rows:?}");
    assert_eq!(rows.len(), simple);
    app.undo();
    assert!(has_setups_branch(&browser_rows(&mut app)));
}

#[test]
fn setup_1_can_be_shown_by_hand_and_stays_until_the_simple_view_is_asked_for() {
    // Parts drawn apart, to be cut from separate boards: Show setups first.
    let mut app = lower_part();
    app.add_part().unwrap();
    let simple = browser_rows(&mut app).len();
    app.show_setups(true);
    let job = app.edit.as_ref().unwrap().job.clone();
    assert_eq!(job.setups.len(), 1);
    assert!(job.setups_shown && job.lists_setups());
    let rows = browser_rows(&mut app);
    assert!(has_setups_branch(&rows), "{rows:?}");
    assert_eq!(depth_of(&rows, "Setup 1"), Some(2));
    assert_eq!(depth_of(&rows, "Part 1 #1"), Some(3));
    // Saved with the project, read back.
    let dir = temp_dir("setups-shown");
    let file = dir.join("p.chipload");
    chipload_job::save_job(&job, &file).unwrap();
    assert!(chipload_job::load_job(&file).unwrap().setups_shown);
    // The project's own menu is a setup's now, not the board's.
    app.show_setups(false);
    assert_eq!(browser_rows(&mut app).len(), simple);
    assert!(!app.edit.as_ref().unwrap().job.setups_shown);
}

#[test]
fn a_part_placed_again_shows_its_instances_in_the_simple_view() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    let rows = browser_rows(&mut app);
    assert!(!has_setups_branch(&rows));
    // The part itself and its new instance, one deeper than the part.
    assert_eq!(depth_of(&rows, "Instance 1"), Some(2), "{rows:?}");
    assert_eq!(depth_of(&rows, "Instance 2"), Some(2));
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
    use egui_kittest::kittest::Queryable;
    assert!(harness.query_by_label("Nest").is_some());
    assert!(harness.query_by_label_contains("Setup 1").is_none());
}
