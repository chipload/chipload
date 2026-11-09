// SPDX-License-Identifier: GPL-3.0-or-later
//! Draw > Frame… (docs/specs/shape-library.md §5): a frame round the selection, or to a
//! size with nothing selected, set under Selection with a preview on the canvas. The frame
//! itself is `chipload_pipeline::frames`'.

use chipload_core::Point;
use chipload_pipeline::frames::{
    BORDER_LAYER, CornerStyle, FRAME_LAYER, FrameChange, FrameKind, FrameSize, MOST_BORDERS,
    NewFrame, add_frame, detach_frame, kind_of, place_frame, set_frame, style_of,
};

use super::modify::ModifyTool;
use super::*;
use crate::editor::shapes_of;

/// The frame tool's values, kept for the session (§3.4: drawn again with the last ones).
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSettings {
    pub kind: FrameKind,
    /// Off: a tenth of the shorter side.
    pub corner_set: bool,
    /// The corner size, mm, when set.
    pub corner: f64,
    pub borders: u32,
    /// Outline to first border, and between borders, mm.
    pub gap: f64,
    /// The borders' own corners, in a rectangular frame; None: the outline's shape.
    pub border_corners: Option<CornerStyle>,
    pub double: bool,
    /// A border's groove width at the surface, mm.
    pub border_width: f64,
    /// Round the selection: from its box to the innermost line, mm.
    pub margin: f64,
    /// With nothing selected: the outline's size, mm.
    pub width: f64,
    pub height: f64,
    pub layer: String,
    pub border_layer: String,
}

impl Default for FrameSettings {
    fn default() -> Self {
        Self {
            kind: FrameKind::Rounded,
            corner_set: false,
            corner: 10.0,
            borders: 1,
            gap: 5.0,
            border_corners: None,
            double: false,
            border_width: 0.0,
            margin: 10.0,
            width: 300.0,
            height: 150.0,
            layer: FRAME_LAYER.to_string(),
            border_layer: BORDER_LAYER.to_string(),
        }
    }
}

/// The frame the preview shows, kept until the values, the selection or the drawing change.
#[derive(Debug, Clone, Default)]
pub(super) struct FramePreview {
    key: String,
    pub lines: Vec<Vec<Point>>,
    /// Its size and corner, or why it can't be drawn.
    pub place: Option<Result<(f64, f64, f64), String>>,
}

/// A frame's name in the window's language.
fn kind_name(ui: &egui::Ui, kind: FrameKind) -> String {
    match kind {
        FrameKind::Rectangle => t!(ui, "frame-kind-rectangle"),
        FrameKind::Rounded => t!(ui, "frame-kind-rounded"),
        FrameKind::Notched => t!(ui, "frame-kind-notched"),
        FrameKind::Chamfered => t!(ui, "frame-kind-chamfered"),
        FrameKind::Oval => t!(ui, "frame-kind-oval"),
        FrameKind::Arched => t!(ui, "frame-kind-arched"),
        FrameKind::Plaque => t!(ui, "frame-kind-plaque"),
        FrameKind::Stepped => t!(ui, "frame-kind-stepped"),
        FrameKind::DoubleArch => t!(ui, "frame-kind-double-arch"),
        FrameKind::Gable => t!(ui, "frame-kind-gable"),
        FrameKind::Shield => t!(ui, "frame-kind-shield"),
        FrameKind::Banner => t!(ui, "frame-kind-banner"),
        FrameKind::Octagon => t!(ui, "frame-kind-octagon"),
    }
}

/// A rectangular frame's corner style in the window's language.
fn corners_name(ui: &egui::Ui, style: CornerStyle) -> String {
    match style {
        CornerStyle::Sharp => t!(ui, "frame-corners-sharp"),
        CornerStyle::Round => t!(ui, "frame-corners-round"),
        CornerStyle::Notch => t!(ui, "frame-corners-notch"),
        CornerStyle::Chamfer => t!(ui, "frame-corners-chamfer"),
        CornerStyle::Step => t!(ui, "frame-corners-step"),
    }
}

/// The rows for a frame's borders after the count: gap, their corners (a rectangular
/// frame), double lines and the groove width (docs/specs/sign-maker.md §5).
struct BorderValues<'a> {
    kind: FrameKind,
    borders: u32,
    gap: &'a mut f64,
    corners: &'a mut Option<CornerStyle>,
    double: &'a mut bool,
    width: &'a mut f64,
}

