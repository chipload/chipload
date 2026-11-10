// SPDX-License-Identifier: GPL-3.0-or-later
//! *Two-sided…* (docs/specs/two-sided-wizard.md §4): a back side with its dowel pins in one
//! dialog, the board drawn with where both sides cut, the turn axis and the pins. What it
//! makes is `chipload_pipeline::two_sided`'s.

use chipload_core::Point;
use chipload_job::library::ToolLibrary;
use chipload_job::{Job, Register, Turn};
use chipload_pipeline::dowels::{DEFAULT_PIN_DEPTH, DEFAULT_PIN_DIAMETER, DowelLayout, pin_tool};
use chipload_pipeline::two_sided::{
    PinsRequest, TwoSidedRequest, make_two_sided, two_sided_layout,
};

use super::wizard_tools::{ToolKind, ToolOffer, WizardTool, tool_combo};
use super::*;

/// What the dialog holds while it is open.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TwoSidedSettings {
    pub front: u64,
    pub turn: Turn,
    pub register: Register,
    pub diameter: f64,
    pub depth: f64,
    /// The tool that cuts the holes; None: chosen for them.
    pub tool: Option<WizardTool>,
    pub zero: bool,
    /// The front's fit margin widened to this, when asked (decision 5).
    pub margin: Option<f64>,
}

impl TwoSidedSettings {
    /// The defaults (§4): the back's values when there is one, else left to right by pins;
    /// its dowel holes' size, depth and tool when it has some, else 8 mm pins 8 mm deep cut
    /// with a drill of their size or the largest end mill that bores them; the work zero at
    /// the first pin (decision 8).
    pub(super) fn for_front(job: &Job, front: u64) -> Self {
        let link = job.back_of(front).and_then(|s| s.back_of);
        let holes = job.dowels_of(front).and_then(|i| job.operations.get(i));
        let diameter = holes
            .and_then(|op| {
                let select = op.select().resolved(&job.drawing);
                job.drawing.shapes.iter().find_map(|s| match s.geometry {
                    chipload_job::Geometry::Circle { radius, .. } if select.may_pick(s) => {
                        Some(2.0 * radius)
                    }
                    _ => None,
                })
            })
            .unwrap_or(DEFAULT_PIN_DIAMETER);
        let depth = holes
            .and_then(|op| match op {
                chipload_job::Operation::Holes(h) => h.through,
                _ => None,
            })
            .unwrap_or(DEFAULT_PIN_DEPTH);
        let tool = holes
            .map(chipload_job::Operation::tool)
            .or_else(|| pin_tool(job, diameter))
            .map(WizardTool::Project);
        Self {
            front,
            turn: link.map_or(Turn::LeftRight, |b| b.turn),
            register: link.map_or(Register::Pins, |b| b.register),
            diameter,
            depth,
            tool,
            zero: true,
            margin: None,
        }
    }

    /// The project with a library tool chosen copied in, and the pipeline's request; None
    /// when the library tool is gone.
    fn request(&self, job: &Job, library: Option<&ToolLibrary>) -> Option<(Job, TwoSidedRequest)> {
        let mut job = job.clone();
        let tool = match &self.tool {
            Some(tool) => Some(tool.take(&mut job, library)?),
            None => None,
        };
        let pins = (self.register == Register::Pins).then_some(PinsRequest {
            diameter: self.diameter,
            depth: self.depth,
            zero: self.zero,
            tool,
        });
        let request = TwoSidedRequest {
            front: self.front,
            turn: self.turn,
            register: self.register,
            pins,
            margin: self.margin,
        };
        Some((job, request))
    }
}

/// What the dialog draws, kept until its values or the project change.
#[derive(Debug, Clone, Default)]
pub(super) struct TwoSidedPreview {
    key: String,
    /// The pins' layout, or why there is none.
    layout: Option<Result<DowelLayout, String>>,
    /// The drawing's outlines, faint under the rest.
    lines: Vec<Vec<Point>>,
}

impl ChiploadApp {
    /// *Two-sided…* for the active setup (its front, when it is a back).
    pub fn start_two_sided(&mut self) {
        let Some(job) = self.edit.as_ref().map(|e| &e.job) else {
            return;
        };
        let Some(active) = job.active_setup_id() else {
            return;
        };
        let front = job.front_of(active).map_or(active, |s| s.id);
        self.start_two_sided_for(front);
    }

