// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: input, painting and hover texts.

use super::*;
use chipload_core::order::Ordered;

/// How far beyond a rectangle's end its rotation handle sits, in screen pixels.
const ROTATE_HANDLE_PX: f64 = 24.0;

impl ChiploadApp {
    /// Switch to the 3D view, starting from a three-quarter view (docs/specs/3d-view.md).
    pub fn view_3d(&mut self) {
        if !self.three_d {
            self.three_d = true;
            self.sketch = None; // drawing and measuring work in top view only
            self.measure = None;
            let pivot = self.orbit_pivot();
            self.view
                .orient(-30f64.to_radians(), 55f64.to_radians(), pivot);
        }
    }

    /// Back to looking straight down, as the 2D view always did.
    pub fn view_top(&mut self) {
        self.three_d = false;
        let pivot = self.orbit_pivot();
        self.view.orient(0.0, 0.0, pivot);
    }

    /// Each contour's copies, as (contour id, shift), copy 1 left out.
    fn copy_shifts(&self) -> Vec<(usize, Point)> {
        let Some(edit) = &self.edit else {
            return Vec::new();
        };
        if edit.settings.parts.iter().all(|p| p.count() < 2) {
            return Vec::new();
        }
        let mut job = edit.settings.clone();
        job.operations = edit.ops.iter().map(|e| e.op.clone()).collect();
        chipload_pipeline::sheet::contour_copies(&job, &self.contours)
            .into_iter()
            .flat_map(|(c, shifts)| {
                shifts
                    .into_iter()
                    .filter(|s| *s != (0.0, 0.0))
                    .map(move |s| (c.id, s))
            })
            .collect()
    }

