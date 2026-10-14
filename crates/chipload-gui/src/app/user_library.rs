// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's tool library and machine profiles (docs/specs/tool-library-and-machines.md
//! §8, library-in-browser.md): the library rows of a project tool and the machine, and
//! copying values into and out of the project. The Browser rows and the entries' properties
//! are in library_view.rs. The rules are in `chipload_job::library`; this only shows and
//! edits values.

use std::collections::BTreeMap;

use chipload_job::library::{AppSettings, load_settings, update_settings};
use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, Link, MachineProfile, linked_cutting, machine_link,
    same_name, tool_link, update_machines, update_tools,
};
use chipload_job::{Tool, ToolChange};
use chipload_pipeline::library::{Library, machines_path, tools_path};
use chipload_pipeline::{DEFAULT_MAX_OVERCUT, DEFAULT_RPM, DEFAULT_SAFE_Z, DEFAULT_TOOL_DIAMETER};

use super::forms::{feed, machine_rows, mm, safe_z_row, tool_rows, work_zero_rows};
use super::library_transfer::PendingImport;
use super::library_view::uid_key;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryKind {
    Tools,
    Machines,
}

/// What a library row works on: the project's machine, or one of its tools (by id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveTarget {
    Machine,
    Tool(i64),
}

impl SaveTarget {
    fn kind(self) -> LibraryKind {
        match self {
            SaveTarget::Machine => LibraryKind::Machines,
            SaveTarget::Tool(_) => LibraryKind::Tools,
        }
    }
}

/// What the job form asks of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRequest {
    UseMachine(String),
    /// Take the linked profile's machine values again, keeping the job values.
    UpdateMachine,
    /// Give project tool `id` the values of the named library tool.
    UseTool(i64, String),
    /// Copy a library tool into the project as a tool of its own
    /// (docs/specs/tool-per-operation.md §8).
    AddTool(String),
    /// Ask for a name, then save the project's machine or tool into the library.
    Save(SaveTarget),
    /// Save the project's machine or tool into the entry it is linked to
    /// (docs/specs/library-in-browser.md §3).
    SaveLinked(SaveTarget),
    /// Show a library entry, by uid, in the Browser and Properties.
    Show(LibraryKind, String),
}

/// The library rows of a project tool or the machine: the entries to choose from, and how
/// the project's values stand to the entry they came from.
pub(super) struct LibraryRows {
    target: SaveTarget,
    /// None: no settings folder, so no library.
    names: Option<Vec<String>>,
    /// The link, and the linked entry's uid while it is in the library.
    link: Option<(Link, Option<String>)>,
}

impl LibraryRows {
    pub(super) fn machine(job: &Job, library: Option<&Library>) -> Self {
        let names = library.map(|l| l.machines.machines.iter().map(|m| m.name.clone()).collect());
        let link = library.map(|l| {
            let link = machine_link(&l.machines, job);
            let uid = job
                .machine
                .library_uid
                .clone()
                .filter(|_| link.entry_name().is_some());
            (link, uid)
        });
        LibraryRows {
            target: SaveTarget::Machine,
            names,
            link,
        }
    }

    pub(super) fn tool(tool: &Tool, library: Option<&Library>) -> Self {
        let names = library.map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect());
        let link = library.map(|l| {
            let link = tool_link(&l.tools, tool);
            let uid = tool
                .library_uid
                .clone()
                .filter(|_| link.entry_name().is_some());
            (link, uid)
        });
        LibraryRows {
            target: SaveTarget::Tool(tool.id),
            names,
            link,
        }
    }

    fn what(&self) -> &'static str {
        match self.target {
            SaveTarget::Machine => "profile",
            SaveTarget::Tool(_) => "library tool",
        }
    }
}

#[cfg(test)]
impl LibraryRows {
    pub(super) fn note_text(&self) -> Option<String> {
        self.link.as_ref().map(|(link, _)| link.text(self.what()))
    }
}

fn target_salt(target: SaveTarget) -> i64 {
    match target {
        SaveTarget::Machine => -1,
        SaveTarget::Tool(id) => id,
    }
}

