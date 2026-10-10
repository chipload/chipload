// SPDX-License-Identifier: GPL-3.0-or-later
//! Job files: models, schema-version migrations and loading.
//!
//! The persisted format: every file has an integer
//! `schema_version`; older versions are migrated step by step, missing or newer versions are
//! refused, and validation only ever targets the current version.

pub mod migrations;
pub mod models;

use std::path::Path;

use serde_json::Value;
use thiserror::Error;

pub use models::*;

#[derive(Debug, Error)]
pub enum JobError {
    /// Missing, malformed or newer-than-known `schema_version`. Never guessed.
    #[error(
        "Job schema_version {found} is not supported: this version of Chipload reads \
         {oldest} to {current}. Update Chipload or check the file.",
        oldest = OLDEST_VERSION
    )]
    UnsupportedVersion { found: String, current: i64 },
    /// A job from before projects (schema 1-3), which this version no longer reads.
    #[error(
        "Job schema_version {found} is from before projects and is no longer read. Open it \
         with Chipload 0.3.0 and save it, or import its DXF again."
    )]
    TooOld { found: i64 },
    /// An older file that can't be carried forward without changing what it means.
    #[error("This job (schema_version {from}) can't be opened: {reason}")]
    CannotMigrate { from: i64, reason: String },
    #[error("Cannot read job file {path}: {reason}")]
    Unreadable { path: String, reason: String },
    #[error("Invalid job file {path}: {reason}")]
    Invalid { path: String, reason: String },
}

/// Load a job file: migrate older versions, refuse unknown or newer ones, then validate.
pub fn load_job(path: &Path) -> Result<Job, JobError> {
    let shown = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| JobError::Unreadable {
        path: shown.clone(),
        reason: e.to_string(),
    })?;
    let data: Value = serde_json::from_str(&text).map_err(|e| JobError::Unreadable {
        path: shown.clone(),
        reason: e.to_string(),
    })?;
    if !data.is_object() {
        return Err(JobError::Unreadable {
            path: shown,
            reason: "must contain a JSON object".into(),
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
        reason: e.to_string(),
    })?;
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
    Ok(out)
}

/// Save as the current schema version, via a temporary file and a rename so a reader never
/// sees a half-written job.
pub fn save_job(job: &Job, path: &Path) -> Result<(), JobError> {
    let failed = |e: std::io::Error| JobError::Unreadable {
        path: path.display().to_string(),
        reason: format!("cannot write: {e}"),
    };
    let text = dump_job(job)?;
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    std::fs::write(&tmp, text).map_err(failed)?;
    std::fs::rename(&tmp, path).map_err(failed)
}

/// Validate already-migrated data as the current version.
pub fn parse_job(data: Value, path: &str) -> Result<Job, JobError> {
    let job: Job = serde_json::from_value(data).map_err(|e| JobError::Invalid {
        path: path.to_string(),
        reason: e.to_string(),
    })?;
    let problems = job.validate();
    if problems.is_empty() {
        Ok(job)
    } else {
        Err(JobError::Invalid {
            path: path.to_string(),
            reason: problems.join("; "),
        })
    }
}
