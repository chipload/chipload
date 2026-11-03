// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's materials (docs/specs/materials.md, docs/specs/cutting-data.md): names to
//! choose for a project or an operation. The values for each are rows of the tools' cutting
//! data. A persisted library file, `materials.json`, loaded and saved as `tools.json` is.

use chipload_i18n::Message;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::library::{
    Collection, Entry, LibraryError, LibraryFile, ToolLibrary, fresh_uid, name_problems, position,
    refused, uid_problems, uniform_row, written_by,
};

pub const MATERIALS_FILE: &str = "materials.json";
/// Current `schema_version` of `materials.json`. Version 3 moved the values per tool into
/// the tools' cutting data (docs/specs/cutting-data.md §8).
pub const MATERIALS_VERSION: i64 = 3;

/// A material: a name to choose. Its values are the tools' rows for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Material {
    /// The material's name, unique in the library (upper or lower case don't matter).
    pub name: String,
    /// What projects and the tools' rows name the material by: 16 lowercase hex digits,
    /// never changed.
    pub uid: String,
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
    /// The name of the material new projects start with. Null: none.
    // Spec: docs/specs/job-settings-layout.md §3 (optional, no version bump).
    #[serde(default)]
    pub default_material: Option<String>,
    /// The stock thickness new projects start with, mm. Left out: 10 mm.
    // Spec: the user, 2026-11-03 (optional, no version bump). Written only when set, so
    // 0.32, which refuses unknown fields, still reads a file that doesn't use it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_thickness: Option<f64>,
    /// The materials.
    pub materials: Vec<Material>,
}

