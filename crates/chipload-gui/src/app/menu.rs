// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard shortcuts, the menu bar and the status bar.

use super::shortcuts::{self, Action, hover_keys, with_keys};
use super::*;
use chipload_job::edit::Mirror;

impl ChiploadApp {
    // -- user interface

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        // Not while typing in a text field.
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        self.clipboard_events(ctx);
        for binding in shortcuts::BINDINGS {
            if ctx.input_mut(|i| i.consume_shortcut(&binding.keys)) {
                self.act(binding.action, ctx);
            }
        }
        if self.sketch.is_some() {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                self.finish_drawing();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
                && let Some(sketch) = self.sketch.as_mut()
            {
                sketch.undo_point();
            }
        }
        if self.sketch.is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete)) {
            self.delete_selected_shapes();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.measure.is_some() {
                self.measure = None;
            } else if self.sketch.is_some() {
                self.cancel_drawing();
            } else {
                self.selection = CanvasSelection::Nothing;
            }
        }
    }

    pub(super) fn menu(&mut self, ui: &mut egui::Ui) {
        let has_job = self.edit.is_some();
        let has_result = self.result.is_some();
        MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button(with_keys("New project", Action::New)).clicked() {
                    self.ask_new();
                }
                if ui.button(with_keys("Open…", Action::Open)).clicked() {
                    self.ask_open();
                }
                let recent = self.recent.clone();
                ui.add_enabled_ui(!recent.is_empty(), |ui| {
                    ui.menu_button("Open recent", |ui| {
                        for path in &recent {
                            let name = file_name(Path::new(path));
                            if ui.button(name).on_hover_text(path).clicked() {
                                self.open_recent(path);
                            }
                        }
                        ui.separator();
                        if ui.button("Clear list").clicked() {
                            self.forget_recent(None);
                        }
                    });
                });
                if ui
                    .button(with_keys("Import DXF…", Action::Import))
                    .clicked()
                {
                    self.ask_import();
                }
                if ui
                    .button("Import image…")
                    .on_hover_text("Trace a picture into shapes, for V-carving")
                    .clicked()
                {
                    self.import_image_dialog();
                }
                if ui.button("Open path…").clicked() {
                    self.prompt = Some(Prompt::Path(PathPurpose::Open, String::new()));
                }
                if ui.button("Import image from path…").clicked() {
                    self.prompt = Some(Prompt::Path(PathPurpose::Image, String::new()));
                }
                ui.separator();
                let (update, update_path) = self.update_labels();
                if ui.add_enabled(has_job, egui::Button::new(update)).clicked() {
                    self.ask_update();
                }
                if ui
                    .add_enabled(has_job, egui::Button::new(update_path))
                    .clicked()
                {
                    self.prompt = Some(Prompt::Path(PathPurpose::Update, String::new()));
                }
                ui.separator();
                if ui
                    .add_enabled(has_job, egui::Button::new(with_keys("Save", Action::Save)))
                    .clicked()
                {
                    self.ask_save(false);
                }
                if ui
                    .add_enabled(
                        has_job,
                        egui::Button::new(with_keys("Save as…", Action::SaveAs)),
                    )
                    .clicked()
                {
                    self.ask_save(true);
                }
                if ui
                    .add_enabled(has_job, egui::Button::new("Save to path…"))
                    .clicked()
                {
                    self.prompt = Some(Prompt::Path(
                        PathPurpose::Save,
                        self.default_project_path().display().to_string(),
                    ));
                }
                ui.separator();
                if ui
                    .add_enabled(
                        has_result || self.generating(),
                        egui::Button::new(with_keys("Export G-code…", Action::Export)),
                    )
                    .clicked()
                {
                    self.export_when_ready();
                }
                if ui
                    .add_enabled(has_result, egui::Button::new("Export G-code to path…"))
                    .clicked()
                {
                    self.prompt = Some(Prompt::Path(
                        PathPurpose::Export,
                        self.default_export_path().display().to_string(),
                    ));
                }
                ui.separator();
                if ui.button(with_keys("Quit", Action::Quit)).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(
                        !self.undo.is_empty(),
                        egui::Button::new(with_keys("Undo", Action::Undo)),
                    )
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(
                        !self.redo.is_empty(),
                        egui::Button::new(with_keys("Redo", Action::Redo)),
                    )
                    .clicked()
                {
                    self.redo();
                }
                ui.separator();
                let drawn = !self.deletable_shapes().0.is_empty();
                let only_drawn = "Only drawn shapes; imported ones change only with the DXF";
                if ui
                    .add_enabled(drawn, egui::Button::new(with_keys("Copy", Action::Copy)))
                    .on_disabled_hover_text(only_drawn)
                    .clicked()
                    && let Some(text) = self.copy_selected_shapes()
                {
                    ui.ctx().copy_text(text);
                }
                if ui
                    .add_enabled(
                        !self.shape_clipboard.is_empty() && has_job,
                        egui::Button::new(with_keys("Paste", Action::Paste)),
                    )
                    .on_hover_text("10 mm right and up from the copied shapes; Ctrl+V over the canvas pastes at the cursor")
                    .clicked()
                {
                    self.paste(None);
                }
                if ui
                    .add_enabled(drawn, egui::Button::new("Mirror left–right"))
                    .on_hover_text("Flip the selected drawn shapes about the middle of their box")
                    .on_disabled_hover_text(only_drawn)
                    .clicked()
                {
                    self.mirror_selected(Mirror::LeftRight);
                }
                if ui
                    .add_enabled(drawn, egui::Button::new("Mirror up–down"))
                    .on_hover_text("Flip the selected drawn shapes about the middle of their box")
                    .on_disabled_hover_text(only_drawn)
                    .clicked()
                {
                    self.mirror_selected(Mirror::UpDown);
                }
                ui.separator();
                if ui
                    .add_enabled(
                        drawn,
                        egui::Button::new("Delete shapes  (Del)"),
                    )
                    .on_hover_text(
                        "Deletes the selected drawn shapes; imported ones change only with the DXF",
                    )
                    .clicked()
                {
                    self.delete_selected_shapes();
                }
                if ui
                    .add_enabled(
                        self.selector_json().is_some(),
                        egui::Button::new(with_keys("Copy as selector", Action::CopySelector)),
                    )
                    .clicked()
                {
                    self.copy_selector(ui.ctx());
                }
            });
            ui.menu_button("Job", |ui| {
                if self.generating() {
                    if ui.button("Cancel generation").clicked() {
                        self.cancel_generation();
                    }
                } else if ui
                    .add_enabled(
                        self.can_generate(),
                        egui::Button::new(with_keys("Generate", Action::Generate)),
                    )
                    .clicked()
                {
                    self.generate();
                }
            });
            ui.menu_button("Settings", |ui| {
                if ui.button("Tool library…").clicked() {
                    self.open_library_window(LibraryKind::Tools);
                }
                if ui.button("Machines…").clicked() {
                    self.open_library_window(LibraryKind::Machines);
                }
                ui.separator();
                let mut sheet = self.setup_sheet;
                if ui
                    .checkbox(&mut sheet, "Write a setup sheet with the G-code")
                    .on_hover_text(
                        "An HTML page for the operator next to the exported files: stock, \
                         zero, tools in order, operations, times and warnings",
                    )
                    .changed()
                {
                    self.set_setup_sheet(sheet);
                }
                ui.separator();
                ui.label("Colour theme");
                for (choice, label, hover) in [
                    (Theme::System, "System", "Light or dark, as the operating system is set"),
                    (Theme::Light, "Light", "Light panels (the canvas stays dark)"),
                    (Theme::Dark, "Dark", "Dark panels"),
                ] {
                    if ui
                        .radio(self.theme == choice, label)
                        .on_hover_text(hover)
                        .clicked()
                    {
                        self.set_theme(choice);
                    }
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button(with_keys("Fit view", Action::Fit)).clicked() {
                    self.fit_pending = true;
                }
                if ui.button(with_keys("Top view", Action::TopView)).clicked() {
                    self.view_top();
                }
                if ui
                    .add(
                        egui::Button::new(with_keys("Measure", Action::Measure))
                            .selected(self.measure.is_some()),
                    )
                    .on_hover_text("Two clicks give the distance, ΔX, ΔY and the angle")
                    .clicked()
                {
                    self.toggle_measure();
                }
                if ui.button("3D view").clicked() {
                    self.view_3d();
                }
                ui.menu_button("Z scale in 3D", |ui| {
                    for factor in [1.0, 2.0, 5.0, 10.0] {
                        ui.radio_value(&mut self.view.z_scale, factor, format!("× {factor}"));
                    }
                });
                ui.checkbox(&mut self.legend_open, "Show legend");
                let mut auto = self.auto_generate;
                if ui
                    .checkbox(&mut auto, "Update toolpath automatically")
                    .on_hover_text("Generate by itself half a second after each change")
                    .changed()
                {
                    self.set_auto_generate(auto);
                }
                ui.checkbox(&mut self.show_images, "Show images")
                    .on_hover_text("The pictures traced into shapes, faintly under the drawing");
                let mut sim = self.show_simulation;
                if ui
                    .checkbox(&mut sim, "Show simulation")
                    .on_hover_text(SIMULATION_HINT)
                    .changed()
                {
                    self.set_show_simulation(sim);
                }
            });
            ui.menu_button("Draw", |ui| {
                let can_draw = self.can_draw();
                for (tool, label) in [
                    (Tool::Circle, "Circle"),
                    (Tool::Rectangle, "Rectangle"),
                    (Tool::Polyline, "Polyline"),
                ] {
                    if ui.add_enabled(can_draw, egui::Button::new(label)).clicked() {
                        self.start_drawing(tool);
                    }
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("User guide").clicked() {
                    self.open_user_guide(ui.ctx());
                }
                if ui
                    .button(with_keys("Keyboard shortcuts", Action::Shortcuts))
                    .clicked()
                {
                    self.shortcuts_open = true;
                }
                if ui.button("About and licences").clicked() {
                    self.about_open = true;
                }
            });
        });
    }

    /// The toolbar under the menu: file | generate and export | view | draw.
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        let has_job = self.edit.is_some();
        let has_result = self.result.is_some();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
            if ui
                .button("New")
                .on_hover_text(hover_keys("New project", Action::New))
                .clicked()
            {
                self.ask_new();
            }
            if ui
                .button("Open")
                .on_hover_text(hover_keys("Open a project", Action::Open))
                .clicked()
            {
                self.ask_open();
            }
            if ui
                .button("Import DXF")
                .on_hover_text(hover_keys("New project from a DXF", Action::Import))
                .clicked()
            {
                self.ask_import();
            }
            if ui
                .add_enabled(has_job, egui::Button::new("Save"))
                .on_hover_text(hover_keys("Save the project", Action::Save))
                .clicked()
            {
                self.ask_save(false);
            }
            ui.separator();
            // The main action stands out.
            let accent = ui.visuals().selection.bg_fill;
            let on_accent = ui.visuals().selection.stroke.color;
            if self.generating() {
                if ui
                    .button(RichText::new("■ Cancel").strong())
                    .on_hover_text("Stop generating")
                    .clicked()
                {
                    self.cancel_generation();
                }
            } else if ui
                .add_enabled(
                    self.can_generate(),
                    egui::Button::new(RichText::new("▶ Generate").strong().color(on_accent))
                        .fill(accent),
                )
                .on_hover_text(hover_keys(
                    "Plan the toolpaths, write and check the G-code",
                    Action::Generate,
                ))
                .clicked()
            {
                self.generate();
            }
            if ui
                .add_enabled(
                    has_result || self.generating(),
                    egui::Button::new("⬇ Export G-code"),
                )
                .on_hover_text(hover_keys(
                    if has_result {
                        "Save the checked G-code"
                    } else {
                        "Save the G-code once it is generated"
                    },
                    Action::Export,
                ))
                .on_disabled_hover_text("Generate first")
                .clicked()
            {
                self.export_when_ready();
            }
            ui.separator();
            if ui
                .button("Fit")
                .on_hover_text(hover_keys("Fit the view", Action::Fit))
                .clicked()
            {
                self.fit_pending = true;
            }
            // Top and 3D side by side, one of them on: a two-way switch.
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if ui
                    .add(egui::Button::new("Top").selected(!self.three_d))
                    .on_hover_text(hover_keys("Look straight down", Action::TopView))
                    .clicked()
                {
                    self.view_top();
                }
                // Z stretch is labelled whenever it isn't true scale (docs/specs/3d-view.md §4).
                let three_d = if self.view.z_scale == 1.0 {
                    "3D".to_string()
                } else {
                    format!("3D (Z × {})", self.view.z_scale)
                };
                if ui
                    .add(egui::Button::new(three_d).selected(self.three_d))
                    .on_hover_text("Left-drag turns and tilts; right-drag or Shift+drag pans")
                    .clicked()
                {
                    self.view_3d();
                }
            });
            let label = if self.simulating() {
                "Simulating…"
            } else {
                "Simulation"
            };
            let response = ui.add_enabled(
                self.result.is_some(),
                egui::Button::new(label).selected(self.show_simulation),
            );
            if response
                .on_hover_text(SIMULATION_HINT)
                .on_disabled_hover_text("Generate the toolpath first")
                .clicked()
            {
                self.set_show_simulation(!self.show_simulation);
            }
            ui.separator();
            let can_draw = self.can_draw();
            for (tool, label, hint) in [
                (
                    Tool::Circle,
                    "○ Circle",
                    "Draw a circle: centre, then a point on it",
                ),
                (
                    Tool::Rectangle,
                    "▭ Rectangle",
                    "Draw a rectangle: two opposite corners",
                ),
                (
                    Tool::Polyline,
                    "✎ Polyline",
                    "Draw a polyline: click points",
                ),
            ] {
                let active = self.sketch.as_ref().is_some_and(|s| s.tool == tool);
                if ui
                    .add_enabled(can_draw, egui::Button::new(label).selected(active))
                    .on_hover_text(hint)
                    .clicked()
                {
                    self.start_drawing(tool);
                }
            }
        });
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let snapped = self.snapped.map(|(p, hit)| {
                let name = hit.map_or_else(String::new, |h| format!(" — snapped to {}", h.name()));
                (p, name)
            });
            if self.generating() {
                ui.spinner();
                ui.label("Generating…");
            } else if let Some(text) =
                self.measure_status(snapped.as_ref().map_or("", |(_, name)| name.as_str()))
            {
                ui.label(text);
            } else if let (Some(sketch), Some((at, name))) = (&self.sketch, &snapped) {
                let step = grid_step(self.view.scale);
                ui.label(format!("{}{name}", sketch.status(*at, step)));
            } else if let (Some(_), Some((at, name))) = (&self.handle_drag, &snapped) {
                ui.label(format!(
                    "Dragging to ({:.3}, {:.3}){name} (Alt: no snapping; Shift on a middle \
                     bends the segment)",
                    at.0, at.1
                ));
            } else if let Some((text, until)) = &self.status {
                if Instant::now() < *until {
                    ui.label(text);
                    ui.ctx().request_repaint_after(*until - Instant::now());
                } else {
                    self.status = None;
                }
            } else if self.polyline_selected() {
                ui.label(RichText::new(crate::shape_form::POLYLINE_HELP).weak());
            } else if self.edit.is_some() {
                ui.label(
                    RichText::new(
                        "Click a contour to select it · Ctrl+click adds or removes · \
                         right-click selects its layer · Esc clears",
                    )
                    .weak(),
                );
            }
            let copies = self
                .edit
                .as_ref()
                .and_then(|e| e.settings.copies)
                .map(|c| c.count())
                .filter(|&n| n > 1);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(n) = copies {
                    ui.label(format!("{n} copies"));
                    ui.separator();
                }
                if let Some((x, y)) = self.cursor {
                    // Drawing coordinates, and program coordinates when they differ.
                    let (zx, zy) = self
                        .edit
                        .as_ref()
                        .map_or((0.0, 0.0), |e| e.settings.work_zero_xy());
                    if (zx, zy) == (0.0, 0.0) {
                        ui.monospace(format!("X {x:.3}  Y {y:.3} mm"));
                    } else {
                        ui.monospace(format!(
                            "X {x:.3}  Y {y:.3} mm  (work X {:.3}  Y {:.3})",
                            x - zx,
                            y - zy
                        ));
                    }
                }
            });
        });
    }
}

/// What the simulation toggle does (docs/specs/simulation.md §4).
const SIMULATION_HINT: &str = "Show the stock as the program leaves it: every move cut with \
    the tool's shape (darker is deeper; red is below the stock). Display only.";