pub(super) fn library_rows(
    ui: &mut egui::Ui,
    request: &mut Option<LibraryRequest>,
    rows: LibraryRows,
) {
    let target = rows.target;
    let what = rows.what();
    let choose = match target {
        SaveTarget::Machine => "Use profile…",
        SaveTarget::Tool(_) => "Use library tool…",
    };
    let use_entry = |name: String| match target {
        SaveTarget::Machine => LibraryRequest::UseMachine(name),
        SaveTarget::Tool(id) => LibraryRequest::UseTool(id, name),
    };
    let linked = rows.link.as_ref().and_then(|(_, uid)| uid.clone());
    ui.label("Library");
    ui.horizontal_wrapped(|ui| {
        let available = rows.names.is_some();
        let names = rows.names.clone().unwrap_or_default();
        ui.add_enabled_ui(available && !names.is_empty(), |ui| {
            ComboBox::from_id_salt(("library", choose, target_salt(target)))
                .selected_text(choose)
                .show_ui(ui, |ui| {
                    for name in names {
                        if ui.selectable_label(false, &name).clicked() {
                            *request = Some(use_entry(name));
                        }
                    }
                })
        });
        if !available {
            ui.add_enabled(false, egui::Button::new("Save to library…"))
                .on_disabled_hover_text(format!(
                    "No settings folder found; set {}",
                    chipload_pipeline::library::CONFIG_DIR_VAR
                ));
        } else if linked.is_some() {
            if ui
                .button("Save to library")
                .on_hover_text(format!(
                    "Write these values into the {what} it is linked to, for other projects"
                ))
                .clicked()
            {
                *request = Some(LibraryRequest::SaveLinked(target));
            }
            if ui
                .button("Save as new…")
                .on_hover_text("Keep these values in your library under another name")
                .clicked()
            {
                *request = Some(LibraryRequest::Save(target));
            }
        } else if ui
            .button("Save to library…")
            .on_hover_text("Copies these values into your library, for other projects")
            .clicked()
        {
            *request = Some(LibraryRequest::Save(target));
        }
    });
    ui.end_row();
    let Some((link, uid)) = rows.link else {
        return;
    };
    ui.label("");
    ui.horizontal_wrapped(|ui| {
        ui.weak(link.text(what));
        if let Link::Differs(name, _) = &link
            && ui
                .button("Update from library")
                .on_hover_text("Copy the library's values into the project (undo with Ctrl+Z)")
                .clicked()
        {
            *request = Some(match target {
                SaveTarget::Machine => LibraryRequest::UpdateMachine,
                SaveTarget::Tool(_) => use_entry(name.clone()),
            });
        }
        if let Some(uid) = uid
            && ui
                .button("Show in library")
                .on_hover_text("Open it under This computer in the Browser")
                .clicked()
        {
            *request = Some(LibraryRequest::Show(target.kind(), uid));
        }
    });
    ui.end_row();
}

/// The library's state in the app: entries being edited, and what import and export said.
#[derive(Debug, Clone, Default)]
pub(super) struct LibraryState {
    /// Tools being edited and not saved yet, by uid.
    pub(super) tool_drafts: BTreeMap<String, LibraryTool>,
    /// Machine profiles being edited and not saved yet, by uid.
    pub(super) machine_drafts: BTreeMap<String, MachineProfile>,
    pub(super) error: Option<String>,
    /// What the last import or export did, shown in that folder's properties.
    pub(super) message: Option<(LibraryKind, String)>,
    /// A file read for import, waiting for Import or Cancel.
    pub(super) import: Option<PendingImport>,
    /// A Library folder to open in the Browser on its next frame.
    pub(super) reveal: Option<LibraryKind>,
    /// The facts of the post shown in Properties, by name (docs/specs/this-computer.md §4).
    pub(super) post_facts: Option<(String, Result<Vec<String>, String>)>,
}

/// "New tool", or "New tool 2" … if that is taken.
pub(super) fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while taken(&name) {
        name = format!("{base} {n}");
        n += 1;
    }
    name
}

/// Which library entry a save goes to.
enum Pick {
    Name(String),
    Uid(String),
}

fn refused(text: String) -> LibraryError {
    LibraryError::Refused(text)
}

