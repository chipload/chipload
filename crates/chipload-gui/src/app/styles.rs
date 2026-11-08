// SPDX-License-Identifier: GPL-3.0-or-later
//! Styles in the window (docs/specs/styles.md §3): *Save as style…* on an operation, a
//! style applied to the canvas selection from the ribbon, the canvas' right-click menu or
//! the style's own menu, and the Styles folder under *This computer* with a form per style.

use std::collections::BTreeMap;

use chipload_job::library::{LibraryError, update_library};
use chipload_job::styles::{Style, StyleLibrary, StyleSide, style_of};
use chipload_job::{Operation, Side};
use chipload_pipeline::library::styles_path;
use eframe::egui::Id;

use super::browser::{Row, op_icon, title_line};
use super::library_view::{key_uid, uid_key};
use super::user_library::library_header;
use super::*;
use crate::icons::Icon;

const FOLDER: &str = "browser-library-styles";

/// What a click in the Styles folder, a style's form or a style menu asks for, done after
/// drawing.
pub(super) enum StyleAct {
    Focus(Focus),
    Apply(String),
    Rename(String, String),
    Delete(String),
    /// Write the style's draft, or drop it (docs/specs/styles.md §9, step 3b).
    Save(String),
    Revert(String),
}

impl ChiploadApp {
    /// Save operation `i` (0-based) as style `name`, in place of a style of that name
    /// (the prompt said so). False, with the reason shown, when it can't be saved.
    pub fn save_style(&mut self, i: usize, name: &str) -> bool {
        self.commit();
        self.reload_library();
        let Some(dir) = self.user.settings_dir.clone() else {
            return false;
        };
        let Some(edit) = &self.edit else {
            return false;
        };
        let style = match style_of(&edit.job, i, name, &self.user.library.tools) {
            Ok(style) => style,
            Err(err) => {
                let text = self.say(&err.message());
                self.error(vec![text]);
                return false;
            }
        };
        let name = style.name.clone();
        let saved = update_library(&styles_path(&dir), |library: &mut StyleLibrary| {
            library.put(style, true)
        });
        self.reload_library();
        match saved {
            Ok(()) => {
                let status = t!(self, "styles-saved", name = &name);
                self.set_status(status, STATUS_SECONDS);
                true
            }
            Err(err) => {
                let text = self.say(&err.message());
                self.error(vec![text]);
                false
            }
        }
    }

    /// Apply style `uid` to the canvas selection, as one undo step (docs/specs/styles.md
    /// §3.2): its operations at the end of the active part, the first one opened. None, with
    /// the reason shown, when it can't be used.
    pub fn apply_style(&mut self, uid: &str) -> Option<usize> {
        self.commit();
        let style = self
            .user
            .library
            .styles
            .styles
            .iter()
            .find(|s| s.uid == uid)?
            .clone();
        let selection = self.op_selection();
        let lang = self.prefs.language;
        let tools = self.user.library.tools.clone();
        let edit = self.edit.as_mut()?;
        let open = matches!(style.operation().ok(), Some(Operation::Path(_)));
        let select = selection.selector(&self.contours, open);
        let made = match chipload_pipeline::styles::style_operations(
            &edit.job,
            &style,
            &select,
            &self.contours,
            &tools,
            lang,
        ) {
            Ok(made) => made,
            Err(err) => {
                let text = self.say(&err.message());
                self.error(vec![text]);
                return None;
            }
        };
        let messages = made.made.messages.clone();
        let edit = self.edit.as_mut()?;
        edit.job.tools.extend(made.tools);
        let index = edit.insert_made(made.made);
        self.focus = Focus::Op(index);
        self.commit();
        if !messages.is_empty() {
            self.set_report(Report::of_messages(messages));
        }
        Some(index)
    }

    /// Whether a style can be applied now: a project, something selected, a style.
    pub(super) fn can_apply_style(&self) -> bool {
        self.edit.is_some()
            && self.selection != CanvasSelection::Nothing
            && !self.user.library.styles.styles.is_empty()
    }

