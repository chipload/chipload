// SPDX-License-Identifier: GPL-3.0-or-later
//! The `chipload` command line: import drawings into projects, plan and run them, check
//! G-code. Projects are `.chipload` files (docs/specs/project-file.md).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod dxfs;
mod images;
mod lengths;
mod library;
mod parts;

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
use chipload_pipeline::copies::{copies_text, grid_text, parts_lines, pitch_for_gap};
use chipload_pipeline::estimate::{estimate, estimate_lines};
use chipload_pipeline::library::{Library, apply_library, settings_dir};
use chipload_pipeline::placement::{
    Move as PlaceMove, dxf_list, dxf_size, place_drawing, place_dxf, placement_text,
};
use chipload_pipeline::report::{
    format_contours, format_import_report, format_shapes, severity_name,
};
use chipload_pipeline::shapes::{add_shape, copy_shapes, delete_shape, mirror_shapes, set_shape};
use chipload_pipeline::sheet::{SheetInfo, setup_sheet, sheet_path, write_setup_sheet};
use chipload_pipeline::{
    PipelineError, RunResult, add_rest_pocket, empty_project, load_drawing, new_project, run_job,
    set_op_rest, set_op_tool, update_drawing, write_atomically, write_gcode_files,
};
use chipload_post::{load_builtin_post, render_program};
use chipload_toolpath::arcs::Circle;
use chipload_toolpath::model::{Move, MoveKind, Program};
use chipload_toolpath::{ToolpathError, plan_job};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

mod mcp;
mod out;
mod reference;
mod texts;

use out::{EXIT_FAILED, Out};

#[derive(Parser)]
#[command(
    name = "chipload",
    version,
    about = "2.5D CAM: DXF in, checked G-code out"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Print one JSON object: the result, notes, files written, or the error with the
    /// project's problems (docs/specs/scripting.md §3).
    #[arg(long, global = true)]
    json: bool,
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
        /// Which DXF of the project (as `dxf list` shows); needed when it has several.
        #[arg(long)]
        dxf: Option<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Plan a project's toolpath without writing G-code.
    Plan {
        /// The project (.chipload).
        project: PathBuf,
        /// For this run only: set a project value by path, e.g. `stock.thickness=18`
        /// (see `chipload set`). The project file isn't changed.
        #[arg(long = "set", value_name = "PATH=VALUE")]
        sets: Vec<String>,
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
        /// The setup sheet's lengths and feeds: mm (default) or inch. The G-code's units are
        /// the machine's (machine.gcode_units).
        #[arg(long, value_name = "UNITS")]
        sheet_units: Option<UnitsArg>,
        /// For this run only: set a project value by path, e.g. `stock.thickness=18`
        /// (see `chipload set`). The project file isn't changed.
        #[arg(long = "set", value_name = "PATH=VALUE")]
        sets: Vec<String>,
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
        #[arg(value_parser = crate::lengths::length, long)]
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
        /// For this run only: set a project value by path, e.g. `stock.thickness=18`
        /// (see `chipload set`). The project file isn't changed.
        #[arg(long = "set", value_name = "PATH=VALUE")]
        sets: Vec<String>,
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
        /// Its lengths and feeds: mm (default) or inch. The G-code's units are the
        /// machine's (machine.gcode_units).
        #[arg(long)]
        units: Option<UnitsArg>,
        /// For this run only: set a project value by path, e.g. `stock.thickness=18`
        /// (see `chipload set`). The project file isn't changed.
        #[arg(long = "set", value_name = "PATH=VALUE")]
        sets: Vec<String>,
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
    /// Pictures traced into shapes for V-carving: add, change, list, preview, remove.
    Image {
        #[command(subcommand)]
        action: images::ImageAction,
    },
    /// Text written into shapes for V-carving, pocketing or profiling: add, change, list,
    /// remove, and the built-in fonts (docs/specs/text.md).
    Text {
        #[command(subcommand)]
        action: texts::TextAction,
    },
    /// A project's DXFs: add another, list them, remove one (docs/specs/drawing-sources.md).
    Dxf {
        #[command(subcommand)]
        action: dxfs::DxfAction,
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
    /// A project's parts: groups of operations, each cut with its copies before the next
    /// (docs/specs/parts.md).
    Part {
        #[command(subcommand)]
        action: parts::PartAction,
    },
    /// Cut a part several times in a grid (docs/specs/copies-grid.md).
    Copies {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part (as `part list` shows it); needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// Columns x rows, e.g. "3x2".
        #[arg(long)]
        grid: Option<String>,
        /// Pitch "DX,DY" in mm, copy to copy; negative grows left or down.
        #[arg(long, allow_hyphen_values = true, conflicts_with = "gap")]
        pitch: Option<String>,
        /// Set the pitch so that this many mm are left between the cuts of neighbours.
        #[arg(value_parser = crate::lengths::length, long)]
        gap: Option<f64>,
        /// The cut order.
        #[arg(long)]
        order: Option<OrderArg>,
        /// Back to one copy.
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
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["align", "at"])]
        to: Option<String>,
        /// Put the lower-left corner of its bounding box at "X,Y" mm (the app's Position).
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["by", "align"])]
        at: Option<String>,
        /// Align the drawing's extent with the stock.
        #[arg(long)]
        align: Option<AlignArg>,
        /// With --align bottom-left: how far in from the stock's corner, mm.
        #[arg(value_parser = crate::lengths::length, long)]
        margin: Option<f64>,
        /// Turn it about its centre by this many degrees counter-clockwise (negative:
        /// clockwise), any angle, on top of any turn it has. Before --align when both are
        /// given.
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["by", "to"])]
        rotate: Option<f64>,
        /// Scale it about its centre by this factor (> 0), on top of any scale it has. After
        /// --rotate, before --align.
        #[arg(long, allow_hyphen_values = true, conflicts_with_all = ["by", "to", "width", "height"])]
        scale: Option<f64>,
        /// Scale it about its centre until it is this wide, mm (the height follows).
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true, conflicts_with_all = ["by", "to", "height"])]
        width: Option<f64>,
        /// Scale it about its centre until it is this high, mm (the width follows).
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true, conflicts_with_all = ["by", "to"])]
        height: Option<f64>,
        /// Back to where the DXF has it: no turn, no scale, no move.
        #[arg(
            long,
            conflicts_with_all = ["by", "to", "at", "align", "margin", "rotate", "scale", "width", "height"]
        )]
        reset: bool,
        /// Which DXF of the project (as `dxf list` shows); needed when it has several.
        #[arg(long, conflicts_with = "image")]
        dxf: Option<u64>,
        /// A picture of the project instead (as `image list` shows it): --at, --rotate and
        /// --width apply, as `image set`.
        #[arg(long, conflicts_with = "text")]
        image: Option<u64>,
        /// A text of the project instead (as `text list` shows it): --at (its anchor: the
        /// corner or the centre, as set), --rotate and --height (its capital height) apply,
        /// as `text set`.
        #[arg(long)]
        text: Option<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print Chipload's licence and the licences of every third-party component.
    Licenses,
    /// Print a project value by path, as JSON (docs/specs/scripting.md §4), e.g.
    /// `operations.1.end_depth`: keys and indexes from 0, as in the file.
    Get {
        /// The project (.chipload).
        project: PathBuf,
        path: String,
    },
    /// Set project values by path (docs/specs/scripting.md §4): `operations.1.end_depth=6.5`.
    /// A value is JSON when it parses (6.5, true, null, {"after_tool": 1}), else text. The
    /// project must still load; problems are listed but don't stop the write.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        #[arg(required = true, value_name = "PATH=VALUE")]
        values: Vec<String>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List a project's problems with their paths, without planning; exits 1 if any.
    CheckProject {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Print a page of the user guide's reference (docs/specs/user-guide.md §4).
    #[command(hide = true)]
    Reference {
        #[arg(value_enum)]
        page: ReferencePage,
    },
    /// Print the JSON Schema of a file format (docs/specs/scripting.md §2).
    Schema {
        #[arg(value_enum)]
        format: SchemaArg,
    },
    /// Serve the commands as tools to an AI assistant over the Model Context Protocol, on
    /// stdin and stdout (e.g. `claude mcp add chipload -- chipload mcp`).
    Mcp,
}

