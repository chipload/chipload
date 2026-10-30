// SPDX-License-Identifier: GPL-3.0-or-later
//! "This computer" in the Browser, beside the library (docs/specs/this-computer.md):
//! Preferences, and the post-processors with what each writes.

use chipload_job::Units;
use chipload_pipeline::posts::{builtin_posts, post_fact_messages};
use eframe::egui::Id;

use super::browser::{Row, title_line, toggle_folder};
use super::menu::language_name;
use super::*;
use crate::icons::Icon;

const POSTS: &str = "browser-posts";

impl ChiploadApp {
    /// Every post: the built-in ones, then yours as read, with why one can't be used.
    pub(super) fn post_list(&self) -> Vec<(String, bool, Option<String>)> {
        let mut list: Vec<(String, bool, Option<String>)> = builtin_posts()
            .into_iter()
            .map(|name| (name.to_string(), true, None))
            .collect();
        list.extend(self.library.user_posts.iter().map(|(name, read)| {
            let problem = read.as_ref().err().map(|why| self.say(why));
            (name.clone(), false, problem)
        }));
        list
    }

    /// How many of your posts can't be used.
    pub(super) fn post_problems(&self) -> usize {
        self.library
            .user_posts
            .iter()
            .filter(|(_, read)| read.is_err())
            .count()
    }

    /// The Post-processors folder and its posts; returns a focus clicked.
    pub(super) fn post_rows(&mut self, ui: &mut egui::Ui) -> Option<Focus> {
        let posts = self.post_list();
        let id = Id::new(POSTS);
        let open_id = id.with("folder-open");
        let open = super::browser::folder_open(ui, open_id, false);
        let out = Row::new(id, 1, t!(ui, "computer-posts"))
            .folder(open)
            .icon(Icon::Export)
            .trailing(posts.len().to_string())
            .selected(self.focus == Focus::Posts)
            .alert(self.post_problems() > 0)
            .show(ui);
        if out.toggled {
            toggle_folder(ui, open_id, false);
        }
        let mut focus = None;
        if out
            .response
            .on_hover_text(t!(ui, "computer-posts-hover"))
            .clicked()
        {
            focus = Some(Focus::Posts);
        }
        if !open {
            return focus;
        }
        let current = self.edit.as_ref().map(|e| e.settings.machine.post.clone());
        for (k, (name, builtin, problem)) in posts.iter().enumerate() {
            let k = u32::try_from(k).unwrap_or(u32::MAX);
            let mut trailing = vec![if *builtin {
                t!(ui, "computer-post-built-in")
            } else {
                t!(ui, "computer-post-yours")
            }];
            if current.as_deref() == Some(name.as_str()) {
                trailing.push(t!(ui, "computer-post-this-project"));
            }
            let row = Row::new(id.with(name), 2, name)
                .icon(Icon::Export)
                .trailing(trailing.join(" · "))
                .selected(self.focus == Focus::Post(k))
                .alert(problem.is_some())
                .show(ui);
            if row.response.clicked() {
                focus = Some(Focus::Post(k));
            }
        }
        focus
    }

