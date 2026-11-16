// SPDX-License-Identifier: GPL-3.0-or-later
//! Planning reliefs (pictures and models).

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::model::Move;
use crate::profile::labelled_note;
use crate::rim::{Area, Ramp};
use crate::select::Selection;
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_i18n::{Message, Value};
use chipload_job::{Entry, Job, Note, OpRef, Problem, Side, Tool, ToolType};

use super::check::labelled;
use super::check_tools::plunge_unknown;
use super::{Context, Planned, job_error};

/// A relief (docs/specs/stl-relief.md §5, §7): its model placed and indexed, finishing lines
/// across the area, and the path checked against the model.
pub(super) fn plan_relief_op(
    op: &chipload_job::ReliefOp,
    ctx: &Context,
    sel: &Selection,
    start: Point,
    shift: Point,
) -> Result<Planned, ToolpathError> {
    let model = ctx
        .job
        .drawing
        .models
        .iter()
        .find(|m| m.id == op.model)
        .ok_or_else(|| job_error(Message::new("job-no-model").arg("id", op.model)))?;
    let cutter = ctx
        .tool
        .cutter()
        .ok_or_else(|| job_error(Message::new("toolpath-relief-shape").arg("tool", ctx.tool.id)))?;
    let mesh = crate::relief::placed_mesh(model).map_err(|e| job_error(e.message()))?;
    if sel.contours.iter().all(|c| !c.closed) {
        return Err(job_error(Message::new("toolpath-relief-no-area")));
    }
    // A pass of the relief: its smoothing pass (no levels) or its roughing pass; the
    // resolved pass carries only its own.
    if let Some(finish) = ctx.finish_tool {
        let pass = match (op.smoothing, op.roughing) {
            (Some(smooth), _) => Some((smooth.allowance, None)),
            (None, Some(rough)) => Some((rough.allowance, Some(rough.step_down))),
            (None, None) => None,
        };
        if let Some((allowance, step_down)) = pass {
            let pass = ReliefPass {
                finish,
                allowance,
                step_down,
            };
            return plan_relief_pass(op, ctx, sel, start, shift, &mesh, cutter, pass);
        }
    }
    let index = crate::drop::DropMesh::new(&mesh, ctx.tool.diameter);
    let (start_depth, end_depth) = (op.start_depth, op.end_depth);
    let floor = ctx.top_z - (end_depth - start_depth);
    let stock_top = ctx.top_z + start_depth;
    let (area, ramp) = relief_area(&sel.contours, op.rim.as_ref(), floor, stock_top)?;
    let notes = past_the_board(ctx, &area).into_iter().collect();
    let cut = crate::relief::ReliefCut {
        mesh: &index,
        cutter,
        top: ctx.top_z,
        floor,
        area: &area,
        ramp: ramp.as_ref(),
        step: op.stepover * ctx.tool.diameter,
        angle: op.angle,
        zigzag: op.zigzag,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        shift,
        progress: Some(ctx.cancel),
    };
    let (moves, end) = crate::relief::relief_moves(&cut, start);
    ctx.cancel.check()?;
    if let Some((depth, at)) = crate::relief::verify(&moves, &mesh, cutter, ctx.top_z, shift) {
        return Err(job_error(
            Message::new("toolpath-relief-gouge")
                .arg("depth", Value::mm(depth))
                .arg("x", Value::mm(at.0))
                .arg("y", Value::mm(at.1)),
        ));
    }
    ramp_checked(&moves, ramp.as_ref(), cutter)?;
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

/// What a relief's pass leaves and how it goes down: the finish's tool, the allowance, and
/// the step down between levels for a roughing pass (None: the surface followed once, a
/// smoothing pass).
#[derive(Clone, Copy)]
struct ReliefPass<'a> {
    finish: &'a Tool,
    allowance: f64,
    step_down: Option<f64>,
}

