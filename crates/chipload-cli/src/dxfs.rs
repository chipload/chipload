// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload dxf …`: a project's DXFs, each with its own placement
//! (docs/specs/drawing-sources.md §2). `place --dxf` and `update --dxf` move and update one.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{Job, Units, save_job};
use chipload_pipeline::dxfs::{add_named_dxf, remove_dxf};
use chipload_pipeline::placement::{dxf_size, placement_text};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum DxfAction {
    /// Add a DXF to the project, where the file has it; move it with `place --dxf ID`.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// The DXF.
        dxf: PathBuf,
        /// The DXF's units, if it doesn't say.
        #[arg(long, value_parser = parse_units)]
        units: Option<Units>,
        /// Its name; default: the file's name without its extension.
        #[arg(long)]
        name: Option<String>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// The project's DXFs: id, name, file, placement and shapes.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Remove a DXF and its shapes.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// The DXF's id (as `dxf list` shows it).
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn parse_units(text: &str) -> Result<Units, String> {
    match text {
        "mm" => Ok(Units::Mm),
        "inch" => Ok(Units::Inch),
        _ => Err(format!("expected mm or inch, got {text:?}")),
    }
}

pub fn dxf(out: &mut Out, action: DxfAction) -> ExitCode {
    match action {
        DxfAction::Add {
            project,
            dxf,
            units,
            name,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let (job, report) = match add_named_dxf(&job, &dxf, units, name.as_deref()) {
                Ok(done) => done,
                Err(err) => return out.fail(err),
            };
            for line in &report.lines {
                out.line(line.clone());
            }
            let id = job.drawing.dxfs.last().map_or(0, |d| d.id);
            out.set("dxf", json!(id));
            out.set("added", json!(report.added));
            write(out, &job, output.as_deref().unwrap_or(&project))
        }
        DxfAction::List { project } => list(out, &project),
        DxfAction::Remove {
            project,
            id,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            match remove_dxf(&job, id) {
                Ok((job, lines)) => {
                    for line in lines {
                        out.line(line);
                    }
                    write(out, &job, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
    }
}

fn list(out: &mut Out, project: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    if job.drawing.dxfs.is_empty() {
        out.line("No DXFs.");
    }
    let mut rows = Vec::new();
    for dxf in &job.drawing.dxfs {
        let shapes = job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.dxf() == Some(dxf.id))
            .count();
        let size = dxf_size(&job.drawing.shapes, dxf.id);
        out.line(format!(
            "{} {}: {}, {}; {shapes} {}{}",
            dxf.id,
            dxf.name,
            dxf.file,
            placement_text(dxf.placement),
            if shapes == 1 { "shape" } else { "shapes" },
            size.map(|(w, h)| format!(", {w:.3} × {h:.3} mm"))
                .unwrap_or_default()
        ));
        rows.push(json!({
            "id": dxf.id,
            "name": dxf.name,
            "file": dxf.file,
            "sha256": dxf.sha256,
            "units": dxf.units,
            "placement": dxf.placement,
            "shapes": shapes,
            "size": size.map(|(w, h)| [w, h]),
        }));
    }
    out.set("dxfs", Value::Array(rows));
    ExitCode::SUCCESS
}

fn write(out: &mut Out, job: &Job, target: &Path) -> ExitCode {
    if let Err(err) = save_job(job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}
