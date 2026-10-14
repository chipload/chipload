// SPDX-License-Identifier: GPL-3.0-or-later
//! The side panels' contents: the selected shape's editor, the properties panel and the
//! start screen. The Browser is in browser.rs.

use super::forms::{JobRequest, OpFormRequest, ToolChoice};

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

    /// Right: the selected operation's fields, or the project settings.
    pub(super) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        if self.focus.is_library() {
            self.library_properties(ui);
            return;
        }
        if self.edit.is_none() {
            ui.weak("No project open.");
            return;
        }
        self.problems_strip(ui);
        if !self.selection.ids(&self.contours).is_empty() {
            self.selection_section(ui);
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
        }
        problems::set_form_problems(ui, self.form_problem_map(self.focus));
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let focus = self.focus;
        let contours = &self.contours;
        let library = self.settings_dir.as_ref().map(|_| &self.library);
        let mut op_request = OpFormRequest::default();
        let mut request = None;
        let cutting = match focus {
            Focus::Op(i) => edit
                .ops
                .get(i)
                .and_then(|e| self_tool_cutting(library, &edit.settings.tools, e.op.tool())),
            _ => None,
        };
        ui.scope(|ui| match focus {
            Focus::Job => request = job_form(ui, edit, library),
            Focus::Part(id) => request = forms::part_form(ui, edit, id),
            Focus::Tools => request = forms::tools_form(ui, edit, library),
            Focus::Tool(id) => request = forms::tool_form(ui, edit, library, id),
            Focus::Dxf(id) => request = forms::dxf_form(ui, edit, id),
            Focus::Image(id) => request = forms::image_form(ui, edit, id),
            // Drawn by `library_properties` above.
            Focus::LibraryTools
            | Focus::LibraryTool(_)
            | Focus::LibraryMachines
            | Focus::LibraryMachine(_)
            | Focus::Preferences
            | Focus::Posts
            | Focus::Post(_) => {}
            Focus::Op(i) => {
                let choice = ToolChoice {
                    tools: &edit.settings.tools,
                    library: library
                        .map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect()),
                    cutting,
                };
                if let Some(entry) = edit.ops.get_mut(i) {
                    op_request = op_form(ui, i, entry, contours, choice);
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
        match request {
            Some(JobRequest::Library(request)) => self.library_request(request),
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
                    .add_filter("DXF drawings", &["dxf"])
                    .pick_file()
                {
                    self.update_dxf_from(Some(dxf), &path, None);
                }
            }
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
    let tool = tools.iter().find(|t| t.id == id)?;
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
                    ui.weak("2.5D CAM: drawings in, checked G-code out");
                    ui.add_space(16.0);
                    let wide = egui::vec2(START_WIDTH, 32.0);
                    if ui
                        .add(egui::Button::new("New project").min_size(wide))
                        .on_hover_text("An empty project: draw shapes or import a DXF later")
                        .clicked()
                    {
                        self.ask_new();
                    }
                    if ui
                        .add(egui::Button::new("Import DXF…").min_size(wide))
                        .on_hover_text("A new project from a drawing (Ctrl+I)")
                        .clicked()
                    {
                        self.ask_import();
                    }
                    if ui
                        .add(egui::Button::new("Open project…").min_size(wide))
                        .on_hover_text("Ctrl+O")
                        .clicked()
                    {
                        self.ask_open();
                    }
                    let recent: Vec<String> =
                        self.recent.iter().take(START_RECENT).cloned().collect();
                    if !recent.is_empty() {
                        ui.add_space(16.0);
                        ui.label(RichText::new("Recent").strong());
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
                        RichText::new(
                            "⚠ Machines are dangerous. Check every program in a G-code viewer \
                             and with an air cut before cutting material.",
                        )
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
