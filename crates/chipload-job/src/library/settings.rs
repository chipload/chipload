// SPDX-License-Identifier: GPL-3.0-or-later
//! The app's settings: theme, language, units, recent files, new-project defaults.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::files::LibraryFile;
use super::files::written_by;
use super::{RECENT_FONTS_MAX, RECENT_MAX, SETTINGS_VERSION};
use crate::models::{Units, ZZero};

/// The GUI's colours (docs/specs/open-recent.md section 1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Light or dark as the operating system is set.
    #[default]
    System,
    Light,
    Dark,
}

/// App settings kept between sessions (docs/specs/open-recent.md).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSettings {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    /// Absolute paths, most recent first, at most `RECENT_MAX`, no duplicates.
    pub recent_projects: Vec<String>,
    /// Since version 2.
    pub theme: Theme,
    /// Generate the toolpath by itself shortly after each edit
    /// (docs/specs/background-generation.md). Since version 3.
    pub auto_generate: bool,
    /// Write a setup sheet next to exported G-code (docs/specs/setup-sheet.md §4). Missing
    /// (written before it existed): on.
    #[serde(default = "setup_sheet_default")]
    pub setup_sheet: bool,
    /// Draw the grid on the canvas (docs/specs/browser-and-layout.md §6b). Missing
    /// (written before it existed): on.
    #[serde(default = "show_grid_default")]
    pub show_grid: bool,
    /// Draw rulers along the canvas's edges (docs/specs/rulers.md). Missing
    /// (written before it existed): on.
    #[serde(default = "show_grid_default")]
    pub show_rulers: bool,
    /// In 3D, a left-right drag moves the view round the model, as before issue #16, instead
    /// of turning the model with the mouse (docs/specs/3d-view.md §3). Missing (written
    /// before it existed): off.
    #[serde(default)]
    pub reverse_orbit: bool,
    /// The units the app shows and takes lengths and feeds in; the project stays in mm
    /// (docs/specs/inch-units.md §5). Missing (written before it existed): mm.
    #[serde(default = "display_units_default")]
    pub units: Units,
    /// The app's language: `"system"` or a language code such as `"de"`
    /// (docs/specs/translations.md decision 5). A code the app doesn't know (written by a
    /// newer version) counts as `"system"`. Missing (written before it existed): system.
    #[serde(default = "language_default")]
    pub language: String,
    /// Check for new versions once a day (docs/specs/update-notice.md): true or false as
    /// the user answered; missing until asked.
    #[serde(default)]
    pub update_check: Option<bool>,
    /// When the daily check last ran, seconds since 1970 (UTC). Missing: never.
    #[serde(default)]
    pub update_checked_at: Option<u64>,
    /// A version the user chose to skip, e.g. "0.30.0". Missing: none.
    #[serde(default)]
    pub update_skipped: Option<String>,
    /// The fonts last picked for a text, most recent first, at most `RECENT_FONTS_MAX`, no
    /// duplicates (docs/specs/font-list.md). Missing (written before it existed): none.
    #[serde(default)]
    pub recent_fonts: Vec<String>,
    /// What the last project started in the window was started with, offered again by the
    /// new project dialog (docs/specs/new-project-dialog.md §4). Missing: none yet; left out
    /// then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_project: Option<NewProject>,
    /// The G-code file each recent project was last exported to, so Ctrl+E writes there
    /// again without asking (docs/specs/fewer-clicks.md §5): project path to G-code path,
    /// only for projects in `recent_projects`. Missing: none; left out then.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub exports: BTreeMap<String, String>,
}

/// The new project dialog's choices (docs/specs/new-project-dialog.md §2, §4): the library's
/// machine, material and tools by uid, and the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewProject {
    /// The machine profile's uid. Null: the library's default.
    #[serde(default)]
    pub machine: Option<String>,
    /// The material's uid. Null: none.
    #[serde(default)]
    pub material: Option<String>,
    /// The board's thickness, mm.
    pub thickness: f64,
    /// The board's size, mm: its size, or a fitted board's while the drawing is empty.
    pub x: f64,
    pub y: f64,
    /// Fitted round the drawing with this margin, mm. Null: the size.
    #[serde(default)]
    pub fit: Option<f64>,
    /// Where Z0 is.
    pub z_zero: ZZero,
    /// The tools' uids, in the order they are numbered.
    #[serde(default)]
    pub tools: Vec<String>,
}

/// The `language` that follows the system's language.
pub const SYSTEM_LANGUAGE: &str = "system";

fn language_default() -> String {
    SYSTEM_LANGUAGE.to_string()
}

fn display_units_default() -> Units {
    Units::Mm
}

fn setup_sheet_default() -> bool {
    true
}

