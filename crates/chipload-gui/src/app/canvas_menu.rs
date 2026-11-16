// SPDX-License-Identifier: GPL-3.0-or-later
//! The canvas's right-click menu and its hover texts.

use super::*;

impl ChiploadApp {
    /// The canvas's right-click menu: tabs, start points, the Compass, selecting, styles.
    pub(super) fn canvas_context_menu(&mut self, response: &egui::Response) {
        // Right-clicking a corner marker opens its menu, not the canvas's.
        let corner_clicked = response
            .secondary_clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|pos| self.corner_marker_at(pos));
        if let Some(k) = corner_clicked {
            self.open_corner_menu(k);
        } else if response.secondary_clicked() {
            let at = response.interact_pointer_pos();
            self.drag.context_start = at.and_then(|pos| self.start_marker_at(pos));
            self.drag.context_tab = at.and_then(|pos| self.tab_marker_at(pos));
            self.drag.context_ring = at.and_then(|pos| self.tab_ring_at(pos));
            self.context_pick = at.and_then(|pos| self.scene.pick(&self.view, pos));
        }
        let context_start = self.drag.context_start;
        let (context_tab, context_ring) = (self.drag.context_tab, self.drag.context_ring);
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
        let compass_item = !self.compass_shown() && self.can_show_compass();
        // A right-click on the Compass's centre or an arrow: its own menu instead
        // (compass.md §15).
        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(part) = self.compass_part_at(pos)
        {
            self.open_compass_pick_menu(part, pos);
        }
        if corner_clicked.is_none() && !self.compass_pick_menu_open() {
            response.context_menu(|ui| {
                // A tab, or the ring a tab can be added to (docs/specs/tabs-by-hand.md §1).
                if let (None, Some(marker)) = (context_start, context_tab) {
                    if ui.button(t!(ui, "canvas-tab-remove")).clicked() {
                        self.remove_tab(marker);
                        ui.close();
                    }
                    if ui.button(t!(ui, "canvas-tab-even")).clicked() {
                        self.even_tabs(marker);
                        ui.close();
                    }
                    return;
                }
                if let (None, Some((ring, at))) = (context_start, context_ring) {
                    if ui.button(t!(ui, "canvas-tab-add")).clicked() {
                        self.add_tab(ring, at);
                        ui.close();
                    }
                    ui.separator();
                }
                // The Compass on the selection (docs/specs/compass.md §3).
                if context_start.is_none() && compass_item {
                    let label = super::shortcuts::with_keys(
                        &t!(ui, "compass-menu-item"),
                        super::shortcuts::Action::Compass,
                    );
                    if ui
                        .button(label)
                        .on_hover_text(t!(ui, "compass-hover"))
                        .clicked()
                    {
                        self.toggle_compass();
                        ui.close();
                    }
                    ui.separator();
                }
                match (context_start, picked_id) {
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
                    (None, Some(id)) => self.contour_menu(ui, id, &layers, from),
                    (None, None) => {
                        ui.label(t!(ui, "canvas-right-click-hint"));
                    }
                }
                // The Q menu does the same in two keys (docs/specs/fewer-clicks.md §2).
                if context_start.is_none() && self.selection != CanvasSelection::Nothing {
                    ui.separator();
                    ui.weak(t!(ui, "quick-hint"));
                }
                // A style for what is selected (docs/specs/styles.md §3.2).
                if context_start.is_none()
                    && self.selection != CanvasSelection::Nothing
                    && !self.user.library.styles.styles.is_empty()
                {
                    ui.separator();
                    let mut act = None;
                    super::header::submenu(ui, t!(ui, "styles-apply-menu"), |ui| {
                        self.style_menu(ui, &mut act);
                    });
                    if let Some(act) = act {
                        self.style_act(act);
                        ui.close();
                    }
                }
                // What can be done with the selection: groups, the shape library.
                if context_start.is_none() && !self.framed_shape_ids().is_empty() {
                    ui.separator();
                    self.selection_menu(ui);
                }
            });
        }
    }

    /// The right-click menu's items for a contour: select it, its layer, similar shapes,
    /// shapes inside it (docs/specs/fewer-clicks.md §3).
    fn contour_menu(
        &mut self,
        ui: &mut egui::Ui,
        id: usize,
        layers: &[String],
        from: Option<chipload_core::chain::ShapeFrom>,
    ) {
        if ui
            .button(t!(ui, "canvas-select-contour", id = id.to_string()))
            .clicked()
        {
            self.click(Some(id), false);
            ui.close();
        }
        for layer in layers {
            if ui
                .button(t!(ui, "canvas-select-layer", layer = layer))
                .clicked()
            {
                self.selection = CanvasSelection::Layer(layer.clone(), from);
                self.focus = Focus::Selection;
                ui.close();
            }
        }
        // Selecting by kind (docs/specs/fewer-clicks.md §3).
        let add = ui.input(|i| i.modifiers.command);
        if ui
            .button(t!(ui, "canvas-select-similar"))
            .on_hover_text(t!(ui, "canvas-select-similar-hover"))
            .clicked()
        {
            self.select_similar(id, add);
            ui.close();
        }
        if self.has_inside(id)
            && ui
                .button(t!(ui, "canvas-select-inside"))
                .on_hover_text(t!(ui, "canvas-select-inside-hover"))
                .clicked()
        {
            self.select_inside(id, add);
            ui.close();
        }
    }

    /// The operation the focus is on, or one of its passes.
    pub(super) fn focused_operation(&self) -> Option<usize> {
        match self.focus {
            Focus::Op(i)
            | Focus::RestPass(i, _)
            | Focus::FloorPass(i)
            | Focus::RoughingPass(i)
            | Focus::SmoothingPass(i) => Some(i),
            _ => None,
        }
    }

    /// Hover texts for a corner marker, a start point, a tab or plunges.
    pub(super) fn canvas_hover_text(&self, ui: &egui::Ui, response: &egui::Response) {
        let response = response.clone();
        let plunges = response
            .hover_pos()
            .map(|pos| self.scene.plunges_at(&self.view, pos))
            .unwrap_or_default();
        let on_start = response
            .hover_pos()
            .and_then(|pos| self.start_marker_at(pos))
            .and_then(|i| self.scene.start_markers.get(i));
        let on_corner = response
            .hover_pos()
            .and_then(|pos| self.corner_marker_at(pos))
            .filter(|_| self.corners_menu_closed());
        if let Some(k) = on_corner.filter(|_| !response.dragged()) {
            let text = self.corner_hover(k);
            response.on_hover_text_at_pointer(text);
        } else if let Some(marker) = on_start.filter(|_| !response.dragged()) {
            let text = match marker.stored {
                Some(_) => t!(ui, "canvas-start-moved-hover"),
                None => t!(ui, "canvas-start-hover"),
            };
            response.on_hover_text_at_pointer(text);
        } else if let Some(k) = response
            .hover_pos()
            .and_then(|pos| self.tab_marker_at(pos))
            .filter(|_| !response.dragged())
        {
            let text = if self.scene.tab_markers[k].tab.by_hand {
                t!(ui, "canvas-tab-hover-placed")
            } else {
                t!(ui, "canvas-tab-hover")
            };
            response.on_hover_text_at_pointer(text);
        } else if !plunges.is_empty() && !response.dragged() {
            let text = self.plunge_text(&plunges);
            response.on_hover_text_at_pointer(text);
        }
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
            .and_then(|e| e.job.operations.get(index))
            .map(|e| e.name().to_string())
            .unwrap_or_default();
        chipload_job::operation_label_message(index, &name)
            .render(self.prefs.language, self.prefs.units.into())
    }
}
