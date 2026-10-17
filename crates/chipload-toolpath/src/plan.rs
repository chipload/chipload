// SPDX-License-Identifier: GPL-3.0-or-later
//! Job + imported contours -> Program: the toolpath half of `chipload_pipeline::run_job`,
//! which then posts and independently checks it.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Line, Primitive};
use chipload_core::text::mm;
use chipload_job::{
    Copies, CopyOrder, Entry, HoleMethod, HolesOp, Job, MAX_LEAD_DIAMETERS, Note, OpRef, Operation,
    Part, PathOp, PocketOp, Problem, ProfileOp, RestRefusal, Side, Tabs, Tool, ToolType, VcarveOp,
    ZZero, rest_refusal,
};

use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM, DEPTH_TOL_MM};

use crate::ToolpathError;
use crate::arcs::{ArcTable, tag_moves};
use crate::cancel::Cancel;
use crate::corners::{CornerCut, RingSlot, add_legs, find_corners, unreached_note};
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
use crate::profile::{PathRing, pass_depths, plan_profile};
use crate::select::{Selection, select_contours, select_paths};
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
            at, op, selection, ..
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
                    .map(|n| Note::at(at.operation, n.clone())),
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
        arcs,
        comments,
        ..
    } = gathered;
    check_parts_apart(job, &moves, &op_spans)?;
    let first = groups
        .first()
        .ok_or_else(|| job_error("Every operation is disabled: nothing to cut".into()))?;
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
        groups,
    })
}

/// An operation or rest pass that is cut, with its selection, chosen once on the drawing as
/// drawn.
struct Chosen<'a> {
    at: OpRef,
    /// The operation; for a rest pass, the pocket it is (Job::resolve).
    op: Operation,
    label: String,
    selection: Selection<'a>,
}

