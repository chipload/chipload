// SPDX-License-Identifier: GPL-3.0-or-later
//! The `chipload` command line: import drawings into projects, plan and run them, check
//! G-code. Projects are `.chipload` files (docs/specs/project-file.md).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod library;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_check::{CheckIssue, check_gcode, limits_for_tool, limits_from_job};
use chipload_core::chain::Contour;
use chipload_dxf::{ImportResult, UnitsFrom, import_dxf, shapes_from_dxf, units_name};
use chipload_job::edit::Mirror;
use chipload_job::{
    Copies, CopyOrder, Geometry, Job, Placement, Spindle, Units, WorkZero, load_job,
    polyline_segment_count, save_job,
};
use chipload_pipeline::chip_load::{chip_load, chip_load_lines};
use chipload_pipeline::copies::{copies_text, pitch_for_gap};
use chipload_pipeline::estimate::{estimate, estimate_lines};
use chipload_pipeline::library::{Library, apply_library, settings_dir};
use chipload_pipeline::placement::{
    Move as PlaceMove, imported_size, place_drawing, placement_text,
};
use chipload_pipeline::report::{
    format_contours, format_import_report, format_material_left, format_shapes, severity_name,
};
use chipload_pipeline::shapes::{add_shape, copy_shapes, delete_shape, mirror_shapes, set_shape};
use chipload_pipeline::sheet::{SheetInfo, setup_sheet, sheet_path, write_setup_sheet};
use chipload_pipeline::{
    PipelineError, RunResult, empty_project, load_drawing, new_project, run_job, set_op_rest,
    set_op_tool, update_drawing, write_atomically, write_gcode_files,
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
    /// Create an empty project, to draw in or import a DXF into later (`update --from`).
    New {
        /// The project to write (.chipload).
        #[arg(short, long)]
        output: PathBuf,
        /// The machine profile to start from (default: your default machine).
        #[arg(long)]
        machine: Option<String>,
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
        /// The machine profile to start from (default: your default machine).
        #[arg(long)]
        machine: Option<String>,
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
        /// Where to write the G-code. With several tools and one file per tool:
        /// part.nc gives part-1-T1.nc, part-2-T2.nc, ...
        #[arg(short, long)]
        output: PathBuf,
        /// For this run only: one file per tool, or one file with M6 tool changes
        /// (default: the project's machine setting).
        #[arg(long)]
        tool_change: Option<library::ToolChangeArg>,
        /// Don't write the setup sheet (by default `part-sheet.html` next to `part.nc`).
        #[arg(long)]
        no_sheet: bool,
    },
    /// Simulate the material a project's program removes (docs/specs/simulation.md): print
    /// the numbers and write a shaded top view (PNG). The job is generated and checked as by
    /// `run`; no G-code is written.
    Simulate {
        /// The project (.chipload).
        project: PathBuf,
        /// Where to write the image (.png); default: the project's name with `-sim.png`.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Cell size, mm (default: automatic, from the stock's size).
        #[arg(long)]
        cell: Option<f64>,
        /// Print the numbers only.
        #[arg(long)]
        no_image: bool,
        /// The stock after only the first N moves of the program (docs/specs/playback.md
        /// §5), for tests and scripts.
        #[arg(long, value_name = "N")]
        after: Option<usize>,
        /// Also print a hash of the heights (for regression tests).
        #[arg(long, hide = true)]
        hash: bool,
    },
    /// Write a project's setup sheet for the operator (HTML), without the G-code. The job is
    /// generated and checked as by `run`.
    Sheet {
        /// The project (.chipload).
        project: PathBuf,
        /// Where to write the sheet (.html).
        #[arg(short, long)]
        output: PathBuf,
        /// The G-code the sheet is for, as given to `run -o`, for its file names
        /// (default: the project's name with the post's extension).
        #[arg(long)]
        gcode: Option<PathBuf>,
        /// For this sheet only: one file per tool, or one file with M6 tool changes.
        #[arg(long)]
        tool_change: Option<library::ToolChangeArg>,
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
        /// The project's tool that cuts this program, for its stock margin (one file of a
        /// set written one file per tool). Default: the largest tool the project uses.
        #[arg(long)]
        tool: Option<i64>,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Change an operation of a project.
    Op {
        #[command(subcommand)]
        action: OpAction,
    },
    /// Draw in a project: add, change or delete circles, rectangles and polylines.
    Shape {
        #[command(subcommand)]
        action: ShapeAction,
    },
    /// Your tool library: tools kept for every project (copied into a project when used).
    Tool {
        #[command(subcommand)]
        action: library::ToolAction,
    },
    /// Your machine profiles: machine values a new project starts with.
    Machine {
        #[command(subcommand)]
        action: library::MachineAction,
    },
    /// Post-processors: the built-in ones and yours (docs/specs/user-posts.md).
    Post {
        #[command(subcommand)]
        action: library::PostAction,
    },
    /// Cut the part several times in a grid (docs/specs/copies-grid.md).
    Copies {
        /// The project (.chipload).
        project: PathBuf,
        /// Columns x rows, e.g. "3x2".
        #[arg(long)]
        grid: Option<String>,
        /// Pitch "DX,DY" in mm, copy to copy; negative grows left or down.
        #[arg(long, allow_hyphen_values = true, conflicts_with = "gap")]
        pitch: Option<String>,
        /// Set the pitch so that this many mm are left between the cuts of neighbours.
        #[arg(long)]
        gap: Option<f64>,
        /// The cut order.
        #[arg(long)]
        order: Option<OrderArg>,
        /// Back to a single part.
        #[arg(long, conflicts_with_all = ["grid", "pitch", "gap", "order"])]
        off: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move, turn or scale the imported drawing (docs/specs/drawing-placement.md,
    /// drawing-rotation.md, drawing-scale.md). Without an option, print where it is.
    Place {
        /// The project (.chipload).
        project: PathBuf,
        /// Move by "DX,DY" mm.
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["to", "align"])]
        by: Option<String>,
        /// Put it "DX,DY" mm from where the DXF has it; 0,0 is as drawn.
        #[arg(long, allow_hyphen_values = true, conflicts_with = "align")]
        to: Option<String>,
        /// Align the drawing's extent with the stock.
        #[arg(long)]
        align: Option<AlignArg>,
        /// With --align bottom-left: how far in from the stock's corner, mm.
        #[arg(long)]
        margin: Option<f64>,
        /// Turn it about its centre, degrees counter-clockwise: 90, 180, 270 or -90. Before
        /// --align when both are given.
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["by", "to"])]
        rotate: Option<i32>,
        /// Scale it about its centre by this factor (> 0), on top of any scale it has. After
        /// --rotate, before --align.
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["by", "to", "width", "height"])]
        scale: Option<f64>,
        /// Scale it about its centre until it is this wide, mm (the height follows).
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["by", "to", "height"])]
        width: Option<f64>,
        /// Scale it about its centre until it is this high, mm (the width follows).
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["by", "to"])]
        height: Option<f64>,
        /// Back to where the DXF has it: no turn, no scale, no move.
        #[arg(
            long,
            conflicts_with_all = ["by", "to", "align", "margin", "rotate", "scale", "width", "height"]
        )]
        reset: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print Chipload's licence and the licences of every third-party component.
    Licenses,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum AlignArg {
    /// Bottom-left corner on the stock's, --margin mm in.
    BottomLeft,
    /// Centre on the stock's centre.
    Center,
}

