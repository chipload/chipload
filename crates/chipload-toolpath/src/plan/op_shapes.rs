// SPDX-License-Identifier: GPL-3.0-or-later
//! Planning follow paths, chamfers, flutes, mouldings, prisms, patterns and surfaces.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_i18n::{Message, Value};
use chipload_job::{
    ChamferOp, Entry, FluteOp, MouldingOp, Operation, PathOp, PatternOp, PrismOp, Side, SurfaceOp,
    ToolType,
};

use super::check_tools::short_ramp_refusal;
use super::{Context, Planned, job_error};
use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::path::{PathCut, order_paths, path_moves};
use crate::profile::{contour_context, cut_clockwise, labelled_note, pass_depths};
use crate::select::Selection;
use crate::surface::{SurfaceCut, surface_moves};

pub(super) fn plan_path_op(
    op: &PathOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let mut arcs = ArcTable::default();
    let mut notes = Vec::new();
    let paths = order_paths(&selection.contours, start, ctx.sense, &mut arcs, &mut notes);
    let mut notes: Vec<Message> = notes
        .into_iter()
        .map(|n| labelled_note(ctx.label_message, n))
        .collect();
    // A V-bit without an angle: its groove's width can't be told.
    if ctx.tool.kind == ToolType::Vbit && ctx.tool.angle.is_none() {
        let note = Message::new("toolpath-groove-no-angle").arg("tool", ctx.tool.id);
        notes.push(labelled_note(ctx.label_message, note));
    }
    let cut = PathCut {
        pass_z: ctx.pass_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        entry: op.entry,
        tool_diameter: ctx.tool.diameter,
        spindle: ctx.sense,
    };
    let mut moves = Vec::new();
    let mut end = start;
    if op.entry.ramp_angle().is_some() {
        for path in &paths {
            let length = if path.closed {
                crate::ramp::polyline_length(&crate::profile::closed_path(&path.points))
            } else {
                crate::ramp::polyline_length(&path.points)
            };
            short_ramp_refusal(ctx.tool, length).map_err(|why| {
                job_error(labelled_note(
                    &contour_context(ctx.label_message, path.contour_id),
                    why,
                ))
            })?;
        }
    }
    for path in &paths {
        ctx.cancel.check()?;
        let (more, at) = path_moves(path, &cut, &arcs)
            .map_err(|err| err.with_context(contour_context(ctx.label_message, path.contour_id)))?;
        moves.extend(more);
        end = at;
    }
    Ok(Planned {
        moves,
        notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs,
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    })
}

/// A chamfer (docs/specs/chamfer.md): each closed shape's line offset half the V-bit's tip
/// into the waste, followed at every depth pass as a closed path.
pub(super) fn plan_chamfer_op(
    op: &ChamferOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let tip = ctx.tool.tip_diameter.unwrap_or(0.0);
    let rings = crate::chamfer::chamfer_rings(
        op,
        ctx.label_message,
        &selection.contours,
        tip,
        ctx.sense,
        start,
    )?;
    let cut = PathCut {
        pass_z: ctx.pass_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        entry: op.entry,
        tool_diameter: ctx.tool.diameter,
        spindle: ctx.sense,
    };
    if op.entry.ramp_angle().is_some() {
        for path in &rings.paths {
            let length = crate::ramp::polyline_length(&crate::profile::closed_path(&path.points));
            short_ramp_refusal(ctx.tool, length).map_err(|why| {
                job_error(labelled_note(
                    &contour_context(ctx.label_message, path.contour_id),
                    why,
                ))
            })?;
        }
    }
    let mut moves = Vec::new();
    let mut end = start;
    for path in &rings.paths {
        ctx.cancel.check()?;
        let (more, at) = path_moves(path, &cut, &rings.arcs)
            .map_err(|err| err.with_context(contour_context(ctx.label_message, path.contour_id)))?;
        moves.extend(more);
        end = at;
    }
    Ok(Planned {
        moves,
        notes: rings.notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: rings.arcs,
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    })
}

/// A flute (docs/specs/fluting.md): each open shape cut back and forth, every pass its
/// profile cut off at the pass's level.
pub(super) fn plan_flute_op(
    op: &FluteOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let mut arcs = ArcTable::default();
    let mut found = Vec::new();
    let depth = op.end_depth - op.start_depth;
    let levels: Vec<f64> = ctx.pass_z.iter().map(|z| ctx.top_z - z).collect();
    let flutes = crate::flute::flutes(
        op,
        depth,
        &levels,
        &selection.contours,
        &mut arcs,
        &mut found,
    );
    let notes = found
        .into_iter()
        .map(|n| labelled_note(ctx.label_message, n))
        .collect();
    let cut = crate::flute::FluteCut {
        pass_z: ctx.pass_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
    };
    let mut moves = Vec::new();
    let mut end = start;
    for (flute, reverse) in crate::flute::order(flutes, start) {
        ctx.cancel.check()?;
        let (more, at) = crate::flute::flute_moves(&flute, reverse, &cut, &arcs);
        moves.extend(more);
        end = at;
    }
    Ok(Planned {
        moves,
        notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs,
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    })
}

