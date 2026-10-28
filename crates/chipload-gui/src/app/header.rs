// SPDX-License-Identifier: GPL-3.0-or-later
//! The header (docs/specs/browser-and-layout.md §5): a slim top bar (name, quick icons, the
//! project's tab, settings and help) over a ribbon of grouped buttons. Each group's caption
//! opens the whole menu it stands for, so every menu item stays reachable.

use eframe::egui::{Color32, Rect, Response, WidgetInfo, WidgetType, vec2};

use super::shortcuts::{Action, hover_keys};
use super::*;
use crate::icons::{self, Icon};

/// The top bar's height, points.
pub(super) const TOP_BAR_HEIGHT: f32 = 30.0;
/// A ribbon button's height, and its narrowest width, points.
const RIBBON_BUTTON: (f32, f32) = (44.0, 46.0);
/// Where the ribbon keeps whether its large buttons show their icons alone (they don't fit
/// with their names).
const RIBBON_COMPACT: &str = "ribbon-compact";
/// A large button's width with its icon alone, points.
pub(super) const RIBBON_COMPACT_WIDTH: f32 = 32.0;
/// A ribbon button's icon, points.
const RIBBON_ICON: f32 = 20.0;
/// A top bar icon button, points.
const QUICK_BUTTON: f32 = 24.0;

impl ChiploadApp {
    /// What Export writes: every part that is cut, which the preview may not show
    /// (docs/specs/active-part-preview.md §2).
    fn export_hover(&self) -> String {
        let Some(edit) = &self.edit else {
            return t!(self, "header-export-hover");
        };
        let cut = edit.settings.parts.iter().filter(|p| p.enabled).count();
        let shown = self
            .result_part
            .and_then(|id| edit.settings.part(id))
            .map(|p| p.name.as_str());
        match (cut, shown) {
            (n, Some(name)) if n > 1 => {
                t!(
                    self,
                    "header-export-hover-parts-shown",
                    count = n,
                    part = name
                )
            }
            (n, _) if n > 1 => t!(self, "header-export-hover-parts", count = n),
            _ => t!(self, "header-export-hover"),
        }
    }

