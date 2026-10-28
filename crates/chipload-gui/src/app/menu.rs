// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard shortcuts, the menus' items (shown from the header, header.rs) and the status
//! bar.

use super::shortcuts::{self, Action, with_keys};
use super::*;
use crate::i18n::Lang;
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
        // Typing a value while drawing, or offsetting, opens the entry at the cursor
        // (drawing-input.md §2, drawing-editing.md §2).
        let typing_tool =
            self.sketch.is_some() || self.modify == Some(super::modify::ModifyTool::Offset);
        if typing_tool && self.typed.is_none() {
            let first = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Text(t)
                        if t.starts_with(|c: char| {
                            c.is_ascii_digit() || matches!(c, '-' | '.' | '@')
                        }) =>
                    {
                        Some(t.clone())
                    }
                    _ => None,
                })
            });
            let pointer = ctx.input(|i| i.pointer.latest_pos());
            if let (Some(first), Some(pointer)) = (first, pointer) {
                self.start_typing(&first, pointer + egui::vec2(16.0, 16.0));
            }
        }
        if self.sketch.is_none()
            && self.measure.is_none()
            && !self.placing_text
            && let Some(tool) = self.last_tool
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
        {
            self.start_drawing(tool);
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
            } else if self.modify.is_some() {
                self.cancel_modify();
            } else if self.placing_text {
                self.placing_text = false;
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

        if ui
            .button(with_keys(&t!(ui, "menu-new-project"), Action::New))
            .clicked()
        {
            self.ask_new();
        }
        if ui
            .button(with_keys(&t!(ui, "menu-open"), Action::Open))
            .clicked()
        {
            self.ask_open();
        }
        let recent = self.recent.clone();
        ui.add_enabled_ui(!recent.is_empty(), |ui| {
            ui.menu_button(t!(ui, "menu-open-recent"), |ui| {
                for path in &recent {
                    let name = file_name(Path::new(path));
                    if ui.button(name).on_hover_text(path).clicked() {
                        self.open_recent(path);
                    }
                }
                ui.separator();
                if ui.button(t!(ui, "menu-clear-list")).clicked() {
                    self.forget_recent(None);
                }
            });
        });
        if ui
            .button(with_keys(&t!(ui, "menu-import-dxf"), Action::Import))
            .clicked()
        {
            self.ask_import();
        }
        if ui
            .button(t!(ui, "menu-import-image"))
            .on_hover_text(t!(ui, "menu-import-image-hover"))
            .clicked()
        {
            self.import_image_dialog();
        }
        if ui.button(t!(ui, "menu-open-path")).clicked() {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Open, String::new()));
        }
        if ui.button(t!(ui, "menu-import-image-path")).clicked() {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Image, String::new()));
        }
        ui.separator();
        let (update, update_path) = self.update_labels();
        // With several DXFs, each is updated from its row in the Browser.
        let one = has_job && self.dxf_count() <= 1;
        let several = t!(ui, "menu-several-dxfs");
        if ui
            .add_enabled(one, egui::Button::new(update))
            .on_disabled_hover_text(&several)
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
        let adding = has_job;
        if ui
            .add_enabled(adding, egui::Button::new(t!(ui, "menu-add-dxf")))
            .on_hover_text(t!(ui, "menu-add-dxf-hover"))
            .clicked()
        {
            self.ask_add_dxf();
        }
        if ui
            .add_enabled(adding, egui::Button::new(t!(ui, "menu-add-dxf-path")))
            .clicked()
        {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::AddDxf, String::new()));
        }
        ui.separator();
        if ui
            .add_enabled(
                has_job,
                egui::Button::new(with_keys(&t!(ui, "menu-save"), Action::Save)),
            )
            .clicked()
        {
            self.ask_save(false);
        }
        if ui
            .add_enabled(
                has_job,
                egui::Button::new(with_keys(&t!(ui, "menu-save-as"), Action::SaveAs)),
            )
            .clicked()
        {
            self.ask_save(true);
        }
        if ui
            .add_enabled(has_job, egui::Button::new(t!(ui, "menu-save-path")))
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
                egui::Button::new(with_keys(&t!(ui, "menu-export"), Action::Export)),
            )
            .clicked()
        {
            self.export_when_ready();
        }
        if ui
            .add_enabled(has_result, egui::Button::new(t!(ui, "menu-export-path")))
            .clicked()
        {
            self.dialogs.prompt = Some(Prompt::Path(
                PathPurpose::Export,
                self.default_export_path().display().to_string(),
            ));
        }
        let has_shapes = self
            .edit
            .as_ref()
            .is_some_and(|e| !e.settings.drawing.shapes.is_empty());
        if ui
            .add_enabled(has_shapes, egui::Button::new(t!(ui, "menu-export-dxf")))
            .on_hover_text(t!(ui, "menu-export-dxf-hover"))
            .clicked()
        {
            self.ask_export_dxf();
            ui.close();
        }
        ui.separator();
        if ui
            .button(with_keys(&t!(ui, "menu-quit"), Action::Quit))
            .clicked()
        {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    /// Undo and redo.
    pub(super) fn undo_menu(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(
                !self.undo.is_empty(),
                egui::Button::new(with_keys(&t!(ui, "menu-undo"), Action::Undo)),
            )
            .clicked()
        {
            self.undo();
        }
        if ui
            .add_enabled(
                !self.redo.is_empty(),
                egui::Button::new(with_keys(&t!(ui, "menu-redo"), Action::Redo)),
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
        let only_drawn = t!(ui, "menu-only-drawn");
        if ui
            .add_enabled(
                drawn,
                egui::Button::new(with_keys(
                    &t!(ui, "menu-toggle-construction"),
                    Action::Construction,
                )),
            )
            .on_hover_text(t!(ui, "menu-construction-hover"))
            .on_disabled_hover_text(&only_drawn)
            .clicked()
        {
            self.toggle_selected_construction();
            ui.close();
        }
        if ui
            .add_enabled(
                drawn,
                egui::Button::new(with_keys(&t!(ui, "menu-copy"), Action::Copy)),
            )
            .on_disabled_hover_text(&only_drawn)
            .clicked()
            && let Some(text) = self.copy_selected_shapes()
        {
            ui.ctx().copy_text(text);
        }
        if ui
            .add_enabled(
                !self.shape_clipboard.is_empty() && has_job,
                egui::Button::new(with_keys(&t!(ui, "menu-paste"), Action::Paste)),
            )
            .on_hover_text(t!(
                ui,
                "units-text-paste-hover",
                shift = crate::units::text(ui.ctx(), chipload_pipeline::shapes::PASTE_SHIFT_MM)
            ))
            .clicked()
        {
            self.paste(None);
        }
        if ui
            .add_enabled(drawn, egui::Button::new(t!(ui, "menu-mirror-left-right")))
            .on_hover_text(t!(ui, "menu-mirror-hover"))
            .on_disabled_hover_text(&only_drawn)
            .clicked()
        {
            self.mirror_selected(Mirror::LeftRight);
        }
        if ui
            .add_enabled(drawn, egui::Button::new(t!(ui, "menu-mirror-up-down")))
            .on_hover_text(t!(ui, "menu-mirror-hover"))
            .on_disabled_hover_text(&only_drawn)
            .clicked()
        {
            self.mirror_selected(Mirror::UpDown);
        }
        let any_drawn = self
            .edit
            .as_ref()
            .is_some_and(|e| e.settings.drawing.shapes.iter().any(|s| s.is_drawn()));
        if ui
            .add_enabled(any_drawn, egui::Button::new(t!(ui, "menu-join-ends")))
            .on_hover_text(t!(ui, "menu-join-ends-hover"))
            .on_disabled_hover_text(t!(ui, "menu-join-ends-disabled-hover"))
            .clicked()
        {
            self.ask_join();
        }
        ui.separator();
        if ui
            .add_enabled(
                drawn,
                egui::Button::new(format!("{}  (Del)", t!(ui, "menu-delete-shapes"))),
            )
            .on_hover_text(t!(ui, "menu-delete-shapes-hover"))
            .clicked()
        {
            self.delete_selected_shapes();
        }
        if ui
            .add_enabled(
                self.selector_json().is_some(),
                egui::Button::new(with_keys(
                    &t!(ui, "menu-copy-selector"),
                    Action::CopySelector,
                )),
            )
            .clicked()
        {
            self.copy_selector(ui.ctx());
        }
    }

    /// The Job menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn job_menu(&mut self, ui: &mut egui::Ui) {
        if self.generating() {
            if ui.button(t!(ui, "menu-cancel-generation")).clicked() {
                self.cancel_generation();
            }
        } else if ui
            .add_enabled(
                self.can_generate(),
                egui::Button::new(with_keys(&t!(ui, "menu-generate"), Action::Generate)),
            )
            .clicked()
        {
            self.generate();
        }
    }

    /// The View menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn view_menu(&mut self, ui: &mut egui::Ui) {
        if ui
            .button(with_keys(&t!(ui, "menu-fit-view"), Action::Fit))
            .clicked()
        {
            self.fit_pending = true;
        }
        if ui
            .button(with_keys(&t!(ui, "menu-top-view"), Action::TopView))
            .clicked()
        {
            self.view_top();
        }
        if ui
            .add(
                egui::Button::new(with_keys(&t!(ui, "menu-measure"), Action::Measure))
                    .selected(self.measure.is_some()),
            )
            .on_hover_text(t!(ui, "menu-measure-hover"))
            .clicked()
        {
            self.toggle_measure();
        }
        if ui.button(t!(ui, "menu-3d-view")).clicked() {
            self.view_3d();
        }
        ui.menu_button(t!(ui, "menu-z-scale"), |ui| {
            for factor in [1.0, 2.0, 5.0, 10.0] {
                ui.radio_value(&mut self.view.z_scale, factor, format!("× {factor}"));
            }
        });
        let mut grid = self.show_grid;
        if ui
            .checkbox(
                &mut grid,
                with_keys(&t!(ui, "menu-show-grid"), Action::Grid),
            )
            .on_hover_text(t!(ui, "menu-show-grid-hover"))
            .changed()
        {
            self.set_show_grid(grid);
        }
        ui.checkbox(&mut self.legend_open, t!(ui, "menu-show-legend"));
        ui.checkbox(&mut self.browser_open, t!(ui, "menu-show-browser"));
        ui.checkbox(&mut self.properties_open, t!(ui, "menu-show-properties"));
        if ui
            .button(with_keys(
                &if self.browser_open || self.properties_open {
                    t!(ui, "menu-fold-panels")
                } else {
                    t!(ui, "menu-open-panels")
                },
                Action::Panels,
            ))
            .on_hover_text(t!(ui, "menu-panels-hover"))
            .clicked()
        {
            self.toggle_panels();
        }
        let mut auto = self.auto_generate;
        if ui
            .checkbox(&mut auto, t!(ui, "menu-auto-update"))
            .on_hover_text(t!(ui, "menu-auto-update-hover"))
            .changed()
        {
            self.set_auto_generate(auto);
        }
        ui.checkbox(&mut self.show_toolpath, t!(ui, "menu-show-toolpath"))
            .on_hover_text(t!(ui, "menu-show-toolpath-hover"));
        let mut drawing = self.show_drawing();
        if ui
            .checkbox(&mut drawing, t!(ui, "menu-show-drawing"))
            .on_hover_text(t!(ui, "menu-show-drawing-hover"))
            .changed()
        {
            self.set_show_drawing(drawing);
        }
        ui.checkbox(&mut self.show_images, t!(ui, "menu-show-images"))
            .on_hover_text(t!(ui, "menu-show-images-hover"));
        let mut sim = self.show_simulation;
        if ui
            .checkbox(&mut sim, t!(ui, "menu-show-simulation"))
            .on_hover_text(simulation_hint(ui))
            .changed()
        {
            self.set_show_simulation(sim);
        }
        ui.separator();
        ui.label(t!(ui, "menu-units"));
        for (choice, label, hover) in [
            (
                chipload_job::Units::Mm,
                "mm".to_string(),
                t!(ui, "menu-units-mm-hover"),
            ),
            (
                chipload_job::Units::Inch,
                t!(ui, "menu-units-inch"),
                t!(ui, "menu-units-inch-hover"),
            ),
        ] {
            if ui
                .radio(self.units == choice, label)
                .on_hover_text(hover)
                .clicked()
            {
                self.set_units(choice);
            }
        }
        ui.separator();
        ui.label(t!(ui, "menu-theme"));
        for (choice, label, hover) in [
            (
                Theme::System,
                t!(ui, "menu-theme-system"),
                t!(ui, "menu-theme-system-hover"),
            ),
            (
                Theme::Light,
                t!(ui, "menu-theme-light"),
                t!(ui, "menu-theme-light-hover"),
            ),
            (
                Theme::Dark,
                t!(ui, "menu-theme-dark"),
                t!(ui, "menu-theme-dark-hover"),
            ),
        ] {
            if ui
                .radio(self.theme == choice, label)
                .on_hover_text(hover)
                .clicked()
            {
                self.set_theme(choice);
            }
        }
        ui.separator();
        self.language_menu(ui);
    }

    /// View > Language: the system's, or one of ours, each named in itself; those not yet
    /// read by a native speaker ask for a review (docs/specs/translations.md).
    fn language_menu(&mut self, ui: &mut egui::Ui) {
        ui.label(t!(ui, "menu-language"));
        let system = chipload_job::library::SYSTEM_LANGUAGE;
        let mut choice = None;
        let codes = std::iter::once(system).chain(crate::i18n::Language::ALL.map(|l| l.code()));
        for code in codes {
            let mut response = ui.radio(self.language_setting == code, language_name(ui, code));
            response = match crate::i18n::Language::from_tag(code) {
                None => response.on_hover_text(t!(ui, "menu-language-system-hover")),
                Some(l) if !l.reviewed() => {
                    response.on_hover_text(t!(ui, "menu-language-review-hover"))
                }
                Some(_) => response,
            };
            if response.clicked() {
                choice = Some(code);
            }
        }
        if let Some(setting) = choice {
            self.set_language(setting);
        }
    }

    /// The Draw menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn draw_menu(&mut self, ui: &mut egui::Ui) {
        let can_draw = self.can_draw();
        for (tool, label, action) in [
            (Tool::Circle, t!(ui, "menu-draw-circle"), Action::DrawCircle),
            (
                Tool::Rectangle,
                t!(ui, "menu-draw-rectangle"),
                Action::DrawRectangle,
            ),
            (
                Tool::Polyline,
                t!(ui, "menu-draw-polyline"),
                Action::DrawPolyline,
            ),
            (Tool::Arc, t!(ui, "menu-draw-arc"), Action::DrawArc),
            (Tool::Slot, t!(ui, "menu-draw-slot"), Action::DrawSlot),
            (
                Tool::Polygon,
                t!(ui, "menu-draw-polygon"),
                Action::DrawPolygon,
            ),
        ] {
            let label = with_keys(&label, action);
            if ui.add_enabled(can_draw, egui::Button::new(label)).clicked() {
                self.start_drawing(tool);
            }
        }
        if ui
            .add_enabled(can_draw, egui::Button::new(t!(ui, "menu-draw-text")))
            .on_hover_text(t!(ui, "menu-draw-text-hover"))
            .clicked()
        {
            self.start_text();
        }
    }

    /// The Modify menu's items: the editing tools (docs/specs/drawing-editing.md §8).
    pub(super) fn modify_menu(&mut self, ui: &mut egui::Ui) {
        let can_draw = self.can_draw();
        let label = with_keys(&t!(ui, "menu-modify-offset"), Action::Offset);
        if ui
            .add_enabled(can_draw, egui::Button::new(label))
            .on_hover_text(t!(ui, "header-offset-hover"))
            .clicked()
        {
            self.start_modify(super::modify::ModifyTool::Offset);
            ui.close();
        }
        for (tool, label, hover) in [
            (
                super::modify::ModifyTool::Fillet,
                t!(ui, "menu-modify-fillet"),
                t!(ui, "header-fillet-hover"),
            ),
            (
                super::modify::ModifyTool::Chamfer,
                t!(ui, "menu-modify-chamfer"),
                t!(ui, "header-chamfer-hover"),
            ),
            (
                super::modify::ModifyTool::Trim,
                with_keys(&t!(ui, "menu-modify-trim"), Action::Trim),
                t!(ui, "header-trim-hover"),
            ),
            (
                super::modify::ModifyTool::Extend,
                t!(ui, "menu-modify-extend"),
                t!(ui, "header-extend-hover"),
            ),
            (
                super::modify::ModifyTool::Break,
                t!(ui, "menu-modify-break"),
                t!(ui, "header-break-hover"),
            ),
            (
                super::modify::ModifyTool::Union,
                t!(ui, "menu-modify-union"),
                t!(ui, "header-union-hover"),
            ),
            (
                super::modify::ModifyTool::Subtract,
                t!(ui, "menu-modify-subtract"),
                t!(ui, "header-subtract-hover"),
            ),
            (
                super::modify::ModifyTool::Intersect,
                t!(ui, "menu-modify-intersect"),
                t!(ui, "header-intersect-hover"),
            ),
            (
                super::modify::ModifyTool::Array,
                t!(ui, "menu-modify-array"),
                t!(ui, "header-array-hover"),
            ),
            (
                super::modify::ModifyTool::PolarArray,
                t!(ui, "menu-modify-polar-array"),
                t!(ui, "header-polar-array-hover"),
            ),
        ] {
            if ui
                .add_enabled(
                    can_draw && self.modify_ready(tool),
                    egui::Button::new(label),
                )
                .on_hover_text(hover)
                .clicked()
            {
                self.start_modify(tool);
                ui.close();
            }
        }
    }

    /// The Help menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn help_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(t!(ui, "menu-user-guide")).clicked() {
            self.open_user_guide(ui.ctx());
        }
        if ui
            .button(with_keys(&t!(ui, "menu-shortcuts"), Action::Shortcuts))
            .clicked()
        {
            self.dialogs.shortcuts_open = true;
        }
        if ui.button(t!(ui, "menu-about")).clicked() {
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
                        t!(ui, "menu-status-part-of", part = k, count = parts.len())
                    }
                    None => t!(ui, "menu-status-all-parts"),
                })
            });
            let time = self.result.as_ref().map(|r| {
                let time = chipload_pipeline::estimate::duration_message(r.estimate.total)
                    .render(ui.lang(), self.units.into());
                let cycle = t!(ui, "menu-status-cycle", time = time);
                match &scope {
                    Some(scope) => format!("{scope} · {cycle}"),
                    None => cycle,
                }
            });
            let grid = (self.show_grid && self.edit.is_some())
                .then(|| grid_step(self.view.scale, self.units));
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
            let units = self.units;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(name) = active {
                    ui.weak(t!(ui, "menu-status-active-part", name = name))
                        .on_hover_text(t!(ui, "menu-status-active-part-hover"));
                    ui.separator();
                }
                if let Some(post) = post {
                    ui.weak(chipload_pipeline::units::length_unit(units))
                        .on_hover_text(t!(ui, "menu-status-units-hover"));
                    ui.separator();
                    if let Some(step) = grid {
                        ui.weak(t!(
                            ui,
                            "menu-status-grid",
                            step = chipload_pipeline::units::length_text(step, units)
                        ))
                        .on_hover_text(t!(ui, "menu-status-grid-hover"));
                        ui.separator();
                    }
                    ui.weak(post)
                        .on_hover_text(t!(ui, "menu-status-post-hover"));
                    ui.separator();
                }
                if let Some(time) = time {
                    ui.weak(time)
                        .on_hover_text(t!(ui, "menu-status-time-hover"));
                    ui.separator();
                }
                if let Some(n) = copies {
                    // With parts, what comes off the sheet (docs/specs/parts.md §8).
                    ui.label(if several {
                        t!(ui, "menu-status-pieces", count = n)
                    } else {
                        t!(ui, "menu-status-copies", count = n)
                    });
                    ui.separator();
                }
                if let Some((x, y)) = cursor {
                    // Drawing coordinates, and program coordinates when they differ.
                    let (zx, zy) = zero;
                    let n = |mm: f64| crate::draw::fixed(mm, units);
                    let unit = chipload_pipeline::units::length_unit(units);
                    if (zx, zy) == (0.0, 0.0) {
                        ui.monospace(format!("X {}  Y {} {unit}", n(x), n(y)));
                    } else {
                        let work = t!(ui, "menu-status-work", x = n(x - zx), y = n(y - zy));
                        ui.monospace(format!("X {}  Y {} {unit}  {work}", n(x), n(y)));
                    }
                    ui.separator();
                }
                // What is left of the bar: the hint or message, cut short with "…" rather
                // than run under the facts on the right.
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    self.polygon_sides_field(ui);
                    self.construction_field(ui);
                    self.offset_fields(ui);
                    self.status_message(ui);
                });
            });
        });
    }

    /// While drawing a polygon: its number of sides, kept for the next one
    /// (docs/specs/drawing-input.md §5).
    fn polygon_sides_field(&mut self, ui: &mut egui::Ui) {
        let Some(sketch) = self.sketch.as_mut().filter(|s| s.tool == Tool::Polygon) else {
            return;
        };
        ui.label(t!(ui, "menu-polygon-sides"));
        if ui
            .add(
                egui::DragValue::new(&mut self.polygon_sides)
                    .range(chipload_job::POLYGON_SIDES)
                    .speed(0.1),
            )
            .changed()
        {
            sketch.sides = self.polygon_sides;
        }
        ui.separator();
    }

    /// While a drawing tool is active: whether it draws construction geometry (key K).
    fn construction_field(&mut self, ui: &mut egui::Ui) {
        if self.sketch.is_none() {
            return;
        }
        let label = with_keys(&t!(ui, "menu-construction"), Action::Construction);
        ui.checkbox(&mut self.construction_mode, label)
            .on_hover_text(t!(ui, "menu-construction-hover"));
        ui.separator();
    }

    /// The status bar's left side: what is going on, a message, or a hint.
    fn status_message(&mut self, ui: &mut egui::Ui) {
        let snapped = self.drag.snapped.map(|(p, hit)| {
            let name = hit.map_or_else(String::new, |h| {
                format!(
                    " {}",
                    t!(
                        ui,
                        "menu-snapped-to",
                        target = crate::draw::snap_name(&h, ui)
                    )
                )
            });
            (p, name)
        });
        if self.generating() {
            ui.spinner();
            ui.label(t!(ui, "menu-generating"));
        } else if let Some(text) =
            self.measure_status(snapped.as_ref().map_or("", |(_, name)| name.as_str()))
        {
            ui.label(text);
        } else if let Some(text) = self.modify_status(self.cursor, ui.input(|i| i.modifiers.alt)) {
            ui.label(text);
        } else if let (Some(sketch), Some((at, name))) = (&self.sketch, &snapped) {
            let step = grid_step(self.view.scale, self.units);
            ui.label(format!(
                "{}{name}",
                sketch.status(*at, step, self.units, ui)
            ));
        } else if let (true, Some((at, name))) = (
            self.drag.handle.is_some() || self.drag.moving.is_some(),
            &snapped,
        ) {
            ui.label(t!(
                ui,
                "menu-dragging",
                x = crate::draw::fixed(at.0, self.units),
                y = crate::draw::fixed(at.1, self.units),
                snapped = name.as_str()
            ));
        } else if let Some((text, until)) = &self.status {
            if Instant::now() < *until {
                ui.label(text);
                ui.ctx().request_repaint_after(*until - Instant::now());
            } else {
                self.status = None;
            }
        } else if self.polyline_selected() {
            ui.label(RichText::new(crate::shape_form::polyline_help(ui)).weak());
        } else if self.edit.is_some() {
            ui.label(RichText::new(t!(ui, "menu-status-hint")).weak());
        }
    }
}

/// What the simulation toggle does (docs/specs/simulation.md §4).
pub(super) fn simulation_hint(src: &impl crate::i18n::Lang) -> String {
    t!(src, "menu-simulation-hint")
}

/// A `language` setting as the language menus name it: *System*, or the language in itself,
/// asking for a review when no native speaker has read it yet.
pub(super) fn language_name(src: &impl crate::i18n::Lang, setting: &str) -> String {
    match crate::i18n::Language::from_tag(setting) {
        None => t!(src, "menu-language-system"),
        Some(l) if l.reviewed() => l.native_name().to_string(),
        Some(l) => t!(src, "menu-language-review", language = l.native_name()),
    }
}
