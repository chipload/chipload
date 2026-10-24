// SPDX-License-Identifier: GPL-3.0-or-later
//! The open project's file, watched for changes made by another program (an AI assistant
//! through `chipload mcp`, a script, an editor): reloaded quietly, or asked about when there
//! are unsaved changes (docs/specs/reload-changed-file.md).

use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use super::*;

/// How often the file is looked at (§2).
pub(super) const WATCH_INTERVAL: Duration = Duration::from_secs(1);

/// What identifies a version of the file: its modification time and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

fn stamp(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some(Stamp {
        modified: meta.modified().ok(),
        len: meta.len(),
    })
}

/// The watch on the project's file.
#[derive(Debug, Default)]
pub(super) struct DiskWatch {
    /// The version the app has (opened, saved, reloaded or declined).
    known: Option<Stamp>,
    /// A different version seen at the last look; acted on when it is still the same at
    /// the next (settled, §2).
    pending: Option<Stamp>,
    /// When to look next.
    next: Option<Instant>,
    /// The file's project, offered while the prompt asks (§4).
    offer: Option<EditJob>,
}

impl ChiploadApp {
    /// The file as it is now is the version the app has (after open and save).
    pub(super) fn remember_disk(&mut self) {
        self.disk = DiskWatch {
            known: self.job_path.as_deref().and_then(stamp),
            ..DiskWatch::default()
        };
    }

    /// Look at the file if it is time; returns how long until the next look.
    pub(super) fn watch_disk(&mut self, now: Instant) -> Option<Duration> {
        let path = self.job_path.clone()?;
        if let Some(next) = self.disk.next
            && now < next
        {
            return Some(next - now);
        }
        self.disk.next = Some(now + WATCH_INTERVAL);
        // A prompt on screen waits; the file is looked at again after it.
        if self.dialogs.prompt.is_some() {
            return Some(WATCH_INTERVAL);
        }
        let Some(current) = stamp(&path) else {
            return Some(WATCH_INTERVAL); // gone or unreadable: left alone
        };
        if Some(current) == self.disk.known {
            self.disk.pending = None;
        } else if self.disk.pending != Some(current) {
            self.disk.pending = Some(current); // still being written, perhaps
        } else {
            self.disk.pending = None;
            self.disk.known = Some(current);
            self.file_changed(&path);
        }
        Some(WATCH_INTERVAL)
    }

    /// Look at the file now, whenever the last look was (tests).
    #[cfg(test)]
    pub(super) fn look_at_disk_now(&mut self) {
        self.disk.next = None;
        self.watch_disk(Instant::now());
    }

    /// A settled new version of the file: reload, ask, or say why it can't be loaded.
    fn file_changed(&mut self, path: &Path) {
        let job = match load_job(path) {
            Ok(job) => job,
            Err(err) => {
                let mut report = Report::failed(
                    t!(self, "disk-cannot-load", file = file_name(path)),
                    err.message(),
                );
                report.push(Tone::Error, t!(self, "disk-unchanged"));
                self.error_report(report);
                return;
            }
        };
        let theirs = EditJob::from_job(&job);
        if self.saved.as_ref() == Some(&theirs) {
            return; // the same project, written again
        }
        if self.edit.as_ref() == Some(&theirs) {
            self.saved = Some(theirs); // what the user has, saved by someone else
            return;
        }
        if self.dirty() {
            self.disk.offer = Some(theirs);
            self.dialogs.prompt = Some(Prompt::ChangedOnDisk);
        } else {
            self.reload(theirs);
        }
    }

    /// The prompt's *Load it*: the file's version, the unsaved changes one Ctrl+Z away.
    pub fn load_from_disk(&mut self) {
        self.dialogs.prompt = None;
        if let Some(theirs) = self.disk.offer.take() {
            self.reload(theirs);
        }
    }

    /// The prompt's *Keep mine*: nothing changes; this version isn't asked about again.
    pub fn keep_mine(&mut self) {
        self.dialogs.prompt = None;
        self.disk.offer = None;
    }

    /// The file's project in place of the one shown, as one undo step (§3).
    fn reload(&mut self, theirs: EditJob) {
        self.commit();
        self.edit = Some(theirs.clone());
        self.saved = Some(theirs);
        self.commit();
        self.after_history_step();
        let name = self.job_path.as_deref().map(file_name).unwrap_or_default();
        self.set_report(Report::info([t!(
            self,
            "disk-reloaded-report",
            file = name.as_str()
        )]));
        let text = t!(self, "disk-reloaded", file = name);
        self.set_status(text, STATUS_SECONDS);
    }
}