    /// The top bar: the name, Open, Save, Undo, Redo; the project's tab in the middle;
    /// Settings, Help and the version at the right.
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let logo = logo_color(ui.visuals());
            let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
            icons::paint_logo(ui.painter(), rect, logo);
            ui.add_space(4.0);
            ui.label(RichText::new("Chipload").strong());
            ui.add_space(14.0);
            let has_job = self.edit.is_some();
            let hover = hover_keys(&t!(ui, "header-open-hover"), Action::Open);
            if quick_button(ui, Icon::Open, &hover, true).clicked() {
                self.ask_open();
            }
            let hover = hover_keys(&t!(ui, "header-save-hover"), Action::Save);
            if quick_button(ui, Icon::Save, &hover, has_job).clicked() {
                self.ask_save(false);
            }
            let hover = hover_keys(&t!(ui, "header-undo"), Action::Undo);
            if quick_button(ui, Icon::Undo, &hover, !self.undo.is_empty()).clicked() {
                self.undo();
            }
            let hover = hover_keys(&t!(ui, "header-redo"), Action::Redo);
            if quick_button(ui, Icon::Redo, &hover, !self.redo.is_empty()).clicked() {
                self.redo();
            }
            let left_end = ui.min_rect().right();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .weak(),
                );
                ui.add_space(6.0);
                let label = t!(ui, "header-help");
                let help = quick_button(ui, Icon::Help, &label, true);
                egui::Popup::menu(&help).show(|ui| self.help_menu(ui));
                // The colour theme: a click goes on to the next (docs/specs/
                // view-and-settings.md §3).
                let (icon, hover, next) = match self.theme {
                    Theme::System => (
                        Icon::ThemeSystem,
                        t!(ui, "header-theme-system-hover"),
                        Theme::Light,
                    ),
                    Theme::Light => (
                        Icon::ThemeLight,
                        t!(ui, "header-theme-light-hover"),
                        Theme::Dark,
                    ),
                    Theme::Dark => (
                        Icon::ThemeDark,
                        t!(ui, "header-theme-dark-hover"),
                        Theme::System,
                    ),
                };
                if quick_button(ui, icon, &hover, true).clicked() {
                    self.set_theme(next);
                }
                let right_start = ui.min_rect().left();
                self.project_tab(ui, full, left_end, right_start);
            });
        });
    }

    /// The open project's name on a tab in the middle of the top bar; a dot while it has
    /// unsaved changes.
    fn project_tab(&self, ui: &mut egui::Ui, full: Rect, left: f32, right: f32) {
        if self.edit.is_none() {
            return;
        }
        let name = self
            .job_path
            .as_deref()
            .map_or_else(|| t!(ui, "header-new-project"), file_name);
        let dirty = self.dirty();
        let galley = egui::WidgetText::from(RichText::new(&name)).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (right - left - 60.0).clamp(40.0, 360.0),
            egui::TextStyle::Body,
        );
        let width = galley.size().x + if dirty { 40.0 } else { 28.0 };
        let mut rect = Rect::from_center_size(
            egui::pos2(
                full.center().x,
                full.bottom() - (TOP_BAR_HEIGHT - 6.0) / 2.0,
            ),
            vec2(width, TOP_BAR_HEIGHT - 6.0),
        );
        // Off the middle when the window is too narrow for both sides.
        if rect.left() < left + 8.0 {
            rect = rect.translate(vec2(left + 8.0 - rect.left(), 0.0));
        }
        if rect.right() > right - 8.0 {
            return;
        }
        let painter = ui.painter();
        let visuals = ui.visuals();
        painter.rect_filled(
            rect,
            egui::CornerRadius {
                nw: 6,
                ne: 6,
                sw: 0,
                se: 0,
            },
            visuals.panel_fill,
        );
        let text_at = egui::pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0);
        painter.galley(text_at, galley.clone(), visuals.text_color());
        if dirty {
            let dot = egui::pos2(rect.right() - 14.0, rect.center().y);
            painter.circle_filled(dot, 3.5, visuals.text_color());
        }
        let hover = match (&self.job_path, dirty) {
            (Some(path), true) => t!(
                ui,
                "header-tab-unsaved-hover",
                path = path.display().to_string()
            ),
            (Some(path), false) => path.display().to_string(),
            (None, _) => t!(ui, "header-tab-not-saved-hover"),
        };
        ui.interact(rect, egui::Id::new("project-tab"), Sense::hover())
            .on_hover_text(hover);
    }

    /// A ribbon group: its buttons in a row, and under them its caption, which opens
    /// `menu`.
    fn group(
        &mut self,
        ui: &mut egui::Ui,
        caption: &str,
        buttons: impl FnOnce(&mut Self, &mut egui::Ui),
        menu: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let row = ui.horizontal(|ui| buttons(self, ui)).response;
            let text = RichText::new(format!("{} ▾", caption.to_uppercase()))
                .small()
                .color(ui.visuals().weak_text_color());
            // A caption wider than its buttons (a one-button group in a longer language)
            // widens the group rather than wrapping.
            let caption_width = egui::WidgetText::from(text.clone())
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    egui::TextStyle::Small,
                )
                .size()
                .x;
            ui.allocate_ui_with_layout(
                vec2(row.rect.width().max(caption_width + 8.0), 18.0),
                Layout::top_down(Align::Center),
                |ui| {
                    let response = ui
                        .add(
                            egui::Button::new(text)
                                .frame(false)
                                .wrap_mode(egui::TextWrapMode::Extend),
                        )
                        .on_hover_text(t!(ui, "header-group-hover", group = caption));
                    egui::Popup::menu(&response).show(|ui| menu(self, ui));
                },
            );
        });
    }

    /// The ribbon: FILE, DRAW, OPERATION, LIBRARY and VIEW groups, Generate and Export at the
    /// right. The groups get the width Generate and Export leave. When their buttons with
    /// names don't fit, as in a longer language or a narrow window, the large buttons show
    /// their icons alone, named on hover; narrower still, the groups scroll.
    pub(super) fn ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        let actions = self.ribbon_actions(ui);
        let mut clicked = None;
        let compact_id = egui::Id::new(RIBBON_COMPACT);
        let full_id = egui::Id::new("ribbon-full-width");
        let compact = ui.data(|d| d.get_temp::<bool>(compact_id)).unwrap_or(false);
        let language = self.language;
        ui.horizontal(|ui| {
            let groups = vec2(
                (ui.available_width() - actions.width(ui)).max(0.0),
                RIBBON_BUTTON.0 + 20.0,
            );
            let used = ui
                .allocate_ui_with_layout(groups, Layout::left_to_right(Align::Min), |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("ribbon")
                        .show(ui, |ui| ui.horizontal(|ui| self.ribbon_groups(ui)))
                        .content_size
                        .x
                })
                .inner;
            // With names, the width they took is what the groups need; with icons alone, the
            // width last measured with names (in this language) says when names fit again.
            let next = if compact {
                let full = ui.data(|d| d.get_temp::<(crate::i18n::Language, f32)>(full_id));
                match full {
                    Some((measured_in, width)) if measured_in == language => width > groups.x,
                    _ => false,
                }
            } else {
                ui.data_mut(|d| d.insert_temp(full_id, (language, used)));
                used > groups.x + 0.5
            };
            if next != compact {
                ui.data_mut(|d| d.insert_temp(compact_id, next));
                ui.ctx().request_repaint();
            }
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                clicked = actions.show(ui);
            });
        });
        match clicked {
            Some(RibbonAction::Export) => self.export_when_ready(),
            Some(RibbonAction::Generate) => {
                self.generate();
            }
            Some(RibbonAction::Cancel) => self.cancel_generation(),
            None => {}
        }
        ui.add_space(2.0);
    }

    /// What Generate and Export show this frame.
    fn ribbon_actions(&self, ui: &egui::Ui) -> RibbonActions {
        let has_result = self.result.is_some();
        let export_hover = if has_result {
            self.export_hover()
        } else if self.generating() {
            t!(ui, "header-export-generating-hover")
        } else {
            t!(ui, "header-export-first-hover")
        };
        RibbonActions {
            can_export: has_result || self.generating(),
            export_hover,
            generating: self.generating(),
            can_generate: self.can_generate(),
        }
    }

    /// The ribbon's groups, left to right.
    fn ribbon_groups(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 2.0;
        let has_job = self.edit.is_some();
        let caption = t!(ui, "header-group-file");
        self.group(
            ui,
            &caption,
            |app, ui| {
                let label = t!(ui, "header-new");
                if ribbon_button(ui, Icon::New, &label, true, false)
                    .on_hover_text(hover_keys(&t!(ui, "header-new-hover"), Action::New))
                    .clicked()
                {
                    app.ask_new();
                }
                let label = t!(ui, "header-open");
                if ribbon_button(ui, Icon::Open, &label, true, false)
                    .on_hover_text(hover_keys(&t!(ui, "header-open-hover"), Action::Open))
                    .clicked()
                {
                    app.ask_open();
                }
                let label = t!(ui, "header-import");
                if ribbon_button(ui, Icon::Import, &label, true, false)
                    .on_hover_text(hover_keys(&t!(ui, "header-import-hover"), Action::Import))
                    .clicked()
                {
                    app.ask_import();
                }
                let label = t!(ui, "header-save");
                if ribbon_button(ui, Icon::Save, &label, has_job, false)
                    .on_hover_text(hover_keys(&t!(ui, "header-save-hover"), Action::Save))
                    .clicked()
                {
                    app.ask_save(false);
                }
            },
            |app, ui| {
                app.file_menu(ui);
                ui.separator();
                app.undo_menu(ui);
            },
        );
        ui.separator();
        let caption = t!(ui, "header-group-draw");
        self.group(
            ui,
            &caption,
            |app, ui| {
                let can_draw = app.can_draw();
                // The shape tools as small icon buttons, three to a row, each named with
                // its key on hover (docs/specs/drawing-input.md §2).
                let tools = [
                    (
                        Tool::Circle,
                        Icon::Circle,
                        t!(ui, "header-circle"),
                        t!(ui, "header-circle-hover"),
                        Action::DrawCircle,
                    ),
                    (
                        Tool::Rectangle,
                        Icon::Rectangle,
                        t!(ui, "header-rectangle"),
                        t!(ui, "header-rectangle-hover"),
                        Action::DrawRectangle,
                    ),
                    (
                        Tool::Polyline,
                        Icon::Polyline,
                        t!(ui, "header-polyline"),
                        t!(ui, "header-polyline-hover"),
                        Action::DrawPolyline,
                    ),
                    (
                        Tool::Arc,
                        Icon::Arc,
                        t!(ui, "header-arc"),
                        t!(ui, "header-arc-hover"),
                        Action::DrawArc,
                    ),
                    (
                        Tool::Slot,
                        Icon::Slot,
                        t!(ui, "header-slot"),
                        t!(ui, "header-slot-hover"),
                        Action::DrawSlot,
                    ),
                    (
                        Tool::Polygon,
                        Icon::Polygon,
                        t!(ui, "header-polygon"),
                        t!(ui, "header-polygon-hover"),
                        Action::DrawPolygon,
                    ),
                ];
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                    for row in tools.chunks(3) {
                        ui.horizontal(|ui| {
                            for (tool, icon, label, hint, action) in row {
                                let active = app.sketch.as_ref().is_some_and(|s| s.tool == *tool);
                                let hint = format!("{label}: {hint}");
                                if small_ribbon_button(ui, *icon, label, can_draw, active)
                                    .on_hover_text(hover_keys(&hint, *action))
                                    .clicked()
                                {
                                    app.start_drawing(*tool);
                                }
                            }
                        });
                    }
                });
                let label = t!(ui, "header-text");
                if ribbon_button(ui, Icon::Text, &label, can_draw, app.placing_text())
                    .on_hover_text(t!(ui, "header-text-hover"))
                    .clicked()
                {
                    app.start_text();
                }
                let label = t!(ui, "header-image");
                if ribbon_button(ui, Icon::Image, &label, true, false)
                    .on_hover_text(t!(ui, "header-image-hover"))
                    .clicked()
                {
                    app.import_image_dialog();
                }
                let adding = has_job && app.dxf_count() > 0;
                let label = t!(ui, "header-add-dxf");
                if ribbon_button(ui, Icon::Dxf, &label, adding, false)
                    .on_hover_text(if adding {
                        t!(ui, "header-add-dxf-hover")
                    } else {
                        t!(ui, "header-add-dxf-first-hover")
                    })
                    .clicked()
                {
                    app.ask_add_dxf();
                }
            },
            |app, ui| {
                app.draw_menu(ui);
                ui.separator();
                if ui.button(t!(ui, "header-import-image-menu")).clicked() {
                    app.import_image_dialog();
                }
                if ui
                    .add_enabled(
                        app.edit.is_some() && app.dxf_count() > 0,
                        egui::Button::new(t!(ui, "header-add-dxf-menu")),
                    )
                    .clicked()
                {
                    app.ask_add_dxf();
                }
                ui.separator();
                app.shape_menu(ui);
            },
        );
        ui.separator();
        // Editing drawn shapes (docs/specs/drawing-editing.md §8): small buttons in two rows.
        let caption = t!(ui, "header-group-modify");
        self.group(
            ui,
            &caption,
            |app, ui| {
                let can_draw = app.can_draw();
                let tools = [
                    (
                        super::modify::ModifyTool::Offset,
                        Icon::Offset,
                        t!(ui, "header-offset"),
                        t!(ui, "header-offset-hover"),
                        Some(Action::Offset),
                    ),
                    (
                        super::modify::ModifyTool::Fillet,
                        Icon::Fillet,
                        t!(ui, "header-fillet"),
                        t!(ui, "header-fillet-hover"),
                        None,
                    ),
                    (
                        super::modify::ModifyTool::Chamfer,
                        Icon::Chamfer,
                        t!(ui, "header-chamfer"),
                        t!(ui, "header-chamfer-hover"),
                        None,
                    ),
                    (
                        super::modify::ModifyTool::Trim,
                        Icon::Trim,
                        t!(ui, "header-trim"),
                        t!(ui, "header-trim-hover"),
                        Some(Action::Trim),
                    ),
                    (
                        super::modify::ModifyTool::Extend,
                        Icon::Extend,
                        t!(ui, "header-extend"),
                        t!(ui, "header-extend-hover"),
                        None,
                    ),
                    (
                        super::modify::ModifyTool::Break,
                        Icon::Break,
                        t!(ui, "header-break"),
                        t!(ui, "header-break-hover"),
                        None,
                    ),
                    (
                        super::modify::ModifyTool::Union,
                        Icon::Union,
                        t!(ui, "header-union"),
                        t!(ui, "header-union-hover"),
                        None,
                    ),
                    (
                        super::modify::ModifyTool::Subtract,
                        Icon::Subtract,
                        t!(ui, "header-subtract"),
                        t!(ui, "header-subtract-hover"),
                        None,
                    ),
                    (
                        super::modify::ModifyTool::Intersect,
                        Icon::Intersect,
                        t!(ui, "header-intersect"),
                        t!(ui, "header-intersect-hover"),
                        None,
                    ),
                ];
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                    for row in tools.chunks(tools.len().div_ceil(2)) {
                        ui.horizontal(|ui| {
                            for (tool, icon, label, hint, action) in row {
                                let active = app.modify == Some(*tool);
                                let hint = format!("{label}: {hint}");
                                let hint = match action {
                                    Some(action) => hover_keys(&hint, *action),
                                    None => hint,
                                };
                                let enabled = can_draw && app.modify_ready(*tool);
                                if small_ribbon_button(ui, *icon, label, enabled, active)
                                    .on_hover_text(hint)
                                    .clicked()
                                {
                                    app.start_modify(*tool);
                                }
                            }
                        });
                    }
                });
            },
            |app, ui| app.modify_menu(ui),
        );
        ui.separator();
        let caption = t!(ui, "header-group-operation");
        self.group(
            ui,
            &caption,
            |app, ui| {
                let selected = !app.selection.ids(&app.contours).is_empty();
                for (label, kind, hover) in super::browser::new_operations(ui) {
                    let icon = match kind {
                        OpKind::Profile => Icon::Profile,
                        OpKind::Pocket => Icon::Pocket,
                        OpKind::Vcarve => Icon::Vcarve,
                        OpKind::Path => Icon::FollowPath,
                        OpKind::Holes => Icon::Holes,
                        OpKind::Surface => Icon::Surface,
                    };
                    let short = if kind == OpKind::Path {
                        t!(ui, "header-follow")
                    } else {
                        label
                    };
                    let enabled = selected || !kind.needs_selection();
                    let response = ribbon_button(ui, icon, &short, enabled, false);
                    let response = if enabled {
                        response.on_hover_text(hover)
                    } else {
                        response.on_hover_text(t!(
                            ui,
                            "header-needs-selection-hover",
                            operation = hover
                        ))
                    };
                    if response.clicked() {
                        app.add_operation(kind);
                    }
                }
            },
            |app, ui| {
                let selected = !app.selection.ids(&app.contours).is_empty();
                for (_, kind, hover) in super::browser::new_operations(ui) {
                    if ui
                        .add_enabled(
                            selected || !kind.needs_selection(),
                            egui::Button::new(new_operation(ui, kind)),
                        )
                        .on_hover_text(hover)
                        .on_disabled_hover_text(t!(ui, "header-select-shapes-first"))
                        .clicked()
                    {
                        app.add_operation(kind);
                    }
                }
                ui.separator();
                app.job_menu(ui);
            },
        );
        ui.separator();
        let caption = t!(ui, "header-group-library");
        self.group(
            ui,
            &caption,
            |app, ui| {
                let label = t!(ui, "header-tools");
                if ribbon_button(ui, Icon::Tools, &label, true, false)
                    .on_hover_text(t!(ui, "header-tools-hover"))
                    .clicked()
                {
                    app.show_library(LibraryKind::Tools, None);
                }
                let label = t!(ui, "header-machines");
                if ribbon_button(ui, Icon::Machine, &label, true, false)
                    .on_hover_text(t!(ui, "header-machines-hover"))
                    .clicked()
                {
                    app.show_library(LibraryKind::Machines, None);
                }
            },
            |app, ui| {
                if ui.button(t!(ui, "header-tool-library")).clicked() {
                    app.show_library(LibraryKind::Tools, None);
                }
                if ui.button(t!(ui, "header-machines")).clicked() {
                    app.show_library(LibraryKind::Machines, None);
                }
            },
        );
        ui.separator();
        let caption = t!(ui, "header-group-view");
        self.group(
            ui,
            &caption,
            |app, ui| {
                let label = t!(ui, "header-measure");
                if ribbon_button(ui, Icon::Measure, &label, has_job, app.measure.is_some())
                    .on_hover_text(hover_keys(&t!(ui, "header-measure-hover"), Action::Measure))
                    .clicked()
                {
                    app.toggle_measure();
                }
            },
            |app, ui| app.view_menu(ui),
        );
    }
}

