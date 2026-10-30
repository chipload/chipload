// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload setup …`: a project's setups, each one board clamped once with its stock, its
//! work zero and the parts cut from it, run as a program of its own (docs/specs/setups.md
//! §8). The board's values are set by path: `set P setups.1.stock.thickness=6`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{Job, save_job};
use chipload_pipeline::setups::{
    activate_setup, add_setup, move_part_to_setup, remove_setup, set_setup,
};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum SetupAction {
    /// The project's setups: id, name, on or off, board, work zero and parts.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Add a setup at the end, a board like the active one's, with a first part of its own;
    /// both become active.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// Its name; default: "Setup N".
        #[arg(long)]
        name: Option<String>,
        /// Its board's thickness; default: the active setup's.
        #[arg(long, value_parser = crate::lengths::length)]
        thickness: Option<f64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Rename a setup, or turn it off or on (a setup that is off isn't cut).
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The setup's id (as `setup list` shows it).
        id: u64,
        /// Its new name.
        #[arg(long)]
        name: Option<String>,
        /// Leave the setup and its parts out.
        #[arg(long, conflicts_with = "on")]
        off: bool,
        /// Cut the setup again.
        #[arg(long)]
        on: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Make a setup the active one: shown, simulated, where new parts go.
    Activate {
        /// The project (.chipload).
        project: PathBuf,
        /// The setup's id.
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move a part into a setup: it is cut from that board, with its zero.
    Part {
        /// The project (.chipload).
        project: PathBuf,
        /// The part's id (as `part list` shows it).
        part: u64,
        /// The setup's id.
        #[arg(long)]
        to: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Remove a setup. Refused while it has parts, unless --with-parts.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// The setup's id.
        id: u64,
        /// Remove its parts and their operations with it.
        #[arg(long)]
        with_parts: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn setup(out: &mut Out, action: SetupAction) -> ExitCode {
    let (project, output) = match &action {
        SetupAction::List { project } => return list(out, project),
        SetupAction::Add {
            project, output, ..
        }
        | SetupAction::Set {
            project, output, ..
        }
        | SetupAction::Activate {
            project, output, ..
        }
        | SetupAction::Part {
            project, output, ..
        }
        | SetupAction::Remove {
            project, output, ..
        } => (project.clone(), output.clone()),
    };
    let job = match load(out, &project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let changed = match action {
        SetupAction::List { .. } => return ExitCode::SUCCESS,
        SetupAction::Add {
            name, thickness, ..
        } => add_setup(&job, name.as_deref(), thickness).map(|(job, id, part)| {
            out.line(format!(
                "Added setup {id} with part {part}, now the active setup and part"
            ));
            out.set("setup", json!(id));
            out.set("part", json!(part));
            job
        }),
        SetupAction::Set {
            id, name, off, on, ..
        } => {
            let enabled = match (off, on) {
                (true, _) => Some(false),
                (_, true) => Some(true),
                _ => None,
            };
            set_setup(&job, id, name.as_deref(), enabled)
        }
        SetupAction::Activate { id, .. } => activate_setup(&job, id),
        SetupAction::Part { part, to, .. } => move_part_to_setup(&job, part, to),
        SetupAction::Remove { id, with_parts, .. } => {
            remove_setup(&job, id, with_parts).map(|(job, lines)| {
                for line in lines {
                    out.line(line);
                }
                job
            })
        }
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
    let active = job.active_setup_id();
    let mut rows = Vec::new();
    for setup in &job.setups {
        let one = job.for_setup(setup.id);
        let (x, y) = one.stock_size();
        let parts: Vec<u64> = job
            .parts
            .iter()
            .filter(|p| p.setup == setup.id)
            .map(|p| p.id)
            .collect();
        let numbers: Vec<String> = parts.iter().map(u64::to_string).collect();
        let mm = chipload_core::text::mm;
        out.line(format!(
            "{} {}{}{}: board {} × {} × {} mm; zero {}; parts {}",
            setup.id,
            setup.name,
            if Some(setup.id) == active {
                " (active)"
            } else {
                ""
            },
            if setup.enabled { "" } else { ", off" },
            mm(x),
            mm(y),
            mm(setup.stock.thickness),
            one.work_zero_text(),
            if numbers.is_empty() {
                "none".to_string()
            } else {
                numbers.join(", ")
            }
        ));
        rows.push(json!({
            "id": setup.id,
            "name": setup.name,
            "enabled": setup.enabled,
            "active": Some(setup.id) == active,
            "board": [x, y, setup.stock.thickness],
            "work_zero": setup.work_zero,
            "parts": parts,
        }));
    }
    out.set("setups", Value::Array(rows));
    ExitCode::SUCCESS
}

fn write(out: &mut Out, job: &Job, target: &Path) -> ExitCode {
    if let Err(err) = save_job(job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}
