// SPDX-License-Identifier: GPL-3.0-or-later
//! Materials in the window (docs/specs/materials.md, docs/specs/cutting-data.md §7):
//! choosing the material for every operation, following it when an operation's tool
//! changes, saving an operation's values into its tool's row for its material, and the
//! Library's Materials folder with a form per material: its name, and the tools with values
//! for it. Edits to a material stay a draft until *Save*, as for tools.

use chipload_job::library::{LibraryError, update_library, update_tools};
use chipload_job::materials::MaterialLibrary;
use chipload_pipeline::library::{materials_path, tools_path};
use chipload_pipeline::materials::{use_material, values_to_save};
use eframe::egui::Id;

use super::browser::{Row, title_line};
use super::library_view::{key_uid, uid_key};
use super::user_library::{library_header, unique_name};
use super::*;
use crate::icons::Icon;

const FOLDER: &str = "browser-library-materials";

/// What a click in the Materials folder or a material's form asks for, done after drawing.
enum MaterialAct {
    Focus(Focus),
    New,
    Save(String),
    Revert(String),
    Delete(String),
    UseInProject(String),
    MakeDefault(String),
}

impl ChiploadApp {
    /// Choose the material for every operation and those added later (None: none), as one
    /// undo step: the operations take its values for their tools, the stock stays; the
    /// report says what changed.
    pub fn use_material(&mut self, uid: Option<&str>) -> bool {
        self.commit();
        self.reload_library();
        let material = match uid {
            Some(uid) => match self.user.library.materials.find_uid(uid) {
                Some(m) => Some(m.clone()),
                None => return false,
            },
            None => None,
        };
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        let tools = self.user.library.tools.clone();
        let (job, messages) = use_material(&job, &tools, material.as_ref());
        // The "side not chosen" marks stay: operations aren't added or removed.
        if let Some(edit) = self.edit.as_mut() {
            edit.adopt(job);
        }
        self.commit();
        self.set_report(Report::of_messages(messages));
        true
    }

