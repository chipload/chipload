// SPDX-License-Identifier: GPL-3.0-or-later
//! Operations' tools and their extra passes: rest, floor, roughing and smoothing.

use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::library::{Cutting, LibraryTool};
use chipload_job::{
    Entry, Job, Operation, PocketOp, RestRefusal, Tool, ToolType, VcarveOp, find_tool, rest_refusal,
};

use crate::PipelineError;

/// "there is no operation 7; the project has 6".
pub(crate) fn no_operation(number: usize, count: usize) -> PipelineError {
    PipelineError::Job(
        Message::new("pipeline-no-operation")
            .arg("number", number)
            .arg("count", count),
    )
}

/// Set the tool operation `number` (1-based, as listed) cuts with. The tool must be in the
/// project; feeds stay as they are (docs/specs/tool-per-operation.md §7, decision 6).
pub fn set_op_tool(job: &mut Job, number: usize, tool: i64) -> Result<(), PipelineError> {
    if job.tool(tool).is_none() {
        return Err(PipelineError::Job(
            Message::new("pipeline-no-tool-use").arg("tool", tool),
        ));
    }
    let count = job.operations.len();
    let op = number
        .checked_sub(1)
        .and_then(|i| job.operations.get_mut(i))
        .ok_or_else(|| no_operation(number, count))?;
    op.set_tool(tool);
    op.fit_helixes(&job.tools);
    Ok(())
}

/// Make pocket `number` (1-based, as listed) rest machine after tool `after` (a project
/// tool), or a whole pocket again with None (docs/specs/rest-machining.md §6). Whether the
/// tool is larger is checked with the job's other rules, so the problem shows in the GUI too.
pub fn set_op_rest(job: &mut Job, number: usize, after: Option<i64>) -> Result<(), PipelineError> {
    if let Some(tool) = after
        && job.tool(tool).is_none()
    {
        return Err(PipelineError::Job(
            Message::new("pipeline-no-tool-use").arg("tool", tool),
        ));
    }
    let count = job.operations.len();
    let op = number
        .checked_sub(1)
        .and_then(|i| job.operations.get_mut(i))
        .ok_or_else(|| no_operation(number, count))?;
    let Operation::Pocket(pocket) = op else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-pocket").arg("number", number),
        ));
    };
    pocket.rest = after.map(|after_tool| chipload_job::Rest { after_tool });
    Ok(())
}

/// A rest pass for `pocket` with project tool `tool` (docs/specs/rest-passes.md §2): it
/// rests after the last tool of the pocket's chain (its own, or its last pass's). The feeds,
/// step down and stepover are `cutting`'s (the library's values for the tool) when given,
/// else the pocket's; the entry is the pocket's, made to fit the smaller tool. Refused, with
/// the reason, for dogbones and for a tool that isn't a smaller endmill.
pub fn rest_pass_of(
    pocket: &PocketOp,
    tools: &[Tool],
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::ClearingPass, Message> {
    if chipload_job::has_bones(pocket.corners, &pocket.corner_choices) {
        return Err(Message::new("pipeline-rest-dogbones"));
    }
    let before = pocket.rest_passes.last().map_or(pocket.tool, |p| p.tool);
    let no_tool = |id: i64| Message::new("pipeline-no-tool").arg("tool", id);
    let small = find_tool(tools, tool).ok_or_else(|| no_tool(tool))?;
    let large = find_tool(tools, before).ok_or_else(|| no_tool(before))?;
    match rest_refusal(small, large) {
        Some(RestRefusal::SmallNotEndmill) => {
            return Err(Message::new("pipeline-rest-not-endmill").arg("tool", tool));
        }
        Some(RestRefusal::LargeNotEndmill) => {
            return Err(Message::new("pipeline-rest-before-not-endmill").arg("tool", large.id));
        }
        Some(RestRefusal::NotLarger) => {
            return Err(Message::new("pipeline-rest-not-smaller")
                .arg("tool", tool)
                .arg("diameter", Value::Length(small.diameter, Digits::Shortest))
                .arg("before", large.id)
                .arg(
                    "before_diameter",
                    Value::Length(large.diameter, Digits::Shortest),
                ));
        }
        None => {}
    }
    let entry = match pocket.entry {
        Entry::Helix { .. } => Entry::default_helix(small.diameter),
        Entry::Plunge {} if small.center_cutting == Some(false) => {
            Entry::default_helix(small.diameter)
        }
        other => other,
    };
    let (feed, plunge_feed, step_down, stepover) = match cutting {
        Some(c) => (c.feed, c.plunge_feed, c.step_down, c.stepover),
        None => (
            pocket.feed,
            pocket.plunge_feed,
            pocket.step_down,
            pocket.stepover,
        ),
    };
    let (slot_feed, slot_step_down) = match cutting {
        Some(c) => (Some(c.slot_feed), Some(c.slot_step_down)),
        None => (pocket.slot_feed, pocket.slot_step_down),
    };
    Ok(chipload_job::ClearingPass {
        tool,
        feed,
        plunge_feed,
        step_down,
        stepover,
        slot_feed,
        slot_step_down,
        entry,
        enabled: true,
    })
}

/// Add a rest pass to pocket `number` (1-based) with tool `tool` (docs/specs/rest-passes.md
/// §6). Returns which pass it is.
pub fn add_rest_pass(
    job: &mut Job,
    number: usize,
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::OpRef, PipelineError> {
    let count = job.operations.len();
    let index = number
        .checked_sub(1)
        .filter(|&i| i < count)
        .ok_or_else(|| no_operation(number, count))?;
    let tools = job.tools.clone();
    let Operation::Pocket(pocket) = &mut job.operations[index] else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-pocket").arg("number", number),
        ));
    };
    let pass = rest_pass_of(pocket, &tools, tool, cutting).map_err(|e| {
        PipelineError::Job(
            Message::new("pipeline-operation-refused")
                .arg("number", number)
                .arg("reason", e),
        )
    })?;
    pocket.rest_passes.push(pass);
    Ok(chipload_job::OpRef::pass(
        index,
        pocket.rest_passes.len() - 1,
    ))
}

