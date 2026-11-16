// SPDX-License-Identifier: GPL-3.0-or-later
//! The ribbon: its groups of buttons, left to right, with Generate and Export at the right;
//! large buttons show their icons alone when their names don't fit.

use eframe::egui::{Color32, Rect, Response, WidgetInfo, WidgetType, vec2};

use super::shortcuts::{Action, hover_keys};
use super::*;
use crate::icons::{self, Icon};

/// A ribbon button's height, and its narrowest width, points.
const RIBBON_BUTTON: (f32, f32) = (44.0, 44.0);

/// Where the ribbon keeps whether its large buttons show their icons alone (they don't fit
/// with their names).
const RIBBON_COMPACT: &str = "ribbon-compact";

/// A large button's width with its icon alone, points.
pub(super) const RIBBON_COMPACT_WIDTH: f32 = 32.0;

/// A ribbon button's icon, points.
const RIBBON_ICON: f32 = 20.0;

/// One group of the ribbon's buttons.
type RibbonGroup = fn(&mut ChiploadApp, &mut egui::Ui);

/// The File group: new, open, import, save.
fn file_group(app: &mut ChiploadApp, ui: &mut egui::Ui) {
    let has_job = app.edit.is_some();
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
}

/// The Draw group: the shape tools, three to a row, each named with its key on hover
/// (docs/specs/drawing-input.md §2).
fn draw_group(app: &mut ChiploadApp, ui: &mut egui::Ui) {
    let has_job = app.edit.is_some();
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
        // Four a row of small buttons, so the ribbon keeps its names at the
        // default width. A frame alone is Draw > Frame… and the canvas'
        // right-click; WIZARDS > Sign makes a whole sign (the user, 2026-11-10).
        // Text is the last small button, so the sources (picture, drawing file,
        // 3D model) keep their names (the user, 2026-11-13).
        let rows = tools.chunks(4).count();
        for (index, row) in tools.chunks(4).enumerate() {
            ui.horizontal(|ui| {
                for (tool, icon, label, hint, action) in row {
                    let active = app.sketch().is_some_and(|s| s.tool == *tool);
                    let hint = format!("{label}: {hint}");
                    if small_ribbon_button(ui, *icon, label, can_draw, active, None)
                        .on_hover_text(hover_keys(&hint, *action))
                        .clicked()
                    {
                        app.start_drawing(*tool);
                    }
                }
                if index + 1 == rows {
                    let label = t!(ui, "header-text");
                    let hint = format!("{label}: {}", t!(ui, "header-text-hover"));
                    let active = app.placing_text();
                    if small_ribbon_button(ui, Icon::Text, &label, can_draw, active, None)
                        .on_hover_text(hint)
                        .clicked()
                    {
                        app.start_text();
                    }
                }
            });
        }
    });
    let label = t!(ui, "header-image");
    if ribbon_button(ui, Icon::Image, &label, true, false)
        .on_hover_text(t!(ui, "header-image-hover"))
        .clicked()
    {
        app.import_image_dialog();
    }
    // Any project takes a drawing file, its first too (the user, 2026-10-28).
    let adding = has_job;
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
    // A 3D model for a relief, beside the picture and the drawing files: found where
    // the other sources are (the user, 2026-11-13).
    let label = t!(ui, "header-model");
    if ribbon_button(ui, Icon::Model, &label, true, false)
        .on_hover_text(t!(ui, "header-model-hover"))
        .clicked()
    {
        app.add_model_dialog();
    }
}

