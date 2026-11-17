// SPDX-License-Identifier: GPL-3.0-or-later
//! The Nest window (docs/specs/nesting.md §7): the board, the gap and margin, each part's
//! count and turns; the nest computed on a worker thread and previewed as ghosts on the
//! board, what didn't fit parked beside it; Apply as one undo step. Nest again runs with
//! the setup's kept settings, without the window. The nesting itself is the pipeline's.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use chipload_job::Turns;
use chipload_pipeline::Cancel;
use chipload_pipeline::nest::{NEST_GAP_MM, NEST_MARGIN_MM, NestRequest, NestResult, nest_setup};

use super::*;

type Outcome = Result<Result<NestResult, PipelineError>, String>;

/// Each instance's closed outlines in a preview, parked (true) when it didn't fit.
type Ghosts = Vec<(Vec<Vec<Point>>, bool)>;

/// A nest being computed.
struct NestRun {
    cancel: Arc<Cancel>,
    receiver: Receiver<Outcome>,
}

/// One part's row: how many times it is cut and how it may turn.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct NestRow {
    pub(super) part: u64,
    pub(super) name: String,
    pub(super) count: u32,
    pub(super) turns: Turns,
}

/// What the window shows of a nest computed for its settings.
pub(super) struct NestPreview {
    pub(super) result: NestResult,
    /// Each instance's closed outlines where it would go, parked when it didn't fit.
    ghosts: Ghosts,
    /// The board nested on.
    board: (Point, Point),
}

/// The Nest window's state, or a Nest again running without it.
pub(super) struct NestDialog {
    pub(super) setup: u64,
    /// The board follows the drawing: nesting gives it the size below.
    pub(super) follows: bool,
    pub(super) board: (f64, f64),
    pub(super) gap: f64,
    pub(super) margin: f64,
    pub(super) rows: Vec<NestRow>,
    run: Option<NestRun>,
    pub(super) preview: Option<Result<NestPreview, String>>,
    /// Apply the nest when the run in progress ends.
    apply_when_done: bool,
    /// Shown as a window; false for Nest again.
    shown: bool,
}

impl NestDialog {
    /// The request these settings make.
    fn request(&self) -> NestRequest {
        NestRequest {
            setup: Some(self.setup),
            board: self.shown.then_some(self.board),
            gap: Some(self.gap),
            margin: Some(self.margin),
            turns: None,
            part_turns: self.rows.iter().map(|r| (r.part, r.turns)).collect(),
            counts: self.rows.iter().map(|r| (r.part, r.count)).collect(),
        }
    }

    /// The settings changed: the preview no longer holds and a run for the old ones stops.
    fn changed(&mut self) {
        self.preview = None;
        if let Some(run) = self.run.take() {
            run.cancel.cancel();
        }
    }
}

