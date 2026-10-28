// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's materials (docs/specs/materials.md): for each material, the feeds, step down
//! and stepover the user has found good for each library tool. A persisted library file,
//! `materials.json`, loaded and saved as `tools.json` is.

use chipload_i18n::Message;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::library::{
    Changes, Collection, Cutting, Entry, LibraryError, LibraryFile, cutting_text, fresh_uid,
    name_problems, position, refused, uid_problems, written_by,
};

pub const MATERIALS_FILE: &str = "materials.json";
/// Current `schema_version` of `materials.json`.
pub const MATERIALS_VERSION: i64 = 2;

/// One tool's values in a material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MaterialTool {
    /// The library tool's uid.
    pub tool_uid: String,
    /// Cutting feed, mm/min.
    pub feed: f64,
    /// Plunge feed, mm/min.
    pub plunge_feed: f64,
    /// Step down, mm.
    pub step_down: f64,
    /// Pocket stepover, a fraction of the tool diameter: more than 0, at most 1.
    pub stepover: f64,
}

impl MaterialTool {
    pub fn cutting(&self) -> Cutting {
        Cutting {
            feed: self.feed,
            plunge_feed: self.plunge_feed,
            step_down: self.step_down,
            stepover: self.stepover,
        }
    }
}

/// A material: a name and the user's values per tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Material {
    /// The material's name, unique in the library (upper or lower case don't matter).
    pub name: String,
    /// What projects name the material by: 16 lowercase hex digits, never changed.
    pub uid: String,
    /// Values for the tools used with this material, one entry per library tool.
    pub tools: Vec<MaterialTool>,
}

impl Material {
    /// The values for the library tool with this uid.
    pub fn values(&self, tool_uid: &str) -> Option<Cutting> {
        self.tools
            .iter()
            .find(|t| t.tool_uid == tool_uid)
            .map(MaterialTool::cutting)
    }

    /// Set (or replace) a tool's values.
    pub fn set_values(&mut self, tool_uid: &str, c: Cutting) {
        let entry = MaterialTool {
            tool_uid: tool_uid.to_string(),
            feed: c.feed,
            plunge_feed: c.plunge_feed,
            step_down: c.step_down,
            stepover: c.stepover,
        };
        match self.tools.iter_mut().find(|t| t.tool_uid == tool_uid) {
            Some(existing) => *existing = entry,
            None => self.tools.push(entry),
        }
    }