fn border_rows(ui: &mut egui::Ui, id: &str, v: BorderValues<'_>) {
    if v.borders == 0 {
        return;
    }
    ui.label(t!(ui, "frame-gap"));
    ui.add(crate::units::length(v.gap))
        .on_hover_text(t!(ui, "frame-gap-hover"));
    ui.end_row();
    if v.kind.corner_style().is_some() {
        ui.label(t!(ui, "frame-border-corners"));
        let shown = match *v.corners {
            Some(style) => corners_name(ui, style),
            None => t!(ui, "frame-border-corners-same"),
        };
        egui::ComboBox::from_id_salt(format!("{id}-border-corners"))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                ui.selectable_value(v.corners, None, t!(ui, "frame-border-corners-same"));
                for style in CornerStyle::ALL {
                    ui.selectable_value(v.corners, Some(style), corners_name(ui, style));
                }
            })
            .response
            .on_hover_text(t!(ui, "frame-border-corners-hover"));
        ui.end_row();
    }
    ui.label(t!(ui, "frame-border-width"));
    ui.add(crate::units::length(v.width))
        .on_hover_text(t!(ui, "frame-border-width-hover"));
    ui.end_row();
    ui.label("");
    ui.checkbox(v.double, t!(ui, "frame-double"))
        .on_hover_text(t!(ui, "frame-double-hover"));
    ui.end_row();
}

impl ChiploadApp {
    /// Draw > Frame…: its values under Selection.
    pub fn start_frame(&mut self) {
        self.start_modify(ModifyTool::Frame);
    }

    /// The shapes of the selected contours, any kind: what a frame goes round.
    pub(super) fn framed_shape_ids(&self) -> Vec<u64> {
        let picked = self.selected_ids();
        let ids: std::collections::BTreeSet<u64> = self
            .contours
            .iter()
            .filter(|c| picked.contains(&c.id))
            .flat_map(shapes_of)
            .collect();
        ids.into_iter().collect()
    }

    /// The frame the tool's values make now: round the selection, or to a size.
    fn new_frame(&self) -> NewFrame {
        let f = &self.canvas_tools.frame;
        let ids = self.framed_shape_ids();
        let size = if ids.is_empty() {
            FrameSize::Fixed {
                width: f.width,
                height: f.height,
                center: None,
            }
        } else {
            FrameSize::Around {
                ids,
                margin: f.margin,
            }
        };
        NewFrame {
            kind: f.kind,
            r: f.corner_set.then_some(f.corner),
            borders: f.borders,
            gap: f.gap,
            border_corners: f.border_corners.filter(|_| f.kind.corner_style().is_some()),
            double: f.double,
            border_width: f.border_width,
            size,
            layer: f.layer.clone(),
            border_layer: f.border_layer.clone(),
        }
    }

    /// The outlines the frame would add, for the preview, and its size.
    pub(super) fn frame_outlines(&mut self) -> Vec<Vec<Point>> {
        let Some(edit) = self.edit.as_ref() else {
            return Vec::new();
        };
        let frame = self.new_frame();
        let key = format!("{frame:?}{:?}", edit.job.drawing);
        if self.canvas_tools.frame_preview.key != key {
            let job = &edit.job;
            let lang = self.prefs.language;
            let units = self.prefs.units.into();
            let place = place_frame(job, &frame)
                .map(|p| (p.width, p.height, p.r))
                .map_err(|e| e.message().render(lang, units));
            let lines = add_frame(job, &frame)
                .map(|edited| {
                    edited
                        .job
                        .drawing
                        .shapes
                        .iter()
                        .filter(|s| edited.new_ids.contains(&s.id))
                        .flat_map(|s| crate::draw::outline(&s.geometry))
                        .collect()
                })
                .unwrap_or_default();
            self.canvas_tools.frame_preview = FramePreview {
                key,
                lines,
                place: Some(place),
            };
        }
        self.canvas_tools.frame_preview.lines.clone()
    }

    /// Add the frame, as one undo step; its shapes are selected and the tool ends.
    pub fn create_frame(&mut self) -> bool {
        let Some(job) = self.job_with_operations() else {
            return false;
        };
        let result = add_frame(&job, &self.new_frame());
        let done = self.take_edit(result, &[]);
        if done {
            self.cancel_modify();
        }
        done
    }

