// SPDX-License-Identifier: GPL-3.0-or-later
//! Loading and saving the library files: migrated, checked, written with a backup.

use std::path::Path;

use chipload_i18n::{Message, Value as I18nValue};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::LibraryError;
use super::machines::MachineLibrary;
use super::settings::AppSettings;
use super::tools::ToolLibrary;
use crate::foreign;

pub(crate) fn written_by() -> String {
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
    let unreadable = |reason: I18nValue| LibraryError::Unreadable {
        path: shown.clone(),
        reason,
    };
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text, &shown),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(L::default()),
        Err(e) => Err(unreadable(foreign(e))),
    }
}

/// A library file's text, read as `load` does: migrated, then checked.
pub(super) fn parse<L: LibraryFile>(text: &str, shown: &str) -> Result<L, LibraryError> {
    let unreadable = |reason: I18nValue| LibraryError::Unreadable {
        path: shown.to_string(),
        reason,
    };
    let data: Value = serde_json::from_str(text).map_err(|e| unreadable(foreign(e)))?;
    let Value::Object(mut object) = data else {
        return Err(unreadable(Message::new("job-error-not-object").into()));
    };
    let version = object.get("schema_version");
    let Some(found) = version
        .and_then(Value::as_i64)
        .filter(|v| (L::OLDEST..=L::VERSION).contains(v))
    else {
        return Err(LibraryError::UnsupportedVersion {
            path: shown.to_string(),
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
        path: shown.to_string(),
        reason: foreign(e),
    })?;
    check(&library, shown)?;
    Ok(library)
}

fn check<L: LibraryFile>(library: &L, path: &str) -> Result<(), LibraryError> {
    let problems = library.validate();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(LibraryError::Invalid {
            path: path.to_string(),
            reason: I18nValue::Text(problems.join("; ")),
        })
    }
}

fn save<L: LibraryFile>(library: &L, path: &Path) -> Result<(), LibraryError> {
    write(library, path, true)
}

/// Write a library file atomically; `backup` keeps the previous file as `.bak`.
pub(super) fn write<L: LibraryFile>(
    library: &L,
    path: &Path,
    backup: bool,
) -> Result<(), LibraryError> {
    let shown = path.display().to_string();
    check(library, &shown)?;
    let failed = |e: std::io::Error| LibraryError::Unwritable {
        path: shown.clone(),
        reason: foreign(e),
    };
    let mut current = library.clone();
    current.stamp();
    let text = serde_json::to_string_pretty(&current).map_err(|e| LibraryError::Invalid {
        path: shown.clone(),
        reason: foreign(e),
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
    if backup && path.exists() {
        std::fs::copy(path, with_suffix(".bak")).map_err(failed)?;
    }
    std::fs::rename(&tmp, path).map_err(failed)
}

/// A library file format: its version, name in messages, and rules.
pub trait LibraryFile: Serialize + DeserializeOwned + Default + Clone {
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

/// Load any library file; a missing file is an empty library (as `load_tools`).
pub fn load_library<L: LibraryFile>(path: &Path) -> Result<L, LibraryError> {
    load(path)
}

/// Save any library file (as `save_tools`).
pub fn save_library<L: LibraryFile>(library: &L, path: &Path) -> Result<(), LibraryError> {
    save(library, path)
}

/// Change any library file (as `update_tools`).
pub fn update_library<L: LibraryFile, T>(
    path: &Path,
    change: impl FnOnce(&mut L) -> Result<T, LibraryError>,
) -> Result<T, LibraryError> {
    update(path, change)
}
