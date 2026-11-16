// SPDX-License-Identifier: GPL-3.0-or-later
//! The tool library: tools with their cutting data per material.

use chipload_i18n::Message;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::exchange::refused;
use super::files::LibraryFile;
use super::files::written_by;
use super::machines::{Changes, mm, optional};
use super::machines::{degrees, number, yes_no};
use super::names::add_uids;
use super::names::{fresh_uid, name_problems, position, same_name, uid_problems};
use super::{LibraryError, TOOLS_VERSION};
use crate::models::{Tool, ToolType};

/// One tested way to cut with a tool: the values an operation takes from a row of the
/// tool's cutting data (docs/specs/cutting-data.md §2, §3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Cutting {
    /// Spindle speed, rpm. Null: the tool's own.
    #[serde(default)]
    pub rpm: Option<f64>,
    /// Plunge feed, mm/min.
    pub plunge_feed: f64,
    /// The feed of a full-width cut (a profile, a pocket's first ring), mm/min.
    pub slot_feed: f64,
    /// The step down of a full-width cut, mm.
    pub slot_step_down: f64,
    /// The feed of a cut at a stepover (a pocket's rings), mm/min.
    pub feed: f64,
    /// The step down of a cut at a stepover, mm.
    pub step_down: f64,
    /// Pocket stepover, a fraction of the tool diameter: more than 0, at most 1.
    pub stepover: f64,
}

impl Cutting {
    /// Every rule violation as "path: message", `at` naming these values.
    pub fn problems(&self, at: &str) -> Vec<String> {
        let mut problems = Vec::new();
        for (field, value) in [
            ("plunge_feed", self.plunge_feed),
            ("slot_feed", self.slot_feed),
            ("slot_step_down", self.slot_step_down),
            ("feed", self.feed),
            ("step_down", self.step_down),
            ("stepover", self.stepover),
        ]
        .into_iter()
        .chain(self.rpm.map(|rpm| ("rpm", rpm)))
        {
            if value <= 0.0 || value.is_nan() {
                problems.push(format!("{at}.{field}: must be greater than 0, got {value}"));
            }
        }
        if self.stepover > 1.0 {
            problems.push(format!(
                "{at}.stepover: must be at most 1 (a fraction of the tool diameter), got {}",
                self.stepover
            ));
        }
        problems
    }

    /// One set of values for both kinds of cut, as an operation's values were before slot
    /// and clearing (docs/specs/cutting-data.md §8).
    pub fn uniform(feed: f64, plunge_feed: f64, step_down: f64, stepover: f64) -> Cutting {
        Cutting {
            rpm: None,
            plunge_feed,
            slot_feed: feed,
            slot_step_down: step_down,
            feed,
            step_down,
            stepover,
        }
    }
}

/// A row of a tool's cutting data: the values for one material, or for any material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CuttingRow {
    /// The material's uid (`materials.json`). Null: any material, the row used without a
    /// material or when the tool has no row for it.
    #[serde(default)]
    pub material_uid: Option<String>,
    /// The values.
    pub values: Cutting,
}

/// One tool of the library: a project tool with a name and optional cutting values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LibraryTool {
    /// The tool's name, unique in the library (upper or lower case don't matter).
    pub name: String,
    /// What projects link the tool by: 16 lowercase hex digits, unique in the library, never
    /// changed (docs/specs/library-in-browser.md §2).
    pub uid: String,
    /// The tool number it gets in a project, when that number is free there.
    pub id: i64,
    /// `endmill`, `drill` or `vbit`.
    #[serde(default, rename = "type")]
    pub kind: ToolType,
    /// Diameter, mm.
    pub diameter: f64,
    /// Number of flutes. Null: unknown.
    #[serde(default)]
    pub flutes: Option<i64>,
    /// Whether it can plunge straight down. Null: unknown.
    #[serde(default)]
    pub center_cutting: Option<bool>,
    /// Spindle speed, rpm. Null: unknown.
    #[serde(default)]
    pub rpm: Option<f64>,
    /// The cutting data: one row per material, and one for any material
    /// (docs/specs/cutting-data.md §2). What an operation takes when a material is chosen,
    /// its tool changes or it is added. Missing: none.
    #[serde(default)]
    pub cutting_data: Vec<CuttingRow>,
    /// V-bits: the included angle, radians. Null: unknown.
    #[serde(default)]
    pub angle: Option<f64>,
    /// V-bits: the flat tip's diameter, mm. Null: pointed.
    #[serde(default)]
    pub tip_diameter: Option<f64>,
    /// End mills: the radius of its corners, mm (docs/specs/corner-radius.md §2). Missing:
    /// square corners; left out of the file then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,
}

