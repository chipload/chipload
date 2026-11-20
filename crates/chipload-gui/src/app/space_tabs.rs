// SPDX-License-Identifier: GPL-3.0-or-later
//! The tabs over the canvas: the drawing, and a space per setup
//! (docs/specs/drawing-and-setup-spaces.md §2).

use super::display::Space;
use super::*;
use crate::icons::Icon;

/// A selected setup's tab: a teal of its own, so a board's space never looks like the
/// drawing's (green, the app's selection; the user, 2026-11-20).
const SETUP_TAB: egui::Color32 = egui::Color32::from_rgb(0x4c, 0x9c, 0x96);

/// How long a part dragged from the Browser is held over a setup's tab before it opens,
/// seconds.
const TAB_OPENS_S: f64 = 0.5;

impl ChiploadApp {
    /// *Drawing*, a tab per setup (a back "Setup 2 (back)"), and *+* to add a setup; the one
    /// shown drawn as selected. In a narrow window they scroll sideways.
    pub(super) fn space_tabs(&mut self, ui: &mut egui::Ui, canvas: egui::Rect) {
        let Some(edit) = &self.edit else {
            return;
        };
        let setups: Vec<(u64, String, bool)> = edit
            .job
            .setups
            .iter()
            .map(|s| {
                let name = if s.back_of.is_some() {
                    t!(ui, "canvas-tab-back", name = s.name.as_str())
                } else {
                    s.name.clone()
                };
                (s.id, name, s.back_of.is_some())
            })
            .collect();
        let active = edit.job.active_setup_id();
        let space = self.space();
        let mut clicked = None;
        let mut add = false;
        // A part held over a setup's tab (docs/specs/drawing-and-setup-spaces.md §5).
        let mut held = None;
        // Below the top ruler, right of the left one.
        let ruler = if self.prefs.show_rulers && self.view.is_top() {
            super::rulers::RULER
        } else {
            0.0
        };
        let strip = egui::Rect::from_min_max(
            canvas.min + egui::vec2(ruler + 6.0, ruler + 6.0),
            // Clear of the X–Y sign (or the 3D cube) in the top right corner.
            egui::pos2(canvas.right() - 104.0, canvas.top() + ruler + 34.0),
        );
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(strip)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        // Too many to fit with their names: the ones not shown give theirs on hover only, so
        // the one shown and + stay in view.
        let font = egui::TextStyle::Button.resolve(ui.style());
        let width = |name: &str| {
            ui.painter()
                .layout_no_wrap(name.to_string(), font.clone(), egui::Color32::WHITE)
                .size()
                .x
                + TAB_CHROME
        };
        let drawing_name = t!(ui, "canvas-tab-drawing");
        let all: f32 =
            width(&drawing_name) + setups.iter().map(|(_, n, _)| width(n)).sum::<f32>() + 60.0;
        let compact = all > strip.width();
        let fill = ui.visuals().panel_fill.gamma_multiply(0.92);
        egui::Frame::new()
            .fill(fill)
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(4, 2))
            .show(&mut ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("space-tabs")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let on = space == Space::Drawing;
                            if space_tab(ui, Icon::Edit, &drawing_name, on, None, compact && !on)
                                .on_hover_text(t!(ui, "canvas-tab-drawing-hover"))
                                .clicked()
                            {
                                clicked = Some(None);
                            }
                            // The drawing apart from the boards.
                            ui.separator();
                            for (id, name, back) in &setups {
                                let on = space == Space::Setup && active == Some(*id);
                                let icon = if *back {
                                    Icon::TurnOver
                                } else {
                                    Icon::Rectangle
                                };
                                let hover = if compact && !on {
                                    format!("{name}\n{}", t!(ui, "canvas-tab-setup-hover"))
                                } else {
                                    t!(ui, "canvas-tab-setup-hover")
                                };
                                let tab =
                                    space_tab(ui, icon, name, on, Some(SETUP_TAB), compact && !on)
                                        .on_hover_text(hover);
                                if tab.clicked() {
                                    clicked = Some(Some(*id));
                                }
                                if tab
                                    .dnd_hover_payload::<super::browser_ops::DraggedPart>()
                                    .is_some()
                                {
                                    held = Some(*id);
                                }
                            }
                            if ui
                                .small_button("+")
                                .on_hover_text(t!(ui, "canvas-tab-add-hover"))
                                .clicked()
                            {
                                add = true;
                            }
                        });
                    });
            });
        // Held there half a second, the tab opens, so the part can be dropped on its board.
        let now = ui.input(|i| i.time);
        match (held, self.display.tab_held) {
            (Some(id), Some((was, since))) if id == was => {
                if now - since >= TAB_OPENS_S && !(space == Space::Setup && active == Some(id)) {
                    clicked = Some(Some(id));
                }
                ui.ctx().request_repaint();
            }
            (Some(id), _) => {
                self.display.tab_held = Some((id, now));
                ui.ctx().request_repaint();
            }
            (None, _) => self.display.tab_held = None,
        }
        if let Some(tab) = clicked {
            self.click_space_tab(tab);
        }
        if add && let Some(id) = self.add_setup() {
            self.click_space_tab(Some(id));
        }
    }
}

