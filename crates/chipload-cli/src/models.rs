// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload model …`: 3D models (STL) carved as reliefs from the top
//! (docs/specs/stl-relief.md §2, §3, §9).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{Job, ModelUp, Units};
use chipload_pipeline::models::{
    ModelReport, NewModel, add_model, read_model, remake, remove_model,
};
use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::out::Out;
use crate::{load, write_project};

#[derive(Subcommand)]
pub enum ModelAction {
    /// Add an STL model, placed on the board; its footprint outline goes on its own layer.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// The STL file (binary or text).
        model: PathBuf,
        /// The unit the file is in, for its own size: mm (default) or inch. The model is
        /// placed by `--width` anyway.
        #[arg(long, value_parser = parse_units)]
        units: Option<Units>,
        /// The layer for its outline; default: the file's name.
        #[arg(long)]
        layer: Option<String>,
        #[command(flatten)]
        place: Place,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Change a model's place, size, depth, up axis or cut level; its outline is drawn
    /// again.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The model's id (as `model list` shows it).
        id: u64,
        /// The layer for its outline.
        #[arg(long)]
        layer: Option<String>,
        /// Make the depth follow the width again, in proportion.
        #[arg(long, conflicts_with = "depth")]
        proportional: bool,
        #[command(flatten)]
        place: Place,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// The project's models: file, triangles, place and size on the board.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Remove a model and its outline.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// The model's id.
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Where a model lies on the board (spec §3); unset: the default (add) or unchanged (set).
#[derive(Args)]
pub struct Place {
    /// Where the footprint's lower-left corner goes: X,Y in mm (default 0,0).
    #[arg(long, value_parser = parse_at)]
    at: Option<(f64, f64)>,
    /// The footprint's width on the board, mm (default: the model's own); X and Y scale
    /// alike.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// How deep the relief goes, mm (default: in proportion to the width).
    #[arg(value_parser = crate::lengths::length, long)]
    depth: Option<f64>,
    /// Turn it about its footprint's centre to this angle, degrees counter-clockwise.
    #[arg(long, allow_hyphen_values = true)]
    rotate: Option<f64>,
    /// Which of the model's axes points up: z (default), y, x, -z, -y or -x.
    #[arg(long, allow_hyphen_values = true, value_parser = parse_up)]
    up: Option<ModelUp>,
    /// How much of the model to keep, from its top: a share (0.5) or a percentage (50%);
    /// the rest, its back, is cut away (default: all of it). Width and depth are the kept
    /// part's.
    #[arg(long, value_parser = parse_keep)]
    keep: Option<f64>,
}

/// A share of the model kept: "0.5" or "50%", more than 0 and at most 1.
fn parse_keep(text: &str) -> Result<f64, String> {
    let text = text.trim();
    let share = match text.strip_suffix('%') {
        Some(percent) => percent.trim().parse::<f64>().map(|p| p / 100.0),
        None => text.parse::<f64>(),
    }
    .map_err(|_| format!("expected a share such as 0.5 or 50%, got {text:?}"))?;
    if share.is_finite() && share > 0.0 && share <= 1.0 {
        Ok(share)
    } else {
        Err(format!(
            "keep more than 0 and at most all of it (1 or 100%), got {text:?}"
        ))
    }
}

fn parse_at(text: &str) -> Result<(f64, f64), String> {
    crate::lengths::point(text).map(|[x, y]| (x, y))
}

fn parse_units(text: &str) -> Result<Units, String> {
    match text {
        "mm" => Ok(Units::Mm),
        "inch" | "in" => Ok(Units::Inch),
        _ => Err(format!("expected mm or inch, got {text:?}")),
    }
}

fn parse_up(text: &str) -> Result<ModelUp, String> {
    [
        ModelUp::Z,
        ModelUp::Y,
        ModelUp::X,
        ModelUp::MinusZ,
        ModelUp::MinusY,
        ModelUp::MinusX,
    ]
    .into_iter()
    .find(|up| up.name().eq_ignore_ascii_case(text))
    .ok_or_else(|| format!("expected z, y, x, -z, -y or -x, got {text:?}"))
}

/// Degrees in 0 ≤ a < 360.
fn turned(degrees: f64) -> Result<f64, String> {
    if !degrees.is_finite() {
        return Err(format!("--rotate takes degrees, got {degrees}"));
    }
    let a = degrees.rem_euclid(360.0);
    Ok(if a >= 360.0 { 0.0 } else { a })
}

pub fn model(out: &mut Out, action: ModelAction) -> ExitCode {
    match action {
        ModelAction::Add {
            project,
            model,
            units,
            layer,
            place,
            output,
        } => add(
            out,
            &project,
            &model,
            units,
            layer,
            &place,
            output.as_deref(),
        ),
        ModelAction::Set {
            project,
            id,
            layer,
            proportional,
            place,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let mut changed = job.clone();
            let applied = (|| -> Result<(), String> {
                let model = changed
                    .drawing
                    .models
                    .iter_mut()
                    .find(|m| m.id == id)
                    .ok_or_else(|| format!("there is no model {id}"))?;
                if let Some((x, y)) = place.at {
                    (model.x, model.y) = (x, y);
                }
                if let Some(width) = place.width {
                    model.width = width;
                }
                if let Some(depth) = place.depth {
                    model.depth = depth;
                    model.proportional = false;
                }
                if proportional {
                    model.proportional = true;
                }
                if let Some(degrees) = place.rotate {
                    model.rotation = turned(degrees)?;
                }
                if let Some(up) = place.up {
                    model.up = up;
                }
                if let Some(keep) = place.keep {
                    model.keep = keep;
                }
                if let Some(layer) = &layer {
                    model.layer = layer.clone();
                }
                Ok(())
            })();
            if let Err(err) = applied {
                return out.fail(err);
            }
            match remake(&changed, id) {
                Ok((job, report)) => {
                    reported(out, &report);
                    write_project(out, &job, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
        ModelAction::List { project } => list(out, &project),
        ModelAction::Remove {
            project,
            id,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            match remove_model(&job, id) {
                Ok(job) => {
                    out.line(format!("Removed model {id} and its outline."));
                    write_project(out, &job, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
    }
}

fn add(
    out: &mut Out,
    project: &Path,
    path: &Path,
    units: Option<Units>,
    layer: Option<String>,
    place: &Place,
    output: Option<&Path>,
) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => return out.fail(format!("{}: {err}", path.display())),
    };
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut new = NewModel::new(&file);
    new.units = units.unwrap_or(Units::Mm);
    new.up = place.up.unwrap_or_default();
    new.layer = layer;
    (new.x, new.y) = place.at.unwrap_or((0.0, 0.0));
    new.width = place.width;
    new.depth = place.depth;
    new.keep = place.keep.unwrap_or(1.0);
    if let Some(degrees) = place.rotate {
        match turned(degrees) {
            Ok(a) => new.rotation = a,
            Err(err) => return out.fail(err),
        }
    }
    let added = read_model(&bytes).and_then(|read| add_model(&job, &read, new));
    match added {
        Ok((job, report)) => {
            reported(out, &report);
            write_project(out, &job, output.unwrap_or(project))
        }
        Err(err) => out.fail(err),
    }
}

/// A report's lines, and its numbers for `--json`.
pub fn reported(out: &mut Out, report: &ModelReport) {
    for line in &report.lines {
        out.line(line.clone());
    }
    out.set("model", json!(report.id));
    out.set("shapes", json!(report.shapes));
    out.set("size", json!(report.size));
    out.set("placed", json!(report.placed));
}

fn list(out: &mut Out, project: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    if job.drawing.models.is_empty() {
        out.line("No models.");
    }
    let mut rows = Vec::new();
    for model in &job.drawing.models {
        let mut depth = if model.proportional {
            format!("{} mm deep, in proportion", model.depth)
        } else {
            format!("{} mm deep", model.depth)
        };
        if model.keep < 1.0 {
            depth.push_str(&format!(", the top {}% kept", model.keep * 100.0));
        }
        out.line(format!(
            "Model {}: {} ({}), {} up, layer {:?}, at {}, {} wide, {}, turned {}°",
            model.id,
            model.name,
            model.file,
            model.up.name(),
            model.layer,
            point(model.x, model.y),
            model.width,
            depth,
            model.rotation
        ));
        let shapes: Vec<u64> = job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.model() == Some(model.id))
            .map(|s| s.id)
            .collect();
        rows.push(json!({
            "id": model.id,
            "name": model.name,
            "file": model.file,
            "units": model.units.name(),
            "up": model.up.name(),
            "layer": model.layer,
            "x": model.x,
            "y": model.y,
            "width": model.width,
            "depth": model.depth,
            "proportional": model.proportional,
            "keep": model.keep,
            "rotation": model.rotation,
            "shapes": shapes,
        }));
    }
    out.set("models", Value::Array(rows));
    ExitCode::SUCCESS
}

fn point(x: f64, y: f64) -> String {
    format!("{x},{y}")
}

/// After `set` on paths: changed models' outlines drawn again.
pub fn remake_changed(out: &mut Out, before: &Job, after: Job) -> Result<Job, ExitCode> {
    let (job, reports) =
        chipload_pipeline::models::remake_changed(before, &after).map_err(|e| out.fail(e))?;
    for report in &reports {
        for line in &report.lines {
            out.line(line.clone());
        }
    }
    Ok(job)
}