/// Where the library is kept, and why it couldn't be read.
pub(super) fn library_header(ui: &mut egui::Ui, path: Option<&Path>, problems: &[String]) {
    match path {
        Some(path) => {
            ui.weak(format!("Kept in {}", path.display()));
        }
        None => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!(
                    "No settings folder found; set {} to use a library.",
                    chipload_pipeline::library::CONFIG_DIR_VAR
                ),
            );
        }
    }
    for problem in problems {
        ui.colored_label(ui.visuals().error_fg_color, problem);
    }
}

pub(super) fn tool_entry_form(ui: &mut egui::Ui, entry: &mut LibraryTool) {
    Grid::new("library-tool").num_columns(2).show(ui, |ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut entry.name);
        ui.end_row();
        ui.label("Tool number")
            .on_hover_text("The id it gets in a project where that id is free");
        ui.add(DragValue::new(&mut entry.id).range(1..=9999));
        ui.end_row();
        let mut tool = entry.tool(entry.id);
        tool_rows(ui, "library", "", &mut tool);
        entry.kind = tool.kind;
        entry.diameter = tool.diameter;
        entry.rpm = tool.rpm;
        entry.center_cutting = tool.center_cutting;
        entry.angle = tool.angle;
        entry.tip_diameter = tool.tip_diameter;
        ui.label("Flutes");
        ui.horizontal(|ui| {
            let mut set = entry.flutes.is_some();
            if ui.checkbox(&mut set, "").changed() {
                entry.flutes = set.then_some(2);
            }
            if let Some(flutes) = &mut entry.flutes {
                ui.add(DragValue::new(flutes).range(1..=16));
            }
        });
        ui.end_row();
        ui.label("Cutting values").on_hover_text(
            "Filled into a new operation with this tool when there is no operation above to \
             copy from. Never changes an existing operation.",
        );
        let mut set = entry.cutting.is_some();
        if ui.checkbox(&mut set, "").changed() {
            entry.cutting = set.then_some(Cutting {
                feed: crate::editor::DEFAULT_FEED,
                plunge_feed: crate::editor::DEFAULT_PLUNGE_FEED,
                step_down: crate::editor::DEFAULT_STEP_DOWN,
                stepover: crate::editor::DEFAULT_STEPOVER,
            });
        }
        ui.end_row();
        if let Some(c) = &mut entry.cutting {
            ui.label("  Feed");
            ui.add(feed(&mut c.feed));
            ui.end_row();
            ui.label("  Plunge feed");
            ui.add(feed(&mut c.plunge_feed));
            ui.end_row();
            ui.label("  Step down");
            ui.add(mm(&mut c.step_down));
            ui.end_row();
            ui.label("  Stepover");
            ui.add(
                DragValue::new(&mut c.stepover)
                    .speed(0.01)
                    .range(0.01..=1.0)
                    .suffix(" × D"),
            );
            ui.end_row();
        }
    });
}

pub(super) fn machine_entry_form(
    ui: &mut egui::Ui,
    profile: &mut MachineProfile,
    library: &Library,
) {
    Grid::new("library-machine").num_columns(2).show(ui, |ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut profile.name);
        ui.end_row();
        let mut machine = profile.machine();
        // A profile only names its post; it is copied into a project when used.
        if let Some(post) = machine_rows(ui, "library", &mut machine, Some(library), None) {
            machine.post = post;
        }
        safe_z_row(ui, "Safe Z", &mut machine);
        work_zero_rows(ui, "library", &mut machine.work_zero, (0.0, 0.0));
        profile.post = machine.post;
        profile.safe_z = machine.safe_z;
        profile.spindle = machine.spindle;
        profile.max_feed = machine.max_feed;
        profile.travel = machine.travel;
        profile.work_zero = machine.work_zero;
        ui.label("Max depth below stock")
            .on_hover_text("How far a cut may go below the stock (into the spoilboard)");
        ui.add(mm(&mut profile.max_overcut));
        ui.end_row();
    });
}

/// What the last import or export of this window did.
pub(super) fn library_message(
    ui: &mut egui::Ui,
    message: Option<&(LibraryKind, String)>,
    kind: LibraryKind,
) {
    if let Some((_, text)) = message.filter(|(k, _)| *k == kind) {
        ui.separator();
        ui.label(text);
    }
}

