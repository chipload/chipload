// SPDX-License-Identifier: GPL-3.0-or-later
//! Tool groups and the rules each tool type has to follow.

use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Entry, FluteOp, HoleMethod, Job, Note, Operation, ProfileOp, Tool, ToolType};

use crate::profile::labelled_note;

/// The enabled operations in tool groups: runs of operations next to each other (disabled
/// ones skipped) that use the same tool (docs/specs/tool-per-operation.md §2). Indexes in
/// `job.operations`.
pub fn tool_groups(job: &Job) -> Vec<(i64, Vec<usize>)> {
    tool_groups_among(job, |_| true)
}

/// [`tool_groups`] of the enabled operations `keep` accepts.
fn tool_groups_among(job: &Job, keep: impl Fn(&Operation) -> bool) -> Vec<(i64, Vec<usize>)> {
    let mut groups: Vec<(i64, Vec<usize>)> = Vec::new();
    for (index, op) in job.operations.iter().enumerate() {
        if !job.cuts(index) || !keep(op) {
            continue;
        }
        match groups.last_mut() {
            Some((tool, ops)) if *tool == op.tool() => ops.push(index),
            _ => groups.push((op.tool(), vec![index])),
        }
    }
    groups
}

/// A note for each tool that comes back after another one: putting its operations next to
/// each other saves a tool change.
pub(super) fn tool_group_notes(job: &Job) -> Vec<Note> {
    // Within a part only: parts are cut one after the other, so a tool coming back in the
    // next part is expected (docs/specs/parts.md §3).
    let mut notes = Vec::new();
    for part in &job.parts {
        let groups = tool_groups_among(job, |op| op.part() == part.id);
        notes.extend(returning_tool_notes(&groups));
    }
    notes
}

/// The notes for tools used in more than one of `groups`.
fn returning_tool_notes(groups: &[(i64, Vec<usize>)]) -> Vec<Note> {
    let mut tools: Vec<i64> = groups.iter().map(|(tool, _)| *tool).collect();
    tools.sort_unstable();
    tools.dedup();
    let mut notes = Vec::new();
    for tool in tools {
        let used: Vec<&Vec<usize>> = groups
            .iter()
            .filter(|(t, _)| *t == tool)
            .map(|(_, ops)| ops)
            .collect();
        if used.len() < 2 {
            continue;
        }
        let numbers: Vec<String> = used
            .iter()
            .flat_map(|ops| ops.iter().map(|i| (i + 1).to_string()))
            .collect();
        notes.push(Note::job(
            Message::new("toolpath-tool-returns")
                .arg("tool", tool)
                .arg("groups", used.len())
                .arg("operations", numbers.join(", ")),
        ));
    }
    notes
}

/// V-carving needs a V-bit with a known angle; a tip that isn't set is taken as pointed and
/// said so (docs/specs/v-carve.md §2, decision 11). The notes carry the label; a problem's
/// text gets it from `labelled`, as for the other tool rules below.
pub(super) fn vcarve_tool_notes(label: &Message, tool: &Tool) -> Result<Vec<Message>, Message> {
    if tool.kind != ToolType::Vbit {
        return Err(Message::new("toolpath-vcarve-vbit")
            .arg("tool", tool.id)
            .arg("type", tool.kind.name_message()));
    }
    if tool.angle.is_none() {
        return Err(Message::new("toolpath-vbit-no-angle").arg("tool", tool.id));
    }
    if tool.tip_diameter.is_none() {
        return Ok(vec![labelled_note(
            label,
            Message::new("toolpath-vbit-no-tip").arg("tool", tool.id),
        )]);
    }
    Ok(Vec::new())
}

/// Pockets need an endmill; a straight plunge also needs it to be centre-cutting. Ok(true):
/// it plunges with a tool of unknown centre-cutting.
pub(super) fn pocket_tool_notes(tool: &Tool, entry: Entry) -> Result<bool, Message> {
    if tool.kind != ToolType::Endmill {
        return Err(Message::new("toolpath-pocket-endmill")
            .arg("tool", tool.id)
            .arg("type", tool.kind.name_message()));
    }
    plunge_unknown(tool, entry)
}

/// A follow path takes an endmill (which must be centre-cutting to plunge), a V-bit or a
/// ball nose (centre-cutting by their shape); a drill can't follow a path
/// (docs/specs/follow-path.md §5, stl-relief.md decision 7).
pub(super) fn path_tool_notes(tool: &Tool, entry: Entry) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Drill => Err(Message::new("toolpath-path-drill").arg("tool", tool.id)),
        ToolType::Vbit | ToolType::Ballnose => Ok(false),
        ToolType::Endmill => plunge_unknown(tool, entry),
    }
}

/// A chamfer takes a V-bit with its angle, which its depth follows from
/// (docs/specs/chamfer.md §4); a V-bit goes down on its point.
pub(super) fn chamfer_tool_rule(tool: &Tool) -> Result<bool, Message> {
    if tool.kind != ToolType::Vbit {
        return Err(Message::new("toolpath-chamfer-vbit")
            .arg("tool", tool.id)
            .arg("type", tool.kind.name_message()));
    }
    if tool.angle.is_none() {
        return Err(Message::new("toolpath-vbit-no-angle").arg("tool", tool.id));
    }
    Ok(false)
}

