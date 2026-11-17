// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload part …`: a project's parts, each a group of operations cut together with its
//! own instances (docs/specs/parts.md §7). `op part` moves an operation between them and
//! `part instance` places a part again (instances.rs).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::Mirror;
use chipload_pipeline::copies::instances_text;
use chipload_pipeline::instances::set_order;
use chipload_pipeline::parts::{activate_part, add_part, remove_part, set_part, set_part_mirror};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::out::Out;
use crate::{load, write_project};

#[derive(Subcommand)]
pub enum PartAction {
    /// The project's parts: id, name, on or off, instances and operations.
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
        /// Cut the part mirrored about the centre line of its box, the drawing as drawn:
        /// left-right, top-bottom, or none (docs/specs/mirror-part.md). Climb stays climb.
        #[arg(long, value_parser = mirror)]
        mirror: Option<MirrorChoice>,
        /// The order its instances are cut in (docs/specs/instances.md §2).
        #[arg(long)]
        order: Option<crate::placing::OrderArg>,
        /// How it may be turned when nested: none, half, quarter or every-15
        /// (docs/specs/nesting.md §3).
        #[arg(long, value_parser = crate::nesting::turns_value)]
        turns: Option<chipload_job::Turns>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Place the part itself (instance 1): cut moved and turned, as an instance is, its
    /// shapes staying where they are drawn (docs/specs/part-placement.md).
    Place {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part (as `part list` shows it); needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// Its move "DX,DY" from where it is drawn (any length unit; plain numbers are mm).
        #[arg(long, allow_hyphen_values = true)]
        at: Option<String>,
        /// Turned about the middle of the part's box, degrees counter-clockwise.
        #[arg(long, allow_hyphen_values = true)]
        angle: Option<f64>,
        /// Back where drawn: not moved, not turned (then --at and --angle, if given).
        #[arg(long)]
        reset: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// A part's instances: the part placed again, moved, turned and perhaps mirrored
    /// (docs/specs/instances.md).
    Instance {
        #[command(subcommand)]
        action: crate::instances::InstanceAction,
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
        PartAction::Instance { .. } => {
            let PartAction::Instance { action } = action else {
                return ExitCode::SUCCESS;
            };
            return crate::instances::instance(out, action);
        }
        PartAction::Place {
            project,
            part,
            at,
            angle,
            reset,
            output,
        } => {
            let target = output.as_deref().unwrap_or(project);
            return crate::instances::place(
                out,
                project,
                *part,
                at.as_deref(),
                *angle,
                *reset,
                target,
            );
        }
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
        PartAction::List { .. } | PartAction::Instance { .. } | PartAction::Place { .. } => {
            return ExitCode::SUCCESS;
        }
        PartAction::Add { name, .. } => add_part(&job, name.as_deref()).map(|(job, id)| {
            out.line(format!("Added part {id}, now the active part"));
            out.set("part", json!(id));
            job
        }),
        PartAction::Set {
            id,
            name,
            off,
            on,
            mirror,
            order,
            turns,
            ..
        } => {
            let enabled = match (off, on) {
                (true, _) => Some(false),
                (_, true) => Some(true),
                _ => None,
            };
            set_part(&job, id, name.as_deref(), enabled)
                .and_then(|job| match mirror {
                    Some(MirrorChoice(mirror)) => set_part_mirror(&job, id, mirror),
                    None => Ok(job),
                })
                .and_then(|job| match order {
                    Some(order) => set_order(&job, id, order.value()),
                    None => Ok(job),
                })
                .and_then(|job| match turns {
                    Some(turns) => chipload_pipeline::parts::set_part_turns(&job, id, turns),
                    None => Ok(job),
                })
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
        Ok(job) => write_project(out, &job, output.as_deref().unwrap_or(&project)),
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
        let copies = if job.instances_of(part.id).len() < 2 {
            "one".to_string()
        } else {
            instances_text(&job, part)
        };
        out.line(format!(
            "{} {}{}{}: {copies}; operations {}",
            part.id,
            part.name,
            if Some(part.id) == active {
                " (active)"
            } else {
                ""
            },
            part.mirror
                .map_or(String::new(), |m| format!(", mirrored {}", m.name()))
                + &crate::instances::placed_text(&job, part)
                + if part.enabled { "" } else { ", off" },
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
            "order": part.order,
            "mirror": part.mirror,
            "setups": job.setups_of_part(part.id),
            "count": job.count_all(part.id),
            "operations": operations,
        }));
    }
    out.set("parts", Value::Array(rows));
    ExitCode::SUCCESS
}

/// A part's mirror as `--mirror` sets it: None for none.
#[derive(Clone, Copy)]
pub struct MirrorChoice(pub(crate) Option<Mirror>);

/// A part's mirror as the command line writes it.
pub(crate) fn mirror(text: &str) -> Result<MirrorChoice, String> {
    match text {
        "left-right" => Ok(MirrorChoice(Some(Mirror::LeftRight))),
        "top-bottom" => Ok(MirrorChoice(Some(Mirror::TopBottom))),
        "none" => Ok(MirrorChoice(None)),
        _ => Err("left-right, top-bottom or none".into()),
    }
}
