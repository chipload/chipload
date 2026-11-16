// SPDX-License-Identifier: GPL-3.0-or-later
//! Project tools and machines linked to library entries by uid.

use chipload_i18n::Message;

use super::machines::MachineLibrary;
use super::tools::{Cutting, LibraryTool, ToolLibrary};
use crate::models::{Job, Tool};
use crate::problem::commas;

/// How a project got a library tool (`use_tool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsedTool {
    /// The tool's id in the project.
    pub id: i64,
    /// False when a project tool with that name and the same values was reused.
    pub added: bool,
}

impl UsedTool {
    /// "tool 3 (added)", "tool 3 (added as 3; tool 1 was taken)", "tool 2 (already in the
    /// project)".
    pub fn describe(&self, library_id: i64) -> String {
        self.describe_message(library_id).english()
    }

    /// `describe` as a message.
    pub fn describe_message(&self, library_id: i64) -> Message {
        let message = match (self.added, self.id == library_id) {
            (false, _) => Message::new("job-library-tool-there"),
            (true, true) => Message::new("job-library-tool-added"),
            (true, false) => Message::new("job-library-tool-added-as").arg("taken", library_id),
        };
        message.arg("id", self.id)
    }
}

/// Put a library tool into a project: a project tool linked to it with the same values is
/// reused; otherwise a copy is added, with the library id if that is free, else the next
/// free id.
pub fn use_tool(job: &mut Job, entry: &LibraryTool) -> UsedTool {
    let existing = job
        .tools
        .iter()
        .find(|t| t.library_uid.as_deref() == Some(entry.uid.as_str()) && entry.same_values(t));
    if let Some(tool) = existing {
        return UsedTool {
            id: tool.id,
            added: false,
        };
    }
    let taken = |id: i64| job.tools.iter().any(|t| t.id == id);
    let id = if entry.id >= 1 && !taken(entry.id) {
        entry.id
    } else {
        job.tools.iter().map(|t| t.id).max().unwrap_or(0).max(0) + 1
    };
    job.tools.push(entry.tool(id));
    UsedTool { id, added: true }
}

/// How a project tool or machine stands to the library entry it came from
/// (docs/specs/library-in-browser.md §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// Not copied from the library.
    None,
    /// No entry has its `library_uid` (deleted, or another computer's library); the name it
    /// was copied under.
    Missing(Option<String>),
    /// The entry, by its current name, has the same values.
    Same(String),
    /// The entry, by its current name, has other values: what updating would change.
    Differs(String, Vec<Message>),
}

/// What a link's entry is called in its messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkEntry {
    /// "library tool"
    Tool,
    /// "profile"
    Profile,
}

impl LinkEntry {
    fn key(self) -> &'static str {
        match self {
            LinkEntry::Tool => "tool",
            LinkEntry::Profile => "profile",
        }
    }
}

impl Link {
    /// One line for the user, in English: `message` in English.
    pub fn text(&self, what: LinkEntry) -> String {
        self.message(what).english()
    }

    /// One line for the user; `what` names the entry ("library tool", "profile").
    pub fn message(&self, what: LinkEntry) -> Message {
        let entry = what.key();
        match self {
            Link::None => Message::new("job-link-none"),
            Link::Missing(Some(name)) => Message::new("job-link-missing-named")
                .arg("entry", entry)
                .arg("name", name),
            Link::Missing(None) => Message::new("job-link-missing").arg("entry", entry),
            Link::Same(name) => Message::new("job-link-same")
                .arg("entry", entry)
                .arg("name", name),
            Link::Differs(name, changes) => Message::new("job-link-differs")
                .arg("entry", entry)
                .arg("name", name)
                .arg("changes", commas(changes.iter().cloned())),
        }
    }

    /// One word for a list: `none`, `missing`, `same` or `differs`.
    pub fn state(&self) -> &'static str {
        match self {
            Link::None => "none",
            Link::Missing(_) => "missing",
            Link::Same(_) => "same",
            Link::Differs(..) => "differs",
        }
    }

    /// The entry's current name, when it is in the library.
    pub fn entry_name(&self) -> Option<&str> {
        match self {
            Link::Same(name) | Link::Differs(name, _) => Some(name),
            Link::None | Link::Missing(_) => None,
        }
    }
}

/// How a project tool stands to the library.
pub fn tool_link(library: &ToolLibrary, tool: &Tool) -> Link {
    let Some(uid) = tool.library_uid.as_deref() else {
        return Link::None;
    };
    match library.find_uid(uid) {
        None => Link::Missing(tool.name.clone()),
        Some(entry) => {
            let changes = entry.changes(tool);
            if changes.is_empty() {
                Link::Same(entry.name.clone())
            } else {
                Link::Differs(entry.name.clone(), changes)
            }
        }
    }
}

/// How a project's machine stands to the library.
pub fn machine_link(library: &MachineLibrary, job: &Job) -> Link {
    let Some(uid) = job.machine.library_uid.as_deref() else {
        return Link::None;
    };
    match library.find_uid(uid) {
        None => Link::Missing(job.machine.name.clone()),
        Some(profile) => {
            let changes = profile.machine_changes(&job.machine);
            if changes.is_empty() {
                Link::Same(profile.name.clone())
            } else {
                Link::Differs(profile.name.clone(), changes)
            }
        }
    }
}

/// A project tool's cutting values for a new operation in `material` (its uid; None: no
/// material): the linked entry's row for it, else its row for any material, while the tool
/// still has the entry's values.
pub fn linked_cutting(
    library: &ToolLibrary,
    tool: &Tool,
    material: Option<&str>,
) -> Option<Cutting> {
    let entry = library.find_uid(tool.library_uid.as_deref()?)?;
    if entry.same_values(tool) {
        entry.cutting_for(material)
    } else {
        None
    }
}
