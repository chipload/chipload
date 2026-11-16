// SPDX-License-Identifier: GPL-3.0-or-later
//! Importing and exporting tools and machines between files.

use std::path::Path;

use chipload_i18n::{Message, Value as I18nValue};
use serde_json::Value;

use super::LibraryError;
use super::files::LibraryFile;
use super::files::{parse, write};
use super::machines::Changes;
use super::machines::{MachineLibrary, MachineProfile, mm, optional};
use super::machines::{feed, number};
use super::names::{fresh_uid, same_name};
use super::tools::{Cutting, LibraryTool, ToolLibrary};
use crate::foreign;
use crate::problem::semicolons;

// Import and export (docs/specs/library-import-export.md).

/// A library that can be exported and imported: its entries and its default.
pub trait Collection: LibraryFile {
    type Entry: Entry;
    /// The list's field in the file ("tools").
    const LIST: &'static str;
    /// One entry in messages ("tool").
    const ENTRY: &'static str;
    /// The other library's list field, and the message (its id) to say when a file holds
    /// that one.
    const OTHER: (&'static str, &'static str);
    fn entries(&self) -> &[Self::Entry];
    fn entries_mut(&mut self) -> &mut Vec<Self::Entry>;
    fn default_name(&self) -> Option<&str>;
    fn set_default_name(&mut self, name: Option<String>);
}

/// An entry of a library: a tool or a machine profile.
pub trait Entry: Clone + PartialEq {
    fn name(&self) -> &str;
    /// What taking `other`'s values would change, one line per value ("diameter 6 mm ->
    /// 8 mm"); empty when they match (the name aside).
    fn differences(&self, other: &Self) -> Vec<Message>;
    /// The same entry with another name.
    fn renamed(&self, name: &str) -> Self;
    fn uid(&self) -> &str;
    /// The same entry with another uid.
    fn with_uid(&self, uid: &str) -> Self;
}

impl Collection for ToolLibrary {
    type Entry = LibraryTool;
    const LIST: &'static str = "tools";
    const ENTRY: &'static str = "tool";
    const OTHER: (&'static str, &'static str) = ("machines", "job-library-holds-machines");
    fn entries(&self) -> &[LibraryTool] {
        &self.tools
    }
    fn entries_mut(&mut self) -> &mut Vec<LibraryTool> {
        &mut self.tools
    }
    fn default_name(&self) -> Option<&str> {
        self.default_tool.as_deref()
    }
    fn set_default_name(&mut self, name: Option<String>) {
        self.default_tool = name;
    }
}

impl Collection for MachineLibrary {
    type Entry = MachineProfile;
    const LIST: &'static str = "machines";
    const ENTRY: &'static str = "machine";
    const OTHER: (&'static str, &'static str) = ("tools", "job-library-holds-tools");
    fn entries(&self) -> &[MachineProfile] {
        &self.machines
    }
    fn entries_mut(&mut self) -> &mut Vec<MachineProfile> {
        &mut self.machines
    }
    fn default_name(&self) -> Option<&str> {
        self.default_machine.as_deref()
    }
    fn set_default_name(&mut self, name: Option<String>) {
        self.default_machine = name;
    }
}

impl Entry for LibraryTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn renamed(&self, name: &str) -> Self {
        LibraryTool {
            name: name.to_string(),
            ..self.clone()
        }
    }
    fn uid(&self) -> &str {
        &self.uid
    }
    fn with_uid(&self, uid: &str) -> Self {
        LibraryTool {
            uid: uid.to_string(),
            ..self.clone()
        }
    }
    fn differences(&self, other: &Self) -> Vec<Message> {
        let mut changes = Changes(other.changes(&self.tool(self.id)));
        changes.differs("id", self.id.into(), other.id.into());
        // One line per material whose row differs, in the order they are listed.
        let mut seen: Vec<Option<&str>> = Vec::new();
        for row in self.cutting_data.iter().chain(&other.cutting_data) {
            let material = row.material_uid.as_deref();
            if seen.contains(&material) {
                continue;
            }
            seen.push(material);
            let (mine, theirs) = (self.row(material), other.row(material));
            if mine != theirs {
                changes.differs(
                    "cutting-values",
                    cutting_text(mine.map(|r| &r.values)),
                    cutting_text(theirs.map(|r| &r.values)),
                );
            }
        }
        changes.0
    }
}