/// A click on Generate, Cancel or Export, acted on after the ribbon is drawn.
enum RibbonAction {
    Generate,
    Cancel,
    Export,
}

/// Generate (or Cancel) and Export, at the ribbon's right.
struct RibbonActions {
    can_export: bool,
    export_hover: String,
    generating: bool,
    can_generate: bool,
}

impl RibbonActions {
    /// The width Export and Generate (or Cancel) take, spacing included.
    fn width(&self, ui: &egui::Ui) -> f32 {
        let label = if self.generating {
            t!(ui, "header-cancel")
        } else {
            t!(ui, "header-generate")
        };
        let export = ribbon_button_width(ui, &t!(ui, "header-export"));
        let generate = generate_button_width(ui, &label);
        RIBBON_ACTIONS_SPACING + export + generate
    }

    fn show(&self, ui: &mut egui::Ui) -> Option<RibbonAction> {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(4.0);
        let mut clicked = None;
        let label = t!(ui, "header-export");
        if ribbon_button(ui, Icon::Export, &label, self.can_export, false)
            .on_hover_text(hover_keys(&self.export_hover, Action::Export))
            .clicked()
        {
            clicked = Some(RibbonAction::Export);
        }
        if self.generating {
            let label = t!(ui, "header-cancel");
            if generate_button(ui, Icon::Stop, &label, true)
                .on_hover_text(t!(ui, "header-cancel-hover"))
                .clicked()
            {
                clicked = Some(RibbonAction::Cancel);
            }
        } else {
            let label = t!(ui, "header-generate");
            if generate_button(ui, Icon::Generate, &label, self.can_generate)
                .on_hover_text(hover_keys(
                    &t!(ui, "header-generate-hover"),
                    Action::Generate,
                ))
                .clicked()
            {
                clicked = Some(RibbonAction::Generate);
            }
        }
        clicked
    }
}

