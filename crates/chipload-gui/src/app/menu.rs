// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard shortcuts, the menus' items (shown from the header, header.rs) and the status
//! bar.

use super::layout::Pane;
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
            self.sketch().is_some() || self.modify() == Some(super::modify::ModifyTool::Offset);
        if typing_tool && self.canvas_tools.typed.is_none() {
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
        if self.sketch().is_none()
            && self.measure().is_none()
            && !self.placing_text()
            && let Some(tool) = self.canvas_tools.last_tool
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
        {
            self.start_drawing(tool);
        }
        if self.sketch().is_some() {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                self.finish_drawing();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
                && let Some(sketch) = self.sketch_mut()
            {
                sketch.undo_point();
            }
        }
        if self.sketch().is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete))
        {
            self.delete_key();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.cancel_zero_drag() {
                // The zero stays where it was (docs/specs/work-zero-marker.md §3).
            } else if self.tool_active() {
                self.end_tool();
            } else {
                self.selection = CanvasSelection::Nothing;
            }
        }
    }

    /// The File menu's items (a ribbon caption's or the top bar's drop-down).
    pub(super) fn file_menu(&mut self, ui: &mut egui::Ui) {
        let has_job = self.edit.is_some();
        let has_result = self.toolpath.result.is_some();

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
        let recent = self.prefs.recent.clone();
        ui.add_enabled_ui(!recent.is_empty(), |ui| {
            super::header::submenu(ui, t!(ui, "menu-open-recent"), |ui| {
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
        if ui
            .button(t!(ui, "menu-add-model"))
            .on_hover_text(t!(ui, "menu-add-model-hover"))
            .clicked()
        {
            self.add_model_dialog();
        }
        if ui.button(t!(ui, "menu-open-path")).clicked() {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Open, String::new()));
        }
        if ui.button(t!(ui, "menu-import-image-path")).clicked() {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Image, String::new()));
        }
        if ui.button(t!(ui, "menu-add-model-path")).clicked() {
            self.dialogs.prompt = Some(Prompt::Path(PathPurpose::Model, String::new()));
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
            .is_some_and(|e| !e.job.drawing.shapes.is_empty());
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
    /// The Edit menu (docs/specs/menu-bar.md §3): undo and redo, the selection's items as on
    /// the canvas' right-click, and Preferences….
    pub(super) fn edit_menu(&mut self, ui: &mut egui::Ui) {
        self.undo_menu(ui);
        ui.separator();
        self.shape_menu(ui);
        ui.separator();
        if ui
            .button(t!(ui, "menu-preferences"))
            .on_hover_text(t!(ui, "menu-preferences-hover"))
            .clicked()
        {
            self.show_preferences();
            ui.close();
        }
    }

    pub(super) fn undo_menu(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(
                !self.history.undo.is_empty(),
                egui::Button::new(with_keys(&t!(ui, "menu-undo"), Action::Undo)),
            )
            .clicked()
        {
            self.undo();
        }
        if ui
            .add_enabled(
                !self.history.redo.is_empty(),
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
                !self.clipboard.shapes.is_empty() && has_job,
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
        if !self.framed_shape_ids().is_empty() {
            self.selection_menu(ui);
            if ui
                .add_enabled(
                    self.can_draw(),
                    egui::Button::new(t!(ui, "menu-frame-selection")),
                )
                .on_hover_text(t!(ui, "menu-frame-selection-hover"))
                .clicked()
            {
                self.start_frame();
                ui.close();
            }
            ui.separator();
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
            .is_some_and(|e| e.job.drawing.shapes.iter().any(|s| s.is_drawn()));
        if ui
            .add_enabled(any_drawn, egui::Button::new(t!(ui, "menu-join-ends")))
            .on_hover_text(t!(ui, "menu-join-ends-hover"))
            .on_disabled_hover_text(t!(ui, "menu-join-ends-disabled-hover"))
            .clicked()
        {
            self.ask_join();
        }
        ui.separator();
        // Delete takes a text, a picture or a DXF selected whole, and letters' texts too.
        let deletable = matches!(
            &self.selection,
            CanvasSelection::Whole { of, left_out }
                if left_out.is_empty() && of.shape_from().is_some()
        ) || {
            let (ids, texts, _) = self.deletion();
            !ids.is_empty() || !texts.is_empty()
        };
        if ui
            .add_enabled(
                deletable,
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
                    .selected(self.measure().is_some()),
            )
            .on_hover_text(t!(ui, "menu-measure-hover"))
            .clicked()
        {
            self.toggle_measure();
        }
        if ui.button(t!(ui, "menu-3d-view")).clicked() {
            self.view_3d();
        }
        super::header::submenu(ui, t!(ui, "menu-z-scale"), |ui| {
            for factor in [1.0, 2.0, 5.0, 10.0] {
                ui.radio_value(&mut self.view.z_scale, factor, format!("× {factor}"));
            }
        });
        let mut reverse = self.prefs.reverse_orbit;
        if ui
            .checkbox(&mut reverse, t!(ui, "menu-reverse-orbit"))
            .on_hover_text(t!(ui, "menu-reverse-orbit-hover"))
            .changed()
        {
            self.set_reverse_orbit(reverse);
        }
        let mut grid = self.prefs.show_grid;
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
        let mut rulers = self.prefs.show_rulers;
        if ui
            .checkbox(&mut rulers, t!(ui, "menu-show-rulers"))
            .on_hover_text(t!(ui, "menu-show-rulers-hover"))
            .changed()
        {
            self.set_show_rulers(rulers);
        }
        ui.checkbox(&mut self.display.legend_open, t!(ui, "menu-show-legend"));
        ui.checkbox(&mut self.display.turn_axis, t!(ui, "menu-show-turn-axis"))
            .on_hover_text(t!(ui, "menu-show-turn-axis-hover"));
        // The panels: each opened or closed, every one hidden with P, the default layout
        // back (docs/specs/dockable-panels.md §1).
        super::header::submenu(ui, t!(ui, "menu-panels"), |ui| {
            for pane in Pane::PANELS {
                let mut open = self.panels.is_open(pane);
                if ui.checkbox(&mut open, pane.title(ui)).changed() {
                    if open {
                        self.panels.show(pane);
                    } else {
                        self.panels.close(pane);
                    }
                }
            }
            ui.separator();
            if ui
                .button(t!(ui, "menu-reset-layout"))
                .on_hover_text(t!(ui, "menu-reset-layout-hover"))
                .clicked()
            {
                self.panels.reset();
                ui.close();
            }
        });
        if ui
            .button(with_keys(
                &if self.panels.hidden() {
                    t!(ui, "menu-open-panels")
                } else {
                    t!(ui, "menu-fold-panels")
                },
                Action::Panels,
            ))
            .on_hover_text(t!(ui, "menu-panels-hover"))
            .clicked()
        {
            self.panels.toggle_hidden();
        }
        let mut auto = self.prefs.auto_generate;
        if ui
            .checkbox(&mut auto, t!(ui, "menu-auto-update"))
            .on_hover_text(t!(ui, "menu-auto-update-hover"))
            .changed()
        {
            self.set_auto_generate(auto);
        }
        ui.checkbox(&mut self.display.toolpath, t!(ui, "menu-show-toolpath"))
            .on_hover_text(t!(ui, "menu-show-toolpath-hover"));
        // Edit always shows the drawing (docs/specs/edit-and-simulate.md §2).
        let mut drawing = self.show_drawing();
        if ui
            .add_enabled(
                self.display.simulate,
                egui::Checkbox::new(&mut drawing, t!(ui, "menu-show-drawing")),
            )
            .on_hover_text(t!(ui, "menu-show-drawing-hover"))
            .changed()
        {
            self.set_show_drawing(drawing);
        }
        ui.checkbox(&mut self.display.images, t!(ui, "menu-show-images"))
            .on_hover_text(t!(ui, "menu-show-images-hover"));
        let mut sim = self.display.simulate;
        if ui
            .checkbox(
                &mut sim,
                with_keys(&t!(ui, "menu-simulate"), Action::Simulate),
            )
            .on_hover_text(simulation_hint(ui))
            .changed()
        {
            self.set_show_simulation(sim);
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
        if ui
            .add_enabled(can_draw, egui::Button::new(t!(ui, "menu-draw-frame")))
            .on_hover_text(t!(ui, "menu-draw-frame-hover"))
            .clicked()
        {
            self.start_frame();
            ui.close();
        }
    }

    /// The Wizards menu's items: the things made in one dialog, drawing and operations
    /// together (docs/specs/sign-maker.md §3).
    pub(super) fn wizards_menu(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(
                self.can_draw(),
                egui::Button::new(t!(ui, "menu-wizard-sign")),
            )
            .on_hover_text(t!(ui, "menu-wizard-sign-hover"))
            .clicked()
        {
            self.start_sign();
            ui.close();
        }
        if ui
            .add_enabled(
                self.edit.is_some(),
                egui::Button::new(t!(ui, "menu-wizard-two-sided")),
            )
            .on_hover_text(t!(ui, "menu-wizard-two-sided-hover"))
            .clicked()
        {
            self.start_two_sided();
            ui.close();
        }
        if ui
            .add_enabled(self.can_inlay(), egui::Button::new(t!(ui, "inlay-menu")))
            .on_hover_text(t!(ui, "inlay-ribbon-hover"))
            .on_disabled_hover_text(t!(ui, "header-select-shapes-first"))
            .clicked()
        {
            self.start_inlay();
            ui.close();
        }
        if ui
            .button(t!(ui, "menu-wizard-relief"))
            .on_hover_text(t!(ui, "header-relief-hover"))
            .clicked()
        {
            self.start_relief();
            ui.close();
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
        self.update_help_items(ui);
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Pieces in all, from every part that is cut (docs/specs/parts.md §8).
            let copies = self
                .edit
                .as_ref()
                .map(|e| {
                    e.job
                        .parts
                        .iter()
                        .filter(|p| p.enabled)
                        .map(|p| p.count())
                        .sum::<u32>()
                })
                .filter(|&n| n > 1);
            // The units, the post and the cycle time once generated, at the far right
            // (docs/specs/browser-and-layout.md §6).
            let post = self.edit.as_ref().map(|e| e.job.machine.post.clone());
            // What the toolpath is of: one part of several, or all of them
            // (docs/specs/active-part-preview.md §4).
            let scope = self.edit.as_ref().and_then(|e| {
                let parts = &e.job.parts;
                (parts.len() > 1).then(|| match self.toolpath.part {
                    Some(id) => {
                        let k = parts.iter().position(|p| p.id == id).map_or(0, |k| k + 1);
                        t!(ui, "menu-status-part-of", part = k, count = parts.len())
                    }
                    None => t!(ui, "menu-status-all-parts"),
                })
            });
            let time = self.toolpath.result.as_ref().map(|r| {
                let time = chipload_pipeline::estimate::duration_message(r.estimate.total)
                    .render(ui.lang(), self.prefs.units.into());
                let cycle = t!(ui, "menu-status-cycle", time = time);
                match &scope {
                    Some(scope) => format!("{scope} · {cycle}"),
                    None => cycle,
                }
            });
            let grid = (self.prefs.show_grid && self.edit.is_some())
                .then(|| grid_step(self.view.scale, self.prefs.units));
            // With two parts or more, the one new operations go into (docs/specs/parts.md §8),
            // unless the cycle time already names it.
            let active = self.edit.as_ref().and_then(|e| {
                let parts = &e.job.parts;
                let id = e.job.active_part_id()?;
                (parts.len() > 1
                    && !(self.toolpath.result.is_some() && self.toolpath.part == Some(id)))
                .then(|| parts.iter().find(|p| p.id == id).map(|p| p.name.clone()))?
            });
            let several = self.edit.as_ref().is_some_and(|e| e.job.parts.len() > 1);
            let zero = self
                .edit
                .as_ref()
                .map_or((0.0, 0.0), crate::editor::EditJob::work_zero_xy);
            let cursor = self.cursor;
            let units = self.prefs.units;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // A newer version, at the far right (docs/specs/update-notice.md §2).
                self.update_notice_status(ui);
                if let Some(name) = active {
                    chip(ui, &t!(ui, "menu-status-active-part", name = name))
                        .on_hover_text(t!(ui, "menu-status-active-part-hover"));
                }
                if let Some(post) = post {
                    chip(ui, chipload_pipeline::units::length_unit(units))
                        .on_hover_text(t!(ui, "menu-status-units-hover"));
                    if let Some(step) = grid {
                        let text = t!(
                            ui,
                            "menu-status-grid",
                            step = chipload_pipeline::units::length_text(step, units)
                        );
                        chip(ui, &text).on_hover_text(t!(ui, "menu-status-grid-hover"));
                    }
                    chip(ui, &post).on_hover_text(t!(ui, "menu-status-post-hover"));
                }
                if let Some(time) = time {
                    chip(ui, &time).on_hover_text(t!(ui, "menu-status-time-hover"));
                }
                if let Some(n) = copies {
                    // With parts, what comes off the sheet (docs/specs/parts.md §8).
                    chip(
                        ui,
                        &if several {
                            t!(ui, "menu-status-pieces", count = n)
                        } else {
                            t!(ui, "menu-status-copies", count = n)
                        },
                    );
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
        if self.sketch().is_none_or(|s| s.tool != Tool::Polygon) {
            return;
        }
        ui.label(t!(ui, "menu-polygon-sides"));
        if ui
            .add(
                egui::DragValue::new(&mut self.canvas_tools.polygon_sides)
                    .range(chipload_job::POLYGON_SIDES)
                    .speed(0.1),
            )
            .changed()
        {
            let sides = self.canvas_tools.polygon_sides;
            if let Some(sketch) = self.sketch_mut() {
                sketch.sides = sides;
            }
        }
        ui.separator();
    }

    /// While a drawing tool is active: whether it draws construction geometry (key K).
    fn construction_field(&mut self, ui: &mut egui::Ui) {
        if self.sketch().is_none() {
            return;
        }
        let label = with_keys(&t!(ui, "menu-construction"), Action::Construction);
        ui.checkbox(&mut self.canvas_tools.construction_mode, label)
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
        } else if let (Some(sketch), Some((at, name))) = (&self.sketch(), &snapped) {
            let step = grid_step(self.view.scale, self.prefs.units);
            ui.label(format!(
                "{}{name}",
                sketch.status(*at, step, self.prefs.units, ui)
            ));
        } else if let (true, Some((at, name))) = (
            self.drag.handle.is_some() || self.drag.moving.is_some(),
            &snapped,
        ) {
            ui.label(t!(
                ui,
                "menu-dragging",
                x = crate::draw::fixed(at.0, self.prefs.units),
                y = crate::draw::fixed(at.1, self.prefs.units),
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
        } else if self.edit.is_some() && self.display.simulate {
            hint_line(ui, &t!(ui, "menu-status-simulate-hint"));
        } else if self.edit.is_some() {
            // Space + drag first (the user, 2026-11-03: never seen before).
            hint_line(ui, &t!(ui, "menu-status-hint"));
        }
    }
}

/// A fact at the status bar's right, in a chip (docs/specs/look-and-feel.md §8).
fn chip(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let fill = crate::style::palette(ui.visuals()).panel;
    egui::Frame::new()
        .fill(fill)
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(7, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(ui.visuals().weak_text_color()));
        })
        .response
}

/// A hint parted by " · ", its [keys] drawn as key caps; items that don't fit are left out
/// whole, after "…", the whole hint in the hover (docs/specs/look-and-feel.md §8).
fn hint_line(ui: &mut egui::Ui, text: &str) {
    let plain = text.replace(['[', ']'], "");
    let room = ui.available_width();
    let gap = 6.0;
    let shown = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let mut used = 0.0;
        for (i, item) in text.split(" · ").enumerate() {
            let pieces = hint_pieces(item);
            let width: f32 = pieces
                .iter()
                .map(|(text, key)| {
                    text_width(ui, text) + if *key { 2.0 * KEY_PAD + 3.0 } else { 3.0 }
                })
                .sum::<f32>()
                + if i > 0 {
                    gap + text_width(ui, "·")
                } else {
                    0.0
                };
            if used + width > room - text_width(ui, " …") {
                if i > 0 {
                    ui.weak("…");
                }
                break;
            }
            used += width;
            if i > 0 {
                ui.add_space(gap / 2.0);
                ui.weak("·");
                ui.add_space(gap / 2.0);
            }
            for (text, key) in pieces {
                if key {
                    key_cap(ui, &text);
                } else {
                    ui.weak(text);
                }
            }
        }
    });
    shown.response.on_hover_text(plain);
}

/// Room inside a key cap, each side, points.
const KEY_PAD: f32 = 4.0;

/// An item of a hint in pieces: (text, whether it is in [brackets]).
fn hint_pieces(item: &str) -> Vec<(String, bool)> {
    let mut pieces = Vec::new();
    let mut rest = item;
    while let Some(open) = rest.find('[') {
        let before = rest[..open].trim();
        if !before.is_empty() {
            pieces.push((before.to_string(), false));
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            rest = after;
            break;
        };
        pieces.push((after[..close].to_string(), true));
        rest = &after[close + 1..];
    }
    let rest = rest.trim();
    if !rest.is_empty() {
        pieces.push((rest.to_string(), false));
    }
    pieces
}

fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Body,
        )
        .size()
        .x
}

/// Keys or a mouse action as a key cap.
fn key_cap(ui: &mut egui::Ui, text: &str) {
    let visuals = ui.visuals();
    let fill = crate::style::palette(visuals).button;
    let stroke = egui::Stroke::new(1.0, crate::style::palette(visuals).divider);
    let color = visuals.text_color();
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(KEY_PAD as i8 - 1, 0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(color));
        });
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

#[cfg(test)]
mod hint_tests {
    use super::hint_pieces;

    #[test]
    fn a_hint_item_is_cut_into_its_key_caps_and_its_words() {
        let piece = |t: &str, key| (t.to_string(), key);
        assert_eq!(
            hint_pieces("[Space+drag] moves the view"),
            [piece("Space+drag", true), piece("moves the view", false)]
        );
        assert_eq!(
            hint_pieces("un [clic] sélectionne"),
            [
                piece("un", false),
                piece("clic", true),
                piece("sélectionne", false)
            ]
        );
        assert_eq!(hint_pieces("no keys"), [piece("no keys", false)]);
        // An unclosed bracket is kept as words, never lost.
        assert_eq!(
            hint_pieces("odd [key"),
            [piece("odd", false), piece("key", false)]
        );
    }
}