impl LibraryTool {
    /// A library entry made from a project tool, with the given uid.
    pub fn from_tool(
        name: &str,
        uid: &str,
        tool: &Tool,
        cutting_data: Vec<CuttingRow>,
    ) -> LibraryTool {
        LibraryTool {
            name: name.to_string(),
            uid: uid.to_string(),
            id: tool.id,
            kind: tool.kind,
            diameter: tool.diameter,
            flutes: tool.flutes,
            center_cutting: tool.center_cutting,
            rpm: tool.rpm,
            cutting_data,
            angle: tool.angle,
            tip_diameter: tool.tip_diameter,
            corner_radius: tool.corner_radius,
        }
    }

    /// The project tool this entry becomes, with the given id, linked to the entry.
    pub fn tool(&self, id: i64) -> Tool {
        Tool {
            id,
            name: Some(self.name.clone()),
            library_uid: Some(self.uid.clone()),
            kind: self.kind,
            diameter: self.diameter,
            flutes: self.flutes,
            center_cutting: self.center_cutting,
            rpm: self.rpm,
            angle: self.angle,
            tip_diameter: self.tip_diameter,
            corner_radius: self.corner_radius,
        }
    }

    /// The row for `material` (its uid; None: the row for any material), if there is one.
    pub fn row(&self, material: Option<&str>) -> Option<&CuttingRow> {
        self.cutting_data
            .iter()
            .find(|r| r.material_uid.as_deref() == material)
    }

    /// The values to cut `material` with (None: no material): its row, else the row for any
    /// material (docs/specs/cutting-data.md §2).
    pub fn cutting_for(&self, material: Option<&str>) -> Option<Cutting> {
        self.row(material)
            .or_else(|| self.row(None))
            .map(|r| r.values)
    }

    /// Set (or replace) the row for `material` (None: any material).
    pub fn set_row(&mut self, material: Option<&str>, values: Cutting) {
        let row = CuttingRow {
            material_uid: material.map(str::to_string),
            values,
        };
        match self
            .cutting_data
            .iter_mut()
            .find(|r| r.material_uid.as_deref() == material)
        {
            Some(existing) => *existing = row,
            None => self.cutting_data.push(row),
        }
    }

    /// Remove the row for `material`; whether there was one.
    pub fn remove_row(&mut self, material: Option<&str>) -> bool {
        let before = self.cutting_data.len();
        self.cutting_data
            .retain(|r| r.material_uid.as_deref() != material);
        self.cutting_data.len() != before
    }

    /// What replacing a project tool's values with this entry would change, one message
    /// per value ("diameter 6 mm -> 8 mm"); empty when they match (id and name aside).
    pub fn changes(&self, tool: &Tool) -> Vec<Message> {
        let mut changes = Changes::default();
        changes.differs(
            "type",
            ToolType::name(tool.kind).into(),
            ToolType::name(self.kind).into(),
        );
        changes.differs("diameter", mm(tool.diameter), mm(self.diameter));
        changes.differs(
            "flutes",
            optional(tool.flutes.map(|f| f.to_string().into())),
            optional(self.flutes.map(|f| f.to_string().into())),
        );
        changes.differs(
            "centre-cutting",
            yes_no(tool.center_cutting),
            yes_no(self.center_cutting),
        );
        changes.differs(
            "rpm",
            optional(tool.rpm.map(number)),
            optional(self.rpm.map(number)),
        );
        changes.differs(
            "angle",
            optional(tool.angle.map(degrees)),
            optional(self.angle.map(degrees)),
        );
        changes.differs(
            "tip",
            optional(tool.tip_diameter.map(mm)),
            optional(self.tip_diameter.map(mm)),
        );
        changes.differs(
            "corner-radius",
            optional(tool.corner_radius.map(mm)),
            optional(self.corner_radius.map(mm)),
        );
        changes.0
    }

