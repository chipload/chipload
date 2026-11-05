// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's styles (docs/specs/styles.md §3, §4): operations saved by name, to use again
//! on a selection. A persisted library file, `styles.json`, beside `tools.json`. Each style
//! keeps its operation as a project of `operation_schema` stores it, read through the
//! project migrations when that is older, so one style that can't be read never stops the
//! others.

use std::path::Path;

use chipload_i18n::{Message, Value as I18nValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::library::{
    Changes, Collection, Entry, LibraryError, LibraryFile, ToolLibrary, fresh_uid, is_uid,
    load_library, name_problems, position, refused, save_library, uid_problems, written_by,
};
use crate::{CURRENT_VERSION, Job, Operation, Selector, Side};

pub const STYLES_FILE: &str = "styles.json";
/// Current `schema_version` of `styles.json`.
pub const STYLES_VERSION: i64 = 1;

/// A library tool a style cuts with, by the id its stored operation gives it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct StyleTool {
    /// The id the stored operation names it by (not a project's: it is mapped when used).
    pub id: i64,
    /// The library tool's uid.
    pub uid: String,
    /// The library tool's name when the style was saved, to name it when it is missing.
    pub name: String,
}

/// Where a profile style cuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StyleSide {
    /// Outside outlines, inside holes, as a new profile does (docs/specs/styles.md §2.1).
    BySelection,
    /// The side the operation has (on the line).
    AsSaved,
}

/// One style.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Style {
    /// The style's name, unique in the library (upper or lower case don't matter).
    pub name: String,
    /// 16 lowercase hex digits, never changed.
    pub uid: String,
    /// The tools its operation names: its own first, then its passes'.
    pub tools: Vec<StyleTool>,
    /// A profile's side; null for other kinds.
    #[serde(default)]
    pub side: Option<StyleSide>,
    /// The operation as a project stores it. Its shapes, name and part are not used, and
    /// its feeds only when the tool has no cutting data.
    pub operation: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct StyleLibrary {
    /// The version of the file format.
    pub schema_version: i64,
    /// The Chipload version that wrote the file.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The project schema the styles' operations are written in.
    pub operation_schema: i64,
    /// The styles, in the order they are listed.
    pub styles: Vec<Style>,
}

impl Default for StyleLibrary {
    fn default() -> Self {
        StyleLibrary {
            schema_version: STYLES_VERSION,
            written_by: None,
            operation_schema: CURRENT_VERSION,
            styles: Vec::new(),
        }
    }
}

impl Style {
    /// The style's operation. Refused, by the style's name, when it doesn't read as one.
    pub fn operation(&self) -> Result<Operation, LibraryError> {
        self.read()
            .map_err(|reason| refused(self.unreadable(reason)))
    }

    fn read(&self) -> Result<Operation, String> {
        serde_json::from_value(self.operation.clone()).map_err(|e| e.to_string())
    }

    /// "Style NAME can't be read: REASON".
    fn unreadable(&self, reason: String) -> Message {
        Message::new("job-style-unreadable")
            .arg("name", self.name.as_str())
            .arg("reason", reason)
    }

    /// The tool its operation cuts with.
    pub fn tool(&self) -> Option<&StyleTool> {
        self.tools.first()
    }
}

impl StyleLibrary {
    /// The index of the style with this name, ignoring case.
    pub fn position(&self, name: &str) -> Option<usize> {
        position(self.styles.iter().map(|s| s.name.as_str()), name)
    }

    pub fn find(&self, name: &str) -> Option<&Style> {
        self.position(name).map(|i| &self.styles[i])
    }

    /// The style with this name; refused when there is none.
    pub fn get(&self, name: &str) -> Result<&Style, LibraryError> {
        self.find(name).ok_or_else(|| no_style(name))
    }

    /// The style with this name, to change; refused when there is none.
    pub fn find_mut(&mut self, name: &str) -> Result<&mut Style, LibraryError> {
        let i = self.position(name).ok_or_else(|| no_style(name))?;
        Ok(&mut self.styles[i])
    }

    /// Add `style` under its name, or put it in place of the style of that name with
    /// `replace` (which keeps that one's uid and place). Refused when the name is taken.
    pub fn put(&mut self, mut style: Style, replace: bool) -> Result<(), LibraryError> {
        match self.position(&style.name) {
            Some(i) if replace => {
                style.uid = self.styles[i].uid.clone();
                self.styles[i] = style;
            }
            Some(_) => {
                return Err(refused(
                    Message::new("job-style-exists").arg("name", style.name.as_str()),
                ));
            }
            None => {
                style.uid = fresh_uid(&style.name, self.styles.iter().map(|s| s.uid.as_str()));
                self.styles.push(style);
            }
        }
        Ok(())
    }

