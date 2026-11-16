// SPDX-License-Identifier: GPL-3.0-or-later
//! Planning V-carves, and the V-groove facts.

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::profile::labelled_note;
use crate::select::Selection;
use crate::vcarve::{VBit, carve_levels, plan_vcarve_at, vcarve_moves};
use chipload_core::Point;
use chipload_core::math;
use chipload_core::tolerances::FLOOR_LEFT_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Tool, ToolType, VcarveOp};

use super::check_passes::plug_face;
use super::{Context, Planned, job_error};

/// How wide a V-bit's groove is at the top, `depth` deep (docs/specs/follow-path.md §5): a
/// plain note. None for another tool, or a V-bit without an angle.
pub(super) fn groove_width(tool: &Tool, depth: f64) -> Option<Message> {
    let width = groove_width_mm(tool, depth)?;
    Some(Message::new("toolpath-groove-width").arg("width", Value::mm(width)))
}

/// How wide a V-bit's groove is at the top, `depth` deep, mm (at most its diameter). None
/// for another tool, or a V-bit without an angle.
pub fn groove_width_mm(tool: &Tool, depth: f64) -> Option<f64> {
    let deepest = vbit_cone_depth(tool)?;
    let tip = tool.tip_diameter.unwrap_or(0.0);
    Some(tip + (tool.diameter - tip) * (depth / deepest).clamp(0.0, 1.0))
}

/// How deep a V-bit's cone goes, from its tip to its full diameter: below it, the shank
/// would rub (as `VBit::deepest` for V-carves). None for another tool, or a V-bit without
/// an angle.
pub fn vbit_cone_depth(tool: &Tool) -> Option<f64> {
    if tool.kind != ToolType::Vbit {
        return None;
    }
    let angle = tool.angle.filter(|a| *a > 0.0)?;
    let tip = tool.tip_diameter.unwrap_or(0.0);
    Some((tool.diameter - tip) / 2.0 / math::tan(angle / 2.0))
}

/// The smallest end mill a floor hint names, mm: below it, no end mill clears the floor.
const FLOOR_HINT_MIN_MM: f64 = 0.1;

pub(super) fn plan_vcarve_op(
    op: &VcarveOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let tool = ctx.tool;
    // Validated in validate_job: a V-bit with an angle.
    let angle = tool.angle.ok_or_else(|| {
        job_error(labelled_note(
            ctx.label_message,
            Message::new("toolpath-error-no-angle").arg("tool", tool.id),
        ))
    })?;
    let bit = VBit {
        angle,
        tip_diameter: tool.tip_diameter.unwrap_or(0.0),
        diameter: tool.diameter,
    };
    // The floor pass's end mill, when it is cut (checked in validate_job).
    let floor_tool = op
        .floor
        .filter(|f| f.enabled)
        .and_then(|f| ctx.job.tool(f.tool))
        .filter(|t| t.kind == ToolType::Endmill);
    // A plug's outline lies below the surface: carved from there, every depth that much
    // deeper (docs/specs/vcarve-inlay.md §2.2).
    let outline = op.outline_depth() - op.start_depth;
    let mut plan = plan_vcarve_at(
        &selection.contours,
        &bit,
        op.end_depth - op.outline_depth(),
        start,
        floor_tool.map(|t| t.diameter / 2.0),
        op.ignore_kinks_deg.to_radians(),
        outline,
    )
    .map_err(|err| err.with_context(ctx.label_message.clone()))?;
    if let Some(plug) = op.plug {
        // Where the board between two shapes is narrower than the tip, the plug's cone cuts
        // it from above its outline, as wide as it is: nothing is left narrow.
        plan.notes.clear();
        // What its floor pass leaves above the outline level stands into the base once the
        // plug is turned over (docs/specs/vcarve-inlay.md §12); a speck lower than
        // FLOOR_LEFT_MM doesn't count, as for any floor. Without a floor pass, the plug's
        // checks say so.
        if plan.floor_left.is_none()
            && let Some(top) = plan.floor_top.map(|top| op.start_depth + top)
            && top < plug.start_depth - FLOOR_LEFT_MM
        {
            plan.notes.push(
                Message::new("toolpath-plug-floor-left")
                    .arg("top", Value::Length(top, Digits::Fixed(2)))
                    .arg("outline", Value::mm(plug.start_depth)),
            );
        }
    }
    let mut notes = std::mem::take(&mut plan.notes);
    // An inlay's pocket: a floor left standing above the plug's face holds the plug up
    // before its walls meet (docs/specs/vcarve-inlay.md §12); a speck lower than
    // FLOOR_LEFT_MM doesn't count.
    if op.plug.is_none()
        && let Some(face) = plug_face(ctx.job, op, angle)
        && let Some(top) = plan.floor_top.map(|top| op.start_depth + top)
        && top < face - FLOOR_LEFT_MM
    {
        notes.push(if plan.floor_left.is_some() {
            Message::new("toolpath-inlay-pocket-no-floor")
        } else {
            Message::new("toolpath-inlay-pocket-floor")
                .arg("top", Value::Length(top, Digits::Fixed(2)))
                .arg("face", Value::mm(face))
        });
    }
    let mut facts: Vec<Message> = plan.floor_left.take().into_iter().collect();
    let floor_hint = plan.floors.iter().map(|f| f.largest).reduce(f64::min);
    if let Some(tool) = floor_tool {
        for left in &plan.floors {
            let note = if left.largest >= FLOOR_HINT_MIN_MM {
                Message::new("toolpath-vcarve-floor-cleared")
                    .arg("largest", Value::mm(left.largest))
            } else {
                Message::new("toolpath-vcarve-floor-cleared-none")
            };
            facts.push(
                note.arg("contours", left.contours.as_str())
                    .arg("diameter", Value::mm(tool.diameter))
                    .arg(
                        "cleared",
                        Value::Number((left.cleared * 100.0).floor(), Digits::Fixed(0)),
                    ),
            );
        }
    }
    // From the end depth the user set, so the passes are what they asked for
    // (docs/specs/v-carve.md decision 12, vcarve-step-down.md).
    let levels = carve_levels(op.end_depth - op.start_depth, op.passes, op.step_down);
    let moves = vcarve_moves(
        &plan.area_walks(),
        &levels,
        ctx.top_z,
        ctx.job.machine.safe_z,
        op.feed,
        op.plunge_feed,
    );
    let end = plan
        .paths
        .last()
        .and_then(|p| p.points.last().copied())
        .unwrap_or(start);
    Ok(Planned {
        moves,
        notes: notes
            .into_iter()
            .map(|n| labelled_note(ctx.label_message, n))
            .collect(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: ArcTable::default(),
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint,
        facts: facts
            .into_iter()
            .map(|n| labelled_note(ctx.label_message, n))
            .collect(),
    })
}