impl ChiploadApp {
    /// *Nest…* on setup `setup` (the active one with None): the window with the setup's
    /// board, kept settings and parts, its preview started.
    pub fn start_nest(&mut self, setup: Option<u64>) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let job = &edit.job;
        let Some(id) = setup.or_else(|| job.active_setup_id()) else {
            return;
        };
        let Some(s) = job.setup(id) else {
            return;
        };
        let follows = s.stock.fit.is_some() || s.stock.origin == chipload_job::StockOrigin::Drawing;
        let board = match (follows, &job.machine.travel) {
            (true, Some(travel)) => (travel.x, travel.y),
            (true, None) => job.for_setup(id).stock_size(),
            (false, _) => (s.stock.x, s.stock.y),
        };
        let rows = job
            .parts
            .iter()
            .filter(|p| p.enabled && job.part_on(p.id, id))
            .map(|p| NestRow {
                part: p.id,
                name: p.name.clone(),
                count: job.for_setup(id).count(p.id),
                turns: p.turns.unwrap_or_default(),
            })
            .collect();
        self.nest = Some(Box::new(NestDialog {
            setup: id,
            follows,
            board,
            gap: s.nest.map_or(NEST_GAP_MM, |n| n.gap),
            margin: s.nest.map_or(NEST_MARGIN_MM, |n| n.margin),
            rows,
            run: None,
            preview: None,
            apply_when_done: false,
            shown: true,
        }));
        self.start_nest_run();
    }

    /// *Nest again* on setup `setup`: nested with its kept settings, applied when done, no
    /// window.
    pub fn nest_again(&mut self, setup: u64) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let Some(s) = edit.job.setup(setup) else {
            return;
        };
        self.nest = Some(Box::new(NestDialog {
            setup,
            follows: false,
            board: (s.stock.x, s.stock.y),
            gap: s.nest.map_or(NEST_GAP_MM, |n| n.gap),
            margin: s.nest.map_or(NEST_MARGIN_MM, |n| n.margin),
            rows: Vec::new(),
            run: None,
            preview: None,
            apply_when_done: true,
            shown: false,
        }));
        self.start_nest_run();
    }

    /// Whether setup `setup` has been nested, so *Nest again* has its settings.
    pub(super) fn was_nested(&self, setup: u64) -> bool {
        self.edit
            .as_ref()
            .and_then(|e| e.job.setup(setup))
            .is_some_and(|s| s.nest.is_some())
    }

    /// Nest the project as it is with the window's settings on a worker thread.
    pub(super) fn start_nest_run(&mut self) {
        let Some(job) = self.edit.as_ref().map(|e| e.job.clone()) else {
            return;
        };
        let lang = crate::i18n::Lang::lang(self);
        let Some(dialog) = self.nest.as_mut() else {
            return;
        };
        dialog.changed();
        let request = dialog.request();
        let (sender, receiver) = channel();
        let cancel = Arc::new(Cancel::new());
        let flag = Arc::clone(&cancel);
        let spawned = std::thread::Builder::new()
            .name("nest".into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    nest_setup(&job, &request, &flag)
                }))
                .map_err(|_| t!(lang, "nest-stopped-unexpectedly"));
                let _ = sender.send(outcome);
            });
        match spawned {
            Ok(_) => dialog.run = Some(NestRun { cancel, receiver }),
            Err(err) => {
                dialog.preview = Some(Err(err.to_string()));
            }
        }
    }

    /// The run's result, if it has arrived. Whether one still runs.
    pub(super) fn poll_nest(&mut self) -> bool {
        let Some(run) = self.nest.as_ref().and_then(|d| d.run.as_ref()) else {
            return false;
        };
        let outcome = match run.receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return true,
            Err(TryRecvError::Disconnected) => Err(t!(self, "nest-stopped-unexpectedly")),
        };
        self.finish_nest(outcome);
        false
    }

    /// Wait for the run in progress and take its result (tests).
    #[cfg(test)]
    pub fn wait_for_nest(&mut self) {
        let Some(run) = self.nest.as_mut().and_then(|d| d.run.take()) else {
            return;
        };
        let outcome = run
            .receiver
            .recv()
            .unwrap_or_else(|_| Err(t!(self, "nest-stopped-unexpectedly")));
        self.finish_nest(outcome);
    }

    fn finish_nest(&mut self, outcome: Outcome) {
        let units = self.prefs.units.into();
        let lang = self.prefs.language;
        let outcome = match outcome {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(err)) => Err(err.message().render(lang, units)),
            Err(text) => Err(text),
        };
        let Some(mut dialog) = self.nest.take() else {
            return;
        };
        dialog.run = None;
        if dialog.apply_when_done {
            match outcome {
                Ok(result) => self.apply_nest(result),
                Err(text) => {
                    self.report.push(Tone::Warning, text);
                    if dialog.shown {
                        dialog.apply_when_done = false;
                        dialog.preview = Some(Err(String::new()));
                        self.nest = Some(dialog);
                    }
                }
            }
            return;
        }
        dialog.preview = Some(outcome.map(|result| self.nest_preview(result)));
        self.nest = Some(dialog);
    }

    /// The ghosts and board a nest shows.
    fn nest_preview(&self, result: NestResult) -> NestPreview {
        let shown = result.job.for_setup(result.setup);
        let board = shown.stock_bounds();
        let preview = result.preview_job();
        let parts: Vec<u64> = preview
            .parts
            .iter()
            .filter(|p| preview.part_on(p.id, result.setup))
            .map(|p| p.id)
            .collect();
        let ghosts =
            chipload_pipeline::parts::instance_outlines_showing(&preview, &self.contours, &parts)
                .into_iter()
                .filter(|i| i.enabled && parts.contains(&i.part))
                .map(|i| {
                    let parked = result.unplaced.contains(&(i.part, i.id));
                    let outlines = i
                        .outlines
                        .into_iter()
                        .map(|(mut points, closed)| {
                            if let (true, Some(&first)) = (closed, points.first()) {
                                points.push(first);
                            }
                            points
                        })
                        .collect();
                    (outlines, parked)
                })
                .collect();
        NestPreview {
            result,
            ghosts,
            board,
        }
    }

    /// The nest applied as one undo step, its summary in the status bar and its notes in
    /// the Report.
    pub(super) fn apply_nest(&mut self, result: NestResult) {
        let units = self.prefs.units.into();
        let lang = self.prefs.language;
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        edit.job = result.job.clone();
        self.commit();
        self.set_status(result.summary().render(lang, units), STATUS_SECONDS);
        for note in result.notes() {
            self.report.push(Tone::Warning, note.render(lang, units));
        }
        self.nest = None;
    }

    /// The preview's ghosts and board for the canvas; nothing while none is shown.
    pub(super) fn nest_scene(&self) -> (Ghosts, Option<(Point, Point)>) {
        match self.nest.as_ref().and_then(|d| d.preview.as_ref()) {
            Some(Ok(p)) => (p.ghosts.clone(), Some(p.board)),
            _ => (Vec::new(), None),
        }
    }

    /// The window, while open.
    pub(super) fn nest_window(&mut self, ctx: &egui::Context) {
        if self.poll_nest() {
            ctx.request_repaint_after(generation::POLL);
        }
        let Some(mut dialog) = self.nest.take() else {
            return;
        };
        if !dialog.shown {
            self.nest = Some(dialog);
            return;
        }
        // With one board, no setup's name (docs/specs/parts-and-setups.md §2a).
        let title = match self.edit.as_ref().map(|e| e.job.setups.as_slice()) {
            Some([_]) | None => t!(ctx, "nest-title-one"),
            Some(setups) => {
                let name = setups
                    .iter()
                    .find(|s| s.id == dialog.setup)
                    .map_or(String::new(), |s| s.name.clone());
                t!(ctx, "nest-title", setup = name)
            }
        };
        let mut open = true;
        let mut action: Option<NestAction> = None;
        let units = self.prefs.units.into();
        let lang = self.prefs.language;
        egui::Window::new(title)
            .id(egui::Id::new("nest-window"))
            .open(&mut open)
            .resizable(false)
            .fade_in(false)
            .default_pos(ctx.content_rect().right_top() + egui::vec2(-340.0, 80.0))
            .show(ctx, |ui| {
                ui.set_width(320.0);
                action = nest_form(ui, &mut dialog, lang, units);
            });
        if !open {
            action = Some(NestAction::Cancel);
        }
        match action {
            Some(NestAction::Changed) => {
                dialog.changed();
                self.nest = Some(dialog);
            }
            Some(NestAction::Preview) => {
                self.nest = Some(dialog);
                self.start_nest_run();
            }
            Some(NestAction::Apply) => match dialog.preview.take() {
                Some(Ok(p)) => self.apply_nest(p.result),
                // Not previewed yet: nested now, applied when done.
                _ => {
                    dialog.apply_when_done = true;
                    let idle = dialog.run.is_none();
                    self.nest = Some(dialog);
                    if idle {
                        self.start_nest_run();
                    }
                }
            },
            Some(NestAction::Cancel) => {
                dialog.changed();
                self.nest = None;
            }
            None => self.nest = Some(dialog),
        }
    }
}