    /// What the 3D view turns around: the stock's centre, else the drawing origin.
    pub(super) fn orbit_pivot(&self) -> Point {
        self.scene.stock.map_or((0.0, 0.0), |((x0, y0), (x1, y1))| {
            ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
        })
    }

    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        self.keep_drawing_in_view();
        self.scene.hide_toolpath = !self.show_toolpath;
        self.scene.hide_drawing = !self.show_drawing();
        self.scene.grid = self.show_grid;
        self.scene.inch_grid = self.units == chipload_job::Units::Inch;
        self.scene.palette = crate::view::Palette::of(ui.visuals().dark_mode);
        self.scene.work_zero = self
            .edit
            .as_ref()
            .map_or((0.0, 0.0), |e| e.settings.work_zero_xy());
        // Copies of each part, faded (docs/specs/copies-grid.md, parts.md §8): the contours
        // its operations cut; copy 1 is the drawing.
        self.scene.copies = self.copy_shifts();
        // Heights for the 3D view: 2D items sit on the stock top (docs/specs/3d-view.md).
        if let Some(edit) = &self.edit {
            let stock = &edit.settings.stock;
            let top = stock.top_z();
            self.view.top_z = top;
            self.scene.stock_z = Some((top, top - stock.thickness));
        }
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        if self.fit_pending {
            let points = self.scene.fit_points(self.view.top_z);
            if !points.is_empty() {
                self.view.fit(&points, rect);
                self.fit_pending = false;
            }
        }
        let (alt, shift) = ui.input(|i| (i.modifiers.alt, i.modifiers.shift));
        let shape_handles = if self.show_drawing() {
            self.shape_handles()
        } else {
            Vec::new()
        };
        let (middles, others): (Vec<_>, Vec<_>) = shape_handles
            .iter()
            .partition(|(h, _)| matches!(h, Handle::Middle(_)));
        self.scene.handles = others.into_iter().map(|&(_, p)| p).collect();
        self.scene.middle_handles = middles.into_iter().map(|&(_, p)| p).collect();
        // A selected picture's or text's box handles (docs/specs/box-handles.md).
        let box_handles = self.box_handle_points();
        self.scene
            .handles
            .extend(box_handles.iter().map(|&(_, p)| p));
        let view = self.view;
        let nearest = |points: &mut dyn Iterator<Item = Point>, pos: egui::Pos2| {
            points
                .enumerate()
                .map(|(i, p)| (i, view.to_screen(p).distance(pos)))
                .filter(|&(_, d)| d <= crate::view::PICK_TOLERANCE_PX)
                .min_by_key(|&(_, d)| Ordered(f64::from(d)))
                .map(|(i, _)| i)
        };
        let on_handle = |pos: egui::Pos2| -> Option<Handle> {
            nearest(&mut shape_handles.iter().map(|&(_, p)| p), pos).map(|i| shape_handles[i].0)
        };
        let on_box_handle = |pos: egui::Pos2| {
            nearest(&mut box_handles.iter().map(|&(_, p)| p), pos).map(|i| box_handles[i].0)
        };
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            if let Some(handle) = on_handle(origin) {
                self.start_handle_drag(handle);
            } else if let Some(handle) = on_box_handle(origin) {
                self.start_box_drag(handle);
            } else {
                self.drag.start = self.start_marker_at(origin);
            }
        }
        if response.drag_stopped() {
            self.end_handle_drag();
            self.end_box_drag();
            self.drop_start();
        }
        self.drag.snapped = None;
        self.scene.snap_mark = None;
        if let Some(marker) = self.drag.start {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.drag_start_to(marker, self.view.to_world(pos));
            }
        } else if self.drag.handle.is_some() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let to = self.snap_world(self.view.to_world(pos), alt, true);
                self.drag_handle_to(to, shift);
            }
        } else if self.dragging_box() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let to = self.snap_world(self.view.to_world(pos), alt, true);
                self.drag_box_to(to, alt);
            }
        } else if response.dragged() {
            // In 3D, left-drag orbits; right-drag or Shift+drag pans. In top view any drag pans.
            let pan = !self.three_d
                || response.dragged_by(egui::PointerButton::Secondary)
                || response.dragged_by(egui::PointerButton::Middle)
                || ui.input(|i| i.modifiers.shift);
            if pan {
                self.view.pan(response.drag_delta());
            } else {
                self.view.orbit(response.drag_delta(), self.orbit_pivot());
            }
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = f64::from(zoom) * (f64::from(scroll) * 0.0015).exp();
            if (factor - 1.0).abs() > 1e-6 {
                self.view.zoom_at(pos, factor);
            }
            self.cursor = Some(self.view.to_world(pos));
        } else {
            self.cursor = None;
        }
        if self.sketch.is_some() {
            self.scene.measure = None;
            let at = self.cursor.map(|c| self.snap_world(c, alt, false));
            if response.double_clicked() {
                self.finish_drawing();
            } else if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let first = self.sketch.as_ref().and_then(|s| s.points.first().copied());
                let on_first = first.is_some_and(|f| {
                    self.view.to_screen(f).distance(pos) <= crate::view::PICK_TOLERANCE_PX
                });
                let p = self.snap_world(self.view.to_world(pos), alt, false);
                self.draw_click(p, on_first);
            }
            self.scene.preview = match (&self.sketch, at) {
                (Some(sketch), Some(at)) => sketch.preview(at),
                _ => Vec::new(),
            };
            self.scene.sketch_points = self
                .sketch
                .as_ref()
                .map(|s| s.points.clone())
                .unwrap_or_default();
        } else if self.placing_text() {
            self.scene.preview.clear();
            self.scene.sketch_points.clear();
            self.scene.measure = None;
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let at = self.snap_world(self.view.to_world(pos), alt, false);
                self.place_text_at(at);
            }
        } else if self.measure.is_some() {
            self.scene.preview.clear();
            self.scene.sketch_points.clear();
            let at = self.cursor.map(|c| self.snap_world(c, alt, false));
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let p = self.snap_world(self.view.to_world(pos), alt, false);
                self.measure_click(p);
            }
            self.scene.measure = self.measure_scene(at);
        } else {
            self.scene.preview.clear();
            self.scene.sketch_points.clear();
            self.scene.measure = None;
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && on_handle(pos).is_none()
                && on_box_handle(pos).is_none()
                && self.start_marker_at(pos).is_none()
                && self.show_drawing()
            {
                let toggle = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                let picked = self.scene.pick(&self.view, pos);
                self.click(picked, toggle);
            }
        }
        let outline = self.box_outline();
        self.scene.preview.extend(outline);
        if response.secondary_clicked() {
            let at = response.interact_pointer_pos();
            self.drag.context_start = at.and_then(|pos| self.start_marker_at(pos));
            self.context_pick = at.and_then(|pos| self.scene.pick(&self.view, pos));
        }
        let context_start = self.drag.context_start;
        let picked = self
            .context_pick
            .and_then(|id| self.contours.iter().find(|c| c.id == id));
        let layers: Vec<String> = picked.map(Contour::layers).unwrap_or_default();
        // The layer within the picked contour's DXF or image, not across all of them
        // (docs/specs/drawing-sources.md §2).
        let from = picked.and_then(|c| {
            let first = c.parts.first()?.segment.from;
            c.parts
                .iter()
                .all(|p| p.segment.from == first)
                .then_some(first)
                .flatten()
        });
        let picked_id = picked.map(|c| c.id);
        response.context_menu(|ui| match (context_start, picked_id) {
            (Some(marker), _) => {
                let moved = self
                    .scene
                    .start_markers
                    .get(marker)
                    .is_some_and(|m| m.stored.is_some());
                if ui
                    .add_enabled(moved, egui::Button::new(t!(ui, "canvas-reset-start")))
                    .on_disabled_hover_text(t!(ui, "canvas-reset-start-disabled-hover"))
                    .clicked()
                {
                    self.reset_start(marker);
                    ui.close();
                }
            }
            (None, Some(id)) => {
                if ui
                    .button(t!(ui, "canvas-select-contour", id = id.to_string()))
                    .clicked()
                {
                    self.click(Some(id), false);
                    ui.close();
                }
                for layer in &layers {
                    if ui
                        .button(t!(ui, "canvas-select-layer", layer = layer))
                        .clicked()
                    {
                        self.selection = CanvasSelection::Layer(layer.clone(), from);
                        ui.close();
                    }
                }
            }
            (None, None) => {
                ui.label(t!(ui, "canvas-right-click-hint"));
            }
        });
        let operation = match self.focus {
            Focus::Op(i) | Focus::RestPass(i, _) => Some(i),
            _ => None,
        };
        let plunges = response
            .hover_pos()
            .map(|pos| self.scene.plunges_at(&self.view, pos))
            .unwrap_or_default();
        let on_start = response
            .hover_pos()
            .and_then(|pos| self.start_marker_at(pos))
            .and_then(|i| self.scene.start_markers.get(i));
        if let Some(marker) = on_start.filter(|_| !response.dragged()) {
            let text = match marker.stored {
                Some(_) => t!(ui, "canvas-start-moved-hover"),
                None => t!(ui, "canvas-start-hover"),
            };
            response.on_hover_text_at_pointer(text);
        } else if !plunges.is_empty() && !response.dragged() {
            let text = self.plunge_text(&plunges);
            response.on_hover_text_at_pointer(text);
        }
        let highlight = Highlight {
            selected: self.selection.ids(&self.contours),
            operation_contours: self.focused_contours(),
            operation,
            followed: self.followed_contours(),
        };
        self.scene.paint(&painter, &self.view, &highlight);
        self.canvas_overlays(ui, rect);
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
        if self.toolpath_stale {
            let top = egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Min));
            ui.scope_builder(top, |ui| {
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Live preview: the new toolpath is on its way (or about to be).
                        let coming =
                            self.generating() || (self.auto_generate && self.changed_at.is_some());
                        if coming {
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
        } else if self.simulating() && self.show_simulation {
            // The toolpath is current, but the simulated stock still shows the last one
            // until its worker is done (user report 2026-10-17: it looked finished).
            let top = egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Min));
            ui.scope_builder(top, |ui| {
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(t!(ui, "canvas-updating-simulation"));
                    });
                });
            });
        }
        let items = crate::view::legend(&self.scene, ui);
        // Anchored at the bottom: placed by its height last frame (estimated on the first).
        let size_id = egui::Id::new("legend-height");
        let rows = if self.legend_open { items.len() } else { 0 };
        let estimate = 30.0 + 15.0 * rows as f32;
        let height = ui.data(|d| d.get_temp::<f32>(size_id)).unwrap_or(estimate);
        let top_left = egui::pos2(inner.left(), (inner.bottom() - height).max(inner.top()));
        let place = egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_max(top_left, inner.right_bottom()))
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
        let arrow = if self.legend_open { "▾" } else { "▸" };
        if ui
            .selectable_label(
                false,
                RichText::new(format!("{arrow} {}", t!(ui, "canvas-legend"))).small(),
            )
            .on_hover_text(t!(ui, "canvas-legend-hover"))
            .clicked()
        {
            self.legend_open = !self.legend_open;
        }
        if !self.legend_open {
            return;
        }
        if self.scene.toolpath.is_some() {
            let label = t!(ui, "canvas-show-toolpath");
            ui.checkbox(&mut self.show_toolpath, RichText::new(label).small())
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

    /// Handles of the selected shape: one drawn shape, in top view, not while drawing.
    pub(super) fn shape_handles(&self) -> Vec<(Handle, Point)> {
        if self.three_d || self.sketch.is_some() || !self.can_draw() {
            return Vec::new();
        }
        let Some(shape) = self.selected_shape().and_then(|id| self.shape(id)) else {
            return Vec::new();
        };
        // The rotation handle sits a fixed distance on screen beyond the rectangle.
        let reach = ROTATE_HANDLE_PX / self.view.scale;
        handles(&shape.geometry, reach)
            .into_iter()
            .map(|(h, p)| (h, (p[0], p[1])))
            .collect()
    }

    /// A drawn polyline is selected and its handles show: the status bar says how to edit it.
    pub(super) fn polyline_selected(&self) -> bool {
        !self.shape_handles().is_empty()
            && self
                .selected_shape()
                .and_then(|id| self.shape(id))
                .is_some_and(|s| matches!(s.geometry, chipload_job::Geometry::Polyline { .. }))
    }

    /// Where the cursor at `at` lands (snap point, else grid; exact with Alt), remembered
    /// for the marker and the status bar. `dragging`: skip the selected shape's own points.
    pub(super) fn snap_world(&mut self, at: Point, free: bool, dragging: bool) -> Point {
        // A dragged box doesn't snap to its own letters or traced shapes.
        let own_box = (dragging && self.dragging_box()).then(|| self.selection.ids(&self.contours));
        let skip: BTreeSet<usize> = match self.selected_shape().filter(|_| dragging) {
            Some(id) => self
                .contours
                .iter()
                .filter(|c| shapes_of(c).contains(&id))
                .map(|c| c.id)
                .collect(),
            None => own_box
                .map(|ids| ids.into_iter().collect())
                .unwrap_or_default(),
        };
        let step = grid_step(self.view.scale, self.units);
        let (p, hit) = snap_cursor(
            at,
            &self.drag.snap_points,
            &skip,
            self.view.scale,
            step,
            self.units,
            free,
        );
        self.drag.snapped = Some((p, hit));
        self.scene.snap_mark = hit.map(|h| h.at);
        p
    }

    pub fn start_handle_drag(&mut self, handle: Handle) {
        let Some(shape) = self.selected_shape().and_then(|id| self.shape(id)) else {
            return;
        };
        self.drag.handle = Some(HandleDrag {
            handle,
            layer: shape.layer.clone(),
            start: shape.geometry.clone(),
        });
    }

    /// Move the dragged handle to `to`. A position that would make the shape invalid
    /// leaves it as it was last valid; the form says why.
    pub fn drag_handle_to(&mut self, to: Point, bend: bool) {
        let Some(drag) = self.drag.handle.clone() else {
            return;
        };
        let refused = match try_drag(&drag.start, drag.handle, [to.0, to.1], bend) {
            Ok(geometry) => self.set_selected_shape(&drag.layer, geometry).err(),
            Err(why) => Some(why),
        };
        let shape = self.selected_shape().and_then(|id| self.shape(id)).cloned();
        if let Some(shape) = shape {
            let mut draft = ShapeDraft::of(&shape);
            draft.error = refused;
            self.shape_draft = Some(draft);
        }
    }

    pub fn end_handle_drag(&mut self) {
        if self.drag.handle.take().is_some()
            && let Some(draft) = self.shape_draft.as_mut()
        {
            draft.error = None;
        }
        self.scene.snap_mark = None;
    }

    // -- start points (docs/specs/helix-and-leads.md part A)

    /// The focused operation's start marker at `pos`, in top view.
    pub(super) fn start_marker_at(&self, pos: egui::Pos2) -> Option<usize> {
        let Focus::Op(operation) = self.focus else {
            return None;
        };
        if self.three_d || self.sketch.is_some() || self.scene.hide_toolpath {
            return None;
        }
        self.scene
            .start_markers
            .iter()
            .enumerate()
            .filter(|(_, m)| m.operation == operation)
            .map(|(i, m)| (i, self.view.to_screen(m.at).distance(pos)))
            .filter(|&(_, d)| d <= crate::view::PICK_TOLERANCE_PX + 2.0)
            .min_by_key(|&(_, d)| Ordered(f64::from(d)))
            .map(|(i, _)| i)
    }

    /// Move a start marker along its ring, towards `to` (drawing coordinates).
    pub fn drag_start_to(&mut self, marker: usize, to: Point) {
        self.drag.start = Some(marker);
        if let Some(m) = self.scene.start_markers.get_mut(marker) {
            m.move_to(to);
        }
    }

    /// Store where the dragged marker was left as the ring's start point, and generate
    /// again so the toolpath starts there.
    pub fn drop_start(&mut self) {
        let Some(marker) = self.drag.start.take() else {
            return;
        };
        let Some(m) = self.scene.start_markers.get(marker).cloned() else {
            return;
        };
        let Some(op) = self.edit.as_mut().and_then(|e| e.ops.get_mut(m.operation)) else {
            return;
        };
        let point = [m.at.0, m.at.1];
        let Some(starts) = op.op.starts_mut() else {
            return;
        };
        match m.stored.filter(|&k| k < starts.len()) {
            Some(k) => starts[k] = point,
            None => starts.push(point),
        }
        self.commit();
        self.generate();
    }

    /// Forget the point that set this marker's ring start, and generate again.
    pub fn reset_start(&mut self, marker: usize) {
        let Some(m) = self.scene.start_markers.get(marker).cloned() else {
            return;
        };
        let (Some(k), Some(op)) = (
            m.stored,
            self.edit.as_mut().and_then(|e| e.ops.get_mut(m.operation)),
        ) else {
            return;
        };
        let Some(starts) = op.op.starts_mut() else {
            return;
        };
        if k < starts.len() {
            starts.remove(k);
        }
        self.commit();
        self.generate();
    }

    /// "Operation 2 (Pocket 1): plunges 3, 13, 23 of 30", one line per operation.
    pub(super) fn plunge_text(&self, hovered: &[usize]) -> String {
        let Some(path) = &self.scene.toolpath else {
            return String::new();
        };
        let mut by_op: std::collections::BTreeMap<Option<usize>, Vec<usize>> =
            std::collections::BTreeMap::new();
        for &i in hovered {
            let op = path.starts[i].operation;
            // Number within its operation, 1-based.
            let n = path.starts[..=i]
                .iter()
                .filter(|s| s.operation == op)
                .count();
            by_op.entry(op).or_default().push(n);
        }
        by_op
            .into_iter()
            .map(|(op, numbers)| {
                let total = path.starts.iter().filter(|s| s.operation == op).count();
                let label = op.map_or_else(
                    || t!(self, "canvas-rejected-toolpath"),
                    |i| self.op_label(i),
                );
                let list: Vec<String> = numbers.iter().map(ToString::to_string).collect();
                t!(
                    self,
                    "canvas-plunges",
                    operation = label,
                    count = numbers.len(),
                    numbers = list.join(", "),
                    total = total.to_string()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(super) fn op_label(&self, index: usize) -> String {
        let name = self
            .edit
            .as_ref()
            .and_then(|e| e.ops.get(index))
            .map(|e| e.op.name().to_string())
            .unwrap_or_default();
        chipload_job::operation_label_message(index, &name).render(self.language, self.units.into())
    }
}
