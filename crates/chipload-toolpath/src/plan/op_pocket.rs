// SPDX-License-Identifier: GPL-3.0-or-later
//! Planning pocket and follow-path operations.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_i18n::{Message, Value};
use chipload_job::{Entry, PocketOp, Side};

use super::{Context, Planned, job_error};
use crate::ToolpathError;
use crate::corners::{
    CornerCut, RingSlot, add_legs, find_corners, pocket_boundaries, pocket_corners, unreached_note,
};
use crate::pocket::{
    Choices, PocketCut, PocketPiece, PocketRequest, PocketRing, plan_pocket, pocket_moves,
};
use crate::profile::{PathRing, labelled_note, pass_depths};
use crate::select::Selection;

pub(super) fn plan_pocket_op(
    op: &PocketOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    starts: &[Point],
) -> Result<Planned, ToolpathError> {
    // A finishing pass (docs/specs/pocket-finish.md): the roughing keeps `wall` from the
    // walls and stops `floor` above the end depth.
    let finish = op.finish.filter(|f| f.wall > 0.0 || f.floor > 0.0);
    let (wall, floor) = finish.map_or((0.0, 0.0), |f| (f.wall.max(0.0), f.floor.max(0.0)));
    let end_z = ctx.pass_z.last().copied().unwrap_or(ctx.top_z);
    let rough_z: Vec<f64> = if floor > 0.0 {
        pass_depths(0.0, ctx.top_z - (end_z + floor), op.step_down)?
            .into_iter()
            .map(|d| ctx.top_z - d)
            .collect()
    } else {
        ctx.pass_z.to_vec()
    };
    let rough = pocket_plan(op, ctx, selection, start, starts, op.clearance - wall, true);
    let rough = if wall > 0.0 {
        rough.map_err(|err| {
            err.with_context(
                Message::new("toolpath-pocket-finish-roughing").arg("wall", Value::mm(wall)),
            )
        })?
    } else {
        rough?
    };
    // The wall laps: today's wall rings, at the pocket's clearance.
    let laps = if wall > 0.0 {
        Some(pocket_plan(
            op,
            ctx,
            selection,
            start,
            starts,
            op.clearance,
            false,
        )?)
    } else {
        None
    };
    let mut notes: Vec<Message> = rough.notes;
    let mut arcs = rough.plan.arcs;
    let mut unreached = rough.unreached;
    if let Some(laps) = &laps {
        // Only the laps' own notes: the clearing is the roughing's.
        notes.extend(
            laps.plan
                .notes
                .iter()
                .filter(|n| lap_note(n))
                .map(|n| labelled_note(ctx.label_message, n.clone())),
        );
        arcs.extend(laps.plan.arcs.clone());
        unreached = laps.unreached.clone();
        for piece in &laps.plan.pieces {
            let roughed = rough
                .plan
                .pieces
                .iter()
                .find(|r| r.contour_ids == piece.contour_ids);
            for ring in piece.rings.iter().filter(|r| r.wall) {
                let label = |err: ToolpathError| err.with_context(ctx.label_message.clone());
                let Some(roughed) = roughed else {
                    let at = ring.plain().first().copied().unwrap_or(start);
                    return Err(label(crate::pocket_finish::narrow(at, wall)));
                };
                let slot = crate::pocket_finish::slot_rings(roughed);
                let radius = ctx.tool.diameter / 2.0;
                if let Some(note) =
                    crate::pocket_finish::reach_note(ring, &slot, radius, wall).map_err(label)?
                {
                    notes.push(labelled_note(ctx.label_message, note));
                }
            }
        }
    }
    let mut out = Planned {
        moves: Vec::new(),
        notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs,
        end: start,
        starts: Vec::new(),
        unreached,
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    };
    let cut = PocketCut {
        pass_z: &rough_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        entry: op.entry,
        slot_feed: op.slot_feed.unwrap_or(op.feed),
        slot_step: op.slot_step_down,
    };
    let floor_z = [end_z];
    let floor_cut = PocketCut {
        pass_z: &floor_z,
        top_z: rough_z.last().copied().unwrap_or(ctx.top_z),
        ..cut
    };
    let lap_cut = crate::pocket_finish::LapCut {
        z: end_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
    };
    let walls_of = laps.as_ref().map_or(&rough.plan.pieces, |l| &l.plan.pieces);
    for piece in walls_of {
        for ring in piece.rings.iter().filter(|r| r.wall) {
            // Without the leads, if it has them.
            out.starts.push((ring.plain().to_vec(), ring.chosen));
        }
    }
    let label = |err: ToolpathError| err.with_context(ctx.label_message.clone());
    for piece in &rough.plan.pieces {
        out.moves.extend(pocket_moves(piece, &cut).map_err(label)?);
        // Retracted above the start of the wall ring.
        if let Some(end) = piece.rings.last().and_then(PocketRing::end) {
            out.end = end;
        }
        if floor > 0.0 {
            out.moves
                .extend(pocket_moves(piece, &floor_cut).map_err(label)?);
        }
        let Some(laps) = &laps else { continue };
        let slot = crate::pocket_finish::slot_rings(piece);
        let finished = laps
            .plan
            .pieces
            .iter()
            .filter(|l| l.contour_ids == piece.contour_ids);
        for ring in finished.flat_map(|l| l.rings.iter().filter(|r| r.wall)) {
            let (moves, end) = crate::pocket_finish::finish_lap(ring, &slot, &lap_cut);
            out.moves.extend(moves);
            out.end = end;
        }
    }
    out.pockets = rough.plan.pieces;
    Ok(out)
}

/// A finishing lap's notes worth passing on from its plan: about its leads and starts.
fn lap_note(note: &Message) -> bool {
    let id = note.id;
    id.starts_with("toolpath-lead")
        || id.starts_with("toolpath-no-lead")
        || id.starts_with("toolpath-start")
}

