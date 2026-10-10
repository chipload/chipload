// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: input, painting and hover texts.

use super::*;

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
        if response.dragged() {
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
            let step = grid_step(self.view.scale);
            if response.double_clicked() {
                self.finish_drawing();
            } else if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let first = self.sketch.as_ref().and_then(|s| s.points.first().copied());
                let on_first = first.is_some_and(|f| {
                    self.view.to_screen(f).distance(pos) <= crate::view::PICK_TOLERANCE_PX
                });
                self.draw_click(snap(self.view.to_world(pos), step), on_first);
            }
            self.scene.preview = match (&self.sketch, self.cursor) {
                (Some(sketch), Some(at)) => sketch.preview(snap(at, step)),
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
