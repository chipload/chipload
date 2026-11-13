// SPDX-License-Identifier: GPL-3.0-or-later
//! Job + imported contours -> Program: the toolpath half of `chipload_pipeline::run_job`,
//! which then posts and independently checks it.

use std::collections::{BTreeMap, BTreeSet};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Line, Primitive};
use chipload_core::text::mm;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    ClearingPass, Copies, CopyOrder, Entry, HoleMethod, HolesOp, Job, MAX_LEAD_DIAMETERS, Mirror,
    Note, OpRef, Operation, Part, PathOp, PocketOp, Problem, ProfileOp, RestRefusal, RimShape,
    Side, Spindle, SurfaceOp, Tabs, Tool, ToolType, VcarveOp, ZZero, rest_refusal,
};

use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM, DEPTH_TOL_MM, FLOOR_LEFT_MM};

use crate::ToolpathError;
use crate::arcs::{ArcTable, tag_moves};
use crate::cancel::Cancel;
use crate::corners::{
    CornerCut, RingSlot, add_legs, find_corners, pocket_boundaries, pocket_corners,
    unmatched_choice, unreached_note,
};
use crate::geom::PrimIndex;
use crate::holes::{
    Bore, DRILL_FIT_MM, HoleCut, bore_moves, bore_radius, drill_moves, find_holes, order_holes,
};
use crate::lap::{RingCut, TabSurface, WallLap, carry_tabs, farthest_from, ring_moves, wall_lap};
use crate::lead::MIN_LEAD_DIAMETERS;
use crate::model::{Move, MoveKind, Program};
use crate::path::{PathCut, order_paths, path_moves};
use crate::pocket::{
    Choices, PocketCut, PocketPiece, PocketRequest, PocketRing, plan_pocket, pocket_moves,
};
use crate::profile::{
    PathRing, contour_context, contour_note, labelled_note, pass_depths, plan_profile,
};
use crate::rim::{Area, Ramp};
use crate::select::{Selection, select_contours, select_paths};
use crate::surface::{SurfaceCut, surface_moves};
use crate::tabs::{
    TabLayout, along_tabs, arc_position, place_tabs, point_at_arc, tab_polylines, tabs_at,
};
use crate::vcarve::{VBit, carve_levels, plan_vcarve_at, vcarve_moves};

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub program: Program,
    /// Warnings for the user, each with the operation it is about.
    pub notes: Vec<Note>,
    /// Facts worth knowing that need no care, such as a V-groove's width: shown as plain
    /// notes, not warnings.
    pub facts: Vec<Note>,
    /// Tool-path stretch of every tab, for display.
    pub tabs: Vec<Vec<Point>>,
    /// Every pocket piece in cut order (rings and links), for display and conformance tests.
    pub pockets: Vec<PocketPiece>,
    /// Which moves of `program` each operation made, in cut order, for display.
    pub op_spans: Vec<OpSpan>,
    /// Every ring whose start the user can move (on the part, copy 1), for the start
    /// markers (docs/specs/helix-and-leads.md part A).
    pub ring_starts: Vec<RingStart>,
    /// Corners that would get a bone but the tool can't reach (on the part, copy 1): the
    /// operation's index and the corner, for the corner markers
    /// (docs/specs/dogbone-per-corner.md §6).
    pub unreached_corners: Vec<(usize, Point)>,
    /// Every profile tab's centre on its ring (on the part, copy 1), for the tab markers
    /// and `op tabs` (docs/specs/tabs-by-hand.md).
    pub tab_centres: Vec<TabCentre>,
    /// V-carves whose floor pass leaves part of the floor: the operation and the largest end
    /// mill diameter that would clear it all, mm (docs/specs/vcarve-wide-strokes.md §4.1).
    pub floor_hints: Vec<(usize, f64)>,
    /// The tool groups in cut order (docs/specs/tool-per-operation.md §2): one per tool
    /// change, each a program of its own or a part of one. A one-tool job has one.
    pub groups: Vec<ToolGroup>,
}

/// Operations (and rest passes) next to each other in cut order that use the same tool, and
/// the moves they made.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolGroup {
    pub tool: Tool,
    /// The operations and rest passes, in cut order (docs/specs/rest-passes.md §3).
    pub operations: Vec<OpRef>,
    pub moves: std::ops::Range<usize>,
}

impl ToolGroup {
    /// The group's moves as a program of their own: its tool, the rest from `program`.
    pub fn program(&self, program: &Program) -> Program {
        Program {
            tool_id: self.tool.id,
            tool_diameter: self.tool.diameter,
            rpm: self.tool.rpm.unwrap_or(0.0),
            spindle: program.spindle,
            safe_z: program.safe_z,
            moves: program.moves[self.moves.clone()].to_vec(),
            comments: program.comments.clone(),
        }
    }
}

/// A ring whose start can be moved: profile rings and pocket wall rings.
#[derive(Debug, Clone, PartialEq)]
pub struct RingStart {
    /// Index in `job.operations`.
    pub operation: usize,
    /// The tool-centre ring in cut direction, starting at its start.
    pub ring: Vec<Point>,
    /// Index in the operation's `starts` of the point that set it; None: the planner's.
    pub stored: Option<usize>,
}

/// A profile tab's centre on its ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabCentre {
    /// Index in `job.operations`.
    pub operation: usize,
    /// Which of the operation's rings, in the order of its ring starts.
    pub ring: usize,
    pub at: Point,
    /// Placed by hand (`tabs.at`), not spaced by Chipload.
    pub by_hand: bool,
}

/// The moves one operation contributed to the program.
#[derive(Debug, Clone, PartialEq)]
pub struct OpSpan {
    /// Index in `job.operations`.
    pub operation: usize,
    /// Which of its pocket's rest passes; None: the operation itself
    /// (docs/specs/rest-passes.md).
    pub pass: Option<usize>,
    /// Which copy of the part (docs/specs/copies-grid.md): 1 is the drawing as drawn.
    pub copy: u32,
    pub moves: std::ops::Range<usize>,
    /// Of the operation's tool, mm.
    pub tool_radius: f64,
}

/// How far above the stock top a rapid goes down before a plunge (docs/specs/approach.md):
/// the air above it is crossed at rapid, the rest at the plunge feed.
pub const APPROACH_CLEARANCE_MM: f64 = 1.0;

pub fn plan_job(job: &Job, contours: &[Contour]) -> Result<Plan, ToolpathError> {
    plan_job_cancellable(job, contours, &Cancel::new())
}

/// [`plan_job`] that stops with [`ToolpathError::Cancelled`] once `cancel` is set
/// (docs/specs/background-generation.md). A run that isn't cancelled gives the same plan.
pub fn plan_job_cancellable(
    job: &Job,
    contours: &[Contour],
    cancel: &Cancel,
) -> Result<Plan, ToolpathError> {
    // One program is one setup's (docs/specs/setups.md §4): the active one.
    let one = job.active_setup_id().map(|id| job.for_setup(id));
    let job = one.as_ref().unwrap_or(job);
    let mut notes = validate_job(job)?;
    let chosen = select_all(job, contours)?;
    // Construction geometry is never cut and never counts (as an island, a wall a dogbone
    // must keep clear of, …); selection above already refused naming it.
    let cutting: Vec<Contour> = contours
        .iter()
        .filter(|c| !c.is_construction())
        .cloned()
        .collect();
    // Every contour some operation cuts (docs/specs/dogbones.md §6), and by which part.
    let cut_ids: BTreeSet<usize> = chosen
        .iter()
        .flat_map(|c| c.selection.contours.iter().map(|k| k.id))
        .collect();
    let mut cut_by_part: BTreeMap<u64, BTreeSet<usize>> = BTreeMap::new();
    for c in &chosen {
        cut_by_part
            .entry(c.op.part())
            .or_default()
            .extend(c.selection.contours.iter().map(|k| k.id));
    }
    let cut = CutContours {
        ids: &cut_ids,
        by_part: &cut_by_part,
    };
    let (steps, mut groups) = cut_order(job, &chosen)?;
    let mut gathered = Gathered::default();
    let mut facts = Vec::new();
    let mut position: Point = (0.0, 0.0);
    for step in &steps {
        cancel.check()?;
        let Chosen {
            at,
            op,
            selection,
            label_message: label,
            ..
        } = &chosen[step.chosen];
        let group = &mut groups[step.group];
        if group.moves.is_empty() {
            group.moves = gathered.moves.len()..gathered.moves.len();
        }
        let tool = op_tool(job, op).map_err(|err| err.at(at.operation))?;
        if !gathered.started.contains(at) {
            gathered.started.push(*at);
            if let Operation::Path(p) = op
                && let Some(width) = groove_width(tool, p.end_depth - p.start_depth)
            {
                facts.push(Note::at(at.operation, labelled_note(label, width)));
            }
            if let Some(below) = through_overcut(job, op) {
                facts.push(Note::at(at.operation, labelled_note(label, below)));
            }
            notes.extend(
                selection
                    .notes
                    .iter()
                    .map(|n| Note::at(at.operation, labelled_note(label, n.clone()))),
            );
        }
        let planned = plan_step(
            job,
            &cutting,
            cancel,
            &chosen[step.chosen],
            step,
            position,
            &cut,
        )?;
        position = planned.end;
        gathered.add(job, step, *at, tool.diameter / 2.0, planned, &mut notes);
        groups[step.group].moves.end = gathered.moves.len();
    }
    let Gathered {
        mut moves,
        facts: op_facts,
        tabs,
        pockets,
        op_spans,
        ring_starts,
        unreached_corners,
        tab_centres,
        floor_hints,
        arcs,
        comments,
        ..
    } = gathered;
    check_parts_apart(job, &moves, &op_spans)?;
    facts.extend(op_facts);
    let first = groups
        .first()
        .ok_or_else(|| job_error(Message::new("toolpath-all-disabled")))?;
    // Chords of known arcs are tagged; the post writes them as G2/G3 if it does arcs.
    tag_moves(&mut moves, &arcs);
    Ok(Plan {
        program: Program {
            tool_id: first.tool.id,
            tool_diameter: first.tool.diameter,
            rpm: first.tool.rpm.unwrap_or(0.0),
            spindle: job.machine.spindle,
            safe_z: job.machine.safe_z,
            moves,
            comments,
        },
        notes,
        facts,
        tabs,
        pockets,
        op_spans,
        ring_starts,
        unreached_corners,
        tab_centres,
        floor_hints,
        groups,
    })
}

