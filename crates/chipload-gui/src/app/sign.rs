// SPDX-License-Identifier: GPL-3.0-or-later
//! *New sign…* (docs/specs/sign-maker.md §3, §7): the words, the frame, the style, the
//! edge, the tools and the board in one dialog, with the sign drawn as it will be and the
//! operations it adds. The sign itself is `chipload_pipeline::sign`'s.

use chipload_core::Point;
use chipload_job::Tool;
use chipload_job::library::ToolLibrary;
use chipload_pipeline::frames::{CornerStyle, FrameKind, MOST_BORDERS};
use chipload_pipeline::sign::{
    DEFAULT_BOARD_MARGIN_MM, DEFAULT_BOARD_THICKNESS_MM, DEFAULT_BORDER_WIDTH_MM,
    DEFAULT_RELIEF_MM, SignBoard, SignRequest, SignStyle, add_sign,
};

use super::wizard_tools::{ToolKind, ToolOffer, WizardTool, library_choices, tool_combo};
use super::*;

/// What the sign dialog holds while it is open.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SignSettings {
    /// The selection when the dialog opened: the sign goes round these shapes instead of
    /// new words. Empty: the words.
    pub around: Vec<u64>,
    pub words: String,
    pub font: String,
    pub height: f64,
    /// A sign of a fixed size: the frame `size` (width, height; a round frame's diameter
    /// is its width), the letters as high as fit (docs/specs/sign-size-and-start.md §1).
    pub fixed: bool,
    pub size: (f64, f64),
    pub kind: FrameKind,
    pub corner_set: bool,
    pub corner: f64,
    pub margin: f64,
    pub borders: u32,
    pub gap: f64,
    pub border_corners: Option<CornerStyle>,
    pub double: bool,
    /// Round words with the oval frame (docs/specs/text-on-arc.md §6).
    pub round: bool,
    /// Signs from a list, one per line, instead of the words (docs/specs/batch-signs.md).
    pub from_list: bool,
    pub list: String,
    pub border_width: f64,
    pub style: SignStyle,
    pub relief: f64,
    /// How deep single-line words are engraved (docs/specs/single-line-text.md §2).
    pub engrave_depth: f64,
    pub edge: bool,
    pub edge_width: f64,
    pub vbit: Option<WizardTool>,
    pub end_mill: Option<WizardTool>,
    /// Fit the stock round the sign.
    pub fit_board: bool,
    pub thickness: f64,
    pub board_margin: f64,
}

impl SignSettings {
    /// The defaults (§3): the project's first V-bit and its largest end mill, else the
    /// library's; the board fitted round the sign when the drawing is empty.
    fn for_job(job: &chipload_job::Job, library: Option<&ToolLibrary>) -> Self {
        let from_library = |kind| {
            library
                .and_then(|l| library_choices(&job.tools, l, kind).into_iter().next())
                .map(|t| WizardTool::Library(t.uid.clone()))
        };
        let vbit = job
            .tools
            .iter()
            .find(|t| ToolKind::Vbit.fits(t.kind, t.angle, t.corner_radius))
            .map(|t| WizardTool::Project(t.id))
            .or_else(|| from_library(ToolKind::Vbit));
        let mill = job
            .tools
            .iter()
            .filter(|t| ToolKind::EndMill.fits(t.kind, t.angle, t.corner_radius))
            .max_by(|a, b| a.diameter.total_cmp(&b.diameter))
            .map(|t| WizardTool::Project(t.id))
            .or_else(|| from_library(ToolKind::EndMill));
        Self {
            around: Vec::new(),
            words: String::new(),
            font: chipload_pipeline::texts::DEFAULT_FONT.to_string(),
            height: 30.0,
            fixed: false,
            size: (300.0, 100.0),
            kind: FrameKind::Rounded,
            corner_set: false,
            corner: 8.0,
            margin: 10.0,
            borders: 1,
            gap: 5.0,
            border_corners: None,
            double: false,
            round: false,
            from_list: false,
            list: String::new(),
            border_width: DEFAULT_BORDER_WIDTH_MM,
            style: SignStyle::Engraved,
            relief: DEFAULT_RELIEF_MM,
            engrave_depth: chipload_pipeline::sign::DEFAULT_ENGRAVE_DEPTH_MM,
            edge: false,
            edge_width: 2.0,
            vbit,
            end_mill: mill,
            fit_board: job.drawing.shapes.is_empty(),
            thickness: if job.drawing.shapes.is_empty() {
                DEFAULT_BOARD_THICKNESS_MM
            } else {
                job.stock().thickness
            },
            board_margin: DEFAULT_BOARD_MARGIN_MM,
        }
    }

