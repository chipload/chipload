// SPDX-License-Identifier: GPL-3.0-or-later
//! *Relief…* (docs/specs/relief-wizard.md §6): a relief in six steps, each needing only the
//! ones before it: the model, turn and cut, size and place, what is round it, the tools,
//! then a last look. *Make* adds it all as one undo step through the pipeline's
//! `add_relief`; the window only gathers the answers and shows them.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use chipload_job::library::ToolLibrary;
use chipload_job::{Job, Model, ModelUp, Units as JobUnits};
use chipload_pipeline::models::{ReadModel, read_model};
use chipload_pipeline::new_ops::OpKind;
use chipload_pipeline::relief_wizard::{
    Around, DEFAULT_MARGIN_MM, ReliefModel, ReliefRequest, add_relief, rim_width_for,
};

use super::turn_cut::{ModelViews, PlacedPicture, size_fields, turn_fields};
use super::wizard_tools::{ToolKind, ToolOffer, WizardTool, library_choices, tool_combo};
use super::*;

/// The steps, in order.
const STEPS: usize = 6;
/// The board picture's size, points.
const BOARD_VIEW: egui::Vec2 = egui::vec2(300.0, 260.0);
/// A new relief is at most this share of the board's thickness deep, as an added model.
const MOST_DEPTH_SHARE: f64 = 0.5;

/// The board picture draws a grown outline as the silhouette moved round this many times.
const RIM_PICTURE_STEPS: usize = 24;

/// Where the wizard's model comes from.
#[derive(Clone)]
enum Source {
    Nothing,
    /// A file being read on a worker thread.
    Reading(PathBuf, Arc<Mutex<Receiver<Result<ReadModel, String>>>>),
    File(PathBuf, ReadModel),
    Existing(u64),
}

/// The wizard's answers so far.
#[derive(Clone)]
pub(super) struct ReliefWizard {
    step: usize,
    source: Source,
    /// The model as the answers make it, for the views; None until a model is chosen.
    pub(super) draft: Option<Model>,
    views: ModelViews,
    centred: bool,
    thickness: f64,
    /// The model's top below the board's surface, mm: the relief's start depth.
    sink: f64,
    around: Around,
    margin: f64,
    /// The margin typed in: it no longer follows the roughing tool.
    margin_set: bool,
    corner: f64,
    /// The rim's ramp from horizontal, radians.
    ramp: f64,
    /// How far the outline rim is filled out towards the convex hull, 0 to 1.
    hull: f64,
    finish: Option<WizardTool>,
    rough_on: bool,
    rough: Option<WizardTool>,
    /// A smoothing pass after the roughing, with its tool (docs/specs/stl-relief.md §17).
    smooth: bool,
    allowance: f64,
    end_mill: Option<WizardTool>,
}

impl PartialEq for ReliefWizard {
    fn eq(&self, other: &Self) -> bool {
        self.step == other.step && self.draft == other.draft
    }
}

impl std::fmt::Debug for ReliefWizard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReliefWizard(step {})", self.step)
    }
}

/// The first tool of `kind`: the project's, else the library's.
fn first_tool(job: &Job, library: Option<&ToolLibrary>, kind: ToolKind) -> Option<WizardTool> {
    job.tools
        .iter()
        .find(|t| kind.fits(t.kind, t.angle))
        .map(|t| WizardTool::Project(t.id))
        .or_else(|| {
            library
                .and_then(|l| library_choices(&job.tools, l, kind).first().copied())
                .map(|t| WizardTool::Library(t.uid.clone()))
        })
}

/// A chosen tool's diameter, mm.
fn diameter(tool: &WizardTool, job: &Job, library: Option<&ToolLibrary>) -> Option<f64> {
    match tool {
        WizardTool::Project(id) => job.tool(*id).map(|t| t.diameter),
        WizardTool::Library(uid) => library?
            .tools
            .iter()
            .find(|t| t.uid == *uid)
            .map(|t| t.diameter),
    }
}

