// SPDX-License-Identifier: GPL-3.0-or-later
//! Job files: models, schema-version migrations and loading.
//!
//! The persisted format: every file has an integer
//! `schema_version`; older versions are migrated step by step, missing or newer versions are
//! refused, and validation only ever targets the current version.

pub mod cut_list;
pub mod edit;
pub mod explode;
pub mod library;
pub mod materials;
pub mod migrations;
pub mod models;
pub mod note;
pub mod paths;
pub mod problem;
#[cfg(feature = "schema")]
pub mod schema;

use std::path::Path;

use chipload_i18n::{Message, Value as I18nValue};
use serde_json::Value;

pub use cut_list::OpRef;
pub use models::*;
pub use note::Note;
pub use problem::Problem;

/// Why a project file can't be read or written. Its text is a message (`job.ftl`), shown
/// in the app's language; `Display` writes it in English for the command line.
#[derive(Debug)]
pub enum JobError {
    /// Missing, malformed or newer-than-known `schema_version`. Never guessed.
    UnsupportedVersion { found: String, current: i64 },
    /// A job from before projects (schema 1-3), which this version no longer reads.
    TooOld { found: i64 },
    /// An older file that can't be carried forward without changing what it means. The
    /// migration's `reason` is English: it names fields of the old file.
    CannotMigrate { from: i64, reason: String },
    /// `reason` is the file system's or JSON reader's own words, or a message.
    Unreadable { path: String, reason: I18nValue },
    /// `reason` is the JSON reader's own words, or the project's problems.
    Invalid { path: String, reason: I18nValue },
}

impl JobError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            JobError::UnsupportedVersion { found, current } => Message::new("job-error-version")
                .arg("found", found)
                .arg("oldest", OLDEST_VERSION)
                .arg("current", *current),
            JobError::TooOld { found } => Message::new("job-error-too-old").arg("found", *found),
            JobError::CannotMigrate { from, reason } => Message::new("job-error-cannot-migrate")
                .arg("from", *from)
                .arg("reason", reason),
            JobError::Unreadable { path, reason } => Message::new("job-error-unreadable")
                .arg("path", path)
                .arg("reason", reason.clone()),
            JobError::Invalid { path, reason } => Message::new("job-error-invalid")
                .arg("path", path)
                .arg("reason", reason.clone()),
        }
    }
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for JobError {}

/// Words of another library (the file system, the JSON reader), kept as they are.
pub(crate) fn foreign(error: impl std::fmt::Display) -> I18nValue {
    I18nValue::Text(error.to_string())
}

/// Load a job file: migrate older versions, refuse unknown or newer ones, then validate.
pub fn load_job(path: &Path) -> Result<Job, JobError> {
    let shown = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| JobError::Unreadable {
        path: shown.clone(),
        reason: foreign(e),
    })?;
    let data: Value = serde_json::from_str(&text).map_err(|e| JobError::Unreadable {
        path: shown.clone(),
        reason: foreign(e),
    })?;
    if !data.is_object() {
        return Err(JobError::Unreadable {
            path: shown,
            reason: Message::new("job-error-not-object").into(),
        });
    }
    parse_job(migrations::migrate(data)?, &shown)
}

/// Serialise as the current schema version.
///
/// Every field is written, defaults included, so a default that changes in a later version
/// can't silently change what an existing file means. ASCII-only JSON (non-ASCII escaped as
/// \uXXXX), two-space indent, fields in declaration order, ending with a newline.
pub fn dump_job(job: &Job) -> Result<String, JobError> {
    let mut current = job.clone();
    current.schema_version = CURRENT_VERSION;
    current.written_by = Some(format!("chipload {}", env!("CARGO_PKG_VERSION")));
    let text = serde_json::to_string_pretty(&current).map_err(|e| JobError::Invalid {
        path: "(job)".into(),
        reason: foreign(e),
    })?;
    Ok(ascii_json(&text))
}

/// Pretty JSON as written to disk: non-ASCII escaped as \uXXXX, ending with a newline.
pub(crate) fn ascii_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 1);
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            // Non-ASCII only occurs inside JSON strings, where \u escapes are valid.
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out.push('\n');
    out
}

/// Save as the current schema version, via a temporary file and a rename so a reader never
/// sees a half-written job. A file already there is kept as `<name>.bak` first
/// (docs/specs/autosave-and-backup.md §3); if that fails, nothing is changed.
pub fn save_job(job: &Job, path: &Path) -> Result<(), JobError> {
    let failed = |e: std::io::Error| JobError::Unreadable {
        path: path.display().to_string(),
        reason: Message::new("job-error-cannot-write")
            .arg("error", foreign(e))
            .into(),
    };
    let text = dump_job(job)?;
    let with_suffix = |suffix: &str| {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(suffix);
        path.with_file_name(name)
    };
    let tmp = with_suffix(".tmp");
    std::fs::write(&tmp, text).map_err(failed)?;
    if path.exists()
        && let Err(e) = std::fs::copy(path, with_suffix(".bak"))
    {
        // The project stays as it was; the half-done save leaves nothing behind.
        let _ = std::fs::remove_file(&tmp);
        return Err(JobError::Unreadable {
            path: path.display().to_string(),
            reason: Message::new("job-error-no-backup")
                .arg("error", foreign(e))
                .into(),
        });
    }
    std::fs::rename(&tmp, path).map_err(failed)
}

/// Validate already-migrated data as the current version, the end depths of cuts through
/// the stock following its thickness ([`Job::follow_thickness`]).
pub fn parse_job(data: Value, path: &str) -> Result<Job, JobError> {
    let mut job: Job = serde_json::from_value(data).map_err(|e| JobError::Invalid {
        path: path.to_string(),
        reason: foreign(e),
    })?;
    job.follow_thickness();
    let problems = job.validate().into_iter().map(|p| p.text_message);
    match problem::semicolons(problems) {
        None => Ok(job),
        Some(problems) => Err(JobError::Invalid {
            path: path.to_string(),
            reason: problems.into(),
        }),
    }
}
