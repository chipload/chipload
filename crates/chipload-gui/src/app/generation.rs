// SPDX-License-Identifier: GPL-3.0-or-later
//! Generating on a worker thread, and the live preview (docs/specs/background-generation.md):
//! the window stays live while a project is planned, a newer run cancels an older one, and
//! a result is shown only for the project it was made from.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use chipload_pipeline::{Cancel, run_job_cancellable};
use chipload_toolpath::ToolpathError;

use super::*;

/// What plans a project: `run_job_cancellable`, or a stand-in in tests.
pub(super) type Runner = fn(&Job, &Cancel) -> Result<RunResult, PipelineError>;

pub(super) const RUNNER: Runner = run_job_cancellable;

/// How long the project stays unchanged before the live preview generates (§2, decision 3).
pub(super) const QUIET_TIME: Duration = Duration::from_millis(500);

/// How often the window looks for the worker's result while it runs.
pub(super) const POLL: Duration = Duration::from_millis(100);

/// The worker's answer; `Err`: it panicked, with the panic's text.
type Outcome = Result<Result<RunResult, PipelineError>, String>;

/// A run in progress.
pub(super) struct Generation {
    cancel: Arc<Cancel>,
    receiver: Receiver<Outcome>,
    /// The job being planned, for the report.
    job: Job,
    /// The project as it was when the run started: the result is shown only if it still is.
    project: EditJob,
    /// Started by the user (F5, the button): errors also go to the status bar.
    loud: bool,
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "unknown error".into()
    }
}

impl ChiploadApp {
    /// Generate now (F5, the Generate button): errors are reported loudly.
    pub fn generate(&mut self) -> bool {
        self.start_generation(true)
    }

    /// A run is in progress.
    pub fn generating(&self) -> bool {
        self.generation.is_some()
    }

