// SPDX-License-Identifier: GPL-3.0-or-later
//! The look: a little more room, rounder widgets, larger headings and one accent colour,
//! for both light and dark themes.

use eframe::egui::{self, Color32, CornerRadius, FontId, TextStyle};

/// The accent (selection, the Generate button) in the dark theme and the light one.
const ACCENT_DARK: Color32 = Color32::from_rgb(0x2a, 0x6c, 0xb0);
const ACCENT_LIGHT: Color32 = Color32::from_rgb(0xa8, 0xcd, 0xf5);
const ROUNDING: u8 = 4;

pub fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 5.0);
        style.spacing.button_padding = egui::vec2(6.0, 3.0);
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(19.0));
        let visuals = &mut style.visuals;
        let radius = CornerRadius::same(ROUNDING);
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = radius;
        }
        visuals.selection.bg_fill = if visuals.dark_mode {
            ACCENT_DARK
        } else {
            ACCENT_LIGHT
        };
    });
}