/// A chosen tool's name for the summary.
fn tool_name(
    ui: &egui::Ui,
    tool: &WizardTool,
    job: &Job,
    library: Option<&ToolLibrary>,
    units: Units,
) -> String {
    match tool {
        WizardTool::Project(id) => job
            .tool(*id)
            .map(|t| super::forms::tool_label(ui, t, units))
            .unwrap_or_default(),
        WizardTool::Library(uid) => library
            .and_then(|l| l.tools.iter().find(|t| t.uid == *uid))
            .map(|t| format!("+ {}", t.name))
            .unwrap_or_default(),
    }
}

impl ReliefWizard {
    fn new(job: &Job, library: Option<&ToolLibrary>) -> Self {
        ReliefWizard {
            step: 1,
            source: Source::Nothing,
            draft: None,
            views: ModelViews::default(),
            centred: true,
            thickness: job.stock().thickness,
            sink: 0.0,
            around: Around::Rim,
            margin: DEFAULT_MARGIN_MM,
            margin_set: false,
            hull: 0.0,
            corner: 0.0,
            ramp: chipload_job::DEFAULT_RIM.ramp,
            finish: first_tool(job, library, ToolKind::BallNose),
            rough_on: false,
            rough: first_tool(job, library, ToolKind::EndMill),
            smooth: true,
            allowance: chipload_pipeline::DEFAULT_ROUGHING_ALLOWANCE_MM,
            end_mill: None,
        }
    }

