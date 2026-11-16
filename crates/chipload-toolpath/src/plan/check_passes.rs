// SPDX-License-Identifier: GPL-3.0-or-later
//! Rules for surfaces, rest passes, V-carve plugs and floor passes.

use chipload_core::math;
use chipload_core::tolerances::{ANGLE_TOL_RAD, DEPTH_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    ClearingPass, Job, Note, OpRef, Operation, PocketOp, Problem, RestRefusal, Tool, ToolType,
    VcarveOp, rest_refusal,
};

use super::check::{and_list, labelled, length};
use super::check_tools::pocket_tool_notes;
use crate::profile::labelled_note;

/// A surface needs an endmill (docs/specs/surfacing.md §4). It never plunges into
/// material.
pub(super) fn surface_tool_rule(tool: &Tool) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Endmill => Ok(false),
        other => Err(Message::new("toolpath-surface-endmill")
            .arg("tool", tool.id)
            .arg("type", other.name_message())),
    }
}

/// A surface after other operations of its part faces what they already cut
/// (docs/specs/surfacing.md §6): a warning, as it is usually put first.
pub(super) fn surface_order_note(job: &Job, index: usize, label: &Message) -> Option<Message> {
    let part = job.operations.get(index)?.part();
    let before: Vec<Message> = (0..index)
        .filter(|&i| job.cuts(i) && job.operations[i].part() == part)
        .filter(|&i| !job.operations[i].is_board_surface())
        .map(|i| Message::new("toolpath-number").arg("number", i + 1))
        .collect();
    let count = before.len();
    let listed = and_list(before)?;
    Some(
        Message::new("toolpath-surface-after")
            .arg("label", label.clone())
            .arg("count", count)
            .arg("operations", listed),
    )
}

/// A board surface after another one of the same setup faces the same top twice
/// (docs/specs/surface-levels.md decision 4): a warning naming the first.
pub(super) fn board_twice_note(job: &Job, index: usize, label: &Message) -> Option<Message> {
    let op = job.operations.get(index)?;
    let setup = job.setup_of(op)?.id;
    let first = (0..index).find(|&i| {
        let other = &job.operations[i];
        job.cuts(i) && other.is_board_surface() && job.setup_of(other).map(|s| s.id) == Some(setup)
    })?;
    Some(
        Message::new("toolpath-surface-twice")
            .arg("label", label.clone())
            .arg("other", first + 1),
    )
}

/// Rest machining's rules (docs/specs/rest-machining.md §5) for the pocket at `index`,
/// which cuts with `tool`: a problem's text, or the warnings.
pub(super) fn rest_check(
    job: &Job,
    index: usize,
    after_tool: i64,
    tool: &Tool,
    label: &Message,
) -> Result<Vec<Message>, Message> {
    // The warnings carry the label; a problem's text gets it from `labelled`.
    let Some(Operation::Pocket(pocket)) = job.operations.get(index) else {
        return Ok(Vec::new());
    };
    let Some(large) = job.tool(after_tool) else {
        return Err(Message::new("toolpath-rest-no-tool").arg("tool", after_tool));
    };
    match rest_refusal(tool, large) {
        Some(RestRefusal::LargeNotEndmill) => {
            return Err(Message::new("toolpath-rest-not-endmill")
                .arg("tool", after_tool)
                .arg("type", large.kind.name_message()));
        }
        Some(RestRefusal::NotLarger) => {
            return Err(Message::new("toolpath-rest-not-larger")
                .arg("tool", after_tool)
                .arg("large", Value::mm(large.diameter))
                .arg("small", Value::mm(tool.diameter)));
        }
        // The pocket's own tool is checked with the pocket's tool rules.
        Some(RestRefusal::SmallNotEndmill) | None => {}
    }
    if chipload_job::has_bones(pocket.corners, &pocket.corner_choices) {
        return Err(Message::new("toolpath-rest-dogbones"));
    }
    let earlier = job.operations[..index].iter().any(|op| {
        matches!(op, Operation::Pocket(p)
            if p.enabled
                && (p.tool == after_tool || p.rest_passes.iter().any(|r| r.tool == after_tool))
                && p.select == pocket.select
                && p.end_depth >= pocket.end_depth - DEPTH_TOL_MM)
    });
    if earlier {
        Ok(Vec::new())
    } else {
        Ok(vec![labelled_note(
            label,
            Message::new("toolpath-rest-no-earlier").arg("tool", after_tool),
        )])
    }
}

