// SPDX-License-Identifier: GPL-3.0-or-later
//! The header (docs/specs/browser-and-layout.md §5): a slim top bar (name, quick icons, the
//! project's tab, settings and help) over a ribbon of grouped buttons. Each group's caption
//! opens the whole menu it stands for, so every menu item stays reachable.

use eframe::egui::{Color32, Rect, Response, WidgetInfo, WidgetType, vec2};

use super::menu::SIMULATION_HINT;
use super::shortcuts::{Action, hover_keys};
use super::*;
use crate::icons::{self, Icon};

/// The top bar's height, points.
pub(super) const TOP_BAR_HEIGHT: f32 = 30.0;
/// A ribbon button's height, and its narrowest width, points.
const RIBBON_BUTTON: (f32, f32) = (44.0, 46.0);
/// A ribbon button's icon, points.
const RIBBON_ICON: f32 = 20.0;
/// A top bar icon button, points.
const QUICK_BUTTON: f32 = 24.0;

impl ChiploadApp {
    /// The top bar: the name, Open, Save, Undo, Redo; the project's tab in the middle;
    /// Settings, Help and the version at the right.
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let logo = logo_color(ui.visuals());
            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            icons::paint(ui.painter(), rect, Icon::Project, logo);
            ui.add_space(4.0);
            ui.label(RichText::new("Chipload").strong());
            ui.add_space(14.0);
            let has_job = self.edit.is_some();
            if quick_button(
                ui,
                Icon::Open,
                &hover_keys("Open a project", Action::Open),
                true,
            )
            .clicked()
            {
                self.ask_open();
            }
            if quick_button(
                ui,
                Icon::Save,
                &hover_keys("Save the project", Action::Save),
                has_job,
            )
            .clicked()
            {
                self.ask_save(false);
            }
            if quick_button(
                ui,
                Icon::Undo,
                &hover_keys("Undo", Action::Undo),
                !self.undo.is_empty(),
            )
            .clicked()
            {
                self.undo();
            }
            if quick_button(
                ui,
                Icon::Redo,
                &hover_keys("Redo", Action::Redo),
                !self.redo.is_empty(),
            )
            .clicked()
            {
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
                let help = quick_button(ui, Icon::Help, "Help", true);
                egui::Popup::menu(&help).show(|ui| self.help_menu(ui));
                let settings = quick_button(
                    ui,
                    Icon::Settings,
                    "Settings: tool library, machines, setup sheet, colour theme",
                    true,
                );
                egui::Popup::menu(&settings).show(|ui| self.settings_menu(ui));
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
            .map_or("New project".to_string(), file_name);
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
            (Some(path), true) => format!("{}\nUnsaved changes", path.display()),
            (Some(path), false) => path.display().to_string(),
            (None, _) => "Not saved yet".to_string(),
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
            ui.allocate_ui_with_layout(
                vec2(row.rect.width(), 18.0),
                Layout::top_down(Align::Center),
                |ui| {
                    let text = RichText::new(format!("{} ▾", caption.to_uppercase()))
                        .small()
                        .color(ui.visuals().weak_text_color());
                    let response = ui
                        .add(egui::Button::new(text).frame(false))
                        .on_hover_text(format!("Everything in {caption}"));
                    egui::Popup::menu(&response).show(|ui| menu(self, ui));
                },
            );
        });
    }

    /// The ribbon: FILE, DRAW, OPERATION and VIEW groups, Generate and Export at the right.
    pub(super) fn ribbon(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let has_job = self.edit.is_some();
            self.group(
                ui,
                "File",
                |app, ui| {
                    if ribbon_button(ui, Icon::New, "New", true, false)
                        .on_hover_text(hover_keys("New project", Action::New))
                        .clicked()
                    {
                        app.ask_new();
                    }
                    if ribbon_button(ui, Icon::Open, "Open", true, false)
                        .on_hover_text(hover_keys("Open a project", Action::Open))
                        .clicked()
                    {
                        app.ask_open();
                    }
                    if ribbon_button(ui, Icon::Import, "Import", true, false)
                        .on_hover_text(hover_keys("New project from a DXF", Action::Import))
                        .clicked()
                    {
                        app.ask_import();
                    }
                    if ribbon_button(ui, Icon::Save, "Save", has_job, false)
                        .on_hover_text(hover_keys("Save the project", Action::Save))
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
            self.group(
                ui,
                "Draw",
                |app, ui| {
                    let can_draw = app.can_draw();
                    for (tool, icon, label, hint) in [
                        (
                            Tool::Circle,
                            Icon::Circle,
                            "Circle",
                            "Draw a circle: centre, then a point on it",
                        ),
                        (
                            Tool::Rectangle,
                            Icon::Rectangle,
                            "Rectangle",
                            "Draw a rectangle: two opposite corners",
                        ),
                        (
                            Tool::Polyline,
                            Icon::Polyline,
                            "Polyline",
                            "Draw a polyline: click points",
                        ),
                    ] {
                        let active = app.sketch.as_ref().is_some_and(|s| s.tool == tool);
                        if ribbon_button(ui, icon, label, can_draw, active)
                            .on_hover_text(hint)
                            .clicked()
                        {
                            app.start_drawing(tool);
                        }
                    }
                    if ribbon_button(ui, Icon::Image, "Image", true, false)
                        .on_hover_text(
                            "Trace a picture into shapes, for V-carving or a follow path",
                        )
                        .clicked()
                    {
                        app.import_image_dialog();
                    }
                    let adding = has_job && app.dxf_count() > 0;
                    if ribbon_button(ui, Icon::Dxf, "Add DXF", adding, false)
                        .on_hover_text(if adding {
                            "Another part: a DXF placed and updated on its own"
                        } else {
                            "Import a DXF first (FILE > Import)"
                        })
                        .clicked()
                    {
                        app.ask_add_dxf();
                    }
                },
                |app, ui| {
                    app.draw_menu(ui);
                    ui.separator();
                    if ui.button("Import image…").clicked() {
                        app.import_image_dialog();
                    }
                    if ui
                        .add_enabled(
                            app.edit.is_some() && app.dxf_count() > 0,
                            egui::Button::new("Add DXF to project…"),
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
            self.group(
                ui,
                "Operation",
                |app, ui| {
                    let selected = !app.selection.ids(&app.contours).is_empty();
                    for (label, kind, hover) in super::browser::NEW_OPERATIONS {
                        let icon = match kind {
                            OpKind::Profile => Icon::Profile,
                            OpKind::Pocket => Icon::Pocket,
                            OpKind::Vcarve => Icon::Vcarve,
                            OpKind::Path => Icon::FollowPath,
                            OpKind::Holes => Icon::Holes,
                        };
                        let short = if kind == OpKind::Path {
                            "Follow"
                        } else {
                            label
                        };
                        let response = ribbon_button(ui, icon, short, selected, false);
                        let response = if selected {
                            response.on_hover_text(hover)
                        } else {
                            response.on_hover_text(format!("{hover}. Select shapes first."))
                        };
                        if response.clicked() {
                            app.add_operation(kind);
                        }
                    }
                },
                |app, ui| {
                    let selected = !app.selection.ids(&app.contours).is_empty();
                    for (label, kind, hover) in super::browser::NEW_OPERATIONS {
                        if ui
                            .add_enabled(selected, egui::Button::new(format!("New {label}")))
                            .on_hover_text(hover)
                            .on_disabled_hover_text("Select shapes first")
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
            self.group(
                ui,
                "View",
                |app, ui| {
                    if ribbon_button(ui, Icon::Fit, "Fit", has_job, false)
                        .on_hover_text(hover_keys("Fit the view", Action::Fit))
                        .clicked()
                    {
                        app.fit_pending = true;
                    }
                    if ribbon_button(ui, Icon::Top, "Top", has_job, !app.three_d)
                        .on_hover_text(hover_keys("Look straight down", Action::TopView))
                        .clicked()
                    {
                        app.view_top();
                    }
                    // Z stretch is labelled whenever it isn't true scale (docs/specs/3d-view.md §4).
                    let three_d = if app.view.z_scale == 1.0 {
                        "3D".to_string()
                    } else {
                        format!("3D ×{}", app.view.z_scale)
                    };
                    if ribbon_button(ui, Icon::ThreeD, &three_d, has_job, app.three_d)
                        .on_hover_text("Left-drag turns and tilts; right-drag or Shift+drag pans")
                        .clicked()
                    {
                        app.view_3d();
                    }
                    let label = if app.simulating() {
                        "Simulating"
                    } else {
                        "Simulation"
                    };
                    let can = app.result.is_some();
                    if ribbon_button(ui, Icon::Simulation, label, can, app.show_simulation)
                        .on_hover_text(if can {
                            SIMULATION_HINT
                        } else {
                            "Generate the toolpath first"
                        })
                        .clicked()
                    {
                        app.set_show_simulation(!app.show_simulation);
                    }
                    if ribbon_button(ui, Icon::Measure, "Measure", has_job, app.measure.is_some())
                        .on_hover_text(hover_keys(
                            "Two clicks give the distance, ΔX, ΔY and the angle",
                            Action::Measure,
                        ))
                        .clicked()
                    {
                        app.toggle_measure();
                    }
                },
                |app, ui| app.view_menu(ui),
            );
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(4.0);
                let has_result = self.result.is_some();
                let can_export = has_result || self.generating();
                if ribbon_button(ui, Icon::Export, "Export", can_export, false)
                    .on_hover_text(hover_keys(
                        if has_result {
                            "Save the checked G-code"
                        } else if self.generating() {
                            "Save the G-code once it is generated"
                        } else {
                            "Generate first"
                        },
                        Action::Export,
                    ))
                    .clicked()
                {
                    self.export_when_ready();
                }
                if self.generating() {
                    if generate_button(ui, Icon::Stop, "Cancel", true)
                        .on_hover_text("Stop generating")
                        .clicked()
                    {
                        self.cancel_generation();
                    }
                } else if generate_button(ui, Icon::Generate, "Generate", self.can_generate())
                    .on_hover_text(hover_keys(
                        "Plan the toolpaths, write and check the G-code",
                        Action::Generate,
                    ))
                    .clicked()
                {
                    self.generate();
                }
            });
        });
        ui.add_space(2.0);
    }
}

/// A ribbon button: an icon over a short word. Disabled ones are dimmed and don't click.
fn ribbon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
) -> Response {
    let galley = egui::WidgetText::from(RichText::new(label).small()).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    );
    let (height, narrowest) = RIBBON_BUTTON;
    let size = vec2((galley.size().x + 10.0).max(narrowest), height);
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

/// Generate (or Cancel): the one accent-coloured button, an icon beside a word.
fn generate_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> Response {
    let galley = egui::WidgetText::from(RichText::new(label).strong()).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let size = vec2(galley.size().x + 44.0, RIBBON_BUTTON.0);
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
        egui::pos2(rect.left() + 20.0, rect.center().y),
        vec2(16.0, 16.0),
    );
    icons::paint(painter, icon_rect, icon, color);
    let at = egui::pos2(rect.left() + 34.0, rect.center().y - galley.size().y / 2.0);
    painter.galley(at, galley, color);
    response
}

/// A small square icon button for the top bar.
fn quick_button(ui: &mut egui::Ui, icon: Icon, hover: &str, enabled: bool) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(QUICK_BUTTON, QUICK_BUTTON), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, hover));
    let visuals = ui.visuals();
    if enabled && response.hovered() {
        ui.painter().rect_filled(
            rect,
            visuals.widgets.inactive.corner_radius,
            visuals.widgets.hovered.weak_bg_fill,
        );
    }
    let color = if enabled {
        visuals.text_color()
    } else {
        visuals.weak_text_color().gamma_multiply(0.7)
    };
    icons::paint(ui.painter(), rect.shrink(5.0), icon, color);
    response.on_hover_text(hover)
}

/// The logo's colour: a blue that reads on the top bar in either theme.
fn logo_color(visuals: &egui::Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(0x5a, 0x9c, 0xe0)
    } else {
        Color32::from_rgb(0x1f, 0x5c, 0xb0)
    }
}
