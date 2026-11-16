// SPDX-License-Identifier: GPL-3.0-or-later
//! Values saved into the user's library: a project tool or the machine, new entries, the
//! defaults, entries removed.

use chipload_job::ToolChange;
use chipload_job::library::{
    LibraryError, LibraryTool, MachineProfile, same_name, update_machines, update_tools,
};
use chipload_pipeline::library::{machines_path, tools_path};
use chipload_pipeline::{DEFAULT_MAX_OVERCUT, DEFAULT_RPM, DEFAULT_SAFE_Z, DEFAULT_TOOL_DIAMETER};

use super::user_library::{refused, unique_name};
use super::*;
use crate::i18n::Lang;

/// Which library entry a save goes to.
enum Pick {
    Name(String),
    Uid(String),
}

impl ChiploadApp {
    /// Save the project's machine or tool into the library under `name`: an entry of that
    /// name is replaced and keeps its uid, so projects linked to it stay linked; otherwise a
    /// new entry is added. The project is linked to it (docs/specs/library-in-browser.md §3).
    pub fn save_to_library(&mut self, target: SaveTarget, name: &str) -> bool {
        self.save_values(target, Pick::Name(name.trim().to_string()))
    }

    /// Save the project's machine or tool into the entry it is linked to, keeping the
    /// entry's name and uid.
    pub fn save_to_linked(&mut self, target: SaveTarget) -> bool {
        let uid = self.edit.as_ref().and_then(|e| match target {
            SaveTarget::Machine => e.job.machine.library_uid.clone(),
            SaveTarget::Tool(id) => e
                .job
                .tools
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.library_uid.clone()),
        });
        match uid {
            Some(uid) => self.save_values(target, Pick::Uid(uid)),
            None => false,
        }
    }

    fn save_values(&mut self, target: SaveTarget, pick: Pick) -> bool {
        let (Some(dir), Some(edit)) = (self.user.settings_dir.clone(), self.edit.as_ref()) else {
            return false;
        };
        let settings = edit.job.clone();
        let lang = self.lang();
        // The entry's name and uid after saving.
        let result: Result<(String, String), LibraryError> = match target {
            SaveTarget::Machine => update_machines(&machines_path(&dir), |library| {
                let at = match &pick {
                    Pick::Name(name) => library.position(name),
                    Pick::Uid(uid) => library.machines.iter().position(|m| &m.uid == uid),
                };
                match (at, &pick) {
                    (Some(i), _) => {
                        let old = &library.machines[i];
                        let profile = MachineProfile::from_job(&old.name, &old.uid, &settings);
                        library.machines[i] = profile.clone();
                        Ok((profile.name, profile.uid))
                    }
                    (None, Pick::Name(name)) => {
                        let uid = library.new_uid(name);
                        library
                            .machines
                            .push(MachineProfile::from_job(name, &uid, &settings));
                        Ok((name.clone(), uid))
                    }
                    (None, Pick::Uid(_)) => Err(refused(t!(lang, "user-library-profile-gone"))),
                }
            }),
            SaveTarget::Tool(id) => match settings.tool(id) {
                Some(tool) => update_tools(&tools_path(&dir), |library| {
                    let at = match &pick {
                        Pick::Name(name) => library.position(name),
                        Pick::Uid(uid) => library.tools.iter().position(|t| &t.uid == uid),
                    };
                    match (at, &pick) {
                        // A project tool has no cutting values: keep the library's.
                        (Some(i), _) => {
                            let old = &library.tools[i];
                            let entry = LibraryTool::from_tool(
                                &old.name,
                                &old.uid,
                                tool,
                                old.cutting_data.clone(),
                            );
                            library.tools[i] = entry.clone();
                            Ok((entry.name, entry.uid))
                        }
                        (None, Pick::Name(name)) => {
                            let uid = library.new_uid(name);
                            library.tools.push(LibraryTool::from_tool(
                                name,
                                &uid,
                                tool,
                                Vec::new(),
                            ));
                            Ok((name.clone(), uid))
                        }
                        (None, Pick::Uid(_)) => {
                            Err(refused(t!(lang, "user-library-library-tool-gone")))
                        }
                    }
                }),
                None => Err(refused(t!(lang, "user-library-no-tool-to-save"))),
            },
        };
        self.reload_library();
        let (name, uid) = match result {
            Ok(saved) => saved,
            Err(err) => {
                let title = t!(self, "user-library-not-saved");
                self.error_report(Report::failed(title, err.message()));
                return false;
            }
        };
        if let Some(edit) = self.edit.as_mut() {
            match target {
                SaveTarget::Machine => {
                    edit.job.machine.name = Some(name.clone());
                    edit.job.machine.library_uid = Some(uid);
                }
                SaveTarget::Tool(id) => {
                    if let Some(tool) = edit.job.tools.iter_mut().find(|t| t.id == id) {
                        tool.name = Some(name.clone());
                        tool.library_uid = Some(uid);
                    }
                }
            }
        }
        self.commit();
        let status = t!(self, "user-library-saved", name = &name);
        self.set_status(status, STATUS_SECONDS);
        true
    }

    pub(super) fn library_name_prompt(
        &mut self,
        ctx: &egui::Context,
        target: SaveTarget,
        mut text: String,
    ) {
        let (title, taken) = match target.kind() {
            LibraryKind::Machines => (
                t!(ctx, "user-library-save-machine-title"),
                self.user
                    .library
                    .machines
                    .find(text.trim())
                    .map(|m| m.name.clone()),
            ),
            LibraryKind::Tools => (
                t!(ctx, "user-library-save-tool-title"),
                self.user
                    .library
                    .tools
                    .find(text.trim())
                    .map(|t| t.name.clone()),
            ),
        };
        let mut done = false;
        let mut save = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(t!(ui, "user-library-name-in-library"));
                ui.add(TextEdit::singleline(&mut text).desired_width(320.0));
                if let Some(existing) = &taken {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t!(ui, "user-library-replaces", name = existing),
                    );
                }
                ui.horizontal(|ui| {
                    let label = if taken.is_some() {
                        t!(ui, "user-library-replace")
                    } else {
                        t!(ui, "common-save")
                    };
                    if ui
                        .add_enabled(!text.trim().is_empty(), egui::Button::new(label))
                        .clicked()
                    {
                        save = true;
                        done = true;
                    }
                    if ui.button(t!(ui, "common-cancel")).clicked() {
                        done = true;
                    }
                });
            });
        self.dialogs.prompt = (!done).then(|| Prompt::LibraryName(target, text.clone()));
        if save {
            self.save_to_library(target, &text);
        }
    }

    /// Store a library tool edited in Properties: it replaces the tool with its uid, or is
    /// added. A name another tool has is refused.
    pub fn store_library_tool(&mut self, entry: LibraryTool) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let lang = self.lang();
        let result = update_tools(&tools_path(&dir), |library| {
            if library
                .tools
                .iter()
                .any(|t| t.uid != entry.uid && same_name(&t.name, &entry.name))
            {
                return Err(refused(t!(
                    lang,
                    "user-library-tool-exists",
                    name = &entry.name
                )));
            }
            match library.tools.iter().position(|t| t.uid == entry.uid) {
                Some(i) => {
                    if library
                        .default_tool
                        .as_deref()
                        .is_some_and(|d| same_name(d, &library.tools[i].name))
                    {
                        library.default_tool = Some(entry.name.clone());
                    }
                    library.tools[i] = entry;
                }
                None => library.tools.push(entry),
            }
            Ok(())
        });
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }

    /// Store a machine profile edited in Properties, as `store_library_tool`.
    pub fn store_machine_profile(&mut self, profile: MachineProfile) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let lang = self.lang();
        let result = update_machines(&machines_path(&dir), |library| {
            if library
                .machines
                .iter()
                .any(|m| m.uid != profile.uid && same_name(&m.name, &profile.name))
            {
                return Err(refused(t!(
                    lang,
                    "user-library-machine-exists",
                    name = &profile.name
                )));
            }
            match library.machines.iter().position(|m| m.uid == profile.uid) {
                Some(i) => {
                    if library
                        .default_machine
                        .as_deref()
                        .is_some_and(|d| same_name(d, &library.machines[i].name))
                    {
                        library.default_machine = Some(profile.name.clone());
                    }
                    library.machines[i] = profile;
                }
                None => library.machines.push(profile),
            }
            Ok(())
        });
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }

    /// A new library tool or machine, saved at once under a free name ("New tool 2") and
    /// shown in Properties. It starts from the open project's first tool or machine.
    pub fn new_library_entry(&mut self, kind: LibraryKind) -> Option<String> {
        self.reload_library();
        let result = match kind {
            LibraryKind::Tools => {
                let base = t!(self, "user-library-new-tool");
                let name = unique_name(&base, |n| self.user.library.tools.find(n).is_some());
                let uid = self.user.library.tools.new_uid(&name);
                let entry = match self.edit.as_ref().and_then(|e| e.job.tools.first()) {
                    Some(tool) => LibraryTool::from_tool(&name, &uid, tool, Vec::new()),
                    None => LibraryTool {
                        name,
                        uid: uid.clone(),
                        id: 1,
                        kind: ToolType::Endmill,
                        diameter: DEFAULT_TOOL_DIAMETER,
                        flutes: None,
                        center_cutting: None,
                        rpm: Some(DEFAULT_RPM),
                        cutting_data: Vec::new(),
                        angle: None,
                        tip_diameter: None,
                        corner_radius: None,
                    },
                };
                self.store_library_tool(entry).map(|()| uid)
            }
            LibraryKind::Machines => {
                let base = t!(self, "user-library-new-machine");
                let name = unique_name(&base, |n| self.user.library.machines.find(n).is_some());
                let uid = self.user.library.machines.new_uid(&name);
                let profile = match self.edit.as_ref() {
                    Some(edit) => MachineProfile::from_job(&name, &uid, &edit.job),
                    None => MachineProfile {
                        name,
                        uid: uid.clone(),
                        post: "grbl".into(),
                        gcode_units: chipload_job::Units::Mm,
                        safe_z: DEFAULT_SAFE_Z,
                        spindle: Spindle::Cw,
                        max_feed: None,
                        travel: None,
                        // As a new project (docs/specs/work-zero-marker.md §7).
                        work_zero: WorkZero::Stock {
                            corner: chipload_job::StockCorner::BottomLeft,
                        },
                        max_overcut: DEFAULT_MAX_OVERCUT,
                        tool_change: ToolChange::Files,
                    },
                };
                self.store_machine_profile(profile).map(|()| uid)
            }
        };
        match result {
            Ok(uid) => {
                self.show_library(kind, Some(&uid));
                Some(uid)
            }
            Err(err) => {
                self.user.library_state.error = Some(err);
                None
            }
        }
    }

    pub(super) fn remove_from_library(
        &mut self,
        kind: LibraryKind,
        name: &str,
    ) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let result = match kind {
            // Its rows of cutting data go with it.
            LibraryKind::Tools => update_tools(&tools_path(&dir), |library| {
                library.remove(name).map(|_| ())
            }),
            LibraryKind::Machines => update_machines(&machines_path(&dir), |library| {
                library.remove(name).map(|_| ())
            }),
        };
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }

    /// The tool or machine new projects start with.
    pub fn set_library_default(&mut self, kind: LibraryKind, name: &str) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let result = match kind {
            LibraryKind::Tools => update_tools(&tools_path(&dir), |library| {
                library.default_tool = Some(name.to_string());
                Ok(())
            }),
            LibraryKind::Machines => update_machines(&machines_path(&dir), |library| {
                library.default_machine = Some(name.to_string());
                Ok(())
            }),
        };
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }
}