/// A floor pass for `vcarve` with project tool `tool` (docs/specs/vcarve-wide-strokes.md
/// §4). The feeds, step down and stepover are `cutting`'s (the library's values for the
/// tool) when given, else the V-carve's feeds, its depth in as many equal steps as it has
/// passes, and 0.45;
/// it plunges, or goes down in a helix when the end mill isn't centre-cutting. Refused, with
/// the reason, for a tool that isn't an end mill.
pub fn floor_pass_of(
    vcarve: &VcarveOp,
    tools: &[Tool],
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::ClearingPass, Message> {
    let mill =
        find_tool(tools, tool).ok_or_else(|| Message::new("pipeline-no-tool").arg("tool", tool))?;
    if mill.kind != ToolType::Endmill {
        return Err(Message::new("pipeline-floor-not-endmill").arg("tool", tool));
    }
    let entry = if mill.center_cutting == Some(false) {
        Entry::default_helix(mill.diameter)
    } else {
        Entry::Plunge {}
    };
    let (feed, plunge_feed, step_down, stepover) = match cutting {
        Some(c) => (c.feed, c.plunge_feed, c.step_down, c.stepover),
        None => (
            vcarve.feed,
            vcarve.plunge_feed,
            // As many passes as the V-carve, equally deep.
            (vcarve.end_depth - vcarve.start_depth) / f64::from(vcarve.passes.max(1)),
            FLOOR_STEPOVER,
        ),
    };
    let (slot_feed, slot_step_down) = match cutting {
        Some(c) => (Some(c.slot_feed), Some(c.slot_step_down)),
        None => (None, None),
    };
    Ok(chipload_job::ClearingPass {
        tool,
        feed,
        plunge_feed,
        step_down,
        stepover,
        slot_feed,
        slot_step_down,
        entry,
        enabled: true,
    })
}

/// A floor pass's stepover when the library has no values for its tool.
const FLOOR_STEPOVER: f64 = 0.45;

/// Give V-carve `number` (1-based) a floor pass with tool `tool`, in place of any it had
/// (docs/specs/vcarve-wide-strokes.md §7). Returns the pass.
pub fn add_floor_pass(
    job: &mut Job,
    number: usize,
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::OpRef, PipelineError> {
    let count = job.operations.len();
    let index = number
        .checked_sub(1)
        .filter(|&i| i < count)
        .ok_or_else(|| no_operation(number, count))?;
    let tools = job.tools.clone();
    let Operation::Vcarve(vcarve) = &mut job.operations[index] else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-vcarve").arg("number", number),
        ));
    };
    let pass = floor_pass_of(vcarve, &tools, tool, cutting).map_err(|e| {
        PipelineError::Job(
            Message::new("pipeline-operation-refused")
                .arg("number", number)
                .arg("reason", e),
        )
    })?;
    vcarve.floor = Some(pass);
    Ok(chipload_job::OpRef::pass(index, 0))
}

/// What a relief's roughing leaves on the model when not told, mm
/// (docs/specs/stl-relief.md §14 step 3).
pub const DEFAULT_ROUGHING_ALLOWANCE_MM: f64 = 0.5;
/// A roughing pass's stepover, and its step down as a share of the tool, when the library
/// has no values for its tool.
const ROUGHING_STEPOVER: f64 = 0.4;
const ROUGHING_STEP_DOWN_SHARE: f64 = 0.5;