    /// The project with the library tools chosen copied in, and the pipeline's request;
    /// None without both tools.
    fn request(
        &self,
        job: &chipload_job::Job,
        library: Option<&ToolLibrary>,
    ) -> Option<(chipload_job::Job, SignRequest)> {
        let mut job = job.clone();
        let vbit = self.vbit.as_ref()?.take(&mut job, library)?;
        let end_mill = self.end_mill.as_ref()?.take(&mut job, library)?;
        let mut request = SignRequest::new(&self.words, vbit, end_mill);
        request.around.clone_from(&self.around);
        request.font.clone_from(&self.font);
        request.height = self.height;
        request.kind = self.kind;
        request.corner = self.corner_set.then_some(self.corner);
        request.margin = self.margin;
        request.borders = self.borders;
        request.gap = self.gap;
        request.border_corners = self
            .border_corners
            .filter(|_| self.kind.corner_style().is_some());
        request.double = self.double;
        request.round = self.round && self.kind == FrameKind::Oval && self.around.is_empty();
        let size = match request.round {
            true => (self.size.0, self.size.0),
            false => self.size,
        };
        request.size = self.fixed.then_some(size);
        request.border_width = self.border_width;
        request.style = self.style;
        request.relief = self.relief;
        request.engrave_depth = self.engrave_depth;
        request.edge = self.edge.then_some(self.edge_width);
        request.board = self.fit_board.then_some(SignBoard {
            thickness: self.thickness,
            margin: self.board_margin,
        });
        Some((job, request))
    }
}

/// What the dialog makes: one sign, or every sign of a list.
struct Made {
    job: chipload_job::Job,
    operations: Vec<usize>,
    fitted: Option<f64>,
    /// How many signs.
    count: usize,
    /// The new shapes to select afterwards.
    shapes: Vec<u64>,
}

impl SignSettings {
    /// The sign, or the list's signs, in `job` with `request`. For the preview, a list's
    /// signs are only drawn: making their operations takes long, and the picture shows
    /// the outlines.
    fn make(
        &self,
        job: &chipload_job::Job,
        request: SignRequest,
        fonts: &dyn chipload_pipeline::fonts::FontSource,
        library: Option<&ToolLibrary>,
        lang: crate::i18n::Language,
        preview: bool,
    ) -> Result<Made, chipload_pipeline::PipelineError> {
        let before: std::collections::BTreeSet<u64> =
            job.drawing.shapes.iter().map(|s| s.id).collect();
        let new_shapes = |done: &chipload_job::Job| -> Vec<u64> {
            done.drawing
                .shapes
                .iter()
                .filter(|s| !before.contains(&s.id))
                .map(|s| s.id)
                .collect()
        };
        if self.from_list && self.around.is_empty() {
            use chipload_pipeline::sign_batch::{
                BatchRequest, DEFAULT_BATCH_GAP_MM, add_sign_batch, draw_sign_batch,
            };
            let batch = BatchRequest {
                sign: request,
                list: self.list.lines().map(str::to_string).collect(),
                gap: DEFAULT_BATCH_GAP_MM,
            };
            let (done, made) = if preview {
                draw_sign_batch(job, &batch, fonts, lang)?
            } else {
                add_sign_batch(job, &batch, fonts, library, lang)?
            };
            return Ok(Made {
                shapes: new_shapes(&done),
                job: done,
                operations: made.operations,
                fitted: made.fitted,
                count: made.outlines.len(),
            });
        }
        let (done, made) = add_sign(job, &request, fonts, library, lang)?;
        let mut shapes = new_shapes(&done);
        shapes.extend(&self.around);
        Ok(Made {
            shapes,
            job: done,
            operations: made.operations,
            fitted: made.fitted,
            count: 1,
        })
    }
}