/// "New Profile" and the like: a menu item that adds an operation of `kind`.
fn new_operation(ui: &egui::Ui, kind: OpKind) -> String {
    match kind {
        OpKind::Profile => t!(ui, "header-new-profile"),
        OpKind::Pocket => t!(ui, "header-new-pocket"),
        OpKind::Vcarve => t!(ui, "header-new-vcarve"),
        OpKind::Path => t!(ui, "header-new-follow-path"),
        OpKind::Holes => t!(ui, "header-new-holes"),
        OpKind::Surface => t!(ui, "header-new-surface"),
    }
}

/// Room around Export and Generate: the gap to the groups, between the two buttons and at
/// the window's edge.
const RIBBON_ACTIONS_SPACING: f32 = 12.0;

/// A ribbon button's label, laid out once for its width and again to paint.
fn ribbon_label(ui: &egui::Ui, label: &str) -> std::sync::Arc<egui::Galley> {
    egui::WidgetText::from(RichText::new(label).small()).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    )
}

fn ribbon_button_width(ui: &egui::Ui, label: &str) -> f32 {
    (ribbon_label(ui, label).size().x + 6.0).max(RIBBON_BUTTON.1)
}

fn generate_label(ui: &egui::Ui, label: &str) -> std::sync::Arc<egui::Galley> {
    egui::WidgetText::from(RichText::new(label).strong()).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    )
}