    /// A file read: its model as a draft, half the board wide, no deeper than half the
    /// board.
    fn take_file(&mut self, path: PathBuf, read: ReadModel, job: &Job) {
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = path
            .file_stem()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.clone());
        let mut draft = Model {
            id: 0,
            name,
            file,
            sha256: read.sha256.clone(),
            data: read.data.clone(),
            units: JobUnits::Mm,
            up: ModelUp::Z,
            layer: String::new(),
            x: 0.0,
            y: 0.0,
            width: (job.stock_size().0 / 2.0).round().max(1.0),
            rotation: 0.0,
            depth: 1.0,
            proportional: true,
            keep: 1.0,
            picture: None,
        };
        // A picture is a depth map (docs/specs/picture-relief.md §2): no depth of its own.
        if read.picture.is_some() {
            draft.picture = Some(chipload_job::DEFAULT_PICTURE_DEPTH);
            draft.proportional = false;
            draft.depth = chipload_pipeline::models::DEFAULT_PICTURE_DEPTH_MM
                .min(self.thickness * MOST_DEPTH_SHARE);
        }
        self.fit_depth(&mut draft);
        self.draft = Some(draft);
        self.source = Source::File(path, read);
    }

    /// A depth in proportion that would go deeper than half the board: half the board, no
    /// longer in proportion.
    fn fit_depth(&self, draft: &mut Model) {
        let most = self.thickness * MOST_DEPTH_SHARE;
        let deep = chipload_pipeline::models::relief_preview(draft, 8)
            .ok()
            .flatten()
            .is_some_and(|(_, size)| size[2] > most);
        if draft.proportional && deep {
            draft.depth = most;
            draft.proportional = false;
        }
    }

    /// Why the current step can't go on, or None.
    fn blocked(&self, ui: &egui::Ui) -> Option<String> {
        match self.step {
            1 => self
                .draft
                .is_none()
                .then(|| t!(ui, "relief-wizard-need-model")),
            2 => (!self.views.ready()).then(|| t!(ui, "relief-wizard-need-view")),
            3 => {
                let depth = self.views.size().map_or(0.0, |s| s[2]) + self.sink;
                (depth > self.thickness).then(|| {
                    t!(
                        ui,
                        "relief-wizard-too-deep",
                        thickness = crate::units::text(ui.ctx(), self.thickness).as_str()
                    )
                })
            }
            4 => (self.around == Around::Box && self.corner > self.margin)
                .then(|| t!(ui, "relief-wizard-need-margin")),
            5 => {
                if self.finish.is_none() {
                    Some(t!(ui, "relief-wizard-need-finish"))
                } else if self.rough_on && self.rough.is_none() {
                    Some(t!(ui, "relief-wizard-need-rough"))
                } else if self.around == Around::CutOut && self.around_mill().is_none() {
                    Some(t!(ui, "relief-wizard-need-end-mill"))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// How far out the rim's ramp reaches the board's surface from its foot, mm.
    fn rim_run(&self) -> f64 {
        let depth = self.views.size().map_or(0.0, |s| s[2]) + self.sink;
        chipload_job::rim_run(depth, self.ramp)
    }

    /// How far out the rim reaches in all, mm: none for a cut-out.
    fn rim_reach(&self) -> f64 {
        match self.around {
            Around::CutOut => 0.0,
            Around::Rim | Around::Box => self.margin + self.rim_run(),
        }
    }

    /// The end mill for the cut-out: the one chosen, else the roughing one.
    fn around_mill(&self) -> Option<&WizardTool> {
        self.end_mill
            .as_ref()
            .or(self.rough.as_ref().filter(|_| self.rough_on))
    }

    /// Moving on to step 5: rough first when the relief is deeper than the ball nose is
    /// wide (the deep-relief warning's rule).
    fn enter_tools(&mut self, job: &Job, library: Option<&ToolLibrary>) {
        let depth = self.views.size().map_or(0.0, |s| s[2]);
        let width = self
            .finish
            .as_ref()
            .and_then(|t| diameter(t, job, library))
            .unwrap_or(f64::INFINITY);
        self.rough_on = depth > width;
    }
}

impl ChiploadApp {
    /// *Relief…*: open the wizard.
    pub fn start_relief(&mut self) {
        if self.edit.is_none() {
            self.new_project();
        }
        self.reload_library();
        let library = self.library().map(|l| l.tools.clone());
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let wizard = ReliefWizard::new(&edit.job, library.as_ref());
        self.dialogs.prompt = Some(Prompt::Relief(Box::new(wizard)));
    }

    /// Make the relief the wizard's answers ask for, as one undo step.
    fn make_relief(&mut self, w: &ReliefWizard) -> bool {
        self.commit();
        let library = self.library().map(|l| l.tools.clone());
        let lang = self.prefs.language;
        let (Some(edit), Some(draft)) = (self.edit.as_ref(), w.draft.as_ref()) else {
            return false;
        };
        let mut job = edit.job.clone();
        job.stock_mut().thickness = w.thickness;
        let take =
            |job: &mut Job, t: Option<&WizardTool>| t.and_then(|t| t.take(job, library.as_ref()));
        let Some(tool) = take(&mut job, w.finish.as_ref()) else {
            self.error(vec![t!(self, "relief-wizard-need-finish")]);
            return false;
        };
        let rough = if w.rough_on {
            take(&mut job, w.rough.as_ref()).map(|t| (t, w.allowance))
        } else {
            None
        };
        let end_mill = take(&mut job, w.end_mill.as_ref());
        let mut request = ReliefRequest::new(draft.width, tool);
        request.up = draft.up;
        request.rotation = draft.rotation;
        request.keep = draft.keep;
        request.picture = draft.picture;
        request.depth = (!draft.proportional).then_some(draft.depth);
        request.at = (!w.centred).then_some((draft.x, draft.y));
        request.sink = w.sink;
        request.around = w.around;
        request.margin = w.margin;
        request.corner = w.corner;
        request.ramp = w.ramp;
        request.hull = w.hull;
        request.smooth = w.smooth && rough.is_some();
        request.rough = rough;
        request.end_mill = end_mill;
        let made = match &w.source {
            Source::File(path, read) => {
                let file = path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let model = ReliefModel::New {
                    file: &file,
                    read,
                    units: JobUnits::Mm,
                };
                add_relief(&job, model, &request, library.as_ref(), lang)
            }
            Source::Existing(id) => add_relief(
                &job,
                ReliefModel::Existing(*id),
                &request,
                library.as_ref(),
                lang,
            ),
            Source::Nothing | Source::Reading(..) => return false,
        };
        match made {
            Ok((job, made)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(job);
                }
                self.drawing_changed(&[]);
                self.focus = Focus::Op(made.relief);
                self.fit_pending = true;
                self.commit();
                self.view_3d();
                self.set_report(Report::info([t!(
                    self,
                    "relief-wizard-made",
                    count = made.operations.len()
                )]));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "relief-wizard-refused"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Start reading `path` for the wizard on a worker thread.
    fn wizard_read(&self, path: PathBuf) -> Option<Source> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let file = path.clone();
        let lang = crate::i18n::Lang::lang(self);
        std::thread::Builder::new()
            .name("read model".into())
            .spawn(move || {
                let read = std::fs::read(&file)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| {
                        std::panic::catch_unwind(|| read_model(&bytes))
                            .map_err(|_| String::new())?
                            .map_err(|e| e.message().render(lang, chipload_i18n::Units::Mm))
                    });
                let _ = sender.send(read);
            })
            .ok()?;
        Some(Source::Reading(path, Arc::new(Mutex::new(receiver))))
    }

    pub(super) fn relief_prompt(&mut self, ctx: &egui::Context, mut w: Box<ReliefWizard>) {
        let Some(job) = self.edit.as_ref().map(|e| e.job.clone()) else {
            self.dialogs.prompt = None;
            return;
        };
        let library = self.library().map(|l| l.tools.clone());
        let units = self.prefs.units;
        let dark = ctx.global_style().visuals.dark_mode;
        let color = crate::style::op_color(OpKind::Relief, dark);
        // A file being read: taken when read.
        if let Source::Reading(path, receiver) = w.source.clone() {
            let got = receiver.lock().map(|r| r.try_recv());
            match got {
                Ok(Ok(Ok(read))) => {
                    w.take_file(path, read, &job);
                }
                Ok(Ok(Err(why))) => {
                    self.error(vec![t!(
                        self,
                        "models-cannot-read",
                        path = path.display().to_string(),
                        error = why
                    )]);
                    w.source = Source::Nothing;
                }
                Ok(Err(TryRecvError::Empty)) => ctx.request_repaint_after(generation::POLL),
                _ => w.source = Source::Nothing,
            }
        }
        // Centred on the board: placed from the relief's size.
        if w.centred
            && let (Some(draft), Some(size)) = (w.draft.as_mut(), w.views.size())
        {
            let ((x0, y0), (x1, y1)) = job.stock_bounds();
            draft.x = (x0 + x1 - size[0]) / 2.0;
            draft.y = (y0 + y1 - size[1]) / 2.0;
        }
        if let Some(draft) = &w.draft {
            let draft = draft.clone();
            w.views.refresh(ctx, color, &draft);
        }
        // The rim's flat follows the roughing tool until a width is typed in (stl-relief.md
        // §16.6): the tool fits between the model and the ramp.
        if !w.margin_set {
            let rough = w.rough.as_ref().filter(|_| w.rough_on);
            let d = rough.and_then(|t| diameter(t, &job, library.as_ref()));
            w.margin = rim_width_for(d, w.allowance);
        }
        let offer = ToolOffer {
            tools: &job.tools,
            library: library.as_ref(),
            units,
        };
        let mut answer: Option<bool> = None;
        let mut pick_file = false;
        let modal = super::turn_cut::hung_modal("relief-wizard").show(ctx, |ui| {
            ui.label(crate::style::bold(
                ui,
                egui::RichText::new(t!(ui, "relief-wizard-title")).heading(),
            ));
            let name = match w.step {
                1 => t!(ui, "relief-wizard-step-model"),
                2 => t!(ui, "relief-wizard-step-turn"),
                3 => t!(ui, "relief-wizard-step-size"),
                4 => t!(ui, "relief-wizard-step-around"),
                5 => t!(ui, "relief-wizard-step-tools"),
                _ => t!(ui, "relief-wizard-step-check"),
            };
            ui.weak(t!(
                ui,
                "relief-wizard-step",
                step = w.step,
                steps = STEPS,
                name = name
            ));
            ui.add_space(8.0);
            // The step's content scrolls in a short window, so the buttons below it always
            // show (found by the user: on a laptop screen Next fell off the bottom).
            super::turn_cut::scrolled(ui, |ui| match w.step {
                1 => pick_file = step_model(ui, &mut w, &job),
                2 => {
                    w.views.show(ui, color);
                    ui.add_space(8.0);
                    if let Some(draft) = w.draft.as_mut() {
                        egui::Grid::new("relief-wizard-turn")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| turn_fields(ui, draft));
                    }
                }
                _ => {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(380.0);
                            match w.step {
                                3 => step_size(ui, &mut w, &job),
                                4 => step_around(ui, &mut w),
                                5 => step_tools(ui, &mut w, &offer),
                                _ => step_check(ui, &w, &job, library.as_ref(), units),
                            }
                        });
                        let fuller = if w.around == Around::Rim {
                            let ctx = ui.ctx().clone();
                            w.views.fuller_picture(&ctx, w.hull)
                        } else {
                            None
                        };
                        board_picture(ui, &w, &job, color, fuller);
                    });
                }
            });
            ui.add_space(8.0);
            let blocked = w.blocked(ui);
            ui.horizontal(|ui| {
                if w.step > 1 && ui.button(t!(ui, "relief-wizard-back")).clicked() {
                    w.step -= 1;
                }
                if w.step < STEPS {
                    if ui
                        .add_enabled(
                            blocked.is_none(),
                            egui::Button::new(t!(ui, "relief-wizard-next")),
                        )
                        .clicked()
                    {
                        w.step += 1;
                        if w.step == 5 {
                            w.enter_tools(&job, library.as_ref());
                        }
                    }
                } else if ui
                    .button(crate::style::bold(ui, t!(ui, "relief-wizard-make")))
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui.button(t!(ui, "common-cancel")).clicked() {
                    answer = Some(false);
                }
                if let Some(why) = &blocked {
                    ui.weak(why);
                }
            });
        });
        if modal.should_close() && answer.is_none() {
            answer = Some(false);
        }
        if pick_file
            && let Some(path) = rfd::FileDialog::new()
                .add_filter(t!(self, "models-model-filter"), crate::app::MODEL_FILES)
                .add_filter(t!(self, "models-stl-filter"), &["stl", "STL"])
                .add_filter(t!(self, "models-depth-filter"), crate::app::DEPTH_MAP_FILES)
                .pick_file()
            && let Some(reading) = self.wizard_read(path)
        {
            w.draft = None;
            w.source = reading;
        }
        match answer {
            Some(true) => {
                self.dialogs.prompt = None;
                self.make_relief(&w);
            }
            Some(false) => self.dialogs.prompt = None,
            None => self.dialogs.prompt = Some(Prompt::Relief(w)),
        }
    }
}

