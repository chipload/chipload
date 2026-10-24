// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn problem_paths(app: &ChiploadApp) -> Vec<String> {
    app.problems.iter().map(|p| p.path.clone()).collect()
}

#[test]
fn a_bad_value_is_flagged_at_its_field_once_the_edit_is_finished() {
    let mut app = lower_part();
    assert!(app.problems.is_empty(), "{:?}", app.problems);
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 0.0,
        _ => panic!("operation 1 is a pocket"),
    }
    assert!(
        app.problems.is_empty(),
        "not while the value is being edited"
    );
    app.commit();
    assert_eq!(problem_paths(&app), ["operations.0.feed"]);
    let fields = app.form_problem_map(Focus::Op(0));
    assert_eq!(
        fields.get("feed").map(String::as_str),
        Some("Must be greater than 0, got 0 mm/min")
    );
    assert!(app.form_problem_map(Focus::Op(1)).is_empty());
    assert!(app.form_problem_map(Focus::Job).is_empty());
    assert_eq!(
        problems::plain(&app, &app.problems[0]),
        "Feed: must be greater than 0, got 0 mm/min"
    );
    // The forms, cards and strip draw with it.
    app.focus = Focus::Op(0);
    draw_sidebar(&mut app);
    draw_properties(&mut app);
    // Fixed: gone.
    edit_feed(&mut app, 900.0);
    assert!(app.problems.is_empty(), "{:?}", app.problems);
}

#[test]
fn a_job_setting_problem_marks_the_job_and_undo_brings_it_back() {
    let mut app = lower_part();
    app.edit.as_mut().unwrap().settings.machine.safe_z = -2.0;
    app.commit();
    assert_eq!(problem_paths(&app), ["machine.safe_z"]);
    assert_eq!(app.job_problems().count(), 1);
    assert_eq!(
        problems::plain(&app, &app.problems[0]),
        "Safe Z: must be above the stock top (Z 0 mm)"
    );
    let fields = app.form_problem_map(Focus::Job);
    assert_eq!(
        fields.get("machine.safe_z").map(String::as_str),
        Some("Must be above the stock top (Z 0 mm)")
    );
    app.focus = Focus::Job;
    draw_sidebar(&mut app);
    draw_properties(&mut app);
    app.undo();
    assert!(app.problems.is_empty());
    app.redo();
    assert_eq!(problem_paths(&app), ["machine.safe_z"]);
}

#[test]
fn an_operation_without_shapes_is_a_problem_of_the_operation() {
    let mut app = lower_part();
    app.edit.as_mut().unwrap().ops[1]
        .op
        .select_mut()
        .unwrap()
        .shapes = Some(Vec::new());
    app.commit();
    assert_eq!(problem_paths(&app), ["operations.1.select"]);
    assert_eq!(
        problems::plain(&app, &app.problems[0]),
        "Geometry: no geometry: select contours, then Use selection"
    );
}
