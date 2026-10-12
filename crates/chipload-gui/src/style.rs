// SPDX-License-Identifier: GPL-3.0-or-later
//! The look: a little more room, rounder widgets, larger headings and one accent colour,
//! for both light and dark themes. Text colours are set so every kind of text reads at
//! least 4.5:1 against the panels (WCAG AA); egui's own defaults fall short in places.

use eframe::egui::{self, Color32, CornerRadius, FontId, TextStyle};

/// The accent (selection, the Generate button) in the dark theme and the light one.
const ACCENT_DARK: Color32 = Color32::from_rgb(0x2a, 0x6c, 0xb0);
const ACCENT_LIGHT: Color32 = Color32::from_rgb(0xa8, 0xcd, 0xf5);
/// Text on the dark accent (egui's light blue reads only 3.9:1 on it).
const ON_ACCENT_DARK: Color32 = Color32::WHITE;
/// Dark theme: labels, button text, weak text and errors, brighter than egui's.
const TEXT_DARK: Color32 = Color32::from_gray(185);
const BUTTON_TEXT_DARK: Color32 = Color32::from_gray(200);
const WEAK_DARK: Color32 = Color32::from_gray(160);
const ERROR_DARK: Color32 = Color32::from_rgb(0xff, 0x7b, 0x72);
/// Light theme: weak text, warnings and errors, darker than egui's.
const WEAK_LIGHT: Color32 = Color32::from_gray(100);
const WARN_LIGHT: Color32 = Color32::from_rgb(0xa3, 0x52, 0x00);
const ERROR_LIGHT: Color32 = Color32::from_rgb(0xb3, 0x26, 0x1e);
/// Report lines that went well, in each theme (egui has no colour for them).
const GOOD_DARK: Color32 = Color32::from_rgb(0x7f, 0xd8, 0x8f);
const GOOD_LIGHT: Color32 = Color32::from_rgb(0x1e, 0x7e, 0x34);
/// A hovered card in a list: subtler than a hovered button, so weak text on it still reads.
const CARD_HOVER_DARK: Color32 = Color32::from_gray(45);
const CARD_HOVER_LIGHT: Color32 = Color32::from_gray(232);
const ROUNDING: u8 = 4;

/// The fill of a hovered card (an operation in the list).
pub fn card_hover(visuals: &egui::Visuals) -> Color32 {
    if visuals.dark_mode {
        CARD_HOVER_DARK
    } else {
        CARD_HOVER_LIGHT
    }
}

/// The colour of good news ("✔ G-code written") in these visuals.
pub fn good_color(visuals: &egui::Visuals) -> Color32 {
    if visuals.dark_mode {
        GOOD_DARK
    } else {
        GOOD_LIGHT
    }
}

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
        if visuals.dark_mode {
            visuals.selection.bg_fill = ACCENT_DARK;
            visuals.selection.stroke.color = ON_ACCENT_DARK;
            visuals.widgets.noninteractive.fg_stroke.color = TEXT_DARK;
            visuals.widgets.inactive.fg_stroke.color = BUTTON_TEXT_DARK;
            visuals.weak_text_color = Some(WEAK_DARK);
            visuals.error_fg_color = ERROR_DARK;
        } else {
            visuals.selection.bg_fill = ACCENT_LIGHT;
            visuals.weak_text_color = Some(WEAK_LIGHT);
            visuals.warn_fg_color = WARN_LIGHT;
            visuals.error_fg_color = ERROR_LIGHT;
        }
    });
}

/// The WCAG contrast ratio of two colours, 1 (none) to 21 (black on white).
#[cfg(test)]
pub(crate) fn contrast(a: Color32, b: Color32) -> f64 {
    fn channel(c: u8) -> f64 {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    let luminance =
        |c: Color32| 0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b());
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text needs 4.5:1 (WCAG AA for normal text).
    const TEXT: f64 = 4.5;

    #[test]
    fn every_kind_of_text_is_readable_in_both_themes() {
        let ctx = egui::Context::default();
        apply(&ctx);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let v = &ctx.style_of(theme).visuals;
            let panel = v.panel_fill;
            let texts = [
                ("text", v.text_color(), panel),
                ("weak text", v.weak_text_color(), panel),
                ("warning", v.warn_fg_color, panel),
                ("error", v.error_fg_color, panel),
                ("good", good_color(v), panel),
                (
                    "button text",
                    v.widgets.inactive.text_color(),
                    v.widgets.inactive.weak_bg_fill,
                ),
                (
                    "weak text on a hovered card",
                    v.weak_text_color(),
                    card_hover(v),
                ),
                ("error on a hovered card", v.error_fg_color, card_hover(v)),
                (
                    "text on the accent",
                    v.selection.stroke.color,
                    v.selection.bg_fill,
                ),
                (
                    "text in a text field",
                    v.text_color(),
                    v.text_edit_bg_color(),
                ),
            ];
            for (what, fg, bg) in texts {
                let ratio = contrast(fg, bg);
                assert!(ratio >= TEXT, "{theme:?}: {what} reads {ratio:.2}:1");
            }
        }
    }

    #[test]
    fn contrast_matches_wcag() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 1e-9);
        assert!((contrast(Color32::RED, Color32::RED) - 1.0).abs() < 1e-9);
    }
}