/// The sign the dialog shows, kept until its values or the project change.
#[derive(Debug, Clone)]
pub(super) struct SignPreview {
    key: String,
    /// The new shapes' outlines, drawing coordinates.
    lines: Vec<Vec<Point>>,
    /// The operations it adds, by name; or why it can't be made.
    adds: Result<Vec<String>, String>,
    /// The letters' height found for a sign of a fixed size, mm.
    fitted: Option<f64>,
    /// How many signs it makes.
    count: usize,
}

/// The sign being drawn for the dialog on a thread of its own, for the values `key`
/// names: typing never waits for the letters.
pub(super) struct SignWork {
    key: String,
    worker: super::worker::Worker<SignPreview>,
}

impl Default for SignPreview {
    fn default() -> Self {
        Self {
            key: String::new(),
            lines: Vec::new(),
            adds: Ok(Vec::new()),
            fitted: None,
            count: 0,
        }
    }
}

fn style_name(ui: &egui::Ui, style: SignStyle) -> String {
    match style {
        SignStyle::Engraved => t!(ui, "sign-style-engraved"),
        SignStyle::Raised => t!(ui, "sign-style-raised"),
        SignStyle::Inlay => t!(ui, "sign-style-inlay"),
    }
}

impl ChiploadApp {
    /// *New sign…* on the start page (docs/specs/sign-size-and-start.md §2): a new project
    /// with the choices last made (the library's defaults when there are none), without the
    /// New project dialog since the sign fits the board, and the sign's dialog on it.
    /// Cancelled, the empty project stays, as *New project* leaves it.
    pub fn new_sign_project(&mut self) {
        if self.guard(Pending::NewSign) {
            self.new_sign_project_now();
        }
    }

    /// *New sign…* once unsaved changes are saved or let go of.
    pub(super) fn new_sign_project_now(&mut self) {
        let start = self.remembered_start();
        let first = t!(self, "app-new-project");
        self.create_project(chipload_pipeline::empty_project(), &start, first, None);
        if self.edit.is_some() {
            self.start_sign();
        }
    }

    /// *New sign…*: the dialog; round the selection when there is one.
    pub fn start_sign(&mut self) {
        let around = self.framed_shape_ids();
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let library = self.library().map(|l| &l.tools);
        let mut settings = SignSettings::for_job(&edit.job, library);
        settings.around = around;
        // The font last picked, the text tool's list's first (docs/specs/font-list.md).
        if let Some(font) = self
            .text_fonts
            .recent
            .iter()
            .find(|f| self.text_fonts.has(f) != Some(false))
        {
            settings.font.clone_from(font);
        }
        self.canvas_tools.sign_preview = SignPreview::default();
        self.canvas_tools.sign_work = None;
        self.dialogs.prompt = Some(Prompt::Sign(Box::new(settings)));
    }

    /// The sign the settings make, for the preview: its shapes and its operations' names.
    /// The sign the settings make, for the preview: its shapes and its operations' names.
    /// Drawn on a thread of its own: until it is ready, the last picture stays, and a new
    /// one is started for the values as they are then.
    fn sign_preview(&mut self, ctx: &egui::Context, settings: &SignSettings) -> SignPreview {
        if self.edit.is_none() {
            return SignPreview::default();
        }
        // The dialog's values alone: the project can't change while it is open, and the
        // preview starts afresh when it opens (`start_sign`). Writing out the drawing at
        // every frame slowed typing in a large project.
        let key = format!("{settings:?}");
        if let Some(work) = &self.canvas_tools.sign_work {
            #[cfg(test)]
            let done = Some(work.worker.wait());
            #[cfg(not(test))]
            let done = work.worker.poll();
            match done {
                Some(Ok(preview)) => {
                    self.canvas_tools.sign_preview = preview;
                    self.canvas_tools.sign_work = None;
                }
                Some(Err(stopped)) => {
                    let why = stopped.text_or(t!(self, "sign-refused"));
                    self.canvas_tools.sign_preview = SignPreview {
                        key: work.key.clone(),
                        adds: Err(why),
                        ..SignPreview::default()
                    };
                    self.canvas_tools.sign_work = None;
                }
                None => ctx.request_repaint_after(std::time::Duration::from_millis(16)),
            }
        }
        if self.canvas_tools.sign_preview.key != key && self.canvas_tools.sign_work.is_none() {
            self.start_sign_preview(ctx, settings, key);
        }
        self.canvas_tools.sign_preview.clone()
    }

