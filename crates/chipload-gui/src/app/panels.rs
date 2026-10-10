// SPDX-License-Identifier: GPL-3.0-or-later
//! The side panels: operations, selection (with the shape editor), properties, report.

use super::*;

impl ChiploadApp {
    /// The selection section's shape part: the editor for a drawn shape, facts for an
    /// imported one.
    pub(super) fn selected_shape_ui(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_shape() else {
            self.shape_draft = None;
            return;
        };
        let Some(shape) = self.shape(id).cloned() else {
            return;
        };
        if !shape.is_drawn() {
            for line in imported_lines(&shape) {
                ui.label(line);
            }
            return;
        }
        // Start from the stored shape unless the user is fixing values that were refused.
        let keep = self
            .shape_draft
            .as_ref()
            .is_some_and(|d| d.id == id && d.error.is_some());
        if !keep {
            self.shape_draft = Some(ShapeDraft::of(&shape));
        }
        let Some(mut draft) = self.shape_draft.take() else {
            return;
        };
        let action = ui
            .add_enabled_ui(self.can_draw(), |ui| shape_form(ui, &mut draft))
            .inner;
        if action == ShapeAction::Delete {
            self.delete_selected_shape();
            return;
        }
        if draft.differs_from(&shape) {
            draft.error = self
                .set_selected_shape(&draft.layer.clone(), draft.geometry.clone())
                .err();
        }
        self.shape_draft = Some(draft);
    }

    /// Left: operations in cut order, the canvas selection, layers.
    pub(super) fn operations_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Project");
        if self.edit.is_none() {
            ui.label("No project open.");
            if ui.button("Import DXF…").clicked() {
                self.ask_import();
            }
            if ui.button("Open…").clicked() {
                self.ask_open();
            }
            return;
        }
        self.operation_list(ui);
        ui.separator();
        ui.heading("Selection");
        for line in self.selection_lines() {
            ui.label(line);
        }
        self.selected_shape_ui(ui);
        ui.horizontal(|ui| {
            if ui
                .button("New profile")
                .on_hover_text("A profile operation for the selected contours")
                .clicked()
            {
                self.add_operation(OpKind::Profile);
            }
            if ui
                .button("New pocket")
                .on_hover_text("A pocket operation for the selected contours")
                .clicked()
            {
                self.add_operation(OpKind::Pocket);
            }
            if ui
                .add_enabled(
                    self.selector_json().is_some(),
                    egui::Button::new("Copy as selector"),
                )
                .clicked()
            {
                self.copy_selector(ui.ctx());
            }
        });
        ui.collapsing("Layers", |ui| {
            let mut layers: Vec<(String, usize)> = Vec::new();
            for c in &self.contours {
                for layer in c.layers() {
                    match layers.iter_mut().find(|(l, _)| *l == layer) {
                        Some((_, n)) => *n += 1,
                        None => layers.push((layer, 1)),
                    }
                }
            }
            layers.sort();
            for (layer, count) in layers {
                if ui
                    .button(format!("Select layer {layer} ({count})"))
                    .clicked()
                {
                    self.select_layer(&layer);
                }
            }
        });
    }

    pub(super) fn operation_list(&mut self, ui: &mut egui::Ui) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        if ui
            .selectable_label(
                self.focus == Focus::Job,
                "Job settings (stock, machine, tool)",
            )
            .clicked()
        {
            self.focus = Focus::Job;
        }
        let missing: BTreeSet<usize> = edit.missing().into_iter().map(|(i, _)| i).collect();
        let mut moved: Option<(usize, usize)> = None;
        let count = edit.ops.len();
        for i in 0..count {
            let entry = &mut edit.ops[i];
            let summary = geometry_summary(&entry.op, &self.contours);
            let kind = match &entry.op {
                Operation::Profile(_) => "Profile",
                Operation::Pocket(_) => "Pocket",
            };
            let flagged = missing.contains(&i) || self.error_ops.contains(&i);
            let row = ui.horizontal(|ui| {
                ui.dnd_drag_source(egui::Id::new(("operation", i)), i, |ui| {
                    ui.label("≡")
                        .on_hover_text("Drag onto another operation to move it there");
                });
                let mut enabled = entry.op.enabled();
                if ui
                    .add(egui::Checkbox::without_text(&mut enabled))
                    .on_hover_text("Enabled")
                    .changed()
                {
                    match &mut entry.op {
                        Operation::Profile(p) => p.enabled = enabled,
                        Operation::Pocket(p) => p.enabled = enabled,
                    }
                }
                let (start, end) = entry.op.depths();
                let mut text = RichText::new(format!(
                    "{}. {kind}: {}  [{summary}]  {start}–{end} mm",
                    i + 1,
                    entry.op.name()
                ));
                if !enabled {
                    text = text.weak();
                }
                if flagged {
                    text = text.color(egui::Color32::from_rgb(0xf2, 0x8b, 0x82));
                }
                if ui
                    .selectable_label(self.focus == Focus::Op(i), text)
                    .clicked()
                {
                    self.focus = Focus::Op(i);
                }
            });
            if let Some(from) = row.response.dnd_release_payload::<usize>() {
                moved = Some((*from, i));
            } else if row.response.dnd_hover_payload::<usize>().is_some() {
                ui.painter().hline(
                    row.response.rect.x_range(),
                    row.response.rect.top(),
                    egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                );
            }
        }
        let focused = match self.focus {
            Focus::Op(i) if i < count => Some(i),
            _ => None,
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(focused.is_some_and(|i| i > 0), egui::Button::new("↑"))
                .on_hover_text("Move up (Alt+Up)")
                .clicked()
                && let Some(i) = focused
            {
                moved = Some((i, i - 1));
            }
            if ui
                .add_enabled(
                    focused.is_some_and(|i| i + 1 < count),
                    egui::Button::new("↓"),
                )
                .on_hover_text("Move down (Alt+Down)")
                .clicked()
                && let Some(i) = focused
            {
                moved = Some((i, i + 1));
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Duplicate"))
                .clicked()
            {
                self.duplicate_operation();
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Delete"))
                .clicked()
            {
                self.delete_operation();
            }
        });
        if let Some((from, to)) = moved {
            self.move_operation(from, to);
        }
    }

    /// Right: the selected operation's fields, or the project settings.
    pub(super) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        let Some(edit) = self.edit.as_mut() else {
            ui.label("No project open.");
            return;
        };
        let focus = self.focus;
        let contours = &self.contours;
        let library = self.settings_dir.as_ref().map(|_| &self.library);
        let mut use_selection = false;
        let mut request = None;
        ui.scope(|ui| match focus {
            Focus::Job => request = job_form(ui, edit, library),
            Focus::Op(i) => {
                let tool_diameter = edit.settings.tools.first().map_or(0.0, |t| t.diameter);
                if let Some(entry) = edit.ops.get_mut(i) {
                    use_selection = op_form(ui, i, entry, contours, tool_diameter);
                }
            }
        });
        if use_selection {
            self.use_selection();
        }
        if let Some(request) = request {
            self.library_request(request);
        }
    }

    pub(super) fn report_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Report");
        ScrollArea::both()
            .id_salt("report")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(self.report.join("\n")).monospace()).extend(),
                );
            });
    }
}
