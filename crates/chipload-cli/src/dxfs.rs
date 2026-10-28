// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload dxf …`: a project's DXFs, each with its own placement
//! (docs/specs/drawing-sources.md §2). `place --dxf` and `update --dxf` move and update one.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{Job, Units, save_job};
use chipload_pipeline::dxfs::{ExportOptions, add_drawing_file, export_drawing, remove_dxf};
use chipload_pipeline::explode::{explode_dxf, which_dxf};
use chipload_pipeline::placement::{dxf_size, placement_text};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum DxfAction {
    /// Add a DXF or an SVG to the project, where the file has it; move it with
    /// `place --dxf ID` (an SVG shares the DXFs' numbers and commands).
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// The DXF or SVG (.svg, .svgz).
        dxf: PathBuf,
        /// The DXF's units, if it doesn't say.
        #[arg(long, value_parser = parse_units)]
        units: Option<Units>,
        /// An SVG's pixels per inch, if it gives no real size: 96 (Inkscape, browsers) or
        /// 72 (older Illustrator).
        #[arg(long)]
        px_per_inch: Option<f64>,
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
    /// Turn a DXF's shapes into drawn shapes you can edit; the project no longer knows the
    /// DXF (docs/specs/explode-and-join.md). Operations keep cutting the same shapes.
    Explode {
        /// The project (.chipload).
        project: PathBuf,
        /// The DXF's id (as `dxf list` shows it); not needed when the project has one.
        id: Option<u64>,
        /// Then join ends closer than this (as `shape join --gap`), e.g. 0.01 or 0.05mm.
        #[arg(long, value_parser = crate::lengths::length)]
        join: Option<f64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Write the drawing as a DXF: every shape the canvas shows, where it shows it, on its
    /// layers, in mm (docs/specs/dxf-export.md). The file is read back before it is written.
    Export {
        /// The project (.chipload).
        project: PathBuf,
        /// The DXF to write.
        #[arg(short, long)]
        output: PathBuf,
        /// In inches instead of mm.
        #[arg(long)]
        inch: bool,
        /// Hidden groups and layers too.
        #[arg(long)]
        hidden: bool,
        /// Only this layer (repeat for more).
        #[arg(long = "layer")]
        layers: Vec<String>,
        /// Only these shapes, by id (as `inspect` lists them, without the S).
        #[arg(long = "shape", num_args = 1..)]
        shapes: Vec<u64>,
        /// Construction geometry too, each on a layer of its own (LAYER_construction).
        #[arg(long)]
        construction: bool,
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
            px_per_inch,
            name,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let added = add_drawing_file(&job, &dxf, units, px_per_inch, name.as_deref());
            let (job, report) = match added {
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
        DxfAction::Export {
            project,
            output,
            inch,
            hidden,
            layers,
            shapes,
            construction,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let options = ExportOptions {
                inch,
                hidden,
                layers,
                shapes,
                construction,
            };
            let (exported, notes) = match export_drawing(&job, &options) {
                Ok(done) => done,
                Err(err) => return out.fail(err),
            };
            let text = String::from_utf8_lossy(&exported.bytes);
            if let Err(err) = chipload_pipeline::write_atomically(&output, &text) {
                return out.fail(format!("Cannot write {}: {err}", output.display()));
            }
            for note in &notes {
                out.line(note.english());
            }
            out.wrote(&output, "");
            out.set("entities", json!(exported.entities));
            out.set("layers", json!(exported.layers));
            out.set("renamed", json!(exported.renamed));
            ExitCode::SUCCESS
        }
        DxfAction::Explode {
            project,
            id,
            join,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let done = which_dxf(&job, id).and_then(|id| explode_dxf(&job, id, join));
            let (job, report) = match done {
                Ok(done) => done,
                Err(err) => return out.fail(err),
            };
            for line in &report.lines {
                out.line(line.clone());
            }
            out.set("shapes", json!(report.shapes));
            out.set("removed", json!(report.removed));
            write(out, &job, output.as_deref().unwrap_or(&project))
        }
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
                        out.line(line.english());
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
            "format": dxf.format,
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