    /// Save operation `i`'s values (0-based) into its tool's row for its material (else the
    /// project's).
    pub fn save_op_to_material(&mut self, i: usize) -> bool {
        self.commit();
        self.reload_library();
        let Some(dir) = self.user.settings_dir.clone() else {
            return false;
        };
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        let Some(uid) = job
            .operations
            .get(i)
            .and_then(|op| op.material())
            .or(job.stock().material.as_ref())
            .map(|m| m.library_uid.clone())
        else {
            return false;
        };
        let Some(material) = self.user.library.materials.find_uid(&uid).cloned() else {
            return false;
        };
        let saved = update_tools(&tools_path(&dir), |tools| {
            let (tool_uid, values) = values_to_save(&job, i + 1, tools, &material)
                .map_err(|e| LibraryError::Refused(e.message().into()))?;
            if let Some(entry) = tools.tools.iter_mut().find(|t| t.uid == tool_uid) {
                entry.set_row(Some(&material.uid), values);
            }
            Ok(material.name.clone())
        });
        self.reload_library();
        match saved {
            Ok(name) => {
                let status = t!(self, "materials-saved", name = name);
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

    /// The Materials folder of the library in the Browser, and its materials.
    pub(super) fn library_material_rows(&mut self, ui: &mut egui::Ui) {
        let materials = self.user.library.materials.materials.clone();
        let mut act = None;
        let id = Id::new(FOLDER);
        let open_id = id.with("folder-open");
        let open = super::browser::folder_open(ui, open_id, true);
        let out = Row::new(id, 1, t!(ui, "materials-folder"))
            .folder(open)
            .icon(Icon::Material)
            .trailing(materials.len().to_string())
            .selected(self.focus == Focus::LibraryMaterials)
            .show(ui);
        if out.toggled {
            super::browser::toggle_folder(ui, open_id, true);
        }
        let response = out.response.on_hover_text(t!(ui, "materials-folder-hover"));
        if response.clicked() {
            act = Some(MaterialAct::Focus(Focus::LibraryMaterials));
        }
        response.context_menu(|ui| {
            if ui.button(t!(ui, "materials-new")).clicked() {
                act = Some(MaterialAct::New);
            }
        });
        if open {
            let chosen = self
                .edit
                .as_ref()
                .and_then(|e| e.job.stock().material.as_ref())
                .map(|m| m.library_uid.clone());
            for material in &materials {
                let count = self
                    .user
                    .library
                    .tools
                    .tools
                    .iter()
                    .filter(|t| t.row(Some(&material.uid)).is_some())
                    .count();
                let mut note = t!(ui, "materials-tool-count", count = count);
                if self.user.library.materials.default_entry().map(|m| &m.uid)
                    == Some(&material.uid)
                {
                    note = format!("{note} · {}", t!(ui, "library-view-note-default"));
                }
                if chosen.as_deref() == Some(material.uid.as_str()) {
                    note = format!("{note} · {}", t!(ui, "materials-note-in-project"));
                }
                if self
                    .user
                    .library_state
                    .material_drafts
                    .contains_key(&material.uid)
                {
                    note = format!("{note} · {}", t!(ui, "library-view-note-unsaved"));
                }
                let row = Row::new(
                    Id::new(("browser-library-material", &material.uid)),
                    2,
                    &material.name,
                )
                .icon(Icon::Material)
                .trailing(note)
                .selected(self.focus == Focus::LibraryMaterial(uid_key(&material.uid)))
                .show(ui);
                if row.response.clicked() {
                    act = Some(MaterialAct::Focus(Focus::LibraryMaterial(uid_key(
                        &material.uid,
                    ))));
                }
                row.response.context_menu(|ui| {
                    if ui
                        .add_enabled(
                            self.edit.is_some(),
                            egui::Button::new(t!(ui, "materials-use-in-project")),
                        )
                        .clicked()
                    {
                        act = Some(MaterialAct::UseInProject(material.uid.clone()));
                    }
                    if ui.button(t!(ui, "common-delete")).clicked() {
                        act = Some(MaterialAct::Delete(material.uid.clone()));
                    }
                });
            }
        }
        if let Some(act) = act {
            self.material_act(act);
        }
    }

    /// Properties for the Materials folder or one material.
    pub(super) fn material_properties(&mut self, ui: &mut egui::Ui) {
        let mut act = None;
        match self.focus {
            Focus::LibraryMaterials => self.materials_folder_form(ui, &mut act),
            Focus::LibraryMaterial(key) => self.material_form(ui, &key_uid(key), &mut act),
            _ => {}
        }
        if let Some(act) = act {
            self.material_act(act);
        }
    }

    fn materials_folder_form(&self, ui: &mut egui::Ui, act: &mut Option<MaterialAct>) {
        title_line(ui, Icon::Material, None, &t!(ui, "materials-folder"));
        let path = self.user.settings_dir.as_deref().map(materials_path);
        library_header(ui, path.as_deref(), &self.user.library.problems);
        ui.label(t!(
            ui,
            "materials-count",
            count = self.user.library.materials.materials.len()
        ));
        ui.weak(t!(ui, "materials-about"));
        ui.add_space(4.0);
        if ui
            .add_enabled(path.is_some(), egui::Button::new(t!(ui, "materials-new")))
            .clicked()
        {
            *act = Some(MaterialAct::New);
        }
    }

    fn material_form(&mut self, ui: &mut egui::Ui, uid: &str, act: &mut Option<MaterialAct>) {
        let Some(stored) = self.user.library.materials.find_uid(uid).cloned() else {
            title_line(ui, Icon::Material, None, &t!(ui, "materials-folder"));
            ui.weak(t!(ui, "materials-gone"));
            return;
        };
        let mut draft = self
            .user
            .library_state
            .material_drafts
            .get(uid)
            .cloned()
            .unwrap_or_else(|| stored.clone());
        title_line(ui, Icon::Material, None, &stored.name);
        ui.weak(t!(ui, "materials-entry-about"));
        ui.add_space(4.0);
        let tools = self.user.library.tools.clone();
        egui::Grid::new("material-form")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(t!(ui, "materials-name"));
                ui.text_edit_singleline(&mut draft.name);
                ui.end_row();
            });
        ui.add_space(4.0);
        // Its values are the tools' rows for it (docs/specs/cutting-data.md §7): each tool
        // with one, a click away.
        ui.strong(t!(ui, "materials-values"));
        let with_rows: Vec<_> = tools
            .tools
            .iter()
            .filter(|t| t.row(Some(uid)).is_some())
            .collect();
        if with_rows.is_empty() {
            ui.weak(t!(ui, "materials-no-rows"));
        }
        for tool in with_rows {
            if ui
                .link(&tool.name)
                .on_hover_text(t!(ui, "materials-open-tool-hover"))
                .clicked()
            {
                *act = Some(MaterialAct::Focus(Focus::LibraryTool(uid_key(&tool.uid))));
            }
        }
        ui.weak(t!(ui, "materials-rows-about"));
        ui.add_space(4.0);
        let changed = draft != stored;
        if changed {
            self.user
                .library_state
                .material_drafts
                .insert(uid.to_string(), draft);
        } else {
            self.user.library_state.material_drafts.remove(uid);
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(changed, egui::Button::new(t!(ui, "common-save")))
                .on_hover_text(t!(ui, "library-view-save-hover"))
                .clicked()
            {
                *act = Some(MaterialAct::Save(uid.to_string()));
            }
            if ui
                .add_enabled(changed, egui::Button::new(t!(ui, "library-view-revert")))
                .on_hover_text(t!(ui, "library-view-revert-hover"))
                .clicked()
            {
                *act = Some(MaterialAct::Revert(uid.to_string()));
            }
            if changed {
                ui.colored_label(ui.visuals().warn_fg_color, t!(ui, "library-view-not-saved"));
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.edit.is_some() && !changed,
                    egui::Button::new(t!(ui, "materials-use-in-project")),
                )
                .on_hover_text(t!(ui, "materials-use-in-project-hover"))
                .on_disabled_hover_text(t!(ui, "library-view-entry-disabled-hover"))
                .clicked()
            {
                *act = Some(MaterialAct::UseInProject(uid.to_string()));
            }
            let is_default = self
                .user
                .library
                .materials
                .default_entry()
                .map(|m| m.uid.as_str())
                == Some(uid);
            if ui
                .add_enabled(
                    !is_default && !changed,
                    egui::Button::new(t!(ui, "library-view-make-default")),
                )
                .on_hover_text(t!(ui, "library-view-make-default-hover"))
                .clicked()
            {
                *act = Some(MaterialAct::MakeDefault(uid.to_string()));
            }
            if ui
                .button(t!(ui, "common-delete"))
                .on_hover_text(t!(ui, "library-view-delete-hover"))
                .clicked()
            {
                *act = Some(MaterialAct::Delete(uid.to_string()));
            }
        });
        if let Some(error) = &self.user.library_state.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }

