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
    pub kind: FrameKind,
    pub corner_set: bool,
    pub corner: f64,
    pub margin: f64,
    pub borders: u32,
    pub gap: f64,
    pub border_corners: Option<CornerStyle>,
    pub double: bool,
    pub border_width: f64,
    pub style: SignStyle,
    pub relief: f64,
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
            .find(|t| ToolKind::Vbit.fits(t.kind, t.angle))
            .map(|t| WizardTool::Project(t.id))
            .or_else(|| from_library(ToolKind::Vbit));
        let mill = job
            .tools
            .iter()
            .filter(|t| ToolKind::EndMill.fits(t.kind, t.angle))
            .max_by(|a, b| a.diameter.total_cmp(&b.diameter))
            .map(|t| WizardTool::Project(t.id))
            .or_else(|| from_library(ToolKind::EndMill));
        Self {
            around: Vec::new(),
            words: String::new(),
            font: chipload_pipeline::texts::DEFAULT_FONT.to_string(),
            height: 30.0,
            kind: FrameKind::Rounded,
            corner_set: false,
            corner: 8.0,
            margin: 10.0,
            borders: 1,
            gap: 5.0,
            border_corners: None,
            double: false,
            border_width: DEFAULT_BORDER_WIDTH_MM,
            style: SignStyle::Engraved,
            relief: DEFAULT_RELIEF_MM,
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
        request.border_width = self.border_width;
        request.style = self.style;
        request.relief = self.relief;
        request.edge = self.edge.then_some(self.edge_width);
        request.board = self.fit_board.then_some(SignBoard {
            thickness: self.thickness,
            margin: self.board_margin,
        });
        Some((job, request))
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
}

impl Default for SignPreview {
    fn default() -> Self {
        Self {
            key: String::new(),
            lines: Vec::new(),
            adds: Ok(Vec::new()),
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
        self.dialogs.prompt = Some(Prompt::Sign(Box::new(settings)));
    }

    /// The sign the settings make, for the preview: its shapes and its operations' names.
    fn sign_preview(&mut self, settings: &SignSettings) -> SignPreview {
        let Some(edit) = self.edit.as_ref() else {
            return SignPreview::default();
        };
        let key = format!(
            "{settings:?}{:?}{:?}{:?}",
            edit.job.drawing,
            edit.job.tools,
            self.library().map(|l| &l.tools)
        );
        if self.canvas_tools.sign_preview.key == key {
            return self.canvas_tools.sign_preview.clone();
        }
        let lang = self.prefs.language;
        let units = self.prefs.units.into();
        let library = self.library().map(|l| l.tools.clone());
        let fonts = self.font_source();
        let before: std::collections::BTreeSet<u64> =
            edit.job.drawing.shapes.iter().map(|s| s.id).collect();
        let made = match settings.request(&edit.job, library.as_ref()) {
            None => Err(t!(self, "sign-no-tools")),
            Some((job, request)) => add_sign(&job, &request, &*fonts, library.as_ref(), lang)
                .map_err(|e| e.message().render(lang, units)),
        };
        let preview = match made {
            Ok((job, sign)) => SignPreview {
                key,
                lines: job
                    .drawing
                    .shapes
                    .iter()
                    .filter(|s| !before.contains(&s.id) || settings.around.contains(&s.id))
                    .flat_map(|s| crate::draw::outline(&s.geometry))
                    .collect(),
                adds: Ok(sign
                    .operations
                    .iter()
                    .map(|&i| job.operations[i].name().to_string())
                    .collect()),
            },
            Err(why) => SignPreview {
                key,
                lines: Vec::new(),
                adds: Err(why),
            },
        };
        self.canvas_tools.sign_preview = preview.clone();
        preview
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
        match add_sign(&job, &request, &*fonts, library.as_ref(), lang) {
            Ok((job, made)) => {
                let mut ids: Vec<u64> = job
                    .drawing
                    .shapes
                    .iter()
                    .filter(|s| made.text.is_some() && s.text() == made.text)
                    .map(|s| s.id)
                    .collect();
                ids.extend(&settings.around);
                ids.push(made.outline);
                ids.extend(&made.borders);
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(job);
                }
                self.drawing_changed(&ids);
                self.fit_pending = true;
                self.commit();
                self.set_report(Report::info([t!(
                    self,
                    "sign-made",
                    count = made.operations.len()
                )]));
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
        let preview = self.sign_preview(&settings);
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
                    sign_fields(ui, &mut settings, &offer, &self.text_fonts);
                });
            ui.add_space(6.0);
            sign_picture(ui, &preview.lines);
            match &preview.adds {
                Ok(names) => {
                    ui.weak(t!(ui, "sign-adds", operations = names.join(", ")));
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
) {
    if s.around.is_empty() {
        word_fields(ui, s, fonts);
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
/// computer's) and height.
fn word_fields(ui: &mut egui::Ui, s: &mut SignSettings, fonts: &super::text_fonts::TextFonts) {
    ui.label(crate::style::bold(ui, t!(ui, "sign-words")));
    ui.add(
        egui::TextEdit::multiline(&mut s.words)
            .desired_rows(2)
            .desired_width(f32::INFINITY)
            .hint_text(t!(ui, "sign-words-hint")),
    );
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
            ui.label(t!(ui, "sign-height"));
            ui.add(crate::units::length(&mut s.height).range(1.0..=1000.0));
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
            tool_combo(ui, "sign-vbit", &mut s.vbit, offer, ToolKind::Vbit, false);
            ui.end_row();
            ui.label(t!(ui, "sign-end-mill"));
            tool_combo(
                ui,
                "sign-end-mill",
                &mut s.end_mill,
                offer,
                ToolKind::EndMill,
                false,
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