/// A flute takes a ball nose, a V-bit or an endmill (docs/specs/fluting.md §5); an endmill
/// of unknown centre-cutting plunges at a square end.
pub(super) fn flute_tool_rule(tool: &Tool, op: &FluteOp) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Drill => Err(Message::new("toolpath-path-drill").arg("tool", tool.id)),
        ToolType::Vbit | ToolType::Ballnose => Ok(false),
        ToolType::Endmill if op.ramp_start <= 0.0 || op.ramp_end <= 0.0 => {
            plunge_unknown(tool, Entry::Plunge {})
        }
        ToolType::Endmill => Ok(false),
    }
}

/// A moulding takes a ball nose or an endmill whose shape is known (docs/specs/moulding.md
/// §8); its passes go down straight at their start, so an endmill of unknown
/// centre-cutting gets the plunge note.
pub(super) fn moulding_tool_rule(tool: &Tool) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Vbit | ToolType::Drill => Err(Message::new("toolpath-moulding-tool")
            .arg("tool", tool.id)
            .arg("type", tool.kind.name_message())),
        _ if tool.cutter().is_none() => {
            Err(Message::new("toolpath-relief-shape").arg("tool", tool.id))
        }
        ToolType::Ballnose => Ok(false),
        ToolType::Endmill => plunge_unknown(tool, Entry::Plunge {}),
    }
}

/// A prism takes a V-bit with its angle, which its slopes follow (docs/specs/prism.md §4);
/// a V-bit goes down on its point.
pub(super) fn prism_tool_rule(tool: &Tool) -> Result<bool, Message> {
    if tool.kind != ToolType::Vbit {
        return Err(Message::new("toolpath-prism-vbit")
            .arg("tool", tool.id)
            .arg("type", tool.kind.name_message()));
    }
    if tool.angle.is_none() {
        return Err(Message::new("toolpath-vbit-no-angle").arg("tool", tool.id));
    }
    Ok(false)
}

/// A drill drills and an endmill bores (docs/specs/holes.md §1, §2): the method must match
/// the tool; a V-bit can't cut holes.
pub(super) fn holes_tool_notes(tool: &Tool, method: HoleMethod) -> Result<(), Message> {
    if tool.kind == ToolType::Vbit {
        return Err(Message::new("toolpath-holes-vbit").arg("tool", tool.id));
    }
    let wanted = method.tool_type();
    if tool.kind != wanted {
        let method_name = match method {
            HoleMethod::Drill { .. } => "drill",
            HoleMethod::Bore { .. } => "bore",
        };
        return Err(Message::new("toolpath-holes-method")
            .arg("method", method_name)
            .arg("tool", tool.id)
            .arg("type", tool.kind.with_article_message()));
    }
    Ok(())
}

/// Going down a tab's flank is a ramp (docs/specs/profile-spiral.md §4): flanks steeper
/// than the ramp angle are refused for a tool marked not centre-cutting, and a note when
/// it's unknown.
pub(super) fn flank_notes(
    number: usize,
    tool: &Tool,
    op: &ProfileOp,
) -> Result<Vec<Message>, Message> {
    let Some(tabs) = &op.tabs else {
        return Ok(Vec::new());
    };
    let ramp_angle = op.ramp_angle;
    if tabs.flank_angle <= ramp_angle + ANGLE_TOL_RAD {
        return Ok(Vec::new());
    }
    let (flank, ramp) = (tabs.flank_angle.to_degrees(), ramp_angle.to_degrees());
    match tool.center_cutting {
        Some(false) => Err(Message::new("toolpath-flank-not-centre-cutting")
            .arg("tool", tool.id)
            .arg("flank", Value::Number(flank, Digits::Fixed(1)))
            .arg("ramp", Value::Number(ramp, Digits::Fixed(1)))),
        None => Ok(vec![labelled_note(
            &Message::new("job-operation-label").arg("number", number),
            Message::new("toolpath-flank-unknown-centre-cutting")
                .arg("flank", Value::Number(flank, Digits::Fixed(1)))
                .arg("ramp", Value::Number(ramp, Digits::Fixed(1)))
                .arg("tool", tool.id),
        )]),
        Some(true) => Ok(Vec::new()),
    }
}

/// A tool marked not centre-cutting ramps only along a path at least its diameter long
/// (the review of 2026-11-02): on a shorter one it goes back and forth over its own centre,
/// whose dead zone never moves off the material under it, as in a plunge.
pub(super) fn short_ramp_refusal(tool: &Tool, length: f64) -> Result<(), Message> {
    if tool.center_cutting == Some(false) && length < tool.diameter - CHAIN_TOL_MM {
        return Err(Message::new("toolpath-ramp-too-short")
            .arg("tool", tool.id)
            .arg("length", Value::mm(length))
            .arg("diameter", Value::mm(tool.diameter)));
    }
    Ok(())
}

/// A straight plunge needs a centre-cutting tool (docs/specs/ramp-entry.md §6): refused
/// for a tool marked not centre-cutting; Ok(true) when it's unknown (one note per tool).
/// A ramp needs neither.
pub(super) fn plunge_unknown(tool: &Tool, entry: Entry) -> Result<bool, Message> {
    if entry.ramp_angle().is_some() {
        return Ok(false);
    }
    match tool.center_cutting {
        Some(false) => Err(Message::new("toolpath-plunge-not-centre-cutting").arg("tool", tool.id)),
        None => Ok(true),
        Some(true) => Ok(false),
    }
}