impl ChiploadApp {
    /// A part's row dragged from the Browser over a setup's space: its outline under the
    /// pointer (the grid's step; Alt: free); let go, a new instance of it there, not turned
    /// or mirrored, selected, one undo step. Over a sheet beside the board, on that sheet's
    /// setup, which becomes the active one (docs/specs/drawing-and-setup-spaces.md §5).
    pub(super) fn drop_part(&mut self, ui: &egui::Ui, response: &egui::Response, editing: bool) {
        if !editing || !self.view.is_top() || self.space() != Space::Setup {
            return;
        }
        let Some(dragged) = egui::DragAndDrop::payload::<super::browser_ops::DraggedPart>(ui.ctx())
        else {
            return;
        };
        let Some(pos) = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| response.rect.contains(*p))
        else {
            return;
        };
        let part = dragged.0;
        let Some(job) = self.edit.as_ref().map(|e| &e.job) else {
            return;
        };
        let Some(centre) = job.part_centre(part) else {
            return;
        };
        let raw = self.view.to_world(pos);
        let at = if ui.input(|i| i.modifiers.alt) {
            raw
        } else {
            let step = grid_step(self.view.scale, self.prefs.units);
            crate::draw::snap(raw, step, self.prefs.units)
        };
        let sheet = self.scene.sheet_at(at).map(|s| (s.setup, s.offset));
        let (dx, dy) = (at.0 - centre.0, at.1 - centre.1);
        let outlines = chipload_pipeline::parts::part_outlines(job, &self.contours, part);
        self.scene
            .preview
            .extend(outlines.into_iter().map(|(points, closed)| {
                let mut moved: Vec<Point> = points.iter().map(|p| (p.0 + dx, p.1 + dy)).collect();
                if let (true, Some(&first)) = (closed, moved.first()) {
                    moved.push(first);
                }
                moved
            }));
        if response
            .dnd_release_payload::<super::browser_ops::DraggedPart>()
            .is_none()
        {
            return;
        }
        let (setup, offset) = match sheet {
            Some((id, offset)) => {
                self.activate_sheet(id, pos);
                (Some(id), offset)
            }
            None => (None, (0.0, 0.0)),
        };
        self.drop_instance(part, setup, [dx - offset.0, dy - offset.1]);
    }
}

/// A tab's width besides its name: padding and the icon.
const TAB_CHROME: f32 = 3.0 * 6.0 + 14.0;

/// A tab: its icon and name (only its icon when `icon_only`, its name then on hover), filled
/// when it is the space shown, with `fill` (None: the app's selection colour).
fn space_tab(
    ui: &mut egui::Ui,
    icon: Icon,
    name: &str,
    on: bool,
    fill: Option<egui::Color32>,
    icon_only: bool,
) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let visuals = ui.visuals().clone();
    let text_color = if on {
        visuals.selection.stroke.color
    } else {
        visuals.widgets.inactive.text_color()
    };
    let shown = if icon_only { "" } else { name };
    let galley = ui
        .painter()
        .layout_no_wrap(shown.to_string(), font, text_color);
    let icon_size = 14.0;
    let pad = egui::vec2(6.0, 3.0);
    let text_width = if icon_only { -pad.x } else { galley.size().x };
    let size = egui::vec2(
        pad.x * 3.0 + icon_size + text_width,
        (galley.size().y.max(icon_size) + pad.y * 2.0).max(22.0),
    );
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, name)
    });
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let back = match (on, response.hovered()) {
            (true, _) => Some(fill.unwrap_or(visuals.selection.bg_fill)),
            (false, true) => Some(visuals.widgets.hovered.weak_bg_fill),
            (false, false) => None,
        };
        if let Some(back) = back {
            painter.rect_filled(rect, 4.0, back);
        }
        // A setup's tab not shown still carries its colour, underlined.
        if let (false, Some(fill)) = (on, fill) {
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 4.0, rect.bottom() - 1.5),
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 1.5),
                ],
                egui::Stroke::new(1.5, fill.gamma_multiply(0.7)),
            );
        }
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad.x, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        );
        crate::icons::paint(painter, icon_rect, icon, text_color);
        painter.galley(
            egui::pos2(
                icon_rect.right() + pad.x,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            text_color,
        );
    }
    response
}
