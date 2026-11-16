// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's tool library and machine profiles (docs/specs/tool-library-and-machines.md).
//!
//! Two persisted formats, `tools.json` and `machines.json`, kept in the user's settings
//! folder. A project **copies** what it uses from them; planning and G-code never read them.
//! Loading follows the job rules: one function per format, older versions migrated, missing,
//! malformed or newer versions refused. A missing file is an empty library.

use chipload_i18n::{Message, Value as I18nValue};

mod exchange;
mod files;
mod links;
mod machines;
mod names;
mod recovery;
mod settings;
mod tools;

pub(crate) use exchange::refused;
pub use exchange::{
    Collection, Entry, ImportPlan, export, import_into, import_plan, read_import, write_export,
};
pub(crate) use files::written_by;
pub use files::{
    LibraryFile, load_library, load_machines, load_settings, load_tools, save_library,
    save_machines, save_tools, update_library, update_machines, update_settings, update_tools,
};
pub use links::{Link, LinkEntry, UsedTool, linked_cutting, machine_link, tool_link, use_tool};
pub(crate) use machines::Changes;
pub use machines::{
    MachineLibrary, MachineProfile, machines_v1_to_v2, machines_v2_to_v3, machines_v3_to_v4,
};
pub use names::{fresh_uid, same_name, uid_for_name};
pub(crate) use names::{is_uid, name_problems, position, uid_problems};
pub use recovery::{RECOVERY_VERSION, RecoveryNote, load_recovery_note, save_recovery_note};
pub use settings::{
    AppSettings, NewProject, SYSTEM_LANGUAGE, Theme, settings_v1_to_v2, settings_v2_to_v3,
};
pub(crate) use tools::uniform_row;
pub use tools::{
    Cutting, CuttingRow, LibraryTool, ToolLibrary, tools_v1_to_v2, tools_v2_to_v3, tools_v3_to_v4,
};

pub const TOOLS_FILE: &str = "tools.json";
pub const MACHINES_FILE: &str = "machines.json";
/// Current `schema_version` of `tools.json`.
/// Version 2 added a V-bit's `angle` and `tip_diameter` (docs/specs/v-carve.md §9), version
/// 3 each tool's `uid` (docs/specs/library-in-browser.md §2), version 4 its cutting data, a
/// row per material in place of one set of values (docs/specs/cutting-data.md §8).
pub const TOOLS_VERSION: i64 = 4;
/// Current `schema_version` of `machines.json`. Version 3 added each profile's `uid`,
/// version 4 its `gcode_units` (docs/specs/inch-units.md §4.1).
pub const MACHINES_VERSION: i64 = 4;
/// App settings: the recent projects (docs/specs/open-recent.md).
pub const SETTINGS_FILE: &str = "settings.json";
/// Current `schema_version` of `settings.json`. Version 2 added `theme`, version 3
/// `auto_generate`.
pub const SETTINGS_VERSION: i64 = 3;
/// How many recent projects are kept.
pub const RECENT_MAX: usize = 10;
/// How many recently used fonts are kept (docs/specs/font-list.md).
pub const RECENT_FONTS_MAX: usize = 5;

/// Why a library, settings or recovery file can't be read or changed. Its text is a
/// message (`job.ftl`), shown in the app's language; `Display` writes it in English.
#[derive(Debug)]
pub enum LibraryError {
    /// Missing, malformed or newer-than-known `schema_version`. Never guessed.
    UnsupportedVersion {
        path: String,
        found: String,
        current: i64,
    },
    /// `reason` is the file system's or JSON reader's own words, or a message.
    Unreadable {
        path: String,
        reason: I18nValue,
    },
    /// `reason` is the JSON reader's own words, or the file's rule violations (English:
    /// they name the file's fields).
    Invalid {
        path: String,
        reason: I18nValue,
    },
    Unwritable {
        path: String,
        reason: I18nValue,
    },
    /// A change the library refuses, e.g. a name that isn't there: a message, or words the
    /// caller wrote.
    Refused(I18nValue),
}

impl LibraryError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            LibraryError::UnsupportedVersion {
                path,
                found,
                current,
            } => Message::new("job-library-error-version")
                .arg("path", path)
                .arg("found", found)
                .arg("current", *current),
            LibraryError::Unreadable { path, reason } => {
                Message::new("job-library-error-unreadable")
                    .arg("path", path)
                    .arg("reason", reason.clone())
            }
            LibraryError::Invalid { path, reason } => Message::new("job-library-error-invalid")
                .arg("path", path)
                .arg("reason", reason.clone()),
            LibraryError::Unwritable { path, reason } => {
                Message::new("job-library-error-unwritable")
                    .arg("path", path)
                    .arg("reason", reason.clone())
            }
            LibraryError::Refused(reason) => {
                Message::new("job-library-error-refused").arg("reason", reason.clone())
            }
        }
    }
}

impl std::fmt::Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for LibraryError {}
