// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's tool library and machine profiles (docs/specs/tool-library-and-machines.md
//! §8, library-in-browser.md): the library rows of a project tool and the machine, and
//! copying values into and out of the project. The Browser rows and the entries' properties
//! are in library_view.rs. The rules are in `chipload_job::library`; this only shows and
//! edits values.

use std::collections::BTreeMap;

use chipload_job::Tool;
use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, Link, LinkEntry, MachineProfile, machine_link,
};
use chipload_pipeline::library::Library;

use super::library_transfer::PendingImport;
use super::library_view::uid_key;
use super::*;
use crate::i18n::Lang;

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
    pub(super) fn kind(self) -> LibraryKind {
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
    /// The project's tools that differ from their library tools take its values
    /// (docs/specs/tool-updates.md §3).
    UpdateAllTools,
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

    /// A project tool's rows; `link`: how it stands to its library tool, a speed from the
    /// setup's material aside (`project_tool_link`).
    pub(super) fn tool(tool: &Tool, library: Option<&Library>, link: Option<Link>) -> Self {
        let names = library.map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect());
        let link = link.map(|link| {
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

    fn what(&self) -> LinkEntry {
        match self.target {
            SaveTarget::Machine => LinkEntry::Profile,
            SaveTarget::Tool(_) => LinkEntry::Tool,
        }
    }
}

#[cfg(test)]
impl LibraryRows {
    /// The entry the dropdown shows, while it is in the library.
    pub(super) fn entry_name(&self) -> Option<String> {
        self.link
            .as_ref()
            .and_then(|(link, _)| link.entry_name().map(str::to_string))
    }

    pub(super) fn note_text(&self) -> Option<String> {
        let units = chipload_i18n::Units::Mm;
        self.link
            .as_ref()
            .map(|(link, _)| status_text(crate::i18n::Language::En, link, self.what(), units))
    }
}

fn target_salt(target: SaveTarget) -> i64 {
    match target {
        SaveTarget::Machine => -1,
        SaveTarget::Tool(id) => id,
    }
}

/// The library rows of the project's machine or a project tool
/// (docs/specs/linked-machine.md §3, decision 7): the entry in a dropdown (another one is
/// used when picked), one status line with only the buttons that apply, and the rarer
/// choices behind ⋯.
pub(super) fn library_rows(
    ui: &mut egui::Ui,
    request: &mut Option<LibraryRequest>,
    rows: LibraryRows,
) {
    let target = rows.target;
    let what = rows.what();
    let use_entry = |name: String| match target {
        SaveTarget::Machine => LibraryRequest::UseMachine(name),
        SaveTarget::Tool(id) => LibraryRequest::UseTool(id, name),
    };
    let (label, choose) = match target {
        SaveTarget::Machine => (
            t!(ui, "user-library-profile"),
            t!(ui, "user-library-use-profile"),
        ),
        SaveTarget::Tool(_) => (t!(ui, "user-library-tool"), t!(ui, "user-library-use-tool")),
    };
    let linked = rows.link.as_ref().and_then(|(_, uid)| uid.clone());
    let current = match rows.link.as_ref().map(|(link, _)| link) {
        Some(Link::Same(name) | Link::Differs(name, _)) => Some(name.clone()),
        _ => None,
    };
    ui.label(label);
    ui.horizontal(|ui| {
        let names = rows.names.clone().unwrap_or_default();
        let shown = current.clone().unwrap_or(choose);
        let combo = ui.add_enabled_ui(!names.is_empty(), |ui| {
            ComboBox::from_id_salt(("library", target_salt(target)))
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for name in names {
                        let on = current.as_deref() == Some(name.as_str());
                        if ui.selectable_label(on, &name).clicked() && !on {
                            *request = Some(use_entry(name));
                        }
                    }
                })
        });
        if rows.names.is_none() {
            combo.response.on_disabled_hover_text(t!(
                ui,
                "user-library-no-settings-folder-set",
                var = chipload_pipeline::library::CONFIG_DIR_VAR
            ));
            return;
        }
        if linked.is_some() {
            super::header::submenu(ui, "⋯", |ui| {
                if ui
                    .button(t!(ui, "user-library-save-as-new"))
                    .on_hover_text(t!(ui, "user-library-save-as-new-hover"))
                    .clicked()
                {
                    *request = Some(LibraryRequest::Save(target));
                    ui.close();
                }
                if let Some(uid) = &linked
                    && ui
                        .button(t!(ui, "user-library-show-in-library"))
                        .on_hover_text(t!(ui, "user-library-show-in-library-hover"))
                        .clicked()
                {
                    *request = Some(LibraryRequest::Show(target.kind(), uid.clone()));
                    ui.close();
                }
            })
            .response
            .on_hover_text(t!(ui, "user-library-more-hover"));
        }
    });
    ui.end_row();
    let Some((link, _)) = rows.link else {
        return;
    };
    ui.label("");
    ui.vertical(|ui| {
        let units = crate::units::display(ui.ctx()).into();
        let status = status_text(ui.lang(), &link, what, units);
        ui.add(egui::Label::new(egui::RichText::new(status).weak()).wrap());
        if matches!(link, Link::Same(_)) {
            return;
        }
        ui.horizontal_wrapped(|ui| match &link {
            Link::Same(_) => {}
            Link::Differs(name, _) => {
                if ui
                    .button(t!(ui, "user-library-update-from-library"))
                    .on_hover_text(t!(ui, "user-library-update-from-library-hover"))
                    .clicked()
                {
                    *request = Some(match target {
                        SaveTarget::Machine => LibraryRequest::UpdateMachine,
                        SaveTarget::Tool(_) => use_entry(name.clone()),
                    });
                }
                if ui
                    .button(t!(ui, "user-library-save-to-library"))
                    .on_hover_text(match target {
                        SaveTarget::Machine => t!(ui, "user-library-save-to-linked-profile-hover"),
                        SaveTarget::Tool(_) => t!(ui, "user-library-save-to-linked-tool-hover"),
                    })
                    .clicked()
                {
                    *request = Some(LibraryRequest::SaveLinked(target));
                }
            }
            Link::None | Link::Missing(_) => {
                if ui
                    .button(t!(ui, "user-library-save-to-library-named"))
                    .on_hover_text(t!(ui, "user-library-save-to-library-hover"))
                    .clicked()
                {
                    *request = Some(LibraryRequest::Save(target));
                }
            }
        });
    });
    ui.end_row();
}