/// Step 1: a file to choose, or a model already in the project. True: choose a file.
fn step_model(ui: &mut egui::Ui, w: &mut ReliefWizard, job: &Job) -> bool {
    let pick = ui.button(t!(ui, "relief-wizard-choose-file")).clicked();
    match &w.source {
        Source::Reading(path, _) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!(ui, "models-reading", file = file_name(path)));
            });
        }
        Source::File(path, read) => {
            ui.label(t!(
                ui,
                "relief-wizard-file",
                file = file_name(path),
                count = read.report.triangles
            ));
        }
        _ => {}
    }
    if !job.drawing.models.is_empty() {
        ui.add_space(6.0);
        ui.label(t!(ui, "relief-wizard-in-project"));
        for model in &job.drawing.models {
            let chosen = matches!(w.source, Source::Existing(id) if id == model.id);
            if ui.radio(chosen, &model.name).clicked() && !chosen {
                w.source = Source::Existing(model.id);
                w.draft = Some(model.clone());
            }
        }
    }
    pick
}

/// Step 3: width, depth, where it lies, the board's thickness.
fn step_size(ui: &mut egui::Ui, w: &mut ReliefWizard, job: &Job) {
    let ctx = ui.ctx().clone();
    egui::Grid::new("relief-wizard-size")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            if let Some(draft) = w.draft.as_mut() {
                size_fields(ui, draft);
                ui.label(t!(ui, "forms-position"));
                ui.vertical(|ui| {
                    ui.checkbox(&mut w.centred, t!(ui, "relief-wizard-centred"));
                    if !w.centred {
                        ui.horizontal(|ui| {
                            ui.add(crate::units::length(&mut draft.x).prefix("X "));
                            ui.add(crate::units::length(&mut draft.y).prefix("Y "));
                        });
                    }
                });
                ui.end_row();
            }
            ui.label(t!(ui, "relief-wizard-sink"))
                .on_hover_text(t!(ui, "relief-wizard-sink-hover"));
            ui.add(
                crate::units::length(&mut w.sink)
                    .range(0.0..=1_000.0)
                    .speed(0.05),
            );
            ui.end_row();
            ui.label(t!(ui, "relief-wizard-thickness"));
            ui.add(
                crate::units::length(&mut w.thickness)
                    .range(0.1..=1_000.0)
                    .speed(0.1),
            );
            ui.end_row();
        });
    let (bw, bh) = job.stock_size();
    let text = |mm: f64| crate::units::text(&ctx, mm);
    ui.weak(t!(
        ui,
        "relief-wizard-board",
        width = text(bw).as_str(),
        height = text(bh).as_str()
    ));
    if let Some(size) = w.views.size() {
        ui.label(t!(
            ui,
            "turn-cut-size",
            width = text(size[0]).as_str(),
            height = text(size[1]).as_str(),
            depth = text(size[2]).as_str()
        ));
    }
}

