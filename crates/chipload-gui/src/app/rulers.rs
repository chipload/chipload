// SPDX-License-Identifier: GPL-3.0-or-later
//! Rulers along the canvas's top and left edges in top view (docs/specs/rulers.md): drawing
//! coordinates in the display units, ticks on the grid's lines, the cursor marked.

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{self, Align2, FontId, Painter, Rect, Stroke, pos2, vec2};

use super::ChiploadApp;
use crate::draw::{TickSize, ruler_ticks};

/// How thick a ruler is, points.
pub(super) const RULER: f32 = 18.0;

impl ChiploadApp {
    /// The rulers over the canvas `rect`, when they are on and the view is from the top.
    pub(super) fn paint_rulers(&self, ui: &egui::Ui, painter: &Painter, rect: Rect) {
        if !self.show_rulers || !self.view.is_top() || self.edit.is_none() {
            return;
        }
        let visuals = ui.visuals();
        let fill = visuals.panel_fill.gamma_multiply(0.92);
        let line = visuals.widgets.noninteractive.bg_stroke.color;
        let ink = visuals.weak_text_color();
        let mark = visuals.selection.bg_fill;
        let font = FontId::proportional(10.0);
        let top = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + RULER));
        let left = Rect::from_min_max(rect.min, pos2(rect.left() + RULER, rect.bottom()));
        let painter = painter.with_clip_rect(rect);
        painter.rect_filled(top, 0.0, fill);
        painter.rect_filled(left, 0.0, fill);
        painter.line_segment(
            [top.left_bottom(), top.right_bottom()],
            Stroke::new(1.0, line),
        );
        painter.line_segment(
            [left.right_top(), left.right_bottom()],
            Stroke::new(1.0, line),
        );
        let view = &self.view;
        let length = |size: TickSize| match size {
            TickSize::Minor => 4.0,
            TickSize::Half => 7.0,
            TickSize::Major => RULER,
        };
        // Along X.
        let (x0, x1) = (
            view.to_world(top.left_top()).0,
            view.to_world(top.right_top()).0,
        );
        let along_x = painter.with_clip_rect(top.with_min_x(top.left() + RULER));
        for tick in ruler_ticks(x0, x1, view.scale, self.units) {
            let x = view.to_screen((tick.at, 0.0)).x;
            let h = length(tick.size);
            along_x.line_segment(
                [pos2(x, top.bottom() - h), pos2(x, top.bottom())],
                Stroke::new(1.0, ink),
            );
            if let Some(label) = tick.label {
                along_x.text(
                    pos2(x + 3.0, top.top() + 1.0),
                    Align2::LEFT_TOP,
                    label,
                    font.clone(),
                    ink,
                );
            }
        }
        // Along Y, the numbers turned to read upwards.
        let (y0, y1) = (
            view.to_world(left.left_bottom()).1,
            view.to_world(left.left_top()).1,
        );
        let along_y = painter.with_clip_rect(left.with_min_y(left.top() + RULER));
        for tick in ruler_ticks(y0, y1, view.scale, self.units) {
            let y = view.to_screen((0.0, tick.at)).y;
            let w = length(tick.size);
            along_y.line_segment(
                [pos2(left.right() - w, y), pos2(left.right(), y)],
                Stroke::new(1.0, ink),
            );
            if let Some(label) = tick.label {
                let galley = along_y.layout_no_wrap(label, font.clone(), ink);
                let text =
                    egui::epaint::TextShape::new(pos2(left.left() + 1.0, y - 3.0), galley, ink)
                        .with_angle(-FRAC_PI_2);
                along_y.add(text);
            }
        }
        // Where the cursor is.
        if let Some((cx, cy)) = self.cursor {
            let at = view.to_screen((cx, cy));
            if at.x > top.left() + RULER {
                along_x.line_segment(
                    [pos2(at.x, top.top()), pos2(at.x, top.bottom())],
                    Stroke::new(1.5, mark),
                );
            }
            if at.y > left.top() + RULER {
                along_y.line_segment(
                    [pos2(left.left(), at.y), pos2(left.right(), at.y)],
                    Stroke::new(1.5, mark),
                );
            }
        }
        // The corner: the unit.
        let corner = Rect::from_min_size(rect.min, vec2(RULER, RULER));
        painter.rect_filled(corner, 0.0, fill);
        let unit = chipload_pipeline::units::length_unit(self.units);
        painter.text(
            corner.center(),
            Align2::CENTER_CENTER,
            unit,
            FontId::proportional(9.0),
            ink,
        );
    }
}
