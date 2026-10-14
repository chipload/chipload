// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn new_operations_go_into_the_active_part_whatever_is_open() {
    let mut app = two_parts();
    assert_eq!(
        app.edit.as_ref().unwrap().settings.active_part_id(),
        Some(2)
    );
    // An operation of part 1 is open; the new one still goes into part 2, at its end.
    app.focus = Focus::Op(0);
    app.select_row(app.group_selection("dxf:2"), false);
    let index = app.add_operation(OpKind::Profile).unwrap();
    assert_eq!(index, 4);
    assert_eq!(op_parts(&app), [1, 1, 1, 2, 2]);
    // Activating part 1: the next one goes after part 1's operations.
    app.activate_part(1);
    let index = app.add_operation(OpKind::Profile).unwrap();
    assert_eq!(index, 3);
    assert_eq!(op_parts(&app), [1, 1, 1, 1, 2, 2]);
    // It is kept with the project.
    assert_eq!(app.edit.as_ref().unwrap().settings.active_part, Some(1));
}

#[test]
fn operations_move_between_parts_and_the_list_stays_grouped() {
    let mut app = two_parts();
    // Dropped on part 2's row: to the end of part 2.
    app.move_operation_to_part(0, 2);
    assert_eq!(op_parts(&app), [1, 1, 2, 2]);
    assert_eq!(app.focus, Focus::Op(3));
    // Alt+↑ on part 2's first operation: it crosses into part 1.
    app.move_operation(2, 1);
    assert_eq!(op_parts(&app), [1, 1, 1, 2]);
    // Moving part 2 up takes its operation along.
    app.move_part(2, true);
    assert_eq!(op_parts(&app), [2, 1, 1, 1]);
    assert!(app.edit.as_ref().unwrap().to_job().is_ok());
}

#[test]
fn a_part_is_added_turned_off_and_deleted_with_its_operations_and_undo_brings_it_back() {
    let mut app = two_parts();
    let id = app.add_part().unwrap();
    assert_eq!(id, 3);
    assert_eq!(app.focus, Focus::Part(3));
    let names: Vec<String> = app
        .edit
        .as_ref()
        .unwrap()
        .settings
        .parts
        .iter()
        .map(|p| p.name.clone())
        .collect();
    assert_eq!(names, ["simple part", "part B", "Part 3"]);
    app.toggle_part(2);
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    assert!(!job.cuts(3));
    app.delete_part(2);
    assert_eq!(op_parts(&app), [1, 1, 1]);
    app.undo();
    assert_eq!(op_parts(&app), [1, 1, 1, 2]);
    // The last part stays.
    app.delete_part(3);
    app.delete_part(2);
    app.delete_part(1);
    assert_eq!(app.edit.as_ref().unwrap().settings.parts.len(), 1);
}

#[test]
fn a_parts_properties_edit_its_copies_and_the_browser_draws_the_parts() {
    let mut app = two_parts();
    app.focus = Focus::Part(1);
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    // A problem in part 1's copies marks part 1's form.
    app.edit.as_mut().unwrap().settings.parts[0].copies = Some(chipload_job::Copies {
        columns: 0,
        rows: 1,
        dx: 10.0,
        dy: 10.0,
        order: chipload_job::CopyOrder::ByOperation,
    });
    app.commit();
    let fields = app.form_problem_map(Focus::Part(1));
    assert!(fields.contains_key("parts.0.copies.columns"), "{fields:?}");
    assert!(app.form_problem_map(Focus::Part(2)).is_empty());
    draw_properties(&mut app);
    draw_sidebar(&mut app);
}
