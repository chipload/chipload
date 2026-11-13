// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload relief add`: a relief in one step, the same answers in the same order as the
//! window's *Relief…* (docs/specs/relief-wizard.md §6).

use std::path::PathBuf;
use std::process::ExitCode;

use chipload_job::{ModelUp, Units};
use chipload_pipeline::library::{Library, settings_dir};
use chipload_pipeline::models::read_model;
use chipload_pipeline::relief_wizard::{Around, ReliefModel, ReliefRequest, add_relief, rim_width};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::json;

use crate::load;
use crate::models::{parse_at, parse_hull, parse_keep, parse_units, parse_up, turned};
use crate::out::Out;

#[derive(Subcommand)]
pub enum ReliefAction {
    /// Make a relief: the model (an STL file, or --model for one in the project) turned,
    /// cut and sized, centred on the board, its relief with an optional roughing pass, and
    /// what is round it, in a part of its own and in cutting order.
    Add(Box<AddRelief>),
}

/// What goes round the relief.
#[derive(Clone, Copy, ValueEnum)]
enum AroundArg {
    /// A flat round the model's outline at the relief's floor, then a ramp up to the stock
    /// top.
    Rim,
    /// The same round the rectangle round the model.
    Box,
    /// No rim: a profile outside the outline, through the board, with tabs.
    CutOut,
}

impl From<AroundArg> for Around {
    fn from(a: AroundArg) -> Self {
        match a {
            AroundArg::Rim => Around::Rim,
            AroundArg::Box => Around::Box,
            AroundArg::CutOut => Around::CutOut,
        }
    }
}

/// `relief add`'s options, in the wizard's order.
#[derive(Args)]
pub struct AddRelief {
    /// The project (.chipload).
    project: PathBuf,
    /// The STL file; leave it out with --model.
    file: Option<PathBuf>,
    /// A model already in the project (an id from `model list`), in place of a file.
    #[arg(long, conflicts_with = "file")]
    model: Option<u64>,
    /// The unit a new file is read in: mm (default) or inch.
    #[arg(long, value_parser = parse_units)]
    units: Option<Units>,
    /// Which of the model's axes points up: z (default), y, x, -z, -y or -x.
    #[arg(long, allow_hyphen_values = true, value_parser = parse_up)]
    up: Option<ModelUp>,
    /// Turned on the board, degrees counter-clockwise.
    #[arg(long, allow_hyphen_values = true)]
    rotate: Option<f64>,
    /// How much of it to keep from the top: 0.5 or 50% (default: all).
    #[arg(long, value_parser = parse_keep)]
    keep: Option<f64>,
    /// Its width on the board, mm (default: half the board's).
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// How deep the relief goes, mm (default: in proportion to the width).
    #[arg(value_parser = crate::lengths::length, long)]
    depth: Option<f64>,
    /// Where its footprint's lower-left corner goes, X,Y (default: centred on the board).
    #[arg(long, value_parser = parse_at)]
    at: Option<(f64, f64)>,
    /// How far below the board's surface the model's top lies (default 0): for an
    /// unplaned board. The rim's ramp and the roughing start at the surface.
    #[arg(value_parser = crate::lengths::length, long)]
    start_depth: Option<f64>,
    /// What goes round it.
    #[arg(long, value_enum, default_value = "rim")]
    around: AroundArg,
    /// The rim's flat round the outline or the model's box, mm (default 5, or wider: the
    /// roughing tool's diameter and its allowance on both sides).
    #[arg(value_parser = crate::lengths::length, long)]
    margin: Option<f64>,
    /// The ramp from the flat up to the stock top, degrees from horizontal: 10 to 90
    /// (default 45; 90: a straight wall).
    #[arg(long)]
    ramp: Option<f64>,
    /// How far an outline rim is filled out towards the model's convex hull: 0 (default,
    /// the outline as it is) to 1 or 100% (the hull).
    #[arg(long, value_parser = parse_hull)]
    hull: Option<f64>,
    /// A box rim's corner radius, mm, at most the margin (default 0: sharp).
    #[arg(value_parser = crate::lengths::length, long)]
    corner: Option<f64>,
    /// The ball nose that finishes it (a project tool id).
    #[arg(long)]
    tool: i64,
    /// An end mill for a roughing pass first (a project tool id).
    #[arg(long)]
    rough_tool: Option<i64>,
    /// What the roughing pass leaves for the finish, mm.
    #[arg(value_parser = crate::lengths::length, long, requires = "rough_tool")]
    allowance: Option<f64>,
    /// A smoothing pass after the roughing, with the roughing tool, its lines across the
    /// roughing's: it takes the steps the roughing leaves on slopes.
    #[arg(long, requires = "rough_tool")]
    smooth: bool,
    /// The end mill for the cut-out (default: the roughing one).
    #[arg(long)]
    end_mill: Option<i64>,
    /// Write here instead of over the project.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub fn relief(out: &mut Out, action: ReliefAction) -> ExitCode {
    match action {
        ReliefAction::Add(add) => add_one(out, *add),
    }
}