/// The Modify group: editing drawn shapes (docs/specs/drawing-editing.md §8), small
/// buttons in two rows.
fn modify_group(app: &mut ChiploadApp, ui: &mut egui::Ui) {
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
        (
            super::modify::ModifyTool::Array,
            Icon::Array,
            t!(ui, "header-array"),
            t!(ui, "header-array-hover"),
            None,
        ),
        (
            super::modify::ModifyTool::PolarArray,
            Icon::PolarArray,
            t!(ui, "header-polar-array"),
            t!(ui, "header-polar-array-hover"),
            None,
        ),
    ];
    // The Compass starts the second row (docs/specs/compass.md §3).
    let can_compass = app.can_show_compass();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        let rows = tools.chunks(tools.len().div_ceil(2)).count();
        for (index, row) in tools.chunks(tools.len().div_ceil(2)).enumerate() {
            ui.horizontal(|ui| {
                if index + 1 == rows {
                    let label = t!(ui, "compass-menu-item");
                    let hint = hover_keys(
                        &format!("{label}: {}", t!(ui, "compass-hover")),
                        Action::Compass,
                    );
                    let active = app.compass_shown();
                    let enabled = active || can_compass;
                    if small_ribbon_button(ui, Icon::Compass, &label, enabled, active, None)
                        .on_hover_text(hint)
                        .clicked()
                    {
                        app.toggle_compass();
                    }
                }
                for (tool, icon, label, hint, action) in row {
                    let active = app.modify() == Some(*tool);
                    let hint = format!("{label}: {hint}");
                    let hint = match action {
                        Some(action) => hover_keys(&hint, *action),
                        None => hint,
                    };
                    let enabled = can_draw && app.modify_ready(*tool);
                    if small_ribbon_button(ui, *icon, label, enabled, active, None)
                        .on_hover_text(hint)
                        .clicked()
                    {
                        // The active tool's button ends it again.
                        if active {
                            app.cancel_modify();
                        } else {
                            app.start_modify(*tool);
                        }
                    }
                }
            });
        }
    });
}

/// The Operation group: new operations and generating.
fn operation_group(app: &mut ChiploadApp, ui: &mut egui::Ui) {
    let selected = !app.selected_ids().is_empty();
    let dark = ui.visuals().dark_mode;
    let tint = |kind| Some(crate::style::op_color(kind, dark));
    let mut surface = None;
    let mut relief = None;
    let mut chamfer = None;
    let mut flute = None;
    let mut pattern = None;
    for (label, kind, hover) in super::browser::new_operations(ui) {
        let icon = match kind {
            OpKind::Profile => Icon::Profile,
            OpKind::Pocket => Icon::Pocket,
            OpKind::Vcarve => Icon::Vcarve,
            OpKind::Path => Icon::FollowPath,
            OpKind::Holes => Icon::Holes,
            // A small button under Styles (docs/specs/chamfer.md §6).
            OpKind::Chamfer => {
                chamfer = Some((label, hover));
                continue;
            }
            // Not on the ribbon, which then no longer fits English names at the
            // default width: the S menu (L), the Operation menu and the Browser
            // have it (docs/specs/fluting.md §13).
            // Small buttons beside Chamfer (docs/specs/fluting.md §13).
            OpKind::Flute => {
                flute = Some((label, hover));
                continue;
            }
            OpKind::Pattern => {
                pattern = Some((label, hover));
                continue;
            }
            // Not on the ribbon, which has no room left: the S menu (R), the
            // Operation menu and the Browser (docs/specs/prism.md §5).
            OpKind::Prism | OpKind::Moulding => continue,
            OpKind::Surface => {
                surface = Some((label, hover));
                continue;
            }
            // A small button under Surface, so the ribbon keeps its names at the
            // default width (the user, 2026-11-14).
            OpKind::Relief => {
                relief = Some((label, hover));
                continue;
            }
        };
        let short = if kind == OpKind::Path {
            t!(ui, "header-follow")
        } else {
            label
        };
        let enabled = super::browser::can_add(kind, selected, app.has_models());
        let response = tinted_ribbon_button(ui, icon, &short, enabled, false, tint(kind));
        let response = if enabled {
            response.on_hover_text(hover)
        } else {
            response.on_hover_text(t!(ui, "header-needs-selection-hover", operation = hover))
        };
        if response.clicked() {
            app.add_operation(kind);
        }
    }
    // Surface and Relief as small buttons, so the ribbon keeps its names at the
    // default width.
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        // A part's area: its selected outline (docs/specs/surface-levels.md §5).
        if let Some((label, hover)) = surface {
            let enabled = super::browser::can_add(OpKind::Surface, selected, app.has_models());
            let hover = if enabled {
                format!("{label}: {hover}")
            } else {
                format!(
                    "{label}: {}",
                    super::browser::cannot_add(ui, OpKind::Surface)
                )
            };
            if small_ribbon_button(
                ui,
                Icon::Surface,
                &label,
                enabled,
                false,
                tint(OpKind::Surface),
            )
            .on_hover_text(hover)
            .clicked()
            {
                app.add_operation(OpKind::Surface);
            }
        }
        // A relief of the selected model, or the first one (needs a model).
        if let Some((label, hover)) = relief {
            let enabled = app.has_models();
            let hover = if enabled {
                format!("{label}: {hover}")
            } else {
                format!("{label}: {}", t!(ui, "browser-relief-needs-model"))
            };
            if small_ribbon_button(
                ui,
                Icon::Relief,
                &label,
                enabled,
                false,
                tint(OpKind::Relief),
            )
            .on_hover_text(hover)
            .clicked()
            {
                app.add_operation(OpKind::Relief);
            }
        }
    });
    // Your styles (docs/specs/styles.md §3.2), a small button of its own.
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        let label = t!(ui, "styles-ribbon");
        let enabled = app.can_apply_style();
        let hover = if app.user.library.styles.styles.is_empty() {
            t!(ui, "styles-ribbon-none-hover")
        } else if enabled {
            t!(ui, "styles-ribbon-hover")
        } else {
            t!(ui, "styles-select-first")
        };
        let response = small_ribbon_button(ui, Icon::Style, &label, enabled, false, None)
            .on_hover_text(format!("{label}: {hover}"));
        let mut act = None;
        egui::Popup::menu(&response).show(|ui| app.style_menu(ui, &mut act));
        if let Some(act) = act {
            app.style_act(act);
        }
        if let Some((label, hover)) = chamfer {
            let hover = if selected {
                format!("{label}: {hover}")
            } else {
                t!(ui, "header-needs-selection-hover", operation = hover)
            };
            if small_ribbon_button(
                ui,
                Icon::EdgeChamfer,
                &label,
                selected,
                false,
                tint(OpKind::Chamfer),
            )
            .on_hover_text(hover)
            .clicked()
            {
                app.add_operation(OpKind::Chamfer);
            }
        }
    });
    // Flute over Pattern, a column of their own.
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        let small = [
            (flute, Icon::Flute, OpKind::Flute),
            (pattern, Icon::Hatch, OpKind::Pattern),
        ];
        for (named, icon, kind) in small {
            let Some((label, hover)) = named else {
                continue;
            };
            let hover = if selected {
                format!("{label}: {hover}")
            } else {
                t!(ui, "header-needs-selection-hover", operation = hover)
            };
            if small_ribbon_button(ui, icon, &label, selected, false, tint(kind))
                .on_hover_text(hover)
                .clicked()
            {
                app.add_operation(kind);
            }
        }
    });
}