#[derive(Clone, Copy, ValueEnum)]
enum ReferencePage {
    Cli,
    Project,
    Tools,
    Machines,
    Posts,
}

#[derive(Clone, Copy, ValueEnum)]
enum SchemaArg {
    /// A project file (.chipload).
    Project,
    /// The tool library (tools.json, and `tool export` files).
    Tools,
    /// The machine profiles (machines.json, and `machine export` files).
    Machines,
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
    /// Every operation on one copy, then the next copy (was `by-part`).
    #[value(alias = "by-part")]
    ByCopy,
}

/// What `chipload copies` was asked to change.
struct CopiesArgs {
    part: Option<u64>,
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
        /// The operation's number as the app lists it (1 = first; `get` and `set` paths
        /// count from 0).
        #[arg(long)]
        op: usize,
        /// The tool's id in the project.
        #[arg(long)]
        tool: i64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Add a rest pocket after a pocket (docs/specs/rest-machining.md §10): its shapes and
    /// depths, cut with a smaller tool, clearing only what the pocket's tool leaves.
    AddRest {
        /// The project (.chipload).
        project: PathBuf,
        /// The pocket's number as the app lists it (1 = first; `get` and `set` paths count
        /// from 0).
        #[arg(long)]
        op: usize,
        /// The smaller tool's id in the project.
        #[arg(long)]
        tool: i64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move an operation into another part, at the end of its operations
    /// (docs/specs/parts.md §7). Operations are numbered again in the new order.
    Part {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation's number as the app lists it (1 = first; `get` and `set` paths
        /// count from 0).
        #[arg(long)]
        op: usize,
        /// The part's id (as `part list` shows it).
        #[arg(long)]
        part: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Make a pocket clear only what a larger tool leaves (docs/specs/rest-machining.md).
    Rest {
        /// The project (.chipload).
        project: PathBuf,
        /// The pocket's number as the app lists it (1 = first; `get` and `set` paths count
        /// from 0).
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
    /// Remove a drawn shape. Operations that used it are flagged.
    #[command(alias = "delete")]
    Remove {
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
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dx: f64,
        /// How far to move the copies up, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
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
    #[arg(value_parser = crate::lengths::length, long)]
    radius: Option<f64>,
    /// Along the rectangle's own X axis.
    #[arg(value_parser = crate::lengths::length, long)]
    length: Option<f64>,
    /// Along the rectangle's own Y axis.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// Rotation about the centre, degrees, counter-clockwise.
    #[arg(long, allow_hyphen_values = true)]
    angle_deg: Option<f64>,
    /// The radius of the rectangle's corners, at most half its shorter side (default 0:
    /// sharp).
    #[arg(value_parser = crate::lengths::length, long)]
    corner_radius: Option<f64>,
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
pub enum UnitsArg {
    Mm,
    Inch,
}

impl UnitsArg {
    pub fn value(self) -> Units {
        match self {
            UnitsArg::Mm => Units::Mm,
            UnitsArg::Inch => Units::Inch,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if matches!(cli.command, Command::Mcp) {
        return mcp::serve_stdio();
    }
    let mut out = Out::new(cli.json);
    let code = execute(&mut out, cli.command);
    out.finish(code)
}

/// Run one command, its output into `out` (also for the MCP server's tools).
fn execute(out: &mut Out, command: Command) -> ExitCode {
    let units = |u: Option<UnitsArg>| u.map(UnitsArg::value);
    match command {
        Command::Inspect { file, units: u } => inspect(out, &file, units(u)),
        Command::New { output, machine } => create(
            out,
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
        } => import(out, &dxf, units(u), &output, machine.as_deref()),
        Command::Update {
            project,
            from,
            units: u,
            dxf,
            output,
        } => update(out, &project, &from, units(u), dxf, output.as_deref()),
        Command::Plan { project, sets } => plan(out, &project, &sets),
        Command::Get { project, path } => get(out, &project, &path),
        Command::Set {
            project,
            values,
            output,
        } => set(out, &project, &values, output.as_deref()),
        Command::CheckProject { project } => check_project(out, &project),
        Command::Run {
            project,
            output,
            tool_change,
            no_sheet,
            sheet_units,
            sets,
        } => {
            let sheet = (!no_sheet).then(|| units(sheet_units).unwrap_or(Units::Mm));
            run(out, &project, &output, tool_change, sheet, &sets)
        }
        Command::Sheet {
            project,
            output,
            gcode,
            tool_change,
            units: sheet_units,
            sets,
        } => sheet(
            out,
            &project,
            &output,
            gcode.as_deref(),
            tool_change,
            units(sheet_units).unwrap_or(Units::Mm),
            &sets,
        ),
        Command::Simulate {
            project,
            output,
            cell,
            no_image,
            after,
            hash,
            sets,
        } => simulate(
            out,
            &project,
            output.as_deref(),
            SimulateArgs {
                cell,
                after,
                image: !no_image,
                hash,
            },
            &sets,
        ),
        Command::Render { program, post } => render(out, &program, &post),
        Command::Check { gcode, job, tool } => check(out, &gcode, &job, tool),
        Command::Op { action } => op(out, action),
        Command::Shape { action } => shape(out, action),
        Command::Image { action } => images::image(out, action),
        Command::Text { action } => texts::text(out, action),
        Command::Dxf { action } => dxfs::dxf(out, action),
        Command::Tool { action } => library::tool(out, action),
        Command::Machine { action } => library::machine(out, action),
        Command::Post { action } => library::post(out, action),
        Command::Part { action } => parts::part(out, action),
        Command::Copies {
            project,
            part,
            grid,
            pitch,
            gap,
            order,
            off,
            output,
        } => copies(
            out,
            &project,
            CopiesArgs {
                part,
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
            at,
            align,
            margin,
            rotate,
            scale,
            width,
            height,
            reset,
            dxf,
            image,
            text,
            output,
        } => place(
            out,
            &project,
            PlaceArgs {
                by,
                to,
                at,
                align,
                margin,
                rotate,
                scale,
                width,
                height,
                reset,
                dxf,
                image,
                text,
            },
            output.as_deref(),
        ),
        Command::Reference { page } => {
            out.text("text", &reference_page(page));
            ExitCode::SUCCESS
        }
        Command::Schema { format } => {
            let format = match format {
                SchemaArg::Project => chipload_job::schema::Format::Project,
                SchemaArg::Tools => chipload_job::schema::Format::Tools,
                SchemaArg::Machines => chipload_job::schema::Format::Machines,
            };
            out.document(chipload_job::schema::schema(format));
            ExitCode::SUCCESS
        }
        Command::Licenses => {
            let text = format!(
                "{}\n\n{}\n{}\n",
                chipload_pipeline::notices::short_notice(),
                chipload_pipeline::notices::LICENSE,
                chipload_pipeline::notices::THIRD_PARTY
            );
            out.text("text", &text);
            ExitCode::SUCCESS
        }
        Command::Mcp => out.fail("the MCP server is started on its own: chipload mcp"),
    }
}

/// A generated page of the user guide (docs/specs/user-guide.md §4).
fn reference_page(page: ReferencePage) -> String {
    use chipload_job::schema::{Format, schema};
    match page {
        ReferencePage::Cli => {
            // Built, so every argument knows its value count.
            let mut root = <Cli as clap::CommandFactory>::command();
            root.build();
            reference::cli_page(&root)
        }
        ReferencePage::Project => reference::schema_page(
            "Project file",
            "A project (`.chipload`) is a JSON file. Chipload writes every field; when you write \
             one by hand or with a script, the fields marked *no* may be left out. Lengths are \
             in mm, angles in radians, feeds in mm/min. `chipload schema project` prints the \
             JSON Schema this page is made from.",
            &schema(Format::Project),
        ),
        ReferencePage::Tools => reference::schema_page(
            "Tool library file",
            "Your tool library (`tools.json` in the settings folder) and the files `tool export` \
             writes. Lengths in mm, angles in radians, feeds in mm/min.",
            &schema(Format::Tools),
        ),
        ReferencePage::Machines => reference::schema_page(
            "Machine profiles file",
            "Your machine profiles (`machines.json` in the settings folder) and the files \
             `machine export` writes. Lengths in mm, feeds in mm/min.",
            &schema(Format::Machines),
        ),
        ReferencePage::Posts => posts_page(),
    }
}

/// The post-processors' reference: the config's fields, then each built-in post's values.
fn posts_page() -> String {
    use std::fmt::Write as _;
    let config = schemars::SchemaGenerator::default()
        .into_root_schema_for::<chipload_post::PostConfig>()
        .to_value();
    let mut page = reference::schema_page(
        "Post-processors",
        "A post-processor is a small TOML file (`name.toml`) and a Jinja template \
         (`name.*.j2`); see [Post-processors](../posts.md) for writing your own. The TOML \
         file's fields:",
        &config,
    );
    page.push_str("## Built-in post-processors\n\n");
    page.push_str(
        "| Name | File extension | Decimals | Arcs | Spindle wait (s) | Tool changes | Note |\n\
         |---|---|---|---|---|---|---|\n",
    );
    for name in chipload_post::builtin_posts() {
        let Ok(post) = load_builtin_post(name) else {
            continue;
        };
        let changes: Vec<&str> = post.tool_changes.iter().map(|c| c.name()).collect();
        let _ = writeln!(
            page,
            "| `{name}` | `{}` | {} | {} | {} | {} | {} |",
            post.file_extension,
            post.decimals,
            if post.arcs { "G2/G3" } else { "lines" },
            post.spindle_dwell_s,
            changes.join(", "),
            post.note.as_deref().unwrap_or("")
        );
    }
    page
}

fn is_dxf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dxf"))
}

fn import(
    out: &mut Out,
    dxf: &Path,
    units: Option<Units>,
    output: &Path,
    machine: Option<&str>,
) -> ExitCode {
    let imported = match shapes_from_dxf(dxf, units) {
        Ok(imported) => imported,
        Err(err) => return out.fail(err),
    };
    let line = format!(
        "Imported {} ({} shapes).",
        imported.source.file,
        imported.shapes.len()
    );
    let code = create(out, new_project(imported), &line, output, machine);
    if out.json() && code == ExitCode::SUCCESS {
        // The import report as `inspect --json` gives it for the new project.
        if let Ok(result) = load_job(output)
            .map_err(|e| e.to_string())
            .and_then(|job| load_drawing(&job).map_err(|e| e.to_string()))
        {
            out.set("report", import_json(&result));
        }
    }
    code
}

/// Write a new project, started from the library's machine profile and default tool.
fn create(
    out: &mut Out,
    mut job: Job,
    line: &str,
    output: &Path,
    machine: Option<&str>,
) -> ExitCode {
    // The library only gives starting values; without a settings folder there is none.
    let library = settings_dir()
        .map(|dir| Library::load(&dir))
        .unwrap_or_default();
    let notes = match apply_library(&mut job, &library, machine) {
        Ok(notes) => notes,
        Err(err) => return out.fail(err),
    };
    if let Err(err) = save_job(&job, output) {
        return out.fail(err);
    }
    out.line(line.to_string());
    for problem in &library.problems {
        out.err(format!("warning: {problem}"));
    }
    for note in notes {
        out.line(note.to_string());
    }
    out.wrote(output, "");
    ExitCode::SUCCESS
}

fn copies(out: &mut Out, project: &Path, args: CopiesArgs, output: Option<&Path>) -> ExitCode {
    let mut job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let part = match (args.part, job.parts.as_slice()) {
        (Some(id), _) => id,
        (None, [only]) => only.id,
        (None, _) => {
            return out.fail("the project has several parts: give --part ID (see `part list`)");
        }
    };
    let Some(at) = job.parts.iter().position(|p| p.id == part) else {
        return out.fail(format!("no part {part} in the project (see `part list`)"));
    };
    match changed_copies(&job, part, args) {
        Ok(copies) => job.parts[at].copies = copies,
        Err(err) => return out.fail(err),
    }
    let problems = job.validate();
    if !problems.is_empty() {
        return out.fail(chipload_pipeline::joined(&problems));
    }
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&job, target) {
        return out.fail(err);
    }
    match job.parts[at].copies.as_ref().map(grid_text) {
        Some(text) => out.line(format!("Copies: {text}")),
        None => out.line("Copies: off (one copy)"),
    }
    // Overlap is refused when the project is run; say so now.
    if job.parts[at].count() > 1
        && let Ok(drawing) = load_drawing(&job)
        && let Err(err) = plan_job(&job, &drawing.contours)
        && matches!(err.cause(), ToolpathError::Overlap(_))
    {
        out.err(format!("warning: {err}"));
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

fn changed_copies(job: &Job, part: u64, args: CopiesArgs) -> Result<Option<Copies>, String> {
    if args.off {
        return Ok(None);
    }
    let current = job.part(part).and_then(|p| p.copies);
    let mut copies = current.unwrap_or(Copies {
        columns: 1,
        rows: 1,
        dx: 0.0,
        dy: 0.0,
        order: CopyOrder::ByOperation,
    });
    if current.is_none() && args.grid.is_none() {
        return Err("the part has no copies yet: give --grid (and --pitch or --gap)".into());
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
            OrderArg::ByCopy => CopyOrder::ByCopy,
        };
    }
    if let Some(gap) = args.gap {
        (copies.dx, copies.dy) =
            pitch_for_gap(job, part, gap, (copies.dx, copies.dy)).map_err(|e| e.to_string())?;
    } else if current.is_none() && args.pitch.is_none() && copies.count() > 1 {
        return Err("give the spacing with --pitch DX,DY or --gap MM".into());
    }
    Ok(Some(copies))
}

/// What `chipload place` was asked to do.
struct PlaceArgs {
    by: Option<String>,
    to: Option<String>,
    at: Option<String>,
    align: Option<AlignArg>,
    margin: Option<f64>,
    rotate: Option<f64>,
    scale: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    reset: bool,
    dxf: Option<u64>,
    image: Option<u64>,
    text: Option<u64>,
}

fn place(out: &mut Out, project: &Path, args: PlaceArgs, output: Option<&Path>) -> ExitCode {
    if let Some(id) = args.image {
        return place_image(out, project, id, args, output);
    }
    if let Some(id) = args.text {
        return place_text(out, project, id, args, output);
    }
    let job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let which = args.dxf;
    let steps = match place_moves(args) {
        Ok(steps) => steps,
        Err(err) => return out.fail(err),
    };
    if steps.is_empty() {
        let source = match which {
            Some(id) => job.drawing.dxfs.iter().find(|d| d.id == id),
            None if job.drawing.dxfs.len() == 1 => job.drawing.dxfs.first(),
            None if job.drawing.dxfs.len() > 1 => {
                return out.fail(format!(
                    "the project has {} DXFs ({}): say which with --dxf",
                    job.drawing.dxfs.len(),
                    dxf_list(&job.drawing.dxfs)
                ));
            }
            None => None,
        };
        return match source {
            Some(source) => {
                out.line(format!("Drawing: {}", placement_text(source.placement)));
                let size = dxf_size(&job.drawing.shapes, source.id);
                if let Some((w, h)) = size {
                    out.line(format!("Size: {w:.3} × {h:.3} mm"));
                }
                out.set("placement", json!(source.placement));
                out.set("size", json!(size.map(|(w, h)| [w, h])));
                ExitCode::SUCCESS
            }
            None => out.fail("the project has no imported drawing"),
        };
    }
    let mut placed = job;
    let mut lines = Vec::new();
    for how in steps {
        let done = match which {
            Some(id) => place_dxf(&placed, id, how),
            None => place_drawing(&placed, how),
        };
        match done {
            Ok((next, report)) => {
                placed = next;
                lines.extend(report.lines);
            }
            Err(err) => return out.fail(err),
        }
    }
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&placed, target) {
        return out.fail(err);
    }
    for line in &lines {
        out.line(line.to_string());
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// `place --image`: a picture's corner, turn and width, as `image set` sets them
/// (docs/specs/direct-placement.md).
fn place_image(
    out: &mut Out,
    project: &Path,
    id: u64,
    args: PlaceArgs,
    output: Option<&Path>,
) -> ExitCode {
    let others = args.by.is_some()
        || args.to.is_some()
        || args.align.is_some()
        || args.margin.is_some()
        || args.scale.is_some()
        || args.height.is_some()
        || args.reset;
    if others {
        return out.fail("with --image, place takes --at, --rotate and --width (as `image set`)");
    }
    let at = match args.at.as_deref().map(point_arg) {
        Some(Ok(p)) => Some((p[0], p[1])),
        Some(Err(err)) => return out.fail(err),
        None => None,
    };
    if at.is_none() && args.rotate.is_none() && args.width.is_none() {
        return out.fail("with --image, say what to change: --at, --rotate or --width");
    }
    images::place_image(out, project, id, at, args.rotate, args.width, output)
}

/// `place --text`: a text's corner, turn and capital height, as `text set` sets them.
fn place_text(
    out: &mut Out,
    project: &Path,
    id: u64,
    args: PlaceArgs,
    output: Option<&Path>,
) -> ExitCode {
    let others = args.by.is_some()
        || args.to.is_some()
        || args.align.is_some()
        || args.margin.is_some()
        || args.scale.is_some()
        || args.width.is_some()
        || args.reset;
    if others {
        return out.fail("with --text, place takes --at, --rotate and --height (as `text set`)");
    }
    let at = match args.at.as_deref().map(point_arg) {
        Some(Ok(p)) => Some((p[0], p[1])),
        Some(Err(err)) => return out.fail(err),
        None => None,
    };
    if at.is_none() && args.rotate.is_none() && args.height.is_none() {
        return out.fail("with --text, say what to change: --at, --rotate or --height");
    }
    texts::place_text(out, project, id, at, args.rotate, args.height, output)
}

/// The moves `place` was asked for, in order: a turn, a scale, then a move or an alignment.
/// Empty: just print the placement.
fn place_moves(args: PlaceArgs) -> Result<Vec<PlaceMove>, String> {
    if args.reset {
        return Ok(vec![PlaceMove::Reset]);
    }
    let mut steps = Vec::new();
    if let Some(degrees) = args.rotate {
        if !degrees.is_finite() {
            return Err(format!("--rotate takes degrees, got {degrees}"));
        }
        steps.push(PlaceMove::Rotate(degrees));
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
    if let Some(at) = args.at {
        let [x, y] = point_arg(&at).map_err(|e| format!("--at: {e}"))?;
        steps.push(PlaceMove::Corner(x, y));
    }
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

fn update(
    out: &mut Out,
    project: &Path,
    from: &Path,
    units: Option<Units>,
    dxf: Option<u64>,
    output: Option<&Path>,
) -> ExitCode {
    let job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let updated = match dxf {
        Some(id) => chipload_pipeline::dxfs::update_dxf(&job, id, from, units),
        None => update_drawing(&job, from, units),
    };
    let (updated, report) = match updated {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&updated, target) {
        return out.fail(err);
    }
    for line in &report.lines {
        out.line(line.to_string());
    }
    out.wrote(target, "");
    out.set("unchanged", json!(report.unchanged));
    out.set("changed", json!(report.changed));
    out.set("added", json!(report.added));
    out.set("removed", json!(report.removed));
    out.set("lost", json!(report.lost));
    if report.lost.is_empty() {
        ExitCode::SUCCESS
    } else {
        out.err(format!(
            "warning: {} operation(s) lost geometry and can't run until fixed",
            report.lost.len()
        ));
        ExitCode::from(EXIT_FAILED)
    }
}

fn shape(out: &mut Out, action: ShapeAction) -> ExitCode {
    let (project, output) = match &action {
        ShapeAction::Add {
            project, output, ..
        }
        | ShapeAction::Set {
            project, output, ..
        }
        | ShapeAction::Remove {
            project, output, ..
        }
        | ShapeAction::Copy {
            project, output, ..
        }
        | ShapeAction::Mirror {
            project, output, ..
        } => (project.clone(), output.clone()),
    };
    let job = match load(out, &project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut flagged_ops = 0;
    // The new shapes' ids (`shape add`, `shape copy`).
    let mut new_ids: Option<Vec<u64>> = None;
    let changed = match action {
        ShapeAction::Add { kind, values, .. } => new_geometry(kind, &values)
            .map_err(PipelineError::Job)
            .and_then(|geometry| {
                let layer = values.layer.as_deref().unwrap_or("0");
                add_shape(&job, layer, geometry)
            })
            .map(|(job, id)| {
                new_ids = Some(vec![id]);
                (job, vec![format!("Added shape S{id}.")])
            }),
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
        ShapeAction::Remove { id, .. } => delete_shape(&job, id).map(|(job, flagged)| {
            flagged_ops = flagged.len();
            let mut lines = vec![format!("Deleted shape S{id}.")];
            lines.extend(flagged.into_iter().map(|l| format!("warning: {l}")));
            (job, lines)
        }),
        ShapeAction::Copy { ids, dx, dy, .. } => {
            copy_shapes(&job, &ids, (dx, dy)).map(|(job, new)| {
                let names: Vec<String> = new.iter().map(|id| format!("S{id}")).collect();
                new_ids = Some(new.clone());
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
        Err(err) => return out.fail(err),
    };
    let target = output.as_deref().unwrap_or(&project);
    if let Err(err) = save_job(&job, target) {
        return out.fail(err);
    }
    for line in lines {
        out.line(line.to_string());
    }
    out.wrote(target, "");
    if let Some(ids) = new_ids {
        out.set("ids", json!(ids));
    }
    if flagged_ops == 0 {
        ExitCode::SUCCESS
    } else {
        // As for `update`: written, but some operations can't run until fixed.
        ExitCode::from(EXIT_FAILED)
    }
}

/// "X,Y" in mm, each optionally with a unit (`1in,2in`).
fn point_arg(text: &str) -> Result<[f64; 2], String> {
    lengths::point(text)
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
    if v.corner_radius.is_some() && !matches!(kind, KindArg::Rectangle) {
        return Err("--corner-radius is for a rectangle".into());
    }
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
            corner_radius: v.corner_radius.unwrap_or(0.0),
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
                    ("corner-radius", v.corner_radius.is_some()),
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
            corner_radius,
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
            *corner_radius = v.corner_radius.unwrap_or(*corner_radius);
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
                    ("corner-radius", v.corner_radius.is_some()),
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

fn load(out: &mut Out, project: &Path) -> Result<Job, ExitCode> {
    load_job(project).map_err(|e| out.fail(e))
}

/// Load a project with `--set` values applied for this run (docs/specs/scripting.md §4).
fn load_set(out: &mut Out, project: &Path, sets: &[String]) -> Result<Job, ExitCode> {
    let job = load(out, project)?;
    if sets.is_empty() {
        return Ok(job);
    }
    let assignments = sets
        .iter()
        .map(|text| chipload_job::paths::assignment(text))
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| out.fail(e))?;
    let changed = chipload_job::paths::set_all(&job, &assignments).map_err(|e| out.fail(e))?;
    // An image whose settings changed is traced again (docs/specs/image-trace.md §5), a
    // changed text written again (text.md §6).
    let (traced, reports) =
        chipload_pipeline::images::retrace_changed(&job, &changed).map_err(|e| out.fail(e))?;
    let (written, text_reports) =
        chipload_pipeline::texts::remake_changed(&job, &traced).map_err(|e| out.fail(e))?;
    for line in reports
        .iter()
        .flat_map(|r| &r.lines)
        .chain(text_reports.iter().flat_map(|r| &r.lines))
    {
        out.line(line.clone());
    }
    Ok(written)
}

/// Load a project and run it, as `run` and `sheet` do; errors are printed.
fn load_and_run(
    out: &mut Out,
    project: &Path,
    tool_change: Option<library::ToolChangeArg>,
    sets: &[String],
) -> Result<(Job, RunResult), ExitCode> {
    let mut job = load_set(out, project, sets)?;
    if let Some(change) = tool_change {
        job.machine.tool_change = change.value();
    }
    match run_job(&job) {
        Ok(result) => Ok((job, result)),
        Err(err @ PipelineError::CheckFailed { .. }) => {
            out.err(format!("error: {err}"));
            out.err("No output written.");
            Err(ExitCode::from(EXIT_FAILED))
        }
        Err(err) => {
            out.err(format!("error: {err}"));
            out.problems(chipload_pipeline::job_problems(&job));
            Err(ExitCode::from(EXIT_FAILED))
        }
    }
}

/// What the sheet says it was made from: the project's file name and this version; its
/// lengths in `units`.
fn sheet_info(project: &Path, units: Units) -> SheetInfo {
    SheetInfo {
        project: project.file_name().map_or_else(
            || project.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        written_by: format!("chipload {}", env!("CARGO_PKG_VERSION")),
        units,
    }
}

fn sheet(
    out: &mut Out,
    project: &Path,
    output: &Path,
    gcode: Option<&Path>,
    tool_change: Option<library::ToolChangeArg>,
    units: Units,
    sets: &[String],
) -> ExitCode {
    let (job, result) = match load_and_run(out, project, tool_change, sets) {
        Ok(run) => run,
        Err(code) => return code,
    };
    let gcode = gcode.map_or_else(
        || project.with_extension(result.post.file_extension.trim_start_matches('.')),
        Path::to_path_buf,
    );
    let html = setup_sheet(&job, &result, &gcode, &sheet_info(project, units));
    if let Err(err) = write_atomically(output, &html) {
        out.err(format!("error: cannot write {}: {err}", output.display()));
        return ExitCode::from(EXIT_FAILED);
    }
    out.wrote(output, "");
    ExitCode::SUCCESS
}

/// `simulate`'s options.
struct SimulateArgs {
    cell: Option<f64>,
    after: Option<usize>,
    image: bool,
    hash: bool,
}

fn simulate(
    out: &mut Out,
    project: &Path,
    output: Option<&Path>,
    args: SimulateArgs,
    sets: &[String],
) -> ExitCode {
    let SimulateArgs {
        cell,
        after,
        image,
        hash,
    } = args;
    let (job, result) = match load_and_run(out, project, None, sets) {
        Ok(run) => run,
        Err(code) => return code,
    };
    let mut program = result.program.clone();
    if let Some(done) = after {
        let total = program.moves.len();
        if done > total {
            out.err(format!(
                "error: --after {done}: the program has {total} moves"
            ));
            return ExitCode::from(EXIT_FAILED);
        }
        program.moves.truncate(done);
        out.line(format!("After {done} of {total} moves"));
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
            out.err(format!("error: {err}"));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    for line in sim.lines() {
        out.line(line.to_string());
    }
    if hash {
        out.line(format!("Heights hash: {:016x}", sim.heights_hash()));
    }
    out.set(
        "numbers",
        json!({
            "removed_cm3": sim.numbers.removed_cm3,
            "deepest_z": sim.numbers.deepest,
            "below_bottom_mm": sim.numbers.below_bottom,
            "drill_point_assumed": sim.numbers.drill_point_assumed,
            "vbit_angle_assumed": sim.numbers.vbit_angle_assumed,
            "cell_mm": sim.cell,
            "columns": sim.columns,
            "rows": sim.rows,
        }),
    );
    out.set("image", Value::Null);
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
                out.err(format!("error: {err}"));
                return ExitCode::from(EXIT_FAILED);
            }
        };
        if let Err(err) = std::fs::write(&path, bytes) {
            out.err(format!("error: cannot write {}: {err}", path.display()));
            return ExitCode::from(EXIT_FAILED);
        }
        out.wrote(&path, "");
        let shown = std::path::absolute(&path).unwrap_or(path);
        out.set("image", json!(shown.display().to_string()));
    }
    ExitCode::SUCCESS
}

fn run(
    out: &mut Out,
    project: &Path,
    output: &Path,
    tool_change: Option<library::ToolChangeArg>,
    sheet: Option<Units>,
    sets: &[String],
) -> ExitCode {
    let (job, result) = match load_and_run(out, project, tool_change, sets) {
        Ok(run) => run,
        Err(code) => return code,
    };
    for line in format_import_report(&result.import_report) {
        out.line(line.to_string());
    }
    for note in &result.notes {
        out.line(format!("warning: {note}"));
    }
    for warning in &result.check_warnings {
        out.line(format!("check warning: {}", warning.message));
    }
    out.line(format!("Work zero: {}", job.work_zero_text()));
    if let Err(err) = write_gcode_files(output, &result.files) {
        out.err(format!("error: cannot write {}: {err}", output.display()));
        return ExitCode::from(EXIT_FAILED);
    }
    let mut files = Vec::new();
    for file in &result.files {
        let lines = file.gcode.matches('\n').count();
        let path = file.path(output);
        if file.suffix.is_some() {
            let detail = format!(
                " ({lines} lines, tool {}, operations {})",
                file.tool,
                chipload_post::operations_text(&file.operations)
            );
            out.wrote(&path, &detail);
        } else {
            out.wrote(
                &path,
                &format!(" ({lines} lines, post {})", result.post.name),
            );
        }
        files.push(json!({
            "path": std::path::absolute(&path).unwrap_or(path).display().to_string(),
            "tool": file.tool,
            "operations": file.operations,
        }));
    }
    out.set("files", json!(files));
    out.set("estimate", estimate_json(&result.estimate));
    out.set("sheet", Value::Null);
    for line in estimate_lines(&job, &result.estimate, result.files.len() > 1) {
        out.line(line.to_string());
    }
    if result.files.len() > 1 {
        out.line(format!(
            "{} files, post {}: change the tool and set Z zero between them, keeping X and Y \
             zero.",
            result.files.len(),
            result.post.name
        ));
    }
    if let Some(units) = sheet {
        match write_setup_sheet(&job, &result, output, &sheet_info(project, units)) {
            Ok(path) => {
                out.wrote(&path, " (setup sheet)");
                let shown = std::path::absolute(&path).unwrap_or(path);
                out.set("sheet", json!(shown.display().to_string()));
            }
            Err(err) => {
                out.err(format!(
                    "error: cannot write the setup sheet {}: {err}",
                    sheet_path(output).display()
                ));
                return ExitCode::from(EXIT_FAILED);
            }
        }
    }
    ExitCode::SUCCESS
}

/// `get` (docs/specs/scripting.md §4): the value as JSON.
fn get(out: &mut Out, project: &Path, path: &str) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let value = match serde_json::to_value(&job) {
        Ok(value) => value,
        Err(err) => return out.fail(err),
    };
    match chipload_job::paths::get(&value, path) {
        Ok(found) => {
            if out.json() {
                out.set("value", found.clone());
            } else {
                match serde_json::to_string_pretty(found) {
                    Ok(text) => out.line(text),
                    Err(err) => return out.fail(err),
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

/// `set` (§4): every value set, the result loaded again, then written; its problems are
/// listed but don't stop the write.
fn set(out: &mut Out, project: &Path, values: &[String], output: Option<&Path>) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let assignments = match values
        .iter()
        .map(|text| chipload_job::paths::assignment(text))
        .collect::<Result<Vec<_>, String>>()
    {
        Ok(assignments) => assignments,
        Err(err) => return out.fail(err),
    };
    let changed = match chipload_job::paths::set_all(&job, &assignments) {
        Ok(changed) => changed,
        Err(err) => return out.fail(err),
    };
    // An image whose settings changed is traced again (docs/specs/image-trace.md §5), a
    // changed text written again (text.md §6).
    let (changed, reports) = match chipload_pipeline::images::retrace_changed(&job, &changed) {
        Ok(traced) => traced,
        Err(err) => return out.fail(err),
    };
    let (changed, text_reports) = match chipload_pipeline::texts::remake_changed(&job, &changed) {
        Ok(written) => written,
        Err(err) => return out.fail(err),
    };
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&changed, target) {
        return out.fail(err);
    }
    for (path, value) in &assignments {
        out.line(format!("Set {path} = {value}"));
    }
    for report in &reports {
        images::reported(out, report);
    }
    for report in &text_reports {
        texts::reported(out, report);
    }
    let problems = chipload_pipeline::job_problems(&changed);
    problem_lines(out, &problems);
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// `check-project` (§4): the problems the GUI shows, with their paths.
fn check_project(out: &mut Out, project: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let problems = chipload_pipeline::job_problems(&job);
    problem_lines(out, &problems);
    if problems.is_empty() {
        out.line("No problems.");
        ExitCode::SUCCESS
    } else {
        let count = problems.len();
        let first = problems[0].text.clone();
        out.problems(problems);
        out.err(match count {
            1 => format!("error: 1 problem: {first}"),
            _ => format!("error: {count} problems, the first: {first}"),
        });
        ExitCode::from(EXIT_FAILED)
    }
}

/// One line per problem ("problem: operations.0.rest: …"), and the list as JSON.
fn problem_lines(out: &mut Out, problems: &[chipload_job::Problem]) {
    for p in problems {
        out.line(format!("problem: {}: {}", p.path, p.text));
    }
    out.set(
        "problems",
        json!(
            problems
                .iter()
                .map(|p| json!({"path": p.path, "short": p.short, "text": p.text}))
                .collect::<Vec<_>>()
        ),
    );
}

fn op(out: &mut Out, action: OpAction) -> ExitCode {
    match action {
        OpAction::Tool {
            project,
            op,
            tool,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            if let Err(err) = set_op_tool(&mut job, op, tool) {
                return out.fail(err);
            }
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            out.line(format!("Operation {op} now cuts with tool {tool}."));
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        OpAction::AddRest {
            project,
            op,
            tool,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            // The library's cutting values for the tool, as for a new operation.
            let library = settings_dir()
                .map(|dir| Library::load(&dir))
                .unwrap_or_default();
            let cutting = job
                .tools
                .iter()
                .find(|t| t.id == tool)
                .and_then(|t| chipload_job::library::linked_cutting(&library.tools, t));
            let index = match add_rest_pocket(&mut job, op, tool, cutting.as_ref()) {
                Ok(index) => index,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            let from = if cutting.is_some() {
                "the library's cutting values for the tool"
            } else {
                "the pocket's feeds and step down; check them for the smaller tool"
            };
            out.line(format!(
                "Added operation {} ({}), rest after operation {op}'s tool, with {from}.",
                index + 1,
                job.operations[index].name()
            ));
            out.set("operation", json!(index));
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        OpAction::Part {
            project,
            op,
            part,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(index) = op.checked_sub(1) else {
                return out.fail("operations are numbered from 1");
            };
            let (job, at) =
                match chipload_pipeline::parts::move_operation_to_part(&job, index, part) {
                    Ok(done) => done,
                    Err(err) => return out.fail(err),
                };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            out.line(format!(
                "Operation {op} is now operation {} in part {part}.",
                at + 1
            ));
            out.set("operation", json!(at));
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        OpAction::Rest {
            project,
            op,
            after_tool,
            none: _,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            if let Err(err) = set_op_rest(&mut job, op, after_tool) {
                return out.fail(err);
            }
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            match after_tool {
                Some(tool) => out.line(format!(
                    "Operation {op} now rest machines after tool {tool}."
                )),
                None => out.line(format!("Operation {op} now clears the whole pocket.")),
            }
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
    }
}

fn render(out: &mut Out, program_path: &Path, post_name: &str) -> ExitCode {
    let parsed = std::fs::read_to_string(program_path)
        .map_err(|e| format!("cannot read {}: {e}", program_path.display()))
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()))
        .and_then(|value| program_from_json(&value));
    let program = match parsed {
        Ok(program) => program,
        Err(err) => {
            out.err(format!("error: {err}"));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    match load_builtin_post(post_name).and_then(|post| render_program(&program, &post)) {
        Ok(text) => {
            out.text("gcode", &text);
            ExitCode::SUCCESS
        }
        Err(err) => {
            out.err(format!("error: {err}"));
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
                comment: None,
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
        comments: Vec::new(),
    })
}

fn plan(out: &mut Out, project: &Path, sets: &[String]) -> ExitCode {
    let job = match load_set(out, project, sets) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let imported = match load_drawing(&job) {
        Ok(result) => result,
        Err(err) => return out.fail(err),
    };
    let plan = match plan_job(&job, &imported.contours) {
        Ok(plan) => plan,
        Err(err) => {
            out.problems(chipload_pipeline::job_problems(&job));
            return out.fail(err);
        }
    };
    // The post's spindle wait; a post that can't be read only leaves it out here (`run`
    // refuses it).
    let dwell = chipload_pipeline::posts::job_post(&job).map_or(0.0, |p| p.spindle_dwell_s);
    let estimate = estimate(&job, &plan.program, &plan.op_spans, &plan.groups, dwell);
    if out.json() {
        for note in &plan.notes {
            out.line(note.text.clone());
        }
        out.set_all(json!({
            "program": program_with_groups(&plan.program, plan.groups.len()),
            "groups": plan.groups.iter().map(|g| json!({
                "tool": g.tool.id,
                "operations": g.operations,
                "moves": [g.moves.start, g.moves.end],
            })).collect::<Vec<_>>(),
            "tabs": plan.tabs,
            "pockets": plan.pockets.iter().map(|p| json!({
                "contour_ids": p.contour_ids,
                "rings": p.rings.iter().map(|r| &r.points).collect::<Vec<_>>(),
                "open": p.rings.iter().map(|r| r.open).collect::<Vec<_>>(),
                "feed_links": p.feed_links,
            })).collect::<Vec<_>>(),
            "ring_starts": plan.ring_starts.iter().map(|s| json!({
                "operation": s.operation,
                "start": s.ring.first(),
                "stored": s.stored,
            })).collect::<Vec<_>>(),
            "estimate": estimate_json(&estimate),
            "chip_load": job.operations.iter().enumerate()
                .filter(|(i, _)| job.cuts(*i))
                .map(|(i, op)| json!({
                    "operation": i,
                    "chip_load_mm": chip_load(&job, op).mm(),
                })).collect::<Vec<_>>(),
        }));
    } else {
        for note in &plan.notes {
            out.line(format!("warning: {note}"));
        }
        for start in &plan.ring_starts {
            let Some((x, y)) = start.ring.first() else {
                continue;
            };
            let from = match start.stored {
                Some(k) => format!("start point {}", k + 1),
                None => "default".into(),
            };
            out.line(format!(
                "Operation {}: a ring starts at ({x:.3}, {y:.3}) ({from})",
                start.operation + 1
            ));
        }
        if let Some(text) = copies_text(&job) {
            out.line(format!("Copies: {text}"));
        }
        for line in parts_lines(&job) {
            out.line(line);
        }
        for line in estimate_lines(&job, &estimate, plan.groups.len() > 1) {
            out.line(line.to_string());
        }
        for line in chip_load_lines(&job) {
            out.line(line.to_string());
        }
        if plan.groups.len() > 1 {
            for (i, g) in plan.groups.iter().enumerate() {
                out.line(format!(
                    "Tool group {}: tool {}, operations {}",
                    i + 1,
                    g.tool.id,
                    chipload_post::operations_text(&g.operations)
                ));
            }
        }
        out.line(format!("Planned {} moves.", plan.program.moves.len()));
    }
    ExitCode::SUCCESS
}

/// The cycle time estimate as `plan` and `run` print it with `--json`.
fn estimate_json(estimate: &chipload_pipeline::estimate::Estimate) -> Value {
    json!({
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
    })
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

fn inspect(out: &mut Out, path: &Path, units: Option<Units>) -> ExitCode {
    let mut shapes = None;
    let mut work_zero = None;
    let mut copies_line: Vec<String> = Vec::new();
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
            copies_line = copies_text(&job)
                .map(|text| format!("Copies: {text}"))
                .into_iter()
                .chain(parts_lines(&job))
                .collect();
            let several = job.drawing.dxfs.len() > 1;
            placement_line = Some(
                job.drawing
                    .dxfs
                    .iter()
                    .filter(|s| several || s.placement != Placement::default())
                    .map(|s| {
                        if several {
                            format!("DXF {} {}: {}", s.id, s.name, placement_text(s.placement))
                        } else {
                            format!("Drawing: {}", placement_text(s.placement))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
            .filter(|text| !text.is_empty());
            load_drawing(&job).map_err(|e| e.to_string())
        })
    };
    let result = match loaded {
        Ok(result) => result,
        Err(err) => return out.fail(err),
    };
    if out.json() {
        out.set_all(import_json(&result));
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
        out.line(line.to_string());
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
            // The project's shape ids (`select.shapes`); none for a plain DXF.
            let mut shapes: Vec<u64> = c.parts.iter().filter_map(|p| p.segment.shape).collect();
            shapes.sort_unstable();
            shapes.dedup();
            json!({
                "id": c.id,
                "shapes": shapes,
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

fn check(out: &mut Out, gcode: &Path, job_path: &Path, tool: Option<i64>) -> ExitCode {
    let job = match load_job(job_path) {
        Ok(job) => job,
        Err(err) => {
            out.err(format!("error: {err}"));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let text = match std::fs::read_to_string(gcode) {
        Ok(text) => text,
        Err(err) => {
            out.err(format!("error: cannot read {}: {err}", gcode.display()));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let limits = match tool {
        None => limits_from_job(&job),
        Some(id) => match limits_for_tool(&job, id) {
            Some(limits) => limits,
            None => return out.fail(format!("the project has no tool {id}")),
        },
    };
    let report = check_gcode(&text, &limits);
    if out.json() {
        let issues = |list: &[CheckIssue]| -> Vec<serde_json::Value> {
            list.iter()
                .map(|i| json!({"line": i.line, "message": i.message}))
                .collect()
        };
        out.set_all(json!({
            "ok": report.ok(),
            "errors": issues(&report.errors),
            "warnings": issues(&report.warnings),
        }));
    } else {
        for e in &report.errors {
            out.line(format!("error (line {}): {}", e.line, e.message));
        }
        for w in &report.warnings {
            out.line(format!("warning: {}", w.message));
        }
        out.line(
            (if report.ok() {
                "Check passed."
            } else {
                "Check FAILED."
            })
            .to_string(),
        );
    }
    if report.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_FAILED)
    }
}