    /// Rename a style; refused when the new name is taken by another.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), LibraryError> {
        let new = new.trim();
        let i = self.position(old).ok_or_else(|| no_style(old))?;
        if self.position(new).is_some_and(|j| j != i) {
            return Err(refused(Message::new("job-style-exists").arg("name", new)));
        }
        self.styles[i].name = new.to_string();
        Ok(())
    }

    /// Remove a style.
    pub fn remove(&mut self, name: &str) -> Result<Style, LibraryError> {
        let i = self.position(name).ok_or_else(|| no_style(name))?;
        Ok(self.styles.remove(i))
    }

    /// Every rule violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = name_problems(
            "styles",
            self.styles.iter().map(|s| s.name.as_str()),
            None,
            "",
        );
        problems.extend(uid_problems(
            "styles",
            self.styles.iter().map(|s| s.uid.as_str()),
        ));
        if self.operation_schema > CURRENT_VERSION {
            problems.push(format!(
                "operation_schema: {} is newer than this Chipload knows ({CURRENT_VERSION})",
                self.operation_schema
            ));
        }
        for (i, style) in self.styles.iter().enumerate() {
            if style.tools.is_empty() {
                problems.push(format!("styles.{i}.tools: must name the operation's tool"));
            }
            for (k, tool) in style.tools.iter().enumerate() {
                if !is_uid(&tool.uid) {
                    problems.push(format!(
                        "styles.{i}.tools.{k}.uid: must be 16 lowercase hex digits"
                    ));
                }
            }
        }
        problems
    }
}

fn no_style(name: &str) -> LibraryError {
    refused(Message::new("job-style-none").arg("name", name))
}

impl LibraryFile for StyleLibrary {
    const VERSION: i64 = STYLES_VERSION;
    fn validate(&self) -> Vec<String> {
        StyleLibrary::validate(self)
    }
    // `operation_schema` stays as it is: only `load_styles` raises it, after upgrading the
    // operations.
    fn stamp(&mut self) {
        self.schema_version = STYLES_VERSION;
        self.written_by = Some(written_by());
    }
}

/// A style's operation, written in project schema `from`, upgraded to the current one
/// through the project migrations, wrapped in a project of its own (docs/specs/styles.md
/// §4). The reason, in English, when a migration can't carry it.
pub fn upgrade_operation(from: i64, operation: &Value) -> Result<Value, String> {
    let wrapped = serde_json::json!({
        "schema_version": from,
        "operations": [operation],
    });
    let migrated = crate::migrations::migrate(wrapped).map_err(|e| e.to_string())?;
    migrated
        .get("operations")
        .and_then(|ops| ops.get(0))
        .cloned()
        .ok_or_else(|| "the migration dropped the operation".to_string())
}

/// What loading `styles.json` found: the styles, and the ones that couldn't be read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoadedStyles {
    pub library: StyleLibrary,
    /// Styles left out, each with why, by name.
    pub refused: Vec<Message>,
}

/// Load `styles.json` (a missing file: no styles). Styles written in an older project
/// schema are upgraded and the file saved again, with a `.bak`; a style that can't be is
/// left out and named in `refused`, and every other one loads.
pub fn load_styles(path: &Path) -> Result<LoadedStyles, LibraryError> {
    let mut library: StyleLibrary = load_library(path)?;
    let mut refused_styles = Vec::new();
    let from = library.operation_schema;
    let upgrade = from < CURRENT_VERSION;
    let mut kept = Vec::new();
    for mut style in std::mem::take(&mut library.styles) {
        if upgrade {
            match upgrade_operation(from, &style.operation) {
                Ok(operation) => style.operation = operation,
                Err(reason) => {
                    refused_styles.push(style.unreadable(reason));
                    continue;
                }
            }
        }
        match style.read() {
            Ok(_) => kept.push(style),
            Err(reason) => refused_styles.push(style.unreadable(reason)),
        }
    }
    library.styles = kept;
    if upgrade {
        library.operation_schema = CURRENT_VERSION;
        save_library(&library, path)?;
    }
    Ok(LoadedStyles {
        library,
        refused: refused_styles,
    })
}

