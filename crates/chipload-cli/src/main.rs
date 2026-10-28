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
use chipload_i18n::{Language, Message};
use chipload_job::edit::Mirror;
use chipload_job::{
    Copies, CopyOrder, Geometry, Job, Placement, Spindle, Units, WorkZero, load_job,
    polyline_segment_count, save_job,
};
use chipload_pipeline::chip_load::{chip_load, chip_load_lines};
use chipload_pipeline::copies::{copies_text, grid_text, parts_lines, pitch_for_gap};
use chipload_pipeline::drawing_edit::{
    Combine, Corners, Cut, End, OffsetSide, break_shape, combine_shapes, cut_shape_corners,
    extend_shape, offset_shapes, trim_shape,
};
use chipload_pipeline::estimate::{estimate, estimate_lines};
use chipload_pipeline::explode::join_shapes;
use chipload_pipeline::library::{Library, apply_library, settings_dir};
use chipload_pipeline::placement::{
    Move as PlaceMove, dxf_list, dxf_size, place_drawing, place_dxf, placement_text,
};
use chipload_pipeline::report::{
    format_contours, format_import_report, format_shapes, severity_name,
};
use chipload_pipeline::shapes::{
    add_shape, copy_shapes, delete_shape, drawn_middle, mirror_shapes, name_shape, set_shape,
    transform_shapes,
};
use chipload_pipeline::sheet::{SheetInfo, setup_sheet, sheet_path, write_setup_sheet};
use chipload_pipeline::{
    PipelineError, RunResult, add_rest_pass, empty_project, foreign_text, load_drawing,
    new_project, run_job, set_op_rest, set_op_tool, update_drawing, write_atomically,
    write_gcode_files,
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
        /// The setup sheet's language (default: English). The command line itself stays
        /// English.
        #[arg(long, value_name = "LANG")]
        sheet_lang: Option<LangArg>,
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
        /// Its language (default: English). The command line itself stays English.
        #[arg(long)]
        lang: Option<LangArg>,
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
    /// Add a rest pass to a pocket (docs/specs/rest-passes.md): a smaller endmill that clears
    /// what the pocket's tool (or its last pass's) leaves, in the pocket's shapes and depths.
    /// It is cut after the run of pockets with the pocket's tool.
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
    /// Add a surface (docs/specs/surfacing.md): the whole stock taken down flat with a wide
    /// endmill, in straight passes back and forth, first in the active part. Flattens the
    /// spoilboard or faces the stock.
    AddSurface {
        /// The project (.chipload).
        project: PathBuf,
        /// The endmill's id in the project; default: the widest endmill.
        #[arg(long)]
        tool: Option<i64>,
        /// How much comes off the top; default 0.5 mm.
        #[arg(long, value_parser = crate::lengths::length)]
        depth: Option<f64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// A pocket's rest passes (docs/specs/rest-passes.md): without --pass, list them; with
    /// it, change, turn off or on, or remove one.
    RestPass {
        /// The project (.chipload).
        project: PathBuf,
        /// The pocket's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
        /// Which pass: 1 for the first (shown as 3.1 for operation 3).
        #[arg(long)]
        pass: Option<usize>,
        /// The tool's id in the project: an endmill smaller than the tool before it.
        #[arg(long)]
        tool: Option<i64>,
        #[arg(long, value_parser = crate::lengths::feed)]
        feed: Option<f64>,
        #[arg(long, value_parser = crate::lengths::feed)]
        plunge_feed: Option<f64>,
        #[arg(long, value_parser = crate::lengths::length)]
        step_down: Option<f64>,
        /// A fraction of the tool's diameter, at most 1.
        #[arg(long)]
        stepover: Option<f64>,
        /// Cut it (true) or keep it without cutting (false).
        #[arg(long)]
        enabled: Option<bool>,
        /// Remove the pass; the passes after it rest after the tool before it.
        #[arg(long, conflicts_with_all = ["tool", "feed", "plunge_feed", "step_down", "stepover", "enabled"])]
        remove: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Make a rest pocket that copies an earlier pocket (same shapes, depths, clearance and
    /// direction, rest after its tool) a rest pass of that pocket (docs/specs/rest-passes.md
    /// §5). It is then cut after the run of pockets it belongs to.
    MakeRestPass {
        /// The project (.chipload).
        project: PathBuf,
        /// The rest pocket's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
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
    /// List a profile's or pocket's corners on the tool's side, with what each gets (round,
    /// dogbone, T-bone along the longer or shorter wall) and whether that is the corner's
    /// own choice (docs/specs/dogbone-per-corner.md).
    Corners {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
    },
    /// Give one corner of a profile or pocket its own bone, whatever the operation's corners
    /// say (docs/specs/dogbone-per-corner.md): `--bone none` keeps it round. `op corners`
    /// lists the corners.
    Corner {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
        /// The corner, X,Y in drawing coordinates (a plain number is mm).
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true,
              required_unless_present = "reset")]
        at: Option<[f64; 2]>,
        /// What the corner gets.
        #[arg(long, value_enum, conflicts_with_all = ["as_operation", "reset"],
              required_unless_present_any = ["as_operation", "reset"])]
        bone: Option<BoneArg>,
        /// With a T-bone: the wall it runs along (the notch is in the other); default longer.
        #[arg(long, value_enum)]
        along: Option<WallArg>,
        /// Remove the corner's own choice: it gets what the operation gives every corner.
        #[arg(long, conflicts_with = "reset")]
        as_operation: bool,
        /// Remove every corner's own choice.
        #[arg(long)]
        reset: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List a profile's tabs: each contour's, with their centres on the tool path and
    /// whether they were placed by hand (docs/specs/tabs-by-hand.md). The job is planned.
    Tabs {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
    },
    /// Place a profile's tabs by hand (docs/specs/tabs-by-hand.md): add a tab at a point, or
    /// remove the one there. The first on a contour keeps that contour's other tabs where
    /// Chipload put them. `--even` spaces one contour's tabs evenly again, `--reset` all.
    Tab {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
        /// The point, X,Y in drawing coordinates (a plain number is mm): on or near the
        /// contour, within a tool diameter.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true,
              required_unless_present = "reset")]
        at: Option<[f64; 2]>,
        /// Remove the tab nearest to --at.
        #[arg(long, conflicts_with_all = ["even", "reset"])]
        remove: bool,
        /// Space the tabs of the contour nearest to --at evenly again.
        #[arg(long, conflicts_with = "reset")]
        even: bool,
        /// Space every contour's tabs evenly again.
        #[arg(long)]
        reset: bool,
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

#[derive(Clone, Copy, ValueEnum)]
enum BoneArg {
    None,
    Dogbone,
    Tbone,
}

#[derive(Clone, Copy, ValueEnum)]
enum WallArg {
    Longer,
    Shorter,
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
    /// Move drawn shapes by --dx and --dy; they keep their ids, so the operations that cut
    /// them follow (docs/specs/drawn-shapes-box.md).
    Move {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far to move them right, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dx: f64,
        /// How far to move them up, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Turn drawn shapes counter-clockwise about the middle of the box round them, or a
    /// point.
    Turn {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The angle, degrees, counter-clockwise; negative turns clockwise.
        #[arg(long, allow_hyphen_values = true)]
        angle_deg: f64,
        /// Turn about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Scale drawn shapes evenly about the middle of the box round them, or a point.
    Scale {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The factor, greater than 0: 2 doubles them.
        #[arg(long)]
        factor: f64,
        /// Scale about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Add the curve at a distance from drawn shapes, on their layers; the shapes stay
    /// (docs/specs/drawing-editing.md §2). Prints the new ids.
    Offset {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        distance: f64,
        /// Which side: outside or inside of a closed shape, left or right of an open one
        /// (default: outside, or left).
        #[arg(long, value_enum)]
        side: Option<OffsetSideArg>,
        /// Round corners (the exact offset) or sharp ones (the sides extended to meet).
        #[arg(long, value_enum, default_value = "round")]
        corners: CornersArg,
        /// How many, each the distance further than the last.
        #[arg(long, default_value_t = 1)]
        count: u32,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Round corners of a drawn shape with arcs of a radius; it keeps its id
    /// (docs/specs/drawing-editing.md §3).
    Fillet {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// The radius, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        radius: f64,
        /// Only this corner: its point's number, from 0 (default: every corner).
        #[arg(long)]
        corner: Option<usize>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Trim a drawn shape: the piece of it at --at, between the nearest crossings with the
    /// shapes shown, goes; the rest keeps the id, a second piece is a new shape
    /// (docs/specs/drawing-editing.md §4).
    Trim {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// A point on (or near) the piece to remove, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        at: [f64; 2],
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Extend an open drawn shape's straight end along itself to the nearest shape shown
    /// ahead (docs/specs/drawing-editing.md §4).
    Extend {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// Which end.
        #[arg(long, value_enum)]
        end: EndArg,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Break a drawn shape at the point of it nearest --at: an open one becomes two, a
    /// closed one opens there (docs/specs/drawing-editing.md §4).
    Break {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// The point, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        at: [f64; 2],
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Join closed drawn shapes into one: everything any of them covers. The result takes
    /// the first shape's id; the others go (docs/specs/drawing-editing.md §5).
    Union {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, two or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 2..)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Cut closed drawn shapes out of another: BASE keeps its id, the cutters go
    /// (docs/specs/drawing-editing.md §5).
    Subtract {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape cut (as `inspect` lists it, without the S).
        base: u64,
        /// The shapes cut out of it.
        #[arg(required = true)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// What closed drawn shapes have in common: the result takes the first shape's id; the
    /// others go (docs/specs/drawing-editing.md §5).
    Intersect {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, two or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 2..)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Cut corners of a drawn shape straight across, a distance back along each side; it
    /// keeps its id (docs/specs/drawing-editing.md §3).
    Chamfer {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// How far back along each side, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        distance: f64,
        /// Only this corner: its point's number, from 0 (default: every corner).
        #[arg(long)]
        corner: Option<usize>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Join open drawn polylines whose ends are closer than --gap into one, on the same
    /// layer; a chain that comes back to its start closes (docs/specs/explode-and-join.md).
    Join {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S); none: every drawn shape.
        ids: Vec<u64>,
        /// Join ends closer than this, 0.001 to 1 mm.
        #[arg(long, value_parser = crate::lengths::length, default_value = "0.01")]
        gap: f64,
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
enum OffsetSideArg {
    Outside,
    Inside,
    Left,
    Right,
}

#[derive(Clone, Copy, ValueEnum)]
enum EndArg {
    Start,
    End,
}

#[derive(Clone, Copy, ValueEnum)]
enum CornersArg {
    Round,
    Sharp,
}

#[derive(Clone, Copy, ValueEnum)]
enum KindArg {
    Circle,
    Rectangle,
    Polyline,
    Polygon,
}

/// Defining values, all in mm; the angle in degrees.
#[derive(clap::Args)]
struct ShapeValues {
    /// Centre "X,Y" (circle, rectangle, polygon).
    #[arg(long, allow_hyphen_values = true)]
    center: Option<String>,
    /// A circle's radius; a polygon's, to its corners.
    #[arg(value_parser = crate::lengths::length, long)]
    radius: Option<f64>,
    /// A polygon's number of sides, 3 to 1000.
    #[arg(long)]
    sides: Option<u32>,
    /// A polygon's size across its flats (twice the radius to the middle of a side), in
    /// place of --radius.
    #[arg(value_parser = crate::lengths::length, long, conflicts_with = "radius")]
    across_flats: Option<f64>,
    /// Along the rectangle's own X axis.
    #[arg(value_parser = crate::lengths::length, long)]
    length: Option<f64>,
    /// Along the rectangle's own Y axis.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// Rotation about the centre (a polygon: its first corner's angle from +X), degrees,
    /// counter-clockwise.
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
    /// Its name in the Browser; "" takes the name away.
    #[arg(long)]
    name: Option<String>,
}

/// A language the setup sheet can be written in (docs/specs/translations.md step 3).
#[derive(Clone, Copy, ValueEnum)]
pub enum LangArg {
    En,
    De,
    Fr,
    Es,
    It,
    Sv,
}

impl LangArg {
    pub fn value(self) -> Language {
        match self {
            LangArg::En => Language::En,
            LangArg::De => Language::De,
            LangArg::Fr => Language::Fr,
            LangArg::Es => Language::Es,
            LangArg::It => Language::It,
            LangArg::Sv => Language::Sv,
        }
    }
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

/// The command line's stack: Linux's default for a main thread. Windows gives its main
/// thread 1 MB, which the parser's command tree overflows in debug builds (CI, 2026-10-17:
/// every command exited with STATUS_STACK_OVERFLOW).
const STACK_BYTES: usize = 8 * 1024 * 1024;

fn main() -> ExitCode {
    let worker = std::thread::Builder::new()
        .name("chipload".into())
        .stack_size(STACK_BYTES)
        .spawn(command_line);
    match worker.map(std::thread::JoinHandle::join) {
        Ok(Ok(code)) => code,
        // A panic has already printed its message; a thread that can't start says why.
        Ok(Err(_)) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: can't start: {err}");
            ExitCode::FAILURE
        }
    }
}

fn command_line() -> ExitCode {
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
            sheet_lang,
            sets,
        } => {
            let sheet = (!no_sheet).then(|| {
                (
                    units(sheet_units).unwrap_or(Units::Mm),
                    sheet_lang.map_or(Language::En, LangArg::value),
                )
            });
            run(out, &project, &output, tool_change, sheet, &sets)
        }
        Command::Sheet {
            project,
            output,
            gcode,
            tool_change,
            units: sheet_units,
            lang,
            sets,
        } => sheet(
            out,
            &project,
            &output,
            gcode.as_deref(),
            tool_change,
            (
                units(sheet_units).unwrap_or(Units::Mm),
                lang.map_or(Language::En, LangArg::value),
            ),
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
        Err(err) => return out.fail(err.english()),
    };
    if let Err(err) = save_job(&job, output) {
        return out.fail(err);
    }
    out.line(line.to_string());
    for problem in &library.problems {
        out.err(format!("warning: {}", problem.english()));
    }
    for note in notes {
        out.line(note.english());
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

/// "S2, S3".
fn shape_names(ids: &[u64]) -> String {
    ids.iter()
        .map(|id| format!("S{id}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `shape union`, `subtract` or `intersect`: the project and what to print; the new shapes'
/// ids and the number of operations flagged (they used a shape that is gone).
fn combined(
    job: &Job,
    op: Combine,
    ids: &[u64],
    new_ids: &mut Option<Vec<u64>>,
    flagged: &mut usize,
) -> Result<(Job, Vec<String>), PipelineError> {
    let edited = combine_shapes(job, op, ids)?;
    let gone: Vec<u64> = ids
        .iter()
        .copied()
        .filter(|id| !edited.job.drawing.shapes.iter().any(|s| s.id == *id))
        .collect();
    let mut lines = Vec::new();
    if !gone.is_empty() {
        lines.push(format!("Removed shapes {}.", shape_names(&gone)));
    }
    if !edited.new_ids.is_empty() {
        lines.push(format!("Added shapes {}.", shape_names(&edited.new_ids)));
    }
    *flagged = job
        .operations
        .iter()
        .filter(|op| {
            op.select()
                .shapes
                .as_ref()
                .is_some_and(|s| s.iter().any(|id| gone.contains(id)))
        })
        .count();
    lines.extend(edited.notes.iter().map(Message::english));
    *new_ids = Some(edited.new_ids);
    Ok((edited.job, lines))
}

/// `about`, or the middle of the box round shapes `ids`.
fn about_point(
    job: &Job,
    ids: &[u64],
    about: Option<[f64; 2]>,
) -> Result<(f64, f64), PipelineError> {
    match about {
        Some([x, y]) => Ok((x, y)),
        None => drawn_middle(job, ids),
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
        }
        | ShapeAction::Join {
            project, output, ..
        }
        | ShapeAction::Offset {
            project, output, ..
        }
        | ShapeAction::Fillet {
            project, output, ..
        }
        | ShapeAction::Chamfer {
            project, output, ..
        }
        | ShapeAction::Trim {
            project, output, ..
        }
        | ShapeAction::Extend {
            project, output, ..
        }
        | ShapeAction::Break {
            project, output, ..
        }
        | ShapeAction::Union {
            project, output, ..
        }
        | ShapeAction::Subtract {
            project, output, ..
        }
        | ShapeAction::Intersect {
            project, output, ..
        }
        | ShapeAction::Move {
            project, output, ..
        }
        | ShapeAction::Turn {
            project, output, ..
        }
        | ShapeAction::Scale {
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
    // What `shape join` joined and removed.
    let mut joined: Option<(Vec<u64>, Vec<u64>)> = None;
    let changed = match action {
        ShapeAction::Add { kind, values, .. } => new_geometry(kind, &values)
            .map_err(|e| PipelineError::Job(foreign_text(e)))
            .and_then(|geometry| {
                let layer = values.layer.as_deref().unwrap_or("0");
                let (job, id) = add_shape(&job, layer, geometry)?;
                match &values.name {
                    Some(name) => name_shape(&job, id, name).map(|job| (job, id)),
                    None => Ok((job, id)),
                }
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
            .ok_or_else(|| chipload_pipeline::shapes::no_shape(id))
            .and_then(|shape| {
                let geometry = changed_geometry(&shape.geometry, &values)
                    .map_err(|e| PipelineError::Job(foreign_text(e)))?;
                let layer = values.layer.clone().unwrap_or_else(|| shape.layer.clone());
                let job = set_shape(&job, id, &layer, geometry)?;
                match &values.name {
                    Some(name) => name_shape(&job, id, name),
                    None => Ok(job),
                }
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
        ShapeAction::Offset {
            ids,
            distance,
            side,
            corners,
            count,
            ..
        } => {
            let side = side.map(|s| match s {
                OffsetSideArg::Outside => OffsetSide::Outside,
                OffsetSideArg::Inside => OffsetSide::Inside,
                OffsetSideArg::Left => OffsetSide::Left,
                OffsetSideArg::Right => OffsetSide::Right,
            });
            let corners = match corners {
                CornersArg::Round => Corners::Round,
                CornersArg::Sharp => Corners::Sharp,
            };
            offset_shapes(&job, &ids, distance, side, corners, count).map(|edited| {
                new_ids = Some(edited.new_ids.clone());
                let mut lines = vec![format!("Added shapes {}.", shape_names(&edited.new_ids))];
                lines.extend(edited.notes.iter().map(Message::english));
                (edited.job, lines)
            })
        }
        ShapeAction::Fillet {
            id, radius, corner, ..
        } => cut_shape_corners(&job, id, corner, Cut::Fillet(radius))
            .map(|job| (job, vec![format!("Rounded the corners of S{id}.")])),
        ShapeAction::Chamfer {
            id,
            distance,
            corner,
            ..
        } => cut_shape_corners(&job, id, corner, Cut::Chamfer(distance))
            .map(|job| (job, vec![format!("Cut the corners of S{id}.")])),
        ShapeAction::Trim { id, at, .. } => trim_shape(&job, id, (at[0], at[1])).map(|edited| {
            new_ids = Some(edited.new_ids.clone());
            let mut lines = vec![format!("Trimmed S{id}.")];
            if !edited.new_ids.is_empty() {
                lines.push(format!("Added shapes {}.", shape_names(&edited.new_ids)));
            }
            lines.extend(edited.notes.iter().map(Message::english));
            (edited.job, lines)
        }),
        ShapeAction::Extend { id, end, .. } => {
            let end = match end {
                EndArg::Start => End::Start,
                EndArg::End => End::End,
            };
            extend_shape(&job, id, end).map(|edited| (edited.job, vec![format!("Extended S{id}.")]))
        }
        ShapeAction::Break { id, at, .. } => break_shape(&job, id, (at[0], at[1])).map(|edited| {
            new_ids = Some(edited.new_ids.clone());
            let mut lines = vec![format!("Broke S{id}.")];
            if !edited.new_ids.is_empty() {
                lines.push(format!("Added shapes {}.", shape_names(&edited.new_ids)));
            }
            lines.extend(edited.notes.iter().map(Message::english));
            (edited.job, lines)
        }),
        ShapeAction::Union { ids, .. } => {
            combined(&job, Combine::Union, &ids, &mut new_ids, &mut flagged_ops)
        }
        ShapeAction::Subtract { base, ids, .. } => {
            let all: Vec<u64> = std::iter::once(base).chain(ids).collect();
            combined(
                &job,
                Combine::Subtract,
                &all,
                &mut new_ids,
                &mut flagged_ops,
            )
        }
        ShapeAction::Intersect { ids, .. } => combined(
            &job,
            Combine::Intersect,
            &ids,
            &mut new_ids,
            &mut flagged_ops,
        ),
        ShapeAction::Move { ids, dx, dy, .. } => {
            transform_shapes(&job, &ids, (0.0, 0.0), (dx, dy), 1.0, 0.0).map(|job| {
                let text = format!("Moved shapes {}.", shape_names(&ids));
                (job, vec![text])
            })
        }
        ShapeAction::Turn {
            ids,
            angle_deg,
            about,
            ..
        } => about_point(&job, &ids, about).and_then(|c| {
            transform_shapes(&job, &ids, c, c, 1.0, angle_deg).map(|job| {
                let text = format!("Turned shapes {} by {angle_deg}°.", shape_names(&ids));
                (job, vec![text])
            })
        }),
        ShapeAction::Scale {
            ids, factor, about, ..
        } => about_point(&job, &ids, about).and_then(|c| {
            transform_shapes(&job, &ids, c, c, factor, 0.0).map(|job| {
                let text = format!("Scaled shapes {} ×{factor}.", shape_names(&ids));
                (job, vec![text])
            })
        }),
        ShapeAction::Join { ids, gap, .. } => join_shapes(&job, &ids, gap).map(|(job, report)| {
            joined = Some((report.shapes, report.removed));
            (job, report.lines)
        }),
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
    if let Some((shapes, removed)) = joined {
        out.set("shapes", json!(shapes));
        out.set("removed", json!(removed));
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
    if (v.sides.is_some() || v.across_flats.is_some()) && !matches!(kind, KindArg::Polygon) {
        return Err("--sides and --across-flats are for a polygon".into());
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
        KindArg::Polygon => {
            let sides = need(v.sides, "sides", "polygon")?;
            let radius = match v.across_flats {
                Some(flats) => polygon_radius_across_flats(flats, sides),
                None => need(v.radius, "radius", "polygon")?,
            };
            Geometry::Polygon {
                center: need(center, "center", "polygon")?,
                radius,
                sides,
                angle: v.angle_deg.unwrap_or(0.0).to_radians(),
            }
        }
    })
}

/// A polygon's radius to its corners from its size across the flats.
fn polygon_radius_across_flats(flats: f64, sides: u32) -> f64 {
    flats / 2.0 / (std::f64::consts::PI / f64::from(sides.max(3))).cos()
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
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *radius = v.radius.unwrap_or(*radius);
        }
        Geometry::Polygon {
            center: c,
            radius,
            sides,
            angle,
        } => {
            not_for(
                "polygon",
                &[
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("corner-radius", v.corner_radius.is_some()),
                    ("points", v.points.is_some()),
                    ("bulges", v.bulges.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *sides = v.sides.unwrap_or(*sides);
            *radius = match v.across_flats {
                Some(flats) => polygon_radius_across_flats(flats, *sides),
                None => v.radius.unwrap_or(*radius),
            };
            if let Some(degrees) = v.angle_deg {
                *angle = degrees.to_radians();
            }
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
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
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
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
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
/// lengths in `units`, its words in `language`.
fn sheet_info(project: &Path, (units, language): (Units, Language)) -> SheetInfo {
    SheetInfo {
        project: project.file_name().map_or_else(
            || project.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        written_by: format!("chipload {}", env!("CARGO_PKG_VERSION")),
        units,
        language,
    }
}

fn sheet(
    out: &mut Out,
    project: &Path,
    output: &Path,
    gcode: Option<&Path>,
    tool_change: Option<library::ToolChangeArg>,
    words: (Units, Language),
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
    let html = setup_sheet(&job, &result, &gcode, &sheet_info(project, words));
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
    sheet: Option<(Units, Language)>,
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
            "operations": operations_json(&file.operations),
            "rest_passes": rest_passes_json(&file.operations),
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
    if let Some(words) = sheet {
        match write_setup_sheet(&job, &result, output, &sheet_info(project, words)) {
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
        OpAction::RestPass {
            project,
            op,
            pass,
            tool,
            feed,
            plunge_feed,
            step_down,
            stepover,
            enabled,
            remove,
            output,
        } => rest_pass_command(
            out,
            &project,
            op,
            pass,
            RestPassChange {
                tool,
                feed,
                plunge_feed,
                step_down,
                stepover,
                enabled,
                remove,
            },
            output.as_deref(),
        ),
        OpAction::MakeRestPass {
            project,
            op,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(index) = op.checked_sub(1) else {
                return out.fail("operations are numbered from 1");
            };
            let Some(at) = chipload_pipeline::make_rest_pass(&mut job, index) else {
                return out.fail(format!(
                    "operation {op} is not a rest pocket that copies an earlier pocket \
                     (same shapes, depths, clearance and direction, rest after its tool)"
                ));
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            out.line(format!(
                "Operation {op} is now rest pass {at} of operation {}; the operations after it \
                 move up one. It is cut after the run of pockets it belongs to.",
                at.operation + 1
            ));
            out.set("operation", json!(at.operation));
            out.set("pass", json!(at.pass));
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        OpAction::AddSurface {
            project,
            tool,
            depth,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let library = settings_dir()
                .map(|dir| Library::load(&dir))
                .unwrap_or_default();
            let chosen = tool.or_else(|| chipload_pipeline::surface::surface_tool(&job));
            let cutting = job
                .tools
                .iter()
                .find(|t| Some(t.id) == chosen)
                .and_then(|t| chipload_job::library::linked_cutting(&library.tools, t));
            let index = match chipload_pipeline::surface::add_surface(
                &mut job,
                tool,
                depth,
                cutting.as_ref(),
            ) {
                Ok(index) => index,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            let op = &job.operations[index];
            let (_, end_depth) = op.depths();
            let moved = if index + 1 < job.operations.len() {
                " The operations after it move down one."
            } else {
                ""
            };
            out.line(format!(
                "Added operation {}: a surface of the whole stock with tool {}, {} mm off the \
                 top.{moved}",
                index + 1,
                op.tool(),
                chipload_core::text::mm(end_depth)
            ));
            out.set("operation", json!(index));
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
            let at = match add_rest_pass(&mut job, op, tool, cutting.as_ref()) {
                Ok(at) => at,
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
                "Added rest pass {at} to operation {op}, tool {tool}, with {from}."
            ));
            out.set("operation", json!(at.operation));
            out.set("pass", json!(at.pass));
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
        OpAction::Tabs { project, op } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(index) = op.checked_sub(1) else {
                return out.fail("operations are numbered from 1");
            };
            let rows = match chipload_pipeline::tabs::tabs_of(&job, index) {
                Ok(rows) => rows,
                Err(err) => return out.fail(err),
            };
            let mut list = Vec::new();
            for row in &rows {
                let how = if row.by_hand {
                    "placed by hand"
                } else {
                    "spaced"
                };
                out.line(format!(
                    "ring {}  ({}, {})  {how}",
                    row.ring + 1,
                    chipload_core::text::mm(row.at.0),
                    chipload_core::text::mm(row.at.1)
                ));
                list.push(json!({
                    "ring": row.ring + 1,
                    "at": [row.at.0, row.at.1],
                    "by_hand": row.by_hand,
                }));
            }
            out.set("tabs", Value::Array(list));
            ExitCode::SUCCESS
        }
        OpAction::Tab {
            project,
            op,
            at,
            remove,
            even,
            reset,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(index) = op.checked_sub(1) else {
                return out.fail("operations are numbered from 1");
            };
            let at = at.unwrap_or_default();
            let (x, y) = (
                chipload_core::text::mm(at[0]),
                chipload_core::text::mm(at[1]),
            );
            use chipload_pipeline::tabs::{add_tab, even_ring, remove_tab, reset_tabs};
            let done = if reset {
                reset_tabs(&mut job, index).map(|count| {
                    format!("Operation {op}'s tabs are spaced evenly again ({count} were placed by hand).")
                })
            } else if even {
                even_ring(&mut job, index, at).map(|count| {
                    format!(
                        "The tabs of the contour at ({x}, {y}) are spaced evenly again ({count} were placed by hand)."
                    )
                })
            } else if remove {
                remove_tab(&mut job, index, at)
                    .map(|()| format!("Removed operation {op}'s tab nearest to ({x}, {y})."))
            } else {
                add_tab(&mut job, index, at)
                    .map(|()| format!("Added a tab to operation {op} at ({x}, {y})."))
            };
            let line = match done {
                Ok(line) => line,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            out.line(line);
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        OpAction::Corners { project, op } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(index) = op.checked_sub(1) else {
                return out.fail("operations are numbered from 1");
            };
            let rows = match chipload_pipeline::corners::corners_of(&job, index) {
                Ok(rows) => rows,
                Err(err) => return out.fail(err),
            };
            if rows.is_empty() {
                out.line(format!("Operation {op} has no corners on the tool's side."));
            }
            let mut list = Vec::new();
            for row in &rows {
                let gets = gets_name(row.gets);
                let own = if row.chosen { " (its own choice)" } else { "" };
                out.line(format!(
                    "({}, {})  {:.1}°  {gets}{own}",
                    chipload_core::text::mm(row.at[0]),
                    chipload_core::text::mm(row.at[1]),
                    row.angle.to_degrees()
                ));
                list.push(json!({
                    "at": row.at,
                    "angle_deg": row.angle.to_degrees(),
                    "gets": gets,
                    "chosen": row.chosen,
                }));
            }
            out.set("corners", Value::Array(list));
            ExitCode::SUCCESS
        }
        OpAction::Corner {
            project,
            op,
            at,
            bone,
            along,
            as_operation: _,
            reset,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(index) = op.checked_sub(1) else {
                return out.fail("operations are numbered from 1");
            };
            let done = if reset {
                chipload_pipeline::corners::reset_corners(&mut job, index).map(|count| {
                    format!("Removed the choices of {count} corners of operation {op}.")
                })
            } else {
                let at = at.unwrap_or_default();
                let choice = bone.map(|bone| {
                    let bone = match bone {
                        BoneArg::None => chipload_job::Bone::None,
                        BoneArg::Dogbone => chipload_job::Bone::Dogbone,
                        BoneArg::Tbone => chipload_job::Bone::Tbone,
                    };
                    let along = along.map(|wall| match wall {
                        WallArg::Longer => chipload_job::Wall::Longer,
                        WallArg::Shorter => chipload_job::Wall::Shorter,
                    });
                    (bone, along)
                });
                if along.is_some() && !matches!(bone, Some(BoneArg::Tbone)) {
                    return out.fail("--along goes with --bone tbone");
                }
                chipload_pipeline::corners::choose_corner(&mut job, index, at, choice).map(
                    |corner| {
                        let (x, y) = (
                            chipload_core::text::mm(corner[0]),
                            chipload_core::text::mm(corner[1]),
                        );
                        match choice {
                            None => format!(
                                "The corner at ({x}, {y}) of operation {op} gets what the \
                                 operation gives every corner."
                            ),
                            Some(_) => {
                                let gets = match bone {
                                    Some(BoneArg::None) => "round",
                                    Some(BoneArg::Dogbone) => "a dogbone",
                                    _ => match along {
                                        Some(WallArg::Shorter) => "a T-bone along the shorter wall",
                                        _ => "a T-bone along the longer wall",
                                    },
                                };
                                format!("The corner at ({x}, {y}) of operation {op} is now {gets}.")
                            }
                        }
                    },
                )
            };
            let line = match done {
                Ok(line) => line,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            out.line(line);
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

/// What a corner gets, as `op corners` names it.
fn gets_name(gets: Option<chipload_toolpath::corners::Gets>) -> &'static str {
    use chipload_job::Wall;
    use chipload_toolpath::corners::Gets;
    match gets {
        None => "round",
        Some(Gets::Dogbone) => "dogbone",
        Some(Gets::Tbone(Wall::Longer)) => "T-bone along the longer wall",
        Some(Gets::Tbone(Wall::Shorter)) => "T-bone along the shorter wall",
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
                "operations": operations_json(&g.operations),
                "rest_passes": rest_passes_json(&g.operations),
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

/// What `op rest-pass` changes in one pass.
struct RestPassChange {
    tool: Option<i64>,
    feed: Option<f64>,
    plunge_feed: Option<f64>,
    step_down: Option<f64>,
    stepover: Option<f64>,
    enabled: Option<bool>,
    remove: bool,
}

/// `op rest-pass`: list a pocket's passes, or change or remove one.
fn rest_pass_command(
    out: &mut Out,
    project: &Path,
    op: usize,
    pass: Option<usize>,
    change: RestPassChange,
    output: Option<&Path>,
) -> ExitCode {
    let mut job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let Some(index) = op.checked_sub(1).filter(|&i| i < job.operations.len()) else {
        return out.fail(format!("there is no operation {op}"));
    };
    let Some(chipload_job::Operation::Pocket(pocket)) = job.operations.get_mut(index) else {
        return out.fail(format!("operation {op} is not a pocket"));
    };
    let Some(k) = pass else {
        if pocket.rest_passes.is_empty() {
            out.line(format!("Operation {op} has no rest passes."));
        }
        for (k, p) in pocket.rest_passes.iter().enumerate() {
            out.line(format!(
                "{}: tool {}, feed {} / plunge {} mm/min, step down {} mm, stepover {}{}",
                chipload_job::OpRef::pass(index, k),
                p.tool,
                p.feed,
                p.plunge_feed,
                p.step_down,
                p.stepover,
                if p.enabled { "" } else { ", off" }
            ));
        }
        out.set("rest_passes", json!(pocket.rest_passes));
        return ExitCode::SUCCESS;
    };
    let Some(k) = k.checked_sub(1).filter(|&k| k < pocket.rest_passes.len()) else {
        return out.fail(format!(
            "operation {op} has {} rest passes; passes are numbered from 1",
            pocket.rest_passes.len()
        ));
    };
    let at = chipload_job::OpRef::pass(index, k);
    if change.remove {
        pocket.rest_passes.remove(k);
        out.line(format!("Removed rest pass {at}."));
    } else {
        let p = &mut pocket.rest_passes[k];
        if let Some(tool) = change.tool {
            p.tool = tool;
        }
        if let Some(v) = change.feed {
            p.feed = v;
        }
        if let Some(v) = change.plunge_feed {
            p.plunge_feed = v;
        }
        if let Some(v) = change.step_down {
            p.step_down = v;
        }
        if let Some(v) = change.stepover {
            p.stepover = v;
        }
        if let Some(on) = change.enabled {
            p.enabled = on;
        }
        out.line(format!("Changed rest pass {at}."));
    }
    // The passes' own problems refuse the change; the project's others are as they were.
    let prefix = format!("operations.{index}.rest_passes");
    let problems: Vec<_> = chipload_pipeline::job_problems(&job)
        .into_iter()
        .filter(|p| p.path.starts_with(&prefix))
        .collect();
    if !problems.is_empty() {
        return out.fail(chipload_pipeline::joined(&problems));
    }
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// The operations among `refs`, 0-based, as `--json` always listed them.
fn operations_json(refs: &[chipload_job::OpRef]) -> Value {
    json!(
        refs.iter()
            .filter(|r| r.pass.is_none())
            .map(|r| r.operation)
            .collect::<Vec<_>>()
    )
}

/// The rest passes among `refs` (docs/specs/rest-passes.md): `{operation, pass}`, 0-based.
fn rest_passes_json(refs: &[chipload_job::OpRef]) -> Value {
    json!(
        refs.iter()
            .filter_map(|r| r.pass.map(|k| json!({"operation": r.operation, "pass": k})))
            .collect::<Vec<_>>()
    )
}

/// The cycle time estimate as `plan` and `run` print it with `--json`.
fn estimate_json(estimate: &chipload_pipeline::estimate::Estimate) -> Value {
    json!({
        "total_s": estimate.total,
        "cutting_s": estimate.cutting,
        "rapid_s": estimate.rapid,
        "spindle_s": estimate.spindle,
        "operations": estimate.operations.iter().map(|(at, s)| json!({
            "operation": at.operation,
            "pass": at.pass,
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
