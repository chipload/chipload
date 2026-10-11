// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's tool library and machine profiles (docs/specs/tool-library-and-machines.md).
//!
//! Two persisted formats, `tools.json` and `machines.json`, kept in the user's settings
//! folder. A project **copies** what it uses from them; planning and G-code never read them.
//! Loading follows the job rules: one function per format, older versions migrated, missing,
//! malformed or newer versions refused. A missing file is an empty library.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::models::{
    Job, Machine, Spindle, Tool, ToolType, Travel, WorkZero, max_overcut_problems,
};

pub const TOOLS_FILE: &str = "tools.json";
pub const MACHINES_FILE: &str = "machines.json";
/// Current `schema_version` of `tools.json`.
/// Version 2 added a V-bit's `angle` and `tip_diameter` (docs/specs/v-carve.md §9).
pub const TOOLS_VERSION: i64 = 2;
/// Current `schema_version` of `machines.json`.
pub const MACHINES_VERSION: i64 = 1;
/// App settings: the recent projects (docs/specs/open-recent.md).
pub const SETTINGS_FILE: &str = "settings.json";
/// Current `schema_version` of `settings.json`. Version 2 added `theme`.
pub const SETTINGS_VERSION: i64 = 2;
/// How many recent projects are kept.
pub const RECENT_MAX: usize = 10;

#[derive(Debug, Error)]
pub enum LibraryError {
    /// Missing, malformed or newer-than-known `schema_version`. Never guessed.
    #[error(
        "{path}: schema_version {found} is not supported: this version of Chipload reads \
         {current}. Update Chipload or check the file."
    )]
    UnsupportedVersion {
        path: String,
        found: String,
        current: i64,
    },
    #[error("Cannot read {path}: {reason}")]
    Unreadable { path: String, reason: String },
    #[error("Invalid {path}: {reason}")]
    Invalid { path: String, reason: String },
    #[error("Cannot write {path}: {reason}")]
    Unwritable { path: String, reason: String },
    /// A change the library refuses, e.g. a name that isn't there.
    #[error("{0}")]
    Refused(String),
}

/// Values filled into an operation when it is created with this tool. Never applied to an
/// existing operation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cutting {
    pub feed: f64,
    pub plunge_feed: f64,
    pub step_down: f64,
    /// Fraction of the tool diameter, 0 < stepover <= 1 (pockets).
    pub stepover: f64,
}

/// One tool of the library: a project tool with a name and optional cutting values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryTool {
    pub name: String,
    /// The id it gets in a project when that id is free there.
    pub id: i64,
    #[serde(default, rename = "type")]
    pub kind: ToolType,
    pub diameter: f64,
    #[serde(default)]
    pub flutes: Option<i64>,
    #[serde(default)]
    pub center_cutting: Option<bool>,
    #[serde(default)]
    pub rpm: Option<f64>,
    #[serde(default)]
    pub cutting: Option<Cutting>,
    /// V-bits: included angle, radians; None = unknown.
    #[serde(default)]
    pub angle: Option<f64>,
    /// V-bits: flat tip diameter, mm; None = pointed.
    #[serde(default)]
    pub tip_diameter: Option<f64>,
}

impl LibraryTool {
    /// A library entry made from a project tool.
    pub fn from_tool(name: &str, tool: &Tool, cutting: Option<Cutting>) -> LibraryTool {
        LibraryTool {
            name: name.to_string(),
            id: tool.id,
            kind: tool.kind,
            diameter: tool.diameter,
            flutes: tool.flutes,
            center_cutting: tool.center_cutting,
            rpm: tool.rpm,
            cutting,
            angle: tool.angle,
            tip_diameter: tool.tip_diameter,
        }
    }

    /// The project tool this entry becomes, with the given id and the entry's name.
    pub fn tool(&self, id: i64) -> Tool {
        Tool {
            id,
            name: Some(self.name.clone()),
            kind: self.kind,
            diameter: self.diameter,
            flutes: self.flutes,
            center_cutting: self.center_cutting,
            rpm: self.rpm,
            angle: self.angle,
            tip_diameter: self.tip_diameter,
        }
    }