    /// Start drawing the sign for `settings` (named by `key`) on its own thread.
    fn start_sign_preview(&mut self, ctx: &egui::Context, settings: &SignSettings, key: String) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let job = edit.job.clone();
        let settings = settings.clone();
        let lang = self.prefs.language;
        let units = self.prefs.units.into();
        let library = self.library().map(|l| l.tools.clone());
        let fonts = self.font_source();
        let no_tools = t!(self, "sign-no-tools");
        let job_key = key.clone();
        let started = super::worker::Worker::start("sign preview", move |_| {
            let made = match settings.request(&job, library.as_ref()) {
                None => Err(no_tools),
                Some((job, request)) => settings
                    .make(&job, request, &*fonts, library.as_ref(), lang, true)
                    .map_err(|e| e.message().render(lang, units)),
            };
            match made {
                Ok(made) => SignPreview {
                    key: job_key,
                    lines: made
                        .job
                        .drawing
                        .shapes
                        .iter()
                        .filter(|s| made.shapes.contains(&s.id))
                        .flat_map(|s| crate::draw::outline(&s.geometry))
                        .collect(),
                    adds: Ok(made
                        .operations
                        .iter()
                        .map(|&i| made.job.operations[i].name().to_string())
                        .collect()),
                    fitted: made.fitted,
                    count: made.count,
                },
                Err(why) => SignPreview {
                    key: job_key,
                    adds: Err(why),
                    ..SignPreview::default()
                },
            }
        });
        match started {
            Ok(worker) => {
                self.canvas_tools.sign_work = Some(SignWork { key, worker });
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
            Err(err) => {
                self.canvas_tools.sign_preview = SignPreview {
                    key,
                    adds: Err(err.to_string()),
                    ..SignPreview::default()
                };
            }
        }
    }