    fn material_act(&mut self, act: MaterialAct) {
        self.user.library_state.error = None;
        match act {
            MaterialAct::Focus(focus) => {
                self.reload_library();
                self.focus = focus;
            }
            MaterialAct::New => {
                if let Some(uid) = self.new_material() {
                    self.focus = Focus::LibraryMaterial(uid_key(&uid));
                }
            }
            MaterialAct::Save(uid) => {
                self.save_material_draft(&uid);
            }
            MaterialAct::Revert(uid) => {
                self.user.library_state.material_drafts.remove(&uid);
            }
            MaterialAct::Delete(uid) => {
                let name = self
                    .user
                    .library
                    .materials
                    .find_uid(&uid)
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                if self
                    .change_materials(|library| library.remove(&name).map(|_| ()))
                    .is_ok()
                {
                    // Its rows go from the tools too (docs/specs/cutting-data.md §2).
                    if let Some(dir) = self.user.settings_dir.clone()
                        && let Err(err) =
                            chipload_pipeline::library::forget_material_in_tools(&dir, &uid)
                    {
                        self.user.library_state.error = Some(self.say(&err.message()));
                    }
                    self.reload_library();
                    self.user.library_state.material_drafts.remove(&uid);
                    self.focus = Focus::LibraryMaterials;
                    let status = t!(self, "library-view-deleted", name = name);
                    self.set_status(status, STATUS_SECONDS);
                }
            }
            MaterialAct::UseInProject(uid) => {
                self.use_material(Some(&uid));
            }
            MaterialAct::MakeDefault(uid) => {
                self.make_default_material(&uid);
            }
        }
    }

