// SPDX-License-Identifier: GPL-3.0-or-later
//! Planning holes operations: drill cycles and helical bores.

use chipload_core::Point;
use chipload_core::text::mm;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_i18n::{Message, Value};
use chipload_job::{HoleMethod, HolesOp};

use super::{Context, Planned, job_error};
use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::holes::{
    Bore, DRILL_FIT_MM, HoleCut, bore_moves, bore_radius, drill_moves, find_holes, order_holes,
};
use crate::lead::MIN_LEAD_DIAMETERS;
use crate::profile::{contour_context, labelled_note};
use crate::select::Selection;

/// A hole drilled in one go deeper than this many diameters gets a note to drill in pecks.
const DEEP_HOLE_DIAMETERS: f64 = 3.0;

/// The smallest hole an end mill of `diameter` bores: its own size plus a lead of
/// `MIN_LEAD_DIAMETERS` × D on each side (docs/specs/holes.md §4).
fn smallest_bore(diameter: f64) -> f64 {
    diameter * (1.0 + 2.0 * MIN_LEAD_DIAMETERS)
}

pub(super) fn plan_holes_op(
    op: &HolesOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let tool = ctx.tool;
    let (found, others) = find_holes(&selection.contours);
    let mut notes = Vec::new();
    if others > 0 {
        notes.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-holes-not-circles").arg("count", others),
        ));
    }
    let tool_radius = tool.diameter / 2.0;
    // Holes this tool can't cut, by diameter: diameter text -> (diameter, count).
    let mut unfit: std::collections::BTreeMap<String, (f64, usize)> = Default::default();
    let mut cores = 0;
    let mut holes = Vec::new();
    for hole in found {
        let diameter = mm(2.0 * hole.radius);
        let fits = match op.method {
            HoleMethod::Drill { .. } => {
                (2.0 * hole.radius - tool.diameter).abs() <= DRILL_FIT_MM + CHAIN_TOL_MM
            }
            HoleMethod::Bore { .. } => {
                bore_radius(&hole, tool_radius) >= MIN_LEAD_DIAMETERS * tool.diameter - CHAIN_TOL_MM
            }
        };
        if !fits {
            unfit.entry(diameter).or_insert((2.0 * hole.radius, 0)).1 += 1;
            continue;
        }
        if let HoleMethod::Bore { .. } = op.method
            && bore_radius(&hole, tool_radius) > tool_radius + CHAIN_TOL_MM
        {
            // A core is left standing inside the helix's ring (docs/specs/holes.md §4).
            if op.end_depth < ctx.job.stock().thickness - DEPTH_TOL_MM {
                return Err(job_error(labelled_note(
                    ctx.label_message,
                    Message::new("toolpath-error-blind-bore")
                        .arg("diameter", Value::mm(2.0 * hole.radius))
                        .arg("tool", Value::mm(tool.diameter)),
                )));
            }
            cores += 1;
        }
        holes.push(hole);
    }
    for &(diameter, count) in unfit.values() {
        let note = match op.method {
            HoleMethod::Drill { .. } => {
                Message::new("toolpath-holes-not-drill-size").arg("within", Value::mm(DRILL_FIT_MM))
            }
            HoleMethod::Bore { .. } => Message::new("toolpath-holes-too-small-to-bore")
                .arg("smallest", Value::mm(smallest_bore(tool.diameter))),
        };
        notes.push(labelled_note(
            ctx.label_message,
            note.arg("count", count)
                .arg("diameter", Value::mm(diameter))
                .arg("tool", Value::mm(tool.diameter)),
        ));
    }
    if cores > 0 {
        notes.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-holes-core").arg("count", cores),
        ));
    }
    // Deeper than three times the drill in one go: the chips may pack and the drill bind
    // (the review of 2026-11-02); pecks of one diameter are a common start.
    let depth = op.end_depth - op.start_depth;
    if let HoleMethod::Drill { peck: None } = op.method
        && depth > DEEP_HOLE_DIAMETERS * tool.diameter + DEPTH_TOL_MM
    {
        notes.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-holes-deep-no-pecks")
                .arg("depth", Value::mm(depth))
                .arg("peck", Value::mm(tool.diameter)),
        ));
    }
    if holes.is_empty() {
        // Why, with the sizes there are (the review of 2026-11-02: the notes above go with
        // a refusal).
        let sizes = || {
            let least = unfit
                .values()
                .map(|(d, _)| *d)
                .fold(f64::INFINITY, f64::min);
            let most = unfit.values().map(|(d, _)| *d).fold(0.0, f64::max);
            (unfit.len(), Value::mm(least), Value::mm(most))
        };
        let why = match op.method {
            _ if unfit.is_empty() => Message::new("toolpath-error-no-hole-fits"),
            HoleMethod::Drill { .. } => {
                let (count, least, most) = sizes();
                Message::new("toolpath-error-no-hole-drill-size")
                    .arg("within", Value::mm(DRILL_FIT_MM))
                    .arg("count", count)
                    .arg("least", least)
                    .arg("most", most)
            }
            HoleMethod::Bore { .. } => {
                let (count, least, most) = sizes();
                Message::new("toolpath-error-no-hole-to-bore")
                    .arg("smallest", Value::mm(smallest_bore(tool.diameter)))
                    .arg("count", count)
                    .arg("least", least)
                    .arg("most", most)
            }
        };
        return Err(job_error(labelled_note(
            ctx.label_message,
            why.arg("tool", tool.id)
                .arg("diameter", Value::mm(tool.diameter))
                .arg("type", tool.kind.name()),
        )));
    }
    let end_z = ctx.pass_z.last().copied().ok_or_else(|| {
        job_error(labelled_note(
            ctx.label_message,
            Message::new("toolpath-error-no-depth"),
        ))
    })?;
    let cut = HoleCut {
        top_z: ctx.top_z,
        end_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        tool_radius,
    };
    let mut moves = Vec::new();
    let mut end = start;
    for hole in order_holes(holes, start) {
        ctx.cancel.check()?;
        match op.method {
            HoleMethod::Drill { .. } => moves.extend(drill_moves(hole.center, ctx.pass_z, &cut)),
            HoleMethod::Bore {
                step_down,
                ramp_angle,
                direction,
            } => {
                let bore = Bore {
                    step_down,
                    ramp_angle,
                    direction,
                    spindle: ctx.sense,
                };
                moves.extend(bore_moves(&hole, &bore, &cut).map_err(|err| {
                    err.with_context(contour_context(ctx.label_message, hole.contour_id))
                })?);
            }
        }
        end = hole.center;
    }
    Ok(Planned {
        moves,
        notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: ArcTable::default(),
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    })
}