pub(super) fn tool_summary(t: &LibraryTool) -> String {
    format!("{} {} mm", t.kind.name(), t.diameter)
}

pub(super) fn machine_summary(m: &MachineProfile) -> String {
    match &m.travel {
        Some(t) => format!("{}, {} × {} × {} mm", m.post, t.x, t.y, t.z),
        None => m.post.clone(),
    }
}

impl ChiploadApp {
    /// Where the library is kept (None in tests and without a home folder), and read it.
    pub fn set_settings_dir(&mut self, dir: Option<PathBuf>) {
        self.settings_dir = dir;
        self.reload_library();
        let settings = self
            .settings_dir
            .as_deref()
            .map(chipload_pipeline::library::settings_path)
            .and_then(|path| load_settings(&path).ok())
            .unwrap_or_default();
        self.recent = settings.recent_projects;
        self.theme = settings.theme;
        self.auto_generate = settings.auto_generate;
        self.setup_sheet = settings.setup_sheet;
        self.show_grid = settings.show_grid;
        self.start_recovery();
    }

    /// Settings > Colour theme: used at once and kept in the settings for next time.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
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
        let Some(dir) = self.settings_dir.clone() else {
            return;
        };
        let path = chipload_pipeline::library::settings_path(&dir);
        let result = update_settings(&path, |settings| {
            change(settings);
            Ok(settings.recent_projects.clone())
        });
        match result {
            Ok(recent) => self.recent = recent,
            Err(err) => self.set_status(format!("Settings not saved: {err}"), ERROR_STATUS_SECONDS),
        }
    }

    /// Open a project from File > Open recent; one that is gone is reported and forgotten.
    pub fn open_recent(&mut self, path: &str) -> bool {
        let file = PathBuf::from(path);
        if !file.exists() {
            self.error(vec![format!(
                "{path} no longer exists; it was taken off the recent list."
            )]);
            self.forget_recent(Some(path));
            return false;
        }
        if self.guard(Pending::Open(file.clone())) {
            return self.open(&file);
        }
        false
    }

    /// Read the library again (it may have been changed by the CLI or another window).
    pub(super) fn reload_library(&mut self) {
        self.library_state.post_facts = None;
        self.library = self
            .settings_dir
            .as_deref()
            .map(Library::load)
            .unwrap_or_default();
    }

    /// The library, if there is a settings folder.
    pub(super) fn library(&self) -> Option<&Library> {
        self.settings_dir.as_ref().map(|_| &self.library)
    }

    /// A project tool's cutting values from the library: only when the tool came from the
    /// library and still has its values.
    pub(super) fn tool_cutting(&self, id: i64) -> Option<Cutting> {
        let tool = self
            .edit
            .as_ref()?
            .settings
            .tools
            .iter()
            .find(|t| t.id == id)?;
        linked_cutting(&self.library()?.tools, tool)
    }

    pub(super) fn library_request(&mut self, request: LibraryRequest) {
        match request {
            LibraryRequest::UseMachine(name) => {
                self.use_machine_profile(&name);
            }
            LibraryRequest::UpdateMachine => {
                self.update_machine_from_library();
            }
            LibraryRequest::UseTool(id, name) => {
                self.use_library_tool(id, &name);
            }
            LibraryRequest::AddTool(name) => {
                self.add_library_tool(&name);
            }
            LibraryRequest::Save(target) => {
                let current = self.edit.as_ref().and_then(|e| match target {
                    SaveTarget::Machine => e.settings.machine.name.clone(),
                    SaveTarget::Tool(id) => e
                        .settings
                        .tools
                        .iter()
                        .find(|t| t.id == id)
                        .and_then(|t| t.name.clone()),
                });
                self.prompt = Some(Prompt::LibraryName(target, current.unwrap_or_default()));
            }
            LibraryRequest::SaveLinked(target) => {
                self.save_to_linked(target);
            }
            LibraryRequest::Show(kind, uid) => self.show_library(kind, Some(&uid)),
        }
    }

    /// Open the Library in the Browser at `kind`, and show the entry `uid` (or the folder)
    /// in Properties (docs/specs/library-in-browser.md §6).
    pub fn show_library(&mut self, kind: LibraryKind, uid: Option<&str>) {
        self.reload_library();
        self.library_state.reveal = Some(kind);
        self.focus = match (kind, uid) {
            (LibraryKind::Tools, None) => Focus::LibraryTools,
            (LibraryKind::Machines, None) => Focus::LibraryMachines,
            (LibraryKind::Tools, Some(uid)) => Focus::LibraryTool(uid_key(uid)),
            (LibraryKind::Machines, Some(uid)) => Focus::LibraryMachine(uid_key(uid)),
        };
    }

    /// Make `name` the project's post-processor; one of the user's is read again and copied
    /// in (docs/specs/user-posts.md §6). One undo step.
    pub fn choose_project_post(&mut self, name: &str) -> bool {
        self.reload_library();
        let posts = self.library.posts.clone();
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        if let Err(err) =
            chipload_pipeline::posts::choose_post(&mut edit.settings, name, posts.as_deref())
        {
            self.error(vec![format!("Post-processor \"{name}\":"), err.to_string()]);
            return false;
        }
        self.commit();
        self.set_status(format!("Post-processor: {name}"), STATUS_SECONDS);
        true
    }

    /// Copy a machine profile's values into the project (one undo step).
    pub fn use_machine_profile(&mut self, name: &str) -> bool {
        self.reload_library();
        let Some(profile) = self.library.machines.find(name).cloned() else {
            self.error(vec![format!("There is no machine profile \"{name}\".")]);
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut changes = profile.apply(&mut edit.settings);
        changes.extend(chipload_pipeline::library::copy_profile_post(
            &mut edit.settings,
            self.library.posts.as_deref(),
        ));
        self.commit();
        self.report_changes(&format!("Machine profile \"{}\"", profile.name), changes);
        true
    }

    /// Take the linked profile's machine values again (post, spindle, max feed, tool
    /// changes, travel), keeping the project's safe Z, work zero and max depth. One undo
    /// step.
    pub fn update_machine_from_library(&mut self) -> bool {
        self.reload_library();
        let profile = self.edit.as_ref().and_then(|e| {
            let uid = e.settings.machine.library_uid.as_deref()?;
            self.library.machines.find_uid(uid).cloned()
        });
        let Some(profile) = profile else {
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut changes = profile.update_machine_values(&mut edit.settings);
        changes.extend(chipload_pipeline::library::copy_profile_post(
            &mut edit.settings,
            self.library.posts.as_deref(),
        ));
        self.commit();
        self.report_changes(&format!("Machine profile \"{}\"", profile.name), changes);
        true
    }

    /// Give project tool `id` a library tool's values; it keeps its id, so the operations
    /// that use it still do (one undo step).
    pub fn use_library_tool(&mut self, id: i64, name: &str) -> bool {
        self.reload_library();
        let Some(entry) = self.library.tools.find(name).cloned() else {
            self.error(vec![format!("There is no library tool \"{name}\".")]);
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let Some(tool) = edit.settings.tools.iter_mut().find(|t| t.id == id) else {
            self.error(vec![format!("The project has no tool {id}.")]);
            return false;
        };
        let changes = entry.changes(tool);
        *tool = entry.tool(id);
        self.commit();
        self.report_changes(&format!("Library tool \"{}\"", entry.name), changes);
        true
    }

    /// Copy a library tool into the project as a tool of its own, or find it there already
    /// (docs/specs/tool-per-operation.md §8). Returns its id in the project.
    pub fn add_library_tool(&mut self, name: &str) -> Option<i64> {
        let id = self.insert_library_tool(name)?;
        self.commit();
        Some(id)
    }

    /// Cut operation `op` with a library tool, copied into the project if it isn't there
    /// yet: one undo step. The operation keeps its feeds (decision 6).
    pub fn tool_from_library(&mut self, op: usize, name: &str) -> bool {
        let Some(id) = self.insert_library_tool(name) else {
            return false;
        };
        let Some(entry) = self.edit.as_mut().and_then(|e| e.ops.get_mut(op)) else {
            return false;
        };
        entry.op.set_tool(id);
        if let Some(edit) = &mut self.edit
            && let Some(entry) = edit.ops.get_mut(op)
        {
            crate::editor::fit_hole_method(&mut entry.op, &edit.settings.tools);
        }
        self.commit();
        true
    }

    fn insert_library_tool(&mut self, name: &str) -> Option<i64> {
        self.reload_library();
        let Some(entry) = self.library.tools.find(name).cloned() else {
            self.error(vec![format!("There is no library tool \"{name}\".")]);
            return None;
        };
        let edit = self.edit.as_mut()?;
        let used = chipload_job::library::use_tool(&mut edit.settings, &entry);
        self.set_status(
            format!("\"{}\": {}", entry.name, used.describe(entry.id)),
            STATUS_SECONDS,
        );
        Some(used.id)
    }

    fn report_changes(&mut self, what: &str, changes: Vec<String>) {
        let status = if changes.is_empty() {
            format!("{what}: no values changed")
        } else {
            format!("{what}: {} value(s) changed", changes.len())
        };
        let mut lines = vec![format!("{what}:")];
        lines.extend(changes.iter().map(|c| format!("  {c}")));
        if changes.is_empty() {
            lines.push("  no values changed".into());
        }
        self.set_report(Report::info(lines));
        self.set_status(status, STATUS_SECONDS);
    }

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
            SaveTarget::Machine => e.settings.machine.library_uid.clone(),
            SaveTarget::Tool(id) => e
                .settings
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
        let (Some(dir), Some(edit)) = (self.settings_dir.clone(), self.edit.as_ref()) else {
            return false;
        };
        let settings = edit.settings.clone();
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
                    (None, Pick::Uid(_)) => Err(refused(
                        "The profile it came from is not in your library.".into(),
                    )),
                }
            }),
            SaveTarget::Tool(id) => match settings.tools.iter().find(|t| t.id == id) {
                Some(tool) => update_tools(&tools_path(&dir), |library| {
                    let at = match &pick {
                        Pick::Name(name) => library.position(name),
                        Pick::Uid(uid) => library.tools.iter().position(|t| &t.uid == uid),
                    };
                    match (at, &pick) {
                        // A project tool has no cutting values: keep the library's.
                        (Some(i), _) => {
                            let old = &library.tools[i];
                            let entry =
                                LibraryTool::from_tool(&old.name, &old.uid, tool, old.cutting);
                            library.tools[i] = entry.clone();
                            Ok((entry.name, entry.uid))
                        }
                        (None, Pick::Name(name)) => {
                            let uid = library.new_uid(name);
                            library
                                .tools
                                .push(LibraryTool::from_tool(name, &uid, tool, None));
                            Ok((name.clone(), uid))
                        }
                        (None, Pick::Uid(_)) => Err(refused(
                            "The library tool it came from is not in your library.".into(),
                        )),
                    }
                }),
                None => Err(refused("The project has no tool to save.".into())),
            },
        };
        self.reload_library();
        let (name, uid) = match result {
            Ok(saved) => saved,
            Err(err) => {
                self.error(vec!["Not saved to the library:".into(), err.to_string()]);
                return false;
            }
        };
        if let Some(edit) = self.edit.as_mut() {
            match target {
                SaveTarget::Machine => {
                    edit.settings.machine.name = Some(name.clone());
                    edit.settings.machine.library_uid = Some(uid);
                }
                SaveTarget::Tool(id) => {
                    if let Some(tool) = edit.settings.tools.iter_mut().find(|t| t.id == id) {
                        tool.name = Some(name.clone());
                        tool.library_uid = Some(uid);
                    }
                }
            }
        }
        self.commit();
        self.set_status(format!("Saved \"{name}\" to your library"), STATUS_SECONDS);
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
                "Save machine profile",
                self.library
                    .machines
                    .find(text.trim())
                    .map(|m| m.name.clone()),
            ),
            LibraryKind::Tools => (
                "Save tool to library",
                self.library.tools.find(text.trim()).map(|t| t.name.clone()),
            ),
        };
        let mut done = false;
        let mut save = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Name in your library:");
                ui.add(TextEdit::singleline(&mut text).desired_width(320.0));
                if let Some(existing) = &taken {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!("Replaces \"{existing}\" in your library."),
                    );
                }
                ui.horizontal(|ui| {
                    let label = if taken.is_some() { "Replace" } else { "Save" };
                    if ui
                        .add_enabled(!text.trim().is_empty(), egui::Button::new(label))
                        .clicked()
                    {
                        save = true;
                        done = true;
                    }
                    if ui.button("Cancel").clicked() {
                        done = true;
                    }
                });
            });
        self.prompt = (!done).then(|| Prompt::LibraryName(target, text.clone()));
        if save {
            self.save_to_library(target, &text);
        }
    }

    /// Store a library tool edited in Properties: it replaces the tool with its uid, or is
    /// added. A name another tool has is refused.
    pub fn store_library_tool(&mut self, entry: LibraryTool) -> Result<(), String> {
        let dir = self.settings_dir.clone().ok_or("No settings folder")?;
        let result = update_tools(&tools_path(&dir), |library| {
            if library
                .tools
                .iter()
                .any(|t| t.uid != entry.uid && same_name(&t.name, &entry.name))
            {
                return Err(refused(format!(
                    "There is already a tool \"{}\".",
                    entry.name
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
        result.map_err(|e| e.to_string())
    }

    /// Store a machine profile edited in Properties, as `store_library_tool`.
    pub fn store_machine_profile(&mut self, profile: MachineProfile) -> Result<(), String> {
        let dir = self.settings_dir.clone().ok_or("No settings folder")?;
        let result = update_machines(&machines_path(&dir), |library| {
            if library
                .machines
                .iter()
                .any(|m| m.uid != profile.uid && same_name(&m.name, &profile.name))
            {
                return Err(refused(format!(
                    "There is already a machine \"{}\".",
                    profile.name
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
        result.map_err(|e| e.to_string())
    }

    /// A new library tool or machine, saved at once under a free name ("New tool 2") and
    /// shown in Properties. It starts from the open project's first tool or machine.
    pub fn new_library_entry(&mut self, kind: LibraryKind) -> Option<String> {
        self.reload_library();
        let result = match kind {
            LibraryKind::Tools => {
                let name = unique_name("New tool", |n| self.library.tools.find(n).is_some());
                let uid = self.library.tools.new_uid(&name);
                let entry = match self.edit.as_ref().and_then(|e| e.settings.tools.first()) {
                    Some(tool) => LibraryTool::from_tool(&name, &uid, tool, None),
                    None => LibraryTool {
                        name,
                        uid: uid.clone(),
                        id: 1,
                        kind: ToolType::Endmill,
                        diameter: DEFAULT_TOOL_DIAMETER,
                        flutes: None,
                        center_cutting: None,
                        rpm: Some(DEFAULT_RPM),
                        cutting: None,
                        angle: None,
                        tip_diameter: None,
                    },
                };
                self.store_library_tool(entry).map(|()| uid)
            }
            LibraryKind::Machines => {
                let name = unique_name("New machine", |n| self.library.machines.find(n).is_some());
                let uid = self.library.machines.new_uid(&name);
                let profile = match self.edit.as_ref() {
                    Some(edit) => MachineProfile::from_job(&name, &uid, &edit.settings),
                    None => MachineProfile {
                        name,
                        uid: uid.clone(),
                        post: "grbl".into(),
                        safe_z: DEFAULT_SAFE_Z,
                        spindle: Spindle::Cw,
                        max_feed: None,
                        travel: None,
                        work_zero: WorkZero::DrawingOrigin {},
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
                self.library_state.error = Some(err);
                None
            }
        }
    }

    pub(super) fn remove_from_library(
        &mut self,
        kind: LibraryKind,
        name: &str,
    ) -> Result<(), String> {
        let dir = self.settings_dir.clone().ok_or("No settings folder")?;
        let result = match kind {
            LibraryKind::Tools => update_tools(&tools_path(&dir), |library| {
                library.remove(name).map(|_| ())
            }),
            LibraryKind::Machines => update_machines(&machines_path(&dir), |library| {
                library.remove(name).map(|_| ())
            }),
        };
        self.reload_library();
        result.map_err(|e| e.to_string())
    }

    /// The tool or machine new projects start with.
    pub fn set_library_default(&mut self, kind: LibraryKind, name: &str) -> Result<(), String> {
        let dir = self.settings_dir.clone().ok_or("No settings folder")?;
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
        result.map_err(|e| e.to_string())
    }
}