impl Entry for MachineProfile {
    fn name(&self) -> &str {
        &self.name
    }
    fn renamed(&self, name: &str) -> Self {
        MachineProfile {
            name: name.to_string(),
            ..self.clone()
        }
    }
    fn uid(&self) -> &str {
        &self.uid
    }
    fn with_uid(&self, uid: &str) -> Self {
        MachineProfile {
            uid: uid.to_string(),
            ..self.clone()
        }
    }
    fn differences(&self, other: &Self) -> Vec<Message> {
        other.compare(&self.machine(), Some((self.max_overcut, self.work_zero)))
    }
}

pub(crate) fn cutting_text(cutting: Option<&Cutting>) -> I18nValue {
    optional(cutting.map(|c| {
        Message::new("job-library-cutting")
            .arg("slot_feed", feed(c.slot_feed))
            .arg("slot_step_down", mm(c.slot_step_down))
            .arg("feed", feed(c.feed))
            .arg("plunge", feed(c.plunge_feed))
            .arg("step_down", mm(c.step_down))
            .arg("stepover", number(c.stepover))
            .into()
    }))
}

/// What an import does to a library (§4), by name as the library spells it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportPlan {
    /// Entries the library doesn't have, in the file's order.
    pub added: Vec<String>,
    /// Entries the library has with the same values.
    pub same: Vec<String>,
    /// Entries the library has with other values, and what importing would change.
    pub differing: Vec<(String, Vec<Message>)>,
    /// The file's default, taken because the library has none.
    pub default: Option<String>,
}

/// What importing `file` into `library` would do; nothing is changed.
pub fn import_plan<L: Collection>(library: &L, file: &L) -> ImportPlan {
    let mut plan = ImportPlan::default();
    for entry in file.entries() {
        let found = library
            .entries()
            .iter()
            .find(|e| same_name(e.name(), entry.name()));
        match found {
            None => plan.added.push(entry.name().to_string()),
            Some(mine) => {
                let mut differences = mine.differences(entry);
                if differences.is_empty() && !same_values(mine, entry) {
                    differences.push(Message::new("job-library-other-values"));
                }
                if differences.is_empty() {
                    plan.same.push(mine.name().to_string());
                } else {
                    plan.differing.push((mine.name().to_string(), differences));
                }
            }
        }
    }
    if library.default_name().is_none() {
        plan.default = file.default_name().map(str::to_string);
    }
    plan
}

/// Equal values, the name's spelling and the uid aside.
fn same_values<E: Entry>(a: &E, b: &E) -> bool {
    *a == b.renamed(a.name()).with_uid(a.uid())
}

/// Import `file` into `library` (§4): new entries are added, differing ones replaced only
/// with `replace`, the default taken only by a library without one. Returns the plan done.
pub fn import_into<L: Collection>(library: &mut L, file: &L, replace: bool) -> ImportPlan {
    let plan = import_plan(library, file);
    for entry in file.entries() {
        let found = library
            .entries()
            .iter()
            .position(|e| same_name(e.name(), entry.name()));
        match found {
            None => {
                // The file's uid, unless another entry has it.
                let clash = library.entries().iter().any(|e| e.uid() == entry.uid());
                let added = if clash {
                    let uid = fresh_uid(entry.name(), library.entries().iter().map(Entry::uid));
                    entry.with_uid(&uid)
                } else {
                    entry.clone()
                };
                library.entries_mut().push(added);
            }
            Some(i) if replace => {
                // The library's spelling of the name and its uid stay with its position,
                // so projects linked to it stay linked.
                let mine = &library.entries()[i];
                let taken = entry.renamed(mine.name()).with_uid(mine.uid());
                library.entries_mut()[i] = taken;
            }
            Some(_) => {}
        }
    }
    if let Some(name) = &plan.default {
        let spelled = library
            .entries()
            .iter()
            .find(|e| same_name(e.name(), name))
            .map(|e| e.name().to_string());
        library.set_default_name(spelled);
    }
    plan
}