    /// What replacing a project tool's values with this entry would change, one line per
    /// value ("diameter 6 mm -> 8 mm"); empty when they match (id and name aside).
    pub fn changes(&self, tool: &Tool) -> Vec<String> {
        let mut changes = Vec::new();
        let mut differs = |what: &str, from: String, to: String| {
            if from != to {
                changes.push(format!("{what} {from} -> {to}"));
            }
        };
        differs(
            "type",
            tool_type_name(tool.kind).into(),
            tool_type_name(self.kind).into(),
        );
        differs("diameter", mm(tool.diameter), mm(self.diameter));
        differs(
            "flutes",
            optional(tool.flutes.map(|f| f.to_string())),
            optional(self.flutes.map(|f| f.to_string())),
        );
        differs(
            "centre cutting",
            yes_no(tool.center_cutting).into(),
            yes_no(self.center_cutting).into(),
        );
        differs(
            "rpm",
            optional(tool.rpm.map(|r| r.to_string())),
            optional(self.rpm.map(|r| r.to_string())),
        );
        differs(
            "angle",
            optional(tool.angle.map(degrees)),
            optional(self.angle.map(degrees)),
        );
        differs(
            "tip",
            optional(tool.tip_diameter.map(mm)),
            optional(self.tip_diameter.map(mm)),
        );
        changes
    }

    /// Whether a project tool has this entry's values (id and name aside).
    pub fn same_values(&self, tool: &Tool) -> bool {
        self.tool(tool.id)
            == Tool {
                name: Some(self.name.clone()),
                ..tool.clone()
            }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolLibrary {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    /// The tool a new project starts with.
    #[serde(default)]
    pub default_tool: Option<String>,
    pub tools: Vec<LibraryTool>,
}

impl Default for ToolLibrary {
    fn default() -> Self {
        ToolLibrary {
            schema_version: TOOLS_VERSION,
            written_by: None,
            default_tool: None,
            tools: Vec::new(),
        }
    }
}

impl ToolLibrary {
    /// The index of the tool with this name, ignoring case.
    pub fn position(&self, name: &str) -> Option<usize> {
        position(self.tools.iter().map(|t| t.name.as_str()), name)
    }

    pub fn find(&self, name: &str) -> Option<&LibraryTool> {
        self.position(name).map(|i| &self.tools[i])
    }

    /// The tool a new project starts with, if one is set.
    pub fn default_entry(&self) -> Option<&LibraryTool> {
        self.default_tool.as_deref().and_then(|n| self.find(n))
    }

    /// Remove a tool; the default is cleared if it was that tool.
    pub fn remove(&mut self, name: &str) -> Result<LibraryTool, LibraryError> {
        let i = self
            .position(name)
            .ok_or_else(|| LibraryError::Refused(format!("no tool named \"{name}\"")))?;
        if self
            .default_tool
            .as_deref()
            .is_some_and(|d| same_name(d, name))
        {
            self.default_tool = None;
        }
        Ok(self.tools.remove(i))
    }

    /// Every rule violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = name_problems(
            "tools",
            self.tools.iter().map(|t| t.name.as_str()),
            self.default_tool.as_deref(),
            "default_tool",
        );
        for (i, entry) in self.tools.iter().enumerate() {
            let at = format!("tools.{i}");
            problems.extend(entry.tool(entry.id).problems(&at));
            if let Some(c) = &entry.cutting {
                let at = format!("{at}.cutting");
                for (field, value) in [
                    ("feed", c.feed),
                    ("plunge_feed", c.plunge_feed),
                    ("step_down", c.step_down),
                    ("stepover", c.stepover),
                ] {
                    if value <= 0.0 || value.is_nan() {
                        problems.push(format!("{at}.{field}: must be greater than 0, got {value}"));
                    }
                }
                if c.stepover > 1.0 {
                    problems.push(format!(
                        "{at}.stepover: must be at most 1 (a fraction of the tool diameter), \
                         got {}",
                        c.stepover
                    ));
                }
            }
        }
        problems
    }
}