    /// The styles as menu entries, each applying itself to the canvas selection; when
    /// nothing is selected they say so.
    pub(super) fn style_menu(&self, ui: &mut egui::Ui, act: &mut Option<StyleAct>) {
        let styles = &self.user.library.styles.styles;
        if styles.is_empty() {
            ui.weak(t!(ui, "styles-none-yet"));
            return;
        }
        let selected = self.selection != CanvasSelection::Nothing;
        for style in styles {
            if ui
                .add_enabled(
                    selected && self.edit.is_some(),
                    egui::Button::new(&style.name),
                )
                .on_hover_text(self.style_summary(ui, style))
                .on_disabled_hover_text(t!(ui, "styles-select-first"))
                .clicked()
            {
                *act = Some(StyleAct::Apply(style.uid.clone()));
                ui.close();
            }
        }
    }

    /// A style in a few words, in the window's language.
    pub(super) fn style_summary(&self, ui: &egui::Ui, style: &Style) -> String {
        let Ok(op) = style.operation() else {
            return t!(ui, "styles-unreadable");
        };
        let length = |mm: f64| crate::units::text(ui.ctx(), mm);
        let mut parts = vec![kind_name(ui, &op)];
        if let Operation::Profile(p) = &op {
            parts.push(match (style.side, p.side) {
                (Some(StyleSide::AsSaved), Side::On) => t!(ui, "forms-side-on"),
                (Some(StyleSide::AsSaved), Side::Inside) => t!(ui, "forms-side-inside"),
                (Some(StyleSide::AsSaved), Side::Outside) => t!(ui, "forms-side-outside"),
                _ => t!(ui, "styles-side-by-selection"),
            });
        }
        let (_, end) = op.depths();
        parts.push(match op.through() {
            Some(below) => t!(ui, "styles-through", below = length(below)),
            None => t!(ui, "styles-depth", depth = length(end)),
        });
        if let Operation::Profile(p) = &op
            && let Some(tabs) = &p.tabs
        {
            parts.push(t!(ui, "styles-tabs", count = tabs.count));
        }
        if let Some(tool) = style.tool() {
            parts.push(if self.user.library.tools.find_uid(&tool.uid).is_some() {
                tool.name.clone()
            } else {
                t!(ui, "styles-tool-missing", tool = &tool.name)
            });
        }
        parts.join(" · ")
    }

    /// The Styles folder of the library in the Browser, and its styles.
    pub(super) fn library_style_rows(&mut self, ui: &mut egui::Ui) {
        let styles = self.user.library.styles.styles.clone();
        let mut act = None;
        let id = Id::new(FOLDER);
        let open_id = id.with("folder-open");
        let open = super::browser::folder_open(ui, open_id, true);
        let out = Row::new(id, 1, t!(ui, "styles-folder"))
            .folder(open)
            .icon(Icon::Style)
            .trailing(styles.len().to_string())
            .selected(self.focus == Focus::LibraryStyles)
            .show(ui);
        if out.toggled {
            super::browser::toggle_folder(ui, open_id, true);
        }
        if out
            .response
            .on_hover_text(t!(ui, "styles-folder-hover"))
            .clicked()
        {
            act = Some(StyleAct::Focus(Focus::LibraryStyles));
        }
        if open {
            for style in &styles {
                let icon = style
                    .operation()
                    .ok()
                    .map_or(Icon::Style, |op| op_icon(&op));
                let row = Row::new(
                    Id::new(("browser-library-style", &style.uid)),
                    2,
                    &style.name,
                )
                .icon(icon)
                .selected(self.focus == Focus::LibraryStyle(uid_key(&style.uid)))
                .show(ui);
                let response = row.response.on_hover_text(self.style_summary(ui, style));
                if response.clicked() {
                    act = Some(StyleAct::Focus(Focus::LibraryStyle(uid_key(&style.uid))));
                }
                response.context_menu(|ui| {
                    if ui
                        .add_enabled(
                            self.edit.is_some() && self.selection != CanvasSelection::Nothing,
                            egui::Button::new(t!(ui, "styles-apply")),
                        )
                        .on_disabled_hover_text(t!(ui, "styles-select-first"))
                        .clicked()
                    {
                        act = Some(StyleAct::Apply(style.uid.clone()));
                    }
                    if ui.button(t!(ui, "common-delete")).clicked() {
                        act = Some(StyleAct::Delete(style.uid.clone()));
                    }
                });
            }
        }
        if let Some(act) = act {
            self.style_act(act);
        }
    }

