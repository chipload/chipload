// SPDX-License-Identifier: GPL-3.0-or-later
//! Job files: models, schema-version migrations and loading.
//!
//! The same persisted format as the Python implementation: every file has an integer
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
        "Job schema_version {found} is not supported (this app supports up to {current}). \
         Update the application or check the file."
    )]
    UnsupportedVersion { found: String, current: i64 },
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
