// SPDX-License-Identifier: GPL-3.0-or-later
//! `op add` and `op add-surface`: new operations with the app's defaults.

use super::describe;
use super::*;
use chipload_job::{Operation, RampShape};
use clap::Args;

#[derive(Args)]
pub(crate) struct AddArgs {
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
    /// A chamfer's width at the surface, in place of 1 mm; its depth follows from the
    /// V-bit's angle (docs/specs/chamfer.md).
    #[arg(long, value_parser = crate::lengths::length)]
    width: Option<f64>,
    /// A chamfer's waste side, in place of the one the shapes tell: around them
    /// (outside) or within them (inside).
    #[arg(long)]
    side: Option<EdgeSideArg>,
    /// A flute's ramp at both ends, in place of 20 mm (docs/specs/fluting.md); a
    /// pattern's grooves' ramps, in place of square ends (docs/specs/fill-pattern.md).
    #[arg(long, value_parser = crate::lengths::length)]
    ramp: Option<f64>,
    /// A flute's ramp at the shapes' first ends (as drawn); 0 for a square end.
    #[arg(long, value_parser = crate::lengths::length, conflicts_with = "ramp")]
    ramp_start: Option<f64>,
    /// A flute's ramp at the shapes' last ends; 0 for a square end.
    #[arg(long, value_parser = crate::lengths::length, conflicts_with = "ramp")]
    ramp_end: Option<f64>,
    /// A flute's ramps: straight (an even taper) or curved (easing into the floor).
    #[arg(long)]
    ramp_shape: Option<RampShapeArg>,
    #[command(flatten)]
    pattern: PatternArgs,
    /// A prism's deepest edge, for bevelled shapes with a flat top; left out, as deep
    /// as the shapes and the V-bit's cone need (docs/specs/prism.md).
    #[arg(long, value_parser = crate::lengths::length)]
    max_depth: Option<f64>,
    #[command(flatten)]
    moulding: MouldingArgs,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl AddArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let AddArgs {
            project,
            kind,
            what,
            tool,
            depth,
            through,
            width,
            side,
            ramp,
            ramp_start,
            ramp_end,
            ramp_shape,
            pattern,
            max_depth,
            moulding,
            output,
        } = self;
        let moulds = matches!(kind, KindArg::Moulding);
        if !moulds && moulding.given() {
            return out.fail(
                "--profile, --profile-shape, --ends, --taper and --stepover are for a moulding",
            );
        }
        if moulds && through {
            return out.fail("a moulding's depth follows from its profile");
        }
        if moulds
            && (pattern.given_except_radius()
                || ramp.is_some()
                || ramp_start.is_some()
                || ramp_end.is_some())
        {
            return out.fail(
                "a moulding takes --profile with --radius (or --width and --depth for an \
                 ogee), --side, --ends, --taper, --ramp-shape and --stepover",
            );
        }
        let prism = matches!(kind, KindArg::Prism);
        if !prism && max_depth.is_some() {
            return out.fail("--max-depth is for a prism");
        }
        if prism && (depth.is_some() || through) {
            return out.fail(
                "a prism's depth follows from its shapes and the V-bit: give --max-depth \
                 for a flat top",
            );
        }
        let flute = matches!(kind, KindArg::Flute);
        let filled = matches!(kind, KindArg::Pattern);
        let ramps = [ramp_start, ramp_end];
        if !flute && ramps.iter().any(Option::is_some) {
            return out.fail("--ramp-start and --ramp-end are for a flute");
        }
        if !flute && !moulds && ramp_shape.is_some() {
            return out.fail("--ramp-shape is for a flute or a moulding's taper");
        }
        if !flute && !filled && ramp.is_some() {
            return out.fail("--ramp is for a flute or a pattern");
        }
        if !filled && !moulds && pattern.given() {
            return out.fail(
                "--pattern, --spacing, --angle, --between, --wavelength, --height and \
                 --on-line are for a pattern",
            );
        }
        if (flute || filled) && through {
            return out.fail("a flute or a pattern never goes through: give --depth");
        }
        let chamfer = matches!(kind, KindArg::Chamfer);
        if !chamfer && !moulds && (width.is_some() || side.is_some()) {
            return out.fail("--width and --side are for a chamfer or a moulding");
        }
        if chamfer && matches!(side, Some(EdgeSideArg::Left | EdgeSideArg::Right)) {
            return out.fail("a chamfer's side is outside or inside");
        }
        if chamfer && (depth.is_some() || through) {
            return out.fail(
                "a chamfer's depth follows from its width and the V-bit's angle: give \
                 --width",
            );
        }
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
            if let Operation::Prism(f) = &mut new.op {
                f.max_depth = max_depth;
            }
            if let Operation::Moulding(m) = &mut new.op {
                let given = (pattern.radius_given(), width, depth);
                if let Err(why) = moulding.apply(m, given, side, ramp_shape) {
                    return out.fail(why);
                }
            }
            if let Operation::Pattern(f) = &mut new.op {
                // The pattern first: a stroke pattern takes fluted ends unless --ramp
                // says otherwise.
                let tool = job.tool(f.tool).cloned();
                pattern.apply(f, tool.as_ref());
                if let Some(ramp) = ramp {
                    f.ramp = ramp;
                }
            }
            if let Operation::Flute(f) = &mut new.op {
                if let Some(both) = ramp {
                    (f.ramp_start, f.ramp_end) = (both, both);
                }
                f.ramp_start = ramp_start.unwrap_or(f.ramp_start);
                f.ramp_end = ramp_end.unwrap_or(f.ramp_end);
                match ramp_shape {
                    Some(RampShapeArg::Straight) => f.ramp_shape = RampShape::Straight,
                    Some(RampShapeArg::Curved) => f.ramp_shape = RampShape::Curved,
                    None => {}
                }
            }
            if let Operation::Chamfer(c) = &mut new.op {
                if let Some(width) = width {
                    c.width = width;
                }
                match side {
                    Some(EdgeSideArg::Outside) => c.side = chipload_job::EdgeSide::Outside,
                    Some(EdgeSideArg::Inside) => c.side = chipload_job::EdgeSide::Inside,
                    Some(EdgeSideArg::Left | EdgeSideArg::Right) | None => {}
                }
            }
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
                describe(op, &job.tools)
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
}

#[derive(Args)]
pub(crate) struct AddSurfaceArgs {
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
}

impl AddSurfaceArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let AddSurfaceArgs {
            project,
            tool,
            depth,
            output,
        } = self;
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
        let (_, end_depth) = op.depths(&job.tools);
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
}