impl Default for MaterialLibrary {
    fn default() -> Self {
        MaterialLibrary {
            schema_version: MATERIALS_VERSION,
            written_by: None,
            default_material: None,
            default_thickness: None,
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
        });
        Ok(&self.materials[self.materials.len() - 1])
    }

    /// The material a new project starts with, if one is set.
    pub fn default_entry(&self) -> Option<&Material> {
        self.default_material.as_deref().and_then(|n| self.find(n))
    }

    /// Rename a material, keeping it the default if it was.
    pub fn rename(&mut self, old: &str, new: &str) {
        if self
            .default_material
            .as_deref()
            .is_some_and(|d| crate::library::same_name(d, old))
        {
            self.default_material = Some(new.to_string());
        }
    }

    /// Remove a material; the default is cleared if it was that material.
    pub fn remove(&mut self, name: &str) -> Result<Material, LibraryError> {
        let i = self.position(name).ok_or_else(|| no_material(name))?;
        let removed = self.materials.remove(i);
        if self
            .default_material
            .as_deref()
            .is_some_and(|d| crate::library::same_name(d, &removed.name))
        {
            self.default_material = None;
        }
        Ok(removed)
    }

    /// Every rule violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = name_problems(
            "materials",
            self.materials.iter().map(|m| m.name.as_str()),
            self.default_material.as_deref(),
            "default_material",
        );
        problems.extend(uid_problems(
            "materials",
            self.materials.iter().map(|m| m.uid.as_str()),
        ));
        if let Some(t) = self.default_thickness
            && !(t.is_finite() && t > 0.0)
        {
            problems.push(format!(
                "default_thickness: must be more than 0 mm, got {t}"
            ));
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
        match from {
            1 => materials_v1_to_v2(data),
            2 => materials_v2_to_v3(data),
            _ => {}
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

/// Materials 2 -> 3: a material keeps its name and uid; its values per tool belong to the
/// tools now (docs/specs/cutting-data.md §8). They are moved there first, by
/// [`move_material_values`], when the user's library is converted; this step only drops them.
pub fn materials_v2_to_v3(data: &mut serde_json::Map<String, Value>) {
    if let Some(Value::Array(materials)) = data.get_mut("materials") {
        for material in materials {
            if let Value::Object(material) = material {
                material.remove("tools");
            }
        }
    }
    data.insert("schema_version".into(), Value::from(3));
}

/// The values of an old (version 1 or 2) `materials.json` moved into the tools' cutting
/// data (docs/specs/cutting-data.md §8): each material's values for a tool become that
/// tool's row for the material, the same for full-width cuts and cuts at a stepover, at
/// the tool's own speed. A tool that already has a row for the material keeps it, so
/// moving twice changes nothing. Pure: no I/O. One message per tool given rows, and per
/// value dropped because its tool isn't in the library.
pub fn move_material_values(tools: &mut ToolLibrary, old: &Value) -> Vec<Message> {
    let mut messages = Vec::new();
    let materials = old
        .get("materials")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    for material in materials {
        let (Some(uid), Some(name)) = (
            material.get("uid").and_then(Value::as_str),
            material.get("name").and_then(Value::as_str),
        ) else {
            continue;
        };
        let values = material
            .get("tools")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        for value in values {
            let Some(old) = value.as_object() else {
                continue;
            };
            let tool_uid = old.get("tool_uid").and_then(Value::as_str).unwrap_or("");
            let Some(tool) = tools.tools.iter_mut().find(|t| t.uid == tool_uid) else {
                messages.push(
                    Message::new("job-library-values-dropped")
                        .arg("material", name)
                        .arg("uid", tool_uid),
                );
                continue;
            };
            if tool.row(Some(uid)).is_some() {
                continue;
            }
            let row = uniform_row(Value::from(uid), old);
            match serde_json::from_value(row) {
                Ok(row) => {
                    tool.cutting_data.push(row);
                    messages.push(
                        Message::new("job-library-values-moved")
                            .arg("material", name)
                            .arg("tool", tool.name.as_str()),
                    );
                }
                Err(_) => messages.push(
                    Message::new("job-library-values-dropped")
                        .arg("material", name)
                        .arg("uid", tool_uid),
                ),
            }
        }
    }
    messages
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
    fn default_name(&self) -> Option<&str> {
        self.default_material.as_deref()
    }
    fn set_default_name(&mut self, name: Option<String>) {
        self.default_material = name;
    }
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
    fn differences(&self, _other: &Self) -> Vec<Message> {
        // A name and a uid: nothing else to differ.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{Cutting, LibraryTool, load_library, update_library};

    #[test]
    fn materials_are_added_saved_and_read_back() {
        let dir = std::env::temp_dir().join(format!("chipload-materials-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(MATERIALS_FILE);
        // A missing file is an empty library.
        let empty: MaterialLibrary = load_library(&path).unwrap();
        assert!(empty.materials.is_empty());
        update_library(&path, |lib: &mut MaterialLibrary| {
            lib.add("Plywood")?;
            Ok(())
        })
        .unwrap();
        let lib: MaterialLibrary = load_library(&path).unwrap();
        let ply = lib.find("PLYWOOD").unwrap();
        assert_eq!(lib.find_uid(&ply.uid), Some(ply));
        // Same name again: refused, nothing written.
        let again = update_library(&path, |lib: &mut MaterialLibrary| {
            lib.add("plywood").map(|_| ())
        });
        assert!(again.is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Without a default thickness the file is as 0.32 wrote it, so 0.32 still reads it.
    #[test]
    fn the_default_thickness_is_written_only_when_set() {
        let mut lib = MaterialLibrary::default();
        let json = serde_json::to_value(&lib).unwrap();
        assert!(json.get("default_thickness").is_none(), "{json}");
        lib.default_thickness = Some(18.0);
        let json = serde_json::to_value(&lib).unwrap();
        assert_eq!(json["default_thickness"], 18.0);
    }

    #[test]
    fn the_default_follows_a_rename_and_goes_with_the_material() {
        let mut lib = MaterialLibrary::default();
        lib.add("Oak").unwrap();
        lib.default_material = Some("Oak".into());
        assert!(lib.validate().is_empty());
        lib.materials[0].name = "White oak".into();
        lib.rename("oak", "White oak");
        assert_eq!(lib.default_entry().unwrap().name, "White oak");
        assert!(lib.remove("white oak").is_ok());
        assert_eq!(lib.default_material, None);
        assert!(lib.remove("oak").is_err());
        lib.default_material = Some("Pine".into());
        assert!(lib.validate().join("\n").contains("default_material"));
    }

    fn corpus(file: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus")
            .join(file)
    }

    #[test]
    fn old_files_lose_their_values_which_move_into_the_tools() {
        // Version 1 (with a thickness) and 2 load as names only.
        for file in ["materials/v1/materials.json", "materials/v2/materials.json"] {
            let lib: MaterialLibrary = load_library(&corpus(file)).unwrap();
            assert_eq!(lib.schema_version, MATERIALS_VERSION);
            assert!(
                lib.find("Plywood").is_some() && lib.find("Oak").is_some(),
                "{file}"
            );
        }
        // The values, moved: Plywood's for the 6 mm tool become its Plywood row.
        let old: Value = serde_json::from_str(
            &std::fs::read_to_string(corpus("materials/v2/materials.json")).unwrap(),
        )
        .unwrap();
        let mut tools = ToolLibrary::default();
        let tool = crate::Tool {
            id: 1,
            name: None,
            library_uid: None,
            kind: crate::ToolType::Endmill,
            diameter: 6.0,
            flutes: Some(2),
            center_cutting: Some(true),
            rpm: Some(18000.0),
            angle: None,
            tip_diameter: None,
        };
        tools.tools.push(LibraryTool::from_tool(
            "6 mm",
            "a1b2c3d4e5f60718",
            &tool,
            Vec::new(),
        ));
        let messages = move_material_values(&mut tools, &old);
        let ply = old["materials"][0]["uid"].as_str().unwrap();
        assert_eq!(
            tools.tools[0].row(Some(ply)).map(|r| r.values),
            Some(Cutting::uniform(1500.0, 500.0, 2.5, 0.4))
        );
        assert!(tools.validate().is_empty(), "{:?}", tools.validate());
        let text: Vec<String> = messages.iter().map(Message::english).collect();
        assert!(text.iter().any(|t| t.contains("6 mm")), "{text:?}");
        // Twice: nothing more.
        let before = tools.clone();
        move_material_values(&mut tools, &old);
        assert_eq!(tools, before);
    }

    #[test]
    fn a_newer_file_is_refused() {
        let dir =
            std::env::temp_dir().join(format!("chipload-materials-new-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(MATERIALS_FILE);
        std::fs::write(&path, r#"{"schema_version": 4, "materials": []}"#).unwrap();
        let read: Result<MaterialLibrary, _> = load_library(&path);
        assert!(matches!(read, Err(LibraryError::UnsupportedVersion { .. })));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
