// SPDX-License-Identifier: GPL-3.0-or-later
//! One row of the Browser's tree (icon, label, folder arrow, eye, tick, trailing text),
//! and its folders.

use eframe::egui::{Color32, Id, Rect, Response, Stroke, WidgetInfo, WidgetType, vec2};

use super::*;
use crate::icons::{self, Icon};

/// A row's height, points.
pub(super) const ROW_HEIGHT: f32 = 22.0;

/// Indent per level of the tree, points.
pub(super) const INDENT: f32 = 14.0;

/// An icon's square, points.
const ICON: f32 = 14.0;

/// Room between a row's parts, points.
pub(super) const GAP: f32 = 4.0;

/// Room at the row's left and right ends, points.
pub(super) const PAD: f32 = 4.0;

/// One row of the tree, built with its parts and shown with `show`.
/// Room before a main group's row, points.
const GROUP_GAP: f32 = 3.0;

/// Room inside a count's pill, each side, points.
const PILL_PAD: f32 = 5.0;

/// Whether a row's trailing note is a count or a tool number ("3", "T2"), shown in a pill.
fn is_pill(trailing: &str) -> bool {
    let digits = trailing.strip_prefix('T').unwrap_or(trailing);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

pub(super) struct Row {
    id: Id,
    depth: usize,
    open: Option<bool>,
    eye: Option<bool>,
    tick: Option<bool>,
    icon: Option<(Icon, Option<Color32>)>,
    /// A quiet colour for the icon by what the row is (docs/specs/look-and-feel.md §4).
    tint: Option<crate::style::Tint>,
    label: String,
    trailing: String,
    selected: bool,
    dim: bool,
    alert: bool,
    drag: bool,
    strong: bool,
}

/// What happened to a row this frame.
pub(super) struct RowResponse {
    /// The whole row: clicks, double-clicks, the context menu, hover text.
    pub response: Response,
    /// Its triangle was clicked (or the folder row itself, for a plain folder).
    pub toggled: bool,
    /// Its eye was clicked.
    pub eye: bool,
    /// Its tick was clicked.
    pub tick: bool,
}

impl Row {
    pub(super) fn new(id: Id, depth: usize, label: impl Into<String>) -> Self {
        Self {
            id,
            depth,
            open: None,
            eye: None,
            tick: None,
            icon: None,
            tint: None,
            label: label.into(),
            trailing: String::new(),
            selected: false,
            dim: false,
            alert: false,
            drag: false,
            strong: false,
        }
    }

    /// The label in bold: the active part.
    pub(super) fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    /// A folder: open or closed, with a triangle.
    pub(super) fn folder(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// An eye: shown or hidden on the canvas.
    pub(super) fn eye(mut self, shown: bool) -> Self {
        self.eye = Some(shown);
        self
    }

    /// A tick: an operation that is cut or not.
    pub(super) fn tick(mut self, on: bool) -> Self {
        self.tick = Some(on);
        self
    }

    pub(super) fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some((icon, None));
        self
    }

    /// The icon in a quiet colour for what the row is: shapes, the board, tools.
    pub(super) fn icon_tinted(mut self, icon: Icon, tint: crate::style::Tint) -> Self {
        self.icon = Some((icon, None));
        self.tint = Some(tint);
        self
    }

    pub(super) fn icon_colored(mut self, icon: Icon, color: Color32) -> Self {
        self.icon = Some((icon, Some(color)));
        self
    }

    /// Weak text at the row's right end: a count, a tool.
    pub(super) fn trailing(mut self, text: impl Into<String>) -> Self {
        self.trailing = text.into();
        self
    }

    pub(super) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Dimmed: hidden, or an operation that isn't cut.
    pub(super) fn dim(mut self, dim: bool) -> Self {
        self.dim = dim;
        self
    }

    /// A problem: the name in the error colour and a dot at the right end.
    pub(super) fn alert(mut self, alert: bool) -> Self {
        self.alert = alert;
        self
    }

    /// The row can be dragged (operations, to reorder them).
    pub(super) fn draggable(mut self) -> Self {
        self.drag = true;
        self
    }

    pub(super) fn show(self, ui: &mut egui::Ui) -> RowResponse {
        // A little room before each of the project's main groups (docs/specs/look-and-feel.md
        // §7).
        if self.depth == 1 && self.open.is_some() {
            ui.add_space(GROUP_GAP);
        }
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::hover());
        let sense = if self.drag {
            Sense::click_and_drag()
        } else {
            Sense::click()
        };
        let response = ui.interact(rect, self.id, sense);
        let label = self.label.clone();
        let selected = self.selected;
        // Tests read back the tree as drawn: each row's depth and label, in order.
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            d.get_temp_mut_or_default::<Vec<(usize, String)>>(Id::new("browser-rows-drawn"))
                .push((self.depth, label.clone()));
        });
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &label)
        });
        let visuals = ui.visuals().clone();
        let painter = ui.painter_at(rect);
        // The highlight is rounded and starts at the row's own indent, so the guides of the
        // folders above stay clear of it.
        let from = rect.left() + PAD + self.depth as f32 * INDENT - 2.0;
        let mark = Rect::from_min_max(
            egui::pos2(from, rect.top() + 1.0),
            egui::pos2(
                rect.right().min(ui.clip_rect().right()) - 2.0,
                rect.bottom() - 1.0,
            ),
        );
        if self.selected {
            painter.rect_filled(mark, 4.0, visuals.selection.bg_fill);
        } else if response.hovered() {
            painter.rect_filled(mark, 4.0, crate::style::card_hover(&visuals));
        }
        let text_color = if self.selected {
            visuals.selection.stroke.color
        } else if self.alert {
            visuals.error_fg_color
        } else if self.dim {
            visuals.weak_text_color()
        } else {
            visuals.text_color()
        };
        let line_color = if self.selected {
            text_color
        } else {
            visuals.weak_text_color()
        };
        // Indent guides: a thin line under each open folder above.
        let guide = Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color);
        for level in 1..=self.depth {
            let x = rect.left() + PAD + (level as f32 - 0.5) * INDENT;
            painter.vline(x.round() + 0.5, rect.y_range(), guide);
        }
        let mut x = rect.left() + PAD + self.depth as f32 * INDENT;
        let slot =
            |x: f32, w: f32| Rect::from_min_size(egui::pos2(x, rect.top()), vec2(w, rect.height()));
        let mut toggled = false;
        if let Some(open) = self.open {
            let area = slot(x, INDENT);
            let hit = ui.interact(area, self.id.with("open"), Sense::click());
            icons::chevron(&painter, area, open, line_color);
            toggled = hit.clicked();
        }
        x += INDENT;
        let mut eye = false;
        if let Some(shown) = self.eye {
            let area = slot(x, ICON);
            let hit = ui
                .interact(area, self.id.with("eye"), Sense::click())
                .on_hover_text(if shown {
                    t!(ui, "browser-eye-hide-hover")
                } else {
                    t!(ui, "browser-eye-show-hover")
                });
            let color = if hit.hovered() {
                text_color
            } else {
                line_color
            };
            let icon = if shown { Icon::Eye } else { Icon::EyeOff };
            // Shown only while the row is under the mouse, or when the layer is hidden.
            if !shown || response.hovered() || hit.hovered() {
                icons::paint(
                    &painter,
                    area.shrink2(vec2(0.0, (ROW_HEIGHT - ICON) / 2.0)),
                    icon,
                    color,
                );
            }
            eye = hit.clicked();
            x += ICON + GAP;
        }
        let mut tick = false;
        if let Some(on) = self.tick {
            let area = slot(x, ICON);
            let hit = ui
                .interact(area, self.id.with("tick"), Sense::click())
                .on_hover_text(if on {
                    t!(ui, "browser-tick-on-hover")
                } else {
                    t!(ui, "browser-tick-off-hover")
                });
            let fill = if self.selected {
                visuals.selection.bg_fill
            } else {
                visuals.extreme_bg_color
            };
            icons::tick_box(&painter, area, on, line_color, fill);
            tick = hit.clicked();
            x += ICON + GAP;
        }
        if let Some((icon, color)) = self.icon {
            let area = Rect::from_center_size(
                egui::pos2(x + ICON / 2.0, rect.center().y),
                vec2(ICON, ICON),
            );
            let tint = self.tint.map(|t| crate::style::tint(t, visuals.dark_mode));
            let color = match color.or(tint) {
                Some(c) if !self.selected && !self.dim => c,
                _ => line_color,
            };
            icons::paint(&painter, area, icon, color);
            x += ICON + GAP + 2.0;
        }
        // The scroll area's bar may cover the row's right end: stop short of it.
        let mut right = rect.right().min(ui.clip_rect().right()) - PAD;
        if self.alert {
            let dot = egui::pos2(right - 4.0, rect.center().y);
            painter.circle_filled(dot, 3.5, visuals.error_fg_color);
            right -= 8.0 + GAP;
        }
        if !self.trailing.is_empty() {
            let galley = egui::WidgetText::from(RichText::new(&self.trailing).small()).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Small,
            );
            let at = egui::pos2(
                right - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            );
            let mut width = galley.size().x;
            // A count or a tool number in a small pill; longer notes as weak text.
            if is_pill(&self.trailing) {
                let pill = Rect::from_center_size(
                    egui::pos2(at.x + width / 2.0, rect.center().y),
                    vec2(width + 2.0 * PILL_PAD, galley.size().y + 2.0),
                )
                .translate(vec2(-PILL_PAD, 0.0));
                let fill = if self.selected {
                    visuals.selection.stroke.color.linear_multiply(0.18)
                } else {
                    crate::style::palette(&visuals).button
                };
                painter.rect_filled(pill, 7.0, fill);
                painter.galley(at - vec2(PILL_PAD, 0.0), galley.clone(), line_color);
                width += 2.0 * PILL_PAD;
            } else {
                painter.galley(at, galley.clone(), line_color);
            }
            right -= width + GAP * 2.0;
        }
        let mut label = RichText::new(&self.label);
        if self.strong {
            label = crate::style::bold(ui, label);
        }
        let galley = egui::WidgetText::from(label).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (right - x).max(0.0),
            egui::TextStyle::Body,
        );
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            text_color,
        );
        // A plain folder (no other click meaning) opens and closes on a click anywhere.
        RowResponse {
            response,
            toggled,
            eye,
            tick,
        }
    }
}