/// Every cut operation's and rest pass's selection (docs/specs/v0-profile-pipeline.md §3),
/// in the job's cut list (docs/specs/rest-passes.md §3).
fn select_all<'a>(job: &Job, contours: &'a [Contour]) -> Result<Vec<Chosen<'a>>, ToolpathError> {
    let mut chosen = Vec::new();
    // Operations that aren't cut are kept in the file and skipped here; numbering stays as
    // listed.
    for at in job.cut_list() {
        let op = job
            .resolve(at)
            .ok_or_else(|| job_error(format!("No operation {at} in the job")))?;
        let label = job.label(at);
        let selection = match op {
            // Holes take every matching shape and skip what isn't a circle with a note.
            Operation::Path(_) | Operation::Holes(_) => select_paths(contours, op.select(), &label),
            _ => select_contours(contours, op.select(), &label),
        }
        .map_err(|err| err.at(at.operation))?;
        chosen.push(Chosen {
            at,
            op,
            label,
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
        let tool = job
            .tool(tool_id)
            .ok_or_else(|| job_error(format!("Tool {tool_id} is not defined in the job")))?;
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
            match part.and_then(|p| p.copies).map(|c| c.order) {
                Some(CopyOrder::ByCopy) => {
                    for &copy in &copies {
                        steps.extend(run.iter().map(|&i| step(i, copy)));
                    }
                }
                _ => {
                    for &i in &run {
                        steps.extend(copies.iter().map(|&copy| step(i, copy)));
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
        | Operation::Path(PathOp { step_down, .. }) => {
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
    let everything: Vec<Contour> = if op.lead().is_some() || op.corners().is_some() {
        contours.iter().map(|c| c.translated(shift)).collect()
    } else {
        Vec::new()
    };
    let context = Context {
        label: &chosen.label,
        tool,
        job,
        pass_z: &pass_z,
        top_z: top_z - start_depth,
        everything: &everything,
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
    };
    planned.map_err(|err| {
        let part = job.part_of(op);
        let several = part.is_some_and(|p| p.count() > 1);
        let err = if several && job.parts.len() > 1 {
            let name = part.map_or("", |p| p.name.as_str());
            err.with_context(&format!("part {name}, copy {}", step.copy))
        } else if several {
            err.with_context(&format!("copy {}", step.copy))
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
    arcs: ArcTable,
    /// With two parts or more, a comment where each part starts (docs/specs/parts.md §6).
    comments: Vec<String>,
    part_started: Option<u64>,
    /// Operations planned at least once: their selection's notes are listed once.
    started: Vec<OpRef>,
    /// Every copy repeats the original's notes: each is listed once per operation.
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
            let key = (at, note);
            if !self.seen.contains(&key) {
                notes.push(Note::at(index, key.1.clone()));
                self.seen.push(key);
            }
        }
        self.tabs.extend(planned.tabs);
        self.pockets.extend(planned.pockets);
        // Start markers are drawn on the part: the copies' are the same. A rest pass's rings
        // start where the planner puts them: starts belong to the pocket.
        if step.copy == 1 && at.pass.is_none() {
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
    label: &'a str,
    tool: &'a Tool,
    job: &'a Job,
    pass_z: &'a [f64],
    /// Z of the operation's start depth: cut already, where a ramp begins.
    top_z: f64,
    /// Every contour of the drawing, moved with the copy; only filled for leads.
    everything: &'a [Contour],
    cancel: &'a Cancel,
}

/// One operation's moves, warnings and tabs, and where the tool ends up (above that point).
struct Planned {
    moves: Vec<Move>,
    notes: Vec<String>,
    tabs: Vec<Vec<Point>>,
    pockets: Vec<PocketPiece>,
    arcs: ArcTable,
    end: Point,
    /// Rings whose start can be moved, and the stored point that set it.
    starts: Vec<(Vec<Point>, Option<usize>)>,
}

fn plan_pocket_op(
    op: &PocketOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    starts: &[Point],
) -> Result<Planned, ToolpathError> {
    let label = ctx.label;
    // Checked before planning (check_job); never fall back to the whole pocket.
    let rest = match op.rest {
        Some(rest) => Some(
            ctx.job
                .tool(rest.after_tool)
                .map(|t| t.diameter / 2.0)
                .ok_or_else(|| {
                    job_error(format!(
                        "{label}: rest after tool {}, which the project doesn't have",
                        rest.after_tool
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
    let plan = plan_pocket(&request).map_err(|err| err.with_context(label))?;
    let mut plan = plan;
    let mut corner_notes = Vec::new();
    if let Some(style) = op.corners {
        let boundaries = pocket_boundaries(&selection.contours);
        let cut = CornerCut {
            style,
            tool_radius: ctx.tool.diameter / 2.0,
            clearance: op.clearance,
            everything: ctx.everything,
            // Another boundary of the pocket has waste on both sides near a corner.
            exempt: &boundaries,
            label,
        };
        for piece in &mut plan.pieces {
            let unreached = pocket_legs(piece, &selection.contours, &boundaries, &cut)?;
            corner_notes.extend(unreached.iter().map(|c| unreached_note(label, c, style)));
        }
    }
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan
            .notes
            .iter()
            .map(|n| format!("{label}: {n}"))
            .chain(corner_notes)
            .collect(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: plan.arcs,
        end: start,
        starts: Vec::new(),
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
        out.moves
            .extend(pocket_moves(piece, &cut).map_err(|err| err.with_context(label))?);
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
) -> Result<Vec<crate::corners::Corner>, ToolpathError> {
    let mut corners = Vec::new();
    for contour in contours
        .iter()
        .filter(|c| piece.contour_ids.contains(&c.id))
    {
        let is_boundary = boundaries.contains(&contour.id);
        // A corner inside another of the operation's boundaries is in waste that is cleared
        // anyway; one inside another island is no corner of the wall.
        let inside_another = |p: Point| {
            contours.iter().any(|o| {
                o.id != contour.id
                    && boundaries.contains(&o.id) == is_boundary
                    && crate::corners::encloses(o, p)
            })
        };
        corners.extend(
            find_corners(contour, is_boundary, cut.style.max_angle())
                .into_iter()
                .filter(|c| !inside_another(c.at)),
        );
    }
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

/// The pocket's boundaries among its contours: those wholly inside an even number of the
/// others (islands are inside an odd number; crossing contours don't count), as the pocket
/// builds its areas.
fn pocket_boundaries(contours: &[&Contour]) -> Vec<usize> {
    contours
        .iter()
        .filter(|contour| {
            let points = contour.points();
            let around = contours
                .iter()
                .filter(|o| {
                    o.id != contour.id && points.iter().all(|&p| crate::corners::encloses(o, p))
                })
                .count();
            around % 2 == 0
        })
        .map(|c| c.id)
        .collect()
}

/// Dogbones in a profile's rings (docs/specs/dogbones.md §4), each contour's corners in its
/// own rings, reaching the corner moved by `cut.clearance`.
fn profile_legs(
    rings: &mut [PathRing],
    contours: &[&Contour],
    side: Side,
    cut: &CornerCut,
) -> Result<Vec<crate::corners::Corner>, ToolpathError> {
    let mut unreached = Vec::new();
    for contour in contours {
        let corners = find_corners(contour, side == Side::Inside, cut.style.max_angle());
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
    let label = ctx.label;
    let mut arcs = ArcTable::default();
    let mut notes = Vec::new();
    let paths = order_paths(
        &selection.contours,
        start,
        ctx.job.machine.spindle,
        &mut arcs,
        &mut notes,
    );
    let mut notes: Vec<String> = notes.iter().map(|n| format!("{label}: {n}")).collect();
    if let Some(note) = groove_note(ctx.tool, op.end_depth - op.start_depth) {
        notes.push(format!("{label}: {note}"));
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
            .map_err(|err| err.with_context(&format!("{label}, contour #{}", path.contour_id)))?;
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
    })
}

fn plan_holes_op(
    op: &HolesOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let label = ctx.label;
    let tool = ctx.tool;
    let (found, others) = find_holes(&selection.contours);
    let mut notes = Vec::new();
    if others > 0 {
        let what = if others == 1 {
            "1 shape that isn't a circle".to_string()
        } else {
            format!("{others} shapes that aren't circles")
        };
        notes.push(format!("{label}: skipped {what}"));
    }
    let tool_radius = tool.diameter / 2.0;
    // Holes this tool can't cut, by diameter: (diameter text, count).
    let mut unfit: std::collections::BTreeMap<String, usize> = Default::default();
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
            *unfit.entry(diameter).or_default() += 1;
            continue;
        }
        if let HoleMethod::Bore { .. } = op.method
            && bore_radius(&hole, tool_radius) > tool_radius + CHAIN_TOL_MM
        {
            // A core is left standing inside the helix's ring (docs/specs/holes.md §4).
            if op.end_depth < ctx.job.stock.thickness - DEPTH_TOL_MM {
                return Err(job_error(format!(
                    "{label}: hole {diameter} mm is blind and more than twice the {} mm tool; \
                     use a pocket",
                    mm(tool.diameter)
                )));
            }
            cores += 1;
        }
        holes.push(hole);
    }
    for (diameter, count) in &unfit {
        let holes_text = if *count == 1 {
            format!("hole {diameter} mm")
        } else {
            format!("{count} holes of {diameter} mm")
        };
        notes.push(match op.method {
            HoleMethod::Drill { .. } => format!(
                "{label}: {holes_text}: not the {} mm drill's size (within {} mm); not drilled",
                mm(tool.diameter),
                mm(DRILL_FIT_MM)
            ),
            HoleMethod::Bore { .. } => format!(
                "{label}: {holes_text}: too small for the {} mm tool to bore; not cut",
                mm(tool.diameter)
            ),
        });
    }
    if cores > 0 {
        let holes_text = if cores == 1 {
            "1 hole leaves".to_string()
        } else {
            format!("{cores} holes leave")
        };
        notes.push(format!(
            "{label}: {holes_text} a loose core (hole more than twice the tool); hold it down \
             or use a pocket"
        ));
    }
    if holes.is_empty() {
        return Err(job_error(format!(
            "{label}: no selected circle can be cut with tool {} ({} mm {})",
            tool.id,
            mm(tool.diameter),
            tool.kind.name()
        )));
    }
    let end_z = ctx
        .pass_z
        .last()
        .copied()
        .ok_or_else(|| job_error(format!("{label}: no depth to cut to")))?;
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
                    err.with_context(&format!("{label}, contour #{}", hole.contour_id))
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
    })
}

/// How wide a V-bit's groove is at the top, `depth` deep (docs/specs/follow-path.md §5).
fn groove_note(tool: &Tool, depth: f64) -> Option<String> {
    if tool.kind != ToolType::Vbit {
        return None;
    }
    let Some(angle) = tool.angle else {
        return Some(format!(
            "tool {} is a V-bit without an angle: the groove's width is unknown",
            tool.id
        ));
    };
    let tip = tool.tip_diameter.unwrap_or(0.0);
    let width = tip + 2.0 * depth * (angle / 2.0).tan();
    Some(if width > tool.diameter {
        format!(
            "the V-groove is deeper than the V-bit's cone: {} mm wide at the top, with \
             straight walls above the cone",
            mm(tool.diameter)
        )
    } else {
        format!("the V-groove is {} mm wide at the top", mm(width))
    })
}

fn plan_vcarve_op(
    op: &VcarveOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let label = ctx.label;
    let tool = ctx.tool;
    // Validated in validate_job: a V-bit with an angle.
    let angle = tool
        .angle
        .ok_or_else(|| job_error(format!("{label}: tool {} has no angle", tool.id)))?;
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
    .map_err(|err| err.with_context(label))?;
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
        notes: plan.notes.iter().map(|n| format!("{label}: {n}")).collect(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: ArcTable::default(),
        end,
        starts: Vec::new(),
    })
}

fn plan_profile_op(
    op: &ProfileOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    starts: &[Point],
) -> Result<Planned, ToolpathError> {
    let label = ctx.label;
    let machine = &ctx.job.machine;
    let radius = ctx.tool.diameter / 2.0;
    // The rings at the wall; with a finishing pass the roughing rings run `stock` farther
    // out (docs/specs/finish-and-skin.md §2).
    let mut plan = plan_profile(
        op,
        label,
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
            label,
            &selection.contours,
            radius,
            machine.spindle,
            start,
            starts,
        )?),
        None => None,
    };
    let mut rough = rough;
    // Dogbones go into every ring before the tabs are placed (docs/specs/dogbones.md §4).
    if let Some(style) = op.corners {
        let cut = CornerCut {
            style,
            tool_radius: radius,
            clearance: op.clearance,
            everything: ctx.everything,
            exempt: &[],
            label,
        };
        let unreached = profile_legs(&mut plan.rings, &selection.contours, op.side, &cut)?;
        plan.notes
            .extend(unreached.iter().map(|c| unreached_note(label, c, style)));
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
    };
    if let Some(rough) = &rough {
        out.arcs.extend(rough.arcs.clone());
        // The roughing may add a narrow part the wall doesn't have; its notes on start
        // points say the same as the wall's.
        for note in &rough.notes {
            if !out.notes.contains(note) {
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
            err.with_context(&format!("{label}, contour #{}", ring.contour_id))
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
                err.with_context(&format!("{label}, contour #{}", ring.contour_id))
            };
            let slot: Vec<&[Point]> = rough_rings
                .iter()
                .filter(|r| r.contour_id == ring.contour_id)
                .map(|r| r.points.as_slice())
                .collect();
            if let Some(finish) = op.finish
                && z == rough_bottom
            {
                slot_note(ring, &slot, radius, finish.stock, label, &mut out.notes)?;
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
        out.notes.push(format!(
            "{label}: some tabs on contours {} sit on a corner or curve (no straight stretch \
             nearby)",
            ids.join(", ")
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
    let label = ctx.label;
    let Some(tabs) = &op.tabs else {
        return Ok((vec![None; rings.len()], Vec::new()));
    };
    if op.side == Side::Inside {
        out.notes.push(format!(
            "{label}: tabs on an inside profile will hold the cut-out slug"
        ));
    }
    let mut layouts: Vec<Option<TabLayout>> = Vec::with_capacity(rings.len());
    let mut curved: Vec<usize> = Vec::new();
    for ring in rings {
        // Validated: count >= 1.
        let count = usize::try_from(tabs.count).unwrap_or(0);
        let placed = place_tabs(&ring.points, count, tabs.width, ctx.tool.diameter)
            .map_err(|err| err.with_context(&format!("{label}, contour #{}", ring.contour_id)))?;
        if placed.on_curves > 0 {
            curved.push(ring.contour_id);
        }
        out.tabs.extend(tab_polylines(&ring.points, &placed));
        if let Some(note) = joined_feet(&placed, tabs) {
            out.notes
                .push(format!("{label}, contour #{}: {note}", ring.contour_id));
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
    label: &str,
    notes: &mut Vec<String>,
) -> Result<(), ToolpathError> {
    let farthest = farthest_from(&ring.points, slot, radius / 2.0);
    let id = ring.contour_id;
    if farthest >= radius {
        return Err(ToolpathError::Geometry(format!(
            "{label}, contour #{id}: the roughing can't reach part of the wall (the finishing \
             lap runs {} mm from the roughing path, a tool radius or more): use less \
             finishing stock or a smaller tool",
            mm(farthest)
        )));
    }
    if farthest > 2.0 * stock + CHAIN_TOL_MM {
        notes.push(format!(
            "{label}, contour #{id}: the finishing lap cuts up to {} mm in corners or narrow \
             parts (finishing stock {} mm)",
            mm(farthest),
            mm(stock)
        ));
    }
    Ok(())
}

/// A note when neighbouring sloped tabs meet below their top: their feet, each
/// `height / tan(flank)` long, together are longer than the gap between them
/// (docs/specs/profile-spiral.md §4).
fn joined_feet(layout: &TabLayout, tabs: &Tabs) -> Option<String> {
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
            pairs.push(format!("{} and {}", k + 1, next + 1));
        }
    }
    (!pairs.is_empty()).then(|| {
        format!(
            "tabs {} meet below their top (flanks at {:.1}°): they hold as one longer tab",
            pairs.join(", "),
            tabs.flank_angle.to_degrees()
        )
    })
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
        let mine: Vec<OpSpan> = spans
            .iter()
            .filter(|s| {
                job.operations
                    .get(s.operation)
                    .is_some_and(|op| op.part() == part.id)
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
            return Err(ToolpathError::Overlap(format!(
                "Part \"{}\" copy {number} overlaps part \"{}\" copy {other} (X{x0:.3} to \
                 {x1:.3}, Y{y0:.3} to {y1:.3}): move the copies apart or change the grid",
                job.parts[*p].name, job.parts[*q].name
            )));
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
            let text = if same_row {
                format!(
                    "Copies overlap: with columns the pitch dx must be at least {width:.3} mm \
                     (the cut is {width:.3} mm wide), got {:.3}",
                    grid.dx.abs()
                )
            } else if same_column {
                format!(
                    "Copies overlap: with rows the pitch dy must be at least {height:.3} mm \
                     (the cut is {height:.3} mm high), got {:.3}",
                    grid.dy.abs()
                )
            } else {
                format!(
                    "Copies overlap: copies {other} and {number} cut into each other; move \
                     them apart (pitch dx {:.3}, dy {:.3})",
                    grid.dx.abs(),
                    grid.dy.abs()
                )
            };
            return Err(ToolpathError::Overlap(text));
        }
    }
    Ok(())
}

fn job_error(message: String) -> ToolpathError {
    ToolpathError::Job(message)
}

/// The tool an operation names (docs/specs/tool-per-operation.md §4).
pub fn op_tool<'a>(job: &'a Job, op: &Operation) -> Result<&'a Tool, ToolpathError> {
    job.tool(op.tool())
        .ok_or_else(|| job_error(format!("Tool {} is not defined in the job", op.tool())))
}

/// Generation-time rules from the specs, each operation against its own tool. Returns
/// warnings.
pub fn validate_job(job: &Job) -> Result<Vec<Note>, ToolpathError> {
    let (problems, notes) = check_job(job);
    match problems.into_iter().next() {
        Some(first) => Err(match first.operation() {
            Some(index) => job_error(first.text).at(index),
            None => job_error(first.text),
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
fn labelled(path: String, label: &str, short: String) -> Problem {
    let text = format!("{label}: {short}");
    Problem::new(path, short, text)
}

/// The problems, in the order the checks were always made (so the first is the error
/// `validate_job` always gave), and the notes of a job without problems.
fn check_job(job: &Job) -> (Vec<Problem>, Vec<Note>) {
    let mut problems = Vec::new();
    if job.operations.is_empty() {
        problems.push(Problem::whole("operations", "Job has no operations"));
        return (problems, Vec::new());
    }
    if !(0..job.operations.len()).any(|i| job.cuts(i)) {
        problems.push(Problem::whole(
            "operations",
            "Every operation is disabled: nothing to cut",
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
                    format!("Tool {} is not defined in the job", op.tool()),
                ));
            }
            Some(k) => {
                let tool = &job.tools[k];
                if tool.rpm.is_none_or(|rpm| rpm <= 0.0) && !no_rpm.contains(&tool.id) {
                    no_rpm.push(tool.id);
                    let text = format!("Tool {} has no spindle speed (rpm)", tool.id);
                    problems.push(Problem::new(
                        format!("tools.{k}.rpm"),
                        "no spindle speed",
                        text,
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
            format!("must be above the stock top (Z {top})"),
            format!(
                "safe_z {} must be above the stock top (Z {top}); z_zero is '{z_zero}'",
                job.machine.safe_z
            ),
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
        let label = op.label(index);
        let numbered = format!("Operation {number}");
        let at = |field: &str| format!("operations.{index}.{field}");
        let (feed, plunge_feed) = match op {
            Operation::Profile(p) => (p.feed, p.plunge_feed),
            Operation::Pocket(p) => (p.feed, p.plunge_feed),
            Operation::Vcarve(v) => (v.feed, v.plunge_feed),
            Operation::Path(p) => (p.feed, p.plunge_feed),
            Operation::Holes(h) => (h.feed, h.plunge_feed),
        };
        // Below the stock: a warning within the overcut allowance, an error beyond it
        // (docs/specs/depths-and-clearance.md section 2).
        let (_, end_depth) = op.depths();
        let thickness = job.stock.thickness;
        let below = end_depth - thickness;
        if below > job.stock.max_overcut + DEPTH_TOL_MM {
            problems.push(labelled(
                at("end_depth"),
                &label,
                format!(
                    "end depth {end_depth} is {below:.3} mm below the stock \
                     ({thickness} mm thick), more than the allowed {} mm (max depth below \
                     stock)",
                    job.stock.max_overcut
                ),
            ));
        } else if below > DEPTH_TOL_MM {
            notes.push(Note::at(
                index,
                format!("{label}: cuts {below:.3} mm below the stock (into the spoilboard)"),
            ));
        }
        if op.clearance() >= tool.diameter / 2.0 {
            problems.push(labelled(
                at("clearance"),
                &label,
                format!(
                    "clearance {} must be less than the tool radius {}",
                    op.clearance(),
                    tool.diameter / 2.0
                ),
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
                &label,
                format!(
                    "finishing stock {} must be less than the tool radius {}",
                    finish.stock,
                    tool.diameter / 2.0
                ),
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
                &label,
                format!(
                    "tab height {} must be less than the cut depth {cut} (end depth minus \
                     start depth)",
                    tabs.height
                ),
            ));
        }
        if let Operation::Pocket(PocketOp {
            rest: Some(rest), ..
        }) = op
        {
            match rest_check(job, index, rest.after_tool, tool, &label) {
                Ok(more) => notes.extend(more.into_iter().map(|n| Note::at(index, n))),
                Err(text) => problems.push(labelled(at("rest"), &label, text)),
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
                        format!(
                            "{name} {value} exceeds machine max_feed \
                             {max_feed}"
                        ),
                    ));
                }
            }
        }
        if let Entry::Helix { radius, .. } = op.entry() {
            if let Operation::Path(_) = op {
                problems.push(labelled(
                    at("entry"),
                    &label,
                    "helix entry is for pockets; a follow path can ramp in (entry: ramp)"
                        .to_string(),
                ));
            } else if radius >= tool.diameter / 2.0 {
                problems.push(labelled(
                    at("entry"),
                    &label,
                    format!(
                        "helix radius {radius} must be less than the tool radius {}, \
                         or it leaves a core standing in the middle",
                        tool.diameter / 2.0
                    ),
                ));
            }
        }
        if let Some(lead) = op.lead() {
            let longest = MAX_LEAD_DIAMETERS * tool.diameter;
            if lead.radius > longest {
                problems.push(labelled(
                    at("lead"),
                    &label,
                    format!(
                        "lead radius {} is more than {MAX_LEAD_DIAMETERS} × the tool \
                         diameter ({longest:.3} mm)",
                        lead.radius
                    ),
                ));
            }
        }
        // The tool's rules: a problem at its field, or whether the operation plunges
        // straight down with a tool of unknown centre-cutting, and any other notes.
        let (plunges, more): (ToolRule, Vec<String>) = match op {
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
            Operation::Vcarve(_) => match vcarve_tool_notes(&label, tool) {
                Ok(found) => (Ok(false), found),
                Err(e) => {
                    let path = if tool.kind == ToolType::Vbit {
                        format!("tools.{k}.angle")
                    } else {
                        at("tool")
                    };
                    (Err((path, label.clone(), e)), Vec::new())
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
        let listed: Vec<String> = ops
            .iter()
            .map(|(index, what)| format!("{} ({what})", index + 1))
            .collect();
        let which = match listed.len() {
            1 => format!("operation {} plunges", listed[0]),
            n => format!(
                "operations {} and {} plunge",
                listed[..n - 1].join(", "),
                listed[n - 1]
            ),
        };
        notes.push(Note::at(
            first,
            format!(
                "Tool {tool} isn't marked center_cutting and {which} straight down: make sure \
                 it is a centre-cutting tool, or ramp in (entry: ramp)"
            ),
        ));
    }
    (problems, notes)
}

/// A tool rule's outcome: whether the operation plunges with a tool of unknown
/// centre-cutting, or the problem's (path, label, short text).
type ToolRule = Result<bool, (String, String, String)>;

/// The kind of an operation in a note: "pocket", "profile", "follow path".
fn op_kind_name(op: &Operation) -> &'static str {
    match op {
        Operation::Profile(_) => "profile",
        Operation::Pocket(_) => "pocket",
        Operation::Vcarve(_) => "V-carve",
        Operation::Path(_) => "follow path",
        Operation::Holes(_) => "holes",
    }
}

/// Rest machining's rules (docs/specs/rest-machining.md §5) for the pocket at `index`,
/// which cuts with `tool`: a problem's text, or the warnings.
fn rest_check(
    job: &Job,
    index: usize,
    after_tool: i64,
    tool: &Tool,
    label: &str,
) -> Result<Vec<String>, String> {
    // The warnings carry the label; a problem's text gets it from `labelled`.
    let Some(Operation::Pocket(pocket)) = job.operations.get(index) else {
        return Ok(Vec::new());
    };
    let Some(large) = job.tool(after_tool) else {
        return Err(format!(
            "rest after tool {after_tool}, which the project doesn't have"
        ));
    };
    match rest_refusal(tool, large) {
        Some(RestRefusal::LargeNotEndmill) => {
            return Err(format!(
                "rest after tool {after_tool}, a {}; a pocket's tool is an endmill",
                large.kind.name()
            ));
        }
        Some(RestRefusal::NotLarger) => {
            return Err(format!(
                "rest after tool {after_tool} ({} mm) needs a tool larger than this \
                 pocket's ({} mm)",
                mm(large.diameter),
                mm(tool.diameter)
            ));
        }
        // The pocket's own tool is checked with the pocket's tool rules.
        Some(RestRefusal::SmallNotEndmill) | None => {}
    }
    if pocket.corners.is_some() {
        return Err(
            "dogbones and rest machining can't be combined yet; dogbone the large tool's \
             pocket"
                .to_string(),
        );
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
        Ok(vec![format!(
            "{label}: rests after tool {after_tool}, but no earlier pocket cuts these shapes \
             with it as deep; the rest of the pocket will be left uncut"
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
    if pocket.corners.is_some() {
        problems.push(labelled(
            format!("operations.{index}.rest_passes"),
            &job.label(OpRef::op(index)),
            "dogbones and rest passes can't be combined yet; dogbone corners need no rest".into(),
        ));
    }
    for (k, pass) in pocket.rest_passes.iter().enumerate() {
        let label = job.label(OpRef::pass(index, k));
        let at = |field: &str| format!("operations.{index}.rest_passes.{k}.{field}");
        let Some(tool) = job.tool(pass.tool) else {
            problems.push(labelled(
                at("tool"),
                &label,
                format!("tool {} is not defined in the job", pass.tool),
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
                    &label,
                    format!(
                        "tool {} is a {}; a rest pass cuts with an endmill",
                        tool.id,
                        tool.kind.name()
                    ),
                )),
                Some(RestRefusal::NotLarger) => problems.push(labelled(
                    at("tool"),
                    &label,
                    format!(
                        "tool {} ({} mm) must be smaller than tool {} ({} mm) before it",
                        tool.id,
                        mm(tool.diameter),
                        before.id,
                        mm(before.diameter)
                    ),
                )),
                // The tool before it is the pocket's (checked with the pocket) or a pass's
                // (checked above).
                Some(RestRefusal::LargeNotEndmill) | None => {}
            }
        }
        if tool.rpm.is_none_or(|rpm| rpm <= 0.0) {
            problems.push(labelled(
                at("tool"),
                &label,
                format!("tool {} has no spindle speed (rpm)", tool.id),
            ));
        }
        if let Ok(true) = pocket_tool_notes(tool, pass.entry) {
            notes.push(Note::at(
                index,
                format!(
                    "{label}: plunges straight down with tool {}, which isn't marked \
                     centre-cutting; use a helix or a ramp if it isn't",
                    tool.id
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
        notes.push(Note::job(format!(
            "Tool {tool} is used in {} groups (operations {}); moving them next to each other \
             saves a tool change",
            used.len(),
            numbers.join(", ")
        )));
    }
    notes
}

/// V-carving needs a V-bit with a known angle; a tip that isn't set is taken as pointed and
/// said so (docs/specs/v-carve.md §2, decision 11). The notes carry the label; a problem's
/// text gets it from `labelled`, as for the other tool rules below.
fn vcarve_tool_notes(label: &str, tool: &Tool) -> Result<Vec<String>, String> {
    if tool.kind != ToolType::Vbit {
        return Err(format!(
            "V-carving needs a V-bit, tool {} is a {}",
            tool.id,
            tool.kind.name()
        ));
    }
    if tool.angle.is_none() {
        return Err(format!(
            "tool {} is a V-bit with no angle; set its angle",
            tool.id
        ));
    }
    if tool.tip_diameter.is_none() {
        return Ok(vec![format!(
            "{label}: tool {} has no tip diameter set; V-carving takes it as pointed",
            tool.id
        )]);
    }
    Ok(Vec::new())
}

/// Pockets need an endmill; a straight plunge also needs it to be centre-cutting. Ok(true):
/// it plunges with a tool of unknown centre-cutting.
fn pocket_tool_notes(tool: &Tool, entry: Entry) -> Result<bool, String> {
    if tool.kind != ToolType::Endmill {
        return Err(format!(
            "pockets need an endmill, tool {} is a {}",
            tool.id,
            tool.kind.name()
        ));
    }
    plunge_unknown(tool, entry)
}

/// A follow path takes an endmill (which must be centre-cutting to plunge) or a V-bit
/// (centre-cutting by its shape); a drill can't follow a path (docs/specs/follow-path.md §5).
fn path_tool_notes(tool: &Tool, entry: Entry) -> Result<bool, String> {
    match tool.kind {
        ToolType::Drill => Err(format!(
            "tool {} is a drill, which can't follow a path; use an endmill or a V-bit",
            tool.id
        )),
        ToolType::Vbit => Ok(false),
        ToolType::Endmill => plunge_unknown(tool, entry),
    }
}

/// A drill drills and an endmill bores (docs/specs/holes.md §1, §2): the method must match
/// the tool; a V-bit can't cut holes.
fn holes_tool_notes(tool: &Tool, method: HoleMethod) -> Result<(), String> {
    if tool.kind == ToolType::Vbit {
        return Err(format!(
            "tool {} is a V-bit, which can't cut holes; use a drill or an endmill",
            tool.id
        ));
    }
    let wanted = method.tool_type();
    if tool.kind != wanted {
        let (method_name, fits) = match method {
            HoleMethod::Drill { .. } => ("drill", "a drill"),
            HoleMethod::Bore { .. } => ("bore", "an endmill"),
        };
        let is = tool.kind.with_article();
        return Err(format!(
            "the method is {method_name}, which needs {fits}; tool {} is {is}",
            tool.id
        ));
    }
    Ok(())
}

/// Going down a tab's flank is a ramp (docs/specs/profile-spiral.md §4): flanks steeper
/// than the ramp angle are refused for a tool marked not centre-cutting, and a note when
/// it's unknown.
fn flank_notes(number: usize, tool: &Tool, op: &ProfileOp) -> Result<Vec<String>, String> {
    let Some(tabs) = &op.tabs else {
        return Ok(Vec::new());
    };
    let ramp_angle = op.ramp_angle;
    if tabs.flank_angle <= ramp_angle + ANGLE_TOL_RAD {
        return Ok(Vec::new());
    }
    let (flank, ramp) = (tabs.flank_angle.to_degrees(), ramp_angle.to_degrees());
    match tool.center_cutting {
        Some(false) => Err(format!(
            "tool {} is not centre-cutting and the tabs' flanks ({flank:.1}°) are steeper \
             than the ramp angle ({ramp:.1}°); going down a flank is a ramp, so make them \
             no steeper than {ramp:.1}°",
            tool.id
        )),
        None => Ok(vec![format!(
            "Operation {number}: the tabs' flanks ({flank:.1}°) are steeper than the ramp \
             angle ({ramp:.1}°); tool {} isn't marked center_cutting, make sure it is a \
             centre-cutting tool",
            tool.id
        )]),
        Some(true) => Ok(Vec::new()),
    }
}

/// A straight plunge needs a centre-cutting tool (docs/specs/ramp-entry.md §6): refused
/// for a tool marked not centre-cutting; Ok(true) when it's unknown (one note per tool).
/// A ramp needs neither.
fn plunge_unknown(tool: &Tool, entry: Entry) -> Result<bool, String> {
    if entry.ramp_angle().is_some() {
        return Ok(false);
    }
    match tool.center_cutting {
        Some(false) => Err(format!(
            "tool {} is not centre-cutting and this operation plunges straight down; ramp in \
             instead (entry: ramp) or use a centre-cutting tool",
            tool.id
        )),
        None => Ok(true),
        Some(true) => Ok(false),
    }
}