/// Operation `index` of `job` (0-based) as a style named `name` (docs/specs/styles.md
/// §3.1): every tool it names must be in `tools`, the library; its shapes, start points,
/// tabs and corners placed by hand, name, part and material are left out.
pub fn style_of(
    job: &Job,
    index: usize,
    name: &str,
    tools: &ToolLibrary,
) -> Result<Style, LibraryError> {
    let Some(op) = job.operations.get(index) else {
        return Err(refused(
            Message::new("job-style-no-operation")
                .arg("number", i64::try_from(index + 1).unwrap_or(i64::MAX)),
        ));
    };
    let mut style_tools = Vec::new();
    for id in op.tool_ids() {
        let entry = job
            .tool(id)
            .and_then(|t| t.library_uid.as_deref())
            .and_then(|uid| tools.find_uid(uid))
            .ok_or_else(|| {
                refused(Message::new("job-style-tool-not-in-library").arg("tool", id))
            })?;
        style_tools.push(StyleTool {
            id,
            uid: entry.uid.clone(),
            name: entry.name.clone(),
        });
    }
    let mut op = op.clone();
    *op.name_mut() = String::new();
    op.set_part(1);
    op.set_enabled(true);
    op.set_material(None);
    if let Some(select) = op.select_mut() {
        *select = Selector::default();
    }
    if let Some(starts) = op.starts_mut() {
        starts.clear();
    }
    if let Some(points) = op.tab_points_mut() {
        points.clear();
    }
    if let Some(choices) = op.corner_choices_mut() {
        choices.clear();
    }
    let side = side_of(&op);
    let operation = serde_json::to_value(&op).map_err(|e| {
        refused(
            Message::new("job-style-unreadable")
                .arg("name", name)
                .arg("reason", e.to_string()),
        )
    })?;
    Ok(Style {
        name: name.trim().to_string(),
        uid: String::new(),
        tools: style_tools,
        side,
        operation,
    })
}

/// Where a style with operation `op` cuts: a profile on the line keeps it, inside or
/// outside go by the selection; other kinds have no side.
pub fn side_of(op: &Operation) -> Option<StyleSide> {
    match op {
        Operation::Profile(p) if p.side == Side::On => Some(StyleSide::AsSaved),
        Operation::Profile(_) => Some(StyleSide::BySelection),
        _ => None,
    }
}

/// Export and import as for tools (docs/specs/library-import-export.md).
impl Collection for StyleLibrary {
    type Entry = Style;
    const LIST: &'static str = "styles";
    const ENTRY: &'static str = "style";
    const OTHER: (&'static str, &'static str) = ("tools", "job-library-holds-tools");
    fn entries(&self) -> &[Style] {
        &self.styles
    }
    fn entries_mut(&mut self) -> &mut Vec<Style> {
        &mut self.styles
    }
    fn default_name(&self) -> Option<&str> {
        None
    }
    fn set_default_name(&mut self, _name: Option<String>) {}
}

