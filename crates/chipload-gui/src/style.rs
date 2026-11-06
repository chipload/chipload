// SPDX-License-Identifier: GPL-3.0-or-later
//! The look (docs/specs/look-and-feel.md): a little more room, rounder widgets, larger
//! headings, shades from the top bar down to raised cards, the brand's green as the one
//! accent and a colour per operation kind, for both light and dark themes. Text colours
//! are set so every kind of text reads at least 4.5:1 against what it is on (WCAG AA);
//! egui's own defaults fall short in places.

use eframe::egui::{self, Color32, CornerRadius, FontId, TextStyle};

/// Dark theme: labels, button text, weak text and errors, brighter than egui's. Button
/// text, which is also the values in fields, a step above labels (the user, 2026-11-03).
const TEXT_DARK: Color32 = Color32::from_gray(185);
const BUTTON_TEXT_DARK: Color32 = Color32::from_gray(225);
const WEAK_DARK: Color32 = Color32::from_gray(160);
const ERROR_DARK: Color32 = Color32::from_rgb(0xff, 0x7b, 0x72);
/// Light theme: weak text, warnings and errors, darker than egui's.
const WEAK_LIGHT: Color32 = Color32::from_gray(100);
const WARN_LIGHT: Color32 = Color32::from_rgb(0xa3, 0x52, 0x00);
const ERROR_LIGHT: Color32 = Color32::from_rgb(0xb3, 0x26, 0x1e);
/// Report lines that went well, in each theme (egui has no colour for them).
const GOOD_DARK: Color32 = Color32::from_rgb(0x7f, 0xd8, 0x8f);
const GOOD_LIGHT: Color32 = Color32::from_rgb(0x1e, 0x7e, 0x34);
const ROUNDING: u8 = 4;
/// A card's corners (docs/specs/look-and-feel.md §5).
pub const CARD_ROUNDING: u8 = 6;

/// The window's shades and accents in one theme (docs/specs/look-and-feel.md §2, §3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// The top bar, darkest.
    pub top_bar: Color32,
    /// The ribbon, the panels' tab bars and the status bar.
    pub chrome: Color32,
    /// The panels.
    pub panel: Color32,
    /// A form's sections, raised above the panel.
    pub card: Color32,
    /// A hovered card in a list: subtler than a hovered button, so weak text on it reads.
    pub card_hover: Color32,
    /// Text fields and other inset boxes.
    pub field: Color32,
    /// A button's face.
    pub button: Color32,
    pub button_hover: Color32,
    /// 1 px lines between panels and round fields.
    pub divider: Color32,
    /// The one action a screen leads to (Generate), and its text.
    pub strong: Color32,
    pub on_strong: Color32,
    /// What is selected or switched on, and its text.
    pub quiet: Color32,
    pub on_quiet: Color32,
}

/// The dark theme, after the brand's dark `#1d2025` and green `#9ab964`.
pub const DARK: Palette = Palette {
    top_bar: Color32::from_rgb(0x12, 0x14, 0x16),
    chrome: Color32::from_rgb(0x19, 0x1b, 0x1f),
    panel: Color32::from_rgb(0x22, 0x25, 0x2a),
    card: Color32::from_rgb(0x2b, 0x2f, 0x35),
    card_hover: Color32::from_rgb(0x32, 0x37, 0x3e),
    field: Color32::from_rgb(0x15, 0x17, 0x1a),
    button: Color32::from_rgb(0x38, 0x3d, 0x44),
    button_hover: Color32::from_rgb(0x44, 0x4a, 0x52),
    divider: Color32::from_rgb(0x30, 0x34, 0x3a),
    strong: Color32::from_rgb(0x9a, 0xb9, 0x64),
    on_strong: Color32::from_rgb(0x1d, 0x20, 0x25),
    quiet: Color32::from_rgb(0x4d, 0x69, 0x30),
    on_quiet: Color32::WHITE,
};

/// The light theme, after the brand's light `#f3f5f0` and deep green.
pub const LIGHT: Palette = Palette {
    top_bar: Color32::from_rgb(0xd9, 0xdd, 0xd4),
    chrome: Color32::from_rgb(0xe6, 0xe9, 0xe2),
    panel: Color32::from_rgb(0xf3, 0xf5, 0xf0),
    card: Color32::WHITE,
    card_hover: Color32::from_rgb(0xe9, 0xed, 0xe4),
    field: Color32::WHITE,
    button: Color32::from_rgb(0xe2, 0xe6, 0xdd),
    button_hover: Color32::from_rgb(0xd5, 0xdb, 0xce),
    divider: Color32::from_rgb(0xd0, 0xd5, 0xcb),
    strong: Color32::from_rgb(0x55, 0x73, 0x2a),
    on_strong: Color32::WHITE,
    quiet: Color32::from_rgb(0xd5, 0xe3, 0xbd),
    on_quiet: Color32::from_rgb(0x1d, 0x20, 0x25),
};

/// The palette of these visuals.
pub fn palette(visuals: &egui::Visuals) -> &'static Palette {
    if visuals.dark_mode { &DARK } else { &LIGHT }
}

