// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's tool library and machine profiles (docs/specs/tool-library-and-machines.md
//! §8): the library rows of the job form, the Settings windows, and copying values into and
//! out of the project. The rules are in `chipload_job::library`; this only shows and edits
//! values.

use chipload_job::Tool;
use chipload_job::library::{AppSettings, load_settings, update_settings};
use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, MachineProfile, same_name, update_machines, update_tools,
};
use chipload_pipeline::library::{Library, machines_path, tools_path};
use chipload_pipeline::{DEFAULT_MAX_OVERCUT, DEFAULT_RPM, DEFAULT_SAFE_Z, DEFAULT_TOOL_DIAMETER};

use super::forms::{feed, machine_rows, mm, tool_rows, work_zero_rows};
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryKind {
    Tools,
    Machines,
}

/// What the job form asks of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRequest {
    UseMachine(String),
    UseTool(String),
    /// Ask for a name, then save the project's machine or tool into the library.
    Save(LibraryKind),
}

/// The library rows of the job form: the entries to choose from, and where the project's
/// values came from.
pub(super) struct LibraryRows {
    kind: LibraryKind,
    /// None: no settings folder, so no library.
    names: Option<Vec<String>>,
    /// "from profile …" or "differs from …", and the entry to update from.
    note: Option<(String, Option<String>)>,
}

/// "from profile \"X\"", "differs from profile \"X\": …" (with X to update from), or
/// "\"X\" is not in your library".
fn origin_note(
    what: &str,
    name: Option<&str>,
    changes: Option<Vec<String>>,
) -> Option<(String, Option<String>)> {
    let name = name?;
    Some(match changes {
        None => (format!("from {what} \"{name}\", not in your library"), None),
        Some(changes) if changes.is_empty() => (format!("from {what} \"{name}\""), None),
        Some(changes) => (
            format!("differs from {what} \"{name}\": {}", changes.join(", ")),
            Some(name.to_string()),
        ),
    })
}

impl LibraryRows {
    pub(super) fn machine(job: &Job, library: Option<&Library>) -> Self {
        let names = library.map(|l| l.machines.machines.iter().map(|m| m.name.clone()).collect());
        let note = library.and_then(|l| {
            let name = job.machine.name.as_deref();
            let changes = name
                .and_then(|n| l.machines.find(n))
                .map(|profile| profile.changes(job));
            origin_note("profile", name, changes)
        });
        LibraryRows {
            kind: LibraryKind::Machines,
            names,
            note,
        }
    }

    pub(super) fn tool(tool: &Tool, library: Option<&Library>) -> Self {
        let names = library.map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect());
        let note = library.and_then(|l| {
            let name = tool.name.as_deref();
            let changes = name
                .and_then(|n| l.tools.find(n))
                .map(|entry| entry.changes(tool));
            origin_note("library tool", name, changes)
        });
        LibraryRows {
            kind: LibraryKind::Tools,
            names,
            note,
        }
    }
}

#[cfg(test)]
impl LibraryRows {
    pub(super) fn note_text(&self) -> Option<&str> {
        self.note.as_ref().map(|(text, _)| text.as_str())
    }
}

pub(super) fn library_rows(
    ui: &mut egui::Ui,
    request: &mut Option<LibraryRequest>,
    rows: LibraryRows,
) {
    let (choose, save, use_entry): (&str, &str, fn(String) -> LibraryRequest) = match rows.kind {
        LibraryKind::Machines => (
            "Use profile…",
            "Save as profile…",
            LibraryRequest::UseMachine,
        ),
        LibraryKind::Tools => (
            "Use library tool…",
            "Save to library…",
            LibraryRequest::UseTool,
        ),
    };
    ui.label("Library");
    ui.horizontal(|ui| {
        let available = rows.names.is_some();
        let names = rows.names.unwrap_or_default();
        ui.add_enabled_ui(available && !names.is_empty(), |ui| {
            ComboBox::from_id_salt(("library", choose))
                .selected_text(choose)
                .show_ui(ui, |ui| {
                    for name in names {
                        if ui.selectable_label(false, &name).clicked() {
                            *request = Some(use_entry(name));
                        }
                    }
                })
        });
        let button = ui.add_enabled(available, egui::Button::new(save));
        let button = if available {
            button.on_hover_text("Copies these values into your library, for other projects")
        } else {
            button.on_disabled_hover_text(format!(
                "No settings folder found; set {}",
                chipload_pipeline::library::CONFIG_DIR_VAR
            ))
        };
        if button.clicked() {
            *request = Some(LibraryRequest::Save(rows.kind));
        }
    });
    ui.end_row();
    if let Some((text, update)) = rows.note {
        ui.label("");
        ui.horizontal_wrapped(|ui| {
            ui.weak(text);
            if let Some(name) = update
                && ui
                    .button("Update from library")
                    .on_hover_text("Copy the library's values into the project (undo with Ctrl+Z)")
                    .clicked()
            {
                *request = Some(use_entry(name));
            }
        });
        ui.end_row();
    }
}