    /// Make the sign, as one undo step; its part becomes the active one and the sign is
    /// selected. False, with the reason in the report, when it is refused.
    pub(super) fn make_sign(&mut self, settings: &SignSettings) -> bool {
        self.commit();
        self.reload_library();
        let library = self.library().map(|l| l.tools.clone());
        let lang = self.prefs.language;
        let fonts = self.font_source();
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let Some((job, request)) = settings.request(&edit.job, library.as_ref()) else {
            self.error(vec![t!(self, "sign-no-tools")]);
            return false;
        };
        match settings.make(&job, request, &*fonts, library.as_ref(), lang, false) {
            Ok(made) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(made.job);
                }
                self.drawing_changed(&made.shapes);
                self.fit_pending = true;
                self.commit();
                let text = if made.count > 1 {
                    t!(
                        self,
                        "sign-list-made",
                        count = made.count,
                        operations = made.operations.len()
                    )
                } else {
                    t!(self, "sign-made", count = made.operations.len())
                };
                self.set_report(Report::info([text]));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(t!(self, "sign-refused"), err.message()));
                false
            }
        }
    }

    /// The dialog: the values in groups, the sign drawn as it will be, what it adds,
    /// *Make sign*.
    pub(super) fn sign_prompt(&mut self, ctx: &egui::Context, mut settings: Box<SignSettings>) {
        let tools: Vec<Tool> = self
            .edit
            .as_ref()
            .map(|e| e.job.tools.clone())
            .unwrap_or_default();
        let units = self.prefs.units;
        let library = self.library().map(|l| l.tools.clone());
        let preview = self.sign_preview(ctx, &settings);
        let mut answer: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::new("new-sign")).show(ctx, |ui| {
            ui.set_width(560.0);
            ui.label(crate::style::bold(
                ui,
                egui::RichText::new(t!(ui, "sign-title")).heading(),
            ));
            ui.add_space(4.0);
            ui.label(t!(ui, "sign-body"));
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .max_height(ui.ctx().content_rect().height() * 0.6)
                .show(ui, |ui| {
                    let offer = ToolOffer {
                        tools: &tools,
                        library: library.as_ref(),
                        units,
                    };
                    sign_fields(ui, &mut settings, &offer, &self.text_fonts, preview.fitted);
                });
            ui.add_space(6.0);
            sign_picture(ui, &preview.lines);
            // A font whose letters join takes a moment: say the picture is coming.
            if self.canvas_tools.sign_work.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak(t!(ui, "sign-drawing"));
                });
            }
            match &preview.adds {
                Ok(names) => {
                    if preview.count > 1 {
                        ui.weak(t!(ui, "sign-list-count", count = preview.count));
                    }
                    // A list's preview draws the signs only (`SignSettings::make`).
                    if !names.is_empty() {
                        ui.weak(t!(ui, "sign-adds", operations = names.join(", ")));
                    }
                }
                Err(why) => {
                    ui.colored_label(ui.visuals().warn_fg_color, why);
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ready = preview.adds.is_ok();
                if ui
                    .add_enabled(ready, egui::Button::new(t!(ui, "sign-make")))
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui.button(t!(ui, "common-cancel")).clicked() {
                    answer = Some(false);
                }
            });
        });
        if modal.should_close() && answer.is_none() {
            answer = Some(false);
        }
        match answer {
            Some(true) => {
                self.dialogs.prompt = None;
                self.make_sign(&settings);
            }
            Some(false) => self.dialogs.prompt = None,
            None => self.dialogs.prompt = Some(Prompt::Sign(settings)),
        }
    }
}

/// The dialog's values: words, frame, style and edge, tools, board.
fn sign_fields(
    ui: &mut egui::Ui,
    s: &mut SignSettings,
    offer: &ToolOffer<'_>,
    fonts: &super::text_fonts::TextFonts,
    fitted: Option<f64>,
) {
    if s.around.is_empty() {
        word_fields(ui, s, fonts, fitted);
    } else {
        ui.label(crate::style::bold(ui, t!(ui, "sign-around")));
        ui.label(t!(ui, "sign-around-body", count = s.around.len()));
        if ui
            .button(t!(ui, "sign-around-words"))
            .on_hover_text(t!(ui, "sign-around-words-hover"))
            .clicked()
        {
            s.around.clear();
        }
    }
    frame_and_cut_fields(ui, s, offer);
}

