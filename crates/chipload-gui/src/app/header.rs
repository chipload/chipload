// SPDX-License-Identifier: GPL-3.0-or-later
//! The header (docs/specs/browser-and-layout.md §5): a slim top bar (name, quick icons, the
//! project's tab, settings and help) over a ribbon of grouped buttons. Each group's caption
//! opens the whole menu it stands for, so every menu item stays reachable.

use eframe::egui::{Color32, Rect, Response, WidgetInfo, WidgetType, vec2};

use super::ribbon::new_operation;
use super::shortcuts::{Action, hover_keys};
use super::*;
use crate::icons::{self, Icon};

/// The top bar's height, points.
pub(super) const TOP_BAR_HEIGHT: f32 = 30.0;
/// A top bar icon button, points.
const QUICK_BUTTON: f32 = 24.0;
/// Room each side of a menu title in the top bar.
const MENU_TITLE_PADDING: f32 = 6.0;

impl ChiploadApp {
    /// What Export writes: every part that is cut, which the preview may not show
    /// (docs/specs/active-part-preview.md §2).
    pub(super) fn export_hover(&self) -> String {
        let Some(edit) = &self.edit else {
            return t!(self, "header-export-hover");
        };
        let cut = edit.job.parts.iter().filter(|p| p.enabled).count();
        let shown = self
            .toolpath
            .part
            .and_then(|id| edit.job.part(id))
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
            ui.label(crate::style::bold(ui, "Chipload"));
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
            if quick_button(ui, Icon::Undo, &hover, !self.history.undo.is_empty()).clicked() {
                self.undo();
            }
            let hover = hover_keys(&t!(ui, "header-redo"), Action::Redo);
            if quick_button(ui, Icon::Redo, &hover, !self.history.redo.is_empty()).clicked() {
                self.redo();
            }
            ui.add_space(10.0);
            self.menu_bar(ui);
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
                // The colour theme: a click goes on to the next (docs/specs/
                // view-and-settings.md §3).
                let (icon, hover, next) = match self.prefs.theme {
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

    /// The menu bar, in the top bar after the quick icons (docs/specs/menu-bar.md §2). egui's
    /// menu bar takes all the width it is given, so it gets its titles' width.
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        type Menu = fn(&mut ChiploadApp, &mut egui::Ui);
        let menus: [(String, Menu); 9] = [
            (t!(ui, "menu-bar-file"), |app, ui| app.file_menu(ui)),
            (t!(ui, "menu-bar-edit"), |app, ui| app.edit_menu(ui)),
            (t!(ui, "menu-bar-view"), |app, ui| app.view_menu(ui)),
            (t!(ui, "menu-bar-draw"), |app, ui| app.draw_menu(ui)),
            (t!(ui, "menu-bar-modify"), |app, ui| app.modify_menu(ui)),
            (t!(ui, "menu-bar-operation"), |app, ui| {
                app.operation_menu(ui)
            }),
            (t!(ui, "menu-bar-wizards"), |app, ui| app.wizards_menu(ui)),
            (t!(ui, "menu-bar-library"), |app, ui| app.library_menu(ui)),
            (t!(ui, "menu-bar-help"), |app, ui| app.help_menu(ui)),
        ];
        let padding = 2.0 * MENU_TITLE_PADDING;
        let width: f32 = menus
            .iter()
            .map(|(title, _)| {
                egui::WidgetText::from(title.as_str())
                    .into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::TextStyle::Button,
                    )
                    .size()
                    .x
                    + padding
            })
            .sum::<f32>()
            + ui.spacing().item_spacing.x * (menus.len() as f32 - 1.0);
        let height = ui.spacing().interact_size.y;
        ui.allocate_ui(vec2(width.ceil() + 1.0, height), |ui| {
            egui::MenuBar::new()
                .style(|style: &mut egui::Style| {
                    egui::menu::menu_style(style);
                    style.spacing.button_padding = vec2(MENU_TITLE_PADDING, 0.0);
                })
                .ui(ui, |ui| {
                    for (title, menu) in menus {
                        ui.menu_button(title, |ui| menu(self, ui));
                    }
                });
        });
    }

    /// The Operation menu: a new operation for the selection, an inlay, a style, Generate.
    pub(super) fn operation_menu(&mut self, ui: &mut egui::Ui) {
        let selected = !self.selected_ids().is_empty();
        let models = self.has_models();
        for (_, kind, hover) in super::browser::new_operations(ui) {
            if ui
                .add_enabled(
                    super::browser::can_add(kind, selected, models),
                    egui::Button::new(new_operation(ui, kind)),
                )
                .on_hover_text(hover)
                .on_disabled_hover_text(super::browser::cannot_add(ui, kind))
                .clicked()
            {
                self.add_operation(kind);
            }
        }
        submenu(ui, t!(ui, "styles-apply-menu"), |ui| {
            let mut act = None;
            self.style_menu(ui, &mut act);
            if let Some(act) = act {
                self.style_act(act);
            }
        });
        // Layer rules (docs/specs/fewer-clicks.md §4).
        let rules = self
            .user
            .library
            .styles
            .styles
            .iter()
            .any(|s| s.layer.is_some());
        if ui
            .add_enabled(
                rules && self.edit.is_some(),
                egui::Button::new(t!(ui, "styles-apply-rules")),
            )
            .on_hover_text(t!(ui, "styles-apply-rules-hover"))
            .on_disabled_hover_text(t!(ui, "styles-apply-rules-none"))
            .clicked()
        {
            self.apply_layer_rules();
            ui.close();
        }
        ui.separator();
        self.job_menu(ui);
    }

    /// The Library menu: the tool library and the machines.
    pub(super) fn library_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(t!(ui, "header-tool-library")).clicked() {
            self.show_library(LibraryKind::Tools, None);
        }
        if ui.button(t!(ui, "header-machines")).clicked() {
            self.show_library(LibraryKind::Machines, None);
        }
    }

    /// The open project's name on a tab in the middle of the top bar; a dot while it has
    /// unsaved changes.
    fn project_tab(&self, ui: &mut egui::Ui, full: Rect, left: f32, right: f32) {
        if self.edit.is_none() {
            return;
        }
        let name = self
            .file
            .path
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
            crate::style::palette(visuals).chrome,
        );
        let text_at = egui::pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0);
        painter.galley(text_at, galley.clone(), visuals.text_color());
        if dirty {
            let dot = egui::pos2(rect.right() - 14.0, rect.center().y);
            painter.circle_filled(dot, 3.5, visuals.text_color());
        }
        let hover = match (&self.file.path, dirty) {
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
}

/// A small square icon button for the top bar.
fn quick_button(ui: &mut egui::Ui, icon: Icon, hover: &str, enabled: bool) -> Response {
    icon_button(ui, icon, hover, enabled, false, QUICK_BUTTON)
}

/// A menu entry that opens a submenu, or a menu button outside a menu. Inside a menu its
/// arrow is ▸: egui's own (⏵, U+23F5) is in none of the fonts the window loads and showed
/// as an empty box (the user, 2026-11-03).
pub(crate) fn submenu<'a, R>(
    ui: &mut egui::Ui,
    label: impl egui::IntoAtoms<'a>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    if egui::containers::menu::is_in_menu(ui) {
        let button = egui::Button::new(label).right_text("▸");
        let (response, inner) =
            egui::containers::menu::SubMenuButton::from_button(button).ui(ui, add_contents);
        egui::InnerResponse::new(inner.map(|i| i.inner), response)
    } else {
        ui.menu_button(label, add_contents)
    }
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
        crate::style::disabled_color(visuals)
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