    /// Properties for the Styles folder or one style.
    pub(super) fn style_properties(&mut self, ui: &mut egui::Ui) {
        let mut act = None;
        match self.focus {
            Focus::LibraryStyles => self.styles_folder_form(ui),
            Focus::LibraryStyle(key) => self.style_form(ui, &key_uid(key), &mut act),
            _ => {}
        }
        if let Some(act) = act {
            self.style_act(act);
        }
    }

    fn styles_folder_form(&self, ui: &mut egui::Ui) {
        title_line(ui, Icon::Style, None, &t!(ui, "styles-folder"));
        let path = self.user.settings_dir.as_deref().map(styles_path);
        library_header(ui, path.as_deref(), &self.user.library.problems);
        ui.label(t!(
            ui,
            "styles-count",
            count = self.user.library.styles.styles.len()
        ));
        ui.weak(t!(ui, "styles-about"));
    }

    /// A style's form (docs/specs/styles.md §9, step 3b): its name, what to do with it, and
    /// its operation in the operation's own form, the tools the library's. Changes are a
    /// draft until *Save*, as a library tool's are.
    fn style_form(&mut self, ui: &mut egui::Ui, uid: &str, act: &mut Option<StyleAct>) {
        let Some(style) = self
            .user
            .library
            .styles
            .styles
            .iter()
            .find(|s| s.uid == uid)
            .cloned()
        else {
            title_line(ui, Icon::Style, None, &t!(ui, "styles-folder"));
            ui.weak(t!(ui, "styles-gone"));
            return;
        };
        let draft = match chipload_pipeline::styles::style_draft(&style, &self.user.library.tools) {
            Ok(draft) => draft,
            Err(err) => {
                title_line(ui, Icon::Style, None, &style.name);
                ui.colored_label(ui.visuals().warn_fg_color, self.say(&err.message()));
                return;
            }
        };
        let mut op = self
            .user
            .library_state
            .style_drafts
            .get(uid)
            .cloned()
            .unwrap_or_else(|| draft.operation.clone());
        title_line(ui, op_icon(&op), None, &style.name);
        ui.weak(t!(ui, "styles-entry-about"));
        ui.add_space(4.0);
        let draft_id = Id::new(("style-name-draft", uid));
        let mut name: String = ui
            .data_mut(|d| d.get_temp(draft_id))
            .unwrap_or_else(|| style.name.clone());
        crate::app::forms::card(ui, |ui| {
            egui::Grid::new("style-form").num_columns(2).show(ui, |ui| {
                ui.label(t!(ui, "styles-name"));
                ui.text_edit_singleline(&mut name);
                ui.end_row();
            })
        });
        let renamed = name.trim() != style.name && !name.trim().is_empty();
        ui.data_mut(|d| d.insert_temp(draft_id, name.clone()));
        let changed = op != draft.operation;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(renamed, egui::Button::new(t!(ui, "styles-rename")))
                .clicked()
            {
                *act = Some(StyleAct::Rename(uid.to_string(), name.trim().to_string()));
                ui.data_mut(|d| d.remove::<String>(draft_id));
            }
            if ui
                .add_enabled(
                    !changed && self.edit.is_some() && self.selection != CanvasSelection::Nothing,
                    egui::Button::new(t!(ui, "styles-apply")),
                )
                .on_hover_text(t!(ui, "styles-apply-hover"))
                .on_disabled_hover_text(if changed {
                    t!(ui, "styles-save-first")
                } else {
                    t!(ui, "styles-select-first")
                })
                .clicked()
            {
                *act = Some(StyleAct::Apply(uid.to_string()));
            }
            if ui
                .button(t!(ui, "common-delete"))
                .on_hover_text(t!(ui, "library-view-delete-hover"))
                .clicked()
            {
                *act = Some(StyleAct::Delete(uid.to_string()));
            }
        });
        ui.add_space(6.0);
        // Its problems by field, as an operation's form shows them.
        let problems = chipload_pipeline::styles::style_problems(&op, &draft.tools);
        let mut map = BTreeMap::new();
        for problem in &problems {
            map.entry(problem.field().to_string())
                .or_insert_with(|| super::problems::at_field(self, self.prefs.units, problem));
        }
        super::problems::set_form_problems(ui, map);
        let thickness = self
            .user
            .library
            .materials
            .default_thickness
            .unwrap_or(chipload_pipeline::DEFAULT_THICKNESS);
        let choice = forms::ToolChoice {
            selected: 0,
            selection_text: "",
            tools: &draft.tools,
            library: None,
            cutting: None,
            materials: None,
            style: true,
        };
        let stock = forms::StockDepths {
            thickness,
            max_overcut: chipload_pipeline::DEFAULT_MAX_OVERCUT,
        };
        let mut side_chosen = true;
        forms::op_form(ui, 0, &mut op, &mut side_chosen, &[], choice, stock);
        super::problems::set_form_problems(ui, BTreeMap::new());
        ui.add(egui::Label::new(egui::RichText::new(t!(ui, "styles-feeds-note")).weak()).wrap());
        ui.add_space(4.0);
        let changed = op != draft.operation;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(changed, egui::Button::new(t!(ui, "common-save")))
                .on_hover_text(t!(ui, "styles-save-hover"))
                .clicked()
            {
                *act = Some(StyleAct::Save(uid.to_string()));
            }
            if ui
                .add_enabled(changed, egui::Button::new(t!(ui, "library-view-revert")))
                .on_hover_text(t!(ui, "library-view-revert-hover"))
                .clicked()
            {
                *act = Some(StyleAct::Revert(uid.to_string()));
            }
            if changed {
                ui.colored_label(ui.visuals().warn_fg_color, t!(ui, "library-view-not-saved"));
            }
        });
        if changed {
            self.user
                .library_state
                .style_drafts
                .insert(uid.to_string(), op);
        } else {
            self.user.library_state.style_drafts.remove(uid);
        }
    }

    /// Write style `uid`'s draft into `styles.json`; the draft goes once it is saved.
    pub fn save_style_draft(&mut self, uid: &str) -> bool {
        let Some(op) = self.user.library_state.style_drafts.get(uid).cloned() else {
            return false;
        };
        let Some(style) = self
            .user
            .library
            .styles
            .styles
            .iter()
            .find(|s| s.uid == uid)
            .cloned()
        else {
            return false;
        };
        let saved = match chipload_pipeline::styles::style_from_draft(
            &style,
            &op,
            &self.user.library.tools,
        ) {
            Ok(saved) => saved,
            Err(err) => {
                self.user.library_state.error = Some(self.say(&err.message()));
                return false;
            }
        };
        let name = style.name.clone();
        if self
            .change_styles(|library| {
                *library.find_mut(&name)? = saved;
                Ok(())
            })
            .is_err()
        {
            return false;
        }
        self.user.library_state.style_drafts.remove(uid);
        let status = t!(self, "styles-saved", name = name);
        self.set_status(status, STATUS_SECONDS);
        true
    }

    pub(super) fn style_act(&mut self, act: StyleAct) {
        self.user.library_state.error = None;
        match act {
            StyleAct::Focus(focus) => {
                self.reload_library();
                self.focus = focus;
            }
            StyleAct::Apply(uid) => {
                self.apply_style(&uid);
            }
            StyleAct::Rename(uid, new) => {
                let old = self.style_name(&uid);
                let _ = self.change_styles(|library| library.rename(&old, &new));
            }
            StyleAct::Delete(uid) => {
                self.user.library_state.style_drafts.remove(&uid);
                self.delete_style(&uid);
            }
            StyleAct::Save(uid) => {
                self.save_style_draft(&uid);
            }
            StyleAct::Revert(uid) => {
                self.user.library_state.style_drafts.remove(&uid);
            }
        }
    }

    /// Delete style `uid` from the library; the Browser shows the folder after.
    pub fn delete_style(&mut self, uid: &str) -> bool {
        let name = self.style_name(uid);
        if self
            .change_styles(|library| library.remove(&name).map(|_| ()))
            .is_err()
        {
            return false;
        }
        if self.focus == Focus::LibraryStyle(uid_key(uid)) {
            self.focus = Focus::LibraryStyles;
        }
        let status = t!(self, "library-view-deleted", name = name);
        self.set_status(status, STATUS_SECONDS);
        true
    }

    fn style_name(&self, uid: &str) -> String {
        self.user
            .library
            .styles
            .styles
            .iter()
            .find(|s| s.uid == uid)
            .map(|s| s.name.clone())
            .unwrap_or_default()
    }

    /// Change `styles.json` and read the library again; a refusal is shown in the form.
    fn change_styles<T>(
        &mut self,
        change: impl FnOnce(&mut StyleLibrary) -> Result<T, LibraryError>,
    ) -> Result<T, ()> {
        let Some(dir) = self.user.settings_dir.clone() else {
            return Err(());
        };
        let result = update_library(&styles_path(&dir), change);
        self.reload_library();
        result.map_err(|e| {
            self.user.library_state.error = Some(self.say(&e.message()));
        })
    }

    /// *Save as style…*: a name for operation `i` as a style, saved on *Save*.
    pub(super) fn style_name_prompt(&mut self, ctx: &egui::Context, i: usize, mut text: String) {
        let taken = self
            .user
            .library
            .styles
            .find(text.trim())
            .map(|s| s.name.clone());
        let mut done = false;
        let mut save = false;
        egui::Window::new(t!(ctx, "styles-save-title"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(t!(ui, "styles-save-name"));
                let edit = ui.add(egui::TextEdit::singleline(&mut text).desired_width(320.0));
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if let Some(existing) = &taken {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t!(ui, "user-library-replaces", name = existing),
                    );
                }
                ui.weak(t!(ui, "styles-save-about"));
                ui.horizontal(|ui| {
                    let label = if taken.is_some() {
                        t!(ui, "user-library-replace")
                    } else {
                        t!(ui, "common-save")
                    };
                    let can = !text.trim().is_empty();
                    if (ui.add_enabled(can, egui::Button::new(label)).clicked()) || (enter && can) {
                        save = true;
                        done = true;
                    }
                    if ui.button(t!(ui, "common-cancel")).clicked() {
                        done = true;
                    }
                });
            });
        self.dialogs.prompt = (!done).then(|| Prompt::StyleName(i, text.clone()));
        if save {
            self.save_style(i, &text);
        }
    }

    /// Open *Save as style…* for operation `i`, with its name to start from.
    pub fn ask_style_name(&mut self, i: usize) {
        let name = self
            .edit
            .as_ref()
            .and_then(|e| e.job.operations.get(i))
            .map(|op| op.name().to_string())
            .unwrap_or_default();
        self.dialogs.prompt = Some(Prompt::StyleName(i, name));
    }
}

/// "Profile", "Pocket" …: what an operation is, in the window's language.
fn kind_name(ui: &egui::Ui, op: &Operation) -> String {
    match op {
        Operation::Profile(_) => t!(ui, "forms-kind-profile"),
        Operation::Pocket(_) => t!(ui, "forms-kind-pocket"),
        Operation::Vcarve(_) => t!(ui, "forms-kind-vcarve"),
        Operation::Path(_) => t!(ui, "forms-kind-path"),
        Operation::Holes(_) => t!(ui, "forms-kind-holes"),
        Operation::Surface(_) => t!(ui, "forms-kind-surface"),
    }
}