/// An operation or rest pass that is cut, with its selection, chosen once on the drawing as
/// drawn.
struct Chosen<'a> {
    at: OpRef,
    /// The operation; for a rest pass, the pocket it is (Job::resolve).
    op: Operation,
    label_message: Message,
    selection: Selection<'a>,
}

/// The contours each cut operation and rest pass of the active setup selects, in cut order:
/// all that planning reads of the drawing (the contours some operation cuts are these
/// together). For telling whether a change to the drawing can change a plan
/// (docs/specs/edit-and-simulate.md §5). A back setup's run reads its front's too: it is
/// simulated on the front's result and checked against its dowel pins
/// (docs/specs/two-sided.md §6, §10), so a front's selections follow the back's.
pub fn selections(job: &Job, contours: &[Contour]) -> Result<Vec<Vec<Contour>>, ToolpathError> {
    let Some(active) = job.active_setup_id() else {
        return selections_of(job, contours);
    };
    let mut all = selections_of(&job.for_setup(active), contours)?;
    let front = job
        .setup(active)
        .and_then(|s| s.back_of)
        .and(job.front_of(active));
    if let Some(front) = front {
        all.extend(selections_of(&job.for_setup(front.id), contours)?);
    }
    Ok(all)
}

fn selections_of(job: &Job, contours: &[Contour]) -> Result<Vec<Vec<Contour>>, ToolpathError> {
    Ok(select_all(job, contours)?
        .into_iter()
        .map(|chosen| chosen.selection.contours.into_iter().cloned().collect())
        .collect())
}

/// Every cut operation's and rest pass's selection (docs/specs/v0-profile-pipeline.md §3),
/// in the job's cut list (docs/specs/rest-passes.md §3).
fn select_all<'a>(job: &Job, contours: &'a [Contour]) -> Result<Vec<Chosen<'a>>, ToolpathError> {
    let mut chosen = Vec::new();
    // Operations that aren't cut are kept in the file and skipped here; numbering stays as
    // listed.
    for at in job.cut_list() {
        let op = job.resolve(at).ok_or_else(|| {
            job_error(Message::new("toolpath-error-no-operation").arg("at", at.to_string()))
        })?;
        let label_message = job.label_message(at);
        let selection = match op {
            // Holes take every matching shape and skip what isn't a circle with a note.
            Operation::Path(_) | Operation::Holes(_) => select_paths(
                contours,
                &op.select().resolved(&job.drawing),
                &label_message,
            ),
            // The whole stock, no shapes (docs/specs/surfacing.md §2).
            Operation::Surface(_) => Ok(Selection {
                contours: Vec::new(),
                notes: Vec::new(),
            }),
            _ => select_contours(
                contours,
                &op.select().resolved(&job.drawing),
                &label_message,
            ),
        }
        .map_err(|err| err.at(at.operation))?;
        // Every corner choice must still match a corner (docs/specs/dogbone-per-corner.md
        // §4): a missing bone makes a part that doesn't fit.
        if let Some(choice) = unmatched_choice(&op, &selection.contours) {
            return Err(job_error(
                Message::new("toolpath-corner-choice-unmatched")
                    .arg("label", label_message.clone())
                    .arg(
                        "at",
                        Value::Point(choice.at[0], choice.at[1], Digits::Fixed(3)),
                    ),
            )
            .at(at.operation));
        }
        chosen.push(Chosen {
            at,
            op,
            label_message,
            selection,
        });
    }
    Ok(chosen)
}

/// The contours some operation cuts, and which part's operations cut each.
struct CutContours<'a> {
    ids: &'a BTreeSet<usize>,
    by_part: &'a BTreeMap<u64, BTreeSet<usize>>,
}

/// One step of the cut order: an operation planned for one copy of its part.
struct Step {
    /// Index in the chosen operations.
    chosen: usize,
    /// Which copy of the part (docs/specs/copies-grid.md): 1 is the drawing as drawn.
    copy: u32,
    /// How far the copy is moved from the drawing.
    shift: Point,
    /// Index in the tool groups.
    group: usize,
}

/// The cut order: part by part (docs/specs/parts.md §3), each part with its copies
/// (docs/specs/copies-grid.md §3), tool group by tool group
/// (docs/specs/tool-per-operation.md §2). A part's copies' order applies within a group,
/// so a grid needs one tool change, not one per copy. The groups' move ranges are empty
/// until planned.
fn cut_order(job: &Job, chosen: &[Chosen]) -> Result<(Vec<Step>, Vec<ToolGroup>), ToolpathError> {
    // Tool groups: chosen operations next to each other in cut order with the same tool.
    let mut tool_runs: Vec<(i64, Vec<usize>)> = Vec::new();
    for (i, c) in chosen.iter().enumerate() {
        match tool_runs.last_mut() {
            Some((tool, members)) if *tool == c.op.tool() => members.push(i),
            _ => tool_runs.push((c.op.tool(), vec![i])),
        }
    }
    let mut steps: Vec<Step> = Vec::new();
    let mut groups: Vec<ToolGroup> = Vec::new();
    for (tool_id, members) in tool_runs {
        let tool = job.tool(tool_id).ok_or_else(|| {
            job_error(Message::new("toolpath-tool-undefined").arg("tool", tool_id))
        })?;
        // A group can run on from one part into the next: each part's run with its copies.
        let mut runs: Vec<Vec<usize>> = Vec::new();
        for &i in &members {
            let part = chosen[i].op.part();
            match runs.last_mut() {
                Some(run) if chosen[run[0]].op.part() == part => run.push(i),
                _ => runs.push(vec![i]),
            }
        }
        let group = groups.len();
        for run in runs {
            let part = job.part_of(&chosen[run[0]].op);
            let copies = part.map_or_else(|| vec![(1, (0.0, 0.0))], Part::in_cut_order);
            let step = |i: usize, (copy, shift): (u32, Point)| Step {
                chosen: i,
                copy,
                shift,
                group,
            };
            // A surface faces the stock once, whatever the part's copies
            // (docs/specs/surfacing.md §2).
            let copies_of = |i: usize| -> Vec<(u32, Point)> {
                match chosen[i].op {
                    Operation::Surface(_) => vec![(1, (0.0, 0.0))],
                    _ => copies.clone(),
                }
            };
            match part.and_then(|p| p.copies).map(|c| c.order) {
                Some(CopyOrder::ByCopy) => {
                    for &copy in &copies {
                        steps.extend(
                            run.iter()
                                .filter(|&&i| copies_of(i).contains(&copy))
                                .map(|&i| step(i, copy)),
                        );
                    }
                }
                _ => {
                    for &i in &run {
                        steps.extend(copies_of(i).into_iter().map(|copy| step(i, copy)));
                    }
                }
            }
        }
        groups.push(ToolGroup {
            tool: tool.clone(),
            operations: members.iter().map(|&i| chosen[i].at).collect(),
            moves: 0..0,
        });
    }
    Ok((steps, groups))
}

/// The Z of every depth pass of `op`, going down from the stock top at `top_z`.
fn pass_heights(op: &Operation, top_z: f64) -> Result<Vec<f64>, ToolpathError> {
    let (start_depth, end_depth) = op.depths();
    let heights = |depths: Vec<f64>| depths.into_iter().map(|d| top_z - d).collect();
    Ok(match op {
        Operation::Profile(ProfileOp { step_down, .. })
        | Operation::Pocket(PocketOp { step_down, .. })
        | Operation::Path(PathOp { step_down, .. })
        | Operation::Surface(SurfaceOp { step_down, .. }) => {
            heights(pass_depths(start_depth, end_depth, *step_down)?)
        }
        // A V-carve sets its own passes from how deep it goes (docs/specs/v-carve.md §3); a
        // relief follows its model in one pass (docs/specs/stl-relief.md §5).
        Operation::Vcarve(_) | Operation::Relief(_) => Vec::new(),
        // A drill's pecks (docs/specs/holes.md §3); a bore goes to the end in one helix.
        Operation::Holes(HolesOp {
            method: HoleMethod::Drill { peck },
            ..
        }) => heights(pass_depths(
            start_depth,
            end_depth,
            peck.unwrap_or(end_depth - start_depth),
        )?),
        Operation::Holes(_) => vec![top_z - end_depth],
    })
}