    /// Remove a tool's values; whether it had any.
    pub fn remove_values(&mut self, tool_uid: &str) -> bool {
        let before = self.tools.len();
        self.tools.retain(|t| t.tool_uid != tool_uid);
        self.tools.len() != before
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MaterialLibrary {
    /// The version of the file format.
    pub schema_version: i64,
    /// The Chipload version that wrote the file.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The materials.
    pub materials: Vec<Material>,
}

impl Default for MaterialLibrary {
    fn default() -> Self {
        MaterialLibrary {
            schema_version: MATERIALS_VERSION,
            written_by: None,
            materials: Vec::new(),
        }
    }
}

impl MaterialLibrary {
    /// The index of the material with this name, ignoring case.
    pub fn position(&self, name: &str) -> Option<usize> {
        position(self.materials.iter().map(|m| m.name.as_str()), name)
    }

    pub fn find(&self, name: &str) -> Option<&Material> {
        self.position(name).map(|i| &self.materials[i])
    }

    /// The material with this name, to change; refused when there is none.
    pub fn find_mut(&mut self, name: &str) -> Result<&mut Material, LibraryError> {
        let i = self.position(name).ok_or_else(|| no_material(name))?;
        Ok(&mut self.materials[i])
    }

    /// The material with this uid.
    pub fn find_uid(&self, uid: &str) -> Option<&Material> {
        self.materials.iter().find(|m| m.uid == uid)
    }

    /// Add a new material with no values; refused when the name is taken.
    pub fn add(&mut self, name: &str) -> Result<&Material, LibraryError> {
        let name = name.trim();
        if self.position(name).is_some() {
            return Err(refused(
                Message::new("job-library-material-exists").arg("name", name),
            ));
        }
        let uid = fresh_uid(name, self.materials.iter().map(|m| m.uid.as_str()));
        self.materials.push(Material {
            name: name.to_string(),
            uid,
            tools: Vec::new(),
        });
        Ok(&self.materials[self.materials.len() - 1])
    }

    /// Remove a material.
    pub fn remove(&mut self, name: &str) -> Result<Material, LibraryError> {
        let i = self.position(name).ok_or_else(|| no_material(name))?;
        Ok(self.materials.remove(i))
    }

    /// Remove a library tool's values from every material: the names of those that had
    /// some.
    pub fn forget_tool(&mut self, tool_uid: &str) -> Vec<String> {
        self.materials
            .iter_mut()
            .filter_map(|m| m.remove_values(tool_uid).then(|| m.name.clone()))
            .collect()
    }

    /// Every rule violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = name_problems(
            "materials",
            self.materials.iter().map(|m| m.name.as_str()),
            None,
            "",
        );
        problems.extend(uid_problems(
            "materials",
            self.materials.iter().map(|m| m.uid.as_str()),
        ));
        for (i, material) in self.materials.iter().enumerate() {
            let at = format!("materials.{i}");
            for (k, tool) in material.tools.iter().enumerate() {
                let at = format!("{at}.tools.{k}");
                problems.extend(tool.cutting().problems(&at));
                if material.tools[..k]
                    .iter()
                    .any(|t| t.tool_uid == tool.tool_uid)
                {
                    problems.push(format!(
                        "{at}.tool_uid: \"{}\" has values twice",
                        tool.tool_uid
                    ));
                }
            }
        }
        problems
    }
}

fn no_material(name: &str) -> LibraryError {
    refused(Message::new("job-library-no-material").arg("name", name))
}

impl LibraryFile for MaterialLibrary {
    const VERSION: i64 = MATERIALS_VERSION;
    const OLDEST: i64 = 1;
    fn validate(&self) -> Vec<String> {
        MaterialLibrary::validate(self)
    }
    fn stamp(&mut self) {
        self.schema_version = MATERIALS_VERSION;
        self.written_by = Some(written_by());
    }
    fn upgrade(from: i64, data: &mut serde_json::Map<String, Value>) {
        if from == 1 {
            materials_v1_to_v2(data);
        }
    }
}

/// Materials 1 -> 2: a material no longer has a thickness. Choosing a material never
/// changes the stock (the user, 2026-10-28), so the value had no use; it changed no cut.
pub fn materials_v1_to_v2(data: &mut serde_json::Map<String, Value>) {
    if let Some(Value::Array(materials)) = data.get_mut("materials") {
        for material in materials {
            if let Value::Object(material) = material {
                material.remove("thickness");
            }
        }
    }
    data.insert("schema_version".into(), Value::from(2));
}

/// Export and import as for tools (docs/specs/library-import-export.md).
impl Collection for MaterialLibrary {
    type Entry = Material;
    const LIST: &'static str = "materials";
    const ENTRY: &'static str = "material";
    const OTHER: (&'static str, &'static str) = ("tools", "job-library-holds-tools");
    fn entries(&self) -> &[Material] {
        &self.materials
    }
    fn entries_mut(&mut self) -> &mut Vec<Material> {
        &mut self.materials
    }
    // Materials have no default.
    fn default_name(&self) -> Option<&str> {
        None
    }
    fn set_default_name(&mut self, _name: Option<String>) {}
}

