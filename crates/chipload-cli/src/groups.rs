// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload group …`: groups of drawn shapes and texts, selected, moved and cut as one
//! (docs/specs/shape-groups.md §6).

use std::path::PathBuf;
use std::process::ExitCode;

use chipload_job::{Job, save_job};
use chipload_pipeline::PipelineError;
use chipload_pipeline::groups::{
    Members, add_to_group, delete_group, group, group_box, remove_from_group, rename_group,
    transform_group, ungroup,
};
use clap::Subcommand;
use serde_json::json;

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum GroupAction {
    /// Group shapes and texts; prints the group's id. With shapes of one group among them,
    /// the rest join it; of several, those groups become one.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S); a letter stands for its
        /// whole text.
        ids: Vec<u64>,
        /// Texts (ids as `text list` shows them).
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        texts: Vec<u64>,
        /// Its name (default: "Group N").
        #[arg(long)]
        name: Option<String>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// The project's groups and what is in them.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Rename a group, or add to it and take out of it.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The group's id.
        id: u64,
        #[arg(long)]
        name: Option<String>,
        /// Shapes to add (any in another group move over).
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        add: Vec<u64>,
        /// Texts to add.
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        add_texts: Vec<u64>,
        /// Shapes to take out; they stay where they are, free.
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        remove: Vec<u64>,
        /// Texts to take out.
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        remove_texts: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Ungroup: the shapes and texts stay, free. Operations made from the group get its
    /// shapes by id.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Delete a group with its shapes and texts.
    Delete {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move a group, its texts too.
    Move {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        /// How far right, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dx: f64,
        /// How far up, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dy: f64,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Turn a group counter-clockwise about the middle of its box, or a point.
    Turn {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        /// The angle, degrees, counter-clockwise; negative turns clockwise.
        #[arg(long, allow_hyphen_values = true)]
        angle_deg: f64,
        /// Turn about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Scale a group about the middle of its box, or a point; its texts' height scales.
    Scale {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        /// The factor: 2 doubles, 0.5 halves.
        #[arg(long)]
        factor: f64,
        /// Scale about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// The middle of group `id`'s box, or `about`.
fn about_point(job: &Job, id: u64, about: Option<[f64; 2]>) -> Result<(f64, f64), PipelineError> {
    if let Some([x, y]) = about {
        return Ok((x, y));
    }
    let ((x0, y0), (x1, y1)) = group_box(job, id).ok_or_else(|| {
        PipelineError::Job(chipload_i18n::Message::new("pipeline-no-group").arg("id", id))
    })?;
    Ok(((x0 + x1) / 2.0, (y0 + y1) / 2.0))
}

pub fn group_command(out: &mut Out, action: GroupAction) -> ExitCode {
    if let GroupAction::List { project } = &action {
        return list(out, project);
    }
    let (project, output) = match &action {
        GroupAction::Add {
            project, output, ..
        }
        | GroupAction::Set {
            project, output, ..
        }
        | GroupAction::Remove {
            project, output, ..
        }
        | GroupAction::Delete {
            project, output, ..
        }
        | GroupAction::Move {
            project, output, ..
        }
        | GroupAction::Turn {
            project, output, ..
        }
        | GroupAction::Scale {
            project, output, ..
        } => (project.clone(), output.clone()),
        GroupAction::List { .. } => return ExitCode::FAILURE,
    };
    let job = match load(out, &project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let done: Result<(Job, String), PipelineError> = match action {
        GroupAction::List { .. } => return ExitCode::FAILURE,
        GroupAction::Add {
            ids, texts, name, ..
        } => Members::of(&job, &ids, &texts)
            .and_then(|m| group(&job, &m, name.as_deref()))
            .map(|(job, id)| {
                out.set("id", json!(id));
                (job, format!("Group {id}."))
            }),
        GroupAction::Set {
            id,
            name,
            add,
            add_texts,
            remove,
            remove_texts,
            ..
        } => set(
            &job,
            id,
            name.as_deref(),
            (&add, &add_texts),
            (&remove, &remove_texts),
        )
        .map(|job| (job, format!("Changed group {id}."))),
        GroupAction::Remove { id, .. } => {
            ungroup(&job, id).map(|job| (job, format!("Ungrouped group {id}.")))
        }
        GroupAction::Delete { id, .. } => delete_group(&job, id).map(|job| {
            (
                job,
                format!("Deleted group {id} with its shapes and texts."),
            )
        }),
        GroupAction::Move { id, dx, dy, .. } => {
            transform_group(&job, id, (0.0, 0.0), (dx, dy), 1.0, 0.0)
                .map(|job| (job, format!("Moved group {id}.")))
        }
        GroupAction::Turn {
            id,
            angle_deg,
            about,
            ..
        } => about_point(&job, id, about).and_then(|c| {
            transform_group(&job, id, c, c, 1.0, angle_deg)
                .map(|job| (job, format!("Turned group {id} by {angle_deg}°.")))
        }),
        GroupAction::Scale {
            id, factor, about, ..
        } => about_point(&job, id, about).and_then(|c| {
            transform_group(&job, id, c, c, factor, 0.0)
                .map(|job| (job, format!("Scaled group {id} ×{factor}.")))
        }),
    };
    match done {
        Ok((job, line)) => {
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            out.line(line);
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

/// A group renamed, added to and taken out of.
fn set(
    job: &Job,
    id: u64,
    name: Option<&str>,
    add: (&[u64], &[u64]),
    remove: (&[u64], &[u64]),
) -> Result<Job, PipelineError> {
    let mut job = match name {
        Some(name) => rename_group(job, id, name)?,
        None => job.clone(),
    };
    if !add.0.is_empty() || !add.1.is_empty() {
        job = add_to_group(&job, id, &Members::of(&job, add.0, add.1)?)?;
    }
    if !remove.0.is_empty() || !remove.1.is_empty() {
        job = remove_from_group(&job, &Members::of(&job, remove.0, remove.1)?)?;
    }
    Ok(job)
}

fn list(out: &mut Out, project: &std::path::Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    if job.drawing.groups.is_empty() {
        out.line("No groups.");
    }
    let mut rows = Vec::new();
    for group in &job.drawing.groups {
        let shapes: Vec<String> = group.shapes.iter().map(|s| format!("S{s}")).collect();
        let texts: Vec<String> = group.texts.iter().map(|t| format!("T{t}")).collect();
        let all: Vec<String> = shapes.iter().chain(&texts).cloned().collect();
        out.line(format!(
            "{}: \"{}\": {}",
            group.id,
            group.name,
            all.join(", ")
        ));
        rows.push(json!({
            "id": group.id,
            "name": group.name,
            "shapes": group.shapes,
            "texts": group.texts,
        }));
    }
    out.set("groups", json!(rows));
    ExitCode::SUCCESS
}
