// SPDX-License-Identifier: GPL-3.0-or-later
//! The `chipload` command line: import drawings into projects, plan and run them, check
//! G-code. Projects are `.chipload` files (docs/specs/project-file.md).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod dxfs;
mod frames;
mod groups;
mod images;
mod lengths;
mod library;
mod materials;
mod models;
mod parts;
mod setups;
mod shape_library;
mod signs;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_check::{CheckIssue, check_gcode, limits_for_tool, limits_from_job};
use chipload_core::chain::Contour;
use chipload_dxf::{
    ImportResult, UnitsFrom, contours_from_shapes, import_dxf, is_svg, shapes_from_file, units_name,
};
use chipload_i18n::{Language, Message};
use chipload_job::edit::Mirror;
use chipload_job::{
    Copies, CopyOrder, Geometry, Job, Placement, Spindle, StockCorner, Units, WorkZero, ZZero,
    load_job, polyline_segment_count, save_job,
};
use chipload_pipeline::chip_load::{chip_load, chip_load_lines};
use chipload_pipeline::copies::{copies_text, grid_text, parts_lines, pitch_for_gap};
use chipload_pipeline::drawing_edit::{
    AlignTo, AlignWith, Combine, Corners, Cut, End, OffsetSide, Polar, align_shapes, array_grid,
    array_polar, break_shape, combine_shapes, cut_shape_corners, distribute_shapes, extend_shape,
    offset_shapes, trim_shape,
};
use chipload_pipeline::estimate::{Lengths, estimate, estimate_lines};
use chipload_pipeline::explode::join_shapes;
use chipload_pipeline::library::{Library, apply_library, settings_dir};
use chipload_pipeline::placement::{
    Move as PlaceMove, dxf_list, dxf_size, place_drawing, place_dxf, placement_text,
};
use chipload_pipeline::report::{
    format_contours, format_import_report, format_shapes, severity_name,
};
use chipload_pipeline::shapes::{
    add_shape, copy_shapes, delete_shape, drawn_middle, mirror_shapes, name_shape,
    set_construction, set_shape, transform_shapes,
};
use chipload_pipeline::sheet::{SheetInfo, setup_sheet, write_setup_sheet};
use chipload_pipeline::{
    PipelineError, RunResult, add_floor_pass, add_rest_pass, empty_project, foreign_text,
    load_drawing, new_project, run_job, set_op_rest, set_op_tool, update_drawing_with,
    write_atomically, write_gcode_files,
};
use chipload_post::{load_builtin_post, render_program};
use chipload_toolpath::arcs::Circle;
use chipload_toolpath::model::{Move, MoveKind, Program};
use chipload_toolpath::{ToolpathError, plan_job};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

mod mcp;
mod ops;
mod out;
mod placing;
mod reference;
mod running;
mod shapes;
mod styles;
mod texts;