fn generate_button_width(ui: &egui::Ui, label: &str) -> f32 {
    generate_label(ui, label).size().x + 38.0
}

/// A ribbon button: an icon over a short word; when the ribbon is compact, the icon alone,
/// named in bold on hover above whatever else its hover says. Disabled ones are dimmed and
/// don't click.
pub(super) fn ribbon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
) -> Response {
    let compact = ui
        .data(|d| d.get_temp::<bool>(egui::Id::new(RIBBON_COMPACT)))
        .unwrap_or(false);
    let galley = ribbon_label(ui, label);
    let width = if compact {
        RIBBON_COMPACT_WIDTH
    } else {
        ribbon_button_width(ui, label)
    };
    let size = vec2(width, RIBBON_BUTTON.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, selected, label));
    let visuals = ui.visuals();
    let painter = ui.painter();
    let corner = visuals.widgets.inactive.corner_radius;
    let color = if selected {
        painter.rect_filled(rect, corner, visuals.selection.bg_fill);
        visuals.selection.stroke.color
    } else {
        if enabled && response.hovered() {
            painter.rect_filled(rect, corner, visuals.widgets.hovered.weak_bg_fill);
        }
        if enabled {
            visuals.text_color()
        } else {
            visuals.weak_text_color().gamma_multiply(0.7)
        }
    };
    if compact {
        let icon_rect = Rect::from_center_size(rect.center(), vec2(RIBBON_ICON, RIBBON_ICON));
        icons::paint(painter, icon_rect, icon, color);
        let name = |ui: &mut egui::Ui| {
            ui.strong(label);
        };
        return response.on_hover_ui(name).on_disabled_hover_ui(name);
    }
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 4.0 + RIBBON_ICON / 2.0),
        vec2(RIBBON_ICON, RIBBON_ICON),
    );
    icons::paint(painter, icon_rect, icon, color);
    let at = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.bottom() - 3.0 - galley.size().y,
    );
    painter.galley(at, galley, color);
    response
}

