// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's library in the Browser and in Properties (docs/specs/library-in-browser.md
//! §4, §5): a second root above the project with the library's tools and machine profiles,
//! and their forms. Edits stay a draft until *Save*, which writes the library file.

use chipload_job::library::same_name;
use eframe::egui::Id;

use super::browser::{Row, folder, title_line, toggle_folder, tool_icon};
use super::library_transfer::import_preview;
use super::user_library::{
    LibraryKind, library_header, library_message, machine_entry_form, machine_summary,
    tool_entry_form, tool_summary,
};
use super::*;
use crate::icons::Icon;

/// A library tool dragged onto the project's tools, by uid.
#[derive(Debug, Clone)]
pub(super) struct DraggedTool(pub String);

/// A uid (16 hex digits) as the number a `Focus` holds.
pub(super) fn uid_key(uid: &str) -> u64 {
    u64::from_str_radix(uid, 16).unwrap_or(0)
}

/// The uid of a `Focus` number.
pub(super) fn key_uid(key: u64) -> String {
    format!("{key:016x}")
}

const ROOT: &str = "browser-library";
const TOOLS: &str = "browser-library-tools";
const MACHINES: &str = "browser-library-machines";

/// What a click in the library asks for, done after it is drawn.
enum Act {
    Focus(Focus),
    /// Copy the library tool with this uid into the project.
    AddTool(String),
    /// Copy the machine profile with this uid into the project.
    UseMachine(String),
    Default(LibraryKind, String),
    Delete(LibraryKind, String),
    New(LibraryKind),
    Import(LibraryKind),
    Export(LibraryKind),
    /// Write the draft of the entry with this uid.
    Save(LibraryKind, String),
    /// Drop the draft of the entry with this uid.
    Revert(LibraryKind, String),
}

fn folder_menu(ui: &mut egui::Ui, kind: LibraryKind, act: &mut Option<Act>) {
    let new = match kind {
        LibraryKind::Tools => "New tool",
        LibraryKind::Machines => "New machine",
    };
    if ui.button(new).clicked() {
        *act = Some(Act::New(kind));
    }
    if ui.button("Import…").clicked() {
        *act = Some(Act::Import(kind));
    }
    if ui.button("Export…").clicked() {
        *act = Some(Act::Export(kind));
    }
}

impl ChiploadApp {
    /// The Library root of the Browser.
    pub(super) fn library_tree(&mut self, ui: &mut egui::Ui) {
        if let Some(kind) = self.library_state.reveal.take() {
            let sub = match kind {
                LibraryKind::Tools => TOOLS,
                LibraryKind::Machines => MACHINES,
            };
            ui.data_mut(|d| {
                d.insert_persisted(Id::new(ROOT).with("folder-open"), true);
                d.insert_persisted(Id::new(sub).with("folder-open"), true);
            });
        }
        let root = Row::new(Id::new(ROOT), 0, "Library")
            .icon(Icon::Library)
            .trailing("this computer")
            .alert(!self.library.problems.is_empty());
        let (open, out) = folder(ui, root, false);
        out.response.on_hover_text(
            "Your tools and machines, kept on this computer for every project. A project keeps \
             copies of what it uses, so it cuts the same in a year and on another computer.",
        );
        if !open {
            return;
        }
        if self.settings_dir.is_none() {
            Row::new(Id::new(ROOT).with("none"), 1, "No settings folder")
                .dim(true)
                .show(ui)
                .response
                .on_hover_text(format!(
                    "Set {} to keep a library",
                    chipload_pipeline::library::CONFIG_DIR_VAR
                ));
            return;
        }
        let mut act = None;
        self.library_tool_rows(ui, &mut act);
        self.library_machine_rows(ui, &mut act);
        if let Some(act) = act {
            self.library_act(act);
        }
    }

