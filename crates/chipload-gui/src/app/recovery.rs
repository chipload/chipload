// SPDX-License-Identifier: GPL-3.0-or-later
//! Recovery copies while a project has unsaved changes, and the window that offers left-over
//! ones at start (docs/specs/autosave-and-backup.md §2). The files are
//! `chipload_pipeline::recovery`'s.

use std::time::SystemTime;

use chipload_pipeline::recovery::{Leftover, Session, leftovers, recovery_dir};

use super::*;

/// How long after a finished edit the recovery copy is written (decision 1).
pub(super) const RECOVERY_DELAY: Duration = Duration::from_secs(2);

/// "3 minutes ago", for when a copy was written.
fn ago(written: Option<SystemTime>) -> String {
    let Some(seconds) = written
        .and_then(|w| SystemTime::now().duration_since(w).ok())
        .map(|d| d.as_secs())
    else {
        return "at an unknown time".into();
    };
    let plural = |n: u64, unit: &str| {
        let s = if n == 1 { "" } else { "s" };
        format!("{n} {unit}{s} ago")
    };
    match seconds {
        0..60 => "less than a minute ago".into(),
        60..3600 => plural(seconds / 60, "minute"),
        3600..86_400 => plural(seconds / 3600, "hour"),
        _ => plural(seconds / 86_400, "day"),
    }
}

/// What a left-over copy is called in the window.
fn leftover_title(left: &Leftover) -> String {
    match &left.project_path {
        Some(path) => file_name(path),
        None => "Untitled (never saved)".into(),
    }
}

/// What was clicked for a left-over copy.
enum LeftoverChoice {
    Open(usize),
    Delete(usize),
    Later(usize),
}

impl ChiploadApp {
    /// Find left-over copies, then start this window's session (none without a settings
    /// folder, as in tests that don't ask for one).
    pub(super) fn start_recovery(&mut self) {
        self.recovery = None;
        self.leftovers.clear();
        let Some(dir) = self.settings_dir.as_deref().map(recovery_dir) else {
            return;
        };
        self.leftovers = leftovers(&dir);
        match Session::start(&dir) {
            Ok(session) => self.recovery = Some(session),
            Err(err) => self.set_status(
                format!("Recovery copies are off: {err}"),
                ERROR_STATUS_SECONDS,
            ),
        }
    }

    /// Write the recovery copy once the project has been quiet for the delay. Returns how
    /// long to wait before looking again, if a write is due later.
    pub(super) fn recovery_tick(&mut self, now: Instant) -> Option<Duration> {
        let since = now.saturating_duration_since(self.recovery_due?);
        if since < RECOVERY_DELAY {
            return Some(RECOVERY_DELAY - since);
        }
        self.write_recovery();
        None
    }

    /// The recovery copy as the project is now: written while there are unsaved changes,
    /// removed when there are none. An incomplete project keeps the last copy (decision 4).
    pub(super) fn write_recovery(&mut self) {
        self.recovery_due = None;
        let Some(session) = &self.recovery else {
            return;
        };
        let result = if self.dirty() {
            match self.edit.as_ref().map(EditJob::to_job) {
                Some(Ok(job)) => session.write(&job, self.job_path.as_deref()),
                _ => Ok(()),
            }
        } else {
            session.clear()
        };
        if let Err(err) = result {
            self.set_status(format!("{err}"), ERROR_STATUS_SECONDS);
        }
    }

    /// The changes were saved or deliberately dropped: no copy to keep.
    pub(super) fn clear_recovery(&mut self) {
        self.recovery_due = None;
        if let Some(Err(err)) = self.recovery.as_ref().map(Session::clear) {
            self.set_status(format!("{err}"), ERROR_STATUS_SECONDS);
        }
    }

    /// Open a left-over copy as an unsaved project with its original path, made this
    /// window's copy at once, and the left-over removed.
    pub fn open_leftover(&mut self, index: usize) -> bool {
        if index >= self.leftovers.len() {
            return false;
        }
        let left = self.leftovers.remove(index);
        let job = match left.open() {
            Ok(job) => job,
            Err(err) => {
                self.error(vec![
                    "Cannot open the recovery copy:".into(),
                    err.to_string(),
                ]);
                self.leftovers.insert(index, left);
                return false;
            }
        };
        let title = leftover_title(&left);
        self.start_editing(EditJob::from_job(&job), false, left.project_path.clone());
        self.write_recovery();
        if let Err(err) = left.delete() {
            self.set_status(format!("{err}"), ERROR_STATUS_SECONDS);
        }
        self.set_report(Report::info([format!(
            "Recovered unsaved work: {title}. Save it to keep it."
        )]));
        true
    }

    /// Delete a left-over copy for good.
    pub fn delete_leftover(&mut self, index: usize) {
        if index < self.leftovers.len()
            && let Err(err) = self.leftovers.remove(index).delete()
        {
            self.set_status(format!("{err}"), ERROR_STATUS_SECONDS);
        }
    }

    /// Recover unsaved work: one row per left-over copy.
    pub(super) fn recovery_window(&mut self, ctx: &egui::Context) {
        if self.leftovers.is_empty() {
            return;
        }
        let dirty = self.dirty();
        let mut choice = None;
        egui::Window::new("Recover unsaved work")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Chipload closed without saving these changes:");
                for (i, left) in self.leftovers.iter().enumerate() {
                    ui.separator();
                    ui.strong(leftover_title(left));
                    if let Some(path) = &left.project_path {
                        ui.label(RichText::new(path.display().to_string()).weak());
                    }
                    ui.label(format!("Changes from {}", ago(left.written)));
                    if left.project_changed {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            "The project file has been saved since this copy was made.",
                        );
                    }
                    if let Some(problem) = &left.problem {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            format!("Can't be opened: {problem}"),
                        );
                    }
                    ui.horizontal(|ui| {
                        let can_open = left.problem.is_none() && !dirty;
                        if ui
                            .add_enabled(can_open, egui::Button::new("Open"))
                            .on_hover_text("Open it as an unsaved project; save to keep it")
                            .on_disabled_hover_text(if dirty {
                                "Save or close the current project first"
                            } else {
                                "This copy can't be read"
                            })
                            .clicked()
                        {
                            choice = Some(LeftoverChoice::Open(i));
                        }
                        if ui.button("Delete").clicked() {
                            choice = Some(LeftoverChoice::Delete(i));
                        }
                        if ui
                            .button("Later")
                            .on_hover_text("Keep it and ask again next time")
                            .clicked()
                        {
                            choice = Some(LeftoverChoice::Later(i));
                        }
                    });
                }
            });
        match choice {
            Some(LeftoverChoice::Open(i)) => {
                self.open_leftover(i);
            }
            Some(LeftoverChoice::Delete(i)) => self.delete_leftover(i),
            // Dropping it releases its lock: offered again at the next start.
            Some(LeftoverChoice::Later(i)) => drop(self.leftovers.remove(i)),
            None => {}
        }
    }
}