    /// Preferences: the settings folder, the setup sheet, updating automatically, the units,
    /// the theme and the language.
    pub(super) fn preferences_form(&mut self, ui: &mut egui::Ui) {
        let title = t!(ui, "computer-preferences");
        title_line(ui, Icon::Settings, None, &title);
        ui.weak(t!(ui, "computer-preferences-about"));
        ui.add_space(4.0);
        let mut sheet = self.setup_sheet;
        let mut auto = self.auto_generate;
        let mut theme = self.theme;
        let mut units = self.units;
        let mut language = self.language_setting.clone();
        Grid::new("preferences")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(t!(ui, "computer-settings-folder"));
                match &self.settings_dir {
                    Some(dir) => ui.label(dir.display().to_string()),
                    None => ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t!(
                            ui,
                            "computer-settings-folder-none",
                            var = chipload_pipeline::library::CONFIG_DIR_VAR
                        ),
                    ),
                };
                ui.end_row();
                ui.label(t!(ui, "computer-setup-sheet"))
                    .on_hover_text(t!(ui, "computer-setup-sheet-hover"));
                ui.checkbox(&mut sheet, t!(ui, "computer-setup-sheet-write"));
                ui.end_row();
                ui.label(t!(ui, "computer-toolpath"));
                ui.checkbox(&mut auto, t!(ui, "computer-toolpath-auto"))
                    .on_hover_text(t!(ui, "computer-toolpath-auto-hover"));
                ui.end_row();
                ui.label(t!(ui, "computer-units"))
                    .on_hover_text(t!(ui, "computer-units-hover"));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut units, Units::Mm, "mm");
                    ui.selectable_value(&mut units, Units::Inch, t!(ui, "menu-units-inch"));
                });
                ui.end_row();
                ui.label(t!(ui, "computer-theme"));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut theme, Theme::System, t!(ui, "computer-theme-system"));
                    ui.selectable_value(&mut theme, Theme::Light, t!(ui, "computer-theme-light"));
                    ui.selectable_value(&mut theme, Theme::Dark, t!(ui, "computer-theme-dark"));
                });
                ui.end_row();
                ui.label(t!(ui, "menu-language"));
                let system = chipload_job::library::SYSTEM_LANGUAGE;
                ComboBox::from_id_salt("language")
                    .selected_text(language_name(ui, &language))
                    .show_ui(ui, |ui| {
                        let label = language_name(ui, system);
                        ui.selectable_value(&mut language, system.to_string(), label);
                        for choice in crate::i18n::Language::ALL {
                            let label = language_name(ui, choice.code());
                            ui.selectable_value(&mut language, choice.code().to_string(), label);
                        }
                    });
                ui.end_row();
                self.update_setting_row(ui);
            });
        if sheet != self.setup_sheet {
            self.set_setup_sheet(sheet);
        }
        if auto != self.auto_generate {
            self.set_auto_generate(auto);
        }
        if theme != self.theme {
            self.set_theme(theme);
        }
        if units != self.units {
            self.set_units(units);
        }
        if language != self.language_setting {
            self.set_language(&language);
        }
    }

    /// The Post-processors folder: where yours go and how to make one.
    pub(super) fn posts_form(&mut self, ui: &mut egui::Ui) {
        let title = t!(ui, "computer-posts");
        title_line(ui, Icon::Export, None, &title);
        let yours = self.library.user_posts.len();
        ui.label(t!(
            ui,
            "computer-posts-count",
            builtin = builtin_posts().len(),
            yours = yours
        ));
        match &self.library.posts {
            Some(dir) => {
                ui.label(t!(
                    ui,
                    "computer-posts-folder",
                    folder = dir.display().to_string()
                ));
            }
            None => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    t!(ui, "computer-posts-no-folder"),
                );
            }
        }
        ui.weak(t!(ui, "computer-posts-about"));
        if ui
            .button(t!(ui, "computer-posts-reload"))
            .on_hover_text(t!(ui, "computer-posts-reload-hover"))
            .clicked()
        {
            self.reload_library();
        }
    }

    /// One post: what it writes, and *Use in this project*.
    pub(super) fn post_form(&mut self, ui: &mut egui::Ui, k: usize) {
        let Some((name, builtin, _)) = self.post_list().get(k).cloned() else {
            let title = t!(ui, "computer-post");
            title_line(ui, Icon::Export, None, &title);
            ui.weak(t!(ui, "computer-post-gone"));
            return;
        };
        title_line(ui, Icon::Export, None, &name);
        // Read once per post shown (a user post is two files), and again after a reload.
        let cached = self
            .library_state
            .post_facts
            .as_ref()
            .is_some_and(|(n, _)| *n == name);
        if !cached {
            let facts = post_fact_messages(&name, self.library.posts.as_deref())
                .map(|lines| lines.iter().map(|line| self.say(line)).collect())
                .map_err(|e| self.say(&e.message()));
            self.library_state.post_facts = Some((name.clone(), facts));
        }
        let facts = self
            .library_state
            .post_facts
            .as_ref()
            .map(|(_, facts)| facts.clone());
        match facts.unwrap_or_else(|| Ok(Vec::new())) {
            Ok(lines) => {
                for line in lines {
                    ui.label(line);
                }
            }
            Err(err) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    t!(ui, "computer-post-unusable", error = err),
                );
            }
        }
        ui.add_space(4.0);
        let current = self.edit.as_ref().map(|e| e.settings.machine.post.clone());
        let mine = current.as_deref() == Some(name.as_str());
        let mut chosen = false;
        ui.horizontal(|ui| {
            chosen = ui
                .add_enabled(
                    self.edit.is_some() && !mine,
                    egui::Button::new(t!(ui, "computer-post-use")),
                )
                .on_hover_text(if builtin {
                    t!(ui, "computer-post-use-hover")
                } else {
                    t!(ui, "computer-post-use-copy-hover")
                })
                .on_disabled_hover_text(if mine {
                    t!(ui, "computer-post-in-use")
                } else {
                    t!(ui, "computer-post-no-project")
                })
                .clicked();
        });
        if chosen {
            self.choose_project_post(&name);
        }
    }
}
