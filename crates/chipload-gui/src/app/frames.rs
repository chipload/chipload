// SPDX-License-Identifier: GPL-3.0-or-later
//! Draw > Frame… (docs/specs/shape-library.md §5): a frame round the selection, or to a
//! size with nothing selected, set under Selection with a preview on the canvas. The frame
//! itself is `chipload_pipeline::frames`'.

use chipload_core::Point;
use chipload_pipeline::frames::{
    BORDER_LAYER, FRAME_LAYER, FrameKind, FrameSize, MOST_BORDERS, NewFrame, add_frame, place_frame,
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
    }
}

impl ChiploadApp {
    /// Draw > Frame…: its values under Selection.
    pub fn start_frame(&mut self) {
        self.start_modify(ModifyTool::Frame);
    }

    /// The shapes of the selected contours, any kind: what a frame goes round.
    pub(super) fn framed_shape_ids(&self) -> Vec<u64> {
        let picked = self.selection.ids(&self.contours);
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
        let f = &self.frame;
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
        let key = format!("{frame:?}{:?}", edit.settings.drawing);
        if self.frame_preview.key != key {
            let job = &edit.settings;
            let lang = self.language;
            let units = self.units.into();
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
            self.frame_preview = FramePreview {
                key,
                lines,
                place: Some(place),
            };
        }
        self.frame_preview.lines.clone()
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
        if self.modify != Some(ModifyTool::Frame) {
            return;
        }
        let around = !self.framed_shape_ids().is_empty();
        // The preview's numbers, for the size shown and the reason a frame can't be.
        self.frame_outlines();
        let place = self.frame_preview.place.clone();
        ui.separator();
        ui.strong(t!(ui, "frame-title"));
        ui.label(if around {
            t!(ui, "frame-around")
        } else {
            t!(ui, "frame-to-size")
        });
        self.frame_kinds(ui);
        let units = self.units;
        let f = &mut self.frame;
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
            if f.borders > 0 {
                ui.label(t!(ui, "frame-gap"));
                ui.add(crate::units::length(&mut f.gap))
                    .on_hover_text(t!(ui, "frame-gap-hover"));
                ui.end_row();
            }
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

    /// The frames as small pictures to pick from, the chosen one marked.
    fn frame_kinds(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for kind in FrameKind::ALL {
                let size = egui::vec2(44.0, 30.0);
                let selected = self.frame.kind == kind;
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                let visuals = ui.style().interact_selectable(&response, selected);
                let painter = ui.painter();
                if selected || response.hovered() {
                    painter.rect_filled(rect, 3.0, visuals.weak_bg_fill);
                }
                let stroke = egui::Stroke::new(1.5, visuals.fg_stroke.color);
                let inner = rect.shrink(6.0);
                let (w, h) = (f64::from(inner.width()), f64::from(inner.height()));
                // Twice the default corner: the kinds tell apart at this size.
                let r = kind.default_r(w, h) * 2.0;
                if let Ok(chain) = chipload_draw::frames::outline(kind, (0.0, 0.0), w, h, r) {
                    let points: Vec<egui::Pos2> = chain
                        .prims
                        .iter()
                        .flat_map(|p| p.points())
                        .map(|(x, y)| inner.center() + egui::vec2(x as f32, -y as f32))
                        .collect();
                    painter.add(egui::Shape::closed_line(points, stroke));
                }
                let response = response.on_hover_text(kind_name(ui, kind));
                if response.clicked() {
                    self.frame.kind = kind;
                }
            }
        });
    }
}
