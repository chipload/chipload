// SPDX-License-Identifier: GPL-3.0-or-later
//! Materials in the window (docs/specs/materials.md): choosing the material for every
//! operation, following it when an operation's tool changes, saving an operation's values
//! into its material, and the Library's Materials folder with a form
//! per material. Edits to a material stay a draft until *Save*, as for tools.

use chipload_job::library::{Cutting, LibraryError, update_library};
use chipload_job::materials::MaterialLibrary;
use chipload_pipeline::library::materials_path;
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
}

impl ChiploadApp {
    /// Choose the material for every operation and those added later (None: none), as one
    /// undo step: the operations take its values for their tools, the stock stays; the
    /// report says what changed.
    pub fn use_material(&mut self, uid: Option<&str>) -> bool {
        self.commit();
        self.reload_library();
        let material = match uid {
            Some(uid) => match self.library.materials.find_uid(uid) {
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
        let (job, messages) = use_material(&job, material.as_ref());
        let mut next = EditJob::from_job(&job);
        for (entry, old) in next.ops.iter_mut().zip(&edit.ops) {
            entry.side_chosen = old.side_chosen;
        }
        self.edit = Some(next);
        self.commit();
        self.set_report(Report::of_messages(messages));
        true
    }

    /// Save operation `i`'s values (0-based) for its tool in its material (else the
    /// project's).
    pub fn save_op_to_material(&mut self, i: usize) -> bool {
        self.commit();
        self.reload_library();
        let Some(dir) = self.settings_dir.clone() else {
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
            .or(job.stock.material.as_ref())
            .map(|m| m.library_uid.clone())
        else {
            return false;
        };
        let tools = self.library.tools.clone();
        let saved = update_library(&materials_path(&dir), |library: &mut MaterialLibrary| {
            let material = library
                .materials
                .iter_mut()
                .find(|m| m.uid == uid)
                .ok_or_else(|| LibraryError::Refused(chipload_i18n::Value::Text(uid.clone())))?;
            let (tool_uid, values) = values_to_save(&job, i + 1, &tools, material)
                .map_err(|e| LibraryError::Refused(e.message().into()))?;
            material.set_values(&tool_uid, values);
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
        let materials = self.library.materials.materials.clone();
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
                .and_then(|e| e.settings.stock.material.as_ref())
                .map(|m| m.library_uid.clone());
            for material in &materials {
                let mut note = t!(ui, "materials-tool-count", count = material.tools.len());
                if chosen.as_deref() == Some(material.uid.as_str()) {
                    note = format!("{note} · {}", t!(ui, "materials-note-in-project"));
                }
                if self
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
        let path = self.settings_dir.as_deref().map(materials_path);
        library_header(ui, path.as_deref(), &self.library.problems);
        ui.label(t!(
            ui,
            "materials-count",
            count = self.library.materials.materials.len()
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
        let Some(stored) = self.library.materials.find_uid(uid).cloned() else {
            title_line(ui, Icon::Material, None, &t!(ui, "materials-folder"));
            ui.weak(t!(ui, "materials-gone"));
            return;
        };
        let mut draft = self
            .library_state
            .material_drafts
            .get(uid)
            .cloned()
            .unwrap_or_else(|| stored.clone());
        title_line(ui, Icon::Material, None, &stored.name);
        ui.weak(t!(ui, "materials-entry-about"));
        ui.add_space(4.0);
        let tools = self.library.tools.clone();
        egui::Grid::new("material-form")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(t!(ui, "materials-name"));
                ui.text_edit_singleline(&mut draft.name);
                ui.end_row();
            });
        ui.add_space(4.0);
        ui.strong(t!(ui, "materials-values"));
        let mut remove = None;
        for (k, entry) in draft.tools.iter_mut().enumerate() {
            let name = tools.find_uid(&entry.tool_uid).map_or_else(
                || t!(ui, "materials-tool-not-in-library"),
                |t| t.name.clone(),
            );
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(name).strong());
                if ui
                    .small_button("×")
                    .on_hover_text(t!(ui, "materials-remove-tool-hover"))
                    .clicked()
                {
                    remove = Some(k);
                }
            });
            egui::Grid::new(("material-tool", k))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label(t!(ui, "forms-feed"));
                    ui.add(crate::units::feed(&mut entry.feed).range(1.0..=100_000.0));
                    ui.end_row();
                    ui.label(t!(ui, "forms-plunge-feed"));
                    ui.add(crate::units::feed(&mut entry.plunge_feed).range(1.0..=100_000.0));
                    ui.end_row();
                    ui.label(t!(ui, "forms-step-down"));
                    ui.add(crate::units::length(&mut entry.step_down).range(0.01..=1000.0));
                    ui.end_row();
                    ui.label(t!(ui, "forms-stepover"));
                    ui.add(
                        crate::units::number(ui, &mut entry.stepover)
                            .range(0.01..=1.0)
                            .speed(0.01),
                    );
                    ui.end_row();
                });
        }
        if let Some(k) = remove {
            draft.tools.remove(k);
        }
        // Another library tool's values: they start from the tool's own, else a new
        // operation's.
        let missing: Vec<_> = tools
            .tools
            .iter()
            .filter(|t| draft.values(&t.uid).is_none())
            .collect();
        if !missing.is_empty() {
            egui::ComboBox::from_id_salt("material-add-tool")
                .selected_text(t!(ui, "materials-add-tool"))
                .show_ui(ui, |ui| {
                    for tool in missing {
                        if ui.selectable_label(false, &tool.name).clicked() {
                            let values = tool.cutting.unwrap_or(Cutting {
                                feed: chipload_pipeline::chip_load::default_feed(tool.diameter),
                                plunge_feed: chipload_pipeline::chip_load::default_plunge_feed(
                                    tool.diameter,
                                ),
                                step_down: chipload_pipeline::DEFAULT_STEP_DOWN,
                                stepover: chipload_pipeline::materials::DEFAULT_STEPOVER,
                            });
                            draft.set_values(&tool.uid, values);
                        }
                    }
                });
        } else if tools.tools.is_empty() {
            ui.weak(t!(ui, "materials-no-library-tools"));
        }
        ui.add_space(4.0);
        let changed = draft != stored;
        if changed {
            self.library_state
                .material_drafts
                .insert(uid.to_string(), draft);
        } else {
            self.library_state.material_drafts.remove(uid);
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
            if ui
                .button(t!(ui, "common-delete"))
                .on_hover_text(t!(ui, "library-view-delete-hover"))
                .clicked()
            {
                *act = Some(MaterialAct::Delete(uid.to_string()));
            }
        });
        if let Some(error) = &self.library_state.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }

    fn material_act(&mut self, act: MaterialAct) {
        self.library_state.error = None;
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
                self.library_state.material_drafts.remove(&uid);
            }
            MaterialAct::Delete(uid) => {
                let name = self
                    .library
                    .materials
                    .find_uid(&uid)
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                if self
                    .change_materials(|library| library.remove(&name).map(|_| ()))
                    .is_ok()
                {
                    self.library_state.material_drafts.remove(&uid);
                    self.focus = Focus::LibraryMaterials;
                    let status = t!(self, "library-view-deleted", name = name);
                    self.set_status(status, STATUS_SECONDS);
                }
            }
            MaterialAct::UseInProject(uid) => {
                self.use_material(Some(&uid));
            }
        }
    }

    /// A new material, "New material" (or "New material 2" …); its uid.
    pub fn new_material(&mut self) -> Option<String> {
        self.reload_library();
        let base = t!(self, "materials-new-name");
        let name = unique_name(&base, |n| self.library.materials.find(n).is_some());
        self.change_materials(|library| library.add(&name).map(|m| m.uid.clone()))
            .ok()
    }

    /// After operation `i`'s tool changed: it takes its material's values for the new
    /// tool, if the material has them (docs/specs/materials.md §2).
    pub fn follow_op_material(&mut self, i: usize) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let values = edit.ops.get(i).and_then(|e| {
            let material = self
                .library
                .materials
                .find_uid(&e.op.material()?.library_uid)?;
            chipload_pipeline::materials::tool_values(&edit.settings, material, e.op.tool())
        });
        if let (Some(values), Some(entry)) = (values, edit.ops.get_mut(i)) {
            entry.op.apply_cutting(&values);
        }
    }

    /// Write the draft of material `uid` into the library file.
    pub fn save_material_draft(&mut self, uid: &str) -> bool {
        let Some(mut draft) = self.library_state.material_drafts.get(uid).cloned() else {
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
            match library.materials.iter_mut().find(|m| m.uid == draft.uid) {
                Some(m) => *m = draft.clone(),
                None => library.materials.push(draft.clone()),
            }
            Ok(())
        });
        if saved.is_ok() {
            self.library_state.material_drafts.remove(uid);
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
        let Some(dir) = self.settings_dir.clone() else {
            return Err(());
        };
        let result = update_library(&materials_path(&dir), change);
        self.reload_library();
        result.map_err(|e| {
            self.library_state.error = Some(self.say(&e.message()));
        })
    }
}