/// One step planned: the operation on its copy's moved contours, built and verified like
/// the original. An error names the copy and is marked as the operation's.
fn plan_step(
    job: &Job,
    contours: &[Contour],
    cancel: &Cancel,
    chosen: &Chosen,
    step: &Step,
    position: Point,
    cut: &CutContours,
) -> Result<Planned, ToolpathError> {
    let index = chosen.at.operation;
    let op = &chosen.op;
    let tool = op_tool(job, op).map_err(|err| err.at(index))?;
    let top_z = job.stock().top_z();
    let pass_z = pass_heights(op, top_z).map_err(|err| err.at(index))?;
    let (start_depth, _) = op.depths();
    let shift = step.shift;
    // A mirrored part is planned as drawn with the cutting sense turned, then mirrored about
    // its copy's centre line (docs/specs/mirror-part.md §3).
    let mirror = job.mirror_of(op).map(|(mirror, at)| match mirror {
        Mirror::LeftRight => (mirror, at + shift.0),
        Mirror::TopBottom => (mirror, at + shift.1),
    });
    let position = mirror.map_or(position, |m| mirrored_point(m, position));
    // A bone's leg minds only what is cut (docs/specs/dogbones.md §6): a line no operation
    // cuts leaves no web to break. A mirrored part is planned as drawn, so what other parts
    // cut is seen mirrored into its frame, where it lies relative to the part once the part
    // is mirrored (docs/specs/mirror-part.md §3).
    let own = cut.by_part.get(&op.part());
    let cut_contours: Vec<Contour> = if op.has_bones() {
        contours
            .iter()
            .filter(|c| cut.ids.contains(&c.id))
            .map(|c| {
                let moved = c.translated(shift);
                match mirror {
                    Some((m, at)) if !own.is_some_and(|ids| ids.contains(&c.id)) => {
                        moved.mirrored(m == Mirror::LeftRight, at)
                    }
                    _ => moved,
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    let choices: Vec<chipload_job::CornerChoice> = op
        .corner_choices()
        .iter()
        .map(|c| chipload_job::CornerChoice {
            at: [c.at[0] + shift.0, c.at[1] + shift.1],
            ..*c
        })
        .collect();
    let tab_points: Vec<Point> = match op {
        Operation::Profile(ProfileOp {
            tabs: Some(tabs), ..
        }) => tabs
            .at
            .iter()
            .map(|p| (p[0] + shift.0, p[1] + shift.1))
            .collect(),
        _ => Vec::new(),
    };
    let context = Context {
        label_message: &chosen.label_message,
        tool,
        job,
        pass_z: &pass_z,
        top_z: top_z - start_depth,
        choices: &choices,
        tab_points: &tab_points,
        cut_contours: &cut_contours,
        cancel,
        floor_pass: job.is_floor_pass(chosen.at),
        finish_tool: job
            .is_roughing_pass(chosen.at)
            .then(|| job.tool(job.operations[chosen.at.operation].tool()))
            .flatten(),
        sense: job.cutting_sense_of(op),
    };
    let mut moved: Vec<Contour> = chosen
        .selection
        .contours
        .iter()
        .map(|c| c.translated(shift))
        .collect();
    // An inlay plug carves the rest of the board: its outline round the artwork
    // (docs/specs/vcarve-inlay.md §2.2), for the V-carve and its floor pass alike.
    moved.extend(plug_board(job, chosen.at.operation, mirror));
    let shifted = Selection {
        contours: moved.iter().collect(),
        notes: Vec::new(),
    };
    let starts: Vec<Point> = op
        .starts()
        .iter()
        .map(|p| (p[0] + shift.0, p[1] + shift.1))
        .collect();
    let planned = match op {
        Operation::Pocket(pocket) => plan_pocket_op(pocket, &context, &shifted, position, &starts),
        Operation::Profile(profile) => {
            plan_profile_op(profile, &context, &shifted, position, &starts)
        }
        Operation::Vcarve(vcarve) => plan_vcarve_op(vcarve, &context, &shifted, position),
        Operation::Path(path) => plan_path_op(path, &context, &shifted, position),
        Operation::Holes(holes) => plan_holes_op(holes, &context, &shifted, position),
        Operation::Surface(surface) => Ok(plan_surface_op(surface, &context, position)),
        Operation::Relief(relief) => plan_relief_op(relief, &context, &shifted, position, shift),
    };
    let planned = planned.map(|planned| match mirror {
        Some(mirror) => mirrored_planned(planned, mirror),
        None => planned,
    });
    planned.map_err(|err| {
        let part = job.part_of(op);
        let several = part.is_some_and(|p| p.count() > 1);
        let err = if several && job.parts.len() > 1 {
            let name = part.map_or("", |p| p.name.as_str());
            err.with_context(
                Message::new("toolpath-error-part-copy")
                    .arg("part", name)
                    .arg("copy", step.copy),
            )
        } else if several {
            err.with_context(Message::new("toolpath-error-copy").arg("copy", step.copy))
        } else {
            err
        };
        err.at(index)
    })
}

/// The board an inlay plug at operation `index` carves, as a contour round the artwork in
/// the frame the part is planned in (mirrored back for a mirrored part): the stock grown by
/// as much as keeps every wall and the floor pass's end mill clear of its edges, with round
/// corners (docs/specs/vcarve-inlay.md §2.2). Grown by the glue gap's width, g·tan(α/2),
/// the V-bit's walls meet the channel floor at the board's edge; by the end mill's radius
/// more, that floor reaches the edge too, corners included. None for anything but a plug.
fn plug_board(job: &Job, index: usize, mirror: Option<(Mirror, f64)>) -> Option<Contour> {
    let Some(Operation::Vcarve(vcarve)) = job.operations.get(index) else {
        return None;
    };
    let plug = vcarve.plug?;
    let slope = (job.tool(vcarve.tool)?.angle? / 2.0).tan();
    let floor = vcarve
        .floor
        .filter(|f| f.enabled)
        .and_then(|f| job.tool(f.tool))
        .map_or(0.0, |t| t.diameter / 2.0);
    let grow = (vcarve.end_depth - plug.start_depth).max(0.0) * slope + floor;
    let (low, high) = job.stock_bounds();
    let (a, b) = match mirror {
        Some(m) => (mirrored_point(m, low), mirrored_point(m, high)),
        None => (low, high),
    };
    let (x0, y0, x1, y1) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
    Some(rounded_rectangle((x0, y0), (x1, y1), grow))
}

/// The rectangle (`low`, `high`) grown by `grow` all round, its corners quarter circles,
/// counter-clockwise, as a closed contour of no shape (id 0).
fn rounded_rectangle(low: Point, high: Point, grow: f64) -> Contour {
    use chipload_core::chain::{OrientedSegment, Segment};
    use chipload_core::primitives::{Arc, Line, Primitive};
    use std::f64::consts::FRAC_PI_2;
    let (x0, y0, x1, y1) = (low.0, low.1, high.0, high.1);
    let g = grow;
    let mut prims = vec![
        Primitive::Line(Line {
            start: (x0, y0 - g),
            end: (x1, y0 - g),
        }),
        Primitive::Line(Line {
            start: (x1 + g, y0),
            end: (x1 + g, y1),
        }),
        Primitive::Line(Line {
            start: (x1, y1 + g),
            end: (x0, y1 + g),
        }),
        Primitive::Line(Line {
            start: (x0 - g, y1),
            end: (x0 - g, y0),
        }),
    ];
    if g > 0.0 {
        let corners = [(x1, y0), (x1, y1), (x0, y1), (x0, y0)];
        for (k, &center) in corners.iter().enumerate() {
            let arc = Primitive::Arc(Arc {
                center,
                radius: g,
                start_angle: -FRAC_PI_2 + k as f64 * FRAC_PI_2,
                sweep: FRAC_PI_2,
            });
            prims.insert(2 * k + 1, arc);
        }
    }
    let parts = prims
        .into_iter()
        .map(|prim| OrientedSegment {
            segment: Segment {
                prim,
                layer: String::new(),
                handle: String::new(),
                source_type: String::new(),
                shape: None,
                from: None,
                construction: false,
            },
            reversed: false,
        })
        .collect();
    Contour {
        id: 0,
        parts,
        closed: true,
        invalid_reason: None,
        nesting_depth: Some(0),
    }
}

/// `p` mirrored about X = at (left to right) or Y = at (top to bottom).
fn mirrored_point((mirror, at): (Mirror, f64), p: Point) -> Point {
    match mirror {
        Mirror::LeftRight => (2.0 * at - p.0, p.1),
        Mirror::TopBottom => (p.0, 2.0 * at - p.1),
    }
}

/// One step's plan as cut on a mirrored part (docs/specs/mirror-part.md §3): the moves,
/// their arcs, the tabs and pocket pieces shown with them, and where the tool ends,
/// mirrored. Arcs turn the other way, and so does a helix. The markers the user moves (ring
/// starts, tab centres, unreached corners) stay on the shapes as drawn, where they are
/// stored.
fn mirrored_planned(mut planned: Planned, mirror: (Mirror, f64)) -> Planned {
    let flip = |p: Point| mirrored_point(mirror, p);
    for m in &mut planned.moves {
        match mirror.0 {
            Mirror::LeftRight => m.x = m.x.map(|x| 2.0 * mirror.1 - x),
            Mirror::TopBottom => m.y = m.y.map(|y| 2.0 * mirror.1 - y),
        }
        if let Some(circle) = &mut m.circle {
            circle.center = flip(circle.center);
        }
        m.kind = match m.kind {
            MoveKind::ArcCw => MoveKind::ArcCcw,
            MoveKind::ArcCcw => MoveKind::ArcCw,
            kind => kind,
        };
    }
    planned.arcs = planned.arcs.mapped(flip);
    for tab in &mut planned.tabs {
        for p in tab.iter_mut() {
            *p = flip(*p);
        }
    }
    for piece in &mut planned.pockets {
        for ring in &mut piece.rings {
            for p in &mut ring.points {
                *p = flip(*p);
            }
            for p in ring.unstepped.iter_mut().flatten() {
                *p = flip(*p);
            }
        }
        if let Some(helix) = &mut piece.helix {
            helix.circle.center = flip(helix.circle.center);
            helix.start = flip(helix.start);
            helix.clockwise = !helix.clockwise;
        }
    }
    planned.end = flip(planned.end);
    planned
}

/// Every plunge from safe Z gets a rapid down to the approach height first
/// (docs/specs/approach.md): a feed straight down that starts at safe Z and ends below
/// `top_z + APPROACH_CLEARANCE_MM`. Nothing goes below the stock top as a rapid, and a
/// safe Z at or below the approach height leaves the moves as they are.
fn lower_approaches(moves: Vec<Move>, top_z: f64, safe_z: f64) -> Vec<Move> {
    let approach = top_z + APPROACH_CLEARANCE_MM;
    if safe_z <= approach + DEPTH_TOL_MM {
        return moves;
    }
    let mut out = Vec::with_capacity(moves.len() + 8);
    let mut z: Option<f64> = None;
    for m in moves {
        let plunge = m.kind == MoveKind::Feed && m.x.is_none() && m.y.is_none();
        if plunge
            && let (Some(from), Some(to)) = (z, m.z)
            && (from - safe_z).abs() <= DEPTH_TOL_MM
            && to < approach - DEPTH_TOL_MM
        {
            out.push(Move::rapid_z(approach));
        }
        if let Some(mz) = m.z {
            z = Some(mz);
        }
        out.push(m);
    }
    out
}

/// Everything the steps planned so far, in cut order.
#[derive(Default)]
struct Gathered {
    moves: Vec<Move>,
    /// The operations' facts, each said once (a V-carve's floor left, for one).
    facts: Vec<Note>,
    tabs: Vec<Vec<Point>>,
    pockets: Vec<PocketPiece>,
    op_spans: Vec<OpSpan>,
    ring_starts: Vec<RingStart>,
    unreached_corners: Vec<(usize, Point)>,
    tab_centres: Vec<TabCentre>,
    floor_hints: Vec<(usize, f64)>,
    arcs: ArcTable,
    /// With two parts or more, a comment where each part starts (docs/specs/parts.md §6).
    comments: Vec<String>,
    part_started: Option<u64>,
    /// Operations planned at least once: their selection's notes are listed once.
    started: Vec<OpRef>,
    /// Every copy repeats the original's notes: each is listed once per operation, by its
    /// English words.
    seen: Vec<(OpRef, String)>,
}

impl Gathered {
    fn add(
        &mut self,
        job: &Job,
        step: &Step,
        at: OpRef,
        tool_radius: f64,
        planned: Planned,
        notes: &mut Vec<Note>,
    ) {
        let index = at.operation;
        let first = self.moves.len();
        self.moves.extend(lower_approaches(
            planned.moves,
            job.stock().top_z(),
            job.machine.safe_z,
        ));
        self.comment_part_start(job, index, first);
        self.op_spans.push(OpSpan {
            operation: index,
            pass: at.pass,
            copy: step.copy,
            moves: first..self.moves.len(),
            tool_radius,
        });
        for note in planned.notes {
            // By its words: a copy's numbers can differ in the last bits.
            let key = (at, note.english());
            if !self.seen.contains(&key) {
                notes.push(Note::at(index, note));
                self.seen.push(key);
            }
        }
        for fact in planned.facts {
            let key = (at, fact.english());
            if !self.seen.contains(&key) {
                self.facts.push(Note::at(index, fact));
                self.seen.push(key);
            }
        }
        self.tabs.extend(planned.tabs);
        self.pockets.extend(planned.pockets);
        // Start markers are drawn on the part: the copies' are the same. A rest pass's rings
        // start where the planner puts them: starts belong to the pocket.
        if step.copy == 1
            && let Some(largest) = planned.floor_hint
        {
            self.floor_hints.push((index, largest));
        }
        if step.copy == 1 && at.pass.is_none() {
            self.unreached_corners
                .extend(planned.unreached.iter().map(|&p| (index, p)));
            self.tab_centres
                .extend(
                    planned
                        .tab_centres
                        .iter()
                        .map(|&(ring, at, by_hand)| TabCentre {
                            operation: index,
                            ring,
                            at,
                            by_hand,
                        }),
                );
            self.ring_starts
                .extend(planned.starts.into_iter().map(|(ring, stored)| RingStart {
                    operation: index,
                    ring,
                    stored,
                }));
        }
        self.arcs.extend(planned.arcs);
    }

    /// With two parts or more, a comment on the first move of each part
    /// (docs/specs/parts.md §6).
    fn comment_part_start(&mut self, job: &Job, index: usize, first: usize) {
        let op = &job.operations[index];
        if job.parts.len() > 1
            && self.part_started != Some(op.part())
            && let (Some(part), Some(start)) = (job.part_of(op), self.moves.get_mut(first))
        {
            self.part_started = Some(op.part());
            let copies = match part.count() {
                1 => String::new(),
                n => format!(", {n} copies"),
            };
            let number = job.parts.iter().position(|p| p.id == part.id).unwrap_or(0) + 1;
            start.comment = u32::try_from(self.comments.len()).ok();
            self.comments
                .push(format!("Part {number}: {}{copies}", part.name));
        }
    }
}

/// What every operation needs from the job.
struct Context<'a> {
    label_message: &'a Message,
    tool: &'a Tool,
    job: &'a Job,
    pass_z: &'a [f64],
    /// Z of the operation's start depth: cut already, where a ramp begins.
    top_z: f64,
    /// The operation's corner choices, moved with the copy.
    choices: &'a [chipload_job::CornerChoice],
    /// Its tabs placed by hand, moved with the copy.
    tab_points: &'a [Point],
    /// The contours some operation cuts, moved with the copy: a bone's leg is checked
    /// against these (docs/specs/dogbones.md §6).
    cut_contours: &'a [Contour],
    cancel: &'a Cancel,
    /// A V-carve's floor pass, planned as a pocket (docs/specs/vcarve-wide-strokes.md §4).
    floor_pass: bool,
    /// For a relief's roughing pass, the relief's own (finishing) tool
    /// (docs/specs/stl-relief.md §14 step 3); None otherwise.
    finish_tool: Option<&'a Tool>,
    /// The spindle's turn in the drawing's coordinates, which every travel direction is
    /// chosen with: the other way for a back setup ([`Job::cutting_sense`]).
    sense: Spindle,
}

/// One operation's moves, warnings and tabs, and where the tool ends up (above that point).
struct Planned {
    moves: Vec<Move>,
    notes: Vec<Message>,
    tabs: Vec<Vec<Point>>,
    pockets: Vec<PocketPiece>,
    arcs: ArcTable,
    end: Point,
    /// Rings whose start can be moved, and the stored point that set it.
    starts: Vec<(Vec<Point>, Option<usize>)>,
    /// Corners that would get a bone but the tool can't reach.
    unreached: Vec<Point>,
    /// Each tab's ring (index in the rings), centre and whether it was placed by hand.
    tab_centres: Vec<(usize, Point, bool)>,
    /// A V-carve with a floor pass that leaves part of the floor: the largest end mill
    /// diameter that would clear it all, mm (docs/specs/vcarve-wide-strokes.md §4.1).
    floor_hint: Option<f64>,
    /// Facts about the operation, not warnings: a V-carve's floor left at the surface and
    /// how much of it a floor pass clears (vcarve-wide-strokes.md §6).
    facts: Vec<Message>,
}

fn plan_pocket_op(
    op: &PocketOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    starts: &[Point],
) -> Result<Planned, ToolpathError> {
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
        clearance: op.clearance,
        stepover: op.stepover * ctx.tool.diameter,
        direction: op.direction,
        spindle: ctx.sense,
        start,
        choices: Choices {
            starts,
            lead: op.lead.map(|l| l.radius),
            helix: match op.entry {
                Entry::Helix { radius, .. } => Some(radius),
                _ => None,
            },
            round_corners: true,
            spiral: true,
            cancel: Some(ctx.cancel),
            rest,
            floor: ctx.floor_pass,
        },
    };
    let plan = plan_pocket(&request).map_err(|err| err.with_context(ctx.label_message.clone()))?;
    let mut plan = plan;
    let mut corner_notes = Vec::new();
    let mut unreached_at = Vec::new();
    if op.corners.is_some() || !ctx.choices.is_empty() {
        let boundaries = pocket_boundaries(&selection.contours);
        let cut = CornerCut {
            corners: op.corners,
            choices: ctx.choices,
            tool_radius: ctx.tool.diameter / 2.0,
            clearance: op.clearance,
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
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan
            .notes
            .iter()
            .map(|n| labelled_note(ctx.label_message, n.clone()))
            .chain(corner_notes)
            .collect(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: plan.arcs,
        end: start,
        starts: Vec::new(),
        unreached: unreached_at,
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    };
    let cut = PocketCut {
        pass_z: ctx.pass_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        entry: op.entry,
        slot_feed: op.slot_feed.unwrap_or(op.feed),
        slot_step: op.slot_step_down,
    };
    for piece in &plan.pieces {
        for ring in piece.rings.iter().filter(|r| r.wall) {
            // Without the leads, if it has them.
            out.starts.push((ring.plain().to_vec(), ring.chosen));
        }
        out.moves.extend(
            pocket_moves(piece, &cut).map_err(|err| err.with_context(ctx.label_message.clone()))?,
        );
        // Retracted above the start of the wall ring.
        if let Some(end) = piece.rings.last().and_then(PocketRing::end) {
            out.end = end;
        }
    }
    out.pockets = plan.pieces;
    Ok(out)
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
fn profile_legs(
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

fn plan_path_op(
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

/// A hole drilled in one go deeper than this many diameters gets a note to drill in pecks.
const DEEP_HOLE_DIAMETERS: f64 = 3.0;

/// The smallest hole an end mill of `diameter` bores: its own size plus a lead of
/// `MIN_LEAD_DIAMETERS` × D on each side (docs/specs/holes.md §4).
fn smallest_bore(diameter: f64) -> f64 {
    diameter * (1.0 + 2.0 * MIN_LEAD_DIAMETERS)
}

fn plan_holes_op(
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

/// A surface (docs/specs/surfacing.md): the whole stock, back and forth along its longer
/// side, every depth started beside it.
fn plan_surface_op(op: &SurfaceOp, ctx: &Context, start: Point) -> Planned {
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

/// How wide a V-bit's groove is at the top, `depth` deep (docs/specs/follow-path.md §5): a
/// plain note. None for another tool, or a V-bit without an angle.
fn groove_width(tool: &Tool, depth: f64) -> Option<Message> {
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
    Some((tool.diameter - tip) / 2.0 / (angle / 2.0).tan())
}

/// The smallest end mill a floor hint names, mm: below it, no end mill clears the floor.
const FLOOR_HINT_MIN_MM: f64 = 0.1;

fn plan_vcarve_op(
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

fn plan_profile_op(
    op: &ProfileOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    starts: &[Point],
) -> Result<Planned, ToolpathError> {
    let machine = &ctx.job.machine;
    let radius = ctx.tool.diameter / 2.0;
    // The rings at the wall; with a finishing pass the roughing rings run `stock` farther
    // out (docs/specs/finish-and-skin.md §2).
    let mut plan = plan_profile(
        op,
        ctx.label_message,
        &selection.contours,
        radius,
        ctx.sense,
        start,
        starts,
    )?;
    let rough_op = op.finish.map(|finish| ProfileOp {
        clearance: op.clearance - finish.stock,
        ..op.clone()
    });
    let rough = match &rough_op {
        Some(rough_op) => Some(plan_profile(
            rough_op,
            ctx.label_message,
            &selection.contours,
            radius,
            ctx.sense,
            start,
            starts,
        )?),
        None => None,
    };
    let mut rough = rough;
    for ring in plan.rings.iter().chain(rough.iter().flat_map(|r| &r.rings)) {
        let length = crate::ramp::polyline_length(&crate::profile::closed_path(&ring.points));
        short_ramp_refusal(ctx.tool, length).map_err(|why| {
            job_error(labelled_note(
                &contour_context(ctx.label_message, ring.contour_id),
                why,
            ))
        })?;
    }
    let mut unreached_at = Vec::new();
    // Dogbones go into every ring before the tabs are placed (docs/specs/dogbones.md §4).
    if op.corners.is_some() || !ctx.choices.is_empty() {
        let cut = CornerCut {
            corners: op.corners,
            choices: ctx.choices,
            tool_radius: radius,
            clearance: op.clearance,
            everything: ctx.cut_contours,
            exempt: &[],
            walls: &selection.contours,
            clears_inside: false,
            label: ctx.label_message,
        };
        let (unreached, warnings) =
            profile_legs(&mut plan.rings, &selection.contours, op.side, &cut)?;
        plan.notes.extend(warnings);
        unreached_at.extend(unreached.iter().map(|(c, _)| c.at));
        plan.notes.extend(
            unreached
                .iter()
                .map(|(c, bone)| unreached_note(ctx.label_message, c, *bone)),
        );
        if let (Some(rough), Some(rough_op)) = (&mut rough, &rough_op) {
            let rough_cut = CornerCut {
                clearance: rough_op.clearance,
                ..cut
            };
            // The roughing's legs are the wall's, a little short: the same warnings.
            profile_legs(&mut rough.rings, &selection.contours, op.side, &rough_cut)?;
        }
    }
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan.notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: plan.arcs,
        end: start,
        starts: plan
            .rings
            .iter()
            .zip(&plan.chosen)
            .map(|(ring, &stored)| (ring.points.clone(), stored))
            .collect(),
        unreached: unreached_at,
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    };
    if let Some(rough) = &rough {
        out.arcs.extend(rough.arcs.clone());
        // The roughing may add a narrow part the wall doesn't have; its notes on start
        // points say the same as the wall's.
        let mut words: Vec<String> = out.notes.iter().map(Message::english).collect();
        for note in &rough.notes {
            let english = note.english();
            if !words.contains(&english) {
                words.push(english);
                out.notes.push(note.clone());
            }
        }
    }
    // The notes on start points come after the geometry's, as before.
    out.notes.extend(plan.start_notes);
    let end_z = ctx.pass_z.last().copied().unwrap_or(ctx.top_z);
    // With a skin the roughing stops `thickness` above the end depth.
    let rough_z: Vec<f64> = match op.skin {
        Some(skin) => pass_depths(op.start_depth, op.end_depth - skin.thickness, op.step_down)?
            .into_iter()
            .map(|d| ctx.top_z - (d - op.start_depth))
            .collect(),
        None => ctx.pass_z.to_vec(),
    };
    let rough_bottom = rough_z.last().copied().unwrap_or(end_z);
    // The tabs are placed on the wall rings; the roughing rings carry them over.
    let (layouts, curved) = profile_tabs(op, ctx, &plan.rings, &mut out)?;
    let surface = |layout: &Option<TabLayout>| -> Option<(TabLayout, f64, f64)> {
        let tabs = op.tabs.as_ref()?;
        Some((layout.clone()?, end_z + tabs.height, tabs.flank_angle))
    };
    let rough_rings = rough.as_ref().map_or(&plan.rings, |r| &r.rings);
    // Roughing: every ring as a spiral, down to the skin.
    for (k, ring) in rough_rings.iter().enumerate() {
        let context = |err: ToolpathError| {
            err.with_context(contour_context(ctx.label_message, ring.contour_id))
        };
        let layout = if rough.is_some() {
            carried_tabs(&plan.rings, &layouts, k, rough_rings).map_err(context)?
        } else {
            // The roughing rings are the wall rings.
            layouts.get(k).cloned().flatten()
        };
        let tabs = surface(&layout);
        let cut = RingCut {
            ring: &ring.points,
            tabs: as_surface(&tabs),
            pass_z: &rough_z,
            top_z: ctx.top_z,
            safe_z: machine.safe_z,
            feed: op.feed,
            plunge_feed: op.plunge_feed,
            ramp_angle: Some(op.ramp_angle),
            tool_diameter: ctx.tool.diameter,
        };
        let cut = ring_moves(&cut, &out.arcs).map_err(context)?;
        out.moves.extend(cut.moves);
        out.end = cut.end;
    }
    // Then a finishing lap on every wall ring, then a skin lap (§4).
    let stages = [op.finish.map(|_| rough_bottom), op.skin.map(|_| end_z)];
    for z in stages.into_iter().flatten() {
        for (ring, layout) in plan.rings.iter().zip(&layouts) {
            let context = |err: ToolpathError| {
                err.with_context(contour_context(ctx.label_message, ring.contour_id))
            };
            let slot: Vec<&[Point]> = rough_rings
                .iter()
                .filter(|r| r.contour_id == ring.contour_id)
                .map(|r| r.points.as_slice())
                .collect();
            if let Some(finish) = op.finish
                && z == rough_bottom
            {
                slot_note(
                    ring,
                    &slot,
                    radius,
                    finish.stock,
                    ctx.label_message,
                    &mut out.notes,
                )?;
            }
            let tabs = surface(layout);
            let lap = WallLap {
                ring: &ring.points,
                tabs: as_surface(&tabs),
                slot: &slot,
                slot_z: rough_bottom,
                z,
                safe_z: machine.safe_z,
                feed: op.feed,
                plunge_feed: op.plunge_feed,
                ramp_angle: op.ramp_angle,
                tool_diameter: ctx.tool.diameter,
            };
            let cut = wall_lap(&lap, &out.arcs).map_err(context)?;
            out.moves.extend(cut.moves);
            out.end = cut.end;
        }
    }
    if !curved.is_empty() {
        let ids: Vec<String> = curved.iter().map(|c| format!("#{c}")).collect();
        out.notes.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-tabs-on-curves").arg("contours", ids.join(", ")),
        ));
    }
    Ok(out)
}

/// The tabs on every wall ring (docs/specs/profile-spiral.md §4): each ring's layout (None
/// without tabs), with the tabs' polylines and notes added to `out`, and the contours
/// whose tabs sit on a curve.
fn profile_tabs(
    op: &ProfileOp,
    ctx: &Context,
    rings: &[PathRing],
    out: &mut Planned,
) -> Result<(Vec<Option<TabLayout>>, Vec<usize>), ToolpathError> {
    let Some(tabs) = &op.tabs else {
        return Ok((vec![None; rings.len()], Vec::new()));
    };
    if op.side == Side::Inside {
        out.notes.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-tabs-inside"),
        ));
    }
    // Tabs placed by hand go to their nearest ring (docs/specs/tabs-by-hand.md §2).
    let mut by_ring: Vec<Vec<Point>> = vec![Vec::new(); rings.len()];
    for &p in ctx.tab_points {
        let nearest = rings
            .iter()
            .enumerate()
            .filter_map(|(k, r)| arc_position(&r.points, p).map(|(_, d)| (k, d)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match nearest {
            Some((k, d)) if d <= ctx.tool.diameter => by_ring[k].push(p),
            _ => {
                return Err(ToolpathError::Geometry(
                    Message::new("toolpath-tab-no-ring")
                        .arg("label", ctx.label_message.clone())
                        .arg("at", Value::Point(p.0, p.1, Digits::Fixed(3))),
                ));
            }
        }
    }
    let mut layouts: Vec<Option<TabLayout>> = Vec::with_capacity(rings.len());
    let mut curved: Vec<usize> = Vec::new();
    for (k, (ring, points)) in rings.iter().zip(&by_ring).enumerate() {
        // Validated: count >= 1.
        let count = usize::try_from(tabs.count).unwrap_or(0);
        let placed = if let (true, Some(axis)) = (points.is_empty(), tabs.along) {
            along_tabs(&ring.points, count, axis, tabs.width, ctx.tool.diameter)
        } else if points.is_empty() {
            place_tabs(&ring.points, count, tabs.width, ctx.tool.diameter)
        } else {
            tabs_at(&ring.points, points, tabs.width, ctx.tool.diameter)
        }
        .map_err(|err| err.with_context(contour_context(ctx.label_message, ring.contour_id)))?;
        out.tab_centres.extend(placed.spans.iter().map(|&(s0, s1)| {
            (
                k,
                point_at_arc(&ring.points, (s0 + s1) / 2.0),
                !points.is_empty(),
            )
        }));
        if placed.on_curves > 0 {
            curved.push(ring.contour_id);
        }
        out.tabs.extend(tab_polylines(&ring.points, &placed));
        if let Some(note) = joined_feet(&placed, tabs) {
            out.notes
                .push(contour_note(ctx.label_message, ring.contour_id, note));
        }
        layouts.push(Some(placed));
    }
    Ok((layouts, curved))
}

/// A tab layout with its top and flank angle, as the laps follow it.
fn as_surface(tabs: &Option<(TabLayout, f64, f64)>) -> Option<TabSurface<'_>> {
    tabs.as_ref()
        .map(|(layout, z_tab, flank_angle)| TabSurface {
            layout,
            z_tab: *z_tab,
            flank_angle: *flank_angle,
        })
}

/// The tabs of the wall rings of a contour, carried over to roughing ring `k` of `rough`
/// (docs/specs/finish-and-skin.md §2): every lap leaves the same tabs.
fn carried_tabs(
    walls: &[PathRing],
    layouts: &[Option<TabLayout>],
    k: usize,
    rough: &[PathRing],
) -> Result<Option<TabLayout>, ToolpathError> {
    let ring = &rough[k];
    let others: Vec<&[Point]> = rough
        .iter()
        .enumerate()
        .filter(|&(j, r)| r.contour_id == ring.contour_id && j != k)
        .map(|(_, r)| r.points.as_slice())
        .collect();
    let mut carried: Option<TabLayout> = None;
    for (wall, layout) in walls.iter().zip(layouts) {
        let Some(layout) = layout else { continue };
        if wall.contour_id != ring.contour_id {
            continue;
        }
        let more = carry_tabs(&wall.points, layout, &ring.points, &others)?;
        carried = Some(match carried {
            None => more,
            Some(mut so_far) => {
                so_far.spans.extend(more.spans);
                so_far.spans.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut joined: Vec<(f64, f64)> = Vec::new();
                for span in so_far.spans {
                    match joined.last_mut() {
                        Some(last) if span.0 <= last.1 => last.1 = last.1.max(span.1),
                        _ => joined.push(span),
                    }
                }
                so_far.spans = joined;
                so_far
            }
        });
    }
    Ok(carried)
}

/// The finishing lap must run in the roughing's slot: refused where it is a tool radius or
/// more from the roughing path (the roughing couldn't get there), a note where it cuts
/// more than twice the stock (docs/specs/finish-and-skin.md §6).
fn slot_note(
    ring: &PathRing,
    slot: &[&[Point]],
    radius: f64,
    stock: f64,
    label_message: &Message,
    notes: &mut Vec<Message>,
) -> Result<(), ToolpathError> {
    let farthest = farthest_from(&ring.points, slot, radius / 2.0);
    let id = ring.contour_id;
    if farthest >= radius {
        return Err(ToolpathError::Geometry(contour_note(
            label_message,
            id,
            Message::new("toolpath-error-roughing-reach").arg("farthest", Value::mm(farthest)),
        )));
    }
    if farthest > 2.0 * stock + CHAIN_TOL_MM {
        notes.push(contour_note(
            label_message,
            id,
            Message::new("toolpath-finish-lap-cuts")
                .arg("most", Value::mm(farthest))
                .arg("stock", Value::mm(stock)),
        ));
    }
    Ok(())
}

/// A note when neighbouring sloped tabs meet below their top: their feet, each
/// `height / tan(flank)` long, together are longer than the gap between them
/// (docs/specs/profile-spiral.md §4).
fn joined_feet(layout: &TabLayout, tabs: &Tabs) -> Option<Message> {
    if tabs.is_straight() || layout.spans.len() < 2 {
        return None;
    }
    let feet = 2.0 * tabs.height / tabs.flank_angle.tan();
    let count = layout.spans.len();
    let mut pairs = Vec::new();
    for k in 0..count {
        let (_, end) = layout.spans[k];
        let next = (k + 1) % count;
        let mut gap = layout.spans[next].0 - end;
        if next == 0 {
            gap += layout.length;
        }
        if gap < feet {
            pairs.push(
                Message::new("toolpath-and")
                    .arg("list", k + 1)
                    .arg("last", next + 1),
            );
        }
    }
    let pairs = comma_list(pairs)?;
    Some(
        Message::new("toolpath-tabs-joined")
            .arg("pairs", pairs)
            .arg(
                "flank",
                Value::Number(tabs.flank_angle.to_degrees(), Digits::Fixed(1)),
            ),
    )
}

/// The XY extent the tool cuts in one copy (or in all, with None): every feed move's end
/// points, plus the radius of the tool that made them. None if it cuts nothing.
pub fn cut_extent(moves: &[Move], spans: &[OpSpan], copy: Option<u32>) -> Option<(Point, Point)> {
    let mut extent: Option<(Point, Point)> = None;
    let mut at: (Option<f64>, Option<f64>) = (None, None);
    for span in spans {
        let r = span.tool_radius;
        for m in &moves[span.moves.clone()] {
            let from = at;
            at = (m.x.or(at.0), m.y.or(at.1));
            if copy.is_some_and(|c| c != span.copy) || m.kind != MoveKind::Feed {
                continue;
            }
            for (x, y) in [from, at] {
                let (Some(x), Some(y)) = (x, y) else { continue };
                let ((x0, y0), (x1, y1)) = extent.unwrap_or(((x - r, y - r), (x + r, y + r)));
                extent = Some((
                    (x0.min(x - r), y0.min(y - r)),
                    (x1.max(x + r), y1.max(y + r)),
                ));
            }
        }
    }
    extent
}

/// What one copy of a part cuts: every feed move as a segment, grouped by the radius of
/// the tool that made it, and the box round them widened by those radii.
struct CutArea {
    extent: (Point, Point),
    by_radius: Vec<(f64, PrimIndex)>,
}

/// The cut of `copy` among `spans`; None if it cuts nothing.
fn cut_area(moves: &[Move], spans: &[OpSpan], copy: u32) -> Option<CutArea> {
    let extent = cut_extent(moves, spans, Some(copy))?;
    let mut by_radius: std::collections::BTreeMap<u64, (f64, Vec<Primitive>)> = Default::default();
    let mut at: (Option<f64>, Option<f64>) = (None, None);
    for span in spans {
        for m in &moves[span.moves.clone()] {
            let from = at;
            at = (m.x.or(at.0), m.y.or(at.1));
            if span.copy != copy || m.kind != MoveKind::Feed {
                continue;
            }
            if let ((Some(x0), Some(y0)), (Some(x1), Some(y1))) = (from, at) {
                by_radius
                    .entry(span.tool_radius.to_bits())
                    .or_insert_with(|| (span.tool_radius, Vec::new()))
                    .1
                    .push(Primitive::Line(Line {
                        start: (x0, y0),
                        end: (x1, y1),
                    }));
            }
        }
    }
    let by_radius = by_radius
        .into_values()
        .map(|(radius, prims)| (radius, PrimIndex::new(prims, radius.max(1.0))))
        .collect();
    Some(CutArea { extent, by_radius })
}

/// Whether the tools cut into each other: a feed move of one copy comes closer to a feed
/// move of the other than the two radii (less the chaining tolerance, so touching is
/// allowed). The boxes decide first; the segments only where they overlap.
fn areas_overlap(a: &CutArea, b: &CutArea) -> bool {
    if !boxes_overlap(a.extent, b.extent) {
        return false;
    }
    a.by_radius.iter().any(|(ra, index_a)| {
        b.by_radius.iter().any(|(rb, index_b)| {
            let limit = ra + rb - CHAIN_TOL_MM;
            index_a.prims.iter().any(|prim| match prim {
                Primitive::Line(l) => index_b
                    .segment_distance_within(l.start, l.end, limit)
                    .is_some(),
                Primitive::Arc(_) => false,
            })
        })
    })
}

/// Copies must not cut into each other or into other parts (docs/specs/copies-grid.md
/// §4, docs/specs/parts.md §5). Copies 1, the parts as drawn, may overlap each other.
fn check_parts_apart(job: &Job, moves: &[Move], spans: &[OpSpan]) -> Result<(), ToolpathError> {
    let mut areas: Vec<(usize, u32, CutArea)> = Vec::new();
    for (p, part) in job.parts.iter().enumerate() {
        // A surface covers the whole stock, every part's copies included
        // (docs/specs/surfacing.md §2): it overlaps them by design.
        let mine: Vec<OpSpan> = spans
            .iter()
            .filter(|s| {
                job.operations
                    .get(s.operation)
                    .is_some_and(|op| op.part() == part.id && !matches!(op, Operation::Surface(_)))
            })
            .cloned()
            .collect();
        if let Some(grid) = &part.copies {
            check_copies_apart(grid, moves, &mine)?;
        }
        for (number, _) in part.in_cut_order() {
            if let Some(area) = cut_area(moves, &mine, number) {
                areas.push((p, number, area));
            }
        }
    }
    for (k, (p, number, a)) in areas.iter().enumerate() {
        for (q, other, b) in &areas[..k] {
            let generated = *number > 1 || *other > 1;
            if p == q || !generated || !areas_overlap(a, b) {
                continue;
            }
            let ((x0, y0), (x1, y1)) = a.extent;
            return Err(ToolpathError::Overlap(
                Message::new("toolpath-error-parts-overlap")
                    .arg("part", job.parts[*p].name.as_str())
                    .arg("copy", *number)
                    .arg("other_part", job.parts[*q].name.as_str())
                    .arg("other_copy", *other)
                    .arg("x0", Value::LengthNumber(x0, Digits::Fixed(3)))
                    .arg("x1", Value::Length(x1, Digits::Fixed(3)))
                    .arg("y0", Value::LengthNumber(y0, Digits::Fixed(3)))
                    .arg("y1", Value::Length(y1, Digits::Fixed(3))),
            ));
        }
    }
    Ok(())
}

/// Whether two boxes overlap by more than the chaining tolerance.
fn boxes_overlap(a: (Point, Point), b: (Point, Point)) -> bool {
    let ((ax0, ay0), (ax1, ay1)) = a;
    let ((bx0, by0), (bx1, by1)) = b;
    ax0 + CHAIN_TOL_MM < bx1
        && bx0 + CHAIN_TOL_MM < ax1
        && ay0 + CHAIN_TOL_MM < by1
        && by0 + CHAIN_TOL_MM < ay1
}

/// Copies of a part must not cut into each other (docs/specs/copies-grid.md §4): the
/// tools' paths are compared, so copies whose boxes overlap but whose cuts don't (shapes
/// that interleave) are allowed. The message says what pitch would keep two copies in a
/// row or a column apart for sure: the cut's width or height.
fn check_copies_apart(
    grid: &Copies,
    moves: &[Move],
    spans: &[OpSpan],
) -> Result<(), ToolpathError> {
    let copies: Vec<(u32, Point, CutArea)> = grid
        .in_cut_order()
        .into_iter()
        .filter_map(|(number, shift)| cut_area(moves, spans, number).map(|a| (number, shift, a)))
        .collect();
    let Some((_, _, first)) = copies.first() else {
        return Ok(());
    };
    let ((x0, y0), (x1, y1)) = first.extent;
    let (width, height) = (x1 - x0, y1 - y0);
    for (k, (number, shift, a)) in copies.iter().enumerate() {
        for (other, shift_other, b) in &copies[..k] {
            if !areas_overlap(a, b) {
                continue;
            }
            let same_row = (shift.1 - shift_other.1).abs() <= CHAIN_TOL_MM;
            let same_column = (shift.0 - shift_other.0).abs() <= CHAIN_TOL_MM;
            let fixed = |v: f64| Value::LengthNumber(v, Digits::Fixed(3));
            let text = if same_row {
                Message::new("toolpath-error-copies-dx")
                    .arg("width", Value::Length(width, Digits::Fixed(3)))
                    .arg("got", fixed(grid.dx.abs()))
            } else if same_column {
                Message::new("toolpath-error-copies-dy")
                    .arg("height", Value::Length(height, Digits::Fixed(3)))
                    .arg("got", fixed(grid.dy.abs()))
            } else {
                Message::new("toolpath-error-copies-overlap")
                    .arg("other", *other)
                    .arg("copy", *number)
                    .arg("dx", fixed(grid.dx.abs()))
                    .arg("dy", fixed(grid.dy.abs()))
            };
            return Err(ToolpathError::Overlap(text));
        }
    }
    Ok(())
}

fn job_error(message: Message) -> ToolpathError {
    ToolpathError::Job(message)
}

/// The tool an operation names (docs/specs/tool-per-operation.md §4).
pub fn op_tool<'a>(job: &'a Job, op: &Operation) -> Result<&'a Tool, ToolpathError> {
    job.tool(op.tool())
        .ok_or_else(|| job_error(Message::new("toolpath-tool-undefined").arg("tool", op.tool())))
}

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
fn labelled(path: String, label: &Message, short: Message) -> Problem {
    let text = Message::new("job-operation-problem")
        .arg("label", label.clone())
        .arg("problem", short.clone());
    Problem::new(path, short, text)
}

/// A length as Rust writes it with `{}`, with its unit.
fn length(mm: f64) -> Value {
    Value::Length(mm, Digits::Shortest)
}

/// The problems, in the order the checks were always made (so the first is the error
/// `validate_job` always gave), and the notes of a job without problems.
/// How far a through cut goes into the spoilboard, as a plain note (docs/specs/styles.md
/// §2.1); None for a cut that isn't through or stays in the stock.
fn through_overcut(job: &Job, op: &Operation) -> Option<Message> {
    op.through()?;
    let stock = job.setup_of(op).map_or(job.stock(), |s| &s.stock);
    let below = op.depths().1 - job.drill_point(op) - stock.thickness;
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
            Operation::Holes(h) => (h.feed, h.plunge_feed),
            Operation::Surface(f) => (f.feed, f.plunge_feed),
            Operation::Relief(r) => (r.feed, r.plunge_feed),
        };
        // Below the stock: a warning within the overcut allowance, an error beyond it
        // (docs/specs/depths-and-clearance.md section 2).
        let (_, end_depth) = op.depths();
        // Its own setup's board (docs/specs/setups.md §5).
        let stock = job.setup_of(op).map_or(job.stock(), |s| &s.stock);
        let thickness = stock.thickness;
        // A drill's point goes deeper than its full diameter: the allowance is for the
        // diameter (docs/specs/holes.md decision 10).
        let below = end_depth - job.drill_point(op) - thickness;
        // Dowel holes go deeper, for the pins (docs/specs/two-sided.md §6).
        let allowed = op.allowed_below(stock.max_overcut);
        if below > allowed + DEPTH_TOL_MM {
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
            let (more_problems, more_notes) = roughing_checks(job, index, rough);
            problems.extend(more_problems);
            notes.extend(more_notes);
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
            if let Operation::Path(_) = op {
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
        Operation::Holes(_) => "holes",
        Operation::Surface(_) => "surface",
        Operation::Relief(_) => "relief",
    }
}

/// Items joined with commas: "1, 2, 3".
fn comma_list(items: Vec<Message>) -> Option<Message> {
    let mut items = items.into_iter();
    let first = items.next()?;
    Some(items.fold(first, |list, item| {
        Message::new("toolpath-comma")
            .arg("list", list)
            .arg("item", item)
    }))
}

/// Items joined with commas and "and" before the last: "1, 2 and 3".
fn and_list(mut items: Vec<Message>) -> Option<Message> {
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

/// A relief (docs/specs/stl-relief.md §5, §7): its model placed and indexed, finishing lines
/// across the area, and the path checked against the model.
fn plan_relief_op(
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
    if let (Some(finish), Some(rough)) = (ctx.finish_tool, op.roughing) {
        return plan_relief_roughing(op, ctx, sel, start, shift, &mesh, cutter, finish, rough);
    }
    let index = crate::drop::DropMesh::new(&mesh, ctx.tool.diameter);
    let (start_depth, end_depth) = (op.start_depth, op.end_depth);
    let floor = ctx.top_z - (end_depth - start_depth);
    let stock_top = ctx.top_z + start_depth;
    let (area, ramp) = relief_area(&sel.contours, op.rim.as_ref(), floor, stock_top);
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
    };
    let (moves, end) = crate::relief::relief_moves(&cut, start);
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

/// A relief's roughing pass (docs/specs/stl-relief.md §14 step 3): the tool grown by the
/// allowance, its area shrunk so it never cuts wider than the finish, level by level, and
/// the path checked to keep the allowance.
#[allow(clippy::too_many_arguments)]
fn plan_relief_roughing(
    op: &chipload_job::ReliefOp,
    ctx: &Context,
    sel: &Selection,
    start: Point,
    shift: Point,
    mesh: &chipload_mesh::Mesh,
    cutter: chipload_core::cutter::Cutter,
    finish: &Tool,
    rough: chipload_job::ReliefRoughing,
) -> Result<Planned, ToolpathError> {
    let allowance = rough.allowance;
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
            let (area, ramp) = relief_area(&sel.contours, Some(rim), floor, stock_top + allowance);
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
        step: rough.stepover * ctx.tool.diameter,
        angle: op.angle,
        zigzag: op.zigzag,
        safe_z: ctx.job.machine.safe_z,
        feed: rough.feed,
        plunge_feed: rough.plunge_feed,
        shift,
    };
    let (moves, end) = crate::relief::rough_moves(&cut, stock_top, rough.step_down, start);
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
    Ok(Planned {
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
) -> (Area, Option<Ramp>) {
    let loops: Vec<Vec<Point>> = contours
        .iter()
        .filter(|c| c.closed)
        .map(|c| c.points())
        .collect();
    let Some(rim) = rim else {
        return (Area::Loops(loops), None);
    };
    // The outer loops: inside an even number of others. Holes are cut to the floor.
    let outer: Vec<Vec<Point>> = loops
        .iter()
        .enumerate()
        .filter(|&(i, l)| {
            let Some(&p) = l.first() else {
                return false;
            };
            let around = loops
                .iter()
                .enumerate()
                .filter(|&(j, other)| j != i && crate::relief::encloses(other, p))
                .count();
            around % 2 == 0
        })
        .map(|(_, l)| l.clone())
        .collect();
    let (loops, foot) = match rim.shape {
        RimShape::Outline => (outer, rim.width),
        // The rectangle the corner radius inside the flat's edge, grown by it again.
        RimShape::Box => {
            let area = Area::Loops(outer);
            let corner = rim.corner_radius.clamp(0.0, rim.width.max(0.0));
            let loops = area.bounds().map_or_else(Vec::new, |(lo, hi)| {
                let g = rim.width - corner;
                let (x0, y0, x1, y1) = (lo.0 - g, lo.1 - g, hi.0 + g, hi.1 + g);
                vec![vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]]
            });
            (loops, corner)
        }
    };
    let ramp = Ramp {
        loops: loops.clone(),
        foot,
        tan: rim.ramp.tan(),
        floor,
        top,
    };
    let area = Area::Grown {
        loops,
        reach: ramp.reach(),
    };
    (area, Some(ramp))
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

/// A relief's roughing pass (docs/specs/stl-relief.md §14 step 3): it cuts with a project
/// end mill or plain ball nose with a spindle speed.
fn roughing_checks(
    job: &Job,
    index: usize,
    rough: &chipload_job::ReliefRoughing,
) -> (Vec<Problem>, Vec<Note>) {
    let (mut problems, mut notes) = (Vec::new(), Vec::new());
    let label_message = job.label_message(OpRef::pass(index, 0));
    let at = |field: &str| format!("operations.{index}.roughing.{field}");
    let Some(tool) = job.tool(rough.tool) else {
        problems.push(labelled(
            at("tool"),
            &label_message,
            Message::new("toolpath-rest-pass-no-tool").arg("tool", rough.tool),
        ));
        return (problems, notes);
    };
    if tool
        .cutter()
        .and_then(|c| c.grown(rough.allowance))
        .is_none()
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

/// A surface needs an endmill (docs/specs/surfacing.md §4). It never plunges into
/// material.
fn surface_tool_rule(tool: &Tool) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Endmill => Ok(false),
        other => Err(Message::new("toolpath-surface-endmill")
            .arg("tool", tool.id)
            .arg("type", other.name_message())),
    }
}

/// A surface after other operations of its part faces what they already cut
/// (docs/specs/surfacing.md §6): a warning, as it is usually put first.
fn surface_order_note(job: &Job, index: usize, label: &Message) -> Option<Message> {
    let part = job.operations.get(index)?.part();
    let before: Vec<Message> = (0..index)
        .filter(|&i| job.cuts(i) && job.operations[i].part() == part)
        .filter(|&i| !matches!(job.operations[i], Operation::Surface(_)))
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

/// Rest machining's rules (docs/specs/rest-machining.md §5) for the pocket at `index`,
/// which cuts with `tool`: a problem's text, or the warnings.
fn rest_check(
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
fn rest_pass_checks(job: &Job, index: usize, pocket: &PocketOp) -> (Vec<Problem>, Vec<Note>) {
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
fn plug_face(job: &Job, pocket: &VcarveOp, angle: f64) -> Option<f64> {
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
fn plug_checks(
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
    let deepest = (bit.diameter - tip) / 2.0 / (angle / 2.0).tan();
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
        let short = tip / 2.0 / std::f64::consts::FRAC_PI_4.sin();
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
fn floor_pass_checks(job: &Job, index: usize, floor: &ClearingPass) -> (Vec<Problem>, Vec<Note>) {
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
fn tool_group_notes(job: &Job) -> Vec<Note> {
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
fn vcarve_tool_notes(label: &Message, tool: &Tool) -> Result<Vec<Message>, Message> {
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
fn pocket_tool_notes(tool: &Tool, entry: Entry) -> Result<bool, Message> {
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
fn path_tool_notes(tool: &Tool, entry: Entry) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Drill => Err(Message::new("toolpath-path-drill").arg("tool", tool.id)),
        ToolType::Vbit | ToolType::Ballnose => Ok(false),
        ToolType::Endmill => plunge_unknown(tool, entry),
    }
}

/// A drill drills and an endmill bores (docs/specs/holes.md §1, §2): the method must match
/// the tool; a V-bit can't cut holes.
fn holes_tool_notes(tool: &Tool, method: HoleMethod) -> Result<(), Message> {
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
fn flank_notes(number: usize, tool: &Tool, op: &ProfileOp) -> Result<Vec<Message>, Message> {
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
fn short_ramp_refusal(tool: &Tool, length: f64) -> Result<(), Message> {
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
fn plunge_unknown(tool: &Tool, entry: Entry) -> Result<bool, Message> {
    if entry.ramp_angle().is_some() {
        return Ok(false);
    }
    match tool.center_cutting {
        Some(false) => Err(Message::new("toolpath-plunge-not-centre-cutting").arg("tool", tool.id)),
        None => Ok(true),
        Some(true) => Ok(false),
    }
}
