// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's preferences kept in the app settings (settings.json): view and output
//! choices, units, language, colour theme and the recent files.

use super::*;

/// The user's preferences, as kept in the app settings.
pub struct Prefs {
    /// View > Update toolpath automatically, kept in the app settings.
    pub auto_generate: bool,
    /// Settings > Write a setup sheet with the G-code (docs/specs/setup-sheet.md §4).
    pub setup_sheet: bool,
    /// View → Show grid (kept in the app settings).
    pub show_grid: bool,
    /// Rulers along the canvas's edges in top view (docs/specs/rulers.md).
    pub show_rulers: bool,
    /// Settings > Colour theme, kept in the app settings.
    pub theme: Theme,
    /// Settings > Units: what lengths and feeds are shown and typed in
    /// (docs/specs/inch-units.md §5), kept in the app settings.
    pub units: chipload_job::Units,
    /// Settings > Language as kept in the app settings: `"system"` or a language code.
    pub language_setting: String,
    /// The language the app is shown in: `language_setting` resolved
    /// (docs/specs/translations.md decision 5).
    pub language: crate::i18n::Language,
    /// The theme last handed to egui.
    pub(super) applied_theme: Option<Theme>,
    /// File > Open recent (docs/specs/open-recent.md), most recent first.
    pub recent: Vec<String>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            auto_generate: true,
            setup_sheet: true,
            show_grid: true,
            show_rulers: true,
            units: chipload_job::Units::Mm,
            language_setting: chipload_job::library::SYSTEM_LANGUAGE.to_string(),
            language: crate::i18n::Language::En,
            theme: Theme::System,
            applied_theme: None,
            recent: Vec::new(),
        }
    }
}
