// SPDX-License-Identifier: GPL-3.0-or-later
//! Machine profiles and their library, and the changes between two profiles in words.

use chipload_i18n::{Digits, Message, Value as I18nValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::exchange::refused;
use super::files::LibraryFile;
use super::files::written_by;
use super::names::add_uids;
use super::names::{fresh_uid, name_problems, position, same_name, uid_problems};
use super::{LibraryError, MACHINES_VERSION};
use crate::models::{
    Job, Machine, Spindle, ToolChange, Travel, Units, WorkZero, max_overcut_problems,
};

/// One machine profile: a project's machine values plus the spoilboard allowance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MachineProfile {
    /// The profile's name, unique in the file.
    pub name: String,
    /// What projects link the profile by: 16 lowercase hex digits, unique in the file,
    /// never changed (docs/specs/library-in-browser.md §2).
    pub uid: String,
    /// The post-processor: `grbl`, `linuxcnc`, `mach3`, or the name of your own.
    pub post: String,
    /// The units its G-code is written in: `mm` (G21) or `inch` (G20).
    pub gcode_units: Units,
    /// The height for rapid moves sideways, mm above Z0. Above the stock top.
    pub safe_z: f64,
    /// The spindle's direction: `cw` (M3) or `ccw` (M4).
    pub spindle: Spindle,
    /// The machine's fastest feed, mm/min: higher feeds are refused, and the cycle time
    /// uses it for rapids. Null: unknown.
    #[serde(default)]
    pub max_feed: Option<f64>,
    /// The machine's travel, mm. When set, a program that moves farther is refused. Null:
    /// not checked.
    #[serde(default)]
    pub travel: Option<Travel>,
    /// The point of the drawing that becomes X0 Y0 in the G-code. Missing: the drawing's
    /// origin.
    pub work_zero: WorkZero,
    /// How far a new project's cuts may go below the stock bottom, mm
    /// (`stock.max_overcut`).
    pub max_overcut: f64,
    /// With several tools: `files` (one file per tool) or `program` (one file with tool
    /// changes).
    pub tool_change: ToolChange,
}

impl MachineProfile {
    /// A profile made from a project's machine values, with the given uid.
    pub fn from_job(name: &str, uid: &str, job: &Job) -> MachineProfile {
        let m = &job.machine;
        MachineProfile {
            name: name.to_string(),
            uid: uid.to_string(),
            post: m.post.clone(),
            gcode_units: m.gcode_units,
            safe_z: m.safe_z,
            spindle: m.spindle,
            max_feed: m.max_feed,
            travel: m.travel.clone(),
            work_zero: job.work_zero(),
            max_overcut: job.stock().max_overcut,
            tool_change: m.tool_change,
        }
    }

    /// The project machine this profile becomes, linked to the profile.
    pub fn machine(&self) -> Machine {
        Machine {
            name: Some(self.name.clone()),
            library_uid: Some(self.uid.clone()),
            post: self.post.clone(),
            gcode_units: self.gcode_units,
            safe_z: self.safe_z,
            spindle: self.spindle,
            max_feed: self.max_feed,
            travel: self.travel.clone(),
            tool_change: self.tool_change,
            // A user post is copied in by the caller, which knows the posts folder.
            custom_post: None,
        }
    }

    /// What applying this profile would change in a project, one line per value
    /// ("safe Z 5 -> 10 mm"); empty when the values already match (the name aside).
    pub fn changes(&self, job: &Job) -> Vec<Message> {
        self.compare(
            &job.machine,
            Some((job.stock().max_overcut, job.work_zero())),
        )
    }

    /// What differs in the machine values only (post, tool changes, spindle, max feed,
    /// travel): what the link compares (docs/specs/linked-machine.md §2). Safe Z, the work
    /// zero and the max overcut are the job's, and a profile only starts them.
    pub fn machine_changes(&self, old: &Machine) -> Vec<Message> {
        self.compare(old, None)
    }