    /// A Library folder's row; returns whether it is open.
    fn library_folder(
        &self,
        ui: &mut egui::Ui,
        kind: LibraryKind,
        count: usize,
        act: &mut Option<Act>,
    ) -> bool {
        let (key, label, icon, focus, hover) = match kind {
            LibraryKind::Tools => (
                TOOLS,
                "Tools",
                Icon::Tools,
                Focus::LibraryTools,
                "Your tool library: drag a tool onto the project's Tools to use it",
            ),
            LibraryKind::Machines => (
                MACHINES,
                "Machines",
                Icon::Machine,
                Focus::LibraryMachines,
                "Your machine profiles: the values new projects start with",
            ),
        };
        let id = Id::new(key);
        let open_id = id.with("folder-open");
        let open = super::browser::folder_open(ui, open_id, true);
        let out = Row::new(id, 1, label)
            .folder(open)
            .icon(icon)
            .trailing(count.to_string())
            .selected(self.focus == focus)
            .show(ui);
        if out.toggled {
            toggle_folder(ui, open_id, true);
        }
        let response = out.response.on_hover_text(hover);
        if response.clicked() {
            *act = Some(Act::Focus(focus));
        }
        response.context_menu(|ui| folder_menu(ui, kind, act));
        open
    }

    fn library_tool_rows(&mut self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let tools = self.library.tools.tools.clone();
        if !self.library_folder(ui, LibraryKind::Tools, tools.len(), act) {
            return;
        }
        let default = self.library.tools.default_tool.clone();
        let has_job = self.edit.is_some();
        for tool in &tools {
            let is_default = default.as_deref().is_some_and(|d| same_name(d, &tool.name));
            let draft = self.library_state.tool_drafts.contains_key(&tool.uid);
            let row = Row::new(Id::new(("browser-library-tool", &tool.uid)), 2, &tool.name)
                .icon(tool_icon(tool.kind))
                .trailing(leaf_note(&tool_summary(tool), is_default, draft))
                .selected(self.focus == Focus::LibraryTool(uid_key(&tool.uid)))
                .draggable()
                .show(ui);
            row.response
                .dnd_set_drag_payload(DraggedTool(tool.uid.clone()));
            let response = row.response.on_hover_text(
                "Click to edit it; drag it onto the project's Tools to use it there",
            );
            if response.clicked() {
                *act = Some(Act::Focus(Focus::LibraryTool(uid_key(&tool.uid))));
            }
            response.context_menu(|ui| {
                if ui
                    .add_enabled(has_job, egui::Button::new("Add to project"))
                    .clicked()
                {
                    *act = Some(Act::AddTool(tool.uid.clone()));
                }
                if ui
                    .add_enabled(!is_default, egui::Button::new("Make default"))
                    .clicked()
                {
                    *act = Some(Act::Default(LibraryKind::Tools, tool.name.clone()));
                }
                if ui.button("Delete").clicked() {
                    *act = Some(Act::Delete(LibraryKind::Tools, tool.name.clone()));
                }
            });
        }
    }

    fn library_machine_rows(&mut self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let machines = self.library.machines.machines.clone();
        if !self.library_folder(ui, LibraryKind::Machines, machines.len(), act) {
            return;
        }
        let default = self.library.machines.default_machine.clone();
        let has_job = self.edit.is_some();
        for machine in &machines {
            let is_default = default
                .as_deref()
                .is_some_and(|d| same_name(d, &machine.name));
            let draft = self.library_state.machine_drafts.contains_key(&machine.uid);
            let row = Row::new(
                Id::new(("browser-library-machine", &machine.uid)),
                2,
                &machine.name,
            )
            .icon(Icon::Machine)
            .trailing(leaf_note(&machine_summary(machine), is_default, draft))
            .selected(self.focus == Focus::LibraryMachine(uid_key(&machine.uid)))
            .show(ui);
            if row.response.clicked() {
                *act = Some(Act::Focus(Focus::LibraryMachine(uid_key(&machine.uid))));
            }
            row.response.context_menu(|ui| {
                if ui
                    .add_enabled(has_job, egui::Button::new("Use in this project"))
                    .clicked()
                {
                    *act = Some(Act::UseMachine(machine.uid.clone()));
                }
                if ui
                    .add_enabled(!is_default, egui::Button::new("Make default"))
                    .clicked()
                {
                    *act = Some(Act::Default(LibraryKind::Machines, machine.name.clone()));
                }
                if ui.button("Delete").clicked() {
                    *act = Some(Act::Delete(LibraryKind::Machines, machine.name.clone()));
                }
            });
        }
    }

