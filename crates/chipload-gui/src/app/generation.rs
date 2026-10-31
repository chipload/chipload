// SPDX-License-Identifier: GPL-3.0-or-later
//! Generating on a worker thread, and the live preview (docs/specs/background-generation.md):
//! the window stays live while a project is planned, a newer run cancels an older one, and
//! a result is shown only for the project it was made from.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use chipload_i18n::Message;
use chipload_job::Problem;
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
    /// The one part planned for the preview; None: the whole project
    /// (docs/specs/active-part-preview.md).
    scope: Option<u64>,
    /// What the run is made from: an edit that leaves it so keeps the run
    /// (docs/specs/edit-and-simulate.md §5).
    inputs: Option<chipload_pipeline::CutInputs>,
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
        self.start_generation_of(loud, false)
    }

    /// The part a preview plans: the active one, when the active setup has several parts
    /// that are cut and "All parts" is off (docs/specs/active-part-preview.md §1), and it is
    /// in that setup with operations of its own (an empty part has nothing to show; the
    /// setup's parts are previewed then, docs/specs/setups.md §7). None: all of the setup's.
    pub(super) fn preview_part(&self) -> Option<u64> {
        let edit = self.edit.as_ref()?;
        let settings = &edit.settings;
        let setup = settings.active_setup_id();
        let in_setup = |p: &chipload_job::Part| p.enabled && Some(p.setup) == setup;
        let cut = settings.parts.iter().filter(|p| in_setup(p)).count();
        let active = settings.active_part_id()?;
        let on = settings.part(active).is_some_and(in_setup);
        let has_operations = edit.ops.iter().any(|e| e.op.part() == active);
        (!self.all_parts && cut > 1 && on && has_operations).then_some(active)
    }

    /// As `start_generation`; `whole` plans every part whatever the preview shows (for an
    /// export).
    pub(super) fn start_generation_of(&mut self, loud: bool, whole: bool) -> bool {
        let scope = if whole { None } else { self.preview_part() };
        self.start_generation_scoped(loud, scope)
    }

    /// Plan only part `scope`, as if every other part were switched off, or every part
    /// that is cut (None).
    pub(super) fn start_generation_scoped(&mut self, loud: bool, scope: Option<u64>) -> bool {
        self.commit();
        self.stop_generation();
        self.changed_at = None;
        let Some(edit) = &self.edit else {
            self.fail(vec![t!(self, "generation-no-project")], loud);
            return false;
        };
        let project = edit.clone();
        let job = match edit.to_job() {
            // Only the active part: the others are planned as if switched off.
            Ok(mut job) => {
                if let Some(id) = scope {
                    for part in &mut job.parts {
                        part.enabled = part.id == id;
                    }
                }
                job
            }
            Err(problems) => {
                self.error_ops = problems.iter().filter_map(Problem::operation).collect();
                self.fail_with(
                    Report::of_problems(&t!(self, "app-project-incomplete"), &problems),
                    loud,
                );
                return false;
            }
        };
        let inputs = self.cut_inputs_now();
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
            let text = t!(self, "generation-could-not-start", error = err.to_string());
            self.fail(vec![text], loud);
            return false;
        }
        self.generation = Some(Generation {
            cancel,
            receiver,
            job,
            project,
            loud,
            scope,
            inputs,
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
            self.export_path_when_done = None;
            self.export_part_when_done = None;
            self.set_status(t!(self, "generation-cancelled"), STATUS_SECONDS);
        }
    }

    /// Apply the worker's result if it has arrived. Returns Some(success) when the run is
    /// over.
    pub(super) fn poll_generation(&mut self) -> Option<bool> {
        let outcome = match self.generation.as_ref()?.receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(t!(self, "generation-worker-stopped")),
        };
        let generation = self.generation.take()?;
        Some(self.finish_generation(generation, outcome))
    }

    /// Wait for the run in progress and apply its result (tests, screenshots).
    #[cfg(test)]
    pub fn wait_for_generation(&mut self) -> bool {
        let Some(generation) = self.generation.take() else {
            return false;
        };
        let outcome = generation
            .receiver
            .recv()
            .unwrap_or_else(|_| Err(t!(self, "generation-worker-stopped")));
        self.finish_generation(generation, outcome)
    }

    /// Generate and wait for the result (tests, screenshots).
    #[cfg(test)]
    pub fn generate_and_wait(&mut self) -> bool {
        self.generate() && self.wait_for_generation()
    }

    fn finish_generation(&mut self, generation: Generation, outcome: Outcome) -> bool {
        // Made for a project that has changed since in a way that can change the run:
        // dropped (a shape no operation selects keeps it).
        if self.edit.as_ref() != Some(&generation.project)
            && (generation.inputs.is_none() || self.cut_inputs_now() != generation.inputs)
        {
            return false;
        }
        let refusable = self.undo_if_refused.take().as_ref() == Some(&generation.project);
        let Generation {
            job,
            loud,
            scope,
            inputs,
            ..
        } = generation;
        let done = match outcome {
            Ok(Ok(result)) => {
                self.result_part = scope;
                self.show_result(&job, result);
                self.shown_inputs = inputs;
                true
            }
            Ok(Err(PipelineError::Toolpath(ToolpathError::Cancelled))) => false,
            Ok(Err(PipelineError::CheckFailed {
                messages, program, ..
            })) => {
                self.clear_toolpath();
                self.scene.toolpath = Some(ToolpathShape::new(&program, true, &[]));
                let mut report = Report::error([t!(self, "generation-check-failed")]);
                for message in messages {
                    report.push_message(Tone::Error, message, None);
                }
                self.fail_with(report, loud);
                false
            }
            Ok(Err(err)) if refusable => {
                // A dropped tab or start the planner refuses goes back where it was, with
                // the reason; the markers come back with the run of the project before.
                let reason = self.say(&err.message());
                self.take_back_edit();
                self.start_generation(false);
                let text = t!(self, "canvas-drop-refused", reason = reason);
                self.set_status(text, ERROR_STATUS_SECONDS);
                false
            }
            Ok(Err(err)) => {
                self.clear_toolpath();
                let op = match &err {
                    PipelineError::Toolpath(err) => err.operation(),
                    _ => None,
                };
                self.error_ops = op.into_iter().collect();
                let mut report = Report::error([t!(self, "generation-failed")]);
                report.push_message(Tone::Error, err.message(), op);
                self.fail_with(report, loud);
                false
            }
            Err(panic) => {
                self.clear_toolpath();
                let lines = vec![
                    t!(self, "generation-failed-unexpectedly", error = panic),
                    t!(self, "generation-please-report"),
                ];
                self.fail(lines, loud);
                false
            }
        };
        self.toolpath_stale = false;
        if std::mem::take(&mut self.export_when_done) && done {
            self.ask_export();
        }
        if let Some(path) = self.export_path_when_done.take()
            && done
        {
            self.export_gcode(&path);
        }
        if let Some((id, path)) = self.export_part_when_done.take()
            && done
        {
            self.export_part(id, &path);
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
        self.corners.unreached.clone_from(&result.unreached_corners);
        self.show_tabs(&result.tab_centres);
        let mut report = Report::default();
        let lines: usize = result
            .files
            .iter()
            .map(|f| f.gcode.matches('\n').count())
            .sum();
        let done = match result.files.len() {
            1 => t!(
                self,
                "generation-done",
                lines = lines,
                post = result.post.name.as_str()
            ),
            n => t!(
                self,
                "generation-done-files",
                lines = lines,
                files = n,
                post = result.post.name.as_str()
            ),
        };
        report.push(Tone::Good, done);
        let several = result.files.len() > 1;
        for line in chipload_pipeline::estimate::estimate_lines(job, &result.estimate, several) {
            report.push_message(Tone::Info, line.message, line.operation);
        }
        for note in &result.notes {
            report.push_message(Tone::Warning, note.message.clone(), note.operation);
        }
        for warning in &result.check_warnings {
            let message = Message::new("generation-check-warning")
                .arg("message", warning.text_message.clone());
            report.push_message(Tone::Warning, message, None);
        }
        let zero = Self::work_zero_message(job);
        report.push_message(Tone::Info, zero, None);
        if let Some(part) = self.result_part.and_then(|id| job.part(id)) {
            let text = t!(self, "generation-one-part-shown", part = part.name.as_str());
            report.push(Tone::Info, text);
        }
        self.set_report(report);
        self.result_job = Some(job.clone());
        self.result = Some(result);
        // A new program: the old simulation goes, a new one starts if switched on.
        self.sim.serial += 1;
        self.drop_simulation();
        self.reset_playback();
        self.start_simulation();
    }

    /// Where X, Y and Z zero are, shown in the app's language and display units: "Work zero:
    /// stock bottom left = drawing (0.000, 0.000) mm, Z0 = stock top".
    fn work_zero_message(job: &chipload_job::Job) -> Message {
        use chipload_i18n::{Digits, Value};
        use chipload_job::{StockCorner, WorkZero, ZZero};
        let at = match job.work_zero() {
            WorkZero::DrawingOrigin {} => "forms-zero-drawing-origin",
            WorkZero::Stock { corner } => match corner {
                StockCorner::Center => "forms-zero-stock-centre",
                StockCorner::BottomLeft => "forms-zero-stock-bottom-left",
                StockCorner::BottomRight => "forms-zero-stock-bottom-right",
                StockCorner::TopLeft => "forms-zero-stock-top-left",
                StockCorner::TopRight => "forms-zero-stock-top-right",
            },
            WorkZero::Point { .. } => "forms-zero-point",
        };
        let z = match job.stock().z_zero {
            ZZero::Top => "units-text-zero-stock-top",
            ZZero::Bottom => "units-text-zero-stock-bottom",
        };
        let (x, y) = job.work_zero_xy();
        let zero = Message::new("units-text-work-zero")
            .arg("at", Message::new(at))
            .arg("point", Value::Point(x, y, Digits::Fixed(3)))
            .arg("z", Message::new(z));
        Message::new("generation-work-zero").arg("zero", zero)
    }

    /// The toolpath and everything shown with it, gone.
    pub(super) fn clear_toolpath(&mut self) {
        self.scene.toolpath = None;
        self.scene.tabs.clear();
        self.scene.start_markers.clear();
        self.scene.tab_markers.clear();
        self.corners.unreached.clear();
        self.sim.serial += 1;
        self.drop_simulation();
        self.playback = None;
        self.scene.progress = None;
    }

    /// An error in the report; also in the status bar when the user asked for the run.
    fn fail(&mut self, lines: Vec<String>, loud: bool) {
        self.fail_with(Report::error(lines), loud);
    }

    /// As `fail`, with the lines leading to their operations.
    fn fail_with(&mut self, report: Report, loud: bool) {
        if loud {
            self.error_report(report);
        } else {
            self.set_report(report);
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

    /// "All parts" on the canvas bar: the preview shows every part, or only the active one;
    /// planned again at once.
    pub fn set_all_parts(&mut self, on: bool) {
        self.all_parts = on;
        if self.edit.is_some() && (self.result.is_some() || self.auto_generate) {
            self.start_generation(false);
        }
    }

    /// Export G-code: now if the toolpath is current, else once the run in progress (or one
    /// started for it) has finished.
    pub(super) fn export_when_ready(&mut self) {
        // An export is always of every part that is cut: a preview of one part is planned
        // again whole first (docs/specs/active-part-preview.md §2).
        let whole = self.result.is_some() && self.result_part.is_none();
        let running_whole = self.generation.as_ref().is_some_and(|g| g.scope.is_none());
        if whole {
            self.ask_export();
        } else if running_whole || self.start_generation_of(true, true) {
            self.export_when_done = true;
            self.set_status(t!(self, "generation-export-when-done"), STATUS_SECONDS);
        }
    }

    /// Settings > Write a setup sheet with the G-code, kept in the app settings.
    /// View → Show grid: kept in the settings for next time.
    /// View > Show drawing: the drawing's contours on the canvas.
    pub fn show_drawing(&self) -> bool {
        self.drawing_hidden.is_none()
    }

    /// Show or hide the drawing; hidden, its contours can't be clicked.
    pub fn set_show_drawing(&mut self, on: bool) {
        self.drawing_hidden = (!on).then(|| self.selection.clone());
    }

    /// A new selection (from the Browser) or a shape being drawn shows the drawing again, so
    /// nothing is edited unseen.
    pub(super) fn keep_drawing_in_view(&mut self) {
        if let Some(at) = &self.drawing_hidden
            && (self.sketch.is_some()
                || (self.selection != *at && self.selection != CanvasSelection::Nothing))
        {
            self.drawing_hidden = None;
        }
    }

    pub fn set_show_grid(&mut self, on: bool) {
        self.show_grid = on;
        self.change_settings(|settings| settings.show_grid = on);
    }

    pub fn set_show_rulers(&mut self, on: bool) {
        self.show_rulers = on;
        self.change_settings(|settings| settings.show_rulers = on);
    }

    pub fn set_setup_sheet(&mut self, on: bool) {
        self.setup_sheet = on;
        self.change_settings(|settings| settings.setup_sheet = on);
    }

    /// View > Update toolpath automatically, kept in the app settings.
    pub fn set_auto_generate(&mut self, on: bool) {
        self.auto_generate = on;
        self.change_settings(|settings| settings.auto_generate = on);
    }

    /// What a run of the project as edited would be made from; None when it can't be
    /// read (docs/specs/edit-and-simulate.md §5).
    pub(super) fn cut_inputs_now(&self) -> Option<chipload_pipeline::CutInputs> {
        let job = self.edit.as_ref()?.to_job().ok()?;
        chipload_pipeline::cut_inputs(&job)
    }

    /// Whether the edit just made leaves the toolpath shown, or the run under way, as it
    /// is: it can't change what they are made from (a shape no operation selects).
    pub(super) fn edit_keeps_cut(&self) -> bool {
        let made_from = match &self.generation {
            Some(generation) => generation.inputs.as_ref(),
            None if self.result.is_some() => self.shown_inputs.as_ref(),
            None => None,
        };
        made_from.is_some_and(|inputs| self.cut_inputs_now().as_ref() == Some(inputs))
    }
}
