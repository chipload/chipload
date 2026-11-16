// SPDX-License-Identifier: GPL-3.0-or-later
//! Rest, floor, inlay, roughing and smoothing passes added, changed and removed.

use super::*;
use clap::Args;

#[derive(Args)]
pub(crate) struct AddRestArgs {
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
}

impl AddRestArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let AddRestArgs {
            project,
            op,
            tool,
            output,
        } = self;
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
}

#[derive(Args)]
pub(crate) struct AddFloorArgs {
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
}

impl AddFloorArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let AddFloorArgs {
            project,
            op,
            tool,
            output,
        } = self;
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
}

#[derive(Args)]
pub(crate) struct AddInlayArgs {
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
}

impl AddInlayArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let AddInlayArgs {
            project,
            what,
            tool,
            depth,
            plug_start,
            gap,
            floor_tool,
            plug_thickness,
            output,
        } = self;
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
}

#[derive(Args)]
pub(crate) struct AddRoughingArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The relief's number as the app lists it (1 = first; `get` and `set` paths count
    /// from 0).
    #[arg(long)]
    op: usize,
    /// The roughing tool's id in the project.
    #[arg(long)]
    tool: i64,
    /// What is left on the model for the finish; default 0.5 mm.
    #[arg(long, value_parser = crate::lengths::length)]
    allowance: Option<f64>,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl AddRoughingArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let AddRoughingArgs {
            project,
            op,
            tool,
            allowance,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        // The library's cutting values for the tool, as for a new operation.
        let library = settings_dir()
            .map(|dir| Library::load(&dir))
            .unwrap_or_default();
        let material = chipload_pipeline::materials::project_material(&job, &library.materials)
            .map(|m| m.uid.clone());
        let cutting = job.tools.iter().find(|t| t.id == tool).and_then(|t| {
            chipload_job::library::linked_cutting(&library.tools, t, material.as_deref())
        });
        let allowance = allowance.unwrap_or(chipload_pipeline::DEFAULT_ROUGHING_ALLOWANCE_MM);
        let at = match chipload_pipeline::add_roughing_pass(
            &mut job,
            op,
            tool,
            allowance,
            cutting.as_ref(),
        ) {
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
            "the relief's feeds; check them for the roughing tool"
        };
        out.line(format!(
            "Added roughing pass {at} to operation {op}, tool {tool}, leaving {} mm, with \
             {from}.",
            chipload_core::text::mm(allowance)
        ));
        out.set("operation", json!(at.operation));
        out.set("pass", json!(at.pass));
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct AddSmoothingArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The relief's number as the app lists it (1 = first; `get` and `set` paths count
    /// from 0).
    #[arg(long)]
    op: usize,
    /// The smoothing tool's id in the project, an end mill or a plain ball nose; default
    /// the roughing tool.
    #[arg(long)]
    tool: Option<i64>,
    /// What is left on the model for the finish; default the roughing's.
    #[arg(long, value_parser = crate::lengths::length)]
    allowance: Option<f64>,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl AddSmoothingArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let AddSmoothingArgs {
            project,
            op,
            tool,
            allowance,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        // The library's cutting values for another tool than the roughing's.
        let library = settings_dir()
            .map(|dir| Library::load(&dir))
            .unwrap_or_default();
        let material = chipload_pipeline::materials::project_material(&job, &library.materials)
            .map(|m| m.uid.clone());
        let cutting = tool
            .and_then(|tool| job.tools.iter().find(|t| t.id == tool))
            .and_then(|t| {
                chipload_job::library::linked_cutting(&library.tools, t, material.as_deref())
            });
        let at = match chipload_pipeline::add_smoothing_pass(&mut job, op, tool, cutting.as_ref()) {
            Ok(at) => at,
            Err(err) => return out.fail(err),
        };
        let Some(chipload_job::Operation::Relief(chipload_job::ReliefOp {
            smoothing: Some(pass),
            ..
        })) = job.operations.get_mut(at.operation)
        else {
            return out.fail(format!("operation {op} is not a relief"));
        };
        if let Some(allowance) = allowance {
            pass.allowance = allowance;
        }
        let pass = *pass;
        let target = output.as_deref().unwrap_or(&project);
        if let Err(err) = crate::save_project(out, &job, target) {
            return out.fail(err);
        }
        out.line(format!(
            "Added smoothing pass {at} to operation {op}, tool {}, leaving {} mm, lines at \
             {}°, stepover {} %.",
            pass.tool,
            chipload_core::text::mm(pass.allowance),
            chipload_core::text::mm(pass.angle.to_degrees()),
            chipload_core::text::mm(pass.stepover * 100.0),
        ));
        out.set("operation", json!(at.operation));
        out.set("pass", json!(at.pass));
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct RemoveSmoothingArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The relief's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl RemoveSmoothingArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let RemoveSmoothingArgs {
            project,
            op,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        let Some(chipload_job::Operation::Relief(relief)) =
            op.checked_sub(1).and_then(|i| job.operations.get_mut(i))
        else {
            return out.fail(format!("operation {op} is not a relief"));
        };
        if relief.smoothing.take().is_none() {
            return out.fail(format!("operation {op} has no smoothing pass"));
        }
        let target = output.as_deref().unwrap_or(&project);
        if let Err(err) = crate::save_project(out, &job, target) {
            return out.fail(err);
        }
        out.line(format!("Removed the smoothing pass of operation {op}."));
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct RemoveRoughingArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The relief's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl RemoveRoughingArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let RemoveRoughingArgs {
            project,
            op,
            output,
        } = self;
        let mut job = match load(out, &project) {
            Ok(job) => job,
            Err(code) => return code,
        };
        let Some(chipload_job::Operation::Relief(relief)) =
            op.checked_sub(1).and_then(|i| job.operations.get_mut(i))
        else {
            return out.fail(format!("operation {op} is not a relief"));
        };
        if relief.roughing.take().is_none() {
            return out.fail(format!("operation {op} has no roughing pass"));
        }
        // A smoothing pass goes only after a roughing pass (stl-relief.md §17).
        let smoothed = relief.smoothing.take().is_some();
        let target = output.as_deref().unwrap_or(&project);
        if let Err(err) = crate::save_project(out, &job, target) {
            return out.fail(err);
        }
        if smoothed {
            out.line(format!(
                "Removed the roughing and smoothing passes of operation {op}."
            ));
        } else {
            out.line(format!("Removed the roughing pass of operation {op}."));
        }
        out.wrote(target, "");
        ExitCode::SUCCESS
    }
}

#[derive(Args)]
pub(crate) struct RemoveFloorArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The V-carve's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl RemoveFloorArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let RemoveFloorArgs {
            project,
            op,
            output,
        } = self;
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
}

#[derive(Args)]
pub(crate) struct RestPassArgs {
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
}

impl RestPassArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let RestPassArgs {
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
        } = self;
        rest_pass_command(
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
        )
    }
}

#[derive(Args)]
pub(crate) struct MakeRestPassArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The rest pocket's number as the app lists it (1 = first).
    #[arg(long)]
    op: usize,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl MakeRestPassArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let MakeRestPassArgs {
            project,
            op,
            output,
        } = self;
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
}

#[derive(Args)]
pub(crate) struct RestArgs {
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
}

impl RestArgs {
    pub(crate) fn run(self, out: &mut Out) -> ExitCode {
        let RestArgs {
            project,
            op,
            after_tool,
            none: _,
            output,
        } = self;
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
    write_project(out, &job, target)
}
