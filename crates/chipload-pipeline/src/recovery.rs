// SPDX-License-Identifier: GPL-3.0-or-later
//! Recovery copies of unsaved projects (docs/specs/autosave-and-backup.md §2).
//!
//! Each editing session has an id and three files in the recovery folder: `<id>.chipload`
//! (an ordinary project file), `<id>.json` (a [`RecoveryNote`]: which project it belongs to)
//! and `<id>.lock`, which the session keeps locked while it runs. The OS releases the lock
//! when the process ends, crash or not, so a set whose lock can be taken is left over.

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chipload_job::library::{LibraryError, RecoveryNote, load_recovery_note, save_recovery_note};
use chipload_job::{Job, JobError, dump_job, load_job};

use crate::write_atomically;

/// The recovery folder's name inside the settings folder.
pub const RECOVERY_DIR: &str = "recovery";

pub fn recovery_dir(settings: &Path) -> PathBuf {
    settings.join(RECOVERY_DIR)
}

#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    #[error("Cannot write the recovery copy {path}: {reason}")]
    Io { path: String, reason: String },
    #[error(transparent)]
    Job(#[from] JobError),
    #[error(transparent)]
    Note(#[from] LibraryError),
}

fn io_error(path: &Path) -> impl Fn(std::io::Error) -> RecoveryError + '_ {
    move |e| RecoveryError::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

/// The three files of a session `id` in `dir`.
fn files(dir: &Path, id: &str) -> (PathBuf, PathBuf, PathBuf) {
    (
        dir.join(format!("{id}.chipload")),
        dir.join(format!("{id}.json")),
        dir.join(format!("{id}.lock")),
    )
}

/// Open (creating it) and lock `path`; None if another process holds it.
fn take_lock(path: &Path) -> std::io::Result<Option<File>> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

/// This window's recovery files, locked while it runs.
#[derive(Debug)]
pub struct Session {
    copy: PathBuf,
    note: PathBuf,
    lock_path: PathBuf,
    /// Held for the session's life; dropped (released) before the lock file is removed.
    lock: Option<File>,
}

impl Session {
    /// A new session in `dir`, with a fresh id and its lock taken.
    pub fn start(dir: &Path) -> Result<Session, RecoveryError> {
        std::fs::create_dir_all(dir).map_err(io_error(dir))?;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let id = format!("{}-{nanos}", std::process::id());
        let (copy, note, lock_path) = files(dir, &id);
        let lock = take_lock(&lock_path).map_err(io_error(&lock_path))?;
        if lock.is_none() {
            return Err(RecoveryError::Io {
                path: lock_path.display().to_string(),
                reason: "already locked".into(),
            });
        }
        Ok(Session {
            copy,
            note,
            lock_path,
            lock,
        })
    }

    /// Write `job` as the recovery copy, with a note naming `project` (None: never saved).
    pub fn write(&self, job: &Job, project: Option<&Path>) -> Result<(), RecoveryError> {
        let text = dump_job(job)?;
        write_atomically(&self.copy, &text).map_err(io_error(&self.copy))?;
        let project_path = project
            .map(|p| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()))
            .map(|p| p.display().to_string());
        let note = RecoveryNote {
            project_path,
            ..RecoveryNote::default()
        };
        save_recovery_note(&note, &self.note)?;
        Ok(())
    }

    /// There is a recovery copy.
    pub fn has_copy(&self) -> bool {
        self.copy.exists()
    }

    /// Remove the copy and its note (the lock stays: the session goes on).
    pub fn clear(&self) -> Result<(), RecoveryError> {
        remove(&self.copy)?;
        remove(&self.note)
    }
}

impl Drop for Session {
    /// A session that ends normally releases and removes its lock. Its copy, if any, is
    /// kept: the window was closed without clearing it, so it is offered next time.
    fn drop(&mut self) {
        drop(self.lock.take());
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

fn remove(path: &Path) -> Result<(), RecoveryError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(path)(e)),
    }
}

/// A recovery copy no running window owns. Its lock is held while this exists, so two
/// windows starting together don't both offer it.
#[derive(Debug)]
pub struct Leftover {
    pub copy: PathBuf,
    note_path: PathBuf,
    lock_path: PathBuf,
    lock: Option<File>,
    /// The project file it belongs to; None if never saved (or the note can't be read).
    pub project_path: Option<PathBuf>,
    /// When the copy was written.
    pub written: Option<SystemTime>,
    /// The project file was saved after the copy was written.
    pub project_changed: bool,
    /// Why it can't be opened (a cut-short copy, a newer Chipload, a broken note).
    pub problem: Option<String>,
}

impl Leftover {
    /// The project as recovered.
    pub fn open(&self) -> Result<Job, RecoveryError> {
        Ok(load_job(&self.copy)?)
    }

    /// Remove the copy, its note and its lock.
    pub fn delete(mut self) -> Result<(), RecoveryError> {
        remove(&self.copy)?;
        remove(&self.note_path)?;
        drop(self.lock.take());
        remove(&self.lock_path)
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Every recovery copy in `dir` that no running window owns, newest first. Lock or note
/// files without a copy are removed on the way (a session that ended normally).
pub fn leftovers(dir: &Path) -> Vec<Leftover> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let ids: BTreeSet<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x == "chipload" || x == "json" || x == "lock")
        })
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    let mut found = Vec::new();
    for id in ids {
        let (copy, note_path, lock_path) = files(dir, &id);
        let Ok(Some(lock)) = take_lock(&lock_path) else {
            continue; // a running window's, or a lock that can't be opened
        };
        if !copy.exists() {
            let _ = std::fs::remove_file(&note_path);
            drop(lock);
            let _ = std::fs::remove_file(&lock_path);
            continue;
        }
        let written = modified(&copy);
        let note = load_recovery_note(&note_path);
        let project_path = note
            .as_ref()
            .ok()
            .and_then(|n| n.project_path.as_ref())
            .map(PathBuf::from);
        let project_changed = match (&project_path, written) {
            (Some(p), Some(w)) => modified(p).is_some_and(|m| m > w),
            _ => false,
        };
        let problem = match (&note, load_job(&copy)) {
            (Err(e), _) => Some(e.to_string()),
            (_, Err(e)) => Some(e.to_string()),
            _ => None,
        };
        found.push(Leftover {
            copy,
            note_path,
            lock_path,
            lock: Some(lock),
            project_path,
            written,
            project_changed,
            problem,
        });
    }
    found.sort_by(|a, b| b.written.cmp(&a.written).then(a.copy.cmp(&b.copy)));
    found
}