/// The status line under the dropdown: the same as the library, how it differs, or not
/// from it (the dropdown already names the entry).
pub(super) fn status_text(
    lang: crate::i18n::Language,
    link: &Link,
    what: LinkEntry,
    units: chipload_i18n::Units,
) -> String {
    match link {
        Link::Same(_) => t!(lang, "user-library-same"),
        Link::Differs(_, changes) => {
            let changes: Vec<String> = changes.iter().map(|c| c.render(lang, units)).collect();
            t!(lang, "user-library-differs", changes = changes.join(", "))
        }
        Link::None | Link::Missing(_) => link.message(what).render(lang, units),
    }
}

/// The library's state in the app: entries being edited, and what import and export said.
#[derive(Debug, Clone, Default)]
pub(super) struct LibraryState {
    /// Tools being edited and not saved yet, by uid.
    pub(super) tool_drafts: BTreeMap<String, LibraryTool>,
    /// Machine profiles being edited and not saved yet, by uid.
    pub(super) machine_drafts: BTreeMap<String, MachineProfile>,
    /// Materials being edited and not saved yet, by uid (docs/specs/materials.md §4).
    pub(super) material_drafts: BTreeMap<String, chipload_job::materials::Material>,
    /// Styles' operations being edited and not saved yet, by uid, as their drafts number
    /// the tools (docs/specs/styles.md §9, step 3b).
    pub(super) style_drafts: BTreeMap<String, chipload_job::Operation>,
    pub(super) error: Option<String>,
    /// What the last import or export did, shown in that folder's properties.
    pub(super) message: Option<(super::library_transfer::Transfer, String)>,
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

/// A refusal in the app's own words, already in its language.
pub(super) fn refused(text: String) -> LibraryError {
    LibraryError::Refused(chipload_i18n::Value::Text(text))
}

impl ChiploadApp {
    /// Read the library again (it may have been changed by the CLI or another window).
    pub(super) fn reload_library(&mut self) {
        self.user.library_state.post_facts = None;
        self.user.library = self
            .user
            .settings_dir
            .as_deref()
            .map(Library::load)
            .unwrap_or_default();
        // An old library was just converted (docs/specs/cutting-data.md §8): the report says
        // what moved, once.
        if !self.user.library.converted.is_empty() {
            let lines = self.user.library.converted.clone();
            self.set_report(Report::of_messages(lines));
        }
    }

    /// The library, if there is a settings folder.
    pub(super) fn library(&self) -> Option<&Library> {
        self.user.settings_dir.as_ref().map(|_| &self.user.library)
    }

