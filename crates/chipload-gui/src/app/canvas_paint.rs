// SPDX-License-Identifier: GPL-3.0-or-later
//! Painting the canvas: the scene, readouts, dimension labels and their typed entry,
//! overlays and the legend.

use super::*;

/// The accent frame round the canvas in Simulate, points.
const SIMULATE_FRAME: f32 = 3.0;
/// The progress bar's width in the canvas's notice, points.
pub(super) const PROGRESS_WIDTH: f32 = 160.0;

impl ChiploadApp {
    /// The scene and everything over it: readouts, the select box, labels, rulers, menus,
    /// overlays and the navigation bar.
    pub(super) fn paint_canvas(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        rect: egui::Rect,
        readout: Option<(Point, String)>,
        labels: &[DimensionLabel],
    ) {
        let operation = self.focused_operation();
        let highlight = Highlight {
            selected: self.selected_ids(),
            operation_contours: self.focused_contours(),
            operation,
            followed: self.followed_contours(),
        };
        self.scene.paint(painter, &self.view, &highlight);
        if let Some((at, text)) = readout {
            self.paint_readout(ui, painter, at, text);
        }
        self.paint_select_box(ui, painter);
        for label in labels {
            paint_label(ui, painter, label);
        }
        self.paint_rulers(ui, painter, rect);
        self.typed_entry(ui);
        self.corner_menu(ui);
        self.quick_menu_ui(ui);
        self.canvas_overlays(ui, rect);
        // Simulate frames the canvas in the accent colour (edit-and-simulate.md decision 10).
        if self.display.simulate {
            let stroke = egui::Stroke::new(SIMULATE_FRAME, ui.visuals().selection.bg_fill);
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
        }
        self.navigation_bar(ui, rect);
        self.orientation(ui, rect);
    }