impl Entry for Material {
    fn name(&self) -> &str {
        &self.name
    }
    fn renamed(&self, name: &str) -> Self {
        Material {
            name: name.to_string(),
            ..self.clone()
        }
    }
    fn uid(&self) -> &str {
        &self.uid
    }
    fn with_uid(&self, uid: &str) -> Self {
        Material {
            uid: uid.to_string(),
            ..self.clone()
        }
    }
    fn differences(&self, other: &Self) -> Vec<Message> {
        let mut changes = Changes(Vec::new());
        // One line per tool whose values differ, in the order they are listed.
        let uids = self
            .tools
            .iter()
            .chain(&other.tools)
            .map(|t| t.tool_uid.as_str());
        let mut seen: Vec<&str> = Vec::new();
        for uid in uids {
            if seen.contains(&uid) {
                continue;
            }
            seen.push(uid);
            let (mine, theirs) = (self.values(uid), other.values(uid));
            if mine != theirs {
                changes.differs(
                    "tool-values",
                    cutting_text(mine.as_ref()),
                    cutting_text(theirs.as_ref()),
                );
            }
        }
        changes.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{load_library, update_library};

    fn values(feed: f64) -> Cutting {
        Cutting {
            feed,
            plunge_feed: 500.0,
            step_down: 2.5,
            stepover: 0.4,
        }
    }

    #[test]
    fn materials_are_added_given_values_and_saved_and_read_back() {
        let dir = std::env::temp_dir().join(format!("chipload-materials-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(MATERIALS_FILE);
        // A missing file is an empty library.
        let empty: MaterialLibrary = load_library(&path).unwrap();
        assert!(empty.materials.is_empty());
        update_library(&path, |lib: &mut MaterialLibrary| {
            lib.add("Plywood")?;
            lib.find_mut("plywood")?
                .set_values("a1b2c3d4e5f60718", values(1500.0));
            Ok(())
        })
        .unwrap();
        let lib: MaterialLibrary = load_library(&path).unwrap();
        let ply = lib.find("PLYWOOD").unwrap();
        assert_eq!(ply.values("a1b2c3d4e5f60718"), Some(values(1500.0)));
        assert_eq!(lib.find_uid(&ply.uid), Some(ply));
        // Same name again: refused, nothing written.
        let again = update_library(&path, |lib: &mut MaterialLibrary| {
            lib.add("plywood").map(|_| ())
        });
        assert!(again.is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn values_are_replaced_removed_and_checked() {
        let mut lib = MaterialLibrary::default();
        lib.add("Oak").unwrap();
        let oak = lib.find_mut("Oak").unwrap();
        oak.set_values("a1b2c3d4e5f60718", values(1500.0));
        oak.set_values("a1b2c3d4e5f60718", values(1200.0));
        assert_eq!(oak.tools.len(), 1);
        assert_eq!(oak.values("a1b2c3d4e5f60718").unwrap().feed, 1200.0);
        assert!(lib.validate().is_empty());
        lib.materials[0].tools[0].stepover = 1.5;
        let problems = lib.validate().join("\n");
        assert!(
            problems.contains("materials.0.tools.0.stepover"),
            "{problems}"
        );
        lib.materials[0].tools[0].stepover = 0.4;
        assert_eq!(lib.forget_tool("a1b2c3d4e5f60718"), vec!["Oak".to_string()]);
        assert!(lib.materials[0].tools.is_empty());
        assert!(lib.remove("oak").is_ok());
        assert!(lib.remove("oak").is_err());
    }

    #[test]
    fn a_version_1_file_loses_its_thicknesses() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/materials/v1/materials.json");
        let lib: MaterialLibrary = load_library(&path).unwrap();
        assert_eq!(lib.schema_version, MATERIALS_VERSION);
        let ply = lib.find("Plywood").unwrap();
        assert_eq!(ply.values("a1b2c3d4e5f60718"), Some(values(1500.0)));
        assert!(lib.find("Oak").unwrap().tools.is_empty());
    }

    #[test]
    fn a_newer_file_is_refused() {
        let dir =
            std::env::temp_dir().join(format!("chipload-materials-new-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(MATERIALS_FILE);
        std::fs::write(&path, r#"{"schema_version": 3, "materials": []}"#).unwrap();
        let read: Result<MaterialLibrary, _> = load_library(&path);
        assert!(matches!(read, Err(LibraryError::UnsupportedVersion { .. })));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
