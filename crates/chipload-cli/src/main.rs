// SPDX-License-Identifier: GPL-3.0-or-later
//! The `chipload` command line: import drawings into projects, plan and run them, check
//! G-code. Projects are `.chipload` files (docs/specs/project-file.md).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_check::{CheckIssue, check_gcode, limits_from_job};
use chipload_core::chain::Contour;
use chipload_dxf::{ImportResult, UnitsFrom, import_dxf, shapes_from_dxf, units_name};
use chipload_job::{Job, Spindle, Units, load_job, save_job};
use chipload_pipeline::report::{
    format_contours, format_import_report, format_material_left, severity_name,
};
use chipload_pipeline::{
    PipelineError, embed_drawing, load_drawing, new_project, run_job, update_drawing,
    write_atomically,
};
use chipload_post::{load_builtin_post, render_program};
use chipload_toolpath::model::{Move, MoveKind, Program};
use chipload_toolpath::plan_job;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

const EXIT_FAILED: u8 = 1;

#[derive(Parser)]
#[command(
    name = "chipload",
    version,
    about = "2.5D CAM: DXF in, checked G-code out"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Dump units, entities, contours and issues of a DXF or a project.
    Inspect {
        /// A .dxf drawing or a .chipload project.
        file: PathBuf,
        /// Units of a DXF that does not declare them.
        #[arg(long)]
        units: Option<UnitsArg>,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Create a project from a DXF, or embed the DXF of an older job (--into).
    Import {
        /// The DXF. With --into it defaults to the job's drawing, next to the job file.
        dxf: Option<PathBuf>,
        /// An older job (.json) whose drawing to embed.
        #[arg(long)]
        into: Option<PathBuf>,
        /// Units of a DXF that does not declare them.
        #[arg(long)]
        units: Option<UnitsArg>,
        /// The project to write (.chipload).
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Replace a project's imported drawing with a new version of the DXF.
    Update {
        /// The project (.chipload).
        project: PathBuf,
        /// The new DXF.
        #[arg(long)]
        from: PathBuf,
        /// Units of the new DXF, if they differ from the last import.
        #[arg(long)]
        units: Option<UnitsArg>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Plan a project's toolpath without writing G-code.
    Plan {
        /// The project (.chipload).
        project: PathBuf,
        /// Print the toolpath as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Generate checked G-code from a project.
    Run {
        /// The project (.chipload).
        project: PathBuf,
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
    /// Check a G-code program against a project's stock, safe height and machine limits.
    Check {
        /// G-code file to check.
        gcode: PathBuf,
        /// The project (or job) the program was generated from.
        #[arg(long)]
        job: PathBuf,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print Chipload's licence and the licences of every third-party component.
    Licenses,
}

#[derive(Clone, Copy, ValueEnum)]
enum UnitsArg {
    Mm,
    Inch,
}

fn main() -> ExitCode {
    let units = |u: Option<UnitsArg>| {
        u.map(|u| match u {
            UnitsArg::Mm => Units::Mm,
            UnitsArg::Inch => Units::Inch,
        })
    };
    match Cli::parse().command {
        Command::Inspect {
            file,
            units: u,
            json,
        } => inspect(&file, units(u), json),
        Command::Import {
            dxf,
            into,
            units: u,
            output,
        } => import(dxf.as_deref(), into.as_deref(), units(u), &output),
        Command::Update {
            project,
            from,
            units: u,
            output,
        } => update(&project, &from, units(u), output.as_deref()),
        Command::Plan { project, json } => plan(&project, json),
        Command::Run { project, output } => run(&project, &output),
        Command::Render { program, post } => render(&program, &post),
        Command::Check { gcode, job, json } => check(&gcode, &job, json),
        Command::Licenses => {
            println!("{}\n", chipload_pipeline::notices::short_notice());
            println!("{}", chipload_pipeline::notices::LICENSE);
            println!("{}", chipload_pipeline::notices::THIRD_PARTY);
            ExitCode::SUCCESS
        }
    }
}

fn fail(err: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {err}");
    ExitCode::from(EXIT_FAILED)
}

fn is_dxf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dxf"))
}

fn import(
    dxf: Option<&Path>,
    into: Option<&Path>,
    units: Option<Units>,
    output: &Path,
) -> ExitCode {
    let (project, report) = match into {
        Some(job_path) => {
            let job = match load_job(job_path) {
                Ok(job) => job,
                Err(err) => return fail(err),
            };
            let Some(external) = &job.drawing.external else {
                return fail(format!(
                    "{} already has its drawing embedded",
                    job_path.display()
                ));
            };
            let drawing = dxf.map_or_else(
                || job_path.with_file_name(&external.file),
                Path::to_path_buf,
            );
            match embed_drawing(&job, &drawing) {
                Ok(done) => done,
                Err(err) => return fail(err),
            }
        }
        None => {
            let Some(dxf) = dxf else {
                return fail("give a DXF to import, or --into <job> to embed a job's drawing");
            };
            match shapes_from_dxf(dxf, units) {
                Ok(imported) => {
                    let line = format!(
                        "Imported {} ({} shapes).",
                        imported.source.file,
                        imported.shapes.len()
                    );
                    (new_project(imported), vec![line])
                }
                Err(err) => return fail(err),
            }
        }
    };
    if let Err(err) = save_job(&project, output) {
        return fail(err);
    }
    for line in report {
        println!("{line}");
    }
    println!("Wrote {}", output.display());
    ExitCode::SUCCESS
}

fn update(project: &Path, from: &Path, units: Option<Units>, output: Option<&Path>) -> ExitCode {
    let job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return fail(err),
    };
    let (updated, report) = match update_drawing(&job, from, units) {
        Ok(done) => done,
        Err(err) => return fail(err),
    };
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&updated, target) {
        return fail(err);
    }
    for line in &report.lines {
        println!("{line}");
    }
    println!("Wrote {}", target.display());
    if report.lost.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "warning: {} operation(s) lost geometry and can't run until fixed",
            report.lost.len()
        );
        ExitCode::from(EXIT_FAILED)
    }
}

fn load(project: &Path) -> Result<Job, ExitCode> {
    load_job(project).map_err(fail)
}

fn run(project: &Path, output: &Path) -> ExitCode {
    let job = match load(project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let result = match run_job(&job) {
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
    for line in format_material_left(&job, &result.material_left) {
        println!("{line}");
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

fn plan(project: &Path, as_json: bool) -> ExitCode {
    let job = match load(project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let imported = match load_drawing(&job) {
        Ok(result) => result,
        Err(err) => return fail(err),
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
                "material_left": plan.material_left.iter().map(|m| json!({
                    "operation": m.operation,
                    "kind": m.kind.name(),
                    "rings": m.rings,
                })).collect::<Vec<_>>(),
            })
        );
    } else {
        for note in &plan.notes {
            println!("warning: {note}");
        }
        for line in format_material_left(&job, &plan.material_left) {
            println!("{line}");
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
    let loaded = if is_dxf(path) {
        import_dxf(path, units).map_err(|e| e.to_string())
    } else {
        load_job(path)
            .map_err(|e| e.to_string())
            .and_then(|job| load_drawing(&job).map_err(|e| e.to_string()))
    };
    let result = match loaded {
        Ok(result) => result,
        Err(err) => return fail(err),
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
        "units_from": match report.units_from { UnitsFrom::Dxf => "dxf", UnitsFrom::User => "user" },
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
