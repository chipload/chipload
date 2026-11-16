// SPDX-License-Identifier: GPL-3.0-or-later
//! Job + imported contours -> Program: the toolpath half of `chipload_pipeline::run_job`,
//! which then posts and independently checks it.
//!
//! Here: selection, cut order and the planning loop. Each operation kind is planned in its
//! `op_*` module, instances are moved in `transform`, parts kept apart in `apart`, and the
//! generation-time rules live in `check`, `check_passes` and `check_tools`.

use std::collections::{BTreeMap, BTreeSet};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    HoleMethod, HolesOp, InstanceOrder, Job, Mirror, Note, OpRef, Operation, Part, PathOp,
    PocketOp, ProfileOp, Spindle, SurfaceOp, Tool,
};

use crate::ToolpathError;
use crate::arcs::{ArcTable, tag_moves};
use crate::cancel::Cancel;
use crate::corners::unmatched_choice;
use crate::model::{Move, Program};
use crate::place::Placement;
use crate::pocket::PocketPiece;
use crate::profile::{labelled_note, pass_depths};
use crate::select::{Selection, select_contours, select_paths};

mod apart;
mod check;
mod check_passes;
mod check_tools;
mod op_holes;
mod op_pocket;
mod op_profile;
mod op_relief;
mod op_shapes;
mod op_vcarve;
mod transform;

use apart::check_parts_apart;
use check::through_overcut;
use op_holes::plan_holes_op;
use op_pocket::plan_pocket_op;
use op_profile::plan_profile_op;
use op_relief::plan_relief_op;
use op_shapes::{
    plan_chamfer_op, plan_flute_op, plan_moulding_op, plan_path_op, plan_pattern_op, plan_prism_op,
    plan_surface_op,
};
use op_vcarve::{groove_width, plan_vcarve_op};
use transform::{
    lower_approaches, mirrored_planned, mirrored_point, placed_planned, plug_board, turned_sense,
};

