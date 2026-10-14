// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn a_result_for_a_project_that_changed_meanwhile_is_dropped() {
    let mut app = lower_part();
    assert!(app.generate());
    assert!(app.generating());
    // Changed but not yet recorded (a value being dragged): the run isn't cancelled, but
    // its result is for the project as it was.
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 777.0,
        _ => panic!("operation 1 is a pocket"),
    }
    assert!(!app.wait_for_generation());
    assert!(app.result.is_none() && app.scene.toolpath.is_none());
    // An edit cancels the run in progress.
    assert!(app.generate());
    edit_feed(&mut app, 650.0);
    assert!(!app.generating());
}

#[test]
fn the_live_preview_generates_once_the_project_is_quiet() {
    let mut app = lower_part();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    edit_feed(&mut app, 650.0);
    let changed = app.changed_at.unwrap();
    assert!(app.toolpath_stale && app.result.is_none());
    // Not yet: it waits for the rest of the quiet time.
    let wait = app.live_preview(changed + Duration::from_millis(200));
    assert_eq!(wait, Some(Duration::from_millis(300)));
    assert!(!app.generating());
    // Then it runs, quietly.
    assert_eq!(app.live_preview(changed + generation::QUIET_TIME), None);
    assert!(app.generating());
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert!(!app.toolpath_stale && app.result.is_some());
    assert!(!app.scene.toolpath.as_ref().unwrap().stale);
    // Switched off, an edit waits for Generate.
    app.auto_generate = false;
    edit_feed(&mut app, 700.0);
    assert_eq!(
        app.live_preview(Instant::now() + Duration::from_secs(5)),
        None
    );
    assert!(!app.generating() && app.toolpath_stale);
}

#[test]
fn the_live_preview_waits_while_the_project_has_problems() {
    // docs/specs/live-problems.md decision 4: the Problems strip says why.
    let mut app = lower_part();
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 0.0,
        _ => panic!("operation 1 is a pocket"),
    }
    app.commit();
    let changed = app.changed_at.unwrap();
    assert_eq!(app.live_preview(changed + generation::QUIET_TIME), None);
    assert!(!app.generating());
    assert!(
        app.changed_at.is_none(),
        "not tried again until the next edit"
    );
    assert!(app.status.is_none(), "no red status-bar message");
    // Generate (F5) says it loudly.
    assert!(!app.generate());
    assert!(app.status.is_some());
}

#[test]
fn a_panic_while_generating_becomes_an_error_and_keeps_the_project() {
    let mut app = lower_part();
    let project = app.edit.clone();
    app.runner = |_, _| panic!("a geometry library fell over");
    assert!(!app.generate_and_wait());
    let text = format!("{:?}", app.report);
    assert!(text.contains("failed unexpectedly"), "{text}");
    assert!(text.contains("a geometry library fell over"), "{text}");
    assert_eq!(app.edit, project);
    assert!(!app.generating());
}

#[test]
fn cancel_stops_the_run_and_export_waits_for_it() {
    let mut app = lower_part();
    assert!(app.generate());
    app.export_when_ready();
    assert!(app.export_when_done);
    app.cancel_generation();
    assert!(!app.generating() && !app.export_when_done);
    assert!(app.result.is_none());
    let status = app.status.as_ref().map(|(text, _)| text.clone());
    assert_eq!(status.as_deref(), Some("Generation cancelled"));
}