/// Step 4: a rim round the outline or round a box, or a cut-out
/// (docs/specs/stl-relief.md §16).
fn step_around(ui: &mut egui::Ui, w: &mut ReliefWizard) {
    let choices = [
        (Around::Rim, t!(ui, "relief-wizard-around-rim")),
        (Around::Box, t!(ui, "relief-wizard-around-box")),
        (Around::CutOut, t!(ui, "relief-wizard-around-cut-out")),
    ];
    for (around, text) in choices {
        ui.radio_value(&mut w.around, around, text);
    }
    if w.around == Around::CutOut {
        return;
    }
    ui.add_space(6.0);
    egui::Grid::new("relief-wizard-around")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "relief-wizard-margin"));
            let typed = ui.add(
                crate::units::length(&mut w.margin)
                    .range(0.0..=1_000.0)
                    .speed(0.1),
            );
            w.margin_set |= typed.changed();
            ui.end_row();
            if w.around == Around::Rim {
                ui.label(t!(ui, "forms-rim-hull"))
                    .on_hover_text(t!(ui, "forms-rim-hull-hover"));
                super::forms::hull_slider(ui, &mut w.hull);
                ui.end_row();
            }
            if w.around == Around::Box {
                ui.label(t!(ui, "relief-wizard-corner"));
                ui.add(
                    crate::units::length(&mut w.corner)
                        .range(0.0..=1_000.0)
                        .speed(0.1),
                );
                ui.end_row();
            }
            ui.label(t!(ui, "relief-wizard-ramp"))
                .on_hover_text(t!(ui, "relief-wizard-ramp-hover"));
            let mut degrees = w.ramp.to_degrees();
            let least = chipload_job::RIM_MIN_RAMP.to_degrees();
            if ui
                .add(
                    crate::units::number(ui, &mut degrees)
                        .range(least..=90.0)
                        .speed(1.0)
                        .suffix("°"),
                )
                .changed()
            {
                w.ramp = degrees.to_radians();
            }
            ui.end_row();
        });
}