/// A small ribbon button, two to the height of a large one: an icon alone, named on hover.
fn small_ribbon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
) -> Response {
    let side = (RIBBON_BUTTON.0 - 2.0) / 2.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(side + 4.0, side), sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, selected, label));
    let visuals = ui.visuals();
    let painter = ui.painter();
    let corner = visuals.widgets.inactive.corner_radius;
    let color = if selected {
        painter.rect_filled(rect, corner, visuals.selection.bg_fill);
        visuals.selection.stroke.color
    } else {
        if enabled && response.hovered() {
            painter.rect_filled(rect, corner, visuals.widgets.hovered.weak_bg_fill);
        }
        if enabled {
            visuals.text_color()
        } else {
            visuals.weak_text_color().gamma_multiply(0.7)
        }
    };
    let icon_rect = Rect::from_center_size(rect.center(), vec2(side - 4.0, side - 4.0));
    icons::paint(painter, icon_rect, icon, color);
    response
}

/// Generate (or Cancel): the one accent-coloured button, an icon beside a word.
fn generate_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> Response {
    let galley = generate_label(ui, label);
    let size = vec2(generate_button_width(ui, label), RIBBON_BUTTON.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    let visuals = ui.visuals();
    let painter = ui.painter();
    let corner = visuals.widgets.inactive.corner_radius;
    let (fill, color) = if enabled {
        let fill = if response.hovered() {
            visuals.selection.bg_fill.gamma_multiply(1.15)
        } else {
            visuals.selection.bg_fill
        };
        (fill, visuals.selection.stroke.color)
    } else {
        (
            visuals.widgets.inactive.weak_bg_fill,
            visuals.weak_text_color().gamma_multiply(0.7),
        )
    };
    painter.rect_filled(rect, corner, fill);
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 17.0, rect.center().y),
        vec2(16.0, 16.0),
    );
    icons::paint(painter, icon_rect, icon, color);
    let at = egui::pos2(rect.left() + 30.0, rect.center().y - galley.size().y / 2.0);
    painter.galley(at, galley, color);
    response
}

