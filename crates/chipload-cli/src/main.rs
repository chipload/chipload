// SPDX-License-Identifier: GPL-3.0-or-later
//! Chipload command line (Rust port, in progress; see docs/decisions/0004).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_check::{CheckIssue, check_gcode, limits_from_job};
use chipload_core::chain::Contour;
use chipload_dxf::{ImportResult, UnitsFrom, import_dxf, units_name};
use chipload_job::{Spindle, Units, load_job};
use chipload_pipeline::report::{format_contours, format_import_report, severity_name};
use chipload_pipeline::{PipelineError, run_job, write_atomically};
use chipload_post::{load_builtin_post, render_program};
use chipload_toolpath::model::{Move, MoveKind, Program};
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
    /// Generate checked G-code from a DXF and a job file.
    Run {
        dxf: PathBuf,
        /// Job file.
        #[arg(long)]
        job: PathBuf,
        /// Where to write the G-code.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Render a toolpath JSON (as `plan --json` prints under "program") through a
    /// post-processor. For testing posts; the output is NOT checked, use `run` to cut.
    Render {
        /// Toolpath JSON file.
        program: PathBuf,
        /// Built-in post-processor.
        #[arg(long, default_value = "grbl")]
        post: String,
    },
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
        Command::Run { dxf, job, output } => run(&dxf, &job, &output),
        Command::Render { program, post } => render(&program, &post),
        Command::Check { gcode, job, json } => check(&gcode, &job, json),
    }
}

fn run(dxf: &Path, job_path: &Path, output: &Path) -> ExitCode {
    let job = match load_job(job_path) {
        Ok(job) => job,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let result = match run_job(&job, dxf) {
        Ok(result) => result,
        Err(err @ PipelineError::CheckFailed { .. }) => {
            eprintln!("error: {err}");
            eprintln!("No output written.");
            return ExitCode::from(EXIT_FAILED);
        }
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    for line in format_import_report(&result.import_report) {
        println!("{line}");
    }
    for note in &result.notes {
        println!("warning: {note}");
    }
    for warning in &result.check_warnings {
        println!("check warning: {}", warning.message);
    }
    if let Err(err) = write_atomically(output, &result.gcode) {
        eprintln!("error: cannot write {}: {err}", output.display());
        return ExitCode::from(EXIT_FAILED);
    }
    println!(
        "Wrote {} ({} lines, post {})",
        output.display(),
        result.gcode.matches('\n').count(),
        result.post.name
    );
    ExitCode::SUCCESS
}

fn render(program_path: &Path, post_name: &str) -> ExitCode {
    let parsed = std::fs::read_to_string(program_path)
        .map_err(|e| format!("cannot read {}: {e}", program_path.display()))
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()))
        .and_then(|value| program_from_json(&value));
    let program = match parsed {
        Ok(program) => program,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    match load_builtin_post(post_name).and_then(|post| render_program(&program, &post)) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(EXIT_FAILED)
        }
    }
}

fn program_from_json(value: &Value) -> Result<Program, String> {
    let number = |v: &Value, key: &str| -> Result<f64, String> {
        v.get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("program.{key} must be a number"))
    };
    let optional = |v: &Value, key: &str| -> Result<Option<f64>, String> {
        match v.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(x) => x
                .as_f64()
                .map(Some)
                .ok_or_else(|| format!("move.{key} must be a number or null")),
        }
    };
    let spindle = match value.get("spindle").and_then(Value::as_str) {
        Some("cw") => Spindle::Cw,
        Some("ccw") => Spindle::Ccw,
        _ => return Err("program.spindle must be \"cw\" or \"ccw\"".into()),
    };
    let moves = value
        .get("moves")
        .and_then(Value::as_array)
        .ok_or("program.moves must be a list")?
        .iter()
        .map(|m| {
            let kind = match m.get("kind").and_then(Value::as_str) {
                Some("rapid") => MoveKind::Rapid,
                Some("feed") => MoveKind::Feed,
                _ => return Err("move.kind must be \"rapid\" or \"feed\"".to_string()),
            };
            Ok(Move {
                kind,
                x: optional(m, "x")?,
                y: optional(m, "y")?,
                z: optional(m, "z")?,
                feed: optional(m, "feed")?,
            })
        })
        .collect::<Result<Vec<Move>, String>>()?;
    Ok(Program {
        tool_id: value
            .get("tool_id")
            .and_then(Value::as_i64)
            .ok_or("program.tool_id must be an integer")?,
        tool_diameter: number(value, "tool_diameter")?,
        rpm: number(value, "rpm")?,
        spindle,
        safe_z: number(value, "safe_z")?,
        moves,
    })
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
            json!({
                "program": program_json(&plan.program),
                "notes": plan.notes,
                "tabs": plan.tabs,
                "pockets": plan.pockets.iter().map(|p| json!({
                    "contour_ids": p.contour_ids,
                    "rings": p.rings,
                    "feed_links": p.feed_links,
                })).collect::<Vec<_>>(),
            })
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
    for line in format_import_report(&result.report)
        .into_iter()
        .chain(format_contours(&result.contours))
    {
        println!("{line}");
    }
    ExitCode::SUCCESS
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