/// A roughing pass for `relief` with project tool `tool` (docs/specs/stl-relief.md §14 step
/// 3), leaving `allowance`. The feeds, step down and stepover are `cutting`'s (the library's
/// values for the tool) when given, the stepover at most the relief's largest; else the
/// relief's feeds, half the tool's diameter and 0.4. Refused, with the reason, for a tool
/// that isn't an end mill or a plain ball nose.
pub fn roughing_of(
    relief: &chipload_job::ReliefOp,
    tools: &[Tool],
    tool: i64,
    allowance: f64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::ReliefRoughing, Message> {
    let mill =
        find_tool(tools, tool).ok_or_else(|| Message::new("pipeline-no-tool").arg("tool", tool))?;
    let fits = matches!(mill.kind, ToolType::Endmill | ToolType::Ballnose)
        && mill.cutter().and_then(|c| c.grown(allowance)).is_some();
    if !fits {
        return Err(Message::new("toolpath-relief-roughing-tool").arg("tool", tool));
    }
    let (feed, plunge_feed, step_down, stepover) = match cutting {
        Some(c) => (
            c.feed,
            c.plunge_feed,
            c.step_down,
            c.stepover.min(chipload_job::RELIEF_MAX_STEPOVER),
        ),
        None => (
            relief.feed,
            relief.plunge_feed,
            mill.diameter * ROUGHING_STEP_DOWN_SHARE,
            ROUGHING_STEPOVER,
        ),
    };
    Ok(chipload_job::ReliefRoughing {
        tool,
        feed,
        plunge_feed,
        step_down,
        stepover,
        allowance,
        enabled: true,
    })
}

/// The ridges a smoothing pass's ball nose leaves on flat ground, mm: the finish takes them
/// (docs/specs/stl-relief.md §17).
pub const SMOOTHING_RIDGE_MM: f64 = 0.1;

/// A smoothing pass's stepover with an end mill, as a fraction of its diameter (§17).
pub const SMOOTHING_END_MILL_STEPOVER: f64 = 0.15;

/// A smoothing pass for `relief` (docs/specs/stl-relief.md §17), after its roughing pass:
/// with project tool `tool`, by default the roughing's. Its allowance is the roughing's;
/// its lines go across the relief's; its stepover is 15 % with an end mill, or for a ball
/// nose the widest leaving [`SMOOTHING_RIDGE_MM`]. Its feeds are the roughing's with the
/// roughing tool, else `cutting`'s (the library's for the tool), else the relief's. Refused,
/// with the reason, without a roughing pass or for a tool that isn't an end mill or a plain
/// ball nose.
pub fn smoothing_of(
    relief: &chipload_job::ReliefOp,
    tools: &[Tool],
    tool: Option<i64>,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::ReliefSmoothing, Message> {
    let rough = relief
        .roughing
        .ok_or_else(|| Message::new("job-relief-smoothing-alone"))?;
    let tool = tool.unwrap_or(rough.tool);
    let mill =
        find_tool(tools, tool).ok_or_else(|| Message::new("pipeline-no-tool").arg("tool", tool))?;
    let fits = matches!(mill.kind, ToolType::Endmill | ToolType::Ballnose)
        && mill
            .cutter()
            .and_then(|c| c.grown(rough.allowance))
            .is_some();
    if !fits {
        return Err(Message::new("toolpath-relief-roughing-tool").arg("tool", tool));
    }
    let (feed, plunge_feed) = match cutting {
        _ if tool == rough.tool => (rough.feed, rough.plunge_feed),
        Some(c) => (c.feed, c.plunge_feed),
        None => (relief.feed, relief.plunge_feed),
    };
    let stepover = crate::new_ops::ridge_stepover(Some(mill), SMOOTHING_RIDGE_MM)
        .unwrap_or(SMOOTHING_END_MILL_STEPOVER);
    Ok(chipload_job::ReliefSmoothing {
        tool,
        feed,
        plunge_feed,
        stepover,
        angle: across(relief.angle),
        allowance: rough.allowance,
        enabled: true,
    })
}

/// The direction across `angle`, radians: a quarter turn on, kept below a half turn, as
/// raster lines go both ways.
fn across(angle: f64) -> f64 {
    (angle + std::f64::consts::FRAC_PI_2).rem_euclid(std::f64::consts::PI)
}

/// Give relief `number` (1-based) a smoothing pass with tool `tool` (None: its roughing
/// tool), in place of any it had (docs/specs/stl-relief.md §17). Returns the pass.
pub fn add_smoothing_pass(
    job: &mut Job,
    number: usize,
    tool: Option<i64>,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::OpRef, PipelineError> {
    let count = job.operations.len();
    let index = number
        .checked_sub(1)
        .filter(|&i| i < count)
        .ok_or_else(|| no_operation(number, count))?;
    let tools = job.tools.clone();
    let Operation::Relief(relief) = &mut job.operations[index] else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-relief").arg("number", number),
        ));
    };
    let pass = smoothing_of(relief, &tools, tool, cutting).map_err(|e| {
        PipelineError::Job(
            Message::new("pipeline-operation-refused")
                .arg("number", number)
                .arg("reason", e),
        )
    })?;
    relief.smoothing = Some(pass);
    Ok(chipload_job::OpRef::pass(index, 1))
}