/// A moulding (docs/specs/moulding.md): the section from its profile (a drawn one from
/// the project), passes along each path, in levels to its deepest.
pub(super) fn plan_moulding_op(
    op: &MouldingOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let label = ctx.label_message;
    let Some(cutter) = ctx.tool.cutter() else {
        return Err(job_error(
            Message::new("toolpath-relief-shape").arg("tool", ctx.tool.id),
        ));
    };
    let section = crate::moulding::Section::of(&op.profile, ctx.job)
        .map_err(|err| err.with_context(label.clone()))?;
    // A drawn profile's depth is known only here: below the stock beyond its allowance is
    // refused, as any operation's end depth is.
    let as_op = Operation::Moulding(op.clone());
    let stock = ctx
        .job
        .setup_of(&as_op)
        .map_or(ctx.job.stock(), |s| &s.stock);
    let bottom = op.start_depth + section.deepest();
    if bottom > stock.thickness + stock.max_overcut + DEPTH_TOL_MM {
        return Err(job_error(labelled_note(
            label,
            Message::new("toolpath-moulding-below-stock")
                .arg("depth", Value::mm(bottom))
                .arg("thickness", Value::mm(stock.thickness)),
        )));
    }
    let mut arcs = ArcTable::default();
    // The profile's own shape is never a path.
    let paths: Vec<&Contour> = selection
        .contours
        .iter()
        .copied()
        .filter(|c| match op.profile {
            chipload_job::MouldProfile::Shape { shape } => {
                !c.parts.iter().any(|p| p.segment.shape == Some(shape))
            }
            _ => true,
        })
        .collect();
    let (passes, found) = crate::moulding::passes(op, &section, cutter, &paths, &mut arcs)
        .map_err(|err| err.with_context(label.clone()))?;
    let notes = found.into_iter().map(|n| labelled_note(label, n)).collect();
    let deepest = -passes.iter().map(|p| p.low).fold(0.0, f64::min);
    let mut facts = Vec::new();
    let mut moves = Vec::new();
    let mut end = start;
    if deepest > DEPTH_TOL_MM {
        facts.push(labelled_note(
            label,
            Message::new("toolpath-moulding-deepest")
                .arg("depth", Value::mm(section.deepest()))
                .arg("passes", passes.len()),
        ));
        let levels = match op.step_down {
            Some(step) => pass_depths(0.0, deepest, step)?,
            None => vec![deepest],
        };
        let side = match op.side {
            chipload_job::MouldSide::Inside | chipload_job::MouldSide::Left => Side::Inside,
            chipload_job::MouldSide::Outside | chipload_job::MouldSide::Right => Side::Outside,
        };
        // Open paths: left of the path is as inside a counter-clockwise ring.
        let ccw = !cut_clockwise(Side::Inside, op.direction, ctx.sense);
        let forward = match op.side {
            chipload_job::MouldSide::Left | chipload_job::MouldSide::Inside => ccw,
            chipload_job::MouldSide::Right | chipload_job::MouldSide::Outside => !ccw,
        };
        let cut = crate::moulding::MouldCut {
            cutter,
            levels: &levels,
            top_z: ctx.top_z,
            safe_z: ctx.job.machine.safe_z,
            feed: op.feed,
            plunge_feed: op.plunge_feed,
            forward,
            clockwise: cut_clockwise(side, op.direction, ctx.sense),
        };
        ctx.cancel.check()?;
        let (more, at) = crate::moulding::moulding_moves(&passes, &cut, &arcs);
        moves = more;
        end = at.unwrap_or(start);
    }
    Ok(Planned {
        moves,
        notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs,
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts,
    })
}

