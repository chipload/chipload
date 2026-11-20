// SPDX-License-Identifier: GPL-3.0-or-later
//! The tabs over the canvas: the drawing, and a space per setup
//! (docs/specs/drawing-and-setup-spaces.md §2).

use super::display::Space;
use super::*;

impl ChiploadApp {
    /// *Drawing*, a tab per setup (a back "Setup 2 (back)"), and *+* to add a setup; the one
    /// shown drawn as selected. In a narrow window they scroll sideways.
    pub(super) fn space_tabs(&mut self, ui: &mut egui::Ui, canvas: egui::Rect) {
        let Some(edit) = &self.edit else {
            return;
        };
        let setups: Vec<(u64, String)> = edit
            .job
            .setups
            .iter()
            .map(|s| {
                let name = if s.back_of.is_some() {
                    t!(ui, "canvas-tab-back", name = s.name.as_str())
                } else {
                    s.name.clone()
                };
                (s.id, name)
            })
            .collect();
        let active = edit.job.active_setup_id();
        let space = self.space();
        let mut clicked = None;
        let mut add = false;
        // Below the top ruler, right of the left one.
        let ruler = if self.prefs.show_rulers && self.view.is_top() {
            super::rulers::RULER
        } else {
            0.0
        };
        let strip = egui::Rect::from_min_max(
            canvas.min + egui::vec2(ruler + 6.0, ruler + 6.0),
            egui::pos2(canvas.right() - 6.0, canvas.top() + ruler + 34.0),
        );
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(strip)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
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
                            if ui
                                .selectable_label(
                                    space == Space::Drawing,
                                    t!(ui, "canvas-tab-drawing"),
                                )
                                .on_hover_text(t!(ui, "canvas-tab-drawing-hover"))
                                .clicked()
                            {
                                clicked = Some(None);
                            }
                            for (id, name) in &setups {
                                let on = space == Space::Setup && active == Some(*id);
                                if ui
                                    .selectable_label(on, name)
                                    .on_hover_text(t!(ui, "canvas-tab-setup-hover"))
                                    .clicked()
                                {
                                    clicked = Some(Some(*id));
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
        if let Some(tab) = clicked {
            self.click_space_tab(tab);
        }
        if add && let Some(id) = self.add_setup() {
            self.click_space_tab(Some(id));
        }
    }
}