    /// A new material, "New material" (or "New material 2" …); its uid.
    pub fn new_material(&mut self) -> Option<String> {
        self.reload_library();
        let base = t!(self, "materials-new-name");
        let name = unique_name(&base, |n| self.user.library.materials.find(n).is_some());
        self.change_materials(|library| library.add(&name).map(|m| m.uid.clone()))
            .ok()
    }

    /// After operation `i`'s tool changed: it takes its material's values for the new
    /// tool, if the material has them (docs/specs/materials.md §2).
    pub fn follow_op_material(&mut self, i: usize) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let values = edit.job.operations.get(i).and_then(|e| {
            let material = self
                .user
                .library
                .materials
                .find_uid(&e.material()?.library_uid)?;
            chipload_pipeline::materials::tool_values(
                &edit.job,
                &self.user.library.tools,
                Some(material),
                e.tool(),
            )
        });
        if let (Some(values), Some(entry)) = (values, edit.job.operations.get_mut(i)) {
            entry.apply_cutting(&values);
        }
    }

    /// Make material `uid` the one new projects start with
    /// (docs/specs/job-settings-layout.md §3).
    pub fn make_default_material(&mut self, uid: &str) -> bool {
        let Some(name) = self
            .user
            .library
            .materials
            .find_uid(uid)
            .map(|m| m.name.clone())
        else {
            return false;
        };
        self.change_materials(|library| {
            library.default_material = Some(name);
            Ok(())
        })
        .is_ok()
    }

    /// Write the draft of material `uid` into the library file.
    pub fn save_material_draft(&mut self, uid: &str) -> bool {
        let Some(mut draft) = self.user.library_state.material_drafts.get(uid).cloned() else {
            return false;
        };
        draft.name = draft.name.trim().to_string();
        let lang = self.lang();
        let saved = self.change_materials(|library| {
            if library.materials.iter().any(|m| {
                m.uid != draft.uid && chipload_job::library::same_name(&m.name, &draft.name)
            }) {
                return Err(LibraryError::Refused(chipload_i18n::Value::Text(t!(
                    lang,
                    "materials-name-taken",
                    name = &draft.name
                ))));
            }
            let old = library.find_uid(&draft.uid).map(|m| m.name.clone());
            if let Some(old) = old {
                library.rename(&old, &draft.name);
            }
            match library.materials.iter_mut().find(|m| m.uid == draft.uid) {
                Some(m) => *m = draft.clone(),
                None => library.materials.push(draft.clone()),
            }
            Ok(())
        });
        if saved.is_ok() {
            self.user.library_state.material_drafts.remove(uid);
            let status = t!(self, "library-view-saved");
            self.set_status(status, STATUS_SECONDS);
        }
        saved.is_ok()
    }

    /// Change `materials.json` and read the library again; a refusal is shown in the form.
    fn change_materials<T>(
        &mut self,
        change: impl FnOnce(&mut MaterialLibrary) -> Result<T, LibraryError>,
    ) -> Result<T, ()> {
        let Some(dir) = self.user.settings_dir.clone() else {
            return Err(());
        };
        let result = update_library(&materials_path(&dir), change);
        self.reload_library();
        result.map_err(|e| {
            self.user.library_state.error = Some(self.say(&e.message()));
        })
    }
}