fn show_grid_default() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            schema_version: SETTINGS_VERSION,
            written_by: None,
            recent_projects: Vec::new(),
            theme: Theme::System,
            auto_generate: true,
            setup_sheet: true,
            show_grid: true,
            show_rulers: true,
            reverse_orbit: false,
            units: Units::Mm,
            language: language_default(),
            update_check: None,
            update_checked_at: None,
            update_skipped: None,
            recent_fonts: Vec::new(),
            new_project: None,
            exports: BTreeMap::new(),
        }
    }
}

impl AppSettings {
    /// Put a project at the top of the recent list (moving it if it is there already).
    pub fn add_recent(&mut self, path: &str) {
        self.recent_projects.retain(|p| p != path);
        self.recent_projects.insert(0, path.to_string());
        self.recent_projects.truncate(RECENT_MAX);
        self.forget_exports_of_old_projects();
    }

    pub fn remove_recent(&mut self, path: &str) {
        self.recent_projects.retain(|p| p != path);
        self.forget_exports_of_old_projects();
    }

    /// Remember the G-code file a project was exported to (docs/specs/fewer-clicks.md §5).
    /// The project goes to the top of the recent list too, as only recent projects keep one.
    pub fn set_export(&mut self, project: &str, gcode: &str) {
        self.add_recent(project);
        self.exports.insert(project.to_string(), gcode.to_string());
    }

    /// The G-code file a project was last exported to, if remembered.
    pub fn export_of(&self, project: &str) -> Option<&str> {
        self.exports.get(project).map(String::as_str)
    }

    fn forget_exports_of_old_projects(&mut self) {
        let recent = &self.recent_projects;
        self.exports.retain(|project, _| recent.contains(project));
    }

    /// Put a font at the top of the recently used fonts (moving it if it is there already).
    pub fn add_recent_font(&mut self, font: &str) {
        self.recent_fonts.retain(|f| f != font);
        self.recent_fonts.insert(0, font.to_string());
        self.recent_fonts.truncate(RECENT_FONTS_MAX);
    }

    /// Every rule violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let fonts = &self.recent_fonts;
        if fonts.len() > RECENT_FONTS_MAX {
            problems.push(format!(
                "recent_fonts: at most {RECENT_FONTS_MAX}, got {}",
                fonts.len()
            ));
        }
        for (i, font) in fonts.iter().enumerate() {
            if font.trim().is_empty() {
                problems.push(format!("recent_fonts.{i}: must not be empty"));
            }
            if fonts[..i].contains(font) {
                problems.push(format!("recent_fonts.{i}: \"{font}\" is listed twice"));
            }
        }
        let recent = &self.recent_projects;
        if recent.len() > RECENT_MAX {
            problems.push(format!(
                "recent_projects: at most {RECENT_MAX}, got {}",
                recent.len()
            ));
        }
        for (i, path) in recent.iter().enumerate() {
            if path.trim().is_empty() {
                problems.push(format!("recent_projects.{i}: must not be empty"));
            }
            if recent[..i].contains(path) {
                problems.push(format!("recent_projects.{i}: \"{path}\" is listed twice"));
            }
        }
        for (project, gcode) in &self.exports {
            if !recent.contains(project) {
                problems.push(format!(
                    "exports.{project}: only a recent project keeps its export file"
                ));
            }
            if gcode.trim().is_empty() {
                problems.push(format!("exports.{project}: must not be empty"));
            }
        }
        problems
    }
}

impl LibraryFile for AppSettings {
    const VERSION: i64 = SETTINGS_VERSION;
    const OLDEST: i64 = 1;
    fn validate(&self) -> Vec<String> {
        AppSettings::validate(self)
    }
    fn stamp(&mut self) {
        self.schema_version = SETTINGS_VERSION;
        self.written_by = Some(written_by());
    }
    fn upgrade(from: i64, data: &mut serde_json::Map<String, Value>) {
        match from {
            1 => settings_v1_to_v2(data),
            2 => settings_v2_to_v3(data),
            _ => {}
        }
    }
}

/// Settings 1 -> 2: the theme, which until then always followed the system.
pub fn settings_v1_to_v2(data: &mut serde_json::Map<String, Value>) {
    data.insert("theme".into(), Value::String("system".into()));
    data.insert("schema_version".into(), Value::from(2));
}

/// Settings 2 -> 3: the toolpath updates by itself after edits, the new default (a UI
/// setting; projects and G-code are unaffected).
pub fn settings_v2_to_v3(data: &mut serde_json::Map<String, Value>) {
    data.insert("auto_generate".into(), Value::Bool(true));
    data.insert("schema_version".into(), Value::from(3));
}