/// One machine profile: a project's machine values plus the spoilboard allowance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineProfile {
    pub name: String,
    pub post: String,
    pub safe_z: f64,
    pub spindle: Spindle,
    #[serde(default)]
    pub max_feed: Option<f64>,
    #[serde(default)]
    pub travel: Option<Travel>,
    /// The work zero this machine is usually set up with.
    pub work_zero: WorkZero,
    /// Kept under `stock` in projects; it is about the spoilboard, so it belongs here.
    pub max_overcut: f64,
}

impl MachineProfile {
    /// A profile made from a project's machine values.
    pub fn from_job(name: &str, job: &Job) -> MachineProfile {
        let m = &job.machine;
        MachineProfile {
            name: name.to_string(),
            post: m.post.clone(),
            safe_z: m.safe_z,
            spindle: m.spindle,
            max_feed: m.max_feed,
            travel: m.travel.clone(),
            work_zero: m.work_zero,
            max_overcut: job.stock.max_overcut,
        }
    }

    /// The project machine this profile becomes, with the profile's name.
    pub fn machine(&self) -> Machine {
        Machine {
            name: Some(self.name.clone()),
            post: self.post.clone(),
            safe_z: self.safe_z,
            spindle: self.spindle,
            max_feed: self.max_feed,
            travel: self.travel.clone(),
            work_zero: self.work_zero,
        }
    }

    /// What applying this profile would change in a project, one line per value
    /// ("safe Z 5 -> 10 mm"); empty when the values already match (the name aside).
    pub fn changes(&self, job: &Job) -> Vec<String> {
        let old = &job.machine;
        let mut changes = Vec::new();
        let mut differs = |what: &str, from: String, to: String| {
            if from != to {
                changes.push(format!("{what} {from} -> {to}"));
            }
        };
        differs("post", old.post.clone(), self.post.clone());
        differs("safe Z", mm(old.safe_z), mm(self.safe_z));
        differs(
            "spindle",
            spindle_name(old.spindle).into(),
            spindle_name(self.spindle).into(),
        );
        differs(
            "max feed",
            optional(old.max_feed.map(|f| format!("{f} mm/min"))),
            optional(self.max_feed.map(|f| format!("{f} mm/min"))),
        );
        differs(
            "travel",
            optional(old.travel.as_ref().map(travel_text)),
            optional(self.travel.as_ref().map(travel_text)),
        );
        differs(
            "work zero",
            work_zero_name(&old.work_zero),
            work_zero_name(&self.work_zero),
        );
        differs(
            "max overcut",
            mm(job.stock.max_overcut),
            mm(self.max_overcut),
        );
        changes
    }

    /// Copy this profile's values (and name) into a project; returns what changed.
    pub fn apply(&self, job: &mut Job) -> Vec<String> {
        let changes = self.changes(job);
        job.machine = self.machine();
        job.stock.max_overcut = self.max_overcut;
        changes
    }
}

fn mm(value: f64) -> String {
    format!("{value} mm")
}

fn degrees(radians: f64) -> String {
    format!("{}°", radians.to_degrees())
}

fn optional(text: Option<String>) -> String {
    text.unwrap_or_else(|| "none".into())
}

fn travel_text(t: &Travel) -> String {
    format!("X{} Y{} Z{} mm", t.x, t.y, t.z)
}

fn tool_type_name(kind: ToolType) -> &'static str {
    match kind {
        ToolType::Endmill => "endmill",
        ToolType::Drill => "drill",
        ToolType::Vbit => "v-bit",
    }
}

fn yes_no(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}

fn spindle_name(spindle: Spindle) -> &'static str {
    match spindle {
        Spindle::Cw => "cw",
        Spindle::Ccw => "ccw",
    }
}

