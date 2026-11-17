// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn new_operations_go_into_the_active_part_whatever_is_open() {
    let mut app = two_parts();
    assert_eq!(app.edit.as_ref().unwrap().job.active_part_id(), Some(2));
    // An operation of part 1 is open; the new one still goes into part 2, at its end.
    app.focus = Focus::Op(0);
    app.select_row(app.group_selection("dxf:2"), false);
    let index = app.add_operation(OpKind::Pocket).unwrap();
    assert_eq!(index, 4);
    assert_eq!(op_parts(&app), [1, 1, 1, 2, 2]);
    // Activating part 1: the next one goes after part 1's operations.
    app.activate_part(1);
    let index = app.add_operation(OpKind::Pocket).unwrap();
    assert_eq!(index, 3);
    assert_eq!(op_parts(&app), [1, 1, 1, 1, 2, 2]);
    // It is kept with the project.
    assert_eq!(app.edit.as_ref().unwrap().job.active_part, Some(1));
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
fn a_board_surface_stays_first_in_its_setup() {
    // The user's finding (2026-11-17): a board surface could be dragged into a part.
    let mut app = two_parts();
    app.selection = Default::default();
    let board = app.add_operation(OpKind::Surface).unwrap();
    assert_eq!(board, 0);
    let before = op_parts(&app);
    let names = |app: &ChiploadApp| -> Vec<String> {
        let ops = &app.edit.as_ref().unwrap().job.operations;
        ops.iter().map(|o| o.name().to_string()).collect()
    };
    let order = names(&app);
    // Dragged down, moved by Alt+↓, dropped on a part's row: it stays.
    app.move_operation(0, 2);
    app.move_operation(0, 1);
    app.move_operation_to_part(0, 2);
    // Nothing moves onto its place either.
    app.move_operation(1, 0);
    assert_eq!(names(&app), order);
    assert_eq!(op_parts(&app), before);
    assert!(app.edit.as_ref().unwrap().job.operations[0].is_board_surface());
    // The other operations still move.
    app.move_operation(2, 1);
    assert_ne!(names(&app), order);
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
        .job
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
    assert_eq!(app.edit.as_ref().unwrap().job.parts.len(), 1);
}

#[test]
fn a_parts_properties_edit_its_copies_and_the_browser_draws_the_parts() {
    let mut app = two_parts();
    app.focus = Focus::Part(1);
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    // A problem in part 1's instances marks part 1's form.
    // Too many instances of part 1: its form is marked.
    let many: Vec<chipload_job::Instance> = (1..=chipload_job::MAX_COPIES + 1)
        .map(|n| chipload_job::Instance::moved(1, n, [f64::from(n) * 100.0, 0.0]))
        .collect();
    app.edit.as_mut().unwrap().job.set_instances_here(1, many);
    app.commit();
    let fields = app.form_problem_map(Focus::Part(1));
    assert!(fields.contains_key("parts.0.instances"), "{fields:?}");
    assert!(app.form_problem_map(Focus::Part(2)).is_empty());
    draw_properties(&mut app);
    draw_sidebar(&mut app);
}

/// A mirrored part (docs/specs/mirror-part.md §5): set on its form, marked in the Browser,
/// cut where its mirrored outlines are drawn, the shapes as drawn left to edit.
#[test]
fn a_mirrored_part_is_drawn_where_it_is_cut() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v35/op_add.chipload")));
    app.focus = Focus::Part(1);
    draw_properties(&mut app);
    app.edit.as_mut().unwrap().job.parts[0].mirror = Some(chipload_job::Mirror::LeftRight);
    app.commit();
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    draw_canvas(&mut app);
    let outlines = app.scene.mirrored.clone();
    assert!(!outlines.is_empty());
    // Mirrored about the part's centre line: the outlines' box is the drawing's.
    let job = app.edited_job().unwrap();
    let ((x0, _), (x1, _)) = job.part_box(1).unwrap();
    let xs = outlines.iter().flatten().map(|p| p.0);
    let (lo, hi) = xs.fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
    assert!(
        (lo - x0).abs() < 0.05 && (hi - x1).abs() < 0.05,
        "{lo} {hi}"
    );
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // As drawn again: nothing mirrored to draw.
    app.edit.as_mut().unwrap().job.parts[0].mirror = None;
    app.commit();
    draw_canvas(&mut app);
    assert!(app.scene.mirrored.is_empty());
}