    /// Properties for a library focus.
    pub(super) fn library_properties(&mut self, ui: &mut egui::Ui) {
        let mut act = None;
        match self.focus {
            Focus::LibraryTools => self.library_folder_form(ui, LibraryKind::Tools, &mut act),
            Focus::LibraryMachines => {
                self.library_folder_form(ui, LibraryKind::Machines, &mut act);
            }
            Focus::LibraryTool(key) => self.library_tool_form(ui, &key_uid(key), &mut act),
            Focus::LibraryMachine(key) => {
                self.library_machine_form(ui, &key_uid(key), &mut act);
            }
            _ => {}
        }
        if let Some(error) = &self.library_state.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if let Some(act) = act {
            self.library_act(act);
        }
    }

    fn library_folder_form(&mut self, ui: &mut egui::Ui, kind: LibraryKind, act: &mut Option<Act>) {
        let dir = self.settings_dir.clone();
        let (title, icon, path, count, about) = match kind {
            LibraryKind::Tools => (
                "Tool library",
                Icon::Tools,
                dir.as_deref().map(chipload_pipeline::library::tools_path),
                self.library.tools.tools.len(),
                "A project keeps a copy of each tool it uses. Click a tool under Library to edit \
                 it; drag it onto the project's Tools, or right-click it, to use it.",
            ),
            LibraryKind::Machines => (
                "Machines",
                Icon::Machine,
                dir.as_deref()
                    .map(chipload_pipeline::library::machines_path),
                self.library.machines.machines.len(),
                "A project keeps a copy of its machine's values. Right-click a profile to use it \
                 in this project; the default is what new projects start with.",
            ),
        };
        title_line(ui, icon, None, title);
        library_header(ui, path.as_deref(), &self.library.problems);
        ui.label(format!(
            "{count} {} in your library.",
            match (kind, count) {
                (LibraryKind::Tools, 1) => "tool",
                (LibraryKind::Tools, _) => "tools",
                (LibraryKind::Machines, 1) => "machine",
                (LibraryKind::Machines, _) => "machines",
            }
        ));
        ui.weak(about);
        ui.add_space(4.0);
        ui.add_enabled_ui(path.is_some(), |ui| {
            ui.horizontal_wrapped(|ui| folder_menu(ui, kind, act));
        });
        let choice = import_preview(ui, self.library_state.import.as_ref(), kind);
        library_message(ui, self.library_state.message.as_ref(), kind);
        if let Some(choice) = choice {
            self.finish_library_import(choice);
        }
    }