fn work_zero_name(zero: &WorkZero) -> String {
    match zero {
        WorkZero::DrawingOrigin {} => "drawing origin".into(),
        WorkZero::Stock { corner } => format!("stock {}", corner.name()),
        WorkZero::Point { x, y } => format!("point X{x} Y{y}"),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineLibrary {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    /// The machine a new project starts with.
    #[serde(default)]
    pub default_machine: Option<String>,
    pub machines: Vec<MachineProfile>,
}

impl Default for MachineLibrary {
    fn default() -> Self {
        MachineLibrary {
            schema_version: MACHINES_VERSION,
            written_by: None,
            default_machine: None,
            machines: Vec::new(),
        }
    }
}

impl MachineLibrary {
    /// The index of the profile with this name, ignoring case.
    pub fn position(&self, name: &str) -> Option<usize> {
        position(self.machines.iter().map(|m| m.name.as_str()), name)
    }

    pub fn find(&self, name: &str) -> Option<&MachineProfile> {
        self.position(name).map(|i| &self.machines[i])
    }

    /// The profile a new project starts with, if one is set.
    pub fn default_entry(&self) -> Option<&MachineProfile> {
        self.default_machine.as_deref().and_then(|n| self.find(n))
    }

    /// Remove a profile; the default is cleared if it was that profile.
    pub fn remove(&mut self, name: &str) -> Result<MachineProfile, LibraryError> {
        let i = self
            .position(name)
            .ok_or_else(|| LibraryError::Refused(format!("no machine named \"{name}\"")))?;
        if self
            .default_machine
            .as_deref()
            .is_some_and(|d| same_name(d, name))
        {
            self.default_machine = None;
        }
        Ok(self.machines.remove(i))
    }

    /// Every rule violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = name_problems(
            "machines",
            self.machines.iter().map(|m| m.name.as_str()),
            self.default_machine.as_deref(),
            "default_machine",
        );
        for (i, profile) in self.machines.iter().enumerate() {
            let at = format!("machines.{i}");
            problems.extend(profile.machine().problems(&at));
            problems.extend(max_overcut_problems(&at, profile.max_overcut));
            if !profile.safe_z.is_finite() {
                problems.push(format!(
                    "{at}.safe_z: must be a number, got {}",
                    profile.safe_z
                ));
            }
            if profile.post.trim().is_empty() {
                problems.push(format!("{at}.post: must name a post-processor"));
            }
            if let Some(feed) = profile.max_feed
                && (feed <= 0.0 || feed.is_nan())
            {
                problems.push(format!("{at}.max_feed: must be greater than 0, got {feed}"));
            }
        }
        problems
    }
}

/// The GUI's colours (docs/specs/open-recent.md section 1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Light or dark as the operating system is set.
    #[default]
    System,
    Light,
    Dark,
}

/// App settings kept between sessions (docs/specs/open-recent.md).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSettings {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    /// Absolute paths, most recent first, at most `RECENT_MAX`, no duplicates.
    pub recent_projects: Vec<String>,
    /// Since version 2.
    pub theme: Theme,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            schema_version: SETTINGS_VERSION,
            written_by: None,
            recent_projects: Vec::new(),
            theme: Theme::System,
        }
    }
}

impl AppSettings {
    /// Put a project at the top of the recent list (moving it if it is there already).
    pub fn add_recent(&mut self, path: &str) {
        self.remove_recent(path);
        self.recent_projects.insert(0, path.to_string());
        self.recent_projects.truncate(RECENT_MAX);
    }

    pub fn remove_recent(&mut self, path: &str) {
        self.recent_projects.retain(|p| p != path);
    }

    /// Every rule violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let recent = &self.recent_projects;
        if recent.len() > RECENT_MAX {
            problems.push(format!(
                "recent_projects: at most {RECENT_MAX}, got {}",
                recent.len()
            ));
        }
        for (i, path) in recent.iter().enumerate() {
            if path.trim().is_empty() {
                problems.push(format!("recent_projects.{i}: must not be empty"));
            }
            if recent[..i].contains(path) {
                problems.push(format!("recent_projects.{i}: \"{path}\" is listed twice"));
            }
        }
        problems
    }
}

/// Names compare ignoring case, so "6 mm Upcut" and "6 mm upcut" can't both exist.
pub fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn position<'a>(mut names: impl Iterator<Item = &'a str>, name: &str) -> Option<usize> {
    names.position(|n| same_name(n, name))
}