/// Step 5: only the tools the answers need.
fn step_tools(ui: &mut egui::Ui, w: &mut ReliefWizard, offer: &ToolOffer<'_>) {
    egui::Grid::new("relief-wizard-tools")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "relief-wizard-finish"));
            tool_combo(
                ui,
                "relief-finish",
                &mut w.finish,
                offer,
                ToolKind::BallNose,
                None,
            );
            ui.end_row();
            ui.checkbox(&mut w.rough_on, t!(ui, "relief-wizard-rough"))
                .on_hover_text(t!(ui, "relief-wizard-rough-hover"));
            ui.add_enabled_ui(w.rough_on, |ui| {
                tool_combo(
                    ui,
                    "relief-rough",
                    &mut w.rough,
                    offer,
                    ToolKind::EndMill,
                    None,
                );
            });
            ui.end_row();
            if w.rough_on {
                ui.label("");
                ui.checkbox(&mut w.smooth, t!(ui, "relief-wizard-smooth"))
                    .on_hover_text(t!(ui, "relief-wizard-smooth-hover"));
                ui.end_row();
                ui.label(t!(ui, "relief-wizard-allowance"));
                ui.add(
                    crate::units::length(&mut w.allowance)
                        .range(0.0..=10.0)
                        .speed(0.05),
                );
                ui.end_row();
            }
            if w.around == Around::CutOut {
                ui.label(t!(ui, "relief-wizard-end-mill"));
                let same = t!(ui, "relief-wizard-same-as-rough");
                let none = w.rough_on.then_some(same.as_str());
                tool_combo(
                    ui,
                    "relief-around",
                    &mut w.end_mill,
                    offer,
                    ToolKind::EndMill,
                    none,
                );
                ui.end_row();
            }
        });
}