#[derive(Clone, Copy, ValueEnum)]
enum OrderArg {
    /// Each operation on every copy, then the next operation.
    ByOperation,
    /// Every operation on one copy, then the next copy.
    ByPart,
}

/// What `chipload copies` was asked to change.
struct CopiesArgs {
    grid: Option<String>,
    pitch: Option<String>,
    gap: Option<f64>,
    order: Option<OrderArg>,
    off: bool,
}

#[derive(Subcommand)]
enum OpAction {
    /// Set the tool an operation cuts with (docs/specs/tool-per-operation.md §7). The tool
    /// must be in the project: copy one in from your library with `tool use`.
    Tool {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation, 1-based as listed.
        #[arg(long)]
        op: usize,
        /// The tool's id in the project.
        #[arg(long)]
        tool: i64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Make a pocket clear only what a larger tool leaves (docs/specs/rest-machining.md).
    Rest {
        /// The project (.chipload).
        project: PathBuf,
        /// The pocket, 1-based as listed.
        #[arg(long)]
        op: usize,
        /// The larger tool's id in the project.
        #[arg(long, conflicts_with = "none", required_unless_present = "none")]
        after_tool: Option<i64>,
        /// Clear the whole pocket again.
        #[arg(long)]
        none: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
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
    /// Copy drawn shapes, moved by --dx and --dy, on their layers; prints the new ids.
    Copy {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far to move the copies right, mm.
        #[arg(long, allow_hyphen_values = true)]
        dx: f64,
        /// How far to move the copies up, mm.
        #[arg(long, allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Mirror drawn shapes in place, about the middle of the box round them. They keep
    /// their ids, so the operations that cut them follow.
    Mirror {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// x: flip the X coordinates (left becomes right); y: flip Y (up becomes down).
        #[arg(long)]
        axis: AxisArg,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum AxisArg {
    X,
    Y,
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
    /// Polyline bulges "B B ...", one per segment: tan(sweep / 4) as in DXF, 0 straight,
    /// 1 a counter-clockwise half circle, -1 clockwise (default: all straight).
    #[arg(long, allow_hyphen_values = true)]
    bulges: Option<String>,
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
        Command::New { output, machine } => create(
            empty_project(),
            "New empty project.",
            &output,
            machine.as_deref(),
        ),
        Command::Import {
            dxf,
            units: u,
            output,
            machine,
        } => import(&dxf, units(u), &output, machine.as_deref()),
        Command::Update {
            project,
            from,
            units: u,
            output,
        } => update(&project, &from, units(u), output.as_deref()),
        Command::Plan { project, json } => plan(&project, json),
        Command::Run {
            project,
            output,
            tool_change,
            no_sheet,
        } => run(&project, &output, tool_change, !no_sheet),
        Command::Sheet {
            project,
            output,
            gcode,
            tool_change,
        } => sheet(&project, &output, gcode.as_deref(), tool_change),
        Command::Simulate {
            project,
            output,
            cell,
            no_image,
            after,
            hash,
        } => simulate(&project, output.as_deref(), cell, after, !no_image, hash),
        Command::Render { program, post } => render(&program, &post),
        Command::Check {
            gcode,
            job,
            tool,
            json,
        } => check(&gcode, &job, tool, json),
        Command::Op { action } => op(action),
        Command::Shape { action } => shape(action),
        Command::Tool { action } => library::tool(action),
        Command::Machine { action } => library::machine(action),
        Command::Post { action } => library::post(action),
        Command::Copies {
            project,
            grid,
            pitch,
            gap,
            order,
            off,
            output,
        } => copies(
            &project,
            CopiesArgs {
                grid,
                pitch,
                gap,
                order,
                off,
            },
            output.as_deref(),
        ),
        Command::Place {
            project,
            by,
            to,
            align,
            margin,
            rotate,
            scale,
            width,
            height,
            reset,
            output,
        } => place(
            &project,
            PlaceArgs {
                by,
                to,
                align,
                margin,
                rotate,
                scale,
                width,
                height,
                reset,
            },
            output.as_deref(),
        ),
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

fn import(dxf: &Path, units: Option<Units>, output: &Path, machine: Option<&str>) -> ExitCode {
    let imported = match shapes_from_dxf(dxf, units) {
        Ok(imported) => imported,
        Err(err) => return fail(err),
    };
    let line = format!(
        "Imported {} ({} shapes).",
        imported.source.file,
        imported.shapes.len()
    );
    create(new_project(imported), &line, output, machine)
}

/// Write a new project, started from the library's machine profile and default tool.
fn create(mut job: Job, line: &str, output: &Path, machine: Option<&str>) -> ExitCode {
    // The library only gives starting values; without a settings folder there is none.
    let library = settings_dir()
        .map(|dir| Library::load(&dir))
        .unwrap_or_default();
    let notes = match apply_library(&mut job, &library, machine) {
        Ok(notes) => notes,
        Err(err) => return fail(err),
    };
    if let Err(err) = save_job(&job, output) {
        return fail(err);
    }
    println!("{line}");
    for problem in &library.problems {
        eprintln!("warning: {problem}");
    }
    for note in notes {
        println!("{note}");
    }
    println!("Wrote {}", output.display());
    ExitCode::SUCCESS
}

fn copies(project: &Path, args: CopiesArgs, output: Option<&Path>) -> ExitCode {
    let mut job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return fail(err),
    };
    match changed_copies(&job, args) {
        Ok(copies) => job.copies = copies,
        Err(err) => return fail(err),
    }
    let problems = job.validate();
    if !problems.is_empty() {
        return fail(problems.join("; "));
    }
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&job, target) {
        return fail(err);
    }
    match copies_text(&job) {
        Some(text) => println!("Copies: {text}"),
        None => println!("Copies: off (one part)"),
    }
    // Overlap is refused when the project is run; say so now.
    if job.copies.is_some_and(|c| c.count() > 1)
        && let Ok(drawing) = load_drawing(&job)
        && let Err(err) = plan_job(&job, &drawing.contours)
        && err.to_string().starts_with("Copies overlap")
    {
        eprintln!("warning: {err}");
    }
    println!("Wrote {}", target.display());
    ExitCode::SUCCESS
}

fn changed_copies(job: &Job, args: CopiesArgs) -> Result<Option<Copies>, String> {
    if args.off {
        return Ok(None);
    }
    let mut copies = job.copies.unwrap_or(Copies {
        columns: 1,
        rows: 1,
        dx: 0.0,
        dy: 0.0,
        order: CopyOrder::ByOperation,
    });
    if job.copies.is_none() && args.grid.is_none() {
        return Err("the project has no copies yet: give --grid (and --pitch or --gap)".into());
    }
    if let Some(grid) = &args.grid {
        let (columns, rows) = grid
            .split_once(['x', 'X'])
            .and_then(|(c, r)| Some((c.trim().parse().ok()?, r.trim().parse().ok()?)))
            .ok_or_else(|| format!("--grid wants columns x rows like 3x2, got \"{grid}\""))?;
        copies.columns = columns;
        copies.rows = rows;
    }
    if let Some(pitch) = &args.pitch {
        [copies.dx, copies.dy] = point_arg(pitch).map_err(|e| format!("--pitch: {e}"))?;
    }
    if let Some(order) = args.order {
        copies.order = match order {
            OrderArg::ByOperation => CopyOrder::ByOperation,
            OrderArg::ByPart => CopyOrder::ByPart,
        };
    }
    if let Some(gap) = args.gap {
        (copies.dx, copies.dy) =
            pitch_for_gap(job, gap, (copies.dx, copies.dy)).map_err(|e| e.to_string())?;
    } else if job.copies.is_none() && args.pitch.is_none() && copies.count() > 1 {
        return Err("give the spacing with --pitch DX,DY or --gap MM".into());
    }
    Ok(Some(copies))
}

/// What `chipload place` was asked to do.
struct PlaceArgs {
    by: Option<String>,
    to: Option<String>,
    align: Option<AlignArg>,
    margin: Option<f64>,
    rotate: Option<i32>,
    scale: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    reset: bool,
}

fn place(project: &Path, args: PlaceArgs, output: Option<&Path>) -> ExitCode {
    let job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return fail(err),
    };
    let steps = match place_moves(args) {
        Ok(steps) => steps,
        Err(err) => return fail(err),
    };
    if steps.is_empty() {
        return match &job.drawing.source {
            Some(source) => {
                println!("Drawing: {}", placement_text(source.placement));
                if let Some((w, h)) = imported_size(&job.drawing.shapes) {
                    println!("Size: {w:.3} × {h:.3} mm");
                }
                ExitCode::SUCCESS
            }
            None => fail("the project has no imported drawing"),
        };
    }
    let mut placed = job;
    let mut lines = Vec::new();
    for how in steps {
        match place_drawing(&placed, how) {
            Ok((next, report)) => {
                placed = next;
                lines.extend(report.lines);
            }
            Err(err) => return fail(err),
        }
    }
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&placed, target) {
        return fail(err);
    }
    for line in &lines {
        println!("{line}");
    }
    println!("Wrote {}", target.display());
    ExitCode::SUCCESS
}

/// The moves `place` was asked for, in order: a turn, a scale, then a move or an alignment.
/// Empty: just print the placement.
fn place_moves(args: PlaceArgs) -> Result<Vec<PlaceMove>, String> {
    if args.reset {
        return Ok(vec![PlaceMove::Reset]);
    }
    let mut steps = Vec::new();
    if let Some(degrees) = args.rotate {
        if degrees % 90 != 0 {
            return Err(format!(
                "--rotate turns in steps of 90 degrees (90, 180, 270, -90), got {degrees}"
            ));
        }
        steps.push(PlaceMove::Turn(degrees / 90));
    }
    if let Some(factor) = args.scale {
        steps.push(PlaceMove::Scale(factor));
    }
    if let Some(width) = args.width {
        steps.push(PlaceMove::Width(width));
    }
    if let Some(height) = args.height {
        steps.push(PlaceMove::Height(height));
    }
    steps.extend(place_move(args.by, args.to, args.align, args.margin)?);
    Ok(steps)
}

/// The move or alignment `place` was asked for, if any.
fn place_move(
    by: Option<String>,
    to: Option<String>,
    align: Option<AlignArg>,
    margin: Option<f64>,
) -> Result<Option<PlaceMove>, String> {
    if margin.is_some() && align != Some(AlignArg::BottomLeft) {
        return Err("--margin goes with --align bottom-left".into());
    }
    if let Some(by) = by {
        let [dx, dy] = point_arg(&by).map_err(|e| format!("--by: {e}"))?;
        return Ok(Some(PlaceMove::By(dx, dy)));
    }
    if let Some(to) = to {
        let [dx, dy] = point_arg(&to).map_err(|e| format!("--to: {e}"))?;
        return Ok(Some(PlaceMove::To(dx, dy)));
    }
    Ok(align.map(|align| match align {
        AlignArg::BottomLeft => PlaceMove::BottomLeft {
            margin: margin.unwrap_or(0.0),
        },
        AlignArg::Center => PlaceMove::Center,
    }))
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
        }
        | ShapeAction::Copy {
            project, output, ..
        }
        | ShapeAction::Mirror {
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
        ShapeAction::Copy { ids, dx, dy, .. } => {
            copy_shapes(&job, &ids, (dx, dy)).map(|(job, new)| {
                let names: Vec<String> = new.iter().map(|id| format!("S{id}")).collect();
                (job, vec![format!("Added shapes {}.", names.join(", "))])
            })
        }
        ShapeAction::Mirror { ids, axis, .. } => {
            let mirror = match axis {
                AxisArg::X => Mirror::LeftRight,
                AxisArg::Y => Mirror::UpDown,
            };
            mirror_shapes(&job, &ids, mirror).map(|job| {
                let names: Vec<String> = ids.iter().map(|id| format!("S{id}")).collect();
                (job, vec![format!("Mirrored shapes {}.", names.join(", "))])
            })
        }
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

fn bulges_arg(text: &str) -> Result<Vec<f64>, String> {
    text.split_whitespace()
        .map(|b| {
            b.parse::<f64>()
                .map_err(|_| format!("expected a bulge as a number, got {b:?}"))
        })
        .collect()
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
        KindArg::Polyline => {
            let closed = !v.open;
            let points = points_arg(need(v.points.as_deref(), "points", "polyline")?)?;
            let bulges = match &v.bulges {
                Some(text) => bulges_arg(text)?,
                None => vec![0.0; polyline_segment_count(points.len(), closed)],
            };
            Geometry::Polyline {
                closed,
                points,
                bulges,
            }
        }
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
                    ("bulges", v.bulges.is_some()),
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
                    ("bulges", v.bulges.is_some()),
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
        Geometry::Polyline {
            closed,
            points,
            bulges,
        } => {
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
            let count = polyline_segment_count(points.len(), *closed);
            if let Some(text) = &v.bulges {
                *bulges = bulges_arg(text)?;
            } else if bulges.len() != count {
                // The segments changed: straight ones stay straight, but which bend a curved
                // one should keep isn't ours to guess.
                if bulges.iter().any(|&b| b != 0.0) {
                    return Err(format!(
                        "the polyline now has {count} segments and some were curved: \
                         give --bulges too"
                    ));
                }
                *bulges = vec![0.0; count];
            }
        }
    }
    Ok(geometry)
}

fn load(project: &Path) -> Result<Job, ExitCode> {
    load_job(project).map_err(fail)
}

/// Load a project and run it, as `run` and `sheet` do; errors are printed.
fn load_and_run(
    project: &Path,
    tool_change: Option<library::ToolChangeArg>,
) -> Result<(Job, RunResult), ExitCode> {
    let mut job = load(project)?;
    if let Some(change) = tool_change {
        job.machine.tool_change = change.value();
    }
    match run_job(&job) {
        Ok(result) => Ok((job, result)),
        Err(err @ PipelineError::CheckFailed { .. }) => {
            eprintln!("error: {err}");
            eprintln!("No output written.");
            Err(ExitCode::from(EXIT_FAILED))
        }
        Err(err) => {
            eprintln!("error: {err}");
            Err(ExitCode::from(EXIT_FAILED))
        }
    }
}

/// What the sheet says it was made from: the project's file name and this version.
fn sheet_info(project: &Path) -> SheetInfo {
    SheetInfo {
        project: project.file_name().map_or_else(
            || project.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        written_by: format!("chipload {}", env!("CARGO_PKG_VERSION")),
    }
}

fn sheet(
    project: &Path,
    output: &Path,
    gcode: Option<&Path>,
    tool_change: Option<library::ToolChangeArg>,
) -> ExitCode {
    let (job, result) = match load_and_run(project, tool_change) {
        Ok(run) => run,
        Err(code) => return code,
    };
    let gcode = gcode.map_or_else(
        || project.with_extension(result.post.file_extension.trim_start_matches('.')),
        Path::to_path_buf,
    );
    let html = setup_sheet(&job, &result, &gcode, &sheet_info(project));
    if let Err(err) = write_atomically(output, &html) {
        eprintln!("error: cannot write {}: {err}", output.display());
        return ExitCode::from(EXIT_FAILED);
    }
    println!("Wrote {}", output.display());
    ExitCode::SUCCESS
}

fn simulate(
    project: &Path,
    output: Option<&Path>,
    cell: Option<f64>,
    after: Option<usize>,
    image: bool,
    hash: bool,
) -> ExitCode {
    let (job, result) = match load_and_run(project, None) {
        Ok(run) => run,
        Err(code) => return code,
    };
    let mut program = result.program.clone();
    if let Some(done) = after {
        let total = program.moves.len();
        if done > total {
            eprintln!("error: --after {done}: the program has {total} moves");
            return ExitCode::from(EXIT_FAILED);
        }
        program.moves.truncate(done);
        println!("After {done} of {total} moves");
    }
    let sim = match chipload_pipeline::simulate::simulate(
        &job,
        &program,
        &result.groups,
        cell,
        &chipload_pipeline::Cancel::new(),
    ) {
        Ok(sim) => sim,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    for line in sim.lines() {
        println!("{line}");
    }
    if hash {
        println!("Heights hash: {:016x}", sim.heights_hash());
    }
    if image {
        let path = output.map_or_else(
            || {
                let stem = project
                    .file_stem()
                    .map_or_else(|| "part".into(), |s| s.to_string_lossy().into_owned());
                project.with_file_name(format!("{stem}-sim.png"))
            },
            Path::to_path_buf,
        );
        let bytes = match sim.png(chipload_pipeline::simulate::PRINT_PALETTE) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("error: {err}");
                return ExitCode::from(EXIT_FAILED);
            }
        };
        if let Err(err) = std::fs::write(&path, bytes) {
            eprintln!("error: cannot write {}: {err}", path.display());
            return ExitCode::from(EXIT_FAILED);
        }
        println!("Wrote {}", path.display());
    }
    ExitCode::SUCCESS
}

fn run(
    project: &Path,
    output: &Path,
    tool_change: Option<library::ToolChangeArg>,
    with_sheet: bool,
) -> ExitCode {
    let (job, result) = match load_and_run(project, tool_change) {
        Ok(run) => run,
        Err(code) => return code,
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
    if let Err(err) = write_gcode_files(output, &result.files) {
        eprintln!("error: cannot write {}: {err}", output.display());
        return ExitCode::from(EXIT_FAILED);
    }
    for file in &result.files {
        let lines = file.gcode.matches('\n').count();
        if file.suffix.is_some() {
            println!(
                "Wrote {} ({lines} lines, tool {}, operations {})",
                file.path(output).display(),
                file.tool,
                chipload_post::operations_text(&file.operations)
            );
        } else {
            println!(
                "Wrote {} ({lines} lines, post {})",
                output.display(),
                result.post.name
            );
        }
    }
    for line in estimate_lines(&job, &result.estimate, result.files.len() > 1) {
        println!("{line}");
    }
    if result.files.len() > 1 {
        println!(
            "{} files, post {}: change the tool and set Z zero between them, keeping X and Y \
             zero.",
            result.files.len(),
            result.post.name
        );
    }
    if with_sheet {
        match write_setup_sheet(&job, &result, output, &sheet_info(project)) {
            Ok(path) => println!("Wrote {} (setup sheet)", path.display()),
            Err(err) => {
                eprintln!(
                    "error: cannot write the setup sheet {}: {err}",
                    sheet_path(output).display()
                );
                return ExitCode::from(EXIT_FAILED);
            }
        }
    }
    ExitCode::SUCCESS
}

fn op(action: OpAction) -> ExitCode {
    match action {
        OpAction::Tool {
            project,
            op,
            tool,
            output,
        } => {
            let mut job = match load(&project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            if let Err(err) = set_op_tool(&mut job, op, tool) {
                return fail(err);
            }
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return fail(err);
            }
            println!("Operation {op} now cuts with tool {tool}.");
            println!("Wrote {}", target.display());
            ExitCode::SUCCESS
        }
        OpAction::Rest {
            project,
            op,
            after_tool,
            none: _,
            output,
        } => {
            let mut job = match load(&project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            if let Err(err) = set_op_rest(&mut job, op, after_tool) {
                return fail(err);
            }
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return fail(err);
            }
            match after_tool {
                Some(tool) => println!("Operation {op} now rest machines after tool {tool}."),
                None => println!("Operation {op} now clears the whole pocket."),
            }
            println!("Wrote {}", target.display());
            ExitCode::SUCCESS
        }
    }
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
    // A plan with tool changes is posted by `run`, which writes each tool's part.
    if let Some(groups) = value.get("tool_groups").and_then(Value::as_u64)
        && groups > 1
    {
        return Err(format!("this plan uses {groups} tool groups; use run"));
    }
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
    // The post's spindle wait; a post that can't be read only leaves it out here (`run`
    // refuses it).
    let dwell = chipload_pipeline::posts::job_post(&job).map_or(0.0, |p| p.spindle_dwell_s);
    let estimate = estimate(&job, &plan.program, &plan.op_spans, &plan.groups, dwell);
    if as_json {
        println!(
            "{}",
            json!({
                "program": program_with_groups(&plan.program, plan.groups.len()),
                "groups": plan.groups.iter().map(|g| json!({
                    "tool": g.tool.id,
                    "operations": g.operations,
                    "moves": [g.moves.start, g.moves.end],
                })).collect::<Vec<_>>(),
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
                "ring_starts": plan.ring_starts.iter().map(|s| json!({
                    "operation": s.operation,
                    "start": s.ring.first(),
                    "stored": s.stored,
                })).collect::<Vec<_>>(),
                "estimate": json!({
                    "total_s": estimate.total,
                    "cutting_s": estimate.cutting,
                    "rapid_s": estimate.rapid,
                    "spindle_s": estimate.spindle,
                    "operations": estimate.operations.iter().map(|(op, s)| json!({
                        "operation": op,
                        "seconds": s,
                    })).collect::<Vec<_>>(),
                    "groups_s": estimate.groups,
                    "tool_changes": estimate.tool_changes,
                    "rapid_mm_min": estimate.rapid_rate,
                    "rapid_assumed": estimate.rapid_assumed,
                }),
                "chip_load": job.operations.iter().enumerate()
                    .filter(|(_, op)| op.enabled())
                    .map(|(i, op)| json!({
                        "operation": i,
                        "chip_load_mm": chip_load(&job, op).mm(),
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
        for start in &plan.ring_starts {
            let Some((x, y)) = start.ring.first() else {
                continue;
            };
            let from = match start.stored {
                Some(k) => format!("start point {}", k + 1),
                None => "default".into(),
            };
            println!(
                "Operation {}: a ring starts at ({x:.3}, {y:.3}) ({from})",
                start.operation + 1
            );
        }
        if let Some(text) = copies_text(&job) {
            println!("Copies: {text}");
        }
        for line in estimate_lines(&job, &estimate, plan.groups.len() > 1) {
            println!("{line}");
        }
        for line in chip_load_lines(&job) {
            println!("{line}");
        }
        if plan.groups.len() > 1 {
            for (i, g) in plan.groups.iter().enumerate() {
                println!(
                    "Tool group {}: tool {}, operations {}",
                    i + 1,
                    g.tool.id,
                    chipload_post::operations_text(&g.operations)
                );
            }
        }
        println!("Planned {} moves.", plan.program.moves.len());
    }
    ExitCode::SUCCESS
}

/// The program as `render` reads it, with how many tool groups it has: `render` refuses a
/// plan with tool changes.
fn program_with_groups(program: &Program, groups: usize) -> Value {
    let mut value = program_json(program);
    value["tool_groups"] = json!(groups);
    value
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
    let mut copies_line = None;
    let mut placement_line = None;
    let loaded = if is_dxf(path) {
        import_dxf(path, units).map_err(|e| e.to_string())
    } else {
        load_job(path).map_err(|e| e.to_string()).and_then(|job| {
            shapes = Some(job.drawing.shapes.clone());
            // Only when set, so a fresh project still inspects exactly like its DXF.
            if job.machine.work_zero != (WorkZero::DrawingOrigin {}) {
                work_zero = Some(format!("Work zero: {}", job.work_zero_text()));
            }
            copies_line = copies_text(&job).map(|text| format!("Copies: {text}"));
            placement_line = job
                .drawing
                .source
                .as_ref()
                .filter(|s| s.placement != Placement::default())
                .map(|s| format!("Drawing: {}", placement_text(s.placement)));
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
        .chain(placement_line)
        .chain(work_zero)
        .chain(copies_line)
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

fn check(gcode: &Path, job_path: &Path, tool: Option<i64>, as_json: bool) -> ExitCode {
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
    let limits = match tool {
        None => limits_from_job(&job),
        Some(id) => match limits_for_tool(&job, id) {
            Some(limits) => limits,
            None => return fail(format!("the project has no tool {id}")),
        },
    };
    let report = check_gcode(&text, &limits);
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
