// SPDX-License-Identifier: GPL-3.0-or-later
//! Generation-time rules: every operation against its own tool and the stock.

use crate::ToolpathError;
use crate::profile::labelled_note;
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    Entry, FillPattern, Job, MAX_LEAD_DIAMETERS, Note, Operation, PocketOp, Problem, ProfileOp,
    Tool, ToolType, VcarveOp, ZZero,
};

use super::check_passes::{
    floor_pass_checks, plug_checks, rest_check, rest_pass_checks, surface_order_note,
    surface_tool_rule,
};
use super::check_tools::{
    chamfer_tool_rule, flank_notes, flute_tool_rule, holes_tool_notes, moulding_tool_rule,
    path_tool_notes, plunge_unknown, pocket_tool_notes, prism_tool_rule, tool_group_notes,
    vcarve_tool_notes,
};
use super::job_error;
use super::op_relief::relief_pass_checks;
use super::op_vcarve::vbit_cone_depth;

/// Generation-time rules from the specs, each operation against its own tool. Returns
/// warnings.
pub fn validate_job(job: &Job) -> Result<Vec<Note>, ToolpathError> {
    let (problems, notes) = check_job(job);
    match problems.into_iter().next() {
        Some(first) => Err(match first.operation() {
            Some(index) => job_error(first.text_message).at(index),
            None => job_error(first.text_message),
        }),
        None => Ok(notes),
    }
}

/// Every rule checked before planning that the job breaks, each with the field it is
/// about (docs/specs/live-problems.md); [`validate_job`] refuses the job for the first.
pub fn job_problems(job: &Job) -> Vec<Problem> {
    check_job(job).0
}

/// A problem of an operation: the field shows `short`, the full text starts with the
/// operation's `label` and ": ".
pub(super) fn labelled(path: String, label: &Message, short: Message) -> Problem {
    let text = Message::new("job-operation-problem")
        .arg("label", label.clone())
        .arg("problem", short.clone());
    Problem::new(path, short, text)
}

/// A length as Rust writes it with `{}`, with its unit.
pub(super) fn length(mm: f64) -> Value {
    Value::Length(mm, Digits::Shortest)
}

/// The problems, in the order the checks were always made (so the first is the error
/// `validate_job` always gave), and the notes of a job without problems.
/// A bull nose's warnings for an operation cut with an end mill's path
/// (docs/specs/corner-radius.md §4.2): a through cut less than the corner radius below the
/// board leaves a fillet on the cut's bottom edge, and a pocket's or surface's stepover
/// wider than the flat leaves ridges on the floor. `below`: how far the cut goes below the
/// board.
fn bull_nose_notes(op: &Operation, tool: &Tool, below: f64) -> Vec<Message> {
    let Some(corner) = tool.corner_radius.filter(|_| tool.is_bull_nose()) else {
        return Vec::new();
    };
    let mut notes = Vec::new();
    if op.through().is_some() && below < corner - DEPTH_TOL_MM {
        notes.push(
            Message::new("toolpath-through-corner")
                .arg("radius", length(corner))
                .arg("below", Value::Length(below.max(0.0), Digits::Trim(3))),
        );
    }
    let stepover = match op {
        Operation::Pocket(p) => Some(p.stepover),
        Operation::Surface(f) => Some(f.stepover),
        _ => None,
    };
    let flat = tool.diameter - 2.0 * corner;
    if let Some(step) = stepover.map(|s| s * tool.diameter)
        && step > flat + DEPTH_TOL_MM
    {
        // Halfway across the band the flat misses, the corner stands this high.
        let half = ((step - flat) / 2.0).min(corner);
        let ridge = corner - (corner * corner - half * half).max(0.0).sqrt();
        notes.push(
            Message::new("toolpath-corner-ridges")
                .arg("ridge", Value::Length(ridge, Digits::Trim(3)))
                .arg("flat", Value::Length(flat, Digits::Trim(3))),
        );
    }
    notes
}

