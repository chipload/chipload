// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard shortcuts, the menus' items (shown from the header, header.rs) and the status
//! bar.

use super::shortcuts::{self, Action, with_keys};
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

    /// The File menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn file_menu(&mut self, ui: &mut egui::Ui) {
        let has_job = self.edit.is_some();
        let has_result = self.result.is_some();

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
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Open, String::new()));
        }
        if ui.button("Import image from path…").clicked() {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Image, String::new()));
        }
        ui.separator();
        let (update, update_path) = self.update_labels();
        // With several DXFs, each is updated from its row in the Browser.
        let one = has_job && self.dxf_count() <= 1;
        let several = "Several DXFs: update one from the Browser (right-click it)";
        if ui
            .add_enabled(one, egui::Button::new(update))
            .on_disabled_hover_text(several)
            .clicked()
        {
            self.ask_update();
        }
        if ui
            .add_enabled(one, egui::Button::new(update_path))
            .on_disabled_hover_text(several)
            .clicked()
        {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Update, String::new()));
        }
        let adding = has_job && self.dxf_count() > 0;
        if ui
            .add_enabled(adding, egui::Button::new("Add DXF to project…"))
            .on_hover_text("Another part: a DXF placed and updated on its own")
            .clicked()
        {
            self.ask_add_dxf();
        }
        if ui
            .add_enabled(adding, egui::Button::new("Add DXF from path…"))
            .clicked()
        {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::AddDxf, String::new()));
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
            self.dialogs.prompt = Some(Prompt::Path(
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
            self.dialogs.prompt = Some(Prompt::Path(
                PathPurpose::Export,
                self.default_export_path().display().to_string(),
            ));
        }
        ui.separator();
        if ui.button(with_keys("Quit", Action::Quit)).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    /// Undo and redo.
    pub(super) fn undo_menu(&mut self, ui: &mut egui::Ui) {
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
    }

    /// The selected drawn shapes' items: copy, paste, mirror, delete; and Copy as selector.
    pub(super) fn shape_menu(&mut self, ui: &mut egui::Ui) {
        let has_job = self.edit.is_some();
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
            .add_enabled(drawn, egui::Button::new("Delete shapes  (Del)"))
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
    }

    /// The Job menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn job_menu(&mut self, ui: &mut egui::Ui) {
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
    }

    /// The View menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn view_menu(&mut self, ui: &mut egui::Ui) {
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
        let mut grid = self.show_grid;
        if ui
            .checkbox(&mut grid, with_keys("Show grid", Action::Grid))
            .on_hover_text("Lines on the snap grid, a stronger one every ten")
            .changed()
        {
            self.set_show_grid(grid);
        }
        ui.checkbox(&mut self.legend_open, "Show legend");
        let mut auto = self.auto_generate;
        if ui
            .checkbox(&mut auto, "Update toolpath automatically")
            .on_hover_text("Generate by itself half a second after each change")
            .changed()
        {
            self.set_auto_generate(auto);
        }
        ui.checkbox(&mut self.show_toolpath, "Show toolpath")
            .on_hover_text("Hide the toolpath to see the drawing or the simulated stock");
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
        ui.separator();
        ui.label("Colour theme");
        for (choice, label, hover) in [
            (
                Theme::System,
                "System",
                "Light or dark, as the operating system is set",
            ),
            (Theme::Light, "Light", "Light panels and canvas"),
            (Theme::Dark, "Dark", "Dark panels and canvas"),
        ] {
            if ui
                .radio(self.theme == choice, label)
                .on_hover_text(hover)
                .clicked()
            {
                self.set_theme(choice);
            }
        }
    }

    /// The Draw menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn draw_menu(&mut self, ui: &mut egui::Ui) {
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
    }

    /// The Help menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn help_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("User guide").clicked() {
            self.open_user_guide(ui.ctx());
        }
        if ui
            .button(with_keys("Keyboard shortcuts", Action::Shortcuts))
            .clicked()
        {
            self.dialogs.shortcuts_open = true;
        }
        if ui.button("About and licences").clicked() {
            self.dialogs.about_open = true;
        }
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Pieces in all, from every part that is cut (docs/specs/parts.md §8).
            let copies = self
                .edit
                .as_ref()
                .map(|e| {
                    e.settings
                        .parts
                        .iter()
                        .filter(|p| p.enabled)
                        .map(|p| p.count())
                        .sum::<u32>()
                })
                .filter(|&n| n > 1);
            // The units, the post and the cycle time once generated, at the far right
            // (docs/specs/browser-and-layout.md §6).
            let post = self.edit.as_ref().map(|e| e.settings.machine.post.clone());
            // What the toolpath is of: one part of several, or all of them
            // (docs/specs/active-part-preview.md §4).
            let scope = self.edit.as_ref().and_then(|e| {
                let parts = &e.settings.parts;
                (parts.len() > 1).then(|| match self.result_part {
                    Some(id) => {
                        let k = parts.iter().position(|p| p.id == id).map_or(0, |k| k + 1);
                        format!("part {k} of {}", parts.len())
                    }
                    None => "all parts".to_string(),
                })
            });
            let time = self.result.as_ref().map(|r| {
                let time = chipload_pipeline::estimate::duration_text(r.estimate.total);
                match &scope {
                    Some(scope) => format!("{scope} · cycle {time}"),
                    None => format!("cycle {time}"),
                }
            });
            let grid = (self.show_grid && self.edit.is_some()).then(|| grid_step(self.view.scale));
            // With two parts or more, the one new operations go into (docs/specs/parts.md §8),
            // unless the cycle time already names it.
            let active = self.edit.as_ref().and_then(|e| {
                let parts = &e.settings.parts;
                let id = e.settings.active_part_id()?;
                (parts.len() > 1 && !(self.result.is_some() && self.result_part == Some(id)))
                    .then(|| parts.iter().find(|p| p.id == id).map(|p| p.name.clone()))?
            });
            let several = self
                .edit
                .as_ref()
                .is_some_and(|e| e.settings.parts.len() > 1);
            let zero = self
                .edit
                .as_ref()
                .map_or((0.0, 0.0), |e| e.settings.work_zero_xy());
            let cursor = self.cursor;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(name) = active {
                    ui.weak(format!("active part: {name}")).on_hover_text(
                        "New operations go into the active part (Browser, right-click a part)",
                    );
                    ui.separator();
                }
                if let Some(post) = post {
                    ui.weak("mm");
                    ui.separator();
                    if let Some(step) = grid {
                        ui.weak(format!("grid {step} mm")).on_hover_text(
                            "The grid's step: clicks while drawing snap to it (G hides it)",
                        );
                        ui.separator();
                    }
                    ui.weak(post)
                        .on_hover_text("The post-processor (Job settings > Machine)");
                    ui.separator();
                }
                if let Some(time) = time {
                    ui.weak(time)
                        .on_hover_text("Estimated cycle time (the report has the details)");
                    ui.separator();
                }
                if let Some(n) = copies {
                    // With parts, what comes off the sheet (docs/specs/parts.md §8).
                    ui.label(if several {
                        format!("{n} pieces")
                    } else {
                        format!("{n} copies")
                    });
                    ui.separator();
                }
                if let Some((x, y)) = cursor {
                    // Drawing coordinates, and program coordinates when they differ.
                    let (zx, zy) = zero;
                    if (zx, zy) == (0.0, 0.0) {
                        ui.monospace(format!("X {x:.3}  Y {y:.3} mm"));
                    } else {
                        ui.monospace(format!(
                            "X {x:.3}  Y {y:.3} mm  (work X {:.3}  Y {:.3})",
                            x - zx,
                            y - zy
                        ));
                    }
                    ui.separator();
                }
                // What is left of the bar: the hint or message, cut short with "…" rather
                // than run under the facts on the right.
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    self.status_message(ui);
                });
            });
        });
    }

    /// The status bar's left side: what is going on, a message, or a hint.
    fn status_message(&mut self, ui: &mut egui::Ui) {
        let snapped = self.drag.snapped.map(|(p, hit)| {
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
        } else if let (Some(_), Some((at, name))) = (&self.drag.handle, &snapped) {
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
    }
}

/// What the simulation toggle does (docs/specs/simulation.md §4).
pub(super) const SIMULATION_HINT: &str = "Show the stock as the program leaves it: every move cut with \
    the tool's shape (darker is deeper; red is below the stock). Display only.";