use ops::*;
use placing::*;
use running::*;
use shapes::*;

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
    /// Dump units, entities, contours and issues of a DXF, an SVG or a project.
    Inspect {
        /// A .dxf or .svg drawing or a .chipload project.
        file: PathBuf,
        /// Units of a DXF that does not declare them.
        #[arg(long)]
        units: Option<UnitsArg>,
        /// Pixels per inch of an SVG that gives no real size: 96 (Inkscape, browsers) or 72
        /// (older Illustrator).
        #[arg(long)]
        px_per_inch: Option<f64>,
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
    /// Create a project from a DXF or an SVG.
    Import {
        /// The DXF or SVG (.svg, .svgz).
        dxf: PathBuf,
        /// Units of a DXF that does not declare them.
        #[arg(long)]
        units: Option<UnitsArg>,
        /// Pixels per inch of an SVG that gives no real size: 96 (Inkscape, browsers) or 72
        /// (older Illustrator).
        #[arg(long)]
        px_per_inch: Option<f64>,
        /// The project to write (.chipload).
        #[arg(short, long)]
        output: PathBuf,
        /// The machine profile to start from (default: your default machine).
        #[arg(long)]
        machine: Option<String>,
    },
    /// Replace a project's imported drawing with a new version of the DXF or SVG.
    Update {
        /// The project (.chipload).
        project: PathBuf,
        /// The new DXF or SVG.
        #[arg(long)]
        from: PathBuf,
        /// Units of the new DXF, if they differ from the last import.
        #[arg(long)]
        units: Option<UnitsArg>,
        /// Pixels per inch of an SVG without a real size, if they differ from the last
        /// import.
        #[arg(long)]
        px_per_inch: Option<f64>,
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
        /// Only this setup (as `setup list` numbers it); default: every setup for `run`, the
        /// active one otherwise (docs/specs/setups.md §8).
        #[arg(long, value_name = "ID")]
        setup: Option<u64>,
        /// Only this part (as `part list` numbers it), as if every other part were switched
        /// off; may be given more than once (docs/specs/part-export.md).
        #[arg(long = "part", value_name = "ID")]
        parts: Vec<u64>,
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
        /// Only this setup (as `setup list` numbers it); default: every setup for `run`, the
        /// active one otherwise (docs/specs/setups.md §8).
        #[arg(long, value_name = "ID")]
        setup: Option<u64>,
        /// Only this part (as `part list` numbers it), as if every other part were switched
        /// off; may be given more than once (docs/specs/part-export.md).
        #[arg(long = "part", value_name = "ID")]
        parts: Vec<u64>,
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
        /// The setup to simulate (as `setup list` numbers it); default: the active one.
        #[arg(long, value_name = "ID")]
        setup: Option<u64>,
        /// The face the picture shows: front, or back (the board turned over, as it lies);
        /// default: the face the setup cuts (docs/specs/two-sided.md §10.2).
        #[arg(long, value_parser = ["front", "back"])]
        face: Option<String>,
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
        /// Only this setup (as `setup list` numbers it); default: every setup for `run`, the
        /// active one otherwise (docs/specs/setups.md §8).
        #[arg(long, value_name = "ID")]
        setup: Option<u64>,
        /// Only this part (as `part list` numbers it), as if every other part were switched
        /// off; may be given more than once (docs/specs/part-export.md).
        #[arg(long = "part", value_name = "ID")]
        parts: Vec<u64>,
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
    /// Groups of drawn shapes and texts: add, list, set, remove (ungroup), delete, move,
    /// turn, scale.
    Group {
        #[command(subcommand)]
        action: groups::GroupAction,
    },
    /// Pictures traced into shapes for V-carving: add, change, list, preview, remove.
    Image {
        #[command(subcommand)]
        action: images::ImageAction,
    },
    /// 3D models (STL) carved as reliefs from the top: add, change, list, remove
    /// (docs/specs/stl-relief.md).
    Model {
        #[command(subcommand)]
        action: models::ModelAction,
    },
    /// Text written into shapes for V-carving, pocketing or profiling: add, change, list,
    /// remove, and the built-in fonts (docs/specs/text.md).
    Text {
        #[command(subcommand)]
        action: texts::TextAction,
    },
    /// Frames for signs, drawn round shapes or to a size, with border lines
    /// (docs/specs/shape-library.md).
    Frame {
        #[command(subcommand)]
        action: frames::FrameAction,
    },
    /// A sign in one step: words, a frame that follows them, and every operation that
    /// cuts it (docs/specs/sign-maker.md).
    Sign {
        #[command(subcommand)]
        action: signs::SignAction,
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
    /// Your materials: your own feeds, step down and stepover per tool, and choosing one
    /// for a project (docs/specs/materials.md).
    Material {
        #[command(subcommand)]
        action: materials::MaterialAction,
    },
    /// Your styles: operations saved by name, used again on shapes in one go
    /// (docs/specs/styles.md).
    Style {
        #[command(subcommand)]
        action: styles::StyleAction,
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
    /// A project's setups: each a board clamped once, with its stock, work zero and parts,
    /// run as a program of its own (docs/specs/setups.md).
    Setup {
        #[command(subcommand)]
        action: setups::SetupAction,
    },
    /// Where the G-code's X0 Y0 Z0 is (docs/specs/work-zero-marker.md §5): a corner or the
    /// centre of the stock, the drawing origin, or a point. Moving it moves no cut, only the
    /// program's numbers. Without an option, print where it is.
    Zero {
        /// The project (.chipload).
        project: PathBuf,
        /// At a corner or the centre of the stock; it follows the stock's size.
        #[arg(long, conflicts_with_all = ["drawing_origin", "at"])]
        stock: Option<CornerArg>,
        /// At the drawing's origin.
        #[arg(long, conflicts_with = "at")]
        drawing_origin: bool,
        /// At the point "X,Y" in drawing coordinates (any length unit; plain numbers are mm).
        #[arg(long, allow_hyphen_values = true)]
        at: Option<String>,
        /// Z0 at the stock's top or bottom.
        #[arg(long)]
        z: Option<ZArg>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
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
    Materials,
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
    /// The materials (materials.json, and `material export` files).
    Materials,
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
    Ru,
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
            LangArg::Ru => Language::Ru,
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
        Command::Inspect {
            file,
            units: u,
            px_per_inch,
        } => inspect(out, &file, units(u), px_per_inch),
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
            px_per_inch,
            output,
            machine,
        } => import(
            out,
            &dxf,
            (units(u), px_per_inch),
            &output,
            machine.as_deref(),
        ),
        Command::Update {
            project,
            from,
            units: u,
            px_per_inch,
            dxf,
            output,
        } => update(
            out,
            &project,
            &from,
            (units(u), px_per_inch),
            dxf,
            output.as_deref(),
        ),
        Command::Plan {
            project,
            parts,
            setup,
            sets,
        } => plan(out, &project, &with_setup(sets, setup), &parts),
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
            parts,
            setup,
            sets,
        } => {
            let sheet = (!no_sheet).then(|| {
                (
                    units(sheet_units).unwrap_or(Units::Mm),
                    sheet_lang.map_or(Language::En, LangArg::value),
                )
            });
            run(
                out,
                &project,
                &output,
                tool_change,
                sheet,
                &sets,
                (&parts, setup),
            )
        }
        Command::Sheet {
            project,
            output,
            gcode,
            tool_change,
            units: sheet_units,
            lang,
            parts,
            setup,
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
            (&with_setup(sets, setup), &parts),
        ),
        Command::Simulate {
            project,
            output,
            cell,
            no_image,
            after,
            hash,
            setup,
            face,
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
                back_face: face.map(|f| f == "back"),
            },
            &with_setup(sets, setup),
        ),
        Command::Render { program, post } => render(out, &program, &post),
        Command::Check { gcode, job, tool } => check(out, &gcode, &job, tool),
        Command::Op { action } => op(out, action),
        Command::Shape { action } => shape(out, action),
        Command::Image { action } => images::image(out, action),
        Command::Model { action } => models::model(out, action),
        Command::Text { action } => texts::text(out, action),
        Command::Frame { action } => frames::frame(out, action),
        Command::Sign { action } => signs::sign(out, action),
        Command::Group { action } => groups::group_command(out, action),
        Command::Dxf { action } => dxfs::dxf(out, action),
        Command::Tool { action } => library::tool(out, action),
        Command::Material { action } => materials::material(out, action),
        Command::Style { action } => styles::style(out, action),
        Command::Machine { action } => library::machine(out, action),
        Command::Post { action } => library::post(out, action),
        Command::Part { action } => parts::part(out, action),
        Command::Setup { action } => setups::setup(out, action),
        Command::Zero {
            project,
            stock,
            drawing_origin,
            at,
            z,
            output,
        } => zero(
            out,
            &project,
            ZeroArgs {
                stock,
                drawing_origin,
                at,
                z,
            },
            output.as_deref(),
        ),
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
                SchemaArg::Materials => chipload_job::schema::Format::Materials,
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
        ReferencePage::Materials => reference::schema_page(
            "Materials file",
            "Your materials (`materials.json` in the settings folder) and the files `material \
             export` writes: your own values per library tool. Lengths in mm, feeds in mm/min.",
            &schema(Format::Materials),
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

/// A drawing file's size when it doesn't give one: a DXF's units, an SVG's pixels per inch.
type SizeGiven = (Option<Units>, Option<f64>);

fn import(
    out: &mut Out,
    dxf: &Path,
    (units, px_per_inch): SizeGiven,
    output: &Path,
    machine: Option<&str>,
) -> ExitCode {
    let imported = match shapes_from_file(dxf, units, px_per_inch, 1.0) {
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
    if let Err(err) = crate::save_project(out, &job, output) {
        return out.fail(err);
    }
    out.line(line.to_string());
    for problem in &library.problems {
        out.err(format!("warning: {}", problem.english()));
    }
    for note in &library.converted {
        out.line(note.english());
    }
    for note in notes {
        out.line(note.english());
    }
    out.wrote(output, "");
    ExitCode::SUCCESS
}

fn update(
    out: &mut Out,
    project: &Path,
    from: &Path,
    (units, px_per_inch): SizeGiven,
    dxf: Option<u64>,
    output: Option<&Path>,
) -> ExitCode {
    let job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let updated = match dxf {
        Some(id) => {
            chipload_pipeline::dxfs::update_drawing_file(&job, id, from, units, px_per_inch)
        }
        None => update_drawing_with(&job, from, units, px_per_inch),
    };
    let (updated, report) = match updated {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &updated, target) {
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

/// Save `job` to `target`, its live frames drawn again round what they go round first
/// (docs/specs/sign-maker.md §4), with a line for each one let go of. Every command that
/// writes a project saves through here.
pub(crate) fn save_project(
    out: &mut Out,
    job: &Job,
    target: &Path,
) -> Result<(), chipload_job::JobError> {
    let (job, notes) = chipload_pipeline::frames::follow_frames(job.clone());
    for note in notes {
        out.line(note.english());
    }
    save_job(&job, target)
}

/// Save `job` to `target` and say so, or fail with why it couldn't be saved.
fn write_project(out: &mut Out, job: &Job, target: &Path) -> ExitCode {
    if let Err(err) = crate::save_project(out, job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
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
        chipload_pipeline::texts::remake_changed(&job, &traced, &texts::computer_fonts())
            .map_err(|e| out.fail(e))?;
    for line in reports
        .iter()
        .flat_map(|r| &r.lines)
        .chain(text_reports.iter().flat_map(|r| &r.lines))
    {
        out.line(line.clone());
    }
    // A changed model's outline drawn again (docs/specs/stl-relief.md §4).
    let written = models::remake_changed(out, &job, written)?;
    // Frames round a changed text drawn again (docs/specs/sign-maker.md §4).
    let (followed, notes) = chipload_pipeline::frames::follow_frames(written);
    for note in notes {
        out.line(note.english());
    }
    Ok(followed)
}

/// `load_set`, then only the parts `parts` cut when any are given
/// (docs/specs/part-export.md); with them their names.
fn load_parts(
    out: &mut Out,
    project: &Path,
    sets: &[String],
    parts: &[u64],
) -> Result<(Job, Vec<String>), ExitCode> {
    let job = load_set(out, project, sets)?;
    if parts.is_empty() {
        return Ok((job, Vec::new()));
    }
    chipload_pipeline::parts::only_parts(&job, parts).map_err(|e| out.fail(e))
}

/// Load a project and run it, as `run` and `sheet` do; errors are printed. With `parts`,
/// only those parts, and their names.
fn load_and_run(
    out: &mut Out,
    project: &Path,
    tool_change: Option<library::ToolChangeArg>,
    sets: &[String],
    parts: &[u64],
) -> Result<(Job, RunResult, Vec<String>), ExitCode> {
    let (mut job, names) = load_parts(out, project, sets, parts)?;
    if let Some(change) = tool_change {
        job.machine.tool_change = change.value();
    }
    match run_job(&job) {
        Ok(result) => Ok((job, result, names)),
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

/// `sets` with `active_setup=ID` when a setup is chosen (docs/specs/setups.md §8).
fn with_setup(mut sets: Vec<String>, setup: Option<u64>) -> Vec<String> {
    if let Some(id) = setup {
        sets.push(format!("active_setup={id}"));
    }
    sets
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
    // `stock…` and `machine.work_zero…` are the active setup's since schema 34.
    let aliased = chipload_job::paths::alias(&job, path);
    let path = aliased.as_deref().unwrap_or(path);
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
    let fonts = texts::computer_fonts();
    let (changed, text_reports) =
        match chipload_pipeline::texts::remake_changed(&job, &changed, &fonts) {
            Ok(written) => written,
            Err(err) => return out.fail(err),
        };
    let changed = match models::remake_changed(out, &job, changed) {
        Ok(changed) => changed,
        Err(code) => return code,
    };
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &changed, target) {
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

fn inspect(out: &mut Out, path: &Path, units: Option<Units>, px: Option<f64>) -> ExitCode {
    let mut shapes = None;
    let mut work_zero = None;
    let mut copies_line: Vec<String> = Vec::new();
    let mut placement_line = None;
    let loaded = if is_svg(path) {
        shapes_from_file(path, units, px, 1.0)
            .map(|read| contours_from_shapes(&read.shapes, std::slice::from_ref(&read.source)))
            .map_err(|e| e.to_string())
    } else if is_dxf(path) {
        import_dxf(path, units).map_err(|e| e.to_string())
    } else {
        load_job(path).map_err(|e| e.to_string()).and_then(|job| {
            shapes = Some(job.drawing.shapes.clone());
            // Only when set, so a fresh project still inspects exactly like its DXF.
            if job.work_zero() != (WorkZero::DrawingOrigin {}) {
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
