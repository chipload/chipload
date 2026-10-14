// SPDX-License-Identifier: GPL-3.0-or-later
//! "This computer" in the Browser, beside the library (docs/specs/this-computer.md):
//! Preferences, and the post-processors with what each writes.

use chipload_pipeline::posts::{builtin_posts, post_facts};
use eframe::egui::Id;

use super::browser::{Row, title_line, toggle_folder};
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
        list.extend(
            self.library
                .user_posts
                .iter()
                .map(|(name, read)| (name.clone(), false, read.as_ref().err().cloned())),
        );
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
        let out = Row::new(id, 1, "Post-processors")
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
            .on_hover_text("What turns toolpaths into G-code for your machine: built in and yours")
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
            let mut trailing = vec![if *builtin { "built in" } else { "yours" }];
            if current.as_deref() == Some(name.as_str()) {
                trailing.push("this project");
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

    /// Preferences: the settings folder, the setup sheet, updating by itself, the theme.
    pub(super) fn preferences_form(&mut self, ui: &mut egui::Ui) {
        title_line(ui, Icon::Settings, None, "Preferences");
        ui.weak("For every project on this computer.");
        ui.add_space(4.0);
        let mut sheet = self.setup_sheet;
        let mut auto = self.auto_generate;
        let mut theme = self.theme;
        Grid::new("preferences")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Settings folder");
                match &self.settings_dir {
                    Some(dir) => ui.label(dir.display().to_string()),
                    None => ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!(
                            "none: set {} to keep settings and a library",
                            chipload_pipeline::library::CONFIG_DIR_VAR
                        ),
                    ),
                };
                ui.end_row();
                ui.label("Setup sheet").on_hover_text(
                    "An HTML page for the operator next to the exported files: stock, zero, \
                     tools in order, operations, times and warnings",
                );
                ui.checkbox(&mut sheet, "write one with the G-code");
                ui.end_row();
                ui.label("Toolpath");
                ui.checkbox(&mut auto, "update it by itself while editing")
                    .on_hover_text("Generate by itself half a second after each change");
                ui.end_row();
                ui.label("Colour theme");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut theme, Theme::System, "System");
                    ui.selectable_value(&mut theme, Theme::Light, "Light");
                    ui.selectable_value(&mut theme, Theme::Dark, "Dark");
                });
                ui.end_row();
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
    }

    /// The Post-processors folder: where yours go and how to make one.
    pub(super) fn posts_form(&mut self, ui: &mut egui::Ui) {
        title_line(ui, Icon::Export, None, "Post-processors");
        let yours = self.library.user_posts.len();
        ui.label(format!(
            "{} built in, {yours} of yours.",
            builtin_posts().len()
        ));
        match &self.library.posts {
            Some(dir) => {
                ui.label(format!("Yours go in {}", dir.display()));
            }
            None => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "No settings folder, so only the built-in posts.",
                );
            }
        }
        ui.weak(
            "A post of your own is two files: NAME.toml with its settings, and the template it \
             names. Start from a copy of a built-in one; the guide's Post-processors page \
             explains both.",
        );
        if ui
            .button("Read the folder again")
            .on_hover_text("After adding or changing a post's files")
            .clicked()
        {
            self.reload_library();
        }
    }

    /// One post: what it writes, and *Use in this project*.
    pub(super) fn post_form(&mut self, ui: &mut egui::Ui, k: usize) {
        let Some((name, builtin, _)) = self.post_list().get(k).cloned() else {
            title_line(ui, Icon::Export, None, "Post-processor");
            ui.weak("This post is no longer there.");
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
            let facts = post_facts(&name, self.library.posts.as_deref()).map_err(|e| e.to_string());
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
                ui.colored_label(ui.visuals().error_fg_color, format!("Can't be used: {err}"));
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
                    egui::Button::new("Use in this project"),
                )
                .on_hover_text(if builtin {
                    "Make it the project's post-processor"
                } else {
                    "Make it the project's post-processor: a copy goes into the project"
                })
                .on_disabled_hover_text(if mine {
                    "The project uses it"
                } else {
                    "Open a project first"
                })
                .clicked();
        });
        if chosen {
            self.choose_project_post(&name);
        }
    }
}