    /// *Two-sided…* for setup `front`: its back made, or changed when it has one.
    pub fn start_two_sided_for(&mut self, front: u64) {
        let Some(job) = self.edit.as_ref().map(|e| &e.job) else {
            return;
        };
        if job.setup(front).is_none_or(|s| s.back_of.is_some()) {
            return;
        }
        let settings = TwoSidedSettings::for_front(job, front);
        self.dialogs.prompt = Some(Prompt::TwoSided(Box::new(settings)));
    }

    fn two_sided_preview(&mut self, settings: &TwoSidedSettings) -> TwoSidedPreview {
        let Some(edit) = self.edit.as_ref() else {
            return TwoSidedPreview::default();
        };
        let library = self.library().map(|l| l.tools.clone());
        let key = format!(
            "{settings:?}{:?}{:?}{:?}{:?}",
            edit.job.drawing, edit.job.operations, edit.job.setups, edit.job.tools
        );
        if self.canvas_tools.two_sided_preview.key == key {
            return self.canvas_tools.two_sided_preview.clone();
        }
        let lang = self.prefs.language;
        let units = self.prefs.units.into();
        let layout = match settings.request(&edit.job, library.as_ref()) {
            None => Err(t!(self, "wizard-library-tool-gone")),
            Some((job, request)) => {
                two_sided_layout(&job, &request).map_err(|e| e.message().render(lang, units))
            }
        };
        let lines = edit
            .job
            .drawing
            .shapes
            .iter()
            .filter(|s| !s.construction)
            .flat_map(|s| crate::draw::outline(&s.geometry))
            .collect();
        let preview = TwoSidedPreview {
            key,
            layout: Some(layout),
            lines,
        };
        self.canvas_tools.two_sided_preview = preview.clone();
        preview
    }

    /// Make the back side, or change it, as one undo step; the back becomes the active
    /// setup, its form open. False, with the reason in the report, when it is refused.
    pub(super) fn make_two_sided(&mut self, settings: &TwoSidedSettings) -> bool {
        self.commit();
        self.reload_library();
        let library = self.library().map(|l| l.tools.clone());
        let lang = self.prefs.language;
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let Some((job, request)) = settings.request(&edit.job, library.as_ref()) else {
            self.error(vec![t!(self, "wizard-library-tool-gone")]);
            return false;
        };
        match make_two_sided(&job, &request, lang) {
            Ok((job, made)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(job);
                }
                self.commit();
                self.refresh_drawing();
                self.focus = Focus::Setup(made.back);
                self.display.turned = true;
                let name = self
                    .edit
                    .as_ref()
                    .and_then(|e| e.job.setup(made.back))
                    .map_or(String::new(), |s| s.name.clone());
                let line = match (made.added, made.dowels) {
                    (true, Some(i)) => t!(
                        self,
                        "two-sided-made-pins",
                        back = name.as_str(),
                        number = i + 1
                    ),
                    (true, None) => t!(self, "two-sided-made", back = name.as_str()),
                    (false, _) => t!(self, "two-sided-changed", back = name.as_str()),
                };
                self.set_report(Report::info([line]));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(t!(self, "two-sided-refused"), err.message()));
                false
            }
        }
    }

    /// The dialog: the front, how it turns and is put back, the pins, the picture, *Make*.
    pub(super) fn two_sided_prompt(
        &mut self,
        ctx: &egui::Context,
        mut settings: Box<TwoSidedSettings>,
    ) {
        let Some(job) = self.edit.as_ref().map(|e| e.job.clone()) else {
            self.dialogs.prompt = None;
            return;
        };
        let units = self.prefs.units;
        let library = self.library().map(|l| l.tools.clone());
        let preview = self.two_sided_preview(&settings);
        let changing = job.back_of(settings.front).is_some();
        let mut answer: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::new("two-sided")).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.label(crate::style::bold(
                ui,
                egui::RichText::new(t!(ui, "two-sided-title")).heading(),
            ));
            ui.add_space(4.0);
            ui.label(t!(ui, "two-sided-body"));
            ui.add_space(8.0);
            let offer = ToolOffer {
                tools: &job.tools,
                library: library.as_ref(),
                units,
            };
            two_sided_fields(ui, &mut settings, &job, &offer);
            ui.add_space(8.0);
            let layout = preview.layout.as_ref();
            let pins = settings.register == Register::Pins;
            match layout {
                Some(Ok(layout)) => {
                    two_sided_picture(ui, layout, &preview.lines, pins, settings.zero);
                    if pins && !layout.fits() {
                        no_room_row(ui, &mut settings, layout);
                    }
                }
                Some(Err(why)) => {
                    ui.colored_label(ui.visuals().warn_fg_color, why);
                }
                None => {}
            }
            if let Some(margin) = settings.margin {
                let old = job
                    .setup(settings.front)
                    .and_then(|s| s.stock.fit)
                    .unwrap_or(0.0);
                let grows = 2.0 * (margin - old);
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    t!(
                        ui,
                        "two-sided-board-grows",
                        grows = crate::units::text(ui.ctx(), grows).as_str()
                    ),
                );
            }
            ui.add_space(6.0);
            let ready = match layout {
                Some(Ok(layout)) => !pins || layout.fits(),
                _ => false,
            };
            ui.horizontal(|ui| {
                let make = if changing {
                    t!(ui, "two-sided-change")
                } else {
                    t!(ui, "two-sided-make")
                };
                if ui.add_enabled(ready, egui::Button::new(make)).clicked() {
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
                self.make_two_sided(&settings);
            }
            Some(false) => self.dialogs.prompt = None,
            None => self.dialogs.prompt = Some(Prompt::TwoSided(settings)),
        }
    }
}

