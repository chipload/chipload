// SPDX-License-Identifier: GPL-3.0-or-later
//! The open project's file changed by another program (docs/specs/reload-changed-file.md).
use std::time::Instant;

use super::*;

/// A copy of a corpus project in a folder of its own, opened.
fn opened(name: &str) -> (ChiploadApp, PathBuf) {
    let dir = temp_dir(name);
    let path = dir.join("part.chipload");
    std::fs::copy(corpus("jobs/v18/dogbones.chipload"), &path).unwrap();
    let mut app = ChiploadApp::default();
    assert!(app.open(&path));
    (app, path)
}

/// Another program sets the stock's X size and saves (each value a different length, so
/// the file's size changes too).
fn changed_elsewhere(path: &Path, x: f64) {
    let mut job = chipload_job::load_job(path).unwrap();
    job.stock.x = x;
    chipload_job::save_job(&job, path).unwrap();
}

/// The stock's X size in the app.
fn stock_x(app: &ChiploadApp) -> f64 {
    app.edit.as_ref().unwrap().settings.stock.x
}

/// The user sets the stock's X size in the app.
fn set_here(app: &mut ChiploadApp, x: f64) {
    app.edit.as_mut().unwrap().settings.stock.x = x;
    app.commit();
}

/// Looks at the file `times` times (a change settles on the second look).
fn look(app: &mut ChiploadApp, times: u32) {
    for _ in 0..times {
        app.look_at_disk_now();
    }
}

#[test]
fn a_file_changed_on_disk_is_reloaded_as_one_undo_step() {
    let (mut app, path) = opened("disk-reload");
    let before = stock_x(&app);
    look(&mut app, 1);
    changed_elsewhere(&path, 151.25);
    // Seen once: not yet (it may still be being written).
    look(&mut app, 1);
    assert_eq!(stock_x(&app), before);
    look(&mut app, 2);
    assert_eq!(stock_x(&app), 151.25);
    assert!(!app.dirty(), "what the file holds counts as saved");
    assert!(
        app.report
            .lines
            .iter()
            .any(|l| l.text.contains("changed on disk")),
        "{:?}",
        app.report
    );
    // Not reloaded again on the next looks.
    let undo_steps = app.undo.len();
    look(&mut app, 3);
    assert_eq!(app.undo.len(), undo_steps);
    app.undo();
    assert_eq!(stock_x(&app), before);
}

#[test]
fn the_apps_own_save_is_not_a_change() {
    let (mut app, path) = opened("disk-own-save");
    look(&mut app, 1);
    set_here(&mut app, 163.125);
    assert!(app.save_project(&path));
    let undo_steps = app.undo.len();
    look(&mut app, 3);
    assert_eq!(app.undo.len(), undo_steps);
    assert!(app.dialogs.prompt.is_none() && !app.dirty());
}

#[test]
fn unsaved_changes_ask_first() {
    let (mut app, path) = opened("disk-ask");
    look(&mut app, 1);
    set_here(&mut app, 150.0);
    changed_elsewhere(&path, 160.5);
    look(&mut app, 2);
    assert!(matches!(app.dialogs.prompt, Some(Prompt::ChangedOnDisk)));
    assert_eq!(stock_x(&app), 150.0, "nothing changed yet");
    // Keep mine: unchanged, still unsaved, and not asked again for this version.
    app.keep_mine();
    look(&mut app, 3);
    assert!(app.dialogs.prompt.is_none());
    assert_eq!(stock_x(&app), 150.0);
    assert!(app.dirty());
    // A later change asks again; Load it loads, and Ctrl+Z brings the unsaved edit back.
    changed_elsewhere(&path, 170.0625);
    look(&mut app, 2);
    assert!(matches!(app.dialogs.prompt, Some(Prompt::ChangedOnDisk)));
    app.load_from_disk();
    assert_eq!(stock_x(&app), 170.0625);
    assert!(!app.dirty());
    app.undo();
    assert_eq!(stock_x(&app), 150.0);
}

#[test]
fn a_broken_file_is_said_once_and_changes_nothing() {
    let (mut app, path) = opened("disk-broken");
    look(&mut app, 1);
    let before = app.edit.clone();
    std::fs::write(&path, "{ not a project").unwrap();
    look(&mut app, 2);
    assert_eq!(app.edit, before);
    let errors = |app: &ChiploadApp| {
        app.report
            .lines
            .iter()
            .filter(|l| l.text.contains("can't be loaded"))
            .count()
    };
    assert_eq!(errors(&app), 1, "{:?}", app.report);
    app.set_report(Report::info(["something else".to_string()]));
    look(&mut app, 3);
    assert_eq!(errors(&app), 0, "not repeated");
}

#[test]
fn a_new_project_watches_nothing() {
    let mut app = ChiploadApp::default();
    app.new_project();
    assert_eq!(app.watch_disk(Instant::now()), None);
}