/// Give relief `number` (1-based) a roughing pass with tool `tool`, in place of any it had
/// (docs/specs/stl-relief.md §14 step 3). Returns the pass.
pub fn add_roughing_pass(
    job: &mut Job,
    number: usize,
    tool: i64,
    allowance: f64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::OpRef, PipelineError> {
    let count = job.operations.len();
    let index = number
        .checked_sub(1)
        .filter(|&i| i < count)
        .ok_or_else(|| no_operation(number, count))?;
    let tools = job.tools.clone();
    let Operation::Relief(relief) = &mut job.operations[index] else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-relief").arg("number", number),
        ));
    };
    let pass = roughing_of(relief, &tools, tool, allowance, cutting).map_err(|e| {
        PipelineError::Job(
            Message::new("pipeline-operation-refused")
                .arg("number", number)
                .arg("reason", e),
        )
    })?;
    relief.roughing = Some(pass);
    Ok(chipload_job::OpRef::pass(index, 0))
}

/// The depths a V-carve's passes go to, mm down from the stock top, the last its end depth:
/// equal loads (docs/specs/v-carve.md decision 12) or an ordinary step down
/// (vcarve-step-down.md). For the window to show under its *Passes* or *Step down*.
pub fn vcarve_pass_depths(vcarve: &chipload_job::VcarveOp) -> Vec<f64> {
    chipload_toolpath::vcarve::carve_levels(
        vcarve.end_depth - vcarve.start_depth,
        vcarve.passes,
        vcarve.step_down,
    )
    .into_iter()
    .map(|level| vcarve.start_depth + level)
    .collect()
}

/// The library's end mills no wider than `largest` (a floor hint, mm), the nearest to it
/// first (docs/specs/vcarve-wide-strokes.md §4.1, decision 8).
pub fn fitting_end_mills(library: &[LibraryTool], largest: f64) -> Vec<&LibraryTool> {
    let mut fitting: Vec<&LibraryTool> = library
        .iter()
        .filter(|t| t.kind == ToolType::Endmill && t.diameter <= largest + DEPTH_TOL_MM)
        .collect();
    // Stable: equal diameters keep the library's order.
    fitting.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
    fitting
}

/// A standalone rest pocket that copies an earlier pocket of the same part: the same shapes,
/// depths, clearance and direction, resting after that pocket's tool (or its last pass's).
/// Its index, for *Make it a rest pass* (docs/specs/rest-passes.md §5).
pub fn rest_pocket_parent(job: &Job, index: usize) -> Option<usize> {
    let Some(Operation::Pocket(rest)) = job.operations.get(index) else {
        return None;
    };
    let after = rest.rest?.after_tool;
    (0..index).rev().find(|&i| {
        let Operation::Pocket(p) = &job.operations[i] else {
            return false;
        };
        let last = p.rest_passes.last().map_or(p.tool, |r| r.tool);
        p.rest.is_none()
            && p.part == rest.part
            && p.select == rest.select
            && p.start_depth == rest.start_depth
            && p.end_depth == rest.end_depth
            && p.clearance == rest.clearance
            && p.direction == rest.direction
            && !chipload_job::has_bones(p.corners, &p.corner_choices)
            && last == after
    })
}

/// *Make it a rest pass*: the rest pocket at `index` becomes the last rest pass of the pocket
/// it copies, keeping its tool, feeds, step down, stepover, entry and on/off. Returns the
/// pass. None when it copies no earlier pocket.
pub fn make_rest_pass(job: &mut Job, index: usize) -> Option<chipload_job::OpRef> {
    let parent = rest_pocket_parent(job, index)?;
    let Operation::Pocket(rest) = job.operations.remove(index) else {
        return None;
    };
    let Operation::Pocket(pocket) = job.operations.get_mut(parent)? else {
        return None;
    };
    pocket.rest_passes.push(chipload_job::ClearingPass {
        tool: rest.tool,
        feed: rest.feed,
        plunge_feed: rest.plunge_feed,
        step_down: rest.step_down,
        stepover: rest.stepover,
        slot_feed: rest.slot_feed,
        slot_step_down: rest.slot_step_down,
        entry: rest.entry,
        enabled: rest.enabled,
    });
    Some(chipload_job::OpRef::pass(
        parent,
        pocket.rest_passes.len() - 1,
    ))
}