/// The Wizards group: a whole thing in one dialog, its drawing and its operations (the
/// user, 2026-11-10).
fn wizards_group(app: &mut ChiploadApp, ui: &mut egui::Ui) {
    let label = t!(ui, "header-sign");
    if ribbon_button(ui, Icon::Sign, &label, app.can_draw(), false)
        .on_hover_text(t!(ui, "header-sign-hover"))
        .clicked()
    {
        app.start_sign();
    }
    let label = t!(ui, "header-two-sided");
    if ribbon_button(ui, Icon::TurnOver, &label, app.edit.is_some(), false)
        .on_hover_text(t!(ui, "header-two-sided-hover"))
        .clicked()
    {
        app.start_two_sided();
    }
    // A V-carve inlay of the selection (docs/specs/vcarve-inlay.md §4), a wizard
    // since it makes a pocket, a plug's setup and floor passes in one step
    // (docs/specs/relief-wizard.md §7).
    let label = t!(ui, "inlay-ribbon");
    let enabled = app.can_inlay();
    let hover = if enabled {
        t!(ui, "inlay-ribbon-hover")
    } else {
        t!(ui, "header-select-shapes-first")
    };
    if ribbon_button(ui, Icon::Vcarve, &label, enabled, false)
        .on_hover_text(hover)
        .clicked()
    {
        app.start_inlay();
    }
    // A relief of a 3D model in one step (docs/specs/relief-wizard.md §6).
    let label = t!(ui, "header-relief");
    if ribbon_button(ui, Icon::Relief, &label, true, false)
        .on_hover_text(t!(ui, "header-relief-hover"))
        .clicked()
    {
        app.start_relief();
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
    generate_hover: String,
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
                .on_hover_text(&self.generate_hover)
                .clicked()
            {
                clicked = Some(RibbonAction::Generate);
            }
        }
        clicked
    }
}

