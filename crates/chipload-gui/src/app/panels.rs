// SPDX-License-Identifier: GPL-3.0-or-later
//! The side panels' contents: the selected shape's editor, the properties panel and the
//! start screen. The Browser is in browser.rs.

use super::forms::{JobRequest, OpFormRequest, ToolChoice};

use super::*;
use chipload_job::find_tool;

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
            for line in imported_lines(&shape, ui) {
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
        draft.grab_name |= std::mem::take(&mut self.grab_shape_name);
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
        if draft.error.is_none() && draft.name_differs_from(&shape) {
            draft.error = self.name_shape(id, &draft.name.clone()).err();
        }
        self.shape_draft = Some(draft);
    }

    /// Position and Turn for several selected drawn shapes together, about the box round
    /// them (docs/specs/drawn-shapes-box.md §3); one shape has them in its own form.
    pub(super) fn selected_shapes_place_ui(&mut self, ui: &mut egui::Ui) {
        let (ids, imported) = self.deletable_shapes();
        if imported || ids.len() < 2 || !self.can_draw() {
            return;
        }
        let Some(bounds) = self
            .edit
            .as_ref()
            .and_then(|e| chipload_pipeline::shapes::drawn_box(&e.settings, &ids))
        else {
            return;
        };
        let edit = egui::Grid::new("selection-place")
            .num_columns(2)
            .show(ui, |ui| {
                crate::shape_form::place_rows(ui, egui::Id::new("selection-place"), bounds)
            })
            .inner;
        if let Some(edit) = edit {
            let (from, to, scale, degrees) = edit.transform(bounds);
            self.transform_shapes(&ids, from, to, scale, degrees);
        }
    }

    /// Right: the selected operation's fields, or the project settings.
    pub(super) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        if self.focus.is_library() {
            self.library_properties(ui);
            return;
        }
        if self.edit.is_none() {
            ui.weak(t!(ui, "panels-no-project"));
            return;
        }
        forms::set_form_width(ui);
        self.problems_strip(ui);
        // One thing at a time (the user, 2026-10-28): what is picked on the canvas, or the
        // Browser's item.
        if self.focus == Focus::Selection {
            if self.selection.ids(&self.contours).is_empty() {
                ui.weak(t!(ui, "panels-nothing-selected"));
                // An array tool waits for shapes: its values and Cancel stay at hand.
                self.array_form(ui);
            } else {
                self.selection_section(ui);
            }
            return;
        }
        problems::set_form_problems(ui, self.form_problem_map(self.focus));
        let selected = self.selection.ids(&self.contours).len();
        let selection_text = self.selection_lines().join("\n");
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let focus = self.focus;
        // A text just placed: its text field takes the keyboard, once.
        let grab = std::mem::take(&mut self.focus_text_field);
        let contours = &self.contours;
        let library = self.settings_dir.as_ref().map(|_| &self.library);
        let mut op_request = OpFormRequest::default();
        let mut request = None;
        // The project's material, and its values for the open operation's tool.
        let material = match (focus, library) {
            (Focus::Op(i), Some(library)) => {
                chipload_pipeline::materials::project_material(&edit.settings, &library.materials)
                    .map(|m| {
                        let values = edit.ops.get(i).and_then(|e| {
                            chipload_pipeline::materials::material_values(
                                &edit.settings,
                                &library.materials,
                                e.op.tool(),
                            )
                        });
                        (m.name.clone(), values)
                    })
            }
            _ => None,
        };
        let cutting = match focus {
            Focus::Op(i) => edit
                .ops
                .get(i)
                .and_then(|e| self_tool_cutting(library, &edit.settings.tools, e.op.tool())),
            _ => None,
        };
        ui.scope(|ui| match focus {
            // Shown above, alone.
            Focus::Selection => {}
            Focus::Job => request = job_form(ui, edit, library),
            Focus::Part(id) => request = forms::part_form(ui, edit, id),
            Focus::Tools => request = forms::tools_form(ui, edit, library),
            Focus::Tool(id) => request = forms::tool_form(ui, edit, library, id),
            Focus::Dxf(id) => request = forms::dxf_form(ui, edit, id),
            Focus::Image(id) => request = forms::image_form(ui, edit, id),
            Focus::Text(id) => request = forms::text_form(ui, edit, id, grab),
            Focus::RestPass(i, k) => request = forms::rest_pass_form(ui, edit, i, k),
            // Drawn by `library_properties` above.
            Focus::LibraryTools
            | Focus::LibraryTool(_)
            | Focus::LibraryMachines
            | Focus::LibraryMachine(_)
            | Focus::LibraryMaterials
            | Focus::LibraryMaterial(_)
            | Focus::Preferences
            | Focus::Posts
            | Focus::Post(_) => {}
            Focus::Op(i) => {
                let choice = ToolChoice {
                    selected,
                    selection_text: &selection_text,
                    tools: &edit.settings.tools,
                    library: library
                        .map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect()),
                    cutting,
                    material,
                };
                let through = edit.settings.stock.thickness + edit.settings.stock.max_overcut;
                if let Some(entry) = edit.ops.get_mut(i) {
                    op_request = op_form(ui, i, entry, contours, choice, through);
                }
            }
        });
        // The map was for this form only (library windows draw the same rows).
        problems::set_form_problems(ui, std::collections::BTreeMap::new());
        if op_request.use_selection {
            self.use_selection();
        }
        if let (Some(name), Focus::Op(i)) = (op_request.tool_from_library, focus) {
            self.tool_from_library(i, &name);
        }
        if let (true, Focus::Op(i)) = (op_request.save_to_material, focus) {
            self.save_op_to_material(i);
        }
        match request {
            Some(JobRequest::Library(request)) => self.library_request(request),
            Some(JobRequest::UseMaterial(uid)) => {
                self.use_material(uid.as_deref());
            }
            Some(JobRequest::OpenOperation(i)) => self.focus = Focus::Op(i),
            Some(JobRequest::RemoveRestPass(i, k)) => self.remove_rest_pass(i, k),
            Some(JobRequest::CopiesGap(part, gap)) => self.set_copies_gap(part, gap),
            Some(JobRequest::Activate(part)) => self.activate_part(part),
            Some(JobRequest::Post(name)) => {
                self.choose_project_post(&name);
            }
            Some(JobRequest::Place(dxf, how)) => {
                self.place_dxf(Some(dxf), how);
            }
            Some(JobRequest::ToolRemoved) => self.focus = Focus::Tools,
            Some(JobRequest::UpdateDxf(dxf)) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(t!(self, "panels-dxf-filter"), DRAWING_FILES)
                    .pick_file()
                {
                    self.update_dxf_from(Some(dxf), &path, None);
                }
            }
            Some(JobRequest::ExplodeDxf(dxf)) => self.ask_explode(dxf),
            Some(JobRequest::RemoveDxf(dxf)) if self.remove_dxf(dxf) => {
                self.focus = Focus::Job;
            }
            Some(JobRequest::TraceSettings(image)) => {
                self.edit_image(image);
            }
            Some(JobRequest::ImagePlace(image, place)) => {
                self.set_image_place(image, &place);
            }
            Some(JobRequest::RemoveImage(image)) if self.remove_image(image) => {
                self.focus = Focus::Job;
            }
            Some(JobRequest::TextChange(id, text)) => {
                self.set_text(id, &text);
            }
            Some(JobRequest::RemoveText(id)) => {
                self.remove_text(id);
            }
            Some(JobRequest::RemoveDxf(_) | JobRequest::RemoveImage(_)) | None => {}
        }
    }
}

