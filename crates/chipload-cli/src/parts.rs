// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload part …`: a project's parts, each a group of operations cut together with its
//! own copies (docs/specs/parts.md §7). `op part` moves an operation between them and
//! `copies --part` sets a part's grid.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{Job, save_job};
use chipload_pipeline::copies::grid_text;
use chipload_pipeline::parts::{activate_part, add_part, remove_part, set_part};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum PartAction {
    /// The project's parts: id, name, on or off, copies and operations.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Add an empty part at the end; it becomes the active part.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// Its name; default: "Part N".
        #[arg(long)]
        name: Option<String>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Rename a part, or turn it off or on (a part that is off isn't cut).
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The part's id (as `part list` shows it).
        id: u64,
        /// Its new name.
        #[arg(long)]
        name: Option<String>,
        /// Leave the part and its operations out of the program.
        #[arg(long, conflicts_with = "on")]
        off: bool,
        /// Cut the part again.
        #[arg(long)]
        on: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Make a part the active one: where the app adds new operations.
    Activate {
        /// The project (.chipload).
        project: PathBuf,
        /// The part's id.
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Remove a part. Refused while it has operations, unless --with-operations.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// The part's id.
        id: u64,
        /// Remove its operations with it.
        #[arg(long)]
        with_operations: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn part(out: &mut Out, action: PartAction) -> ExitCode {
    let (project, output) = match &action {
        PartAction::List { project } => return list(out, project),
        PartAction::Add {
            project, output, ..
        }
        | PartAction::Set {
            project, output, ..
        }
        | PartAction::Activate {
            project, output, ..
        }
        | PartAction::Remove {
            project, output, ..
        } => (project.clone(), output.clone()),
    };
    let job = match load(out, &project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let changed = match action {
        PartAction::List { .. } => return ExitCode::SUCCESS,
        PartAction::Add { name, .. } => add_part(&job, name.as_deref()).map(|(job, id)| {
            out.line(format!("Added part {id}, now the active part"));
            out.set("part", json!(id));
            job
        }),
        PartAction::Set {
            id, name, off, on, ..
        } => {
            let enabled = match (off, on) {
                (true, _) => Some(false),
                (_, true) => Some(true),
                _ => None,
            };
            set_part(&job, id, name.as_deref(), enabled)
        }
        PartAction::Activate { id, .. } => activate_part(&job, id),
        PartAction::Remove {
            id,
            with_operations,
            ..
        } => remove_part(&job, id, with_operations).map(|(job, lines)| {
            for line in lines {
                out.line(line);
            }
            job
        }),
    };
    match changed {
        Ok(job) => write(out, &job, output.as_deref().unwrap_or(&project)),
        Err(err) => out.fail(err),
    }
}

fn list(out: &mut Out, project: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let active = job.active_part_id();
    let mut rows = Vec::new();
    for part in &job.parts {
        let operations: Vec<usize> = (0..job.operations.len())
            .filter(|i| job.operations[*i].part() == part.id)
            .collect();
        let numbers: Vec<String> = operations.iter().map(|i| (i + 1).to_string()).collect();
        let copies = part.copies.as_ref().map_or("one".to_string(), grid_text);
        out.line(format!(
            "{} {}{}{}: {copies}; operations {}",
            part.id,
            part.name,
            if Some(part.id) == active {
                " (active)"
            } else {
                ""
            },
            if part.enabled { "" } else { ", off" },
            if numbers.is_empty() {
                "none".to_string()
            } else {
                numbers.join(", ")
            }
        ));
        rows.push(json!({
            "id": part.id,
            "name": part.name,
            "enabled": part.enabled,
            "active": Some(part.id) == active,
            "copies": part.copies,
            "count": part.count(),
            "operations": operations,
        }));
    }
    out.set("parts", Value::Array(rows));
    ExitCode::SUCCESS
}

fn write(out: &mut Out, job: &Job, target: &Path) -> ExitCode {
    if let Err(err) = save_job(job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}