/// "New Profile" and the like: a menu item that adds an operation of `kind`.
pub(super) fn new_operation(ui: &egui::Ui, kind: OpKind) -> String {
    match kind {
        OpKind::Profile => t!(ui, "header-new-profile"),
        OpKind::Pocket => t!(ui, "header-new-pocket"),
        OpKind::Vcarve => t!(ui, "header-new-vcarve"),
        OpKind::Path => t!(ui, "header-new-follow-path"),
        OpKind::Chamfer => t!(ui, "header-new-chamfer"),
        OpKind::Flute => t!(ui, "header-new-flute"),
        OpKind::Pattern => t!(ui, "header-new-pattern"),
        OpKind::Prism => t!(ui, "header-new-prism"),
        OpKind::Moulding => t!(ui, "header-new-moulding"),
        OpKind::Holes => t!(ui, "header-new-holes"),
        OpKind::Surface => t!(ui, "header-new-surface"),
        OpKind::Relief => t!(ui, "header-new-relief"),
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
    egui::WidgetText::from(crate::style::bold(ui, label)).into_galley(
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
    tinted_ribbon_button(ui, icon, label, enabled, selected, None)
}

/// A [`ribbon_button`] whose icon is painted in `tint` while it is enabled and not selected:
/// an operation's colour (docs/specs/look-and-feel.md §4).
fn tinted_ribbon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
    tint: Option<Color32>,
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
            crate::style::disabled_color(visuals)
        }
    };
    let icon_color = match tint {
        Some(tint) if enabled && !selected => tint,
        _ => color,
    };
    if compact {
        let icon_rect = Rect::from_center_size(rect.center(), vec2(RIBBON_ICON, RIBBON_ICON));
        icons::paint(painter, icon_rect, icon, icon_color);
        let name = |ui: &mut egui::Ui| {
            ui.label(crate::style::bold(ui, label));
        };
        return response.on_hover_ui(name).on_disabled_hover_ui(name);
    }
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 4.0 + RIBBON_ICON / 2.0),
        vec2(RIBBON_ICON, RIBBON_ICON),
    );
    icons::paint(painter, icon_rect, icon, icon_color);
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
    tint: Option<Color32>,
) -> Response {
    let side = (RIBBON_BUTTON.0 - 2.0) / 2.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), sense);
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
            crate::style::disabled_color(visuals)
        }
    };
    let color = match tint {
        Some(tint) if enabled && !selected => tint,
        _ => color,
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
    // The strong accent: the one filled button (docs/specs/look-and-feel.md §2).
    let palette = crate::style::palette(visuals);
    let (fill, color) = if enabled {
        let fill = if response.hovered() {
            palette.strong.lerp_to_gamma(palette.on_strong, 0.12)
        } else {
            palette.strong
        };
        (fill, palette.on_strong)
    } else {
        (
            visuals.widgets.inactive.weak_bg_fill,
            crate::style::disabled_color(visuals),
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

impl ChiploadApp {
    /// A ribbon group: its buttons in a row, and under them its caption, a plain label:
    /// the menus are in the top bar (docs/specs/menu-bar.md §4).
    fn group(
        &mut self,
        ui: &mut egui::Ui,
        caption: &str,
        buttons: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let row = ui.horizontal(|ui| buttons(self, ui)).response;
            let text = RichText::new(caption.to_uppercase())
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
                    ui.add(egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend));
                },
            );
        });
    }

    /// The ribbon: FILE, DRAW, OPERATION and VIEW groups, Generate and Export at the
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
        let language = self.prefs.language;
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
        let has_result = self.toolpath.result.is_some();
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
            generate_hover: if self.edit.is_some() && !self.can_generate() {
                t!(ui, "header-generate-no-operations")
            } else {
                hover_keys(&t!(ui, "header-generate-hover"), Action::Generate)
            },
        }
    }

    /// The ribbon's groups, left to right.
    fn ribbon_groups(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 2.0;
        let groups: [(String, RibbonGroup); 5] = [
            (t!(ui, "header-group-file"), file_group),
            (t!(ui, "header-group-draw"), draw_group),
            (t!(ui, "header-group-modify"), modify_group),
            (t!(ui, "header-group-operation"), operation_group),
            (t!(ui, "header-group-wizards"), wizards_group),
        ];
        for (k, (caption, group)) in groups.into_iter().enumerate() {
            if k > 0 {
                ui.separator();
            }
            self.group(ui, &caption, group);
        }
    }
}
