// SPDX-License-Identifier: GPL-3.0-or-later
//! Job + imported contours -> Program: the toolpath half of `chipload_pipeline::run_job`,
//! which then posts and independently checks it.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Line, Primitive};
use chipload_core::text::mm;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    Copies, CopyOrder, Entry, HoleMethod, HolesOp, Job, MAX_LEAD_DIAMETERS, Note, OpRef, Operation,
    Part, PathOp, PocketOp, Problem, ProfileOp, RestRefusal, Side, SurfaceOp, Tabs, Tool, ToolType,
    VcarveOp, ZZero, rest_refusal,
};

use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM, DEPTH_TOL_MM};

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
use crate::select::{Selection, select_contours, select_paths};
use crate::surface::{SurfaceCut, surface_moves};
use crate::tabs::{TabLayout, place_tabs, tab_polylines};
use crate::vcarve::{VBit, carve_levels, plan_vcarve, vcarve_moves};

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub program: Program,
    /// Warnings for the user, each with the operation it is about.
    pub notes: Vec<Note>,
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
    let mut notes = validate_job(job)?;
    let chosen = select_all(job, contours)?;
    let (steps, mut groups) = cut_order(job, &chosen)?;
    let mut gathered = Gathered::default();
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
            notes.extend(
                selection
                    .notes
                    .iter()
                    .map(|n| Note::at(at.operation, labelled_note(label, n.clone()))),
            );
        }
        let planned = plan_step(job, contours, cancel, &chosen[step.chosen], step, position)?;
        position = planned.end;
        gathered.add(job, step, *at, tool.diameter / 2.0, planned, &mut notes);
        groups[step.group].moves.end = gathered.moves.len();
    }
    let Gathered {
        mut moves,
        tabs,
        pockets,
        op_spans,
        ring_starts,
        unreached_corners,
        arcs,
        comments,
        ..
    } = gathered;
    check_parts_apart(job, &moves, &op_spans)?;
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
        tabs,
        pockets,
        op_spans,
        ring_starts,
        unreached_corners,
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
            Operation::Path(_) | Operation::Holes(_) => {
                select_paths(contours, op.select(), &label_message)
            }
            // The whole stock, no shapes (docs/specs/surfacing.md §2).
            Operation::Surface(_) => Ok(Selection {
                contours: Vec::new(),
                notes: Vec::new(),
            }),
            _ => select_contours(contours, op.select(), &label_message),
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
        // A V-carve sets its own passes from how deep it goes (docs/specs/v-carve.md §3).
        Operation::Vcarve(_) => Vec::new(),
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
) -> Result<Planned, ToolpathError> {
    let index = chosen.at.operation;
    let op = &chosen.op;
    let tool = op_tool(job, op).map_err(|err| err.at(index))?;
    let top_z = job.stock.top_z();
    let pass_z = pass_heights(op, top_z).map_err(|err| err.at(index))?;
    let (start_depth, _) = op.depths();
    let shift = step.shift;
    // Leads keep clear of every contour of the drawing (moved with the copy).
    // Dogbone legs keep clear of them too (docs/specs/dogbones.md §6).
    let everything: Vec<Contour> = if op.lead().is_some() || op.has_bones() {
        contours.iter().map(|c| c.translated(shift)).collect()
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
    let context = Context {
        label_message: &chosen.label_message,
        tool,
        job,
        pass_z: &pass_z,
        top_z: top_z - start_depth,
        everything: &everything,
        choices: &choices,
        cancel,
    };
    let moved: Vec<Contour> = chosen
        .selection
        .contours
        .iter()
        .map(|c| c.translated(shift))
        .collect();
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
    };
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
    tabs: Vec<Vec<Point>>,
    pockets: Vec<PocketPiece>,
    op_spans: Vec<OpSpan>,
    ring_starts: Vec<RingStart>,
    unreached_corners: Vec<(usize, Point)>,
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
            job.stock.top_z(),
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
        self.tabs.extend(planned.tabs);
        self.pockets.extend(planned.pockets);
        // Start markers are drawn on the part: the copies' are the same. A rest pass's rings
        // start where the planner puts them: starts belong to the pocket.
        if step.copy == 1 && at.pass.is_none() {
            self.unreached_corners
                .extend(planned.unreached.iter().map(|&p| (index, p)));
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
    /// Every contour of the drawing, moved with the copy; only filled for leads and bones.
    everything: &'a [Contour],
    /// The operation's corner choices, moved with the copy.
    choices: &'a [chipload_job::CornerChoice],
    cancel: &'a Cancel,
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
        spindle: ctx.job.machine.spindle,
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
            everything: ctx.everything,
            // Another boundary of the pocket has waste on both sides near a corner.
            exempt: &boundaries,
            cleared: &selection.contours,
            label: ctx.label_message,
        };
        for piece in &mut plan.pieces {
            let unreached = pocket_legs(piece, &selection.contours, &boundaries, &cut)?;
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
    };
    let cut = PocketCut {
        pass_z: ctx.pass_z,
        top_z: ctx.top_z,
        safe_z: ctx.job.machine.safe_z,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        entry: op.entry,
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

/// Dogbones in a pocket piece's wall rings (docs/specs/dogbones.md §4): the corners of the
/// piece's boundaries (cut inside) and islands (cut outside); the leads' ranges move with
/// the added points. Returns the corners no wall ring reaches.
fn pocket_legs(
    piece: &mut PocketPiece,
    contours: &[&Contour],
    boundaries: &[usize],
    cut: &CornerCut,
) -> Result<Vec<(crate::corners::Corner, crate::corners::CornerBone)>, ToolpathError> {
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
    Ok(added.unreached)
}

/// Dogbones in a profile's rings (docs/specs/dogbones.md §4), each contour's corners in its
/// own rings, reaching the corner moved by `cut.clearance`.
fn profile_legs(
    rings: &mut [PathRing],
    contours: &[&Contour],
    side: Side,
    cut: &CornerCut,
) -> Result<Vec<(crate::corners::Corner, crate::corners::CornerBone)>, ToolpathError> {
    let mut unreached = Vec::new();
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
    }
    Ok(unreached)
}

fn plan_path_op(
    op: &PathOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let mut arcs = ArcTable::default();
    let mut notes = Vec::new();
    let paths = order_paths(
        &selection.contours,
        start,
        ctx.job.machine.spindle,
        &mut arcs,
        &mut notes,
    );
    let mut notes: Vec<Message> = notes
        .into_iter()
        .map(|n| labelled_note(ctx.label_message, n))
        .collect();
    if let Some(note) = groove_note(ctx.tool, op.end_depth - op.start_depth) {
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
        spindle: ctx.job.machine.spindle,
    };
    let mut moves = Vec::new();
    let mut end = start;
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
    })
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
            if op.end_depth < ctx.job.stock.thickness - DEPTH_TOL_MM {
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
            HoleMethod::Bore { .. } => Message::new("toolpath-holes-too-small-to-bore"),
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
    if holes.is_empty() {
        return Err(job_error(labelled_note(
            ctx.label_message,
            Message::new("toolpath-error-no-hole-fits")
                .arg("tool", tool.id)
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
                    spindle: ctx.job.machine.spindle,
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
    })
}

/// A surface (docs/specs/surfacing.md): the whole stock, back and forth along its longer
/// side, every depth started beside it.
fn plan_surface_op(op: &SurfaceOp, ctx: &Context, start: Point) -> Planned {
    let cut = SurfaceCut {
        stock: ctx.job.stock.xy_bounds(),
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
    }
}

/// How wide a V-bit's groove is at the top, `depth` deep (docs/specs/follow-path.md §5).
fn groove_note(tool: &Tool, depth: f64) -> Option<Message> {
    if tool.kind != ToolType::Vbit {
        return None;
    }
    let Some(angle) = tool.angle else {
        return Some(Message::new("toolpath-groove-no-angle").arg("tool", tool.id));
    };
    let tip = tool.tip_diameter.unwrap_or(0.0);
    let width = tip + 2.0 * depth * (angle / 2.0).tan();
    Some(if width > tool.diameter {
        Message::new("toolpath-groove-past-cone").arg("width", Value::mm(tool.diameter))
    } else {
        Message::new("toolpath-groove-width").arg("width", Value::mm(width))
    })
}

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
    let plan = plan_vcarve(
        &selection.contours,
        &bit,
        op.end_depth - op.start_depth,
        start,
    )
    .map_err(|err| err.with_context(ctx.label_message.clone()))?;
    let levels = carve_levels(plan.deepest, op.step_down)?;
    let moves = vcarve_moves(
        &plan.paths,
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
        notes: plan
            .notes
            .into_iter()
            .map(|n| labelled_note(ctx.label_message, n))
            .collect(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: ArcTable::default(),
        end,
        starts: Vec::new(),
        unreached: Vec::new(),
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
        machine.spindle,
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
            machine.spindle,
            start,
            starts,
        )?),
        None => None,
    };
    let mut rough = rough;
    let mut unreached_at = Vec::new();
    // Dogbones go into every ring before the tabs are placed (docs/specs/dogbones.md §4).
    if op.corners.is_some() || !ctx.choices.is_empty() {
        let cut = CornerCut {
            corners: op.corners,
            choices: ctx.choices,
            tool_radius: radius,
            clearance: op.clearance,
            everything: ctx.everything,
            exempt: &[],
            cleared: &[],
            label: ctx.label_message,
        };
        let unreached = profile_legs(&mut plan.rings, &selection.contours, op.side, &cut)?;
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
    let mut layouts: Vec<Option<TabLayout>> = Vec::with_capacity(rings.len());
    let mut curved: Vec<usize> = Vec::new();
    for ring in rings {
        // Validated: count >= 1.
        let count = usize::try_from(tabs.count).unwrap_or(0);
        let placed = place_tabs(&ring.points, count, tabs.width, ctx.tool.diameter)
            .map_err(|err| err.with_context(contour_context(ctx.label_message, ring.contour_id)))?;
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
    let top = job.stock.top_z();
    let z_zero = match job.stock.z_zero {
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
        };
        // Below the stock: a warning within the overcut allowance, an error beyond it
        // (docs/specs/depths-and-clearance.md section 2).
        let (_, end_depth) = op.depths();
        let thickness = job.stock.thickness;
        let below = end_depth - thickness;
        if below > job.stock.max_overcut + DEPTH_TOL_MM {
            problems.push(labelled(
                at("end_depth"),
                &label_message,
                Message::new("toolpath-too-deep")
                    .arg("end", length(end_depth))
                    .arg("below", Value::Length(below, Digits::Fixed(3)))
                    .arg("thickness", length(thickness))
                    .arg("allowed", length(job.stock.max_overcut)),
            ));
        } else if below > DEPTH_TOL_MM {
            notes.push(Note::at(
                index,
                labelled_note(
                    &label_message,
                    Message::new("toolpath-into-spoilboard")
                        .arg("below", Value::Length(below, Digits::Fixed(3))),
                ),
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

/// A follow path takes an endmill (which must be centre-cutting to plunge) or a V-bit
/// (centre-cutting by its shape); a drill can't follow a path (docs/specs/follow-path.md §5).
fn path_tool_notes(tool: &Tool, entry: Entry) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Drill => Err(Message::new("toolpath-path-drill").arg("tool", tool.id)),
        ToolType::Vbit => Ok(false),
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