/// Names are not empty, trimmed and unique ignoring case; the default names an entry.
fn name_problems<'a>(
    list: &str,
    names: impl Iterator<Item = &'a str>,
    default: Option<&str>,
    default_field: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    let names: Vec<&str> = names.collect();
    for (i, name) in names.iter().enumerate() {
        if name.trim().is_empty() {
            problems.push(format!("{list}.{i}.name: must not be empty"));
        } else if name.trim() != *name {
            problems.push(format!(
                "{list}.{i}.name: must not start or end with spaces, got \"{name}\""
            ));
        }
        if names[..i].iter().any(|earlier| same_name(earlier, name)) {
            problems.push(format!(
                "{list}.{i}.name: \"{name}\" is used twice (names ignore case)"
            ));
        }
    }
    if let Some(default) = default
        && !names.iter().any(|n| same_name(n, default))
    {
        problems.push(format!("{default_field}: \"{default}\" is not in {list}"));
    }
    problems
}

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
        match (self.added, self.id == library_id) {
            (false, _) => format!("tool {} (already in the project)", self.id),
            (true, true) => format!("tool {} (added)", self.id),
            (true, false) => format!(
                "tool {} (added as {}; tool {library_id} was taken)",
                self.id, self.id
            ),
        }
    }
}

/// Put a library tool into a project: a project tool with the same name and values is
/// reused; otherwise a copy is added, with the library id if that is free, else the next
/// free id.
pub fn use_tool(job: &mut Job, entry: &LibraryTool) -> UsedTool {
    let existing = job.tools.iter().find(|t| {
        t.name.as_deref().is_some_and(|n| same_name(n, &entry.name)) && entry.same_values(t)
    });
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

/// A library file format: its version, name in messages, and rules.
trait LibraryFile: Serialize + DeserializeOwned + Default + Clone {
    const VERSION: i64;
    /// The oldest version still read; older ones are upgraded by `upgrade`.
    const OLDEST: i64 = Self::VERSION;
    fn validate(&self) -> Vec<String>;
    fn stamp(&mut self);
    /// Upgrade data of version `from` to `from + 1`. Pure: no I/O, no clock.
    fn upgrade(from: i64, data: &mut serde_json::Map<String, Value>) {
        let _ = (from, data);
    }
}

impl LibraryFile for ToolLibrary {
    const VERSION: i64 = TOOLS_VERSION;
    const OLDEST: i64 = 1;
    fn validate(&self) -> Vec<String> {
        ToolLibrary::validate(self)
    }
    fn stamp(&mut self) {
        self.schema_version = TOOLS_VERSION;
        self.written_by = Some(written_by());
    }
    fn upgrade(from: i64, data: &mut serde_json::Map<String, Value>) {
        if from == 1 {
            tools_v1_to_v2(data);
        }
    }
}

/// Tools 1 -> 2: V-bits may have an angle and a tip; existing ones get neither (unknown).
pub fn tools_v1_to_v2(data: &mut serde_json::Map<String, Value>) {
    data.insert("schema_version".into(), Value::from(2));
}

impl LibraryFile for MachineLibrary {
    const VERSION: i64 = MACHINES_VERSION;
    fn validate(&self) -> Vec<String> {
        MachineLibrary::validate(self)
    }
    fn stamp(&mut self) {
        self.schema_version = MACHINES_VERSION;
        self.written_by = Some(written_by());
    }
}

impl LibraryFile for AppSettings {
    const VERSION: i64 = SETTINGS_VERSION;
    const OLDEST: i64 = 1;
    fn validate(&self) -> Vec<String> {
        AppSettings::validate(self)
    }
    fn stamp(&mut self) {
        self.schema_version = SETTINGS_VERSION;
        self.written_by = Some(written_by());
    }
    fn upgrade(from: i64, data: &mut serde_json::Map<String, Value>) {
        if from == 1 {
            settings_v1_to_v2(data);
        }
    }
}

/// Settings 1 -> 2: the theme, which until then always followed the system.
pub fn settings_v1_to_v2(data: &mut serde_json::Map<String, Value>) {
    data.insert("theme".into(), Value::String("system".into()));
    data.insert("schema_version".into(), Value::from(2));
}

fn written_by() -> String {
    format!("chipload {}", env!("CARGO_PKG_VERSION"))
}

/// Load `tools.json`; a missing file is an empty library.
pub fn load_tools(path: &Path) -> Result<ToolLibrary, LibraryError> {
    load(path)
}

/// Load `machines.json`; a missing file is an empty library.
pub fn load_machines(path: &Path) -> Result<MachineLibrary, LibraryError> {
    load(path)
}

/// Save `tools.json` (§6: validated, atomic, the previous file kept as `.bak`).
pub fn save_tools(library: &ToolLibrary, path: &Path) -> Result<(), LibraryError> {
    save(library, path)
}

/// Save `machines.json` (as `save_tools`).
pub fn save_machines(library: &MachineLibrary, path: &Path) -> Result<(), LibraryError> {
    save(library, path)
}

/// Read `tools.json` again, change it, save it: two windows don't undo each other's
/// unrelated edits. A broken or newer file is refused, never overwritten.
pub fn update_tools<T>(
    path: &Path,
    change: impl FnOnce(&mut ToolLibrary) -> Result<T, LibraryError>,
) -> Result<T, LibraryError> {
    update(path, change)
}

/// As `update_tools`, for `machines.json`.
pub fn update_machines<T>(
    path: &Path,
    change: impl FnOnce(&mut MachineLibrary) -> Result<T, LibraryError>,
) -> Result<T, LibraryError> {
    update(path, change)
}

/// Load `settings.json`; a missing file is the default settings.
pub fn load_settings(path: &Path) -> Result<AppSettings, LibraryError> {
    load(path)
}

/// As `update_tools`, for `settings.json`.
pub fn update_settings<T>(
    path: &Path,
    change: impl FnOnce(&mut AppSettings) -> Result<T, LibraryError>,
) -> Result<T, LibraryError> {
    update(path, change)
}

fn update<L: LibraryFile, T>(
    path: &Path,
    change: impl FnOnce(&mut L) -> Result<T, LibraryError>,
) -> Result<T, LibraryError> {
    let mut library: L = load(path)?;
    let result = change(&mut library)?;
    save(&library, path)?;
    Ok(result)
}

fn load<L: LibraryFile>(path: &Path) -> Result<L, LibraryError> {
    let shown = path.display().to_string();
    let unreadable = |reason: String| LibraryError::Unreadable {
        path: shown.clone(),
        reason,
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(L::default()),
        Err(e) => return Err(unreadable(e.to_string())),
    };
    let data: Value = serde_json::from_str(&text).map_err(|e| unreadable(e.to_string()))?;
    let Value::Object(mut object) = data else {
        return Err(unreadable("must contain a JSON object".into()));
    };
    let version = object.get("schema_version");
    let Some(found) = version
        .and_then(Value::as_i64)
        .filter(|v| (L::OLDEST..=L::VERSION).contains(v))
    else {
        return Err(LibraryError::UnsupportedVersion {
            path: shown,
            found: version.map_or_else(|| "(missing)".into(), Value::to_string),
            current: L::VERSION,
        });
    };
    // Older versions are upgraded one step at a time, then read as the current one.
    for step in found..L::VERSION {
        L::upgrade(step, &mut object);
    }
    let data = Value::Object(object);
    let library: L = serde_json::from_value(data).map_err(|e| LibraryError::Invalid {
        path: shown.clone(),
        reason: e.to_string(),
    })?;
    check(&library, &shown)?;
    Ok(library)
}

fn check<L: LibraryFile>(library: &L, path: &str) -> Result<(), LibraryError> {
    let problems = library.validate();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(LibraryError::Invalid {
            path: path.to_string(),
            reason: problems.join("; "),
        })
    }
}

fn save<L: LibraryFile>(library: &L, path: &Path) -> Result<(), LibraryError> {
    let shown = path.display().to_string();
    check(library, &shown)?;
    let failed = |e: std::io::Error| LibraryError::Unwritable {
        path: shown.clone(),
        reason: e.to_string(),
    };
    let mut current = library.clone();
    current.stamp();
    let text = serde_json::to_string_pretty(&current).map_err(|e| LibraryError::Invalid {
        path: shown.clone(),
        reason: e.to_string(),
    })?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    let with_suffix = |suffix: &str| {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(suffix);
        path.with_file_name(name)
    };
    let tmp = with_suffix(".tmp");
    std::fs::write(&tmp, crate::ascii_json(&text)).map_err(failed)?;
    if path.exists() {
        std::fs::copy(path, with_suffix(".bak")).map_err(failed)?;
    }
    std::fs::rename(&tmp, path).map_err(failed)
}
