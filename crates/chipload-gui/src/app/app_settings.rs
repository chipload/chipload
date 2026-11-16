// SPDX-License-Identifier: GPL-3.0-or-later
//! The app's settings in the window: the settings folder, units, language, theme and
//! recent files (`settings.json`).

use chipload_job::library::{AppSettings, load_settings, update_settings};

use super::*;

impl ChiploadApp {
    /// Where the library is kept (None in tests and without a home folder), and read it.
    pub fn set_settings_dir(&mut self, dir: Option<PathBuf>) {
        self.user.settings_dir = dir;
        self.text_fonts = super::text_fonts::TextFonts::new(self.user.settings_dir.as_deref());
        self.reload_library();
        let settings = self
            .user
            .settings_dir
            .as_deref()
            .map(chipload_pipeline::library::settings_path)
            .and_then(|path| load_settings(&path).ok())
            .unwrap_or_default();
        self.load_update_settings(&settings);
        self.prefs.recent = settings.recent_projects;
        self.text_fonts.recent = settings.recent_fonts;
        self.prefs.theme = settings.theme;
        self.prefs.auto_generate = settings.auto_generate;
        self.prefs.setup_sheet = settings.setup_sheet;
        self.prefs.show_grid = settings.show_grid;
        self.prefs.show_rulers = settings.show_rulers;
        self.prefs.reverse_orbit = settings.reverse_orbit;
        self.prefs.new_project = settings.new_project;
        self.prefs.exports = settings.exports;
        self.prefs.units = settings.units;
        self.show_in(crate::i18n::resolve(&settings.language));
        self.prefs.language_setting = settings.language;
        self.load_layout();
        self.start_recovery();
    }

    /// Settings > Units: used at once and kept in the settings for next time.
    pub fn set_units(&mut self, units: chipload_job::Units) {
        self.prefs.units = units;
        self.change_settings(|settings| settings.units = units);
    }

    /// Settings > Language: used at once and kept in the settings for next time.
    pub fn set_language(&mut self, setting: &str) {
        self.show_in(crate::i18n::resolve(setting));
        self.prefs.language_setting = setting.to_string();
        self.change_settings(|settings| settings.language = setting.to_string());
    }

    /// Show the app in `language`. The start screen's hint, written before the language
    /// was known, is written again in it.
    fn show_in(&mut self, language: crate::i18n::Language) {
        let hint = |l| Report::info([t!(l, "app-start-hint")]);
        if self.report == hint(self.prefs.language) {
            self.report = hint(language);
        }
        self.prefs.language = language;
    }

    /// Settings > Colour theme: used at once and kept in the settings for next time.
    pub fn set_theme(&mut self, theme: Theme) {
        self.prefs.theme = theme;
        self.change_settings(|settings| settings.theme = theme);
    }

    /// Put a project at the top of File > Open recent. A failure to write the settings
    /// only shows in the status bar: it never stops opening or saving.
    pub(super) fn record_recent(&mut self, path: &Path) {
        let Some(text) = std::path::absolute(path)
            .ok()
            .and_then(|p| p.to_str().map(str::to_string))
        else {
            return;
        };
        self.change_settings(|settings| settings.add_recent(&text));
    }

    /// Take a project off the recent list, or empty it (None).
    pub fn forget_recent(&mut self, path: Option<&str>) {
        self.change_settings(|settings| match path {
            Some(path) => settings.remove_recent(path),
            None => settings.recent_projects.clear(),
        });
    }

    pub(super) fn change_settings(&mut self, change: impl FnOnce(&mut AppSettings)) {
        let Some(dir) = self.user.settings_dir.clone() else {
            return;
        };
        let path = chipload_pipeline::library::settings_path(&dir);
        let result = update_settings(&path, |settings| {
            change(settings);
            Ok((settings.recent_projects.clone(), settings.exports.clone()))
        });
        match result {
            Ok((recent, exports)) => {
                self.prefs.recent = recent;
                self.prefs.exports = exports;
            }
            Err(err) => {
                let status = t!(
                    self,
                    "user-library-settings-not-saved",
                    error = self.say(&err.message())
                );
                self.set_status(status, ERROR_STATUS_SECONDS);
            }
        }
    }

    /// Open a project from File > Open recent; one that is gone is reported and forgotten.
    pub fn open_recent(&mut self, path: &str) -> bool {
        let file = PathBuf::from(path);
        if !file.exists() {
            self.error(vec![t!(self, "user-library-recent-gone", path = path)]);
            self.forget_recent(Some(path));
            return false;
        }
        if self.guard(Pending::Open(file.clone())) {
            return self.open(&file);
        }
        false
    }
}
