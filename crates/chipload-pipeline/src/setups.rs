// SPDX-License-Identifier: GPL-3.0-or-later
//! Setups (docs/specs/setups.md): add, change, remove and activate them, and move parts
//! between them. A setup is one board clamped once: its stock, its work zero and the parts
//! cut from it, each run as a program of its own.

use chipload_i18n::Message;
use chipload_job::{Job, Part, Setup};

use crate::PipelineError;
use crate::parts::free_name;

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn index_of(job: &Job, id: u64) -> Result<usize, PipelineError> {
    job.setups
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| refused(Message::new("pipeline-no-setup").arg("id", id)))
}

/// Refuse a result the project's rules don't allow.
fn checked(job: Job) -> Result<Job, PipelineError> {
    match job.validate().into_iter().next() {
        Some(problem) => Err(refused(problem.text_message)),
        None => Ok(job),
    }
}

/// A new setup at the end, which becomes the active one, with a first part of its own, the
/// active part: a board like the active setup's (its stock and zero), `thickness` thick
/// when given. Returns its id and its part's.
pub fn add_setup(
    job: &Job,
    name: Option<&str>,
    thickness: Option<f64>,
) -> Result<(Job, u64, u64), PipelineError> {
    let mut job = job.clone();
    let id = job.setups.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let name = match name {
        Some(name) if name.trim().is_empty() => {
            return Err(refused(Message::new("pipeline-setup-name-empty")));
        }
        Some(name) => name.trim().to_string(),
        None => format!("Setup {}", job.setups.len() + 1),
    };
    let mut stock = job.stock().clone();
    if let Some(thickness) = thickness {
        stock.thickness = thickness;
    }
    let work_zero = job.work_zero();
    job.setups.push(Setup {
        id,
        name,
        enabled: true,
        stock,
        work_zero,
    });
    let part = job.parts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
    let part_name = free_name(&job, None)?;
    job.parts.push(Part {
        id: part,
        name: part_name,
        enabled: true,
        copies: None,
        setup: id,
    });
    job.active_setup = Some(id);
    job.active_part = Some(part);
    Ok((checked(job)?, id, part))
}

/// Rename a setup, or turn it on or off.
pub fn set_setup(
    job: &Job,
    id: u64,
    name: Option<&str>,
    enabled: Option<bool>,
) -> Result<Job, PipelineError> {
    let mut job = job.clone();
    let at = index_of(&job, id)?;
    if let Some(name) = name {
        if name.trim().is_empty() {
            return Err(refused(Message::new("pipeline-setup-name-empty")));
        }
        job.setups[at].name = name.trim().to_string();
    }
    if let Some(enabled) = enabled {
        job.setups[at].enabled = enabled;
    }
    checked(job)
}

/// Make setup `id` the active one: shown, simulated and previewed; new parts go into it,
/// and its first part becomes the active part.
pub fn activate_setup(job: &Job, id: u64) -> Result<Job, PipelineError> {
    index_of(job, id)?;
    let mut job = job.clone();
    job.active_setup = Some(id);
    let in_it = |part: Option<u64>| {
        part.and_then(|p| job.part(p))
            .is_some_and(|p| p.setup == id)
    };
    if !in_it(job.active_part) {
        job.active_part = job.parts.iter().find(|p| p.setup == id).map(|p| p.id);
    }
    Ok(job)
}

/// Remove a setup. Refused for the last one, and while it has parts unless `with_parts`,
/// which removes them and their operations too. Returns what was done, for the report.
pub fn remove_setup(
    job: &Job,
    id: u64,
    with_parts: bool,
) -> Result<(Job, Vec<String>), PipelineError> {
    let at = index_of(job, id)?;
    if job.setups.len() == 1 {
        return Err(refused(Message::new("pipeline-last-setup")));
    }
    let name = job.setups[at].name.clone();
    let parts: Vec<u64> = job
        .parts
        .iter()
        .filter(|p| p.setup == id)
        .map(|p| p.id)
        .collect();
    if !parts.is_empty() && !with_parts {
        return Err(refused(
            Message::new("pipeline-setup-has-parts")
                .arg("name", name)
                .arg("count", parts.len()),
        ));
    }
    let mut job = job.clone();
    job.setups.remove(at);
    job.parts.retain(|p| p.setup != id);
    job.operations.retain(|op| !parts.contains(&op.part()));
    if job.active_setup == Some(id) {
        job.active_setup = job.setups.first().map(|s| s.id);
    }
    if job.active_part.is_some_and(|p| parts.contains(&p)) {
        job.active_part = job.parts.first().map(|p| p.id);
    }
    let mut lines = vec![format!("Removed setup {name}")];
    if !parts.is_empty() {
        lines.push(format!(
            "Removed its {} part(s) and their operations",
            parts.len()
        ));
    }
    if job.parts.is_empty() {
        return Err(refused(Message::new("pipeline-last-part")));
    }
    Ok((checked(job)?, lines))
}

/// Move part `part` into setup `id`: it is cut there, from that board, with its zero.
pub fn move_part_to_setup(job: &Job, part: u64, id: u64) -> Result<Job, PipelineError> {
    index_of(job, id)?;
    let mut job = job.clone();
    let Some(p) = job.parts.iter_mut().find(|p| p.id == part) else {
        return Err(refused(Message::new("pipeline-no-part").arg("id", part)));
    };
    p.setup = id;
    checked(job)
}