    /// What differs from `old`; with the job values too when the project's max overcut and
    /// work zero are given.
    pub(super) fn compare(&self, old: &Machine, job: Option<(f64, WorkZero)>) -> Vec<Message> {
        let job_values = job.is_some();
        let mut changes = Changes::default();
        changes.differs("post", old.post.clone().into(), self.post.clone().into());
        changes.differs(
            "G-code-units",
            old.gcode_units.name().into(),
            self.gcode_units.name().into(),
        );
        changes.differs(
            "tool-changes",
            old.tool_change.name().into(),
            self.tool_change.name().into(),
        );
        if job_values {
            changes.differs("safe-Z", mm(old.safe_z), mm(self.safe_z));
        }
        changes.differs(
            "spindle",
            spindle_name(old.spindle).into(),
            spindle_name(self.spindle).into(),
        );
        changes.differs(
            "max-feed",
            optional(old.max_feed.map(feed)),
            optional(self.max_feed.map(feed)),
        );
        changes.differs(
            "travel",
            optional(old.travel.as_ref().map(travel_text)),
            optional(self.travel.as_ref().map(travel_text)),
        );
        if let Some((max_overcut, work_zero)) = job {
            changes.differs(
                "work-zero",
                work_zero_name(&work_zero),
                work_zero_name(&self.work_zero),
            );
            changes.differs("max-overcut", mm(max_overcut), mm(self.max_overcut));
        }
        changes.0
    }

    /// Copy only this profile's machine values (and its name and uid) into a project,
    /// keeping its safe Z, work zero and max overcut; returns what changed
    /// (docs/specs/linked-machine.md §3). Both *Use profile* and *Update from library* in an
    /// open project (§2: the user, 2026-10-31, "work zero was reset if I changed machine").
    pub fn update_machine_values(&self, job: &mut Job) -> Vec<Message> {
        let changes = self.machine_changes(&job.machine);
        let keep = job.machine.safe_z;
        job.machine = self.machine();
        job.machine.safe_z = keep;
        changes
    }

    /// Copy all this profile's values (and name) into a new project, its work zero and max
    /// overcut into the active setup; returns what changed. An open project takes only the
    /// machine values (`update_machine_values`).
    pub fn apply(&self, job: &mut Job) -> Vec<Message> {
        let changes = self.changes(job);
        job.machine = self.machine();
        job.stock_mut().max_overcut = self.max_overcut;
        *job.work_zero_mut() = self.work_zero;
        changes
    }
}

/// What differs between two entries, one message per value: "diameter 6 mm -> 8 mm".
#[derive(Default)]
pub(crate) struct Changes(pub(crate) Vec<Message>);

impl Changes {
    /// A change of the value `field` (a key of `job-library-field`), if `from` isn't `to`.
    pub(crate) fn differs(&mut self, field: &str, from: I18nValue, to: I18nValue) {
        if from != to {
            self.0.push(
                Message::new("job-library-change")
                    .arg(
                        "what",
                        Message::new("job-library-field").arg("field", field),
                    )
                    .arg("from", from)
                    .arg("to", to),
            );
        }
    }
}

/// A length as Rust writes it, with its unit.
pub(crate) fn mm(value: f64) -> I18nValue {
    I18nValue::Length(value, Digits::Shortest)
}

pub(super) fn feed(value: f64) -> I18nValue {
    I18nValue::Feed(value, Digits::Shortest)
}

pub(super) fn number(value: f64) -> I18nValue {
    I18nValue::Number(value, Digits::Shortest)
}

pub(super) fn degrees(radians: f64) -> I18nValue {
    Message::new("job-library-degrees")
        .arg("degrees", number(radians.to_degrees()))
        .into()
}

pub(crate) fn optional(value: Option<I18nValue>) -> I18nValue {
    value.unwrap_or_else(|| Message::new("job-library-none").into())
}

fn travel_text(t: &Travel) -> I18nValue {
    Message::new("job-library-travel")
        .arg("x", I18nValue::LengthNumber(t.x, Digits::Shortest))
        .arg("y", I18nValue::LengthNumber(t.y, Digits::Shortest))
        .arg("z", mm(t.z))
        .into()
}

pub(super) fn yes_no(value: Option<bool>) -> I18nValue {
    let word = match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    };
    Message::new("job-library-yes-no").arg("value", word).into()
}

fn spindle_name(spindle: Spindle) -> &'static str {
    match spindle {
        Spindle::Cw => "cw",
        Spindle::Ccw => "ccw",
    }
}