    /// Plan the project on a worker thread. Returns whether a run started; an incomplete
    /// project is reported instead.
    pub(super) fn start_generation(&mut self, loud: bool) -> bool {
        self.commit();
        self.stop_generation();
        self.changed_at = None;
        let Some(edit) = &self.edit else {
            self.fail(vec!["Import a DXF or open a project first.".into()], loud);
            return false;
        };
        let project = edit.clone();
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_ops = problems.iter().filter_map(|p| operation_index(p)).collect();
                let mut lines = vec!["The project is not complete:".to_string()];
                lines.extend(problems.into_iter().map(|p| format!("  {p}")));
                self.fail(lines, loud);
                return false;
            }
        };
        let (sender, receiver) = channel();
        let cancel = Arc::new(Cancel::new());
        let flag = Arc::clone(&cancel);
        let runner = self.runner;
        let planned = job.clone();
        let spawned = std::thread::Builder::new()
            .name("generate".into())
            .spawn(move || {
                let outcome =
                    std::panic::catch_unwind(AssertUnwindSafe(|| runner(&planned, &flag)))
                        .map_err(|payload| panic_text(payload.as_ref()));
                // The window may have moved on and dropped the receiver: nothing to tell.
                let _ = sender.send(outcome);
            });
        if let Err(err) = spawned {
            self.fail(vec![format!("Generation could not start: {err}")], loud);
            return false;
        }
        self.generation = Some(Generation {
            cancel,
            receiver,
            job,
            project,
            loud,
        });
        true
    }

    /// Stop the run in progress, if any, without a word (a newer one replaces it).
    pub(super) fn stop_generation(&mut self) {
        if let Some(generation) = self.generation.take() {
            generation.cancel.cancel();
        }
    }

    /// The Cancel button: stop the run and say so.
    pub fn cancel_generation(&mut self) {
        if self.generation.is_some() {
            self.stop_generation();
            self.export_when_done = false;
            self.set_status("Generation cancelled".into(), STATUS_SECONDS);
        }
    }

    /// Apply the worker's result if it has arrived. Returns Some(success) when the run is
    /// over.
    pub(super) fn poll_generation(&mut self) -> Option<bool> {
        let outcome = match self.generation.as_ref()?.receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("the worker stopped without a result".into()),
        };
        let generation = self.generation.take()?;
        Some(self.finish_generation(generation, outcome))
    }

    /// Wait for the run in progress and apply its result (tests, screenshots).
    #[cfg(any(test, feature = "screenshots"))]
    pub fn wait_for_generation(&mut self) -> bool {
        let Some(generation) = self.generation.take() else {
            return false;
        };
        let outcome = generation
            .receiver
            .recv()
            .unwrap_or_else(|_| Err("the worker stopped without a result".into()));
        self.finish_generation(generation, outcome)
    }

    /// Generate and wait for the result (tests, screenshots).
    #[cfg(any(test, feature = "screenshots"))]
    pub fn generate_and_wait(&mut self) -> bool {
        self.generate() && self.wait_for_generation()
    }

    fn finish_generation(&mut self, generation: Generation, outcome: Outcome) -> bool {
        if self.edit.as_ref() != Some(&generation.project) {
            return false; // made for a project that has changed since: dropped
        }
        let Generation { job, loud, .. } = generation;
        let done = match outcome {
            Ok(Ok(result)) => {
                self.show_result(&job, result);
                true
            }
            Ok(Err(PipelineError::Toolpath(ToolpathError::Cancelled))) => false,
            Ok(Err(PipelineError::CheckFailed { errors, program })) => {
                self.clear_toolpath();
                self.scene.toolpath = Some(ToolpathShape::new(&program, true, &[]));
                let mut lines = vec!["G-code check FAILED. Nothing can be exported.".to_string()];
                lines.extend(errors);
                self.fail(lines, loud);
                false
            }
            Ok(Err(err)) => {
                self.clear_toolpath();
                let message = err.to_string();
                self.error_ops = operation_index(&message).into_iter().collect();
                self.fail(vec!["Generation failed:".into(), message], loud);
                false
            }
            Err(panic) => {
                self.clear_toolpath();
                self.fail(
                    vec![
                        format!("Generation failed unexpectedly: {panic}"),
                        "Your project is unchanged. Please report this with the project file."
                            .into(),
                    ],
                    loud,
                );
                false
            }
        };
        self.toolpath_stale = false;
        if std::mem::take(&mut self.export_when_done) && done {
            self.ask_export();
        }
        done
    }

    fn show_result(&mut self, job: &Job, result: RunResult) {
        self.error_ops.clear();
        self.scene.toolpath = Some(ToolpathShape::new(&result.program, false, &result.op_spans));
        self.scene.tabs.clone_from(&result.tabs);
        self.scene.start_markers = result
            .ring_starts
            .iter()
            .map(|s| StartMarker::new(s.operation, s.ring.clone(), s.stored))
            .collect();
        let mut report = Report::default();
        let lines: usize = result
            .files
            .iter()
            .map(|f| f.gcode.matches('\n').count())
            .sum();
        let files = match result.files.len() {
            1 => String::new(),
            n => format!(" in {n} files, one per tool group"),
        };
        report.push(
            Tone::Good,
            format!(
                "Generated {lines} lines{files} for {}. Check passed.",
                result.post.name
            ),
        );
        let several = result.files.len() > 1;
        for line in chipload_pipeline::estimate::estimate_lines(job, &result.estimate, several) {
            report.push(Tone::Info, line);
        }
        for note in &result.notes {
            report.push(Tone::Warning, note.clone());
        }
        for warning in &result.check_warnings {
            report.push(Tone::Warning, format!("Check: {}", warning.message));
        }
        report.push(Tone::Info, format!("Work zero: {}", job.work_zero_text()));
        self.set_report(report);
        self.result_job = Some(job.clone());
        self.result = Some(result);
        // A new program: the old simulation goes, a new one starts if switched on.
        self.result_serial += 1;
        self.drop_simulation();
        self.reset_playback();
        self.start_simulation();
    }

    /// The toolpath and everything shown with it, gone.
    fn clear_toolpath(&mut self) {
        self.scene.toolpath = None;
        self.scene.tabs.clear();
        self.scene.start_markers.clear();
        self.result_serial += 1;
        self.drop_simulation();
        self.playback = None;
        self.scene.progress = None;
    }

    /// An error in the report; also in the status bar when the user asked for the run.
    fn fail(&mut self, lines: Vec<String>, loud: bool) {
        if loud {
            self.error(lines);
        } else {
            self.set_report(Report::error(lines));
        }
    }

    /// The live preview: once the project has been unchanged for the quiet time, generate.
    /// Returns how long to wait before looking again, if a run is due later.
    pub(super) fn live_preview(&mut self, now: Instant) -> Option<Duration> {
        let since = now.saturating_duration_since(self.changed_at?);
        if !self.auto_generate || self.edit.is_none() {
            return None;
        }
        if !self.problems.is_empty() {
            // Nothing to make until they are fixed; the Problems strip says so.
            self.changed_at = None;
            return None;
        }
        if since < QUIET_TIME {
            return Some(QUIET_TIME - since);
        }
        self.start_generation(false);
        None
    }

    /// Export G-code: now if the toolpath is current, else once the run in progress (or one
    /// started for it) has finished.
    pub(super) fn export_when_ready(&mut self) {
        if self.result.is_some() {
            self.ask_export();
        } else if self.generating() || self.start_generation(true) {
            self.export_when_done = true;
            self.set_status(
                "Exporting once the toolpath is generated".into(),
                STATUS_SECONDS,
            );
        }
    }

    /// Settings > Write a setup sheet with the G-code, kept in the app settings.
    pub fn set_setup_sheet(&mut self, on: bool) {
        self.setup_sheet = on;
        self.change_settings(|settings| settings.setup_sheet = on);
    }

    /// View > Update toolpath automatically, kept in the app settings.
    pub fn set_auto_generate(&mut self, on: bool) {
        self.auto_generate = on;
        self.change_settings(|settings| settings.auto_generate = on);
    }
}