/// A pocket's rest passes (docs/specs/rest-passes.md §7): each cuts with a project endmill
/// smaller than the tool before it, with a spindle speed, and not under dogbones. Their
/// plunge notes are as for any pocket.
pub(super) fn rest_pass_checks(
    job: &Job,
    index: usize,
    pocket: &PocketOp,
) -> (Vec<Problem>, Vec<Note>) {
    let (mut problems, mut notes) = (Vec::new(), Vec::new());
    if pocket.rest_passes.is_empty() {
        return (problems, notes);
    }
    if chipload_job::has_bones(pocket.corners, &pocket.corner_choices) {
        problems.push(labelled(
            format!("operations.{index}.rest_passes"),
            &job.label_message(OpRef::op(index)),
            Message::new("toolpath-rest-passes-dogbones"),
        ));
    }
    for (k, pass) in pocket.rest_passes.iter().enumerate() {
        let label_message = job.label_message(OpRef::pass(index, k));
        let at = |field: &str| format!("operations.{index}.rest_passes.{k}.{field}");
        let Some(tool) = job.tool(pass.tool) else {
            problems.push(labelled(
                at("tool"),
                &label_message,
                Message::new("toolpath-rest-pass-no-tool").arg("tool", pass.tool),
            ));
            continue;
        };
        let before_id = match k {
            0 => pocket.tool,
            _ => pocket.rest_passes[k - 1].tool,
        };
        if let Some(before) = job.tool(before_id) {
            match rest_refusal(tool, before) {
                Some(RestRefusal::SmallNotEndmill) => problems.push(labelled(
                    at("tool"),
                    &label_message,
                    Message::new("toolpath-rest-pass-endmill")
                        .arg("tool", tool.id)
                        .arg("type", tool.kind.name_message()),
                )),
                Some(RestRefusal::NotLarger) => problems.push(labelled(
                    at("tool"),
                    &label_message,
                    Message::new("toolpath-rest-pass-larger")
                        .arg("tool", tool.id)
                        .arg("diameter", Value::mm(tool.diameter))
                        .arg("before", before.id)
                        .arg("before_diameter", Value::mm(before.diameter)),
                )),
                // The tool before it is the pocket's (checked with the pocket) or a pass's
                // (checked above).
                Some(RestRefusal::LargeNotEndmill) | None => {}
            }
        }
        if tool.rpm.is_none_or(|rpm| rpm <= 0.0) {
            problems.push(labelled(
                at("tool"),
                &label_message,
                Message::new("toolpath-rest-pass-no-rpm").arg("tool", tool.id),
            ));
        }
        if let Ok(true) = pocket_tool_notes(tool, pass.entry) {
            notes.push(Note::at(
                index,
                labelled_note(
                    &label_message,
                    Message::new("toolpath-rest-pass-plunge").arg("tool", tool.id),
                ),
            ));
        }
    }
    (problems, notes)
}

/// Whether V-carves `a` and `b` are an inlay's pocket and plug as far as their carving
/// goes: the same shapes, and V-bits of the same angle (docs/specs/vcarve-inlay.md
/// decision 1). `angle` is `a`'s bit's.
fn pairs(job: &Job, a: &VcarveOp, angle: f64, b: &VcarveOp) -> bool {
    a.select == b.select
        && job
            .tool(b.tool)
            .and_then(|t| t.angle)
            .is_some_and(|other| (other - angle).abs() <= ANGLE_TOL_RAD)
}

/// How deep the face of the ordinary V-carve `pocket`'s plug lies in it, mm down from the
/// stock top (the shallowest, with several): a V-carve with a `plug` that pairs with it, on,
/// in another setup that is on, its start depth less its own start. None without one.
/// Planned as its own setup, the job has the other setups' parts off (`Job::for_setup`),
/// so their switch isn't asked.
pub(super) fn plug_face(job: &Job, pocket: &VcarveOp, angle: f64) -> Option<f64> {
    let setup = job.part(pocket.part).map(|p| p.setup);
    job.operations
        .iter()
        .filter(|op| {
            op.enabled()
                && job
                    .setup_of(op)
                    .is_some_and(|s| s.enabled && Some(s.id) != setup)
        })
        .filter_map(|op| match op {
            Operation::Vcarve(v) if pairs(job, pocket, angle, v) => {
                v.plug.map(|plug| plug.start_depth - v.start_depth)
            }
            _ => None,
        })
        .reduce(f64::min)
}