/// The entries named (all when `names` is empty) as a library of their own, keeping the
/// default when it is among them. An unknown name is refused.
pub fn export<L: Collection>(library: &L, names: &[String]) -> Result<L, LibraryError> {
    let mut out = L::default();
    if names.is_empty() {
        out.entries_mut().extend(library.entries().iter().cloned());
    } else {
        for name in names {
            let entry = library
                .entries()
                .iter()
                .find(|e| same_name(e.name(), name))
                .ok_or_else(|| {
                    refused(
                        Message::new("job-library-no-entry")
                            .arg("entry", L::ENTRY)
                            .arg("name", name),
                    )
                })?;
            if !out.entries().iter().any(|e| same_name(e.name(), name)) {
                out.entries_mut().push(entry.clone());
            }
        }
    }
    let default = library
        .default_name()
        .filter(|d| out.entries().iter().any(|e| same_name(e.name(), d)));
    out.set_default_name(default.map(str::to_string));
    Ok(out)
}

/// Read a file to import: as the library loader, but a missing file is an error, and a
/// file of the other library is refused with a hint.
pub fn read_import<L: Collection>(path: &Path) -> Result<L, LibraryError> {
    let shown = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| LibraryError::Unreadable {
        path: shown.clone(),
        reason: foreign(e),
    })?;
    if let Ok(Value::Object(object)) = serde_json::from_str::<Value>(&text) {
        let (other, hint) = L::OTHER;
        if !object.contains_key(L::LIST) && object.contains_key(other) {
            return Err(refused(
                Message::new("job-library-other-file")
                    .arg("path", shown)
                    .arg("hint", Message::new(hint)),
            ));
        }
    }
    parse(&text, &shown)
}

/// Write an exported library: the current version, atomically, without a `.bak`.
pub fn write_export<L: Collection>(library: &L, path: &Path) -> Result<(), LibraryError> {
    write(library, path, false)
}

impl ImportPlan {
    /// The plan as lines for the user, in English: `report_messages` in English.
    pub fn report(&self, what: &str, replace: bool) -> Vec<String> {
        self.report_messages(what, replace)
            .iter()
            .map(Message::english)
            .collect()
    }

    /// The plan as lines for the user; `replace` says whether differing entries were
    /// replaced or kept, `what` names the entries ("tools", "machines").
    pub fn report_messages(&self, what: &str, replace: bool) -> Vec<Message> {
        let mut lines = Vec::new();
        if self.added.is_empty() && self.same.is_empty() && self.differing.is_empty() {
            lines.push(Message::new("job-import-empty").arg("list", what));
        }
        if !self.added.is_empty() {
            lines.push(Message::new("job-import-added").arg("names", self.added.join(", ")));
        }
        if !self.same.is_empty() {
            lines.push(Message::new("job-import-same").arg("names", self.same.join(", ")));
        }
        for (name, differences) in &self.differing {
            let id = if replace {
                "job-import-replaced"
            } else {
                "job-import-kept"
            };
            lines.push(Message::new(id).arg("name", name).arg(
                "differences",
                semicolons(differences.iter().cloned()).unwrap_or_else(other_values),
            ));
        }
        if let Some(name) = &self.default {
            lines.push(Message::new("job-import-default").arg("name", name));
        }
        lines
    }

    /// What importing would do, as lines for the user before it is done, in English.
    pub fn preview(&self, what: &str) -> Vec<String> {
        self.preview_messages(what)
            .iter()
            .map(Message::english)
            .collect()
    }

    /// What importing would do, as lines for the user before it is done.
    pub fn preview_messages(&self, what: &str) -> Vec<Message> {
        let mut lines = Vec::new();
        if self.added.is_empty() && self.same.is_empty() && self.differing.is_empty() {
            lines.push(Message::new("job-import-empty").arg("list", what));
        }
        if !self.added.is_empty() {
            lines.push(Message::new("job-import-new").arg("names", self.added.join(", ")));
        }
        if !self.same.is_empty() {
            lines.push(Message::new("job-import-same").arg("names", self.same.join(", ")));
        }
        for (name, differences) in &self.differing {
            lines.push(Message::new("job-import-differs").arg("name", name).arg(
                "differences",
                semicolons(differences.iter().cloned()).unwrap_or_else(other_values),
            ));
        }
        if let Some(name) = &self.default {
            lines.push(Message::new("job-import-becomes-default").arg("name", name));
        }
        lines
    }
}

/// A differing entry always says how; this stands in should its list ever be empty.
fn other_values() -> Message {
    Message::new("job-library-other-values")
}

/// A library's refusal, as a message.
pub(crate) fn refused(message: Message) -> LibraryError {
    LibraryError::Refused(message.into())
}
