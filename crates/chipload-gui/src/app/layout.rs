// SPDX-License-Identifier: GPL-3.0-or-later
//! The panels round the canvas: the Browser on the left, Properties and the shape library on
//! the right, the report at the bottom. Which are open, and each panel's contents apart from
//! where it is shown, so a different layout can show the same contents.

use super::*;

/// A folded side panel's width, and the least widths of the open ones: the Properties
/// forms fit in theirs (narrower labels and fields, rows that wrap).
const FOLDED_WIDTH: f32 = 26.0;
const BROWSER_MIN_WIDTH: f32 = 180.0;
pub(crate) const PROPERTIES_MIN_WIDTH: f32 = 250.0;

/// Which panels are open, and what the right one shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panels {
    /// The Browser and the Properties panel open, or folded to a strip for a larger canvas
    /// (View, the P key, or dragging a panel's edge past its least width).
    pub browser_open: bool,
    pub properties_open: bool,
    /// What the right panel shows: Properties or the shape library
    /// (docs/specs/shape-library.md §5.1).
    pub right_tab: RightTab,
    /// The report panel is open (it opens by itself on warnings and errors).
    pub report_open: bool,
}

impl Default for Panels {
    fn default() -> Self {
        Panels {
            browser_open: true,
            properties_open: true,
            right_tab: RightTab::default(),
            report_open: false,
        }
    }
}

/// What the right panel shows (docs/specs/shape-library.md, decision 9: tabs).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RightTab {
    #[default]
    Properties,
    Shapes,
}

/// A panel's contents, wherever the panel is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelKind {
    Browser,
    Properties,
    Shapes,
}

/// A folded side panel: one button that opens it again. True when clicked.
fn folded_strip(ui: &mut egui::Ui, arrow: &str, hover: &str) -> bool {
    ui.add_space(4.0);
    ui.vertical_centered(|ui| {
        ui.add(egui::Button::new(arrow).frame(false))
            .on_hover_text(hover)
            .clicked()
    })
    .inner
}

impl ChiploadApp {
    /// A panel's contents in `ui`, scrolling where they need to.
    pub(super) fn panel_contents(&mut self, ui: &mut egui::Ui, kind: PanelKind) {
        // The forms are laid out in the panel's width; a row that still doesn't fit
        // scrolls sideways instead of widening the panel.
        let width = ui.available_width();
        match kind {
            PanelKind::Browser => self.sidebar(ui),
            PanelKind::Properties => {
                ScrollArea::both()
                    .id_salt("properties-scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.set_max_width(width);
                        self.properties_panel(ui);
                    });
            }
            PanelKind::Shapes => {
                ScrollArea::vertical()
                    .id_salt("shape-library-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(width);
                        self.shapes_tab(ui);
                    });
            }
        }
    }

    /// The side panels as laid out now: the Browser on the left, Properties and the shape
    /// library as tabs on the right. Each folds to a strip (its edge dragged past the least
    /// width, a double-click on the edge, the strip's button, View or P) for a larger canvas.
    pub(super) fn side_panels(&mut self, ui: &mut egui::Ui) {
        let mut open = self.panels.browser_open;
        let unfold = Panel::show_switched(
            ui,
            &mut open,
            Panel::left("operations-folded")
                .resizable(true)
                .exact_size(FOLDED_WIDTH),
            Panel::left("operations")
                .resizable(true)
                .default_size(340.0)
                .min_size(BROWSER_MIN_WIDTH),
            |ui, open| {
                if open {
                    self.panel_contents(ui, PanelKind::Browser);
                    false
                } else {
                    let hover = t!(ui, "app-show-browser-hover");
                    folded_strip(ui, "▸", &hover)
                }
            },
        )
        .inner;
        self.panels.browser_open = open || unfold;
        let mut open = self.panels.properties_open;
        let unfold = Panel::show_switched(
            ui,
            &mut open,
            Panel::right("side-folded")
                .resizable(true)
                .exact_size(FOLDED_WIDTH),
            Panel::right("side")
                .resizable(true)
                .default_size(440.0)
                .min_size(PROPERTIES_MIN_WIDTH),
            |ui, open| {
                if !open {
                    let hover = t!(ui, "app-show-properties-hover");
                    return folded_strip(ui, "◂", &hover);
                }
                self.right_tabs(ui);
                let kind = match self.panels.right_tab {
                    RightTab::Properties => PanelKind::Properties,
                    RightTab::Shapes => PanelKind::Shapes,
                };
                self.panel_contents(ui, kind);
                false
            },
        )
        .inner;
        self.panels.properties_open = open || unfold;
    }
}
