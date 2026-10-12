// SPDX-License-Identifier: GPL-3.0-or-later
//! Recovery copies of unsaved projects (docs/specs/autosave-and-backup.md §2).

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chipload_job::{Job, load_job};
use chipload_pipeline::recovery::{Session, leftovers};

fn corpus_job() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v14/round_corners.chipload"),
    )
    .unwrap()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-recovery-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn file_count(dir: &Path) -> usize {
    std::fs::read_dir(dir).unwrap().count()
}

#[test]
fn a_running_sessions_copy_is_not_offered_and_a_crashed_ones_is() {
    let dir = temp_dir("crash");
    let project = dir.join("sign.chipload");
    let job = corpus_job();
    let session = Session::start(&dir).unwrap();
    session.write(&job, Some(&project)).unwrap();
    assert!(session.has_copy());
    assert!(leftovers(&dir).is_empty(), "the session's lock is held");
    // The window goes away without clearing its copy (as after a crash: the lock is gone).
    drop(session);
    let found = leftovers(&dir);
    assert_eq!(found.len(), 1);
    let left = &found[0];
    assert_eq!(left.project_path.as_deref(), Some(project.as_path()));
    assert!(left.problem.is_none(), "{:?}", left.problem);
    assert!(!left.project_changed);
    assert!(left.written.is_some());
    let mut opened = left.open().unwrap();
    let mut expected = job.clone();
    opened.written_by = None;
    expected.written_by = None;
    assert_eq!(opened, expected);
    // While one window offers it, another doesn't.
    assert!(leftovers(&dir).is_empty());
    // Later: kept, offered again.
    drop(found);
    let found = leftovers(&dir);
    assert_eq!(found.len(), 1);
    // Delete removes all three files.
    found.into_iter().next().unwrap().delete().unwrap();
    assert_eq!(file_count(&dir), 0);
}

#[test]
fn a_session_that_ends_cleanly_leaves_nothing() {
    let dir = temp_dir("clean");
    let session = Session::start(&dir).unwrap();
    session.write(&corpus_job(), None).unwrap();
    session.clear().unwrap();
    assert!(!session.has_copy());
    drop(session);
    assert_eq!(file_count(&dir), 0);
    assert!(leftovers(&dir).is_empty());
}

#[test]
fn a_never_saved_project_and_a_project_saved_since() {
    let dir = temp_dir("details");
    let project = temp_dir("details-projects").join("box.chipload");
    std::fs::write(&project, "{}").unwrap();
    let first = Session::start(&dir).unwrap();
    first.write(&corpus_job(), Some(&project)).unwrap();
    let second = Session::start(&dir).unwrap();
    second.write(&corpus_job(), None).unwrap();
    drop((first, second));
    // The project file was saved after its copy.
    let later = SystemTime::now() + Duration::from_secs(60);
    std::fs::File::options()
        .write(true)
        .open(&project)
        .unwrap()
        .set_modified(later)
        .unwrap();
    let found = leftovers(&dir);
    assert_eq!(found.len(), 2);
    let saved = found.iter().find(|l| l.project_path.is_some()).unwrap();
    assert!(saved.project_changed);
    assert!(found.iter().any(|l| l.project_path.is_none()));
}

#[test]
fn a_broken_copy_is_listed_with_why_and_never_deleted_by_itself() {
    let dir = temp_dir("broken");
    let session = Session::start(&dir).unwrap();
    session.write(&corpus_job(), None).unwrap();
    drop(session);
    let copy = leftovers(&dir).into_iter().next().unwrap().copy.clone();
    std::fs::write(&copy, "{\"schema_version\": 14, \"stock\"").unwrap();
    let found = leftovers(&dir);
    assert_eq!(found.len(), 1);
    assert!(found[0].problem.is_some());
    assert!(found[0].open().is_err());
    drop(found);
    assert!(copy.exists());
    // Stray lock and note files without a copy are tidied away.
    std::fs::write(dir.join("123-4.lock"), "").unwrap();
    std::fs::write(dir.join("123-4.json"), "{}").unwrap();
    assert_eq!(leftovers(&dir).len(), 1);
    assert!(!dir.join("123-4.lock").exists() && !dir.join("123-4.json").exists());
}
