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
use chipload_job::{Geometry, Job, Spindle, Units, WorkZero, load_job, save_job};
use chipload_pipeline::report::{
    format_contours, format_import_report, format_material_left, format_shapes, severity_name,
};
use chipload_pipeline::shapes::{add_shape, delete_shape, set_shape};
use chipload_pipeline::{
    PipelineError, load_drawing, new_project, run_job, update_drawing, write_atomically,
};
use chipload_post::{load_builtin_post, render_program};
use chipload_toolpath::arcs::Circle;
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
    /// Create a project from a DXF.
    Import {
        /// The DXF.
        dxf: PathBuf,
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
    /// Draw in a project: add, change or delete circles, rectangles and polylines.
    Shape {
        #[command(subcommand)]
        action: ShapeAction,
    },
    /// Print Chipload's licence and the licences of every third-party component.
    Licenses,
}

#[derive(Subcommand)]
enum ShapeAction {
    /// Add a drawn shape; prints its id.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        kind: KindArg,
        #[command(flatten)]
        values: ShapeValues,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Change a drawn shape's values or layer; it keeps its id.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        #[command(flatten)]
        values: ShapeValues,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Delete a drawn shape. Operations that used it are flagged.
    Delete {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum KindArg {
    Circle,
    Rectangle,
    Polyline,
}

/// Defining values, all in mm; the angle in degrees.
#[derive(clap::Args)]
struct ShapeValues {
    /// Centre "X,Y" (circle, rectangle).
    #[arg(long, allow_hyphen_values = true)]
    center: Option<String>,
    #[arg(long)]
    radius: Option<f64>,
    /// Along the rectangle's own X axis.
    #[arg(long)]
    length: Option<f64>,
    /// Along the rectangle's own Y axis.
    #[arg(long)]
    width: Option<f64>,
    /// Rotation about the centre, degrees, counter-clockwise.
    #[arg(long, allow_hyphen_values = true)]
    angle_deg: Option<f64>,
    /// Polyline points "X,Y X,Y ...".
    #[arg(long, allow_hyphen_values = true)]
    points: Option<String>,
    /// The polyline doesn't return to its first point.
    #[arg(long, conflicts_with = "closed")]
    open: bool,
    /// The polyline returns to its first point (the default for new ones).
    #[arg(long)]
    closed: bool,
    #[arg(long)]
    layer: Option<String>,
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
            units: u,
            output,
        } => import(&dxf, units(u), &output),
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
        Command::Shape { action } => shape(action),
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

fn import(dxf: &Path, units: Option<Units>, output: &Path) -> ExitCode {
    let imported = match shapes_from_dxf(dxf, units) {
        Ok(imported) => imported,
        Err(err) => return fail(err),
    };
    let line = format!(
        "Imported {} ({} shapes).",
        imported.source.file,
        imported.shapes.len()
    );
    if let Err(err) = save_job(&new_project(imported), output) {
        return fail(err);
    }
    println!("{line}");
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

fn shape(action: ShapeAction) -> ExitCode {
    let (project, output) = match &action {
        ShapeAction::Add {
            project, output, ..
        }
        | ShapeAction::Set {
            project, output, ..
        }
        | ShapeAction::Delete {
            project, output, ..
        } => (project.clone(), output.clone()),
    };
    let job = match load(&project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut flagged_ops = 0;
    let changed = match action {
        ShapeAction::Add { kind, values, .. } => new_geometry(kind, &values)
            .map_err(PipelineError::Job)
            .and_then(|geometry| {
                let layer = values.layer.as_deref().unwrap_or("0");
                add_shape(&job, layer, geometry)
            })
            .map(|(job, id)| (job, vec![format!("Added shape S{id}.")])),
        ShapeAction::Set { id, values, .. } => job
            .drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| PipelineError::Job(format!("there is no shape {id}")))
            .and_then(|shape| {
                let geometry =
                    changed_geometry(&shape.geometry, &values).map_err(PipelineError::Job)?;
                let layer = values.layer.clone().unwrap_or_else(|| shape.layer.clone());
                set_shape(&job, id, &layer, geometry)
            })
            .map(|job| (job, vec![format!("Changed shape S{id}.")])),
        ShapeAction::Delete { id, .. } => delete_shape(&job, id).map(|(job, flagged)| {
            flagged_ops = flagged.len();
            let mut lines = vec![format!("Deleted shape S{id}.")];
            lines.extend(flagged.into_iter().map(|l| format!("warning: {l}")));
            (job, lines)
        }),
    };
    let (job, lines) = match changed {
        Ok(done) => done,
        Err(err) => return fail(err),
    };
    let target = output.as_deref().unwrap_or(&project);
    if let Err(err) = save_job(&job, target) {
        return fail(err);
    }
    for line in lines {
        println!("{line}");
    }
    println!("Wrote {}", target.display());
    if flagged_ops == 0 {
        ExitCode::SUCCESS
    } else {
        // As for `update`: written, but some operations can't run until fixed.
        ExitCode::from(EXIT_FAILED)
    }
}

fn point_arg(text: &str) -> Result<[f64; 2], String> {
    let parsed = text
        .split_once(',')
        .and_then(|(x, y)| Some([x.trim().parse::<f64>().ok()?, y.trim().parse::<f64>().ok()?]));
    parsed.ok_or_else(|| format!("expected a point as X,Y, got {text:?}"))
}

fn points_arg(text: &str) -> Result<Vec<[f64; 2]>, String> {
    text.split_whitespace().map(point_arg).collect()
}

fn need<T>(value: Option<T>, what: &str, kind: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("a {kind} needs --{what}"))
}

fn new_geometry(kind: KindArg, v: &ShapeValues) -> Result<Geometry, String> {
    let center = v.center.as_deref().map(point_arg).transpose()?;
    Ok(match kind {
        KindArg::Circle => Geometry::Circle {
            center: need(center, "center", "circle")?,
            radius: need(v.radius, "radius", "circle")?,
        },
        KindArg::Rectangle => Geometry::Rectangle {
            center: need(center, "center", "rectangle")?,
            length: need(v.length, "length", "rectangle")?,
            width: need(v.width, "width", "rectangle")?,
            angle: v.angle_deg.unwrap_or(0.0).to_radians(),
        },
        KindArg::Polyline => Geometry::Polyline {
            closed: !v.open,
            points: points_arg(need(v.points.as_deref(), "points", "polyline")?)?,
        },
    })
}

/// The shape's geometry with the given values replaced; values that don't belong to its
/// kind are refused rather than ignored.
fn changed_geometry(current: &Geometry, v: &ShapeValues) -> Result<Geometry, String> {
    let center = v.center.as_deref().map(point_arg).transpose()?;
    let not_for = |kind: &str, given: &[(&str, bool)]| -> Result<(), String> {
        match given.iter().find(|(_, set)| *set) {
            Some((name, _)) => Err(format!("--{name} doesn't apply to a {kind}")),
            None => Ok(()),
        }
    };
    let mut geometry = current.clone();
    match &mut geometry {
        Geometry::Path { .. } => {}
        Geometry::Circle { center: c, radius } => {
            not_for(
                "circle",
                &[
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("angle-deg", v.angle_deg.is_some()),
                    ("points", v.points.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *radius = v.radius.unwrap_or(*radius);
        }
        Geometry::Rectangle {
            center: c,
            length,
            width,
            angle,
        } => {
            not_for(
                "rectangle",
                &[
                    ("radius", v.radius.is_some()),
                    ("points", v.points.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *length = v.length.unwrap_or(*length);
            *width = v.width.unwrap_or(*width);
            if let Some(degrees) = v.angle_deg {
                *angle = degrees.to_radians();
            }
        }
        Geometry::Polyline { closed, points } => {
            not_for(
                "polyline",
                &[
                    ("center", center.is_some()),
                    ("radius", v.radius.is_some()),
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("angle-deg", v.angle_deg.is_some()),
                ],
            )?;
            if let Some(text) = &v.points {
                *points = points_arg(text)?;
            }
            if v.open {
                *closed = false;
            } else if v.closed {
                *closed = true;
            }
        }
    }
    Ok(geometry)
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
    println!("Work zero: {}", job.work_zero_text());
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
                Some("arc_cw") => MoveKind::ArcCw,
                Some("arc_ccw") => MoveKind::ArcCcw,
                _ => {
                    return Err(
                        "move.kind must be \"rapid\", \"feed\", \"arc_cw\" or \"arc_ccw\""
                            .to_string(),
                    );
                }
            };
            let circle = match m.get("circle") {
                None | Some(Value::Null) => None,
                Some(c) => {
                    let center = c
                        .get("center")
                        .and_then(Value::as_array)
                        .and_then(|xy| Some((xy.first()?.as_f64()?, xy.get(1)?.as_f64()?)))
                        .ok_or("move.circle.center must be [x, y]")?;
                    Some(Circle {
                        center,
                        radius: number(c, "radius")?,
                    })
                }
            };
            Ok(Move {
                kind,
                x: optional(m, "x")?,
                y: optional(m, "y")?,
                z: optional(m, "z")?,
                feed: optional(m, "feed")?,
                circle,
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
                    "open": p.open,
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
                MoveKind::ArcCw => "arc_cw",
                MoveKind::ArcCcw => "arc_ccw",
            };
            // On a feed: the circle the chord lies on (merged into an arc by `render` when
            // the post writes arcs). On an arc: its circle.
            let circle = m
                .circle
                .map(|c| json!({"center": [c.center.0, c.center.1], "radius": c.radius}));
            json!({"kind": kind, "x": m.x, "y": m.y, "z": m.z, "feed": m.feed, "circle": circle})
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
    let mut shapes = None;
    let mut work_zero = None;
    let loaded = if is_dxf(path) {
        import_dxf(path, units).map_err(|e| e.to_string())
    } else {
        load_job(path).map_err(|e| e.to_string()).and_then(|job| {
            shapes = Some(job.drawing.shapes.clone());
            // Only when set, so a fresh project still inspects exactly like its DXF.
            if job.machine.work_zero != (WorkZero::DrawingOrigin {}) {
                work_zero = Some(format!("Work zero: {}", job.work_zero_text()));
            }
            load_drawing(&job).map_err(|e| e.to_string())
        })
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
        .chain(shapes.as_deref().map(format_shapes).unwrap_or_default())
        .chain(work_zero)
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
