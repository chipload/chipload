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
    let simple = rows.len();

    // A second board: the branch, both boards with their rows. Board 1's rows cost the
    // branch and its own setup row: two more than the simple view, as 0.37 drew one board.
    let added = app.add_setup().unwrap();
    let rows = browser_rows(&mut app);
    assert!(has_setups_branch(&rows), "{rows:?}");
    assert_eq!(depth_of(&rows, "Setups"), Some(1));
    assert_eq!(depth_of(&rows, "Stock"), Some(3));
    assert_eq!(depth_of(&rows, "Part 1"), Some(3));
    // + Setups, Setup 1; + Setup 2 with its stock, zero and part.
    assert_eq!(rows.len(), simple + 2 + 4, "{rows:?}");

    // Removed again: the simple view is back.
    app.remove_setup(added);
    assert_eq!(app.edit.as_ref().unwrap().job.setups.len(), 1);
    let rows = browser_rows(&mut app);
    assert!(!has_setups_branch(&rows), "{rows:?}");
    assert_eq!(rows.len(), simple);
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