    /// Whether a project tool has this entry's values (id, name and link aside).
    pub fn same_values(&self, tool: &Tool) -> bool {
        self.tool(tool.id)
            == Tool {
                name: Some(self.name.clone()),
                library_uid: Some(self.uid.clone()),
                ..tool.clone()
            }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ToolLibrary {
    /// The version of the file format.
    pub schema_version: i64,
    /// The Chipload version that wrote the file.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The name of the tool new projects start with. Null: the built-in 6 mm endmill.
    #[serde(default)]
    pub default_tool: Option<String>,
    /// The tools.
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
    /// Remove every tool's row for the material with this uid (the material was removed):
    /// the names of the tools that had one.
    pub fn forget_material(&mut self, uid: &str) -> Vec<String> {
        self.tools
            .iter_mut()
            .filter_map(|t| t.remove_row(Some(uid)).then(|| t.name.clone()))
            .collect()
    }

    /// The index of the tool with this name, ignoring case.
    pub fn position(&self, name: &str) -> Option<usize> {
        position(self.tools.iter().map(|t| t.name.as_str()), name)
    }

    pub fn find(&self, name: &str) -> Option<&LibraryTool> {
        self.position(name).map(|i| &self.tools[i])
    }

    /// The tool with this uid.
    pub fn find_uid(&self, uid: &str) -> Option<&LibraryTool> {
        self.tools.iter().find(|t| t.uid == uid)
    }

    /// A uid for a new tool named `name` (§2 of docs/specs/library-in-browser.md).
    pub fn new_uid(&self, name: &str) -> String {
        fresh_uid(name, self.tools.iter().map(|t| t.uid.as_str()))
    }

    /// The tool a new project starts with, if one is set.
    pub fn default_entry(&self) -> Option<&LibraryTool> {
        self.default_tool.as_deref().and_then(|n| self.find(n))
    }

    /// Remove a tool; the default is cleared if it was that tool.
    pub fn remove(&mut self, name: &str) -> Result<LibraryTool, LibraryError> {
        let i = self
            .position(name)
            .ok_or_else(|| refused(Message::new("job-library-no-tool").arg("name", name)))?;
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
        problems.extend(uid_problems(
            "tools",
            self.tools.iter().map(|t| t.uid.as_str()),
        ));
        for (i, entry) in self.tools.iter().enumerate() {
            let at = format!("tools.{i}");
            problems.extend(
                entry
                    .tool(entry.id)
                    .problems(&at)
                    .iter()
                    .map(ToString::to_string),
            );
            for (k, row) in entry.cutting_data.iter().enumerate() {
                let at = format!("{at}.cutting_data.{k}");
                problems.extend(row.values.problems(&format!("{at}.values")));
                if entry.cutting_data[..k]
                    .iter()
                    .any(|r| r.material_uid == row.material_uid)
                {
                    problems.push(format!("{at}.material_uid: the material has two rows"));
                }
            }
        }
        problems
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
        match from {
            1 => tools_v1_to_v2(data),
            2 => tools_v2_to_v3(data),
            3 => tools_v3_to_v4(data),
            _ => {}
        }
    }
}

/// Tools 1 -> 2: V-bits may have an angle and a tip; existing ones get neither (unknown).
pub fn tools_v1_to_v2(data: &mut serde_json::Map<String, Value>) {
    data.insert("schema_version".into(), Value::from(2));
}

/// Tools 2 -> 3: each tool gets the uid of its name, as projects 22 -> 23 do for the tools
/// they copied, so they stay linked.
pub fn tools_v2_to_v3(data: &mut serde_json::Map<String, Value>) {
    add_uids(data, "tools");
}

/// Tools 3 -> 4: a tool's one set of values (`cutting`) becomes its row for any material,
/// with the same values for full-width cuts and cuts at a stepover, so nothing cuts
/// differently (docs/specs/cutting-data.md §8). A tool without values gets no row.
pub fn tools_v3_to_v4(data: &mut serde_json::Map<String, Value>) {
    if let Some(Value::Array(tools)) = data.get_mut("tools") {
        for tool in tools {
            if let Value::Object(tool) = tool {
                let rows = match tool.remove("cutting") {
                    Some(Value::Object(old)) => vec![uniform_row(Value::Null, &old)],
                    _ => Vec::new(),
                };
                tool.insert("cutting_data".into(), Value::Array(rows));
            }
        }
    }
    data.insert("schema_version".into(), Value::from(4));
}

/// A cutting data row for `material` from one old set of values (`feed`, `plunge_feed`,
/// `step_down`, `stepover`): slot and clearing alike, the tool's own speed.
pub(crate) fn uniform_row(material: Value, old: &serde_json::Map<String, Value>) -> Value {
    let value = |key: &str| old.get(key).cloned().unwrap_or(Value::Null);
    serde_json::json!({
        "material_uid": material,
        "values": {
            "rpm": null,
            "plunge_feed": value("plunge_feed"),
            "slot_feed": value("feed"),
            "slot_step_down": value("step_down"),
            "feed": value("feed"),
            "step_down": value("step_down"),
            "stepover": value("stepover"),
        },
    })
}