/// A pocket planned with its corners, and what that said.
struct PocketPlanned {
    plan: crate::pocket::PocketPlan,
    /// The plan's notes and the corners', labelled.
    notes: Vec<Message>,
    unreached: Vec<Point>,
}

/// `op` planned at `clearance` (the roughing of a finished pocket keeps farther from the
/// walls), with its dogbones; `clearing` false plans only for the wall rings (no spiral, no
/// helix).
fn pocket_plan(
    op: &PocketOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    starts: &[Point],
    clearance: f64,
    clearing: bool,
) -> Result<PocketPlanned, ToolpathError> {
    // Checked before planning (check_job); never fall back to the whole pocket.
    let rest = match op.rest {
        Some(rest) => Some(
            ctx.job
                .tool(rest.after_tool)
                .map(|t| t.diameter / 2.0)
                .ok_or_else(|| {
                    job_error(labelled_note(
                        ctx.label_message,
                        Message::new("toolpath-error-rest-tool").arg("tool", rest.after_tool),
                    ))
                })?,
        ),
        None => None,
    };
    let request = PocketRequest {
        contours: &selection.contours,
        tool_radius: ctx.tool.diameter / 2.0,
        clearance,
        stepover: op.stepover * ctx.tool.diameter,
        direction: op.direction,
        spindle: ctx.sense,
        start,
        choices: Choices {
            starts,
            lead: op.lead.map(|l| l.radius),
            helix: match op.entry {
                Entry::Helix { radius, .. } if clearing => Some(radius),
                _ => None,
            },
            round_corners: true,
            spiral: clearing,
            cancel: Some(ctx.cancel),
            rest,
            floor: ctx.floor_pass,
        },
    };
    let mut plan =
        plan_pocket(&request).map_err(|err| err.with_context(ctx.label_message.clone()))?;
    let mut corner_notes = Vec::new();
    let mut unreached_at = Vec::new();
    if op.corners.is_some() || !ctx.choices.is_empty() {
        let boundaries = pocket_boundaries(&selection.contours);
        let cut = CornerCut {
            corners: op.corners,
            choices: ctx.choices,
            tool_radius: ctx.tool.diameter / 2.0,
            clearance,
            everything: ctx.cut_contours,
            // Another boundary of the pocket has waste on both sides near a corner.
            exempt: &boundaries,
            walls: &selection.contours,
            clears_inside: true,
            label: ctx.label_message,
        };
        for piece in &mut plan.pieces {
            let (unreached, warnings) = pocket_legs(piece, &selection.contours, &boundaries, &cut)?;
            corner_notes.extend(warnings);
            unreached_at.extend(unreached.iter().map(|(c, _)| c.at));
            corner_notes.extend(
                unreached
                    .iter()
                    .map(|(c, bone)| unreached_note(ctx.label_message, c, *bone)),
            );
        }
    }
    let notes = plan
        .notes
        .iter()
        .map(|n| labelled_note(ctx.label_message, n.clone()))
        .chain(corner_notes)
        .collect();
    Ok(PocketPlanned {
        plan,
        notes,
        unreached: unreached_at,
    })
}

/// What adding the legs left: the corners no ring reaches, and the legs' warnings.
type Legs = (
    Vec<(crate::corners::Corner, crate::corners::CornerBone)>,
    Vec<Message>,
);

/// Dogbones in a pocket piece's wall rings (docs/specs/dogbones.md §4): the corners of the
/// piece's boundaries (cut inside) and islands (cut outside); the leads' ranges move with
/// the added points. Returns the corners no wall ring reaches.
fn pocket_legs(
    piece: &mut PocketPiece,
    contours: &[&Contour],
    boundaries: &[usize],
    cut: &CornerCut,
) -> Result<Legs, ToolpathError> {
    let of: Vec<&Contour> = contours
        .iter()
        .copied()
        .filter(|c| piece.contour_ids.contains(&c.id))
        .collect();
    let corners = pocket_corners(&of, contours, boundaries);
    let mut slots: Vec<RingSlot> = piece
        .rings
        .iter_mut()
        .filter(|r| r.wall)
        .map(|ring| RingSlot {
            range: ring.lead.clone().unwrap_or(0..ring.points.len()),
            points: &mut ring.points,
        })
        .collect();
    let added = add_legs(&corners, &mut slots, cut)?;
    let walls = piece.rings.iter_mut().filter(|r| r.wall);
    for (count, ring) in added.points.iter().zip(walls) {
        if let Some(range) = &mut ring.lead {
            range.end += count;
        }
    }
    Ok((added.unreached, added.warnings))
}

/// Dogbones in a profile's rings (docs/specs/dogbones.md §4), each contour's corners in its
/// own rings, reaching the corner moved by `cut.clearance`.
pub(super) fn profile_legs(
    rings: &mut [PathRing],
    contours: &[&Contour],
    side: Side,
    cut: &CornerCut,
) -> Result<Legs, ToolpathError> {
    let mut unreached = Vec::new();
    let mut warnings = Vec::new();
    for contour in contours {
        let corners = find_corners(contour, side == Side::Inside);
        if corners.is_empty() {
            continue;
        }
        let mut slots: Vec<RingSlot> = rings
            .iter_mut()
            .filter(|r| r.contour_id == contour.id)
            .map(|r| {
                let range = 0..r.points.len();
                RingSlot {
                    points: &mut r.points,
                    range,
                }
            })
            .collect();
        let added = add_legs(&corners, &mut slots, cut)?;
        unreached.extend(added.unreached);
        warnings.extend(added.warnings);
    }
    Ok((unreached, warnings))
}