/// A small square icon button for the top bar.
fn quick_button(ui: &mut egui::Ui, icon: Icon, hover: &str, enabled: bool) -> Response {
    icon_button(ui, icon, hover, enabled, false, QUICK_BUTTON)
}

/// A square icon button of `side` points: for the top bar and the canvas's navigation bar.
/// A selected one is filled with the accent.
pub(crate) fn icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    hover: &str,
    enabled: bool,
    selected: bool,
    side: f32,
) -> Response {
    plain_icon_button(ui, icon, hover, enabled, selected, side).on_hover_text(hover)
}

/// As `icon_button`, without a hover text: the caller adds its own.
pub(crate) fn plain_icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    name: &str,
    enabled: bool,
    selected: bool,
    side: f32,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, selected, name));
    let visuals = ui.visuals();
    let corner = visuals.widgets.inactive.corner_radius;
    let color = if selected {
        ui.painter()
            .rect_filled(rect, corner, visuals.selection.bg_fill);
        visuals.selection.stroke.color
    } else if enabled {
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, corner, visuals.widgets.hovered.weak_bg_fill);
        }
        visuals.text_color()
    } else {
        visuals.weak_text_color().gamma_multiply(0.7)
    };
    icons::paint(ui.painter(), rect.shrink(side * 0.21), icon, color);
    response
}

/// The mark's green (packaging/brand/README.md): the brand green on dark, a deeper one
/// on light, where the brand green is too pale.
fn logo_color(visuals: &egui::Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(0x9a, 0xb9, 0x64)
    } else {
        Color32::from_rgb(0x6f, 0x8f, 0x3c)
    }
}