    fn library_tool_form(&mut self, ui: &mut egui::Ui, uid: &str, act: &mut Option<Act>) {
        let Some(stored) = self.library.tools.find_uid(uid).cloned() else {
            title_line(ui, Icon::Tools, None, "Library tool");
            ui.weak("This tool is no longer in your library.");
            return;
        };
        let mut draft = self
            .library_state
            .tool_drafts
            .get(uid)
            .cloned()
            .unwrap_or_else(|| stored.clone());
        title_line(ui, tool_icon(stored.kind), None, &stored.name);
        ui.weak("In your library, for every project. Changes are kept when you press Save.");
        ui.add_space(4.0);
        tool_entry_form(ui, &mut draft);
        let changed = draft != stored;
        if changed {
            self.library_state
                .tool_drafts
                .insert(uid.to_string(), draft);
        } else {
            self.library_state.tool_drafts.remove(uid);
        }
        save_buttons(ui, LibraryKind::Tools, uid, changed, act);
        let is_default = self
            .library
            .tools
            .default_tool
            .as_deref()
            .is_some_and(|d| same_name(d, &stored.name));
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.edit.is_some() && !changed,
                    egui::Button::new("Add to project"),
                )
                .on_hover_text("Copy it into the project as a tool of its own")
                .on_disabled_hover_text("Open a project, and save or revert the changes first")
                .clicked()
            {
                *act = Some(Act::AddTool(uid.to_string()));
            }
            entry_buttons(ui, LibraryKind::Tools, &stored.name, is_default, act);
        });
        if let Some(edit) = &self.edit {
            let users: Vec<String> = edit
                .settings
                .tools
                .iter()
                .filter(|t| t.library_uid.as_deref() == Some(uid))
                .map(|t| format!("T{}", t.id))
                .collect();
            if !users.is_empty() {
                ui.weak(format!("This project has a copy: {}", users.join(", ")));
            }
        }
    }

    fn library_machine_form(&mut self, ui: &mut egui::Ui, uid: &str, act: &mut Option<Act>) {
        let Some(stored) = self.library.machines.find_uid(uid).cloned() else {
            title_line(ui, Icon::Machine, None, "Machine profile");
            ui.weak("This profile is no longer in your library.");
            return;
        };
        let mut draft = self
            .library_state
            .machine_drafts
            .get(uid)
            .cloned()
            .unwrap_or_else(|| stored.clone());
        title_line(ui, Icon::Machine, None, &stored.name);
        ui.weak("In your library, for every project. Changes are kept when you press Save.");
        ui.add_space(4.0);
        machine_entry_form(ui, &mut draft, &self.library);
        let changed = draft != stored;
        if changed {
            self.library_state
                .machine_drafts
                .insert(uid.to_string(), draft);
        } else {
            self.library_state.machine_drafts.remove(uid);
        }
        save_buttons(ui, LibraryKind::Machines, uid, changed, act);
        let is_default = self
            .library
            .machines
            .default_machine
            .as_deref()
            .is_some_and(|d| same_name(d, &stored.name));
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.edit.is_some() && !changed,
                    egui::Button::new("Use in this project"),
                )
                .on_hover_text("Copy its values into the project's machine (undo with Ctrl+Z)")
                .on_disabled_hover_text("Open a project, and save or revert the changes first")
                .clicked()
            {
                *act = Some(Act::UseMachine(uid.to_string()));
            }
            entry_buttons(ui, LibraryKind::Machines, &stored.name, is_default, act);
        });
        if let Some(edit) = &self.edit
            && edit.settings.machine.library_uid.as_deref() == Some(uid)
        {
            ui.weak("This project's machine came from it.");
        }
    }

    fn library_act(&mut self, act: Act) {
        self.library_state.error = None;
        match act {
            Act::Focus(focus) => {
                self.reload_library();
                self.focus = focus;
            }
            Act::AddTool(uid) => {
                self.add_library_tool_uid(&uid);
            }
            Act::UseMachine(uid) => {
                if let Some(name) = self.library.machines.find_uid(&uid).map(|m| m.name.clone()) {
                    self.use_machine_profile(&name);
                }
            }
            Act::Default(kind, name) => {
                if let Err(err) = self.set_library_default(kind, &name) {
                    self.library_state.error = Some(err);
                }
            }
            Act::Delete(kind, name) => match self.remove_from_library(kind, &name) {
                Ok(()) => {
                    self.focus = match kind {
                        LibraryKind::Tools => Focus::LibraryTools,
                        LibraryKind::Machines => Focus::LibraryMachines,
                    };
                    self.set_status(
                        format!("Deleted \"{name}\" from your library; projects keep their copies"),
                        STATUS_SECONDS,
                    );
                }
                Err(err) => self.library_state.error = Some(err),
            },
            Act::New(kind) => {
                self.new_library_entry(kind);
            }
            Act::Import(kind) => {
                self.focus = match kind {
                    LibraryKind::Tools => Focus::LibraryTools,
                    LibraryKind::Machines => Focus::LibraryMachines,
                };
                self.import_library_dialog(kind);
            }
            Act::Export(kind) => {
                self.focus = match kind {
                    LibraryKind::Tools => Focus::LibraryTools,
                    LibraryKind::Machines => Focus::LibraryMachines,
                };
                self.export_library_dialog(kind);
            }
            Act::Save(kind, uid) => {
                self.save_library_draft(kind, &uid);
            }
            Act::Revert(LibraryKind::Tools, uid) => {
                self.library_state.tool_drafts.remove(&uid);
            }
            Act::Revert(LibraryKind::Machines, uid) => {
                self.library_state.machine_drafts.remove(&uid);
            }
        }
    }

    /// Write the draft of library entry `uid` into the library file.
    pub fn save_library_draft(&mut self, kind: LibraryKind, uid: &str) -> bool {
        let result = match kind {
            LibraryKind::Tools => {
                let Some(mut entry) = self.library_state.tool_drafts.get(uid).cloned() else {
                    return false;
                };
                entry.name = entry.name.trim().to_string();
                self.store_library_tool(entry)
            }
            LibraryKind::Machines => {
                let Some(mut profile) = self.library_state.machine_drafts.get(uid).cloned() else {
                    return false;
                };
                profile.name = profile.name.trim().to_string();
                self.store_machine_profile(profile)
            }
        };
        match result {
            Ok(()) => {
                match kind {
                    LibraryKind::Tools => {
                        self.library_state.tool_drafts.remove(uid);
                    }
                    LibraryKind::Machines => {
                        self.library_state.machine_drafts.remove(uid);
                    }
                }
                self.set_status("Saved to your library".into(), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.library_state.error = Some(err);
                false
            }
        }
    }

    /// Copy the library tool `uid` into the project (*Add to project*, or a drop on the
    /// project's Tools); returns its id there.
    pub fn add_library_tool_uid(&mut self, uid: &str) -> Option<i64> {
        self.reload_library();
        let name = self.library.tools.find_uid(uid)?.name.clone();
        let id = self.add_library_tool(&name)?;
        self.focus = Focus::Tool(id);
        Some(id)
    }
}

