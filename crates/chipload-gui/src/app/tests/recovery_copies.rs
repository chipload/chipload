// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn recovery_copies(dir: &Path) -> usize {
    std::fs::read_dir(dir.join("recovery"))
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "chipload"))
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn an_edit_is_copied_for_recovery_after_a_pause_and_saving_removes_it() {
    let (mut app, dir) = with_settings("recovery-edit");
    assert!(app.leftovers.is_empty());
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    edit_feed(&mut app, 650.0);
    let due = app.recovery_due.unwrap();
    assert_eq!(
        app.recovery_tick(due + Duration::from_secs(1)),
        Some(Duration::from_secs(1))
    );
    assert_eq!(recovery_copies(&dir), 0, "not before the pause");
    assert_eq!(app.recovery_tick(due + recovery::RECOVERY_DELAY), None);
    assert_eq!(recovery_copies(&dir), 1);
    // An edit that leaves the project incomplete keeps the last complete copy.
    edit_feed(&mut app, 0.0);
    app.write_recovery();
    assert_eq!(recovery_copies(&dir), 1);
    edit_feed(&mut app, 650.0);
    let saved = dir.join("saved.chipload");
    assert!(app.save_project(&saved));
    assert_eq!(recovery_copies(&dir), 0, "saved: nothing to recover");
    // Opening another project drops the copy of discarded changes.
    edit_feed(&mut app, 700.0);
    app.write_recovery();
    assert_eq!(recovery_copies(&dir), 1);
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    assert_eq!(recovery_copies(&dir), 0);
}

#[test]
fn work_left_by_a_crashed_window_is_offered_and_reopened_unsaved() {
    let (mut crashed, dir) = with_settings("recovery-crash");
    let project = dir.join("leads.chipload");
    std::fs::copy(corpus("jobs/v7/leads.chipload"), &project).unwrap();
    assert!(crashed.open(&project));
    edit_feed(&mut crashed, 650.0);
    crashed.write_recovery();
    // Not offered to another window while the first is still running.
    let mut other = ChiploadApp::default();
    other.set_settings_dir(Some(dir.clone()));
    assert!(other.leftovers.is_empty());
    drop(other);
    // The first window goes away without saving or clearing (its lock is released).
    crashed.recovery = None;
    let mut app = ChiploadApp::default();
    app.set_settings_dir(Some(dir.clone()));
    assert_eq!(app.leftovers.len(), 1);
    assert_eq!(
        app.leftovers[0].project_path.as_deref(),
        Some(project.as_path())
    );
    assert!(app.open_leftover(0));
    assert!(app.leftovers.is_empty());
    assert!(app.dirty(), "recovered work is unsaved");
    assert_eq!(app.job_path.as_deref(), Some(project.as_path()));
    match &app.edit.as_ref().unwrap().ops[0].op {
        Operation::Pocket(p) => assert_eq!(p.feed, 650.0),
        _ => panic!("operation 1 is a pocket"),
    }
    // The left-over is gone; this window's own copy holds the work now.
    assert_eq!(recovery_copies(&dir), 1);
    assert!(app.save_project(&project));
    assert_eq!(recovery_copies(&dir), 0);
    assert!(dir.join("leads.chipload.bak").exists());
}

#[test]
fn a_left_over_copy_can_be_deleted() {
    let (mut crashed, dir) = with_settings("recovery-delete");
    assert!(crashed.open(&corpus("jobs/v7/leads.chipload")));
    edit_feed(&mut crashed, 650.0);
    crashed.write_recovery();
    crashed.recovery = None;
    let mut app = ChiploadApp::default();
    app.set_settings_dir(Some(dir.clone()));
    assert_eq!(app.leftovers.len(), 1);
    app.delete_leftover(0);
    assert!(app.leftovers.is_empty());
    assert_eq!(recovery_copies(&dir), 0);
}