/// A project tool's cutting values from the library: only when the tool came from the
/// library and still has its values (as `ChiploadApp::tool_cutting`).
fn self_tool_cutting(
    library: Option<&chipload_pipeline::library::Library>,
    tools: &[chipload_job::Tool],
    id: i64,
) -> Option<chipload_job::library::Cutting> {
    let tool = find_tool(tools, id)?;
    chipload_job::library::linked_cutting(&library?.tools, tool)
}

impl ChiploadApp {
    /// With no project open, the canvas offers the ways to start one, and recent projects.
    pub(super) fn start_screen(&mut self, ui: &mut egui::Ui) {
        // The canvas's own background, so the window doesn't change colour on opening.
        ui.painter().rect_filled(
            ui.max_rect(),
            0.0,
            crate::view::Palette::of(ui.visuals().dark_mode).background,
        );
        ui.add_space((ui.available_height() * 0.18).max(16.0));
        ui.vertical_centered(|ui| {
            egui::Frame::new()
                .fill(ui.visuals().panel_fill)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(8.0)
                .inner_margin(24.0)
                .show(ui, |ui| {
                    ui.set_max_width(START_WIDTH);
                    ui.heading(RichText::new("Chipload").size(28.0).strong());
                    ui.weak(t!(ui, "panels-tagline"));
                    ui.add_space(16.0);
                    let wide = egui::vec2(START_WIDTH, 32.0);
                    if ui
                        .add(egui::Button::new(t!(ui, "panels-new-project")).min_size(wide))
                        .on_hover_text(t!(ui, "panels-new-project-hover"))
                        .clicked()
                    {
                        self.ask_new();
                    }
                    if ui
                        .add(egui::Button::new(t!(ui, "panels-import-dxf")).min_size(wide))
                        .on_hover_text(t!(ui, "panels-import-dxf-hover"))
                        .clicked()
                    {
                        self.ask_import();
                    }
                    if ui
                        .add(egui::Button::new(t!(ui, "panels-open-project")).min_size(wide))
                        .on_hover_text("Ctrl+O")
                        .clicked()
                    {
                        self.ask_open();
                    }
                    let recent: Vec<String> =
                        self.recent.iter().take(START_RECENT).cloned().collect();
                    if !recent.is_empty() {
                        ui.add_space(16.0);
                        ui.label(RichText::new(t!(ui, "panels-recent")).strong());
                        for path in &recent {
                            let name = file_name(Path::new(path));
                            if ui
                                .add(egui::Button::new(name).frame(false))
                                .on_hover_text(path)
                                .clicked()
                            {
                                self.open_recent(path);
                            }
                        }
                    }
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(format!("⚠ {}", t!(ui, "panels-danger")))
                            .small()
                            .color(ui.visuals().warn_fg_color),
                    );
                });
        });
    }
}

/// Width of the start screen's card.
const START_WIDTH: f32 = 320.0;
/// Recent projects shown on the start screen (all are under File > Open recent).
const START_RECENT: usize = 6;