/// The glue gap below which a plug's back may meet the base before its walls do, mm
/// (docs/specs/vcarve-inlay.md §6).
const PLUG_GAP_MIN_MM: f64 = 0.5;

/// The least a plug board keeps under its channel floor, mm (docs/specs/vcarve-inlay.md §6).
const PLUG_BACK_MIN_MM: f64 = 2.0;

/// An inlay plug's checks (docs/specs/vcarve-inlay.md §6). Problems: deeper than its V-bit
/// cuts, copies (each copy would carve the others away), a floor pass's end mill that would
/// stray off the board. Warnings: no pocket for it, a pocket it wouldn't seat in, its part
/// not mirrored, a small glue gap, a thin back, a pocket that ignores kinks, no floor pass,
/// and a flat tip's shortfall at sharp corners.
pub(super) fn plug_checks(
    job: &Job,
    index: usize,
    vcarve: &VcarveOp,
    plug: chipload_job::Plug,
    bit: &Tool,
    label: &Message,
) -> (Vec<Problem>, Vec<Note>) {
    let (mut problems, mut notes) = (Vec::new(), Vec::new());
    let at = |field: &str| format!("operations.{index}.{field}");
    let note = |message: Message| Note::at(index, labelled_note(label, message));
    let Some(angle) = bit.angle else {
        return (problems, notes); // reported with the V-bit's checks
    };
    let tip = bit.tip_diameter.unwrap_or(0.0);
    let deepest = (bit.diameter - tip) / 2.0 / math::tan(angle / 2.0);
    let depth = vcarve.end_depth - vcarve.start_depth;
    if depth > deepest + DEPTH_TOL_MM {
        problems.push(labelled(
            at("end_depth"),
            label,
            Message::new("toolpath-plug-too-deep")
                .arg("tool", bit.id)
                .arg("deepest", Value::Length(deepest, Digits::Fixed(3)))
                .arg("got", length(depth)),
        ));
    }
    if job
        .part_of(&job.operations[index])
        .is_some_and(|p| p.count() > 1)
    {
        problems.push(labelled(
            at("part"),
            label,
            Message::new("toolpath-plug-copies"),
        ));
    }
    let floor = vcarve.floor.filter(|f| f.enabled);
    match floor.and_then(|f| job.tool(f.tool)) {
        Some(mill) if mill.diameter / 2.0 > bit.diameter / 2.0 + tip / 2.0 + DEPTH_TOL_MM => {
            problems.push(labelled(
                at("floor.tool"),
                label,
                Message::new("toolpath-plug-floor-wide")
                    .arg("tool", mill.id)
                    .arg("bit", bit.id),
            ));
        }
        Some(_) => {}
        None => notes.push(note(Message::new("toolpath-plug-no-floor"))),
    }
    // Its pocket: a V-carve of the same shapes with a bit of the same angle, in another
    // setup (docs/specs/vcarve-inlay.md decision 1). Planned as its own setup, the job has
    // the other setups' parts off (`Job::for_setup`): an operation that is on, in a setup
    // that is on, counts.
    let setup = job.setup_of(&job.operations[index]).map(|s| s.id);
    let pockets: Vec<&VcarveOp> = job
        .operations
        .iter()
        .filter(|op| op.enabled() && job.setup_of(op).is_some_and(|s| s.enabled))
        .filter_map(|op| match op {
            Operation::Vcarve(v) => Some((op, v)),
            _ => None,
        })
        .filter(|(op, v)| {
            v.plug.is_none()
                && job.setup_of(op).map(|s| s.id) != setup
                && pairs(job, vcarve, angle, v)
        })
        .map(|(_, v)| v)
        .collect();
    // Degrees to a tenth: the angle is stored in radians.
    let degrees = Value::Number((angle.to_degrees() * 10.0).round() / 10.0, Digits::Shortest);
    if pockets.is_empty() {
        notes.push(note(
            Message::new("toolpath-plug-no-pocket").arg("angle", degrees),
        ));
    }
    // One pocket that fits is enough: a project may carve the same shapes more than once.
    let face = plug.start_depth - vcarve.start_depth;
    let seats = |p: &&VcarveOp| p.end_depth - p.start_depth > face + DEPTH_TOL_MM;
    let fits = pockets
        .iter()
        .any(|p| seats(p) && p.ignore_kinks_deg <= 0.0);
    if !fits && !pockets.is_empty() {
        let deepest = pockets
            .iter()
            .map(|p| p.end_depth - p.start_depth)
            .fold(0.0, f64::max);
        if let Some(kinks) = pockets.iter().find(|p| seats(p)) {
            notes.push(note(Message::new("toolpath-plug-pocket-kinks").arg(
                "degrees",
                Value::Number(kinks.ignore_kinks_deg, Digits::Shortest),
            )));
        } else {
            notes.push(note(
                Message::new("toolpath-plug-wont-seat")
                    .arg("pocket", length(deepest))
                    .arg("face", length(face)),
            ));
        }
    }
    if job.mirror_of(&job.operations[index]).is_none() {
        notes.push(note(Message::new("toolpath-plug-not-mirrored")));
    }
    let gap = vcarve.end_depth - plug.start_depth;
    if gap < PLUG_GAP_MIN_MM - DEPTH_TOL_MM {
        notes.push(note(
            Message::new("toolpath-plug-gap")
                .arg("gap", length(gap))
                .arg("least", length(PLUG_GAP_MIN_MM)),
        ));
    }
    let stock = job
        .setup_of(&job.operations[index])
        .map_or(job.stock(), |s| &s.stock);
    let back = stock.thickness - vcarve.end_depth;
    if back < PLUG_BACK_MIN_MM - DEPTH_TOL_MM {
        notes.push(note(
            Message::new("toolpath-plug-thin-back")
                .arg("back", length(back))
                .arg("least", length(PLUG_BACK_MIN_MM)),
        ));
    }
    if tip > 0.0 {
        // How far from a right-angled corner the fit falls short: w/2 ÷ sin(45°).
        let short = tip / 2.0 / math::sin(std::f64::consts::FRAC_PI_4);
        notes.push(note(
            Message::new("toolpath-plug-flat-tip")
                .arg("tip", length(tip))
                .arg("short", Value::Length(short, Digits::Fixed(2))),
        ));
    }
    (problems, notes)
}

