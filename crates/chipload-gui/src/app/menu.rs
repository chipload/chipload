// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard shortcuts, the menu bar and the status bar.

use super::*;

impl ChiploadApp {
    // -- user interface

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        // Not while typing in a text field.
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let pressed = |s: &KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(s));
        // Shift variants first: they also contain the plain shortcut.
        if pressed(&COPY_SELECTOR) {
            self.copy_selector(ctx);
        }
        if pressed(&SAVE_AS) {
            self.ask_save(true);
        }
        if pressed(&REDO) {
            self.redo();
        }
        if pressed(&SAVE) {
            self.ask_save(false);
        }
        if pressed(&UNDO) {
            self.undo();
        }
        if pressed(&OPEN) {
            self.ask_open();
        }
        if pressed(&IMPORT) {
            self.ask_import();
        }
        if pressed(&EXPORT) {
            self.ask_export();
        }
        if pressed(&GENERATE) && self.can_generate() {
            self.generate();
        }
        if let Focus::Op(i) = self.focus {
            if pressed(&MOVE_UP) && i > 0 {
                self.move_operation(i, i - 1);
            }
            if pressed(&MOVE_DOWN) {
                self.move_operation(i, i + 1);
            }
        }
        if pressed(&FIT) {
            self.fit_pending = true;
        }
        if pressed(&TOP_VIEW) {
            self.view_top();
        }
        if pressed(&QUIT) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
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
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.sketch.is_some() {
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
                if ui.button("Open…  (Ctrl+O)").clicked() {
                    self.ask_open();
                }
                if ui.button("Import DXF…  (Ctrl+I)").clicked() {
                    self.ask_import();
                }
                if ui.button("Open path…").clicked() {
                    self.prompt = Some(Prompt::Path(PathPurpose::Open, String::new()));
                }
                ui.separator();
                if ui
                    .add_enabled(has_job, egui::Button::new("Update drawing from DXF…"))
                    .clicked()
                {
                    self.ask_update();
                }
                if ui
                    .add_enabled(has_job, egui::Button::new("Update drawing from path…"))
                    .clicked()
                {
                    self.prompt = Some(Prompt::Path(PathPurpose::Update, String::new()));
                }
                ui.separator();
                if ui
                    .add_enabled(has_job, egui::Button::new("Save  (Ctrl+S)"))
                    .clicked()
                {
                    self.ask_save(false);
                }
                if ui
                    .add_enabled(has_job, egui::Button::new("Save as…  (Ctrl+Shift+S)"))
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
                    .add_enabled(has_result, egui::Button::new("Export G-code…  (Ctrl+E)"))
                    .clicked()
                {
                    self.ask_export();
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
                if ui.button("Quit  (Ctrl+Q)").clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo  (Ctrl+Z)"))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(
                        !self.redo.is_empty(),
                        egui::Button::new("Redo  (Ctrl+Shift+Z)"),
                    )
                    .clicked()
                {
                    self.redo();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.selector_json().is_some(),
                        egui::Button::new("Copy as selector  (Ctrl+Shift+C)"),
                    )
                    .clicked()
                {
                    self.copy_selector(ui.ctx());
                }
            });
            ui.menu_button("Job", |ui| {
                if ui
                    .add_enabled(self.can_generate(), egui::Button::new("Generate  (F5)"))
                    .clicked()
                {
                    self.generate();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Fit view  (F)").clicked() {
                    self.fit_pending = true;
                }
                if ui.button("Top view  (T)").clicked() {
                    self.view_top();
                }
                if ui.button("3D view").clicked() {
                    self.view_3d();
                }
                ui.menu_button("Z scale in 3D", |ui| {
                    for factor in [1.0, 2.0, 5.0, 10.0] {
                        ui.radio_value(&mut self.view.z_scale, factor, format!("× {factor}"));
                    }
                });
                ui.checkbox(&mut self.show_material_left, "Show material left")
                    .on_hover_text(
                        "Faint: material the tool can't reach (inside corners, narrow slots). \
                         Red: material the toolpath missed.",
                    );
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
                if ui.button("About and licences").clicked() {
                    self.about_open = true;
                }
            });
            ui.separator();
            if ui.button("Open").clicked() {
                self.ask_open();
            }
            if ui.button("Import DXF").clicked() {
                self.ask_import();
            }
            if ui.add_enabled(has_job, egui::Button::new("Save")).clicked() {
                self.ask_save(false);
            }
            if ui
                .add_enabled(self.can_generate(), egui::Button::new("Generate (F5)"))
                .clicked()
            {
                self.generate();
            }
            if ui
                .add_enabled(has_result, egui::Button::new("Export G-code"))
                .clicked()
            {
                self.ask_export();
            }
            if ui.button("Fit").clicked() {
                self.fit_pending = true;
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
            if ui
                .add(egui::Button::new("Top view").selected(!self.three_d))
                .on_hover_text("Look straight down (T)")
                .clicked()
            {
                self.view_top();
            }
            ui.separator();
            let can_draw = self.can_draw();
            for (tool, label, hint) in [
                (
                    Tool::Circle,
                    "Circle",
                    "Draw a circle: centre, then a point on it",
                ),
                (
                    Tool::Rectangle,
                    "Rectangle",
                    "Draw a rectangle: two opposite corners",
                ),
                (Tool::Polyline, "Polyline", "Draw a polyline: click points"),
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
            if let (Some(sketch), Some(at)) = (&self.sketch, self.cursor) {
                let step = grid_step(self.view.scale);
                ui.label(sketch.status(snap(at, step), step));
            } else if let Some((text, until)) = &self.status {
                if Instant::now() < *until {
                    ui.label(text);
                    ui.ctx().request_repaint_after(*until - Instant::now());
                } else {
                    self.status = None;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