/// A relief's roughing pass (docs/specs/stl-relief.md §14 step 3) or smoothing pass (§17):
/// the tool grown by the allowance, its area shrunk so it never cuts wider than the finish,
/// level by level or the surface followed once, and the path checked to keep the
/// allowance.
#[allow(clippy::too_many_arguments)]
fn plan_relief_pass(
    op: &chipload_job::ReliefOp,
    ctx: &Context,
    sel: &Selection,
    start: Point,
    shift: Point,
    mesh: &chipload_mesh::Mesh,
    cutter: chipload_core::cutter::Cutter,
    pass: ReliefPass,
) -> Result<Planned, ToolpathError> {
    let (finish, allowance) = (pass.finish, pass.allowance);
    let grown = cutter.grown(allowance).ok_or_else(|| {
        job_error(Message::new("toolpath-relief-roughing-tool").arg("tool", ctx.tool.id))
    })?;
    let index = crate::drop::DropMesh::new(mesh, 2.0 * grown.radius());
    let inset = cutter.radius() + allowance - finish.diameter / 2.0;
    let floor = ctx.top_z - (op.end_depth - op.start_depth) + allowance;
    let stock_top = ctx.top_z + op.start_depth;
    let room = || job_error(Message::new("toolpath-relief-roughing-room").arg("tool", ctx.tool.id));
    // With a rim, the grown area shrunk and the ramp raised by the allowance (spec §16).
    let (area, ramp) = match op.rim.as_ref() {
        Some(rim) => {
            let (area, ramp) = relief_area(&sel.contours, Some(rim), floor, stock_top + allowance)?;
            let area = if inset > DEPTH_TOL_MM {
                area.shrunk(inset).ok_or_else(room)?
            } else {
                area
            };
            (area, ramp)
        }
        None => {
            let loops = roughing_area(&sel.contours, inset)?;
            if loops.is_empty() {
                return Err(room());
            }
            (Area::Loops(loops), None)
        }
    };
    let cut = crate::relief::ReliefCut {
        mesh: &index,
        cutter: grown,
        top: ctx.top_z + allowance,
        floor,
        area: &area,
        ramp: ramp.as_ref(),
        step: op.stepover * ctx.tool.diameter,
        angle: op.angle,
        zigzag: op.zigzag,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        shift,
        progress: Some(ctx.cancel),
    };
    let (moves, end) = match pass.step_down {
        Some(step_down) => crate::relief::rough_moves(&cut, stock_top, step_down, start),
        None => crate::relief::relief_moves(&cut, start),
    };
    if let Some((depth, at)) =
        crate::relief::verify(&moves, mesh, grown, ctx.top_z + allowance, shift)
    {
        return Err(job_error(
            Message::new("toolpath-relief-gouge")
                .arg("depth", Value::mm(depth))
                .arg("x", Value::mm(at.0))
                .arg("y", Value::mm(at.1)),
        ));
    }
    ramp_checked(&moves, ramp.as_ref(), grown)?;
    // Said once, of the roughing pass: a smoothing pass usually has the same tool.
    let notes = op
        .rim
        .as_ref()
        .filter(|_| pass.step_down.is_some())
        .and_then(|rim| rim_too_narrow(ctx, rim, grown))
        .into_iter()
        .collect();
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

/// A relief's area and ramp (docs/specs/stl-relief.md §16) from its selected contours: the
/// closed ones as they are without a rim. With one, its flat round the outer loops (or the
/// rectangle round them) at `floor`, the ramp up to `top`, and the area out to where the
/// ramp reaches the top.
fn relief_area(
    contours: &[&Contour],
    rim: Option<&chipload_job::ReliefRim>,
    floor: f64,
    top: f64,
) -> Result<(Area, Option<Ramp>), ToolpathError> {
    let Some(rim) = rim else {
        let loops: Vec<Vec<Point>> = contours
            .iter()
            .filter(|c| c.closed)
            .map(|c| c.points())
            .collect();
        return Ok((Area::Loops(loops), None));
    };
    // Holes are cut to the floor; the rim goes round the outer loops.
    let (loops, foot) = crate::rim::rim_base(&crate::rim::outer_loops(contours), rim)?;
    let ramp = Ramp {
        loops: loops.clone(),
        foot,
        tan: chipload_core::math::tan(rim.ramp),
        floor,
        top,
    };
    let area = Area::Grown {
        loops,
        reach: ramp.reach(),
    };
    Ok((area, Some(ramp)))
}

/// A note when a relief's area reaches past the board: the ramp is cut off at its edge
/// (docs/specs/stl-relief.md §16).
fn past_the_board(ctx: &Context, area: &Area) -> Option<Message> {
    if !matches!(area, Area::Grown { .. }) {
        return None;
    }
    let (lo, hi) = area.bounds()?;
    let ((x0, y0), (x1, y1)) = ctx.job.stock_bounds();
    let by = [x0 - lo.0, y0 - lo.1, hi.0 - x1, hi.1 - y1]
        .into_iter()
        .fold(0.0, f64::max);
    (by > CHAIN_TOL_MM).then(|| {
        labelled_note(
            ctx.label_message,
            Message::new("toolpath-relief-past-board").arg("by", Value::mm(by)),
        )
    })
}

/// A note when a rim's flat is narrower than the roughing tool, grown by its allowance: round
/// the model it can't reach the floor (docs/specs/stl-relief.md §16.6).
fn rim_too_narrow(
    ctx: &Context,
    rim: &chipload_job::ReliefRim,
    grown: chipload_core::cutter::Cutter,
) -> Option<Message> {
    let need = 2.0 * grown.radius();
    (rim.width + CHAIN_TOL_MM < need).then(|| {
        labelled_note(
            ctx.label_message,
            Message::new("toolpath-relief-rim-narrow")
                .arg("width", Value::mm(rim.width))
                .arg("tool", ctx.tool.id)
                .arg("need", Value::mm(need)),
        )
    })
}

/// The path checked against the rim's ramp, if any (docs/specs/stl-relief.md §16).
fn ramp_checked(
    moves: &[Move],
    ramp: Option<&Ramp>,
    cutter: chipload_core::cutter::Cutter,
) -> Result<(), ToolpathError> {
    let Some((depth, at)) = ramp.and_then(|r| crate::relief::verify_ramp(moves, r, cutter)) else {
        return Ok(());
    };
    Err(job_error(
        Message::new("toolpath-relief-ramp-gouge")
            .arg("depth", Value::mm(depth))
            .arg("x", Value::mm(at.0))
            .arg("y", Value::mm(at.1)),
    ))
}

/// A relief's area for its roughing tool: the closed contours moved `inset` into the area
/// (holes, inside an odd number of others, grown), as loops of points. Unchanged when
/// `inset` is not more than 0. Empty when the tool has no room.
fn roughing_area(contours: &[&Contour], inset: f64) -> Result<Vec<Vec<Point>>, ToolpathError> {
    let closed: Vec<&Contour> = contours.iter().copied().filter(|c| c.closed).collect();
    if inset <= DEPTH_TOL_MM {
        return Ok(closed.iter().map(|c| c.points()).collect());
    }
    let mut area = Vec::new();
    for (i, c) in closed.iter().enumerate() {
        let Some(&p) = c.points().first() else {
            continue;
        };
        let around = closed
            .iter()
            .enumerate()
            .filter(|&(j, other)| j != i && crate::relief::encloses(&other.points(), p))
            .count();
        let side = if around % 2 == 1 {
            Side::Outside
        } else {
            Side::Inside
        };
        match crate::offset::offset_rings(c, side, inset) {
            Ok(rings) => area.extend(rings),
            // An outline the tool doesn't fit leaves nothing of itself to rough.
            Err(_) if side == Side::Inside => {}
            Err(err) => return Err(err),
        }
    }
    Ok(area)
}

/// A relief's roughing pass (docs/specs/stl-relief.md §14 step 3) or smoothing pass (§17),
/// pass `k` in `field`: it cuts with a project end mill or plain ball nose with a spindle
/// speed.
pub(super) fn relief_pass_checks(
    job: &Job,
    index: usize,
    (k, field): (usize, &str),
    (tool_id, allowance): (i64, f64),
) -> (Vec<Problem>, Vec<Note>) {
    let (mut problems, mut notes) = (Vec::new(), Vec::new());
    let label_message = job.label_message(OpRef::pass(index, k));
    let at = |name: &str| format!("operations.{index}.{field}.{name}");
    let Some(tool) = job.tool(tool_id) else {
        problems.push(labelled(
            at("tool"),
            &label_message,
            Message::new("toolpath-rest-pass-no-tool").arg("tool", tool_id),
        ));
        return (problems, notes);
    };
    if tool.cutter().and_then(|c| c.grown(allowance)).is_none()
        || !matches!(tool.kind, ToolType::Endmill | ToolType::Ballnose)
    {
        problems.push(labelled(
            at("tool"),
            &label_message,
            Message::new("toolpath-relief-roughing-tool").arg("tool", tool.id),
        ));
    }
    if tool.rpm.is_none_or(|rpm| rpm <= 0.0) {
        problems.push(labelled(
            at("tool"),
            &label_message,
            Message::new("toolpath-rest-pass-no-rpm").arg("tool", tool.id),
        ));
    }
    if tool.kind == ToolType::Endmill {
        match plunge_unknown(tool, Entry::Plunge {}) {
            Ok(true) => notes.push(Note::at(
                index,
                labelled_note(
                    &label_message,
                    Message::new("toolpath-rest-pass-plunge").arg("tool", tool.id),
                ),
            )),
            Ok(false) => {}
            Err(short) => problems.push(labelled(at("tool"), &label_message, short)),
        }
    }
    (problems, notes)
}