/// An entry being edited in a Settings window, and the name it has in the library (None
/// for a new one).
#[derive(Debug, Clone)]
struct Draft<T> {
    original: Option<String>,
    value: T,
}

/// The Settings windows.
#[derive(Debug, Clone, Default)]
pub(super) struct LibraryWindows {
    pub(super) tools_open: bool,
    pub(super) machines_open: bool,
    tool: Option<Draft<LibraryTool>>,
    machine: Option<Draft<MachineProfile>>,
    error: Option<String>,
}

/// What was clicked in a Settings window; applied after drawing it.
enum WindowAction {
    Edit(String),
    New,
    Save,
    Delete,
    MakeDefault,
    Cancel,
}

/// "New tool", or "New tool 2" … if that is taken.
fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while taken(&name) {
        name = format!("{base} {n}");
        n += 1;
    }
    name
}

fn refused(text: String) -> LibraryError {
    LibraryError::Refused(text)
}

/// Where the library is kept, and why it couldn't be read.
fn library_header(ui: &mut egui::Ui, path: Option<&Path>, problems: &[String]) {
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

fn tool_entry_form(ui: &mut egui::Ui, entry: &mut LibraryTool) {
    Grid::new("library-tool").num_columns(2).show(ui, |ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut entry.name);
        ui.end_row();
        ui.label("Tool number")
            .on_hover_text("The id it gets in a project where that id is free");
        ui.add(DragValue::new(&mut entry.id).range(1..=9999));
        ui.end_row();
        let mut tool = entry.tool(entry.id);
        tool_rows(ui, "library", &mut tool);
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

fn machine_entry_form(ui: &mut egui::Ui, profile: &mut MachineProfile) {
    Grid::new("library-machine").num_columns(2).show(ui, |ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut profile.name);
        ui.end_row();
        let mut machine = profile.machine();
        machine_rows(ui, "library", &mut machine);
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

/// Draw one Settings window's list and draft; returns what was clicked.
fn window_body<T>(
    ui: &mut egui::Ui,
    names: &[(String, String)],
    default: Option<&str>,
    draft: &mut Option<Draft<T>>,
    error: Option<&str>,
    new_label: &str,
    form: impl FnOnce(&mut egui::Ui, &mut T),
) -> Option<WindowAction> {
    let mut action = None;
    if names.is_empty() {
        ui.weak("Nothing here yet.");
    }
    Grid::new("library-list")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (name, summary) in names {
                let editing = draft
                    .as_ref()
                    .and_then(|d| d.original.as_deref())
                    .is_some_and(|o| same_name(o, name));
                let is_default = default.is_some_and(|d| same_name(d, name));
                let label = if is_default {
                    format!("{name} (default)")
                } else {
                    name.clone()
                };
                if ui.selectable_label(editing, label).clicked() {
                    action = Some(WindowAction::Edit(name.clone()));
                }
                ui.weak(summary);
                ui.end_row();
            }
        });
    if ui.button(new_label).clicked() {
        action = Some(WindowAction::New);
    }
    if let Some(d) = draft {
        ui.separator();
        form(ui, &mut d.value);
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                action = Some(WindowAction::Save);
            }
            let stored = d.original.is_some();
            if ui
                .add_enabled(stored, egui::Button::new("Delete"))
                .clicked()
            {
                action = Some(WindowAction::Delete);
            }
            if ui
                .add_enabled(stored, egui::Button::new("Make default"))
                .on_hover_text("New projects start with it")
                .clicked()
            {
                action = Some(WindowAction::MakeDefault);
            }
            if ui.button("Close").clicked() {
                action = Some(WindowAction::Cancel);
            }
        });
    }
    if let Some(error) = error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    action
}

fn tool_summary(t: &LibraryTool) -> String {
    let kind = match t.kind {
        ToolType::Endmill => "endmill",
        ToolType::Drill => "drill",
        ToolType::Vbit => "v-bit",
    };
    format!("{kind} {} mm", t.diameter)
}

