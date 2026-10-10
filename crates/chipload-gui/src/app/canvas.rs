// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: input, painting and hover texts.

use super::*;

/// How far beyond a rectangle's end its rotation handle sits, in screen pixels.
const ROTATE_HANDLE_PX: f64 = 24.0;

impl ChiploadApp {
    /// Switch to the 3D view, starting from a three-quarter view (docs/specs/3d-view.md).
    pub fn view_3d(&mut self) {
        if !self.three_d {
            self.three_d = true;
            self.sketch = None; // drawing works in top view only
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

    /// What the 3D view turns around: the stock's centre, else the drawing origin.
    fn orbit_pivot(&self) -> Point {
        self.scene.stock.map_or((0.0, 0.0), |((x0, y0), (x1, y1))| {
            ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
        })
    }

    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        self.scene.work_zero = self
            .edit
            .as_ref()
            .map_or((0.0, 0.0), |e| e.settings.work_zero_xy());
        // Copies of the part, faded (docs/specs/copies-grid.md); copy 1 is the drawing.
        self.scene.copies = self
            .edit
            .as_ref()
            .and_then(|e| e.settings.copies)
            .map(|c| {
                let mut shifts: Vec<Point> = c.in_cut_order().into_iter().map(|(_, s)| s).collect();
                shifts.retain(|&s| s != (0.0, 0.0));
                shifts
            })
            .unwrap_or_default();
        // Heights for the 3D view: 2D items sit on the stock top (docs/specs/3d-view.md).
        if let Some(edit) = &self.edit {
            let stock = &edit.settings.stock;
            let top = match stock.z_zero {
                ZZero::Top => 0.0,
                ZZero::Bottom => stock.thickness,
            };
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
        let shape_handles = self.shape_handles();
        self.scene.handles = shape_handles.iter().map(|&(_, p)| p).collect();
        let view = self.view;
        let on_handle = |pos: egui::Pos2| -> Option<Handle> {
            shape_handles
                .iter()
                .map(|&(h, p)| (h, view.to_screen(p).distance(pos)))
                .filter(|&(_, d)| d <= crate::view::PICK_TOLERANCE_PX)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(h, _)| h)
        };
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            && let Some(handle) = on_handle(origin)
        {
            self.start_handle_drag(handle);
        }
        if response.drag_stopped() {
            self.end_handle_drag();
        }
        self.snapped = None;
        self.scene.snap_mark = None;
        if self.handle_drag.is_some() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let to = self.snap_world(self.view.to_world(pos), alt, true);
                self.drag_handle_to(to, shift);
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
        } else {
            self.scene.preview.clear();
            self.scene.sketch_points.clear();
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && on_handle(pos).is_none()
            {
                let toggle = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                let picked = self.scene.pick(&self.view, pos);
                self.click(picked, toggle);
            }
        }
        if response.secondary_clicked() {
            self.context_pick = response
                .interact_pointer_pos()
                .and_then(|pos| self.scene.pick(&self.view, pos));
        }
        let picked = self
            .context_pick
            .and_then(|id| self.contours.iter().find(|c| c.id == id));
        let layers: Vec<String> = picked.map(Contour::layers).unwrap_or_default();
        let picked_id = picked.map(|c| c.id);
        response.context_menu(|ui| match picked_id {
            Some(id) => {
                if ui.button(format!("Select #{id}")).clicked() {
                    self.click(Some(id), false);
                    ui.close();
                }
                for layer in &layers {
                    if ui.button(format!("Select layer {layer}")).clicked() {
                        self.select_layer(layer);
                        ui.close();
                    }
                }
            }
            None => {
                ui.label("Right-click a contour to select its layer.");
            }
        });
        let operation = match self.focus {
            Focus::Op(i) => Some(i),
            _ => None,
        };
        let plunges = response
            .hover_pos()
            .map(|pos| self.scene.plunges_at(&self.view, pos))
            .unwrap_or_default();
        if !plunges.is_empty() && !response.dragged() {
            let text = self.plunge_text(&plunges);
            response.on_hover_text_at_pointer(text);
        } else if self.show_material_left
            && !response.dragged()
            && let Some(at) = self.cursor
            && let Some(piece) = self.scene.left_at(at, operation)
        {
            let text = self.material_left_text(piece);
            response.on_hover_text_at_pointer(text);
        }
        let highlight = Highlight {
            selected: self.selection.ids(&self.contours),
            operation_contours: self.focused_contours(),
            operation,
            show_material_left: self.show_material_left,
        };
        self.scene.paint(&painter, &self.view, &highlight);
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

    /// Where the cursor at `at` lands (snap point, else grid; exact with Alt), remembered
    /// for the marker and the status bar. `dragging`: skip the selected shape's own points.
    pub(super) fn snap_world(&mut self, at: Point, free: bool, dragging: bool) -> Point {
        let skip: BTreeSet<usize> = match self.selected_shape().filter(|_| dragging) {
            Some(id) => self
                .contours
                .iter()
                .filter(|c| shapes_of(c).contains(&id))
                .map(|c| c.id)
                .collect(),
            None => BTreeSet::new(),
        };
        let step = grid_step(self.view.scale);
        let (p, hit) = snap_cursor(at, &self.snap_points, &skip, self.view.scale, step, free);
        self.snapped = Some((p, hit));
        self.scene.snap_mark = hit.map(|h| h.at);
        p
    }

    pub fn start_handle_drag(&mut self, handle: Handle) {
        let Some(shape) = self.selected_shape().and_then(|id| self.shape(id)) else {
            return;
        };
        self.handle_drag = Some(HandleDrag {
            handle,
            layer: shape.layer.clone(),
            start: shape.geometry.clone(),
        });
    }

    /// Move the dragged handle to `to`. A position that would make the shape invalid
    /// leaves it as it was last valid; the form says why.
    pub fn drag_handle_to(&mut self, to: Point, bend: bool) {
        let Some(drag) = self.handle_drag.clone() else {
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
        if self.handle_drag.take().is_some()
            && let Some(draft) = self.shape_draft.as_mut()
        {
            draft.error = None;
        }
        self.scene.snap_mark = None;
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
                let label =
                    op.map_or_else(|| "Rejected toolpath".to_string(), |i| self.op_label(i));
                let list: Vec<String> = numbers.iter().map(ToString::to_string).collect();
                let noun = if numbers.len() == 1 {
                    "plunge"
                } else {
                    "plunges"
                };
                format!("{label}: {noun} {} of {total}", list.join(", "))
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
        match name.as_str() {
            "" => format!("Operation {}", index + 1),
            name => format!("Operation {} ({name})", index + 1),
        }
    }

    pub(super) fn material_left_text(&self, piece: &LeftShape) -> String {
        let label = self.op_label(piece.operation);
        let what = match piece.kind {
            LeftKind::Unreachable => "material the tool can't reach",
            LeftKind::Missed => "material the toolpath missed (see the warning)",
        };
        format!(
            "{label}: {what}, about {:.2} x {:.2} mm",
            piece.size.0, piece.size.1
        )
    }
}