fn work_zero_name(zero: &WorkZero) -> I18nValue {
    let message = match zero {
        WorkZero::DrawingOrigin {} => Message::new("job-library-work-zero-origin"),
        WorkZero::Stock { corner } => {
            Message::new("job-library-work-zero-stock").arg("corner", corner.name())
        }
        WorkZero::Point { x, y } => Message::new("job-library-work-zero-point")
            .arg("x", I18nValue::LengthNumber(*x, Digits::Shortest))
            .arg("y", I18nValue::LengthNumber(*y, Digits::Shortest)),
    };
    message.into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MachineLibrary {
    /// The version of the file format.
    pub schema_version: i64,
    /// The Chipload version that wrote the file.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The name of the profile new projects start with. Null: the built-in values.
    #[serde(default)]
    pub default_machine: Option<String>,
    /// The profiles.
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

    /// The profile with this uid.
    pub fn find_uid(&self, uid: &str) -> Option<&MachineProfile> {
        self.machines.iter().find(|m| m.uid == uid)
    }

    /// A uid for a new profile named `name`.
    pub fn new_uid(&self, name: &str) -> String {
        fresh_uid(name, self.machines.iter().map(|m| m.uid.as_str()))
    }

    /// The profile a new project starts with, if one is set.
    pub fn default_entry(&self) -> Option<&MachineProfile> {
        self.default_machine.as_deref().and_then(|n| self.find(n))
    }

    /// Remove a profile; the default is cleared if it was that profile.
    pub fn remove(&mut self, name: &str) -> Result<MachineProfile, LibraryError> {
        let i = self
            .position(name)
            .ok_or_else(|| refused(Message::new("job-library-no-machine").arg("name", name)))?;
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
        problems.extend(uid_problems(
            "machines",
            self.machines.iter().map(|m| m.uid.as_str()),
        ));
        for (i, profile) in self.machines.iter().enumerate() {
            let at = format!("machines.{i}");
            problems.extend(
                profile
                    .machine()
                    .problems(&at)
                    .iter()
                    .map(ToString::to_string),
            );
            problems.extend(
                max_overcut_problems(&at, profile.max_overcut)
                    .iter()
                    .map(ToString::to_string),
            );
            problems.extend(
                crate::models::work_zero_problems(&format!("{at}.work_zero"), profile.work_zero)
                    .iter()
                    .map(ToString::to_string),
            );
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

/// Machines 2 -> 3: each profile gets the uid of its name.
pub fn machines_v2_to_v3(data: &mut serde_json::Map<String, Value>) {
    add_uids(data, "machines");
}

/// Machines 3 -> 4: each profile says its G-code units; every older one wrote mm (G21).
pub fn machines_v3_to_v4(data: &mut serde_json::Map<String, Value>) {
    if let Some(Value::Array(machines)) = data.get_mut("machines") {
        for machine in machines.iter_mut().filter_map(Value::as_object_mut) {
            machine.insert("gcode_units".into(), Value::from("mm"));
        }
    }
    data.insert("schema_version".into(), Value::from(4));
}

impl LibraryFile for MachineLibrary {
    const VERSION: i64 = MACHINES_VERSION;
    const OLDEST: i64 = 1;
    fn validate(&self) -> Vec<String> {
        MachineLibrary::validate(self)
    }
    fn stamp(&mut self) {
        self.schema_version = MACHINES_VERSION;
        self.written_by = Some(written_by());
    }
    fn upgrade(from: i64, data: &mut serde_json::Map<String, Value>) {
        match from {
            1 => machines_v1_to_v2(data),
            2 => machines_v2_to_v3(data),
            3 => machines_v3_to_v4(data),
            _ => {}
        }
    }
}

/// Machines 1 -> 2: the tool change setting; every profile gets "files", the only way
/// there was (docs/specs/tool-per-operation.md §3.3).
pub fn machines_v1_to_v2(data: &mut serde_json::Map<String, Value>) {
    if let Some(Value::Array(machines)) = data.get_mut("machines") {
        for machine in machines.iter_mut().filter_map(Value::as_object_mut) {
            machine.insert("tool_change".into(), Value::from("files"));
        }
    }
    data.insert("schema_version".into(), Value::from(2));
}
