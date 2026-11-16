// SPDX-License-Identifier: GPL-3.0-or-later
//! The command line's commands and their shared arguments (clap).

use super::*;

#[derive(Subcommand)]
// Parsed once from the command line: the size of `op add`'s many options doesn't matter.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Command {
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
        #[command(flatten)]
        start: StartArgs,
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
        #[command(flatten)]
        start: StartArgs,
        /// Don't apply your styles' layer rules (docs/specs/fewer-clicks.md §4); without
        /// this, each style with a layer pattern adds its operations for the layers it
        /// matches.
        #[arg(long)]
        no_rules: bool,
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
        /// Only this piece of a setup cut in pieces, such as A1 (docs/specs/pieces.md §7);
        /// default: every piece.
        #[arg(long, value_name = "NAME")]
        piece: Option<String>,
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
        /// Only this piece of a setup cut in pieces, on its own board; default: the pieces
        /// put together (docs/specs/pieces.md §7).
        #[arg(long, value_name = "NAME")]
        piece: Option<String>,
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
    /// A relief in one step: a 3D model turned, cut and sized, its relief, a roughing
    /// pass, and what is round it (docs/specs/relief-wizard.md).
    Relief {
        #[command(subcommand)]
        action: reliefs::ReliefAction,
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
    /// A project's parts: groups of operations, each cut with its instances before the next
    /// (docs/specs/parts.md), and the instances themselves (`part instance`).
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
    /// Cut a part several times in a grid: its instances made a grid, the same as `part
    /// instance array`; --off is `part instance remove --all` (docs/specs/instances.md §5).
    Copies {
        /// The project (.chipload).
        project: PathBuf,
        /// Which part (as `part list` shows it); needed when the project has several.
        #[arg(long)]
        part: Option<u64>,
        /// Columns x rows, e.g. "3x2".
        #[arg(long)]
        grid: Option<String>,
        /// Pitch "DX,DY" in mm, from cell to cell; negative grows left or down.
        #[arg(long, allow_hyphen_values = true, conflicts_with = "gap")]
        pitch: Option<String>,
        /// Set the pitch so that this many mm are left between the cuts of neighbours.
        #[arg(value_parser = crate::lengths::length, long)]
        gap: Option<f64>,
        /// The cut order.
        #[arg(long)]
        order: Option<OrderArg>,
        /// No instances: the part is cut once.
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
pub(crate) enum ReferencePage {
    Cli,
    Project,
    Tools,
    Machines,
    Materials,
    Posts,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum SchemaArg {
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

/// What a new project starts with (docs/specs/new-project-dialog.md §5): without these,
/// the library's defaults.
#[derive(clap::Args)]
pub struct StartArgs {
    /// The machine profile to start from (default: your default machine).
    #[arg(long)]
    machine: Option<String>,
    /// The material (default: your default material).
    #[arg(long, conflicts_with = "no_material")]
    material: Option<String>,
    /// No material, even if your library has a default one.
    #[arg(long)]
    no_material: bool,
    /// The board's thickness (default: the material's, else your default thickness).
    #[arg(long, value_parser = crate::lengths::length)]
    thickness: Option<f64>,
    /// The board's size, X×Y, centred on the drawing (e.g. 600x400, 24inx12in).
    #[arg(long, conflicts_with = "fit", value_parser = board_size)]
    size: Option<(f64, f64)>,
    /// The board fitted round the drawing with this margin (the default, 10 mm).
    #[arg(long, value_parser = crate::lengths::length)]
    fit: Option<f64>,
    /// Where Z zero is: the board's top, or the spoilboard.
    #[arg(long)]
    z_zero: Option<ZZeroArg>,
    /// Library tools by name, comma-separated, numbered in this order (default: your
    /// default tool); "" for none.
    #[arg(long)]
    tools: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ZZeroArg {
    Top,
    Spoilboard,
}

/// "600x400" (or `X`, `×`) split in its two halves, trimmed.
pub(crate) fn by(text: &str) -> Option<(&str, &str)> {
    text.split_once(['x', 'X', '×'])
        .map(|(a, b)| (a.trim(), b.trim()))
}

/// "600x400": two lengths.
pub(crate) fn board_size(text: &str) -> Result<(f64, f64), String> {
    let (x, y) = by(text).ok_or_else(|| format!("expected X×Y, such as 600x400, got {text:?}"))?;
    Ok((crate::lengths::length(x)?, crate::lengths::length(y)?))
}

/// "3x2": columns and rows, whole numbers. None when it isn't.
pub(crate) fn grid_counts(text: &str) -> Option<(u32, u32)> {
    let (c, r) = by(text)?;
    Some((c.parse().ok()?, r.parse().ok()?))
}

impl StartArgs {
    /// The pipeline's start for a project whose board is `job`'s until told otherwise.
    pub(crate) fn start(&self, job: &Job) -> chipload_pipeline::library::ProjectStart {
        use chipload_pipeline::library::{Board, ProjectStart};
        let stock = job.stock();
        let board = match (self.size, self.fit) {
            (Some((x, y)), _) => Some(Board { x, y, fit: None }),
            (None, Some(margin)) => Some(Board {
                x: stock.x,
                y: stock.y,
                fit: Some(margin),
            }),
            (None, None) => None,
        };
        ProjectStart {
            machine: self.machine.clone(),
            material: if self.no_material {
                Some(None)
            } else {
                self.material.clone().map(Some)
            },
            thickness: self.thickness,
            board,
            z_zero: self.z_zero.map(|z| match z {
                ZZeroArg::Top => chipload_job::ZZero::Top,
                ZZeroArg::Spoilboard => chipload_job::ZZero::Bottom,
            }),
            tools: self.tools.as_ref().map(|list| {
                list.split(',')
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_string)
                    .collect()
            }),
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