/// Step 6: what will be made, in cutting order, with its tools.
fn step_check(
    ui: &mut egui::Ui,
    w: &ReliefWizard,
    job: &Job,
    library: Option<&ToolLibrary>,
    units: Units,
) {
    let text = |mm: f64| crate::units::text(ui.ctx(), mm);
    let name = |t: &WizardTool| tool_name(ui, t, job, library, units);
    let size = w.views.size().unwrap_or([0.0; 3]);
    let mut lines: Vec<String> = Vec::new();
    let mut tools: Vec<WizardTool> = Vec::new();
    if let (true, Some(rough)) = (w.rough_on, &w.rough) {
        lines.push(t!(
            ui,
            "relief-wizard-summary-rough",
            tool = name(rough),
            allowance = text(w.allowance).as_str()
        ));
        if w.smooth {
            lines.push(t!(ui, "relief-wizard-summary-smooth"));
        }
        tools.push(rough.clone());
    }
    if let Some(finish) = &w.finish {
        lines.push(t!(
            ui,
            "relief-wizard-summary-relief",
            width = text(size[0]).as_str(),
            height = text(size[1]).as_str(),
            depth = text(size[2]).as_str(),
            tool = name(finish)
        ));
        if w.sink > 0.0
            && let Some(line) = lines.last_mut()
        {
            let sunk = t!(
                ui,
                "relief-wizard-summary-sink",
                sink = text(w.sink).as_str()
            );
            line.push_str(&sunk);
        }
        tools.push(finish.clone());
        // The rim is the relief's own: said on its line.
        if w.around != Around::CutOut
            && let Some(line) = lines.last_mut()
        {
            let rim = t!(
                ui,
                "relief-wizard-summary-rim",
                width = text(w.margin).as_str(),
                angle = format!("{:.0}", w.ramp.to_degrees())
            );
            *line = format!("{line}; {rim}");
        }
    }
    if w.around == Around::CutOut
        && let Some(mill) = w.around_mill()
    {
        lines.push(t!(ui, "relief-wizard-summary-cut-out", tool = name(mill)));
        tools.push(mill.clone());
    }
    ui.label(t!(ui, "relief-wizard-summary-part"));
    for (k, line) in lines.iter().enumerate() {
        ui.label(format!("{}. {line}", k + 1));
    }
    let changes = tools.windows(2).filter(|p| p[0] != p[1]).count();
    ui.add_space(4.0);
    ui.label(t!(ui, "relief-wizard-summary-changes", count = changes));
    if job.stock().fit.is_some() && w.around != Around::CutOut {
        ui.weak(t!(ui, "relief-wizard-summary-board-grows"));
    }
}

