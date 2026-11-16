// SPDX-License-Identifier: GPL-3.0-or-later
//! Recovery notes: which project an autosave belongs to.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::LibraryError;
use super::files::LibraryFile;
use super::files::written_by;
use super::files::{parse, write};
use crate::foreign;

/// Current `schema_version` of a recovery note (docs/specs/autosave-and-backup.md §2).
pub const RECOVERY_VERSION: i64 = 1;

/// Beside a recovery copy of a project: which project it belongs to
/// (docs/specs/autosave-and-backup.md §2).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryNote {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    /// The project file the copy was made from; null for a project never saved. Required:
    /// a missing path is not guessed to mean "never saved".
    #[serde(deserialize_with = "required_option")]
    pub project_path: Option<String>,
}

/// An `Option` that must be present (as a value or null), unlike serde's default.
fn required_option<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}

impl LibraryFile for RecoveryNote {
    const VERSION: i64 = RECOVERY_VERSION;
    fn validate(&self) -> Vec<String> {
        match &self.project_path {
            Some(path) if path.trim().is_empty() => {
                vec!["project_path: must name a file, or be null".into()]
            }
            _ => Vec::new(),
        }
    }
    fn stamp(&mut self) {
        self.schema_version = RECOVERY_VERSION;
        self.written_by = Some(written_by());
    }
}

/// Read a recovery note; a missing one is an error (the copy can't be placed without it).
pub fn load_recovery_note(path: &Path) -> Result<RecoveryNote, LibraryError> {
    let shown = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| LibraryError::Unreadable {
        path: shown.clone(),
        reason: foreign(e),
    })?;
    parse(&text, &shown)
}

/// Write a recovery note: the current version, atomically, without a `.bak`.
pub fn save_recovery_note(note: &RecoveryNote, path: &Path) -> Result<(), LibraryError> {
    write(note, path, false)
}