/// The words, their font (the text tool's font list: recently used, built-in, the
/// computer's), and their height or the sign's fixed size (with the height found).
fn word_fields(
    ui: &mut egui::Ui,
    s: &mut SignSettings,
    fonts: &super::text_fonts::TextFonts,
    fitted: Option<f64>,
) {
    ui.horizontal(|ui| {
        ui.label(crate::style::bold(ui, t!(ui, "sign-signs")));
        ui.selectable_value(&mut s.from_list, false, t!(ui, "sign-signs-one"));
        let was = s.from_list;
        ui.selectable_value(&mut s.from_list, true, t!(ui, "sign-signs-list"))
            .on_hover_text(t!(ui, "sign-signs-list-hover"));
        // A list starts on a fixed size, so the signs match (docs/specs/batch-signs.md §5).
        if s.from_list && !was {
            s.fixed = true;
        }
    });
    if s.from_list {
        ui.add(
            egui::TextEdit::multiline(&mut s.list)
                .desired_rows(5)
                .desired_width(f32::INFINITY)
                .hint_text(t!(ui, "sign-list-hint")),
        );
        // Shown while typing, not only in the empty box.
        ui.weak(t!(ui, "sign-list-help"));
    } else {
        ui.label(crate::style::bold(ui, t!(ui, "sign-words")));
        ui.add(
            egui::TextEdit::multiline(&mut s.words)
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .hint_text(t!(ui, "sign-words-hint")),
        );
    }
    egui::Grid::new("sign-words-grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "sign-font"));
            let mut shown = egui::RichText::new(s.font.as_str());
            let family = if chipload_text::builtin(&s.font).is_some() {
                crate::fonts::text_font_family(&s.font)
            } else {
                super::text_fonts::sample_family(&s.font)
            };
            if ui.ctx().fonts(|f| f.families()).contains(&family) {
                shown = shown.family(family);
            }
            egui::ComboBox::from_id_salt("sign-font")
                .selected_text(shown)
                .height(480.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show_ui(ui, |ui| super::forms::font_list(ui, &mut s.font, fonts));
            ui.end_row();
            ui.label(t!(ui, "sign-size"))
                .on_hover_text(t!(ui, "sign-size-hover"));
            ui.horizontal(|ui| {
                ui.selectable_value(&mut s.fixed, false, t!(ui, "sign-size-fit"));
                ui.selectable_value(&mut s.fixed, true, t!(ui, "sign-size-fixed"));
            });
            ui.end_row();
            if !s.fixed {
                ui.label(t!(ui, "sign-height"));
                ui.add(crate::units::length(&mut s.height).range(1.0..=1000.0));
                ui.end_row();
                return;
            }
            if s.round && s.kind == FrameKind::Oval {
                ui.label(t!(ui, "sign-diameter"));
                ui.add(crate::units::length(&mut s.size.0).range(10.0..=5000.0));
            } else {
                ui.label(t!(ui, "sign-outer-size"));
                ui.horizontal(|ui| {
                    ui.add(
                        crate::units::length(&mut s.size.0)
                            .range(10.0..=5000.0)
                            .prefix("W "),
                    );
                    ui.add(
                        crate::units::length(&mut s.size.1)
                            .range(10.0..=5000.0)
                            .prefix("H "),
                    );
                });
            }
            ui.end_row();
            ui.label(t!(ui, "sign-letters"));
            match fitted {
                Some(h) => ui.weak(crate::units::text(ui.ctx(), h)),
                None => ui.weak("–"),
            };
            ui.end_row();
        });
}