/// The dialog's rows: the front, the turn, how it is put back, then the pins.
fn two_sided_fields(ui: &mut egui::Ui, s: &mut TwoSidedSettings, job: &Job, offer: &ToolOffer<'_>) {
    egui::Grid::new("two-sided-grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "two-sided-front"))
                .on_hover_text(t!(ui, "two-sided-front-hover"));
            let name_of = |id: u64| job.setup(id).map_or(String::new(), |s| s.name.clone());
            let before = s.front;
            egui::ComboBox::from_id_salt("two-sided-front")
                .selected_text(name_of(s.front))
                .show_ui(ui, |ui| {
                    for setup in job.setups.iter().filter(|s| s.back_of.is_none()) {
                        ui.selectable_value(&mut s.front, setup.id, setup.name.as_str());
                    }
                });
            if s.front != before {
                *s = TwoSidedSettings::for_front(job, s.front);
            }
            ui.end_row();
            ui.label(t!(ui, "forms-setup-turn"))
                .on_hover_text(t!(ui, "forms-setup-turn-hover"));
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut s.turn,
                    Turn::LeftRight,
                    t!(ui, "forms-setup-turn-left-right"),
                );
                ui.selectable_value(
                    &mut s.turn,
                    Turn::TopBottom,
                    t!(ui, "forms-setup-turn-top-bottom"),
                );
            });
            ui.end_row();
            ui.label(t!(ui, "forms-setup-register"))
                .on_hover_text(t!(ui, "forms-setup-register-hover"));
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut s.register,
                    Register::Pins,
                    t!(ui, "forms-setup-register-pins"),
                );
                ui.selectable_value(
                    &mut s.register,
                    Register::Corner,
                    t!(ui, "forms-setup-register-corner"),
                );
            });
            ui.end_row();
            if s.register != Register::Pins {
                return;
            }
            ui.label(t!(ui, "two-sided-diameter"))
                .on_hover_text(t!(ui, "two-sided-diameter-hover"));
            let before = s.diameter;
            ui.add(crate::units::length(&mut s.diameter).range(1.0..=50.0));
            // A tool chosen for the old size is chosen again for the new one.
            if s.diameter != before
                && let Some(WizardTool::Project(_)) | None = s.tool
            {
                s.tool = pin_tool(job, s.diameter).map(WizardTool::Project);
            }
            ui.end_row();
            ui.label(t!(ui, "two-sided-depth"))
                .on_hover_text(t!(ui, "two-sided-depth-hover"));
            ui.add(crate::units::length(&mut s.depth).range(0.5..=50.0));
            ui.end_row();
            ui.label(t!(ui, "two-sided-tool"))
                .on_hover_text(t!(ui, "two-sided-tool-hover"));
            let automatic = t!(ui, "two-sided-tool-automatic");
            tool_combo(
                ui,
                "two-sided-tool",
                &mut s.tool,
                offer,
                ToolKind::Hole,
                Some(&automatic),
            );
            ui.end_row();
            ui.label(t!(ui, "two-sided-zero"));
            ui.checkbox(&mut s.zero, t!(ui, "two-sided-zero-check"))
                .on_hover_text(t!(ui, "two-sided-zero-hover"));
            ui.end_row();
        });
}