/// A leaf's right end: its summary, *default*, and *unsaved* while it has a draft.
fn leaf_note(summary: &str, is_default: bool, draft: bool) -> String {
    let mut parts = vec![summary.to_string()];
    if is_default {
        parts.push("default".into());
    }
    if draft {
        parts.push("unsaved".into());
    }
    parts.join(" · ")
}

fn save_buttons(
    ui: &mut egui::Ui,
    kind: LibraryKind,
    uid: &str,
    changed: bool,
    act: &mut Option<Act>,
) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(changed, egui::Button::new("Save"))
            .on_hover_text("Write the changes into your library")
            .clicked()
        {
            *act = Some(Act::Save(kind, uid.to_string()));
        }
        if ui
            .add_enabled(changed, egui::Button::new("Revert"))
            .on_hover_text("Drop the changes")
            .clicked()
        {
            *act = Some(Act::Revert(kind, uid.to_string()));
        }
        if changed {
            ui.colored_label(ui.visuals().warn_fg_color, "not saved");
        }
    });
}

fn entry_buttons(
    ui: &mut egui::Ui,
    kind: LibraryKind,
    name: &str,
    is_default: bool,
    act: &mut Option<Act>,
) {
    if ui
        .add_enabled(!is_default, egui::Button::new("Make default"))
        .on_hover_text("New projects start with it")
        .on_disabled_hover_text("New projects start with it")
        .clicked()
    {
        *act = Some(Act::Default(kind, name.to_string()));
    }
    if ui
        .button("Delete")
        .on_hover_text("Take it out of your library; projects keep their copies")
        .clicked()
    {
        *act = Some(Act::Delete(kind, name.to_string()));
    }
}
