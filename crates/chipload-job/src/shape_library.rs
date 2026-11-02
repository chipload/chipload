// SPDX-License-Identifier: GPL-3.0-or-later
//! The shape library's entries: drawn shapes and texts saved under a name, one file each in
//! the settings folder's `shapes/`, in groups that are its folders
//! (docs/specs/shape-library.md §4, §5.2). A persisted format of its own, versioned as the
//! tool library is (`LibraryFile`).

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::library::{LibraryError, LibraryFile, load_library, refused, save_library};
use crate::{Geometry, Shape, Text, shape_problems};
use chipload_i18n::Message;

/// The library's folder inside the settings folder.
pub const SHAPES_DIR: &str = "shapes";
/// An entry's file extension.
pub const SHAPE_EXTENSION: &str = "chipload-shapes";
/// The entry format's version.
pub const SHAPES_VERSION: i64 = 1;
/// The longest name of an entry or a group, in characters.
pub const NAME_MAX_CHARS: usize = 100;

/// One entry: drawn shapes and texts with their letters, as in a project's drawing. Its
/// name is its file's name. Ids are its own: they are new ids when it is inserted.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ShapeEntry {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    /// Drawn shapes, and the letters of `texts` (`path` shapes that name their text).
    pub shapes: Vec<Shape>,
    /// Texts, which stay texts when inserted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<Text>,
}

impl ShapeEntry {
    /// What is wrong with the entry, as "field: message" in English.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.shapes.is_empty() {
            problems.push("shapes: an entry holds at least one shape".to_string());
        }
        let mut ids = BTreeSet::new();
        let texts: BTreeSet<u64> = self.texts.iter().map(|t| t.id).collect();
        for (i, shape) in self.shapes.iter().enumerate() {
            if !ids.insert(shape.id) {
                problems.push(format!("shapes.{i}.id: {} is used twice", shape.id));
            }
            for problem in shape_problems(&shape.geometry) {
                problems.push(format!("shapes.{i}.{}", problem.text));
            }
            if let Geometry::Path {
                image, dxf, text, ..
            } = &shape.geometry
            {
                // Only letters stay paths; other imported shapes are saved as drawn ones.
                if image.is_some() || dxf.is_some() || text.is_none_or(|t| !texts.contains(&t)) {
                    problems.push(format!(
                        "shapes.{i}: a path must be a letter of one of the entry's texts"
                    ));
                }
            }
        }
        if texts.len() != self.texts.len() {
            problems.push("texts: an id is used twice".to_string());
        }
        for (i, text) in self.texts.iter().enumerate() {
            for problem in text.problems(i) {
                problems.push(problem.text.replacen("drawing.texts", "texts", 1));
            }
        }
        problems
    }
}

impl LibraryFile for ShapeEntry {
    const VERSION: i64 = SHAPES_VERSION;
    fn validate(&self) -> Vec<String> {
        self.problems()
    }
    fn stamp(&mut self) {
        self.schema_version = SHAPES_VERSION;
        self.written_by = Some(crate::library::written_by());
    }
}

/// Load an entry. A missing file is an error, unlike the tool library's.
pub fn load_shape_entry(path: &Path) -> Result<ShapeEntry, LibraryError> {
    if !path.is_file() {
        return Err(LibraryError::Unreadable {
            path: path.display().to_string(),
            reason: Message::new("job-shapes-no-entry").into(),
        });
    }
    load_library(path)
}

/// Save an entry: validated, atomic, a file it replaces kept as `.bak`.
pub fn save_shape_entry(entry: &ShapeEntry, path: &Path) -> Result<(), LibraryError> {
    save_library(entry, path)
}

/// Whether `name` can be an entry's or a group's name (a file or folder name on every
/// system): not empty, no longer than [`NAME_MAX_CHARS`], no path separators or characters
/// Windows refuses, not starting with a dot or ending in a dot or space.
pub fn name_problem(name: &str) -> Option<LibraryError> {
    let bad = name.trim().is_empty()
        || name.chars().count() > NAME_MAX_CHARS
        || name.starts_with('.')
        || name.ends_with('.')
        || name.ends_with(' ')
        || name.starts_with(' ')
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        || reserved_on_windows(name);
    bad.then(|| {
        refused(
            Message::new("job-shapes-bad-name")
                .arg("name", name)
                .arg("most", NAME_MAX_CHARS),
        )
    })
}

/// CON, PRN, AUX, NUL, COM1-9 and LPT1-9, with or without an extension: Windows can't
/// make a file of that name.
fn reserved_on_windows(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit())
}