/// No room for the pins: the room needed, and for a fitted board *Widen the margin to …*.
fn no_room_row(ui: &mut egui::Ui, s: &mut TwoSidedSettings, layout: &DowelLayout) {
    let ctx = ui.ctx().clone();
    let len = |mm: f64| crate::units::text(&ctx, mm);
    ui.colored_label(
        ui.visuals().warn_fg_color,
        t!(
            ui,
            "two-sided-no-room",
            room = len(layout.room.max(0.0)).as_str(),
            needed = len(layout.needed).as_str()
        ),
    );
    match layout.margin {
        Some(margin) => {
            if ui
                .button(t!(ui, "two-sided-widen", margin = len(margin).as_str()))
                .on_hover_text(t!(ui, "two-sided-widen-hover"))
                .clicked()
            {
                s.margin = Some(margin);
            }
        }
        None => {
            ui.weak(t!(ui, "two-sided-no-room-own-board"));
        }
    }
}

/// The board to scale: its outline, the drawing faintly, where both sides cut shaded, the
/// turn axis dashed, the pins (in the warning colour when they don't fit) and the work zero
/// at the first pin.
fn two_sided_picture(
    ui: &mut egui::Ui,
    layout: &DowelLayout,
    lines: &[Vec<Point>],
    pins: bool,
    zero: bool,
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 200.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);
    let ((x0, y0), (x1, y1)) = layout.board;
    let inner = rect.shrink(16.0);
    let scale = (f64::from(inner.width()) / (x1 - x0).max(1e-3))
        .min(f64::from(inner.height()) / (y1 - y0).max(1e-3));
    let middle = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let at = |(x, y): Point| {
        inner.center()
            + egui::vec2(
                ((x - middle.0) * scale) as f32,
                (-(y - middle.1) * scale) as f32,
            )
    };
    let text = visuals.text_color();
    let faint = visuals.weak_text_color();
    let box_of = |((a, b), (c, d)): ((f64, f64), (f64, f64))| {
        egui::Rect::from_two_pos(at((a, b)), at((c, d)))
    };
    // Where both sides cut: the pins keep clear of it.
    painter.rect_filled(
        box_of(layout.reach),
        0.0,
        visuals.selection.bg_fill.gamma_multiply(0.35),
    );
    for line in lines {
        let points: Vec<egui::Pos2> = line.iter().map(|&p| at(p)).collect();
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, faint)));
    }
    painter.rect_stroke(
        box_of(layout.board),
        0.0,
        egui::Stroke::new(1.5, text),
        egui::StrokeKind::Middle,
    );
    let (a, b) = layout.axis;
    painter.extend(egui::Shape::dashed_line(
        &[at(a), at(b)],
        egui::Stroke::new(1.2, text),
        9.0,
        4.0,
    ));
    painter.text(
        at(b) + egui::vec2(0.0, -2.0),
        egui::Align2::CENTER_BOTTOM,
        t!(ui, "canvas-turn-axis"),
        egui::FontId::proportional(11.0),
        text,
    );
    if !pins {
        return;
    }
    let colour = if layout.fits() {
        text
    } else {
        visuals.warn_fg_color
    };
    let radius = ((layout.needed - 2.0) / 2.0 * scale).max(2.0) as f32;
    for &c in &layout.centres {
        painter.circle_stroke(at(c), radius, egui::Stroke::new(1.5, colour));
    }
    if zero {
        // The work zero's axes, in the canvas's colours.
        let palette = crate::view::Palette::of(visuals.dark_mode);
        let origin = at(layout.centres[0]);
        let length = 18.0;
        painter.line_segment(
            [origin, origin + egui::vec2(length, 0.0)],
            egui::Stroke::new(2.0, palette.axis_x),
        );
        painter.line_segment(
            [origin, origin + egui::vec2(0.0, -length)],
            egui::Stroke::new(2.0, palette.axis_y),
        );
    }
}