/// How far a through cut goes into the spoilboard, as a plain note (docs/specs/styles.md
/// §2.1); None for a cut that isn't through or stays in the stock.
pub(super) fn through_overcut(job: &Job, op: &Operation) -> Option<Message> {
    op.through()?;
    let stock = job.setup_of(op).map_or(job.stock(), |s| &s.stock);
    let below = op.depths(&job.tools).1 - job.drill_point(op) - stock.thickness;
    (below > DEPTH_TOL_MM).then(|| {
        Message::new("toolpath-through-spoilboard")
            .arg("below", Value::Length(below, Digits::Fixed(3)))
    })
}

fn check_job(job: &Job) -> (Vec<Problem>, Vec<Note>) {
    let mut problems = Vec::new();
    if job.operations.is_empty() {
        problems.push(Problem::whole(
            "operations",
            Message::new("toolpath-no-operations"),
        ));
        return (problems, Vec::new());
    }
    if !(0..job.operations.len()).any(|i| job.cuts(i)) {
        problems.push(Problem::whole(
            "operations",
            Message::new("toolpath-all-disabled"),
        ));
        return (problems, Vec::new());
    }
    let enabled = || {
        job.operations
            .iter()
            .enumerate()
            .filter(|(i, _)| job.cuts(*i))
    };
    let tool_index = |id: i64| job.tools.iter().position(|t| t.id == id);
    let mut no_rpm: Vec<i64> = Vec::new();
    for (index, op) in enabled() {
        match tool_index(op.tool()) {
            None => {
                problems.push(Problem::whole(
                    format!("operations.{index}.tool"),
                    Message::new("toolpath-tool-undefined").arg("tool", op.tool()),
                ));
            }
            Some(k) => {
                let tool = &job.tools[k];
                if tool.rpm.is_none_or(|rpm| rpm <= 0.0) && !no_rpm.contains(&tool.id) {
                    no_rpm.push(tool.id);
                    problems.push(Problem::new(
                        format!("tools.{k}.rpm"),
                        Message::new("toolpath-no-rpm-short"),
                        Message::new("toolpath-no-rpm").arg("tool", tool.id),
                    ));
                }
            }
        }
    }
    // Safe Z clears every setup's board (docs/specs/setups.md §5): the highest top.
    let highest = job
        .setups
        .iter()
        .map(|s| &s.stock)
        .max_by(|a, b| a.top_z().total_cmp(&b.top_z()))
        .unwrap_or(job.stock());
    let top = highest.top_z();
    let z_zero = match highest.z_zero {
        ZZero::Top => "top",
        ZZero::Bottom => "bottom",
    };
    if job.machine.safe_z <= top {
        problems.push(Problem::new(
            "machine.safe_z",
            Message::new("toolpath-safe-z-short").arg("top", length(top)),
            Message::new("toolpath-safe-z")
                .arg("safe", length(job.machine.safe_z))
                .arg("top", length(top))
                .arg("zero", z_zero),
        ));
    }
    let mut notes = tool_group_notes(job);
    // Operations that plunge straight down with a tool not marked centre-cutting, by
    // tool: one note per tool, after the loop.
    let mut plunging: std::collections::BTreeMap<i64, Vec<(usize, &str)>> = Default::default();
    for (index, op) in enabled() {
        let Some(k) = tool_index(op.tool()) else {
            continue; // reported above
        };
        let tool = &job.tools[k];
        let number = index + 1;
        let label_message = op.label_message(index);
        let numbered = Message::new("job-operation-label").arg("number", number);
        let at = |field: &str| format!("operations.{index}.{field}");
        let (feed, plunge_feed) = match op {
            Operation::Profile(p) => (p.feed, p.plunge_feed),
            Operation::Pocket(p) => (p.feed, p.plunge_feed),
            Operation::Vcarve(v) => (v.feed, v.plunge_feed),
            Operation::Path(p) => (p.feed, p.plunge_feed),
            Operation::Chamfer(c) => (c.feed, c.plunge_feed),
            Operation::Flute(f) => (f.feed, f.plunge_feed),
            Operation::Pattern(f) => (f.feed, f.plunge_feed),
            Operation::Prism(f) => (f.feed, f.plunge_feed),
            Operation::Moulding(m) => (m.feed, m.plunge_feed),
            Operation::Holes(h) => (h.feed, h.plunge_feed),
            Operation::Surface(f) => (f.feed, f.plunge_feed),
            Operation::Relief(r) => (r.feed, r.plunge_feed),
        };
        // Below the stock: a warning within the overcut allowance, an error beyond it
        // (docs/specs/depths-and-clearance.md section 2).
        let (_, end_depth) = op.depths(&job.tools);
        // Its own setup's board (docs/specs/setups.md §5).
        let stock = job.setup_of(op).map_or(job.stock(), |s| &s.stock);
        let thickness = stock.thickness;
        // A drill's point goes deeper than its full diameter: the allowance is for the
        // diameter (docs/specs/holes.md decision 10).
        let below = end_depth - job.drill_point(op) - thickness;
        // Dowel holes go deeper, for the pins (docs/specs/two-sided.md §6).
        let allowed = op.allowed_below(stock.max_overcut);
        // A prism's end depth is only how deep its bit could go; how deep it does go follows
        // from its shapes and is checked when it is planned (docs/specs/prism.md §4).
        if below > allowed + DEPTH_TOL_MM && !matches!(op, Operation::Prism(_)) {
            problems.push(labelled(
                at("end_depth"),
                &label_message,
                Message::new("toolpath-too-deep")
                    .arg("end", length(end_depth))
                    .arg("below", Value::Length(below, Digits::Fixed(3)))
                    .arg("thickness", length(thickness))
                    .arg("allowed", length(allowed)),
            ));
        } else if below > DEPTH_TOL_MM && op.through().is_none() {
            // A through cut goes below on purpose: a plain note when planned (`facts`).
            notes.push(Note::at(
                index,
                labelled_note(
                    &label_message,
                    Message::new("toolpath-into-spoilboard")
                        .arg("below", Value::Length(below, Digits::Fixed(3))),
                ),
            ));
        }
        // A V-bit following a path no deeper than its cone: the shank would rub below it
        // (docs/specs/follow-path.md §5).
        if let Operation::Path(p) = op
            && let Some(deepest) = vbit_cone_depth(tool)
            && p.end_depth - p.start_depth > deepest + DEPTH_TOL_MM
        {
            problems.push(labelled(
                at("end_depth"),
                &label_message,
                Message::new("toolpath-path-past-cone")
                    .arg("deepest", Value::Length(deepest, Digits::Trim(3)))
                    .arg("depth", length(p.end_depth - p.start_depth)),
            ));
        }
        // A V-bit's flute or pattern no deeper than its cone (docs/specs/fluting.md §5,
        // fill-pattern.md §6).
        let groove = match op {
            Operation::Flute(f) => Some((f.start_depth, f.end_depth)),
            Operation::Pattern(f) => Some((f.start_depth, f.end_depth)),
            _ => None,
        };
        if let Some((start_depth, end_depth)) = groove
            && let Some(deepest) = vbit_cone_depth(tool)
            && end_depth - start_depth > deepest + DEPTH_TOL_MM
        {
            problems.push(labelled(
                at("end_depth"),
                &label_message,
                Message::new("toolpath-path-past-cone")
                    .arg("deepest", Value::Length(deepest, Digits::Trim(3)))
                    .arg("depth", length(end_depth - start_depth)),
            ));
        }
        // A chamfer no wider than the bit's cone: the shank would rub beyond it
        // (docs/specs/chamfer.md §4).
        if let Operation::Chamfer(c) = op
            && let Some(widest) = tool.widest_chamfer()
            && c.width > widest + DEPTH_TOL_MM
        {
            problems.push(labelled(
                at("width"),
                &label_message,
                Message::new("toolpath-chamfer-past-cone")
                    .arg("widest", Value::Length(widest, Digits::Trim(3)))
                    .arg("tool", tool.id)
                    .arg("width", length(c.width)),
            ));
        }
        notes.extend(
            bull_nose_notes(op, tool, below)
                .into_iter()
                .map(|n| Note::at(index, labelled_note(&label_message, n))),
        );
        if op.clearance() >= tool.diameter / 2.0 {
            problems.push(labelled(
                at("clearance"),
                &label_message,
                Message::new("toolpath-clearance-radius")
                    .arg("clearance", length(op.clearance()))
                    .arg("radius", length(tool.diameter / 2.0)),
            ));
        }
        if let Operation::Profile(ProfileOp {
            finish: Some(finish),
            ..
        }) = op
            && finish.stock >= tool.diameter / 2.0
        {
            problems.push(labelled(
                at("finish.stock"),
                &label_message,
                Message::new("toolpath-finish-radius")
                    .arg("stock", length(finish.stock))
                    .arg("radius", length(tool.diameter / 2.0)),
            ));
        }
        if let Operation::Pocket(PocketOp {
            finish: Some(finish),
            ..
        }) = op
            && finish.wall >= tool.diameter / 2.0
        {
            problems.push(labelled(
                at("finish.wall"),
                &label_message,
                Message::new("toolpath-finish-radius")
                    .arg("stock", length(finish.wall))
                    .arg("radius", length(tool.diameter / 2.0)),
            ));
        }
        if let Operation::Profile(ProfileOp {
            tabs: Some(tabs),
            start_depth,
            end_depth,
            ..
        }) = op
            && tabs.height >= end_depth - start_depth
        {
            let cut = end_depth - start_depth;
            problems.push(labelled(
                at("tabs.height"),
                &label_message,
                Message::new("toolpath-tab-height")
                    .arg("height", length(tabs.height))
                    .arg("cut", length(cut)),
            ));
        }
        if let Operation::Pocket(PocketOp {
            rest: Some(rest), ..
        }) = op
        {
            match rest_check(job, index, rest.after_tool, tool, &label_message) {
                Ok(more) => notes.extend(more.into_iter().map(|n| Note::at(index, n))),
                Err(short) => problems.push(labelled(at("rest"), &label_message, short)),
            }
        }
        if let Operation::Pocket(pocket) = op {
            let (more_problems, more_notes) = rest_pass_checks(job, index, pocket);
            problems.extend(more_problems);
            notes.extend(more_notes);
        }
        if let Operation::Vcarve(VcarveOp {
            floor: Some(floor), ..
        }) = op
        {
            let (more_problems, more_notes) = floor_pass_checks(job, index, floor);
            problems.extend(more_problems);
            notes.extend(more_notes);
        }
        if let Operation::Relief(chipload_job::ReliefOp {
            roughing: Some(rough),
            ..
        }) = op
            && rough.enabled
        {
            let (more_problems, more_notes) =
                relief_pass_checks(job, index, (0, "roughing"), (rough.tool, rough.allowance));
            problems.extend(more_problems);
            notes.extend(more_notes);
            if let Operation::Relief(chipload_job::ReliefOp {
                smoothing: Some(smooth),
                ..
            }) = op
                && smooth.enabled
            {
                let (more_problems, more_notes) = relief_pass_checks(
                    job,
                    index,
                    (1, "smoothing"),
                    (smooth.tool, smooth.allowance),
                );
                problems.extend(more_problems);
                notes.extend(more_notes);
            }
        } else if let Operation::Relief(relief) = op
            && relief.end_depth > tool.diameter + DEPTH_TOL_MM
        {
            // Deeper than the tool is wide in one go: a roughing pass takes most of it
            // (docs/specs/stl-relief.md §14 step 3).
            notes.push(Note::at(
                index,
                labelled_note(
                    &label_message,
                    Message::new("toolpath-relief-deep")
                        .arg("depth", length(relief.end_depth))
                        .arg("tool", tool.id),
                ),
            ));
        }
        if let Operation::Vcarve(vcarve) = op
            && let Some(plug) = vcarve.plug
        {
            let (more_problems, more_notes) =
                plug_checks(job, index, vcarve, plug, tool, &label_message);
            problems.extend(more_problems);
            notes.extend(more_notes);
        }
        if let Some(max_feed) = job.machine.max_feed {
            for (name, value) in [("feed", feed), ("plunge_feed", plunge_feed)] {
                if value > max_feed {
                    problems.push(labelled(
                        at(name),
                        &numbered,
                        Message::new("toolpath-max-feed")
                            .arg("field", name)
                            .arg("feed", Value::Feed(value, Digits::Shortest))
                            .arg("max", Value::Feed(max_feed, Digits::Shortest)),
                    ));
                }
            }
        }
        if let Entry::Helix { radius, .. } = op.entry() {
            if let Operation::Chamfer(_) = op {
                problems.push(labelled(
                    at("entry"),
                    &label_message,
                    Message::new("toolpath-helix-for-chamfer"),
                ));
            } else if let Operation::Path(_) = op {
                problems.push(labelled(
                    at("entry"),
                    &label_message,
                    Message::new("toolpath-helix-for-path"),
                ));
            } else if radius >= tool.diameter / 2.0 {
                problems.push(labelled(
                    at("entry"),
                    &label_message,
                    Message::new("toolpath-helix-radius")
                        .arg("radius", length(radius))
                        .arg("tool_radius", length(tool.diameter / 2.0)),
                ));
            }
        }
        if let Some(lead) = op.lead() {
            let longest = MAX_LEAD_DIAMETERS * tool.diameter;
            if lead.radius > longest {
                problems.push(labelled(
                    at("lead"),
                    &label_message,
                    Message::new("toolpath-lead-radius")
                        .arg("radius", length(lead.radius))
                        .arg("most", Value::Number(MAX_LEAD_DIAMETERS, Digits::Shortest))
                        .arg("longest", Value::Length(longest, Digits::Fixed(3))),
                ));
            }
        }
        // The tool's rules: a problem at its field, or whether the operation plunges
        // straight down with a tool of unknown centre-cutting, and any other notes.
        let (plunges, more): (ToolRule, Vec<Message>) = match op {
            Operation::Pocket(p) => (
                pocket_tool_notes(tool, p.entry).map_err(|e| {
                    let field = if tool.kind == ToolType::Endmill {
                        "entry"
                    } else {
                        "tool"
                    };
                    (at(field), numbered.clone(), e)
                }),
                Vec::new(),
            ),
            // A profile spirals down; only its tabs' flanks can be a plunge.
            Operation::Profile(p) => match flank_notes(number, tool, p) {
                Ok(found) => (Ok(false), found),
                Err(e) => (
                    Err((at("tabs.flank_angle"), numbered.clone(), e)),
                    Vec::new(),
                ),
            },
            Operation::Path(p) => (
                path_tool_notes(tool, p.entry).map_err(|e| (at("tool"), numbered.clone(), e)),
                Vec::new(),
            ),
            Operation::Chamfer(_) => (
                chamfer_tool_rule(tool).map_err(|e| {
                    let path = if tool.kind == ToolType::Vbit {
                        format!("tools.{k}.angle")
                    } else {
                        at("tool")
                    };
                    (path, numbered.clone(), e)
                }),
                Vec::new(),
            ),
            Operation::Flute(f) => (
                flute_tool_rule(tool, f).map_err(|e| (at("tool"), numbered.clone(), e)),
                Vec::new(),
            ),
            // A ball nose or an endmill of known shape (docs/specs/moulding.md §8).
            Operation::Moulding(_) => (
                moulding_tool_rule(tool).map_err(|e| (at("tool"), numbered.clone(), e)),
                Vec::new(),
            ),
            // A V-bit with its angle (docs/specs/prism.md §4).
            Operation::Prism(_) => (
                prism_tool_rule(tool).map_err(|e| {
                    let path = if tool.kind == ToolType::Vbit {
                        format!("tools.{k}.angle")
                    } else {
                        at("tool")
                    };
                    (path, numbered.clone(), e)
                }),
                Vec::new(),
            ),
            // Square ends and rings go straight down (docs/specs/fill-pattern.md §6).
            Operation::Pattern(f) => {
                let plunges = f.ramp <= 0.0 || matches!(f.pattern, FillPattern::Rings { .. });
                let entry = if plunges {
                    Entry::Plunge {}
                } else {
                    Entry::Ramp {
                        ramp_angle: chipload_job::DEFAULT_RAMP_ANGLE_RAD,
                    }
                };
                (
                    path_tool_notes(tool, entry).map_err(|e| (at("tool"), numbered.clone(), e)),
                    Vec::new(),
                )
            }
            Operation::Holes(h) => (
                holes_tool_notes(tool, h.method)
                    .map(|()| false)
                    .map_err(|e| (at("tool"), numbered.clone(), e)),
                Vec::new(),
            ),
            // It goes down beside the stock: no plunge into material.
            Operation::Surface(_) => (
                surface_tool_rule(tool).map_err(|e| (at("tool"), numbered.clone(), e)),
                surface_order_note(job, index, &label_message)
                    .into_iter()
                    .collect(),
            ),
            Operation::Relief(_) => (
                relief_tool_rule(tool).map_err(|e| (at("tool"), numbered.clone(), e)),
                Vec::new(),
            ),
            Operation::Vcarve(_) => match vcarve_tool_notes(&label_message, tool) {
                Ok(found) => (Ok(false), found),
                Err(e) => {
                    let path = if tool.kind == ToolType::Vbit {
                        format!("tools.{k}.angle")
                    } else {
                        at("tool")
                    };
                    (Err((path, label_message.clone(), e)), Vec::new())
                }
            },
        };
        match plunges {
            Ok(true) => plunging
                .entry(tool.id)
                .or_default()
                .push((index, op_kind_name(op))),
            Ok(false) => {}
            Err((path, prefix, short)) => problems.push(labelled(path, &prefix, short)),
        }
        notes.extend(more.into_iter().map(|n| Note::at(index, n)));
    }
    for (tool, ops) in plunging {
        let Some(&(first, _)) = ops.first() else {
            continue;
        };
        let listed: Vec<Message> = ops
            .iter()
            .map(|(index, kind)| {
                Message::new("toolpath-operation-kind")
                    .arg("number", index + 1)
                    .arg("kind", *kind)
            })
            .collect();
        let count = listed.len();
        let Some(listed) = and_list(listed) else {
            continue;
        };
        notes.push(Note::at(
            first,
            Message::new("toolpath-plunge-unknown")
                .arg("tool", tool)
                .arg("count", count)
                .arg("operations", listed),
        ));
    }
    (problems, notes)
}