/// Whether the folder `id` is open (remembered between frames and sessions).
pub(super) fn folder_open(ui: &egui::Ui, id: Id, default: bool) -> bool {
    ui.data_mut(|d| *d.get_persisted_mut_or(id, default))
}

/// Open or close the folder `id`.
pub(super) fn toggle_folder(ui: &egui::Ui, id: Id, default: bool) {
    ui.data_mut(|d| {
        let open = d.get_persisted_mut_or(id, default);
        *open = !*open;
    });
}

/// A folder row: its triangle or a click on it opens and closes it. Returns whether it is
/// open after this frame, and the row's response.
pub(super) fn folder(ui: &mut egui::Ui, row: Row, default: bool) -> (bool, RowResponse) {
    let id = row.id.with("folder-open");
    let open = folder_open(ui, id, default);
    let out = row.folder(open).show(ui);
    if out.toggled || out.response.clicked() {
        toggle_folder(ui, id, default);
        return (!open, out);
    }
    (open, out)
}

/// The Properties panel's title: an icon and what the panel shows.
pub(super) fn title_line(ui: &mut egui::Ui, icon: Icon, color: Option<Color32>, text: &str) {
    super::forms::set_heading(ui, text);
    ui.horizontal(|ui| {
        let size = ui.text_style_height(&egui::TextStyle::Heading);
        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        let color = color.unwrap_or_else(|| ui.visuals().text_color());
        icons::paint(ui.painter(), rect.shrink(1.0), icon, color);
        // Ends in "…" in a narrow panel.
        ui.add(
            egui::Label::new(crate::style::bold(ui, egui::RichText::new(text).heading()))
                .truncate(),
        )
        .on_hover_text(text);
    });
    ui.add_space(2.0);
}