/// The top bar's fill: a step darker than the ribbon in either theme.
pub fn top_bar_fill(visuals: &egui::Visuals) -> Color32 {
    palette(visuals).top_bar
}

/// The fill of a hovered card (an operation in the list).
pub fn card_hover(visuals: &egui::Visuals) -> Color32 {
    palette(visuals).card_hover
}

/// A disabled button's icon and name: clearly off (docs/specs/look-and-feel.md §4).
pub fn disabled_color(visuals: &egui::Visuals) -> Color32 {
    visuals.weak_text_color().gamma_multiply(0.45)
}

/// An operation kind's colour, the same wherever it is shown: the Browser, the ribbon,
/// Properties (docs/specs/look-and-feel.md §4).
pub fn op_color(kind: chipload_pipeline::new_ops::OpKind, dark: bool) -> Color32 {
    use chipload_pipeline::new_ops::OpKind;
    match (kind, dark) {
        (OpKind::Profile, true) => Color32::from_rgb(0x6f, 0xc8, 0xbd),
        (OpKind::Profile, false) => Color32::from_rgb(0x00, 0x79, 0x6b),
        (OpKind::Pocket, true) => Color32::from_rgb(0xc3, 0xa6, 0xf0),
        (OpKind::Pocket, false) => Color32::from_rgb(0x6a, 0x3d, 0xb8),
        (OpKind::Vcarve, true) => Color32::from_rgb(0xf0, 0xb4, 0x6a),
        (OpKind::Vcarve, false) => Color32::from_rgb(0xa8, 0x55, 0x00),
        (OpKind::Path, true) => Color32::from_rgb(0x8f, 0xbc, 0xf5),
        (OpKind::Path, false) => Color32::from_rgb(0x1f, 0x5c, 0xb0),
        (OpKind::Holes, true) => Color32::from_rgb(0xf0, 0x9a, 0xb4),
        (OpKind::Holes, false) => Color32::from_rgb(0xa8, 0x1e, 0x55),
        (OpKind::Surface, true) => Color32::from_rgb(0xb8, 0xc4, 0x8a),
        (OpKind::Surface, false) => Color32::from_rgb(0x5a, 0x6b, 0x1e),
    }
}

/// What a Browser row is, for its icon's quiet colour (docs/specs/look-and-feel.md §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    /// Drawings, layers, groups and shapes.
    Shapes,
    /// Setups and stock: the board, in wood.
    Board,
    /// Tools and the machine: steel.
    Tools,
}

/// A row's quiet icon colour.
pub fn tint(tint: Tint, dark: bool) -> Color32 {
    match (tint, dark) {
        (Tint::Shapes, true) => Color32::from_rgb(0x9d, 0xb4, 0xcf),
        (Tint::Shapes, false) => Color32::from_rgb(0x3a, 0x5f, 0x8c),
        (Tint::Board, true) => Color32::from_rgb(0xcf, 0xab, 0x80),
        (Tint::Board, false) => Color32::from_rgb(0x8a, 0x5a, 0x2b),
        (Tint::Tools, true) => Color32::from_rgb(0xaa, 0xb4, 0xc0),
        (Tint::Tools, false) => Color32::from_rgb(0x4f, 0x5d, 0x6e),
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
        let p = *palette(visuals);
        visuals.panel_fill = p.panel;
        visuals.window_fill = p.panel;
        visuals.extreme_bg_color = p.field;
        visuals.faint_bg_color = p.card;
        visuals.window_stroke.color = p.divider;
        visuals.widgets.noninteractive.bg_fill = p.panel;
        visuals.widgets.noninteractive.weak_bg_fill = p.panel;
        visuals.widgets.noninteractive.bg_stroke.color = p.divider;
        visuals.widgets.inactive.bg_fill = p.button;
        visuals.widgets.inactive.weak_bg_fill = p.button;
        visuals.widgets.hovered.bg_fill = p.button_hover;
        visuals.widgets.hovered.weak_bg_fill = p.button_hover;
        visuals.widgets.open.weak_bg_fill = p.button_hover;
        visuals.selection.bg_fill = p.quiet;
        visuals.selection.stroke.color = p.on_quiet;
        if visuals.dark_mode {
            visuals.widgets.noninteractive.fg_stroke.color = TEXT_DARK;
            visuals.widgets.inactive.fg_stroke.color = BUTTON_TEXT_DARK;
            visuals.weak_text_color = Some(WEAK_DARK);
            visuals.error_fg_color = ERROR_DARK;
        } else {
            // A field is white on a white card: an outline shows where it is.
            visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(1.0, p.divider.gamma_multiply(0.9));
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
                ("text on a card", v.text_color(), palette(v).card),
                ("weak text on a card", v.weak_text_color(), palette(v).card),
                ("text on the chrome", v.text_color(), palette(v).chrome),
                (
                    "weak text on the chrome",
                    v.weak_text_color(),
                    palette(v).chrome,
                ),
                ("text on the top bar", v.text_color(), palette(v).top_bar),
                (
                    "text on the strong accent",
                    palette(v).on_strong,
                    palette(v).strong,
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