impl Entry for Style {
    fn name(&self) -> &str {
        &self.name
    }
    fn renamed(&self, name: &str) -> Self {
        Style {
            name: name.to_string(),
            ..self.clone()
        }
    }
    fn uid(&self) -> &str {
        &self.uid
    }
    fn with_uid(&self, uid: &str) -> Self {
        Style {
            uid: uid.to_string(),
            ..self.clone()
        }
    }
    fn differences(&self, other: &Self) -> Vec<Message> {
        let mut changes = Changes(Vec::new());
        let tool = |s: &Style| I18nValue::Text(s.tool().map_or(String::new(), |t| t.name.clone()));
        changes.differs("style-tool", tool(self), tool(other));
        if self.operation != other.operation || self.side != other.side || self.tools != other.tools
        {
            changes.differs(
                "style-settings",
                Message::new("job-style-these-settings").into(),
                Message::new("job-style-other-settings").into(),
            );
        }
        changes.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryTool;

    fn corpus(file: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus")
            .join(file)
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chipload-styles-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A project of the frozen golden `style_use`, and a library holding its tool 2.
    fn project() -> (Job, ToolLibrary) {
        let job = crate::load_job(&corpus("jobs/v35/style_use.chipload")).unwrap();
        let tool = job.tool(2).unwrap().clone();
        let mut library = ToolLibrary::default();
        let uid = tool.library_uid.clone().unwrap();
        library
            .tools
            .push(LibraryTool::from_tool("6 mm 2F", &uid, &tool, Vec::new()));
        (job, library)
    }

    #[test]
    fn a_style_keeps_the_settings_but_not_the_shapes_or_the_name() {
        let (mut job, library) = project();
        if let Operation::Profile(p) = &mut job.operations[2] {
            p.starts.push([1.0, 2.0]);
            p.tabs.as_mut().unwrap().at.push([3.0, 4.0]);
        }
        let style = style_of(&job, 2, " Cut out ", &library).unwrap();
        assert_eq!(style.name, "Cut out");
        assert_eq!(style.side, Some(StyleSide::BySelection));
        assert_eq!(style.tools.len(), 1);
        assert_eq!(style.tools[0].name, "6 mm 2F");
        let Operation::Profile(p) = style.operation().unwrap() else {
            panic!("a profile")
        };
        assert!(p.select.is_empty() && p.starts.is_empty() && p.name.is_empty());
        assert!(p.tabs.unwrap().at.is_empty());
        assert_eq!(p.through, Some(0.5));
        // A tool not in the library: refused, by its number.
        let err = style_of(&job, 2, "x", &ToolLibrary::default()).unwrap_err();
        assert!(err.message().english().contains("tool 2"), "{err}");
        assert!(style_of(&job, 9, "x", &library).is_err());
    }

    #[test]
    fn names_are_unique_and_replace_keeps_the_place_and_uid() {
        let (job, library) = project();
        let mut styles = StyleLibrary::default();
        styles
            .put(style_of(&job, 0, "Pocket", &library).unwrap(), false)
            .unwrap();
        styles
            .put(style_of(&job, 2, "Cut", &library).unwrap(), false)
            .unwrap();
        assert!(
            styles
                .put(style_of(&job, 2, "pocket", &library).unwrap(), false)
                .is_err()
        );
        let uid = styles.styles[0].uid.clone();
        styles
            .put(style_of(&job, 2, "pocket", &library).unwrap(), true)
            .unwrap();
        assert_eq!(styles.styles[0].uid, uid);
        assert!(matches!(
            styles.styles[0].operation().unwrap(),
            Operation::Profile(_)
        ));
        assert!(styles.rename("Cut", "POCKET").is_err());
        styles.rename("Cut", "Through").unwrap();
        assert_eq!(styles.remove("through").unwrap().name, "Through");
        assert!(styles.validate().is_empty(), "{:?}", styles.validate());
    }

    #[test]
    fn the_v1_sample_loads() {
        // A copy: loading saves an upgraded file, and the frozen sample must stay as it is.
        let dir = std::env::temp_dir().join(format!("chipload-styles-v1-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("styles.json");
        std::fs::copy(corpus("styles/v1/styles.json"), &copy).unwrap();
        let loaded = load_styles(&copy).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
        let names: Vec<&str> = loaded
            .library
            .styles
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(names, ["Pocket 3 mm", "Cut out"]);
        assert!(matches!(
            loaded.library.styles[0].operation().unwrap(),
            Operation::Pocket(_)
        ));
    }

    #[test]
    fn older_operations_go_through_the_project_migrations_and_bad_ones_are_left_out() {
        // A V-carve as schema 34 wrote it: a step down, which schema 35 makes passes of.
        let job: Value = serde_json::from_str(
            &std::fs::read_to_string(corpus("jobs/v34/vcarve_step_down.chipload")).unwrap(),
        )
        .unwrap();
        let vcarve = job["operations"][0].clone();
        let file = serde_json::json!({
            "schema_version": 1,
            "operation_schema": 34,
            "styles": [
                {"name": "Sign", "uid": "00000000000000a1", "side": null,
                 "tools": [{"id": 1, "uid": "00000000000000b1", "name": "V 60"}],
                 "operation": vcarve},
                {"name": "Broken", "uid": "00000000000000a2", "side": null,
                 "tools": [{"id": 1, "uid": "00000000000000b1", "name": "V 60"}],
                 "operation": {"type": "nonsense"}}
            ]
        });
        let dir = temp("migrate");
        let path = dir.join(STYLES_FILE);
        std::fs::write(&path, file.to_string()).unwrap();
        let loaded = load_styles(&path).unwrap();
        assert_eq!(loaded.library.styles.len(), 1);
        assert_eq!(loaded.refused.len(), 1);
        assert!(loaded.refused[0].english().contains("Broken"));
        let Operation::Vcarve(v) = loaded.library.styles[0].operation().unwrap() else {
            panic!("a V-carve")
        };
        assert_eq!(v.passes, 4, "3.5 mm in steps of at most 1 mm");
        // Saved again at the current schema, the old file kept.
        let saved: StyleLibrary = load_library(&path).unwrap();
        assert_eq!(saved.operation_schema, CURRENT_VERSION);
        assert!(dir.join("styles.json.bak").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_with_newer_operations_is_refused() {
        let dir = temp("newer");
        let path = dir.join(STYLES_FILE);
        let file = serde_json::json!({
            "schema_version": 1, "operation_schema": CURRENT_VERSION + 1, "styles": []
        });
        std::fs::write(&path, file.to_string()).unwrap();
        assert!(load_styles(&path).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