    /// Under Selection while the frame tool is active: the frames, their values, Add.
    pub(super) fn frame_form(&mut self, ui: &mut egui::Ui) {
        if self.modify() != Some(ModifyTool::Frame) {
            return;
        }
        let around = !self.framed_shape_ids().is_empty();
        // The preview's numbers, for the size shown and the reason a frame can't be.
        self.frame_outlines();
        let place = self.canvas_tools.frame_preview.place.clone();
        ui.separator();
        ui.label(crate::style::bold(ui, t!(ui, "frame-title")));
        ui.label(if around {
            t!(ui, "frame-around")
        } else {
            t!(ui, "frame-to-size")
        });
        kind_choice(ui, "frame", &mut self.canvas_tools.frame.kind);
        let units = self.prefs.units;
        let f = &mut self.canvas_tools.frame;
        egui::Grid::new("frame-form").num_columns(2).show(ui, |ui| {
            if around {
                ui.label(t!(ui, "frame-margin"));
                ui.add(crate::units::length(&mut f.margin))
                    .on_hover_text(t!(ui, "frame-margin-hover"));
                ui.end_row();
            } else {
                ui.label(t!(ui, "frame-width"));
                ui.add(crate::units::length(&mut f.width));
                ui.end_row();
                ui.label(t!(ui, "frame-height"));
                ui.add(crate::units::length(&mut f.height));
                ui.end_row();
            }
            if f.kind.uses_r() {
                ui.label(t!(ui, "frame-corner"));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut f.corner_set, "")
                        .on_hover_text(t!(ui, "frame-corner-auto-hover"));
                    if f.corner_set {
                        ui.add(crate::units::length(&mut f.corner));
                    } else if let Some(Ok((_, _, r))) = &place {
                        ui.weak(t!(
                            ui,
                            "frame-corner-auto",
                            size = chipload_pipeline::units::length_text(*r, units)
                        ));
                    }
                })
                .response
                .on_hover_text(t!(ui, "frame-corner-hover"));
                ui.end_row();
            }
            ui.label(t!(ui, "frame-borders"));
            ui.add(egui::DragValue::new(&mut f.borders).range(0..=MOST_BORDERS));
            ui.end_row();
            border_rows(
                ui,
                "frame",
                BorderValues {
                    kind: f.kind,
                    borders: f.borders,
                    gap: &mut f.gap,
                    corners: &mut f.border_corners,
                    double: &mut f.double,
                    width: &mut f.border_width,
                },
            );
            ui.label(t!(ui, "frame-layer"));
            ui.add(egui::TextEdit::singleline(&mut f.layer).desired_width(100.0));
            ui.end_row();
            if f.borders > 0 {
                ui.label(t!(ui, "frame-border-layer"));
                ui.add(egui::TextEdit::singleline(&mut f.border_layer).desired_width(100.0));
                ui.end_row();
            }
        });
        let ok = match &place {
            Some(Ok((w, h, _))) => {
                if around {
                    ui.weak(t!(
                        ui,
                        "frame-size",
                        width = chipload_pipeline::units::length_text(*w, units),
                        height = chipload_pipeline::units::length_text(*h, units)
                    ));
                }
                true
            }
            Some(Err(why)) => {
                ui.colored_label(ui.visuals().warn_fg_color, why);
                false
            }
            None => false,
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(ok, egui::Button::new(t!(ui, "frame-add")))
                .clicked()
            {
                self.create_frame();
            }
            if ui.button(t!(ui, "common-cancel")).clicked() {
                self.cancel_modify();
            }
        });
    }

    /// The live frame one of the selected shapes belongs to.
    fn selected_live_frame(&self) -> Option<chipload_job::Frame> {
        let edit = self.edit.as_ref()?;
        self.framed_shape_ids()
            .into_iter()
            .find_map(|id| edit.job.drawing.frame_of(id).cloned())
    }

    /// Under Selection, for a frame that follows what it goes round: its values, changed in
    /// place, and Detach (docs/specs/sign-maker.md §4).
    pub(super) fn live_frame_ui(&mut self, ui: &mut egui::Ui) {
        if self.modify() == Some(ModifyTool::Frame) {
            return;
        }
        let Some(frame) = self.selected_live_frame() else {
            return;
        };
        let units = self.prefs.units;
        let mut kind = kind_of(frame.kind);
        let mut corner = frame.corner;
        let mut margin = frame.margin;
        let mut borders = u32::try_from(frame.borders.len()).unwrap_or(0);
        let mut gap = frame.gap;
        let mut border_corners = frame.border_corners.map(style_of);
        let mut double = frame.double;
        let mut border_width = frame.border_width;
        let lines_per = if frame.double { 2 } else { 1 };
        borders /= lines_per;
        let old_borders = borders;
        let mut width = frame.width;
        let mut height = frame.height;
        let mut detach = false;
        let enabled = self.can_draw();
        crate::app::forms::card(ui, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                ui.label(crate::style::bold(ui, t!(ui, "frame-live-title")));
                ui.weak(match &frame.around {
                    Some(around) => t!(
                        ui,
                        "frame-live-follows",
                        texts = around.texts.len(),
                        others = around.images.len() + around.shapes.len()
                    ),
                    None => t!(ui, "frame-live-fixed"),
                });
                kind_choice(ui, "live-frame", &mut kind);
                egui::Grid::new("live-frame").num_columns(2).show(ui, |ui| {
                    if frame.around.is_some() {
                        ui.label(t!(ui, "frame-margin"));
                        ui.add(crate::units::length(&mut margin))
                            .on_hover_text(t!(ui, "frame-margin-hover"));
                        ui.end_row();
                    } else {
                        ui.label(t!(ui, "frame-width"));
                        ui.add(crate::units::length(&mut width));
                        ui.end_row();
                        ui.label(t!(ui, "frame-height"));
                        ui.add(crate::units::length(&mut height));
                        ui.end_row();
                    }
                    if kind.uses_r() {
                        ui.label(t!(ui, "frame-corner"));
                        ui.add(crate::units::length(&mut corner))
                            .on_hover_text(t!(ui, "frame-corner-hover"));
                        ui.end_row();
                    }
                    ui.label(t!(ui, "frame-borders"));
                    ui.add(egui::DragValue::new(&mut borders).range(0..=MOST_BORDERS));
                    ui.end_row();
                    border_rows(
                        ui,
                        "live-frame",
                        BorderValues {
                            kind,
                            borders,
                            gap: &mut gap,
                            corners: &mut border_corners,
                            double: &mut double,
                            width: &mut border_width,
                        },
                    );
                });
                if frame.around.is_some() {
                    ui.weak(t!(
                        ui,
                        "frame-size",
                        width = chipload_pipeline::units::length_text(frame.width, units),
                        height = chipload_pipeline::units::length_text(frame.height, units)
                    ));
                }
                detach = ui
                    .button(t!(ui, "frame-detach"))
                    .on_hover_text(t!(ui, "frame-detach-hover"))
                    .clicked();
            });
        });
        let Some(job) = self.job_with_operations() else {
            return;
        };
        if detach {
            if let Ok(out) = detach_frame(&job, frame.outline) {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job.drawing = out.drawing;
                }
                self.commit();
            }
            return;
        }
        let change = FrameChange {
            kind: (kind != kind_of(frame.kind)).then_some(kind),
            corner: (corner != frame.corner).then_some(corner),
            margin: (margin != frame.margin).then_some(margin),
            borders: (borders != old_borders).then_some(borders),
            gap: (gap != frame.gap).then_some(gap),
            border_corners: (border_corners != frame.border_corners.map(style_of))
                .then_some(border_corners),
            double: (double != frame.double).then_some(double),
            border_width: (border_width != frame.border_width).then_some(border_width),
            width: (width != frame.width).then_some(width),
            height: (height != frame.height).then_some(height),
        };
        if change != FrameChange::default() {
            let keep: Vec<u64> = frame.shapes().collect();
            let result = set_frame(&job, frame.outline, &change);
            self.take_edit(result, &keep);
        }
    }
}

