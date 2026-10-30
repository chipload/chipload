// SPDX-License-Identifier: GPL-3.0-or-later
//! The update notice in the window (docs/specs/update-notice.md §2): the question asked
//! once, the daily check at start, *Chipload X is available* in the status bar and under
//! Help with *What's new…* and *Skip this version*, *Check for new versions now*, and the
//! setting in Preferences. The check itself is `crate::updates`.

use chipload_job::library::AppSettings;
use eframe::egui;

use super::{ChiploadApp, STATUS_SECONDS};
use crate::i18n::t;
use crate::updates::{self, Outcome, Release, Version};

impl ChiploadApp {
    /// The answer, the last check and the skipped version, from the app settings.
    pub(super) fn load_update_settings(&mut self, settings: &AppSettings) {
        self.updates.check = settings.update_check;
        self.updates.checked_at = settings.update_checked_at;
        self.updates.skipped = settings.update_skipped.as_deref().and_then(Version::parse);
    }

    /// The question shows until answered, when there are settings to keep the answer in
    /// and it wasn't closed this run.
    pub(super) fn update_question_shown(&self) -> bool {
        self.settings_dir.is_some() && self.updates.check.is_none() && !self.updates.bar_closed
    }

    /// The user's answer, kept in the settings. Yes: the daily check runs at once.
    pub fn answer_update_question(&mut self, check: bool) {
        self.updates.check = Some(check);
        self.change_settings(|settings| settings.update_check = Some(check));
        if check {
            self.updates.started = false;
        }
    }

    /// The question's bar, across the top of the canvas.
    pub(super) fn update_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(t!(ui, "updates-ask"));
            if ui.button(t!(ui, "updates-ask-yes")).clicked() {
                self.answer_update_question(true);
            }
            if ui.button(t!(ui, "updates-ask-no")).clicked() {
                self.answer_update_question(false);
            }
            if ui
                .small_button("✕")
                .on_hover_text(t!(ui, "updates-ask-close-hover"))
                .clicked()
            {
                self.updates.bar_closed = true;
            }
        });
    }

    /// Each frame: the daily check once per start when due, and a finished check's outcome.
    pub(super) fn poll_updates(&mut self, ctx: &egui::Context) {
        if !self.updates.started {
            self.updates.started = true;
            let now = updates::now_s();
            if self.settings_dir.is_some()
                && updates::due(self.updates.check, self.updates.checked_at, now)
            {
                self.updates.checked_at = Some(now);
                self.change_settings(|settings| settings.update_checked_at = Some(now));
                let ctx = ctx.clone();
                self.updates.start(false, move || ctx.request_repaint());
            }
        }
        let Some((outcome, manual)) = self.updates.poll() else {
            return;
        };
        // A daily check says nothing unless there is news; Check now always answers
        // (decision 9).
        let status = match (&outcome, manual) {
            (Outcome::Newer(release), _) if self.updates.skipped != Some(release.version) => {
                Some(t!(
                    self,
                    "updates-available",
                    version = release.version.to_string()
                ))
            }
            (_, false) => None,
            (Outcome::Newer(_) | Outcome::Latest, true) => {
                Some(t!(self, "updates-latest", version = updates::THIS_VERSION))
            }
            (Outcome::Failed(why), true) => Some(t!(self, "updates-failed", reason = why.as_str())),
        };
        if let Some(text) = status {
            self.set_status(text, STATUS_SECONDS);
        }
    }

    /// Help > Check for new versions now: at once, whether the daily check is on or off
    /// (decision 8).
    pub fn check_updates_now(&mut self, ctx: &egui::Context) {
        if self.updates.checking() {
            return;
        }
        self.set_status(t!(self, "updates-checking"), STATUS_SECONDS);
        let ctx = ctx.clone();
        self.updates.start(true, move || ctx.request_repaint());
    }

    /// Skip this version: no notice for it again; a later one shows.
    pub fn skip_update(&mut self, version: Version) {
        self.updates.skipped = Some(version);
        self.change_settings(|settings| settings.update_skipped = Some(version.to_string()));
    }

    /// *Chipload X is available* at the right of the status bar, opening its two choices.
    pub(super) fn update_notice_status(&mut self, ui: &mut egui::Ui) {
        let Some(release) = self.updates.notice().cloned() else {
            return;
        };
        let text = t!(
            ui,
            "updates-available",
            version = release.version.to_string()
        );
        let label = egui::RichText::new(text).color(ui.visuals().hyperlink_color);
        ui.menu_button(label, |ui| self.update_choices(ui, &release))
            .response
            .on_hover_text(t!(ui, "updates-available-hover"));
        ui.separator();
    }

    /// Help's entries: the notice, when there is one, and Check now.
    pub(super) fn update_help_items(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        if let Some(release) = self.updates.notice().cloned() {
            let text = t!(
                ui,
                "updates-available",
                version = release.version.to_string()
            );
            ui.menu_button(text, |ui| self.update_choices(ui, &release));
        }
        if ui
            .add_enabled(
                !self.updates.checking(),
                egui::Button::new(t!(ui, "updates-check-now")),
            )
            .clicked()
        {
            self.check_updates_now(ui.ctx());
            ui.close();
        }
    }

    /// What's new… and Skip this version. Nothing is downloaded or installed (decision 4).
    fn update_choices(&mut self, ui: &mut egui::Ui, release: &Release) {
        if ui
            .button(t!(ui, "updates-whats-new"))
            .on_hover_text(t!(ui, "updates-whats-new-hover"))
            .clicked()
        {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab(release.page.as_str()));
            ui.close();
        }
        let version = release.version.to_string();
        if ui
            .button(t!(ui, "updates-skip"))
            .on_hover_text(t!(ui, "updates-skip-hover", version = version.as_str()))
            .clicked()
        {
            self.skip_update(release.version);
            ui.close();
        }
    }

    /// The setting's row in Preferences: the answer to the question, to change.
    pub(super) fn update_setting_row(&mut self, ui: &mut egui::Ui) {
        ui.label(t!(ui, "updates-setting"))
            .on_hover_text(t!(ui, "updates-setting-hover"));
        let mut on = self.updates.check == Some(true);
        if ui
            .checkbox(&mut on, t!(ui, "updates-setting-check"))
            .on_hover_text(t!(ui, "updates-setting-hover"))
            .changed()
        {
            self.answer_update_question(on);
        }
        ui.end_row();
    }
}
