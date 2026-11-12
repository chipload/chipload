// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload op …`: operations added, changed, moved between parts, and their rest and floor
//! passes.

use super::*;
use chipload_job::Operation;
use clap::Args;

#[derive(Subcommand)]
pub(crate) enum OpAction {
    /// Add an operation for shapes of the drawing, with the defaults the app gives
    /// (docs/specs/styles.md §2): a profile of outlines goes outside and through with tabs,
    /// of holes inside and through; of both, two profiles, the holes first. Feeds and step
    /// downs come from the tool's cutting data. Added at the end of the active part.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// What to add.
        kind: KindArg,
        #[command(flatten)]
        what: Shapes,
        /// The tool's id in the project; default: the one the app would take.
        #[arg(long)]
        tool: Option<i64>,
        /// End depth instead of the default (not with --through).
        #[arg(long, value_parser = crate::lengths::length, conflicts_with = "through")]
        depth: Option<f64>,
        /// Cut through the stock, below it by the setup's max overcut.
        #[arg(long)]
        through: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
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
    /// Give a V-carve a floor pass (docs/specs/vcarve-wide-strokes.md §4): an end mill that
    /// clears the floor between the walls of strokes too wide to carve from their centre
    /// line, cut before the V-carve. Replaces the floor pass it had.
    AddFloor {
        /// The project (.chipload).
        project: PathBuf,
        /// The V-carve's number as the app lists it (1 = first; `get` and `set` paths count
        /// from 0).
        #[arg(long)]
        op: usize,
        /// The end mill's id in the project.
        #[arg(long)]
        tool: i64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Make a V-carve inlay of closed shapes (docs/specs/vcarve-inlay.md): a pocket
    /// V-carved in the active part, and in a new setup, the plug board, the plug: the rest
    /// of that board carved, mirrored left to right, so the shapes stand as a tapered plug
    /// that is glued face down into the pocket.
    AddInlay {
        /// The project (.chipload).
        project: PathBuf,
        #[command(flatten)]
        what: Shapes,
        /// The V-bit's id in the project; both are carved with it.
        #[arg(long)]
        tool: i64,
        /// The pocket's depth; default 5 mm.
        #[arg(long, value_parser = crate::lengths::length)]
        depth: Option<f64>,
        /// How deep below the plug board's surface its outline is taken: the plug's face
        /// lies that deep in the pocket, the rest of the pocket's depth left for glue;
        /// default 3 mm.
        #[arg(long, value_parser = crate::lengths::length)]
        plug_start: Option<f64>,
        /// The glue gap: how much farther the plug board is carved, so its back stands
        /// above the base; default 1.5 mm.
        #[arg(long, value_parser = crate::lengths::length)]
        gap: Option<f64>,
        /// An end mill's id for the floor passes of both (the plug's channel is mostly
        /// floor).
        #[arg(long)]
        floor_tool: Option<i64>,
        /// The plug board's thickness; default 10 mm.
        #[arg(long, value_parser = crate::lengths::length)]
        plug_thickness: Option<f64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Take a V-carve's floor pass away (docs/specs/vcarve-wide-strokes.md §4).
    RemoveFloor {
        /// The project (.chipload).
        project: PathBuf,
        /// The V-carve's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
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
    /// With --along or --around, first choose where contours without tabs placed by hand
    /// get theirs (docs/specs/tabs-along-grain.md).
    Tabs {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation's number as the app lists it (1 = first).
        #[arg(long)]
        op: usize,
        /// On the edges along this axis: x (bottom and top) or y (left and right), half on
        /// each.
        #[arg(long, conflicts_with = "around")]
        along: Option<AxisArg>,
        /// Evenly round each contour (as without --along).
        #[arg(long)]
        around: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
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
        /// Place the tabs of the contour nearest to --at again, as the operation places them
        /// (evenly, or along an axis: `op tabs --along`).
        #[arg(long, conflicts_with = "reset")]
        even: bool,
        /// Place every contour's tabs again, as the operation places them.
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

/// What to cut, for `op add`, `op add-inlay` and `style use`: a layer, shapes, a DXF, a
/// group, a text, a picture or a 3D model's outline.
#[derive(Args)]
pub(crate) struct Shapes {
    /// Every shape on this layer (with --dxf: on that DXF's layer).
    #[arg(long)]
    layer: Option<String>,
    /// These shapes, by id (as `inspect` lists them).
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    shapes: Option<Vec<u64>>,
    /// All of this DXF or SVG (an id from `dxf list`).
    #[arg(long, conflicts_with_all = ["group", "text", "image"])]
    dxf: Option<u64>,
    /// All of this group.
    #[arg(long, conflicts_with_all = ["text", "image"])]
    group: Option<u64>,
    /// All of this text (an id from `text list`).
    #[arg(long, conflicts_with = "image")]
    text: Option<u64>,
    /// All of this traced picture (an id from `image list`).
    #[arg(long, conflicts_with = "model")]
    image: Option<u64>,
    /// The outline of this 3D model (an id from `model list`): a relief of it carves within
    /// it (docs/specs/stl-relief.md §4).
    #[arg(long)]
    model: Option<u64>,
}

impl Shapes {
    /// The selector; refused when it names nothing.
    pub(crate) fn selector(self) -> Result<chipload_job::Selector, &'static str> {
        let source = self
            .dxf
            .map(chipload_job::Source::Dxf)
            .or(self.group.map(chipload_job::Source::Group))
            .or(self.text.map(chipload_job::Source::Text))
            .or(self.image.map(chipload_job::Source::Image))
            .or(self.model.map(chipload_job::Source::Model));
        let select = chipload_job::Selector {
            layer: self.layer,
            shapes: self.shapes,
            source,
            exclude: Vec::new(),
        };
        if select.is_empty() {
            return Err(
                "say what to cut: --layer, --shapes, --dxf, --group, --text, --image or --model",
            );
        }
        Ok(select)
    }
}

/// An axis of the drawing, for `op tabs --along`.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum AxisArg {
    X,
    Y,
}

/// The kinds `op add` adds (a surface has `op add-surface`).
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum KindArg {
    Profile,
    Pocket,
    Vcarve,
    Path,
    Holes,
    /// A 3D model carved from the top (docs/specs/stl-relief.md): with --model, of that
    /// model within its outline.
    Relief,
}

impl KindArg {
    fn kind(self) -> chipload_pipeline::new_ops::OpKind {
        use chipload_pipeline::new_ops::OpKind;
        match self {
            KindArg::Profile => OpKind::Profile,
            KindArg::Pocket => OpKind::Pocket,
            KindArg::Vcarve => OpKind::Vcarve,
            KindArg::Path => OpKind::Path,
            KindArg::Holes => OpKind::Holes,
            KindArg::Relief => OpKind::Relief,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum BoneArg {
    None,
    Dogbone,
    Tbone,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum WallArg {
    Longer,
    Shorter,
}

pub(crate) fn op(out: &mut Out, action: OpAction) -> ExitCode {
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
            // An operation with a material takes its values for the new tool
            // (docs/specs/materials.md §2).
            let followed = materials::follow_op_material(&mut job, op - 1);
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
                return out.fail(err);
            }
            out.line(format!("Operation {op} now cuts with tool {tool}."));
            if let Some(name) = followed {
                out.line(format!("Its feeds are {name}'s values for tool {tool}."));
            }
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
            if let Err(err) = crate::save_project(out, &job, target) {
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
        OpAction::Add {
            project,
            kind,
            what,
            tool,
            depth,
            through,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let select = match what.selector() {
                Ok(select) => select,
                Err(why) => return out.fail(why),
            };
            if let Some(id) = tool
                && job.tool(id).is_none()
            {
                return out.fail(format!(
                    "the project has no tool {id}: copy one in from your library with `tool use`"
                ));
            }
            let drawing = match chipload_pipeline::load_drawing(&job) {
                Ok(drawing) => drawing,
                Err(err) => return out.fail(err),
            };
            let library = settings_dir()
                .map(|dir| Library::load(&dir))
                .unwrap_or_default();
            let request = chipload_pipeline::new_ops::NewRequest {
                kind: kind.kind(),
                select,
                tool,
            };
            let mut made = chipload_pipeline::new_ops::new_operations(
                &job,
                &request,
                &drawing.contours,
                Some(&library.tools),
                chipload_i18n::Language::En,
            );
            for new in &mut made.ops {
                set_depth(&job, &mut new.op, depth, through);
            }
            let notes: Vec<String> = made.messages.iter().map(|m| m.english()).collect();
            let count = made.ops.len();
            let unsided = made.ops.iter().any(|n| !n.side_chosen);
            let first = chipload_pipeline::new_ops::insert(&mut job, made);
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
                return out.fail(err);
            }
            for (offset, op) in job.operations[first..first + count].iter().enumerate() {
                out.line(format!(
                    "Added operation {}: {}.",
                    first + offset + 1,
                    describe(op)
                ));
            }
            if unsided {
                out.line(
                    "The selection doesn't tell the profile's side: it cuts outside; set \
                     `side` if that is wrong."
                        .to_string(),
                );
            }
            for note in notes {
                out.line(format!("note: {note}"));
            }
            out.set(
                "operations",
                json!((first..first + count).collect::<Vec<usize>>()),
            );
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
            // The project's material picks the tool's row (docs/specs/cutting-data.md §2).
            let material = chipload_pipeline::materials::project_material(&job, &library.materials)
                .map(|m| m.uid.clone());
            let cutting = job
                .tools
                .iter()
                .find(|t| Some(t.id) == chosen)
                .and_then(|t| {
                    chipload_job::library::linked_cutting(&library.tools, t, material.as_deref())
                });
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
            if let Err(err) = crate::save_project(out, &job, target) {
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
            // The project's material picks the tool's row (docs/specs/cutting-data.md §2).
            let material = chipload_pipeline::materials::project_material(&job, &library.materials)
                .map(|m| m.uid.clone());
            let cutting = job.tools.iter().find(|t| t.id == tool).and_then(|t| {
                chipload_job::library::linked_cutting(&library.tools, t, material.as_deref())
            });
            let at = match add_rest_pass(&mut job, op, tool, cutting.as_ref()) {
                Ok(at) => at,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
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
        OpAction::AddFloor {
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
            // The project's material picks the tool's row (docs/specs/cutting-data.md §2).
            let material = chipload_pipeline::materials::project_material(&job, &library.materials)
                .map(|m| m.uid.clone());
            let cutting = job.tools.iter().find(|t| t.id == tool).and_then(|t| {
                chipload_job::library::linked_cutting(&library.tools, t, material.as_deref())
            });
            let at = match add_floor_pass(&mut job, op, tool, cutting.as_ref()) {
                Ok(at) => at,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
                return out.fail(err);
            }
            let from = if cutting.is_some() {
                "the library's cutting values for the tool"
            } else {
                "the V-carve's feeds; check them for the end mill"
            };
            out.line(format!(
                "Added floor pass {at} to operation {op}, tool {tool}, with {from}."
            ));
            out.set("operation", json!(at.operation));
            out.set("pass", json!(at.pass));
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        OpAction::AddInlay {
            project,
            what,
            tool,
            depth,
            plug_start,
            gap,
            floor_tool,
            plug_thickness,
            output,
        } => {
            use chipload_pipeline::inlay;
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let select = match what.selector() {
                Ok(select) => select,
                Err(why) => return out.fail(why),
            };
            let drawing = match chipload_pipeline::load_drawing(&job) {
                Ok(drawing) => drawing,
                Err(err) => return out.fail(err),
            };
            let library = settings_dir()
                .map(|dir| Library::load(&dir))
                .unwrap_or_default();
            let request = inlay::InlayRequest {
                select,
                tool,
                depth: depth.unwrap_or(inlay::DEFAULT_POCKET_DEPTH_MM),
                plug_start: plug_start.unwrap_or(inlay::DEFAULT_PLUG_START_MM),
                gap: gap.unwrap_or(inlay::DEFAULT_GLUE_GAP_MM),
                floor_tool,
                plug_thickness: plug_thickness.unwrap_or(inlay::DEFAULT_PLUG_THICKNESS_MM),
            };
            let (job, made) = match inlay::add_inlay(
                &job,
                &request,
                &drawing.contours,
                Some(&library.tools),
                chipload_i18n::Language::En,
            ) {
                Ok(done) => done,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
                return out.fail(err);
            }
            let mm = chipload_core::text::mm;
            out.line(format!(
                "Added operation {}: the inlay's pocket, {} mm deep.",
                made.pocket + 1,
                mm(request.depth)
            ));
            out.line(format!(
                "Added setup {} for the plug board ({} mm thick, fitted round the artwork), \
                 with part {} mirrored left to right and operation {}: the plug, its outline \
                 {} mm down, carved {} mm deeper for the glue.",
                made.setup,
                mm(request.plug_thickness),
                made.part,
                made.plug + 1,
                mm(request.plug_start),
                mm(request.gap)
            ));
            out.set("pocket", json!(made.pocket));
            out.set("plug", json!(made.plug));
            out.set("setup", json!(made.setup));
            out.set("part", json!(made.part));
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
        OpAction::RemoveFloor {
            project,
            op,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(chipload_job::Operation::Vcarve(vcarve)) =
                op.checked_sub(1).and_then(|i| job.operations.get_mut(i))
            else {
                return out.fail(format!("operation {op} is not a V-carve"));
            };
            if vcarve.floor.take().is_none() {
                return out.fail(format!("operation {op} has no floor pass"));
            }
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &job, target) {
                return out.fail(err);
            }
            out.line(format!("Removed the floor pass of operation {op}."));
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
            if let Err(err) = crate::save_project(out, &job, target) {
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
        OpAction::Tabs {
            project,
            op,
            along,
            around,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(index) = op.checked_sub(1) else {
                return out.fail("operations are numbered from 1");
            };
            if along.is_some() || around {
                let Some(Operation::Profile(chipload_job::ProfileOp {
                    tabs: Some(tabs), ..
                })) = job.operations.get_mut(index)
                else {
                    return out.fail(format!("operation {op} is not a profile with tabs"));
                };
                tabs.along = along.map(|a| match a {
                    AxisArg::X => chipload_job::Axis::X,
                    AxisArg::Y => chipload_job::Axis::Y,
                });
                let target = output.as_deref().unwrap_or(&project);
                if let Err(err) = crate::save_project(out, &job, target) {
                    return out.fail(err);
                }
                out.line(match along {
                    Some(AxisArg::X) => "Tabs on the edges along X (bottom and top).",
                    Some(AxisArg::Y) => "Tabs on the edges along Y (left and right).",
                    None => "Tabs evenly round each contour.",
                });
                out.wrote(target, "");
            }
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
            if let Err(err) = crate::save_project(out, &job, target) {
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
            if let Err(err) = crate::save_project(out, &job, target) {
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
            if let Err(err) = crate::save_project(out, &job, target) {
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
pub(crate) fn gets_name(gets: Option<chipload_toolpath::corners::Gets>) -> &'static str {
    use chipload_job::Wall;
    use chipload_toolpath::corners::Gets;
    match gets {
        None => "round",
        Some(Gets::Dogbone) => "dogbone",
        Some(Gets::Tbone(Wall::Longer)) => "T-bone along the longer wall",
        Some(Gets::Tbone(Wall::Shorter)) => "T-bone along the shorter wall",
    }
}

/// What `op rest-pass` changes in one pass.
pub(crate) struct RestPassChange {
    tool: Option<i64>,
    feed: Option<f64>,
    plunge_feed: Option<f64>,
    step_down: Option<f64>,
    stepover: Option<f64>,
    enabled: Option<bool>,
    remove: bool,
}

/// `op rest-pass`: list a pocket's passes, or change or remove one.
pub(crate) fn rest_pass_command(
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
    write_project(out, &job, target)
}

/// The operations among `refs`, 0-based, as `--json` always listed them.
pub(crate) fn operations_json(refs: &[chipload_job::OpRef]) -> Value {
    json!(
        refs.iter()
            .filter(|r| r.pass.is_none())
            .map(|r| r.operation)
            .collect::<Vec<_>>()
    )
}

/// The rest passes among `refs` (docs/specs/rest-passes.md): `{operation, pass}`, 0-based.
pub(crate) fn rest_passes_json(refs: &[chipload_job::OpRef]) -> Value {
    json!(
        refs.iter()
            .filter_map(|r| r.pass.map(|k| json!({"operation": r.operation, "pass": k})))
            .collect::<Vec<_>>()
    )
}

/// `op add`'s --depth and --through over the defaults.
fn set_depth(job: &Job, op: &mut Operation, depth: Option<f64>, through: bool) {
    // A relief's depth is its model's (docs/specs/stl-relief.md §3): `model set --depth`.
    if let Operation::Relief(_) = op {
        return;
    }
    if through {
        if let Some(t) = op.through_mut() {
            *t = Some(job.stock().max_overcut);
        }
    } else if let Some(depth) = depth {
        if let Some(t) = op.through_mut() {
            *t = None;
        }
        op.set_end_depth(depth);
    }
    let point = job.drill_point(op);
    op.follow_thickness(job.stock().thickness, point);
}

/// What `op add` added, in a few words: "\"Profile 1\", outside, through to 12.300 mm, with
/// tabs, tool 1".
pub(crate) fn describe(op: &Operation) -> String {
    let mut parts = vec![format!("\"{}\"", op.name())];
    if let Operation::Profile(p) = op {
        parts.push(
            match p.side {
                chipload_job::Side::Inside => "inside",
                chipload_job::Side::Outside => "outside",
                chipload_job::Side::On => "on the line",
            }
            .to_string(),
        );
    }
    let (_, end) = op.depths();
    let depth = chipload_core::text::mm(end);
    parts.push(match op.through() {
        Some(_) => format!("through to {depth} mm"),
        None => format!("{depth} mm deep"),
    });
    if let Operation::Profile(p) = op
        && let Some(tabs) = &p.tabs
    {
        parts.push(format!("{} tabs per contour", tabs.count));
    }
    parts.push(format!("tool {}", op.tool()));
    parts.join(", ")
}