/// The frames as small pictures to pick from, `chosen` marked; a click picks one. The
/// rectangular frames are one picture, with their corners chosen under it (§5).
fn kind_choice(ui: &mut egui::Ui, id: &str, chosen: &mut FrameKind) {
    ui.horizontal_wrapped(|ui| {
        for kind in FrameKind::SHAPES {
            let rectangular = kind.corner_style().is_some();
            let size = egui::vec2(44.0, 30.0);
            let selected = if rectangular {
                chosen.corner_style().is_some()
            } else {
                *chosen == kind
            };
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let visuals = ui.style().interact_selectable(&response, selected);
            let painter = ui.painter();
            if selected || response.hovered() {
                painter.rect_filled(rect, 3.0, visuals.weak_bg_fill);
            }
            let stroke = egui::Stroke::new(1.5, visuals.fg_stroke.color);
            let inner = rect.shrink(6.0);
            let (w, h) = (f64::from(inner.width()), f64::from(inner.height()));
            // The rectangle in the corners chosen; twice the default corner, so that the
            // kinds tell apart at this size.
            let drawn = if rectangular && selected {
                *chosen
            } else {
                kind
            };
            let r = (drawn.default_r(w, h) * 2.0).min(drawn.largest_r(w, h));
            if let Ok(chain) = chipload_draw::frames::outline(drawn, (0.0, 0.0), w, h, r) {
                let points: Vec<egui::Pos2> = chain
                    .prims
                    .iter()
                    .flat_map(|p| p.points())
                    .map(|(x, y)| inner.center() + egui::vec2(x as f32, -y as f32))
                    .collect();
                painter.add(egui::Shape::closed_line(points, stroke));
            }
            let name = if rectangular {
                t!(ui, "frame-kind-rectangle")
            } else {
                kind_name(ui, kind)
            };
            let response = response.on_hover_text(name);
            if response.clicked() && !selected {
                *chosen = kind;
            }
        }
    });
    if let Some(mut style) = chosen.corner_style() {
        ui.horizontal(|ui| {
            ui.label(t!(ui, "frame-corners"));
            egui::ComboBox::from_id_salt(format!("{id}-corners"))
                .selected_text(corners_name(ui, style))
                .show_ui(ui, |ui| {
                    for s in CornerStyle::ALL {
                        ui.selectable_value(&mut style, s, corners_name(ui, s));
                    }
                });
        });
        *chosen = style.kind();
    }
}