pub use apart::cut_extent;
pub use check::{job_problems, validate_job};
pub use check_tools::tool_groups;
pub use op_vcarve::{groove_width_mm, vbit_cone_depth};

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
    // Each step's share of the progress (§7 of background-generation.md): a relief's far
    // larger than another operation's.
    let weights: Vec<f64> = steps
        .iter()
        .map(|step| match chosen[step.chosen].op {
            Operation::Relief(_) => RELIEF_WEIGHT,
            _ => 1.0,
        })
        .collect();
    let total: f64 = weights.iter().sum();
    let mut before = 0.0;
    for (step, weight) in steps.iter().zip(&weights) {
        cancel.check()?;
        if total > 0.0 {
            cancel.set_stage(before / total, weight / total);
        }
        before += weight;
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
            if let Operation::Pattern(f) = op
                && let Some(width) = crate::flute::flute_width_mm(tool, f.end_depth - f.start_depth)
            {
                let width = Message::new("toolpath-pattern-width").arg("width", Value::mm(width));
                facts.push(Note::at(at.operation, labelled_note(label, width)));
            }
            if let Operation::Flute(f) = op
                && let Some(width) = crate::flute::flute_width_mm(tool, f.end_depth - f.start_depth)
            {
                let width = Message::new("toolpath-flute-width").arg("width", Value::mm(width));
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
    cancel.set_stage(0.0, 1.0);
    cancel.stage_done(1.0);
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
            Operation::Path(_)
            | Operation::Flute(_)
            | Operation::Moulding(_)
            | Operation::Holes(_) => select_paths(
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
/// A relief step's share of a run's progress against another operation's 1: its lines
/// take far longer than any other operation (docs/specs/background-generation.md §7).
const RELIEF_WEIGHT: f64 = 20.0;

struct Step {
    /// Index in the chosen operations.
    chosen: usize,
    /// Which copy of the part (docs/specs/copies-grid.md): 1 is the drawing as drawn.
    copy: u32,
    /// How far the copy is moved from the drawing: an instance only moved is planned on
    /// its moved contours.
    shift: Point,
    /// A turned or mirrored instance (docs/specs/instances.md §3): planned on the part as
    /// drawn, then put in its place. None for the part and instances only moved.
    place: Option<Placement>,
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
            let copies = part.map_or_else(|| vec![(1, (0.0, 0.0), None)], |p| spots_of(job, p));
            let step = |i: usize, (copy, shift, place): Spotted| Step {
                chosen: i,
                copy,
                shift,
                place,
                group,
            };
            // A surface faces the stock once, whatever the part's copies
            // (docs/specs/surfacing.md §2); dowel holes belong to the board, cut once where
            // drawn, never placed (docs/specs/part-placement.md §5).
            let copies_of = |i: usize| -> Vec<Spotted> {
                match &chosen[i].op {
                    Operation::Surface(_) => vec![(1, (0.0, 0.0), None)],
                    op if op.is_dowels() => vec![(1, (0.0, 0.0), None)],
                    _ => copies.clone(),
                }
            };
            match part.map(|p| p.order) {
                Some(InstanceOrder::ByInstance) => {
                    for &copy in &copies {
                        steps.extend(
                            run.iter()
                                .filter(|&&i| copies_of(i).iter().any(|c| c.0 == copy.0))
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

/// A part or one of its instances in cut order: its number, the move it is planned with,
/// and for a turned or mirrored instance where it is put after planning.
type Spotted = (u32, Point, Option<Placement>);

/// The part and its instances that are on, in cut order (docs/specs/instances.md §2), the
/// part itself at its own placement (docs/specs/part-placement.md §3). An instance only
/// moved is planned on its moved contours, as copies were, so its G-code stays as it was;
/// a turned or mirrored one is placed about the part's centre.
fn spots_of(job: &Job, part: &Part) -> Vec<Spotted> {
    let centre = job.part_centre(part.id).unwrap_or((0.0, 0.0));
    part.spots()
        .into_iter()
        .map(|(number, spot)| {
            if spot.is_move() {
                (number, (spot.at[0], spot.at[1]), None)
            } else {
                (number, (0.0, 0.0), Some(Placement::new(&spot, centre)))
            }
        })
        .collect()
}

/// The Z of every depth pass of `op`, going down from the stock top at `top_z`.
fn pass_heights(op: &Operation, tools: &[Tool], top_z: f64) -> Result<Vec<f64>, ToolpathError> {
    let (start_depth, end_depth) = op.depths(tools);
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
        // One pass, or equal passes with a step down (docs/specs/chamfer.md §4).
        Operation::Chamfer(c) => match c.step_down {
            Some(step) => heights(pass_depths(start_depth, end_depth, step)?),
            None => vec![top_z - end_depth],
        },
        // The same (docs/specs/fluting.md §4).
        Operation::Flute(f) => match f.step_down {
            Some(step) => heights(pass_depths(start_depth, end_depth, step)?),
            None => vec![top_z - end_depth],
        },
        // Its passes follow from its deepest edge (docs/specs/prism.md §3).
        Operation::Prism(_) => Vec::new(),
        // Its levels follow from the section (docs/specs/moulding.md §6).
        Operation::Moulding(_) => Vec::new(),
        // The same (docs/specs/fill-pattern.md §4).
        Operation::Pattern(f) => match f.step_down {
            Some(step) => heights(pass_depths(start_depth, end_depth, step)?),
            None => vec![top_z - end_depth],
        },
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
    let pass_z = pass_heights(op, &job.tools, top_z).map_err(|err| err.at(index))?;
    let (start_depth, _) = op.depths(&job.tools);
    let shift = step.shift;
    // A turned or mirrored instance is planned on the part as drawn, then put in its place
    // (docs/specs/instances.md §3): where the tool is, seen from the part.
    let place = step.place;
    let position = place.map_or(position, |p| p.inverse(position));
    let placed_from = position;
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
                let theirs = !own.is_some_and(|ids| ids.contains(&c.id));
                // What other parts cut, seen from the instance's part as drawn.
                let moved = match place {
                    Some(p) if theirs => p.inverse_contour(c),
                    _ => c.translated(shift),
                };
                match mirror {
                    Some((m, at)) if theirs => moved.mirrored(m == Mirror::LeftRight, at),
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
            .is_relief_pass(chosen.at)
            .then(|| job.tool(job.operations[chosen.at.operation].tool()))
            .flatten(),
        // A mirrored instance is mirrored after planning, as a mirrored part is: planned
        // the other way round, climb stays climb.
        sense: match place.is_some_and(|p| p.mirrors()) {
            true => turned_sense(job.cutting_sense_of(op)),
            false => job.cutting_sense_of(op),
        },
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
        Operation::Chamfer(chamfer) => plan_chamfer_op(chamfer, &context, &shifted, position),
        Operation::Flute(flute) => plan_flute_op(flute, &context, &shifted, position),
        Operation::Prism(prism) => plan_prism_op(prism, &context, &shifted, position),
        Operation::Moulding(moulding) => plan_moulding_op(moulding, &context, &shifted, position),
        Operation::Pattern(pattern) => {
            plan_pattern_op(pattern, &context, &shifted, position, shift)
        }
        Operation::Holes(holes) => plan_holes_op(holes, &context, &shifted, position),
        Operation::Surface(surface) => Ok(plan_surface_op(surface, &context, position)),
        Operation::Relief(relief) => plan_relief_op(relief, &context, &shifted, position, shift),
    };
    let planned = planned.map(|planned| match mirror {
        Some(mirror) => mirrored_planned(planned, mirror),
        None => planned,
    });
    let planned = planned.map(|planned| match &place {
        Some(place) => placed_planned(planned, place, placed_from),
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
            // How many times it is cut: the part and its instances that are on
            // (docs/specs/instances.md §7).
            let copies = match part.count() {
                1 => String::new(),
                n => format!(", cut {n} times"),
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

fn job_error(message: Message) -> ToolpathError {
    ToolpathError::Job(message)
}

/// The tool an operation names (docs/specs/tool-per-operation.md §4).
pub fn op_tool<'a>(job: &'a Job, op: &Operation) -> Result<&'a Tool, ToolpathError> {
    job.tool(op.tool())
        .ok_or_else(|| job_error(Message::new("toolpath-tool-undefined").arg("tool", op.tool())))
}