/// A prism (docs/specs/prism.md): each outline cut as a ring as deep as the shape allows,
/// in passes to its deepest edge.
pub(super) fn plan_prism_op(
    op: &PrismOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let tool = ctx.tool;
    let (Some(reach), Some(per_mm)) = (tool.widest_chamfer(), tool.chamfer_depth_for(1.0)) else {
        // Refused by the tool rule before planning.
        return Err(job_error(
            Message::new("toolpath-vbit-no-angle").arg("tool", tool.id),
        ));
    };
    let bit = crate::prism::PrismBit {
        tan: 1.0 / per_mm,
        tip: tool.tip_diameter.unwrap_or(0.0),
        reach,
    };
    let found =
        crate::prism::prism_rings(op, ctx.label_message, &selection.contours, bit, ctx.sense)?;
    let notes = found
        .notes
        .iter()
        .map(|n| labelled_note(ctx.label_message, n.clone()))
        .collect();
    // How deep it goes, below the stock only within its allowance.
    let as_op = Operation::Prism(op.clone());
    let stock = ctx
        .job
        .setup_of(&as_op)
        .map_or(ctx.job.stock(), |s| &s.stock);
    let bottom = op.start_depth + found.deepest;
    if bottom > stock.thickness + stock.max_overcut + DEPTH_TOL_MM {
        return Err(job_error(labelled_note(
            ctx.label_message,
            Message::new("toolpath-prism-below-stock")
                .arg("depth", Value::mm(bottom))
                .arg("thickness", Value::mm(stock.thickness)),
        )));
    }
    let mut facts = Vec::new();
    let mut moves = Vec::new();
    let mut end = start;
    if found.deepest > DEPTH_TOL_MM {
        facts.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-prism-deepest").arg("depth", Value::mm(found.deepest)),
        ));
        let levels = match op.step_down {
            Some(step) => pass_depths(0.0, found.deepest, step)?,
            None => vec![found.deepest],
        };
        let cut = crate::prism::PrismCut {
            levels: &levels,
            top_z: ctx.top_z,
            safe_z: ctx.job.machine.safe_z,
            feed: op.feed,
            plunge_feed: op.plunge_feed,
        };
        for ring in crate::prism::order(found.rings, start) {
            ctx.cancel.check()?;
            let (more, at) = crate::prism::prism_moves(&ring, &cut, &found.arcs);
            moves.extend(more);
            end = at;
        }
    }
    Ok(Planned {
        moves,
        notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: found.arcs,
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts,
    })
}

/// A fill pattern (docs/specs/fill-pattern.md): its grooves cut as flutes, or its rings as
/// a follow path's closed rings; laid out from the drawing's origin moved by `shift`.
pub(super) fn plan_pattern_op(
    op: &PatternOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    shift: (f64, f64),
) -> Result<Planned, ToolpathError> {
    let depth = op.end_depth - op.start_depth;
    let levels: Vec<f64> = ctx.pass_z.iter().map(|z| ctx.top_z - z).collect();
    let width = crate::flute::flute_width_mm(ctx.tool, depth).unwrap_or(ctx.tool.diameter);
    let setup = crate::pattern::PatternSetup {
        width,
        depth,
        levels: &levels,
        anchor: shift,
        spindle: ctx.sense,
    };
    let found = crate::pattern::grooves(op, &selection.contours, &setup)
        .map_err(|err| err.with_context(ctx.label_message.clone()))?;
    let notes = found
        .notes
        .into_iter()
        .map(|n| labelled_note(ctx.label_message, n))
        .collect();
    let mut moves = Vec::new();
    let mut end = start;
    let flute_cut = crate::flute::FluteCut {
        pass_z: ctx.pass_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
    };
    for (groove, reverse) in crate::flute::order(found.open, start) {
        ctx.cancel.check()?;
        let (more, at) = crate::flute::flute_moves(&groove, reverse, &flute_cut, &found.arcs);
        moves.extend(more);
        end = at;
    }
    let ring_cut = PathCut {
        pass_z: ctx.pass_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        entry: Entry::Plunge {},
        tool_diameter: ctx.tool.diameter,
        spindle: ctx.sense,
    };
    for ring in crate::pattern::order_rings(found.rings, end) {
        ctx.cancel.check()?;
        let (more, at) = path_moves(&ring, &ring_cut, &found.arcs)
            .map_err(|err| err.with_context(contour_context(ctx.label_message, ring.contour_id)))?;
        moves.extend(more);
        end = at;
    }
    Ok(Planned {
        moves,
        notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: found.arcs,
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    })
}

/// A surface (docs/specs/surfacing.md): the whole stock, back and forth along its longer
/// side, every depth started beside it.
pub(super) fn plan_surface_op(op: &SurfaceOp, ctx: &Context, start: Point) -> Planned {
    let cut = SurfaceCut {
        stock: ctx.job.stock_bounds(),
        tool_radius: ctx.tool.diameter / 2.0,
        stepover: op.stepover * ctx.tool.diameter,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
    };
    let (moves, end) = surface_moves(&cut, ctx.pass_z, start);
    Planned {
        moves,
        notes: Vec::new(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: ArcTable::default(),
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    }
}