/// A V-carve's floor pass (docs/specs/vcarve-wide-strokes.md §4): it cuts with a project
/// end mill with a spindle speed; its plunge notes are as for any pocket.
pub(super) fn floor_pass_checks(
    job: &Job,
    index: usize,
    floor: &ClearingPass,
) -> (Vec<Problem>, Vec<Note>) {
    let (mut problems, mut notes) = (Vec::new(), Vec::new());
    let label_message = job.label_message(OpRef::pass(index, 0));
    let at = |field: &str| format!("operations.{index}.floor.{field}");
    let Some(tool) = job.tool(floor.tool) else {
        problems.push(labelled(
            at("tool"),
            &label_message,
            Message::new("toolpath-rest-pass-no-tool").arg("tool", floor.tool),
        ));
        return (problems, notes);
    };
    if tool.kind != ToolType::Endmill {
        problems.push(labelled(
            at("tool"),
            &label_message,
            Message::new("toolpath-floor-pass-endmill")
                .arg("tool", tool.id)
                .arg("type", tool.kind.name_message()),
        ));
    }
    if tool.rpm.is_none_or(|rpm| rpm <= 0.0) {
        problems.push(labelled(
            at("tool"),
            &label_message,
            Message::new("toolpath-rest-pass-no-rpm").arg("tool", tool.id),
        ));
    }
    if let Ok(true) = pocket_tool_notes(tool, floor.entry) {
        notes.push(Note::at(
            index,
            labelled_note(
                &label_message,
                Message::new("toolpath-rest-pass-plunge").arg("tool", tool.id),
            ),
        ));
    }
    (problems, notes)
}