/// The frame, the style, the edge, the tools and the board.
fn frame_and_cut_fields(ui: &mut egui::Ui, s: &mut SignSettings, offer: &ToolOffer<'_>) {
    ui.separator();
    ui.label(crate::style::bold(ui, t!(ui, "frame-title")));
    super::frames::kind_choice(ui, "sign", &mut s.kind);
    egui::Grid::new("sign-frame-grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "frame-margin"));
            ui.add(crate::units::length(&mut s.margin))
                .on_hover_text(t!(ui, "frame-margin-hover"));
            ui.end_row();
            // The round frame can take its words round its ring
            // (docs/specs/text-on-arc.md §6).
            if s.kind == FrameKind::Oval && s.around.is_empty() {
                ui.label(t!(ui, "sign-round"))
                    .on_hover_text(t!(ui, "sign-round-hover"));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut s.round, false, t!(ui, "sign-round-straight"));
                    ui.selectable_value(&mut s.round, true, t!(ui, "sign-round-round"));
                });
                ui.end_row();
            }
            if s.kind.uses_r() {
                ui.label(t!(ui, "frame-corner"));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.corner_set, "")
                        .on_hover_text(t!(ui, "frame-corner-auto-hover"));
                    if s.corner_set {
                        ui.add(crate::units::length(&mut s.corner));
                    }
                });
                ui.end_row();
            }
            ui.label(t!(ui, "frame-borders"));
            ui.add(egui::DragValue::new(&mut s.borders).range(0..=MOST_BORDERS));
            ui.end_row();
            super::frames::border_rows(
                ui,
                "sign",
                super::frames::BorderValues {
                    kind: s.kind,
                    borders: s.borders,
                    gap: &mut s.gap,
                    corners: &mut s.border_corners,
                    double: &mut s.double,
                    width: &mut s.border_width,
                },
            );
        });
    ui.separator();
    egui::Grid::new("sign-cut-grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "sign-style"));
            ui.horizontal(|ui| {
                for style in SignStyle::ALL {
                    ui.selectable_value(&mut s.style, style, style_name(ui, style));
                }
            })
            .response
            .on_hover_text(t!(ui, "sign-style-hover"));
            ui.end_row();
            let single_line = s.around.is_empty()
                && chipload_text::builtin(&s.font)
                    .is_some_and(chipload_text::BuiltinFont::single_line);
            if single_line && s.style == SignStyle::Engraved {
                ui.label(t!(ui, "sign-engrave-depth"));
                ui.add(crate::units::length(&mut s.engrave_depth).range(0.05..=20.0))
                    .on_hover_text(t!(ui, "sign-engrave-depth-hover"));
                ui.end_row();
            }
            if s.style == SignStyle::Raised {
                ui.label(t!(ui, "sign-relief"));
                ui.add(crate::units::length(&mut s.relief).range(0.1..=100.0))
                    .on_hover_text(t!(ui, "sign-relief-hover"));
                ui.end_row();
            }
            ui.label(t!(ui, "sign-edge"));
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.edge, t!(ui, "sign-edge-chamfer"));
                if s.edge {
                    ui.add(crate::units::length(&mut s.edge_width).range(0.1..=50.0));
                }
            })
            .response
            .on_hover_text(t!(ui, "sign-edge-hover"));
            ui.end_row();
            ui.label(t!(ui, "sign-vbit"));
            tool_combo(ui, "sign-vbit", &mut s.vbit, offer, ToolKind::Vbit, None);
            ui.end_row();
            ui.label(t!(ui, "sign-end-mill"));
            tool_combo(
                ui,
                "sign-end-mill",
                &mut s.end_mill,
                offer,
                ToolKind::EndMill,
                None,
            );
            ui.end_row();
            ui.label(t!(ui, "sign-board"));
            ui.checkbox(&mut s.fit_board, t!(ui, "sign-fit-board"))
                .on_hover_text(t!(ui, "sign-fit-board-hover"));
            ui.end_row();
            if s.fit_board {
                ui.label(t!(ui, "sign-thickness"));
                ui.add(crate::units::length(&mut s.thickness).range(1.0..=500.0));
                ui.end_row();
                ui.label(t!(ui, "sign-board-margin"));
                ui.add(crate::units::length(&mut s.board_margin).range(0.0..=500.0));
                ui.end_row();
            }
        });
}

/// The sign's new shapes, scaled to fit a strip of the dialog.
fn sign_picture(ui: &mut egui::Ui, lines: &[Vec<Point>]) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 150.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let points = lines.iter().flatten();
    let Some(((x0, y0), (x1, y1))) = points.fold(None, |acc: Option<(Point, Point)>, &(x, y)| {
        Some(match acc {
            None => ((x, y), (x, y)),
            Some(((a, b), (c, d))) => ((a.min(x), b.min(y)), (c.max(x), d.max(y))),
        })
    }) else {
        return;
    };
    let inner = rect.shrink(10.0);
    let scale = (f64::from(inner.width()) / (x1 - x0).max(1e-3))
        .min(f64::from(inner.height()) / (y1 - y0).max(1e-3));
    let middle = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let to_screen = |(x, y): Point| {
        inner.center()
            + egui::vec2(
                ((x - middle.0) * scale) as f32,
                (-(y - middle.1) * scale) as f32,
            )
    };
    let stroke = egui::Stroke::new(1.2, ui.visuals().text_color());
    for line in lines {
        let points: Vec<egui::Pos2> = line.iter().map(|&p| to_screen(p)).collect();
        painter.add(egui::Shape::line(points, stroke));
    }
}