/// A tool rule's outcome: whether the operation plunges with a tool of unknown
/// centre-cutting, or the problem's (path, label, short text).
type ToolRule = Result<bool, (String, Message, Message)>;

/// The kind of an operation in a note, as `toolpath-operation-kind` selects it.
fn op_kind_name(op: &Operation) -> &'static str {
    match op {
        Operation::Profile(_) => "profile",
        Operation::Pocket(_) => "pocket",
        Operation::Vcarve(_) => "vcarve",
        Operation::Path(_) => "path",
        Operation::Chamfer(_) => "chamfer",
        Operation::Flute(_) => "flute",
        Operation::Pattern(_) => "pattern",
        Operation::Prism(_) => "prism",
        Operation::Moulding(_) => "moulding",
        Operation::Holes(_) => "holes",
        Operation::Surface(_) => "surface",
        Operation::Relief(_) => "relief",
    }
}

/// Items joined with commas: "1, 2, 3".
pub(super) fn comma_list(items: Vec<Message>) -> Option<Message> {
    let mut items = items.into_iter();
    let first = items.next()?;
    Some(items.fold(first, |list, item| {
        Message::new("toolpath-comma")
            .arg("list", list)
            .arg("item", item)
    }))
}

/// Items joined with commas and "and" before the last: "1, 2 and 3".
pub(super) fn and_list(mut items: Vec<Message>) -> Option<Message> {
    let last = items.pop()?;
    Some(match comma_list(items) {
        Some(list) => Message::new("toolpath-and")
            .arg("list", list)
            .arg("last", last),
        None => last,
    })
}

/// A relief takes a ball nose, an endmill or a V-bit whose shape is known
/// (docs/specs/stl-relief.md §8); an endmill of unknown centre-cutting plunges to each line.
fn relief_tool_rule(tool: &Tool) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Drill => Err(Message::new("toolpath-relief-drill").arg("tool", tool.id)),
        _ if tool.cutter().is_none() => {
            Err(Message::new("toolpath-relief-shape").arg("tool", tool.id))
        }
        ToolType::Endmill => plunge_unknown(tool, Entry::Plunge {}),
        ToolType::Vbit | ToolType::Ballnose => Ok(false),
    }
}