    /// Project tool `id`'s cutting values for the setup's material, from its library tool
    /// (docs/specs/styles.md §2.3).
    pub(super) fn tool_cutting(&self, id: i64) -> Option<Cutting> {
        let edit = self.edit.as_ref()?;
        chipload_pipeline::new_ops::tool_cutting(&edit.job, self.library().map(|l| &l.tools), id)
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
                    SaveTarget::Machine => e.job.machine.name.clone(),
                    SaveTarget::Tool(id) => e
                        .job
                        .tools
                        .iter()
                        .find(|t| t.id == id)
                        .and_then(|t| t.name.clone()),
                });
                self.dialogs.prompt =
                    Some(Prompt::LibraryName(target, current.unwrap_or_default()));
            }
            LibraryRequest::SaveLinked(target) => {
                self.save_to_linked(target);
            }
            LibraryRequest::Show(kind, uid) => self.show_library(kind, Some(&uid)),
            LibraryRequest::UpdateAllTools => {
                self.update_tools_from_library(None);
            }
        }
    }

    /// Preferences, below the Library in the Browser, opened (docs/specs/menu-bar.md §3).
    pub fn show_preferences(&mut self) {
        self.reload_library();
        self.focus = Focus::Preferences;
    }

    /// Open the Library in the Browser at `kind`, and show the entry `uid` (or the folder)
    /// in Properties (docs/specs/library-in-browser.md §6).
    pub fn show_library(&mut self, kind: LibraryKind, uid: Option<&str>) {
        self.reload_library();
        self.user.library_state.reveal = Some(kind);
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
        let posts = self.user.library.posts.clone();
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        if let Err(err) =
            chipload_pipeline::posts::choose_post(&mut edit.job, name, posts.as_deref())
        {
            let title = t!(self, "user-library-post-named", name = name);
            self.error_report(Report::failed(title, err.message()));
            return false;
        }
        self.commit();
        let status = t!(self, "user-library-post-chosen", name = name);
        self.set_status(status, STATUS_SECONDS);
        true
    }

    /// Copy a machine profile's machine values into the project, keeping its safe Z, work
    /// zero and max overcut (docs/specs/linked-machine.md §2; one undo step).
    pub fn use_machine_profile(&mut self, name: &str) -> bool {
        self.reload_library();
        let Some(profile) = self.user.library.machines.find(name).cloned() else {
            self.error(vec![t!(
                self,
                "user-library-no-machine-profile",
                name = name
            )]);
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut changes = profile.update_machine_values(&mut edit.job);
        changes.extend(chipload_pipeline::library::copy_profile_post(
            &mut edit.job,
            self.user.library.posts.as_deref(),
        ));
        self.commit();
        let what = t!(
            self,
            "user-library-machine-profile-named",
            name = &profile.name
        );
        self.report_changes(&what, changes);
        true
    }

    /// Take the linked profile's machine values again (post, spindle, max feed, tool
    /// changes, travel), keeping the project's safe Z, work zero and max depth. One undo
    /// step.
    pub fn update_machine_from_library(&mut self) -> bool {
        self.reload_library();
        let profile = self.edit.as_ref().and_then(|e| {
            let uid = e.job.machine.library_uid.as_deref()?;
            self.user.library.machines.find_uid(uid).cloned()
        });
        let Some(profile) = profile else {
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut changes = profile.update_machine_values(&mut edit.job);
        changes.extend(chipload_pipeline::library::copy_profile_post(
            &mut edit.job,
            self.user.library.posts.as_deref(),
        ));
        self.commit();
        let what = t!(
            self,
            "user-library-machine-profile-named",
            name = &profile.name
        );
        self.report_changes(&what, changes);
        true
    }

    /// Give project tool `id` a library tool's values; it keeps its id, so the operations
    /// that use it still do (one undo step).
    pub fn use_library_tool(&mut self, id: i64, name: &str) -> bool {
        self.reload_library();
        let Some(entry) = self.user.library.tools.find(name).cloned() else {
            self.error(vec![t!(self, "user-library-no-library-tool", name = name)]);
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let Some(before) = edit.job.tools.iter().find(|t| t.id == id).cloned() else {
            self.error(vec![t!(self, "user-library-no-project-tool", id = id)]);
            return false;
        };
        let changes = entry.changes(&before);
        // Updating from its own library tool keeps a speed from the setup's material
        // (docs/specs/tool-updates.md §4).
        let keep = chipload_pipeline::library::material_speed(
            &edit.job,
            &self.user.library.tools,
            &self.user.library.materials,
            &before,
        )
        .filter(|_| before.library_uid.as_deref() == Some(entry.uid.as_str()));
        let Some(tool) = edit.job.tools.iter_mut().find(|t| t.id == id) else {
            return false;
        };
        *tool = entry.tool(id);
        if let Some(rpm) = keep {
            tool.rpm = Some(rpm);
        }
        edit.job.fit_helixes();
        self.commit();
        let what = t!(self, "user-library-library-tool-named", name = &entry.name);
        self.report_changes(&what, changes);
        true
    }

    /// How project tool `tool` stands to its library tool, a speed from the setup's material
    /// aside (docs/specs/tool-updates.md §4). None without a library.
    pub(super) fn project_tool_link(&self, tool: &Tool) -> Option<Link> {
        let library = self.library()?;
        let job = &self.edit.as_ref()?.job;
        Some(chipload_pipeline::library::project_tool_link(
            job,
            &library.tools,
            &library.materials,
            tool,
        ))
    }

    /// The project's tools (all, or tool `only`) that differ from their library tools take
    /// the library's values, as one undo step; the Report says which
    /// (docs/specs/tool-updates.md §2, §3). Whether any changed.
    pub fn update_tools_from_library(&mut self, only: Option<i64>) -> bool {
        self.commit();
        self.reload_library();
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let (job, updated) = chipload_pipeline::library::update_from_library(
            &edit.job,
            &self.user.library.tools,
            &self.user.library.materials,
            only,
        );
        if updated.is_empty() {
            return false;
        }
        if let Some(edit) = self.edit.as_mut() {
            edit.adopt(job);
        }
        self.commit();
        let mut report = Report::info([t!(self, "user-library-tools-updated")]);
        for (id, changes) in updated {
            let text = self.changes_text(&changes);
            let line = t!(self, "user-library-tool-updated", tool = id, changes = text);
            report.push_target(Tone::Info, line, super::report::Target::Tool(id));
        }
        self.set_report(report);
        true
    }

    /// What differs, in one line: "diameter 6 mm -> 6.35 mm; rpm 18000 -> 16000".
    pub(super) fn changes_text(&self, changes: &[Message]) -> String {
        changes
            .iter()
            .map(|c| self.say(c))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Lines for the Report when a project opens: each project tool that differs from its
    /// library tool, and *Update all from library* (docs/specs/tool-updates.md §3).
    pub(super) fn differing_tool_lines(&self, report: &mut Report) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let mut any = false;
        for tool in &edit.job.tools {
            let Some(Link::Differs(name, changes)) = self.project_tool_link(tool) else {
                continue;
            };
            let line = t!(
                self,
                "user-library-tool-differs",
                tool = tool.id,
                name = name,
                changes = self.changes_text(&changes)
            );
            report.push_target(Tone::Warning, line, super::report::Target::Tool(tool.id));
            any = true;
        }
        if any {
            let line = t!(self, "user-library-update-all");
            report.push_target(Tone::Info, line, super::report::Target::UpdateTools);
        }
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
        let Some(entry) = self
            .edit
            .as_mut()
            .and_then(|e| e.job.operations.get_mut(op))
        else {
            return false;
        };
        entry.set_tool(id);
        if let Some(edit) = &mut self.edit
            && let Some(entry) = edit.job.operations.get_mut(op)
        {
            crate::editor::fit_hole_method(entry, &edit.job.tools);
            entry.fit_helixes(&edit.job.tools);
        }
        self.commit();
        true
    }

    pub(super) fn insert_library_tool(&mut self, name: &str) -> Option<i64> {
        self.reload_library();
        let Some(entry) = self.user.library.tools.find(name).cloned() else {
            self.error(vec![t!(self, "user-library-no-library-tool", name = name)]);
            return None;
        };
        let edit = self.edit.as_mut()?;
        let used = chipload_job::library::use_tool(&mut edit.job, &entry);
        let text = format!(
            "\"{}\": {}",
            entry.name,
            self.say(&used.describe_message(entry.id))
        );
        self.set_status(text, STATUS_SECONDS);
        Some(used.id)
    }

    fn report_changes(&mut self, what: &str, changes: Vec<Message>) {
        let status = if changes.is_empty() {
            t!(self, "user-library-no-values-changed-status", what = what)
        } else {
            t!(
                self,
                "user-library-values-changed-status",
                what = what,
                count = changes.len()
            )
        };
        let mut report = Report::info([t!(self, "user-library-changes-heading", what = what)]);
        if changes.is_empty() {
            report.push(
                Tone::Info,
                format!("  {}", t!(self, "user-library-no-values-changed")),
            );
        }
        for change in changes {
            let line = Message::new("report-indented").arg("line", change);
            report.push_message(Tone::Info, line, None);
        }
        self.set_report(report);
        self.set_status(status, STATUS_SECONDS);
    }
}

/// The user's settings folder and what the window read from it.
#[derive(Default)]
pub struct UserFiles {
    /// The user's settings folder (docs/specs/tool-library-and-machines.md); None in tests
    /// and when there is no home folder.
    pub(super) settings_dir: Option<PathBuf>,
    /// The tool library and machine profiles as last read.
    pub(super) library: chipload_pipeline::library::Library,
    pub(super) library_state: LibraryState,
}