fn machine_summary(m: &MachineProfile) -> String {
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
    }

    /// View > Theme: used at once and kept in the settings for next time.
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

    fn change_settings(&mut self, change: impl FnOnce(&mut AppSettings)) {
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

    /// The project tool's cutting values from the library: only when the tool came from
    /// the library and still has its values.
    pub(super) fn tool_cutting(&self) -> Option<Cutting> {
        let tool = self.edit.as_ref()?.settings.tools.first()?;
        let entry = self.library()?.tools.find(tool.name.as_deref()?)?;
        if entry.same_values(tool) {
            entry.cutting
        } else {
            None
        }
    }

    pub(super) fn library_request(&mut self, request: LibraryRequest) {
        match request {
            LibraryRequest::UseMachine(name) => {
                self.use_machine_profile(&name);
            }
            LibraryRequest::UseTool(name) => {
                self.use_library_tool(&name);
            }
            LibraryRequest::Save(kind) => {
                let current = self.edit.as_ref().and_then(|e| match kind {
                    LibraryKind::Machines => e.settings.machine.name.clone(),
                    LibraryKind::Tools => e.settings.tools.first().and_then(|t| t.name.clone()),
                });
                self.prompt = Some(Prompt::LibraryName(kind, current.unwrap_or_default()));
            }
        }
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
        let changes = profile.apply(&mut edit.settings);
        self.commit();
        self.report_changes(&format!("Machine profile \"{}\"", profile.name), changes);
        true
    }

    /// Give the project's tool a library tool's values. The project has one tool, so it is
    /// replaced and keeps its id: operations still use it (one undo step).
    pub fn use_library_tool(&mut self, name: &str) -> bool {
        self.reload_library();
        let Some(entry) = self.library.tools.find(name).cloned() else {
            self.error(vec![format!("There is no library tool \"{name}\".")]);
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let tools = &mut edit.settings.tools;
        let changes = match tools.first_mut() {
            Some(tool) => {
                let changes = entry.changes(tool);
                *tool = entry.tool(tool.id);
                changes
            }
            None => {
                tools.push(entry.tool(entry.id.max(1)));
                vec![format!("added as tool {}", entry.id.max(1))]
            }
        };
        self.commit();
        self.report_changes(&format!("Library tool \"{}\"", entry.name), changes);
        true
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

    /// Save the project's machine or tool into the library under `name`, replacing an
    /// entry of that name. The project records the name.
    pub fn save_to_library(&mut self, kind: LibraryKind, name: &str) -> bool {
        let name = name.trim().to_string();
        let (Some(dir), Some(edit)) = (self.settings_dir.clone(), self.edit.as_ref()) else {
            return false;
        };
        let result = match kind {
            LibraryKind::Machines => {
                let profile = MachineProfile::from_job(&name, &edit.settings);
                update_machines(&machines_path(&dir), |library| {
                    match library.position(&name) {
                        Some(i) => library.machines[i] = profile,
                        None => library.machines.push(profile),
                    }
                    Ok(())
                })
            }
            LibraryKind::Tools => match edit.settings.tools.first() {
                Some(tool) => {
                    let entry = LibraryTool::from_tool(&name, tool, None);
                    update_tools(&tools_path(&dir), |library| {
                        match library.position(&name) {
                            // A project tool has no cutting values: keep the library's.
                            Some(i) => {
                                let cutting = library.tools[i].cutting;
                                library.tools[i] = LibraryTool { cutting, ..entry };
                            }
                            None => library.tools.push(entry),
                        }
                        Ok(())
                    })
                }
                None => Err(refused("The project has no tool to save.".into())),
            },
        };
        self.reload_library();
        if let Err(err) = result {
            self.error(vec!["Not saved to the library:".into(), err.to_string()]);
            return false;
        }
        if let Some(edit) = self.edit.as_mut() {
            match kind {
                LibraryKind::Machines => edit.settings.machine.name = Some(name.clone()),
                LibraryKind::Tools => {
                    if let Some(tool) = edit.settings.tools.first_mut() {
                        tool.name = Some(name.clone());
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
        kind: LibraryKind,
        mut text: String,
    ) {
        let (title, taken) = match kind {
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
        self.prompt = (!done).then(|| Prompt::LibraryName(kind, text.clone()));
        if save {
            self.save_to_library(kind, &text);
        }
    }

    /// Store a tool edited in the Settings window: replace `original`, or add a new one.
    pub fn store_library_tool(
        &mut self,
        original: Option<&str>,
        entry: LibraryTool,
    ) -> Result<(), String> {
        let dir = self.settings_dir.clone().ok_or("No settings folder")?;
        let result = update_tools(&tools_path(&dir), |library| {
            let clash = library.position(&entry.name);
            match original.and_then(|o| library.position(o)) {
                Some(i) => {
                    if clash.is_some_and(|c| c != i) {
                        return Err(refused(format!(
                            "There is already a tool \"{}\".",
                            entry.name
                        )));
                    }
                    if library
                        .default_tool
                        .as_deref()
                        .is_some_and(|d| same_name(d, &library.tools[i].name))
                    {
                        library.default_tool = Some(entry.name.clone());
                    }
                    library.tools[i] = entry;
                }
                None => {
                    if clash.is_some() {
                        return Err(refused(format!(
                            "There is already a tool \"{}\".",
                            entry.name
                        )));
                    }
                    library.tools.push(entry);
                }
            }
            Ok(())
        });
        self.reload_library();
        result.map_err(|e| e.to_string())
    }

    /// Store a profile edited in the Settings window: replace `original`, or add a new one.
    pub fn store_machine_profile(
        &mut self,
        original: Option<&str>,
        profile: MachineProfile,
    ) -> Result<(), String> {
        let dir = self.settings_dir.clone().ok_or("No settings folder")?;
        let result = update_machines(&machines_path(&dir), |library| {
            let clash = library.position(&profile.name);
            match original.and_then(|o| library.position(o)) {
                Some(i) => {
                    if clash.is_some_and(|c| c != i) {
                        return Err(refused(format!(
                            "There is already a machine \"{}\".",
                            profile.name
                        )));
                    }
                    if library
                        .default_machine
                        .as_deref()
                        .is_some_and(|d| same_name(d, &library.machines[i].name))
                    {
                        library.default_machine = Some(profile.name.clone());
                    }
                    library.machines[i] = profile;
                }
                None => {
                    if clash.is_some() {
                        return Err(refused(format!(
                            "There is already a machine \"{}\".",
                            profile.name
                        )));
                    }
                    library.machines.push(profile);
                }
            }
            Ok(())
        });
        self.reload_library();
        result.map_err(|e| e.to_string())
    }

    fn remove_from_library(&mut self, kind: LibraryKind, name: &str) -> Result<(), String> {
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

    /// Open a Settings window (reading the library again first).
    pub fn open_library_window(&mut self, kind: LibraryKind) {
        self.reload_library();
        match kind {
            LibraryKind::Tools => self.library_windows.tools_open = true,
            LibraryKind::Machines => self.library_windows.machines_open = true,
        }
    }

    pub(super) fn library_windows_ui(&mut self, ctx: &egui::Context) {
        if self.library_windows.tools_open {
            self.tools_window(ctx);
        }
        if self.library_windows.machines_open {
            self.machines_window(ctx);
        }
    }

    fn tools_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let path = self.settings_dir.as_deref().map(tools_path);
        let names: Vec<(String, String)> = self
            .library
            .tools
            .tools
            .iter()
            .map(|t| (t.name.clone(), tool_summary(t)))
            .collect();
        let default = self.library.tools.default_tool.clone();
        let problems = self.library.problems.clone();
        let windows = &mut self.library_windows;
        let mut action = None;
        egui::Window::new("Tool library")
            .open(&mut open)
            .default_width(440.0)
            .show(ctx, |ui| {
                library_header(ui, path.as_deref(), &problems);
                ui.add_enabled_ui(path.is_some(), |ui| {
                    action = window_body(
                        ui,
                        &names,
                        default.as_deref(),
                        &mut windows.tool,
                        windows.error.as_deref(),
                        "New tool",
                        tool_entry_form,
                    );
                });
            });
        self.library_windows.tools_open = open;
        if let Some(action) = action {
            self.tool_window_action(action);
        }
    }

    fn tool_window_action(&mut self, action: WindowAction) {
        let windows = &mut self.library_windows;
        windows.error = None;
        match action {
            WindowAction::Edit(name) => {
                windows.tool = self.library.tools.find(&name).map(|t| Draft {
                    original: Some(t.name.clone()),
                    value: t.clone(),
                });
            }
            WindowAction::New => {
                let name = unique_name("New tool", |n| self.library.tools.find(n).is_some());
                // Start from the project's tool, if one is open.
                let value = match self.edit.as_ref().and_then(|e| e.settings.tools.first()) {
                    Some(tool) => LibraryTool::from_tool(&name, tool, None),
                    None => LibraryTool {
                        name,
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
                windows.tool = Some(Draft {
                    original: None,
                    value,
                });
            }
            WindowAction::Save => {
                if let Some(draft) = windows.tool.clone() {
                    let mut value = draft.value;
                    value.name = value.name.trim().to_string();
                    match self.store_library_tool(draft.original.as_deref(), value.clone()) {
                        Ok(()) => {
                            self.library_windows.tool = Some(Draft {
                                original: Some(value.name.clone()),
                                value,
                            });
                        }
                        Err(err) => self.library_windows.error = Some(err),
                    }
                }
            }
            WindowAction::Delete => {
                if let Some(name) = windows.tool.as_ref().and_then(|d| d.original.clone()) {
                    match self.remove_from_library(LibraryKind::Tools, &name) {
                        Ok(()) => self.library_windows.tool = None,
                        Err(err) => self.library_windows.error = Some(err),
                    }
                }
            }
            WindowAction::MakeDefault => {
                if let Some(name) = windows.tool.as_ref().and_then(|d| d.original.clone())
                    && let Err(err) = self.set_library_default(LibraryKind::Tools, &name)
                {
                    self.library_windows.error = Some(err);
                }
            }
            WindowAction::Cancel => windows.tool = None,
        }
    }

    fn machines_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let path = self.settings_dir.as_deref().map(machines_path);
        let names: Vec<(String, String)> = self
            .library
            .machines
            .machines
            .iter()
            .map(|m| (m.name.clone(), machine_summary(m)))
            .collect();
        let default = self.library.machines.default_machine.clone();
        let problems = self.library.problems.clone();
        let windows = &mut self.library_windows;
        let mut action = None;
        egui::Window::new("Machines")
            .open(&mut open)
            .default_width(440.0)
            .show(ctx, |ui| {
                library_header(ui, path.as_deref(), &problems);
                ui.add_enabled_ui(path.is_some(), |ui| {
                    action = window_body(
                        ui,
                        &names,
                        default.as_deref(),
                        &mut windows.machine,
                        windows.error.as_deref(),
                        "New machine",
                        machine_entry_form,
                    );
                });
            });
        self.library_windows.machines_open = open;
        if let Some(action) = action {
            self.machine_window_action(action);
        }
    }

    fn machine_window_action(&mut self, action: WindowAction) {
        let windows = &mut self.library_windows;
        windows.error = None;
        match action {
            WindowAction::Edit(name) => {
                windows.machine = self.library.machines.find(&name).map(|m| Draft {
                    original: Some(m.name.clone()),
                    value: m.clone(),
                });
            }
            WindowAction::New => {
                let name = unique_name("New machine", |n| self.library.machines.find(n).is_some());
                // Start from the open project's machine, if any.
                let value = match self.edit.as_ref() {
                    Some(edit) => MachineProfile::from_job(&name, &edit.settings),
                    None => MachineProfile {
                        name,
                        post: "grbl".into(),
                        safe_z: DEFAULT_SAFE_Z,
                        spindle: Spindle::Cw,
                        max_feed: None,
                        travel: None,
                        work_zero: WorkZero::DrawingOrigin {},
                        max_overcut: DEFAULT_MAX_OVERCUT,
                    },
                };
                windows.machine = Some(Draft {
                    original: None,
                    value,
                });
            }
            WindowAction::Save => {
                if let Some(draft) = windows.machine.clone() {
                    let mut value = draft.value;
                    value.name = value.name.trim().to_string();
                    match self.store_machine_profile(draft.original.as_deref(), value.clone()) {
                        Ok(()) => {
                            self.library_windows.machine = Some(Draft {
                                original: Some(value.name.clone()),
                                value,
                            });
                        }
                        Err(err) => self.library_windows.error = Some(err),
                    }
                }
            }
            WindowAction::Delete => {
                if let Some(name) = windows.machine.as_ref().and_then(|d| d.original.clone()) {
                    match self.remove_from_library(LibraryKind::Machines, &name) {
                        Ok(()) => self.library_windows.machine = None,
                        Err(err) => self.library_windows.error = Some(err),
                    }
                }
            }
            WindowAction::MakeDefault => {
                if let Some(name) = windows.machine.as_ref().and_then(|d| d.original.clone())
                    && let Err(err) = self.set_library_default(LibraryKind::Machines, &name)
                {
                    self.library_windows.error = Some(err);
                }
            }
            WindowAction::Cancel => windows.machine = None,
        }
    }
}