/// The board seen from above with the relief where it will lie, and its rim and ramp or
/// the cut-out's ring round it.
fn board_picture(
    ui: &mut egui::Ui,
    w: &ReliefWizard,
    job: &Job,
    color: egui::Color32,
    fuller: Option<PlacedPicture>,
) {
    let (rect, _) = ui.allocate_exact_size(BOARD_VIEW, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    // What must show: the board, and the relief with its margin.
    let picture = w.views.placed_picture();
    let (mut lo, mut hi) = ((x0, y0), (x1, y1));
    if let Some(PlacedPicture {
        corner: (px, py),
        size: (pw, ph),
        ..
    }) = picture
    {
        let m = w.rim_reach();
        lo = (lo.0.min(px - m), lo.1.min(py - m));
        hi = (hi.0.max(px + pw + m), hi.1.max(py + ph + m));
    }
    let span = ((hi.0 - lo.0).max(1e-6), (hi.1 - lo.1).max(1e-6));
    let scale = (0.9 * rect.width() / span.0 as f32).min(0.9 * rect.height() / span.1 as f32);
    let middle = ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0);
    let at = |x: f64, y: f64| {
        rect.center()
            + egui::vec2(
                (x - middle.0) as f32 * scale,
                -((y - middle.1) as f32) * scale,
            )
    };
    let palette = crate::view::Palette::of(ui.visuals().dark_mode);
    painter.rect_filled(
        egui::Rect::from_two_pos(at(x0, y0), at(x1, y1)),
        0.0,
        palette.stock.gamma_multiply(0.5),
    );
    let Some(PlacedPicture {
        texture,
        corner: (px, py),
        size: (pw, ph),
    }) = picture
    else {
        return;
    };
    let shown = egui::Rect::from_two_pos(at(px, py), at(px + pw, py + ph));
    let whole = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let ring = ui.visuals().strong_text_color();
    // The ramp (shaded) and the flat (lowered: drawn darkest) round the relief.
    let flat = ui.visuals().extreme_bg_color;
    let slope = palette.stock.gamma_multiply(0.25);
    match w.around {
        Around::Box => {
            let (cx, cy) = (px + pw / 2.0, py + ph / 2.0);
            let ring = |grow: f64, corner: f64, fill: egui::Color32| {
                let (hx, hy) = (pw / 2.0 + grow, ph / 2.0 + grow);
                let r = egui::Rect::from_two_pos(at(cx - hx, cy - hy), at(cx + hx, cy + hy));
                let radius = (corner as f32 * scale).min(r.width().min(r.height()) / 2.0);
                painter.rect_filled(r, radius, fill);
                painter.rect_stroke(
                    r,
                    radius,
                    egui::Stroke::new(1.0, color),
                    egui::StrokeKind::Middle,
                );
            };
            ring(w.rim_reach(), w.corner + w.rim_run(), slope);
            ring(w.margin, w.corner, flat);
        }
        Around::Rim => {
            // The silhouette, filled out as the rim fills it (stl-relief.md §16.7), moved
            // round in a circle: the outline grown, near enough.
            let stamp = fuller.map_or(texture, |f| f.texture);
            let grown = |grow: f64, fill: egui::Color32| {
                let d = grow as f32 * scale;
                for k in 0..RIM_PICTURE_STEPS {
                    let a = std::f32::consts::TAU * k as f32 / RIM_PICTURE_STEPS as f32;
                    let offset = egui::vec2(d * a.cos(), d * a.sin());
                    painter.image(stamp, shown.translate(offset), whole, fill);
                }
            };
            grown(w.rim_reach(), slope.to_opaque());
            grown(w.margin, flat);
        }
        Around::CutOut => {
            // The relief's silhouette, a little larger behind it: the cut round it.
            for (dx, dy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0)] {
                painter.image(texture, shown.translate(egui::vec2(dx, dy)), whole, ring);
            }
        }
    }
    painter.image(texture, shown, whole, egui::Color32::WHITE);
    // The board's edge over it all, so it shows where the pocket reaches it.
    painter.rect_stroke(
        egui::Rect::from_two_pos(at(x0, y0), at(x1, y1)),
        0.0,
        egui::Stroke::new(1.0, palette.stock),
        egui::StrokeKind::Middle,
    );
}

#[cfg(test)]
pub(super) fn wizard_of(app: &mut ChiploadApp) -> Option<&mut ReliefWizard> {
    match &mut app.dialogs.prompt {
        Some(Prompt::Relief(w)) => Some(w),
        _ => None,
    }
}

#[cfg(test)]
impl ReliefWizard {
    pub(super) fn step(&self) -> usize {
        self.step
    }

    pub(super) fn choose_existing(&mut self, model: &Model) {
        self.source = Source::Existing(model.id);
        self.draft = Some(model.clone());
    }

    pub(super) fn set_around(&mut self, around: Around) {
        self.around = around;
    }

    pub(super) fn set_hull(&mut self, hull: f64) {
        self.hull = hull;
    }

    /// Whether the board picture shows the shadow filled out (a texture made for it).
    pub(super) fn shows_fuller(&self) -> bool {
        self.views.has_fuller()
    }
}