fn add_one(out: &mut Out, add: AddRelief) -> ExitCode {
    let job = match load(out, &add.project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let width = add
        .width
        .unwrap_or_else(|| (job.stock_size().0 / 2.0).round().max(1.0));
    let mut request = ReliefRequest::new(width, add.tool);
    request.up = add.up.unwrap_or_default();
    if let Some(degrees) = add.rotate {
        match turned(degrees) {
            Ok(a) => request.rotation = a,
            Err(err) => return out.fail(err),
        }
    }
    request.keep = add.keep.unwrap_or(1.0);
    request.depth = add.depth;
    request.at = add.at;
    request.sink = add.start_depth.unwrap_or(0.0);
    request.around = add.around.into();
    request.corner = add.corner.unwrap_or(0.0);
    request.hull = add.hull.unwrap_or(0.0);
    if let Some(degrees) = add.ramp {
        request.ramp = degrees.to_radians();
    }
    request.rough = add.rough_tool.map(|tool| {
        (
            tool,
            add.allowance
                .unwrap_or(chipload_pipeline::DEFAULT_ROUGHING_ALLOWANCE_MM),
        )
    });
    request.margin = add.margin.unwrap_or_else(|| rim_width(&job, request.rough));
    request.smooth = add.smooth;
    request.end_mill = add.end_mill;
    let library = settings_dir()
        .map(|dir| Library::load(&dir))
        .unwrap_or_default();
    let made = match (&add.file, add.model) {
        (Some(path), None) => {
            let bytes = match std::fs::read(path) {
                Ok(bytes) => bytes,
                Err(err) => return out.fail(format!("{}: {err}", path.display())),
            };
            let file = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            read_model(&bytes).and_then(|read| {
                let model = ReliefModel::New {
                    file: &file,
                    read: &read,
                    units: add.units.unwrap_or(Units::Mm),
                };
                add_relief(
                    &job,
                    model,
                    &request,
                    Some(&library.tools),
                    chipload_i18n::Language::En,
                )
            })
        }
        (None, Some(id)) => add_relief(
            &job,
            ReliefModel::Existing(id),
            &request,
            Some(&library.tools),
            chipload_i18n::Language::En,
        ),
        _ => return out.fail("give the STL file, or --model for one in the project"),
    };
    let (job, made) = match made {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = add.output.as_deref().unwrap_or(&add.project);
    if let Err(err) = crate::save_project(out, &job, target) {
        return out.fail(err);
    }
    let part = job.part(made.part).map_or("", |p| p.name.as_str());
    out.line(format!(
        "Made the relief \"{part}\" of model {} ({} round it), in part {}:",
        made.model,
        request.around.name(),
        made.part
    ));
    for &i in &made.operations {
        let op = &job.operations[i];
        out.line(format!("  operation {}: {}", i + 1, op.name()));
    }
    out.wrote(target, "");
    out.set("part", json!(made.part));
    out.set("model", json!(made.model));
    out.set(
        "operations",
        json!(made.operations.iter().map(|i| i + 1).collect::<Vec<_>>()),
    );
    ExitCode::SUCCESS
}
