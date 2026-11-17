// SPDX-License-Identifier: GPL-3.0-or-later
//! *Export all setups…* (docs/specs/parts-and-setups.md §8): every setup's programs and
//! sheets written at once, as `chipload run` writes them, planned on a worker thread so the
//! window stays live. Nothing is written unless every setup plans and passes the checker.
//! The planning and writing are the pipeline's.

use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use chipload_pipeline::export_all::{ExportError, Exported, export_all};

use super::dialogs::save_dialog;
use super::*;

type Outcome = Result<Result<Exported, ExportError>, String>;

/// An export of every setup in progress.
pub(super) struct ExportAllRun {
    receiver: Receiver<Outcome>,
    /// The file the programs are named after.
    output: PathBuf,
}

impl ChiploadApp {
    /// Whether *Export all setups…* has something to do: a project with two setups or more.
    pub(super) fn can_export_all(&self) -> bool {
        self.export_all.is_none() && self.edit.as_ref().is_some_and(|e| e.job.setups.len() > 1)
    }

    /// Ask where, then export every setup.
    pub(super) fn ask_export_all(&mut self) {
        let suggested = self
            .remembered_export()
            .unwrap_or_else(|| self.default_export_path());
        if let Some(path) = save_dialog(&suggested, "G-code", "nc") {
            self.start_export_all(&path);
        }
    }

    /// Plan and write every setup's programs next to `path`, and its sheet when the
    /// setting asks for one, on a worker thread. Refused, with the problems, for a
    /// project that isn't complete. Whether it started.
    pub fn start_export_all(&mut self, path: &Path) -> bool {
        if self.export_all.is_some() {
            return false;
        }
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        let info = self.prefs.setup_sheet.then(|| self.sheet_info(Vec::new()));
        let lang = crate::i18n::Lang::lang(self);
        let (sender, receiver) = channel();
        let output = path.to_path_buf();
        let target = output.clone();
        let spawned = std::thread::Builder::new()
            .name("export-all".into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    export_all(&job, &target, info.as_ref())
                }))
                .map_err(|_| t!(lang, "app-export-all-stopped"));
                let _ = sender.send(outcome);
            });
        match spawned {
            Ok(_) => {
                self.export_all = Some(ExportAllRun { receiver, output });
                self.set_status(t!(self, "app-export-all-running"), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error(vec![err.to_string()]);
                false
            }
        }
    }

    /// The export's result, if it has arrived; a repaint asked for while it runs.
    pub(super) fn poll_export_all(&mut self, ctx: &egui::Context) {
        let Some(run) = &self.export_all else {
            return;
        };
        let outcome = match run.receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(generation::POLL);
                return;
            }
            Err(TryRecvError::Disconnected) => Err(t!(self, "app-export-all-stopped")),
        };
        self.finish_export_all(outcome);
    }

    /// Wait for the export in progress and take its result (tests).
    #[cfg(test)]
    pub fn wait_for_export_all(&mut self) {
        let Some(run) = &self.export_all else {
            return;
        };
        let outcome = run
            .receiver
            .recv()
            .unwrap_or_else(|_| Err(t!(self, "app-export-all-stopped")));
        self.finish_export_all(outcome);
    }

    fn finish_export_all(&mut self, outcome: Outcome) {
        let Some(run) = self.export_all.take() else {
            return;
        };
        let shown = run.output.display().to_string();
        match outcome {
            Ok(Ok(done)) => {
                let mut report = Report::default();
                report.push(
                    Tone::Good,
                    t!(
                        self,
                        "app-exported-all",
                        setups = done.setups.len(),
                        count = done.files.len(),
                        path = shown.as_str()
                    ),
                );
                for file in done.files.iter().chain(&done.sheets) {
                    report.push(Tone::Info, file.display().to_string());
                }
                self.set_report(report);
                let text = t!(
                    self,
                    "app-exported-all",
                    setups = done.setups.len(),
                    count = done.files.len(),
                    path = shown.as_str()
                );
                self.set_status(text, STATUS_SECONDS);
            }
            Ok(Err(ExportError::Run(err))) => {
                self.error_report(Report::failed(
                    t!(self, "app-export-all-failed"),
                    err.message(),
                ));
            }
            Ok(Err(ExportError::Write { path, error })) => {
                // The file system's own words.
                self.error(vec![
                    t!(
                        self,
                        "app-could-not-export",
                        path = path.display().to_string()
                    ),
                    error,
                ]);
            }
            Err(text) => self.error(vec![text]),
        }
    }
}
