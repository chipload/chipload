// SPDX-License-Identifier: GPL-3.0-or-later
//! The side panels: operations, selection (with the shape editor), properties.

use super::forms::JobRequest;
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
        let nothing = self.selection.ids(&self.contours).is_empty();
        for line in self.selection_lines() {
            if nothing {
                ui.weak(line);
            } else {
                ui.label(line);
            }
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
                "⚙ Job settings (stock, machine, tool)",
            )
            .clicked()
        {
            self.focus = Focus::Job;
        }
        ui.add_space(2.0);
        let missing: BTreeSet<usize> = edit.missing().into_iter().map(|(i, _)| i).collect();
        let mut action: Option<CardAction> = None;
        let count = edit.ops.len();
        for i in 0..count {
            let entry = &mut edit.ops[i];
            let flagged = missing.contains(&i) || self.error_ops.contains(&i);
            let selected = self.focus == Focus::Op(i);
            let card = op_card(ui, i, entry, &self.contours, selected, flagged);
            let response = card.response;
            if response.clicked() {
                action = Some(CardAction::Focus(i));
            }
            response.context_menu(|ui| {
                if ui
                    .add_enabled(i > 0, egui::Button::new("Move up"))
                    .clicked()
                {
                    action = Some(CardAction::Move(i, i - 1));
                }
                if ui
                    .add_enabled(i + 1 < count, egui::Button::new("Move down"))
                    .clicked()
                {
                    action = Some(CardAction::Move(i, i + 1));
                }
                if ui.button("Duplicate").clicked() {
                    action = Some(CardAction::Duplicate(i));
                }
                if ui.button("Delete").clicked() {
                    action = Some(CardAction::Delete(i));
                }
            });
            if let Some(from) = response.dnd_release_payload::<usize>() {
                action = Some(CardAction::Move(*from, i));
            } else if response.dnd_hover_payload::<usize>().is_some() {
                ui.painter().hline(
                    response.rect.x_range(),
                    response.rect.top() - 1.0,
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
                action = Some(CardAction::Move(i, i - 1));
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
                action = Some(CardAction::Move(i, i + 1));
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Duplicate"))
                .clicked()
                && let Some(i) = focused
            {
                action = Some(CardAction::Duplicate(i));
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Delete"))
                .clicked()
                && let Some(i) = focused
            {
                action = Some(CardAction::Delete(i));
            }
        });
        match action {
            Some(CardAction::Focus(i)) => self.focus = Focus::Op(i),
            Some(CardAction::Move(from, to)) => self.move_operation(from, to),
            Some(CardAction::Duplicate(i)) => {
                self.focus = Focus::Op(i);
                self.duplicate_operation();
            }
            Some(CardAction::Delete(i)) => {
                self.focus = Focus::Op(i);
                self.delete_operation();
            }
            None => {}
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
        match request {
            Some(JobRequest::Library(request)) => self.library_request(request),
            Some(JobRequest::CopiesGap(gap)) => self.set_copies_gap(gap),
            None => {}
        }
    }
}

/// What a click in the operation list asks for, done after the list is drawn.
enum CardAction {
    Focus(usize),
    Move(usize, usize),
    Duplicate(usize),
    Delete(usize),
}

/// The kind's tag colour: pockets and profiles told apart at a glance.
fn kind_color(op: &Operation, dark: bool) -> egui::Color32 {
    match (op, dark) {
        (Operation::Profile(_), true) => egui::Color32::from_rgb(0x6f, 0xc8, 0xbd),
        (Operation::Profile(_), false) => egui::Color32::from_rgb(0x00, 0x79, 0x6b),
        (Operation::Pocket(_), true) => egui::Color32::from_rgb(0xc3, 0xa6, 0xf0),
        (Operation::Pocket(_), false) => egui::Color32::from_rgb(0x6a, 0x3d, 0xb8),
    }
}

/// One operation as a card: drag handle, enabled box, number, kind and name; below, its
/// geometry and depths. The whole card is clickable (selects it) and right-clickable.
fn op_card(
    ui: &mut egui::Ui,
    i: usize,
    entry: &mut crate::editor::EditOp,
    contours: &[Contour],
    selected: bool,
    flagged: bool,
) -> egui::InnerResponse<()> {
    let summary = geometry_summary(&entry.op, contours);
    let (start, end) = entry.op.depths();
    let kind = match &entry.op {
        Operation::Profile(_) => "PROFILE",
        Operation::Pocket(_) => "POCKET",
    };
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("op-card", i))
            .sense(Sense::click()),
        |ui| {
            let hovered = ui.response().hovered();
            let visuals = ui.visuals().clone();
            let fill = if selected {
                visuals.selection.bg_fill
            } else if hovered {
                visuals.widgets.hovered.weak_bg_fill
            } else {
                visuals.faint_bg_color
            };
            let text_color = if selected {
                visuals.selection.stroke.color
            } else {
                visuals.text_color()
            };
            egui::Frame::new()
                .fill(fill)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.dnd_drag_source(egui::Id::new(("operation", i)), i, |ui| {
                            ui.label(RichText::new("≡").color(text_color))
                                .on_hover_text("Drag onto another operation to move it there");
                        });
                        let mut enabled = entry.op.enabled();
                        if ui
                            .add(egui::Checkbox::without_text(&mut enabled))
                            .on_hover_text("Enabled: cut this operation")
                            .changed()
                        {
                            match &mut entry.op {
                                Operation::Profile(p) => p.enabled = enabled,
                                Operation::Pocket(p) => p.enabled = enabled,
                            }
                        }
                        ui.label(RichText::new(format!("{}", i + 1)).color(text_color));
                        let tag = if selected {
                            text_color
                        } else {
                            kind_color(&entry.op, visuals.dark_mode)
                        };
                        ui.label(RichText::new(kind).small().strong().color(tag));
                        let name = match entry.op.name() {
                            "" => "(no name)".to_string(),
                            name => name.to_string(),
                        };
                        let mut title = RichText::new(name).strong().color(text_color);
                        if flagged {
                            title = RichText::new(format!("⚠ {}", entry.op.name()))
                                .strong()
                                .color(visuals.error_fg_color);
                        }
                        if !enabled {
                            title = title.weak();
                        }
                        ui.add(egui::Label::new(title).truncate().selectable(false));
                    });
                    let mut detail =
                        RichText::new(format!("{summary}  ·  {start}–{end} mm")).small();
                    detail = if selected {
                        detail.color(text_color)
                    } else {
                        detail.weak()
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.add(egui::Label::new(detail).truncate().selectable(false));
                    });
                });
        },
    )
}