/// What the window asks for.
enum NestAction {
    Changed,
    Preview,
    Apply,
    Cancel,
}

/// The window's content: the board, gap and margin, a row per part, the result, the
/// buttons.
fn nest_form(
    ui: &mut egui::Ui,
    dialog: &mut NestDialog,
    lang: crate::i18n::Language,
    units: chipload_i18n::Units,
) -> Option<NestAction> {
    let mut changed = false;
    if dialog.follows {
        ui.label(t!(ui, "nest-board-follows"));
        ui.add_space(4.0);
    }
    egui::Grid::new("nest-settings")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "nest-board"));
            ui.horizontal(|ui| {
                let range = 1.0..=100_000.0;
                changed |= ui
                    .add(crate::units::length(&mut dialog.board.0).range(range.clone()))
                    .changed();
                ui.label("×");
                changed |= ui
                    .add(crate::units::length(&mut dialog.board.1).range(range))
                    .changed();
            });
            ui.end_row();
            ui.label(t!(ui, "nest-gap"))
                .on_hover_text(t!(ui, "nest-gap-hover"));
            changed |= ui
                .add(crate::units::length(&mut dialog.gap).range(0.0..=1000.0))
                .changed();
            ui.end_row();
            ui.label(t!(ui, "nest-margin"))
                .on_hover_text(t!(ui, "nest-margin-hover"));
            changed |= ui
                .add(crate::units::length(&mut dialog.margin).range(0.0..=1000.0))
                .changed();
            ui.end_row();
        });
    ui.add_space(6.0);
    egui::Grid::new("nest-parts")
        .num_columns(3)
        .striped(true)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.strong(t!(ui, "nest-part"));
            ui.strong(t!(ui, "nest-count"));
            ui.strong(t!(ui, "nest-turns"));
            ui.end_row();
            for (k, row) in dialog.rows.iter_mut().enumerate() {
                ui.label(&row.name);
                changed |= ui
                    .add(egui::DragValue::new(&mut row.count).range(1..=999))
                    .on_hover_text(t!(ui, "nest-count-hover"))
                    .changed();
                egui::ComboBox::from_id_salt(("nest-turns", k))
                    .selected_text(turns_text(ui, row.turns))
                    .show_ui(ui, |ui| {
                        for turns in Turns::ALL {
                            changed |= ui
                                .selectable_value(&mut row.turns, turns, turns_text(ui, turns))
                                .changed();
                        }
                    });
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    match (&dialog.preview, dialog.run.is_some()) {
        (_, true) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!(ui, "nest-running"));
            });
        }
        (Some(Ok(p)), false) => {
            ui.label(p.result.summary().render(lang, units));
            for note in p.result.notes() {
                ui.colored_label(ui.visuals().warn_fg_color, note.render(lang, units));
            }
        }
        (Some(Err(text)), false) if !text.is_empty() => {
            ui.colored_label(ui.visuals().error_fg_color, text);
        }
        _ => {
            ui.label(t!(ui, "nest-stale"));
        }
    }
    ui.add_space(8.0);
    let mut action = changed.then_some(NestAction::Changed);
    ui.horizontal(|ui| {
        let ready = matches!(dialog.preview, Some(Ok(_)));
        let running = dialog.run.is_some();
        if super::dialogs::accent_button(ui, &t!(ui, "nest-apply")).clicked() && !running {
            action = Some(NestAction::Apply);
        }
        if ui
            .add_enabled(
                !ready && !running,
                egui::Button::new(t!(ui, "nest-preview")),
            )
            .clicked()
        {
            action = Some(NestAction::Preview);
        }
        if ui.button(t!(ui, "common-cancel")).clicked() {
            action = Some(NestAction::Cancel);
        }
    });
    action
}

/// "Quarter turns" and the rest, for the turns menu.
fn turns_text(ui: &egui::Ui, turns: Turns) -> String {
    match turns {
        Turns::None => t!(ui, "nest-turns-none"),
        Turns::Half => t!(ui, "nest-turns-half"),
        Turns::Quarter => t!(ui, "nest-turns-quarter"),
        Turns::Every15 => t!(ui, "nest-turns-every-15"),
    }
}