    /// On top of the canvas, inside it: the out-of-date notice (or the simulation being
    /// updated) and the legend.
    /// (The legend's right edge is kept under `LEGEND_RIGHT` for the navigation bar.)
    fn canvas_overlays(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let visuals = ui.visuals().clone();
        let frame = egui::Frame::new()
            .fill(visuals.panel_fill.gamma_multiply(0.94))
            .stroke(visuals.widgets.noninteractive.bg_stroke)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(10, 6));
        let inner = rect.shrink(10.0);
        let top = egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min));
        // What is being worked on, with how far it has got (the user, 2026-11-14:
        // "there is no indication that the simulation is generating").
        let busy = match (self.generation_progress(), self.simulation_progress()) {
            (Some(share), _) => Some((t!(ui, "canvas-updating-toolpath"), share)),
            (None, Some(share)) if self.display.simulate && !self.toolpath.stale => {
                // The simulated stock still shows the last one until its worker is done
                // (user report 2026-10-17: it looked finished).
                Some((t!(ui, "canvas-updating-simulation"), share))
            }
            _ => None,
        };
        if let Some((label, share)) = busy {
            ui.scope_builder(top, |ui| {
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(label);
                        ui.add(
                            egui::ProgressBar::new(share as f32)
                                .desired_width(PROGRESS_WIDTH)
                                .show_percentage(),
                        );
                    });
                });
            });
        } else if self.toolpath.stale {
            ui.scope_builder(top, |ui| {
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Live preview: the new toolpath is about to be generated.
                        if self.prefs.auto_generate && self.toolpath.changed_at.is_some() {
                            ui.spinner();
                            ui.label(t!(ui, "canvas-updating-toolpath"));
                        } else {
                            let stale = t!(ui, "canvas-toolpath-out-of-date");
                            ui.label(
                                RichText::new(format!("⚠ {stale}")).color(visuals.warn_fg_color),
                            );
                            if ui
                                .button(format!("▶ {}", t!(ui, "canvas-generate")))
                                .on_hover_text(t!(ui, "canvas-generate-hover"))
                                .clicked()
                            {
                                self.generate();
                            }
                        }
                    });
                });
            });
        }
        let items = crate::view::legend(&self.scene, ui);
        // Anchored at the bottom: placed by its height last frame (estimated on the first).
        let size_id = egui::Id::new("legend-height");
        let rows = if self.display.legend_open {
            items.len()
        } else {
            0
        };
        let estimate = 30.0 + 15.0 * rows as f32;
        let height = ui.data(|d| d.get_temp::<f32>(size_id)).unwrap_or(estimate);
        // Standing on the navigation bar when there's no room for the bar beside it.
        let bottom = ui
            .data(|d| d.get_temp::<f32>(egui::Id::new(super::navigation::NAV_TOP)))
            .map_or(inner.bottom(), |top| inner.bottom().min(top - 6.0));
        let top_left = egui::pos2(inner.left(), (bottom - height).max(inner.top()));
        let place = egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_max(
                top_left,
                egui::pos2(inner.right(), bottom),
            ))
            .layout(egui::Layout::top_down(egui::Align::Min));
        let shown = ui
            .scope_builder(place, |ui| {
                frame.show(ui, |ui| self.legend_ui(ui, items)).response.rect
            })
            .inner;
        if (shown.height() - height).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(size_id, shown.height()));
            ui.ctx().request_repaint();
        }
        // The navigation bar keeps to the right of it.
        ui.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(super::navigation::LEGEND_RIGHT),
                shown.right(),
            )
        });
    }

    /// The legend's title (it folds) and its entries.
    fn legend_ui(&mut self, ui: &mut egui::Ui, items: Vec<(crate::view::Swatch, String)>) {
        ui.spacing_mut().item_spacing.y = 1.0;
        let arrow = if self.display.legend_open {
            "▾"
        } else {
            "▸"
        };
        if ui
            .selectable_label(
                false,
                RichText::new(format!("{arrow} {}", t!(ui, "canvas-legend"))).small(),
            )
            .on_hover_text(t!(ui, "canvas-legend-hover"))
            .clicked()
        {
            self.display.legend_open = !self.display.legend_open;
        }
        if !self.display.legend_open {
            return;
        }
        if self.scene.toolpath.is_some() {
            let label = t!(ui, "canvas-show-toolpath");
            ui.checkbox(&mut self.display.toolpath, RichText::new(label).small())
                .on_hover_text(t!(ui, "canvas-show-toolpath-hover"));
        }
        for (swatch, text) in items {
            ui.horizontal(|ui| {
                let (swatch_rect, _) =
                    ui.allocate_exact_size(egui::vec2(30.0, 14.0), Sense::hover());
                crate::view::paint_swatch(ui.painter(), swatch_rect, swatch, &self.scene.palette);
                ui.label(RichText::new(text).small());
            });
        }
    }

    // -- handles and snapping (docs/specs/drawing-follow-ups.md sections 1 and 2)

    /// The lengths of the selected drawn shape, laid out where they show: in top view, with
    /// nothing being drawn or measured. None while its length is being typed.
    pub(super) fn dimension_labels(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
    ) -> Vec<DimensionLabel> {
        if self.display.three_d
            || self.sketch().is_some()
            || self.measure().is_some()
            || !self.show_drawing()
            || self.modify().is_some()
            || self
                .canvas_tools
                .typed
                .as_ref()
                .is_some_and(|t| matches!(t.target, super::drawing::TypedTarget::Dimension(_)))
        {
            return Vec::new();
        }
        let Some(shape) = self.selected_shape().and_then(|id| self.shape(id)) else {
            return Vec::new();
        };
        let radius = matches!(
            shape.geometry,
            Geometry::Circle { .. } | Geometry::Polygon { .. }
        );
        chipload_job::edit::dimensions(&shape.geometry)
            .into_iter()
            .filter_map(|shown| {
                let number = crate::draw::fixed(shown.value, self.prefs.units);
                let text = if radius {
                    format!("R {number}")
                } else {
                    number
                };
                let galley = painter.layout_no_wrap(
                    text,
                    egui::FontId::proportional(11.0),
                    ui.visuals().text_color(),
                );
                let size = galley.size() + egui::vec2(6.0, 6.0);
                // Left out where its side is too short on screen to hold it.
                let span = shown.span * self.view.scale;
                if span < f64::from(size.x.max(size.y)) + 12.0 {
                    return None;
                }
                // Off the side, outward, clear of the side's middle handle (screen y down).
                let (nx, ny) = (shown.outward[0] as f32, -(shown.outward[1] as f32));
                let reach = 10.0 + (size.x * nx.abs() + size.y * ny.abs()) / 2.0;
                let centre =
                    self.view.to_screen((shown.at[0], shown.at[1])) + egui::vec2(nx, ny) * reach;
                Some(DimensionLabel {
                    dimension: shown.dimension,
                    value: shown.value,
                    rect: egui::Rect::from_center_size(centre, size),
                    galley,
                })
            })
            .collect()
    }

    /// The selection box being dragged: solid when it takes what lies inside it, dashed
    /// when it takes what it touches.
    fn paint_select_box(&self, ui: &egui::Ui, painter: &egui::Painter) {
        let Some((from, to, mode)) = self.select_box() else {
            return;
        };
        let rect = egui::Rect::from_two_pos(self.view.to_screen(from), self.view.to_screen(to));
        let colour = ui.visuals().selection.stroke.color;
        painter.rect_filled(rect, 0.0, colour.gamma_multiply(0.08));
        let stroke = egui::Stroke::new(1.0, colour);
        match mode {
            chipload_draw::select::BoxMode::Inside => {
                painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            }
            chipload_draw::select::BoxMode::Touching => {
                let c = [
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ];
                painter.extend(egui::Shape::dashed_line(&c, stroke, 6.0, 4.0));
            }
        }
    }

    /// The size being drawn, beside the cursor at `at`.
    fn paint_readout(&self, ui: &egui::Ui, painter: &egui::Painter, at: Point, text: String) {
        let visuals = ui.visuals();
        let galley = painter.layout_no_wrap(
            text,
            egui::FontId::proportional(12.0),
            visuals.strong_text_color(),
        );
        let corner = self.view.to_screen(at) + egui::vec2(14.0, -14.0);
        let rect = egui::Align2::LEFT_BOTTOM
            .anchor_size(corner, galley.size())
            .expand(3.0);
        painter.rect_filled(rect, 3.0, visuals.extreme_bg_color.gamma_multiply(0.9));
        painter.galley(
            rect.min + egui::vec2(3.0, 3.0),
            galley,
            visuals.text_color(),
        );
    }

    /// The entry of a value being typed while drawing: Enter applies it, Esc or a click
    /// elsewhere closes it.
    fn typed_entry(&mut self, ui: &egui::Ui) {
        let Some(mut typed) = self.canvas_tools.typed.clone() else {
            return;
        };
        let id = egui::Id::new("draw-typed-entry");
        let error = ui.visuals().error_fg_color;
        let shown = egui::Area::new(id.with("area"))
            .fixed_pos(typed.at)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        let mut edit = egui::TextEdit::singleline(&mut typed.text)
                            .id(id)
                            .desired_width(130.0);
                        if typed.refused {
                            edit = edit.text_color(error);
                        }
                        ui.add(edit)
                    })
                    .inner
            })
            .inner;
        if shown.changed() {
            typed.refused = false;
        }
        if typed.focus {
            shown.request_focus();
            // The cursor after the first character, not before it.
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                let end = egui::text::CCursor::new(typed.text.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), id);
            }
            typed.focus = false;
        }
        let entered = shown.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let closed = shown.lost_focus() && !entered;
        self.canvas_tools.typed = (!closed).then_some(typed);
        if entered {
            let cursor = self.drag.snapped.map(|(p, _)| p).or(self.cursor);
            if let Some(cursor) = cursor {
                self.apply_typed(cursor);
            }
        }
    }
}

/// A length of the selected shape, as it shows on the canvas.
pub(super) struct DimensionLabel {
    pub(super) dimension: chipload_job::edit::Dimension,
    pub(super) value: f64,
    /// Where it is on screen, its frame included.
    pub(super) rect: egui::Rect,
    galley: std::sync::Arc<egui::Galley>,
}

/// A length label: its number on a small framed patch, which a click opens for typing.
fn paint_label(ui: &egui::Ui, painter: &egui::Painter, label: &DimensionLabel) {
    let visuals = ui.visuals();
    painter.rect_filled(
        label.rect,
        3.0,
        visuals.extreme_bg_color.gamma_multiply(0.9),
    );
    painter.rect_stroke(
        label.rect,
        3.0,
        egui::Stroke::new(1.0, visuals.weak_text_color()),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        label.rect.min + egui::vec2(3.0, 3.0),
        label.galley.clone(),
        visuals.text_color(),
    );
}
