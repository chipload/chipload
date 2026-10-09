// SPDX-License-Identifier: GPL-3.0-or-later
//! Chipload command line (Rust port, in progress; see docs/decisions/0004).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_check::{CheckIssue, check_gcode, limits_from_job};
use chipload_core::chain::Contour;
use chipload_dxf::{ImportResult, Severity, UnitsFrom, import_dxf, units_name};
use chipload_job::{Units, load_job};
use chipload_toolpath::model::{MoveKind, Program};
use chipload_toolpath::plan_job;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

const EXIT_FAILED: u8 = 1;

#[derive(Parser)]
#[command(name = "chipload", version, about = "2.5D CAM (Rust port in progress)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Dump entities, units, contours and issues of a DXF.
    Inspect {
        dxf: PathBuf,
        /// Units of the drawing; required if the DXF does not declare them.
        #[arg(long)]
        units: Option<UnitsArg>,
        /// Print the result as JSON (used by the conformance tests).
        #[arg(long)]
        json: bool,
    },
    /// Plan the toolpath for a job (no G-code yet: the post-processor isn't ported).
    Plan {
        dxf: PathBuf,
        /// Job file.
        #[arg(long)]
        job: PathBuf,
        /// Print the toolpath as JSON (used by the conformance tests).
        #[arg(long)]
        json: bool,
    },
    /// Check a G-code program against a job's stock, safe height and machine limits.
    Check {
        /// G-code file to check.
        gcode: PathBuf,
        /// Job file the program was generated from.
        #[arg(long)]
        job: PathBuf,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum UnitsArg {
    Mm,
    Inch,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Inspect { dxf, units, json } => {
            let units = units.map(|u| match u {
                UnitsArg::Mm => Units::Mm,
                UnitsArg::Inch => Units::Inch,
            });
            inspect(&dxf, units, json)
        }
        Command::Plan { dxf, job, json } => plan(&dxf, &job, json),
        Command::Check { gcode, job, json } => check(&gcode, &job, json),
    }
}

fn plan(dxf: &Path, job_path: &Path, as_json: bool) -> ExitCode {
    let job = match load_job(job_path) {
        Ok(job) => job,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let imported = match import_dxf(dxf, Some(job.source.units)) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let plan = match plan_job(&job, &imported.contours) {
        Ok(plan) => plan,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    if as_json {
        println!(
            "{}",
            json!({"program": program_json(&plan.program), "notes": plan.notes})
        );
    } else {
        for note in &plan.notes {
            println!("warning: {note}");
        }
        println!("Planned {} moves.", plan.program.moves.len());
    }
    ExitCode::SUCCESS
}

fn program_json(program: &Program) -> Value {
    let moves: Vec<Value> = program
        .moves
        .iter()
        .map(|m| {
            let kind = match m.kind {
                MoveKind::Rapid => "rapid",
                MoveKind::Feed => "feed",
            };
            json!({"kind": kind, "x": m.x, "y": m.y, "z": m.z, "feed": m.feed})
        })
        .collect();
    json!({
        "tool_id": program.tool_id,
        "tool_diameter": program.tool_diameter,
        "rpm": program.rpm,
        "spindle": match program.spindle { chipload_job::Spindle::Cw => "cw", chipload_job::Spindle::Ccw => "ccw" },
        "safe_z": program.safe_z,
        "moves": moves,
    })
}

fn inspect(path: &Path, units: Option<Units>, as_json: bool) -> ExitCode {
    let result = match import_dxf(path, units) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    if as_json {
        println!("{}", import_json(&result));
        return ExitCode::SUCCESS;
    }
    let report = &result.report;
    let from = match report.units_from {
        UnitsFrom::Dxf => "from DXF",
        UnitsFrom::Job => "from job",
    };
    println!("Units: {} ({from})", units_name(report.units));
    for (kind, count) in &report.imported {
        println!("Imported: {count} {kind}");
    }
    for (kind, count) in &report.flattened {
        println!("Flattened to lines: {count} {kind}");
    }
    for (kind, count) in &report.ignored {
        println!("Ignored: {count} {kind}");
    }
    for issue in &report.issues {
        println!("{}: {}", severity_name(issue.severity), issue.message);
    }
    for contour in &result.contours {
        let state = match (
            &contour.invalid_reason,
            contour.closed,
            contour.nesting_depth,
        ) {
            (Some(reason), _, _) => format!("invalid ({reason})"),
            (None, false, _) => "open".to_string(),
            (None, true, Some(depth)) if depth % 2 == 1 => format!("closed, hole (depth {depth})"),
            (None, true, depth) => format!("closed, outer (depth {})", depth.unwrap_or(0)),
        };
        println!(
            "Contour {}: {state}, layer {}, length {:.3} mm",
            contour.id,
            contour.layers().join(", "),
            contour_length(contour)
        );
    }
    ExitCode::SUCCESS
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn contour_length(contour: &Contour) -> f64 {
    contour.parts.iter().map(|p| p.segment.prim.length()).sum()
}

fn import_json(result: &ImportResult) -> Value {
    let report = &result.report;
    let point = |p: (f64, f64)| json!([p.0, p.1]);
    let contours: Vec<Value> = result
        .contours
        .iter()
        .map(|c| {
            let mut handles: Vec<&str> =
                c.parts.iter().map(|p| p.segment.handle.as_str()).collect();
            handles.sort_unstable();
            handles.dedup();
            json!({
                "id": c.id,
                "closed": c.closed,
                "layers": c.layers(),
                "handles": handles,
                "nesting_depth": c.nesting_depth,
                "invalid_reason": c.invalid_reason,
                "length": contour_length(c),
                "points": c.points().into_iter().map(point).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "units": units_name(report.units),
        "units_from": match report.units_from { UnitsFrom::Dxf => "dxf", UnitsFrom::Job => "job" },
        "imported": report.imported,
        "flattened": report.flattened,
        "ignored": report.ignored,
        "issues": report.issues.iter().map(|i| json!({
            "severity": severity_name(i.severity),
            "code": i.code,
            "message": i.message,
            "location": i.location.map(point),
        })).collect::<Vec<_>>(),
        "bbox": report.bbox.map(|(lo, hi)| json!([point(lo), point(hi)])),
        "contours": contours,
    })
}

fn check(gcode: &Path, job_path: &Path, as_json: bool) -> ExitCode {
    let job = match load_job(job_path) {
        Ok(job) => job,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let text = match std::fs::read_to_string(gcode) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {}: {err}", gcode.display());
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let report = check_gcode(&text, &limits_from_job(&job));
    if as_json {
        let issues = |list: &[CheckIssue]| -> Vec<serde_json::Value> {
            list.iter()
                .map(|i| json!({"line": i.line, "message": i.message}))
                .collect()
        };
        let out = json!({
            "ok": report.ok(),
            "errors": issues(&report.errors),
            "warnings": issues(&report.warnings),
        });
        println!("{out}");
    } else {
        for e in &report.errors {
            println!("error (line {}): {}", e.line, e.message);
        }
        for w in &report.warnings {
            println!("warning: {}", w.message);
        }
        println!(
            "{}",
            if report.ok() {
                "Check passed."
            } else {
                "Check FAILED."
            }
        );
    }
    if report.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_FAILED)
    }
}
