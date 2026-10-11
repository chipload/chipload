// SPDX-License-Identifier: GPL-3.0-or-later
//! Job + imported contours -> Program: the toolpath half of `chipload_pipeline::run_job`,
//! which then posts and independently checks it.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::Primitive;
use chipload_job::{
    Copies, CopyOrder, Entry, Job, MAX_LEAD_DIAMETERS, Operation, PocketOp, ProfileOp, Side, Tool,
    ToolType, VcarveOp, ZZero,
};

use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};

use crate::ToolpathError;
use crate::arcs::{ArcTable, tag_moves};
use crate::geom::PrimIndex;
use crate::lead::{Leads, MIN_LEAD_DIAMETERS, fitted_leads};
use crate::left::{LeftPiece, MaterialLeft, bounds_of};
use crate::model::{Move, MoveKind, Program};
use crate::pocket::{Choices, PocketPiece, plan_pocket_starting, pocket_moves};
use crate::profile::{
    TabsAt, contour_moves, cut_clockwise, led_contour_moves, pass_depths, plan_profile,
};
use crate::select::{Selection, select_contours};
use crate::tabs::{place_tabs, tab_polylines, tabbed_contour_moves};
use crate::vcarve::{VBit, carve_levels, plan_vcarve, vcarve_moves};

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub program: Program,
    /// Warnings for the user.
    pub notes: Vec<String>,
    /// Tool-path stretch of every tab, for display.
    pub tabs: Vec<Vec<Point>>,
    /// Every pocket piece in cut order (rings and links), for display and conformance tests.
    pub pockets: Vec<PocketPiece>,
    /// Material each operation leaves (on the part, copy 1), for display; by operation,
    /// kind, then (y, x).
    pub material_left: Vec<MaterialLeft>,
    /// Which moves of `program` each operation made, in cut order, for display.
    pub op_spans: Vec<OpSpan>,
    /// Every ring whose start the user can move (on the part, copy 1), for the start
    /// markers (docs/specs/helix-and-leads.md part A).
    pub ring_starts: Vec<RingStart>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpSpan {
    /// Index in `job.operations`.
    pub operation: usize,
    /// Which copy of the part (docs/specs/copies-grid.md): 1 is the drawing as drawn.
    pub copy: u32,
    pub moves: std::ops::Range<usize>,
}

pub fn plan_job(job: &Job, contours: &[Contour]) -> Result<Plan, ToolpathError> {
    let (tool, mut notes) = validate_job(job)?;
    let rpm = tool
        .rpm
        .ok_or_else(|| job_error(format!("Tool {} has no spindle speed (rpm)", tool.id)))?;
    let top_z = match job.stock.z_zero {
        ZZero::Top => 0.0,
        ZZero::Bottom => job.stock.thickness,
    };
    let mut moves: Vec<Move> = Vec::new();
    let mut tabs: Vec<Vec<Point>> = Vec::new();
    let mut pockets: Vec<PocketPiece> = Vec::new();
    let mut material_left: Vec<MaterialLeft> = Vec::new();
    let mut op_spans: Vec<OpSpan> = Vec::new();
    let mut ring_starts: Vec<RingStart> = Vec::new();
    let mut arcs = ArcTable::default();
    let mut position: Point = (0.0, 0.0);
    // Every enabled operation's selection, chosen once on the drawing as drawn.
    let mut chosen: Vec<(usize, String, Selection)> = Vec::new();
    for (index, op) in job.operations.iter().enumerate() {
        if !op.enabled() {
            continue; // kept in the file, skipped here; numbering stays as listed
        }
        let label = match op.name() {
            "" => format!("Operation {}", index + 1),
            name => format!("Operation {} ({name})", index + 1),
        };
        let selection = select_contours(contours, op.select(), &label)?;
        chosen.push((index, label, selection));
    }
    let copies = job
        .copies
        .map_or_else(|| vec![(1, (0.0, 0.0))], |c| c.in_cut_order());
    let several = copies.len() > 1;
    // (operation, copy) pairs in cut order (docs/specs/copies-grid.md §3).
    let mut order: Vec<(usize, usize)> = Vec::new();
    match job.copies.map(|c| c.order) {
        Some(CopyOrder::ByPart) => {
            for k in 0..copies.len() {
                order.extend((0..chosen.len()).map(|i| (i, k)));
            }
        }
        _ => {
            for i in 0..chosen.len() {
                order.extend((0..copies.len()).map(|k| (i, k)));
            }
        }
    }
    // Every copy repeats the original's notes: each is listed once per operation.
    let mut seen: Vec<(usize, String)> = Vec::new();
    let mut started: Vec<usize> = Vec::new();
    for (i, k) in order {
        let (index, label, selection) = &chosen[i];
        if !started.contains(index) {
            started.push(*index);
            notes.extend(selection.notes.iter().cloned());
        }
        let (number, offset) = copies[k];
        let op = &job.operations[*index];
        let (start_depth, end_depth) = op.depths();
        // A V-carve sets its own passes from how deep it goes (docs/specs/v-carve.md §3).
        let pass_z: Vec<f64> = match op {
            Operation::Profile(ProfileOp { step_down, .. })
            | Operation::Pocket(PocketOp { step_down, .. }) => {
                pass_depths(start_depth, end_depth, *step_down)?
                    .into_iter()
                    .map(|d| top_z - d)
                    .collect()
            }
            Operation::Vcarve(_) => Vec::new(),
        };
        // Leads keep clear of every contour of the drawing (moved with the copy).
        let everything: Vec<Contour> = if op.lead().is_some() {
            contours.iter().map(|c| c.translated(offset)).collect()
        } else {
            Vec::new()
        };
        let context = Context {
            label,
            tool,
            job,
            pass_z: &pass_z,
            top_z: top_z - start_depth,
            everything: &everything,
        };
        // A copy is planned from its own, moved contours: built and verified like the
        // original.
        let moved: Vec<Contour> = selection
            .contours
            .iter()
            .map(|c| c.translated(offset))
            .collect();
        let shifted = Selection {
            contours: moved.iter().collect(),
            notes: Vec::new(),
        };
        let starts: Vec<Point> = op
            .starts()
            .iter()
            .map(|p| (p[0] + offset.0, p[1] + offset.1))
            .collect();
        let planned = match op {
            Operation::Pocket(pocket) => {
                plan_pocket_op(pocket, &context, &shifted, position, &starts)
            }
            Operation::Profile(profile) => {
                plan_profile_op(profile, &context, &shifted, position, &starts)
            }
            Operation::Vcarve(vcarve) => plan_vcarve_op(vcarve, &context, &shifted, position),
        };
        let planned = match planned {
            Ok(planned) => planned,
            Err(err) if several => return Err(err.with_context(&format!("copy {number}"))),
            Err(err) => return Err(err),
        };
        let first = moves.len();
        moves.extend(planned.moves);
        op_spans.push(OpSpan {
            operation: *index,
            copy: number,
            moves: first..moves.len(),
        });
        for note in planned.notes {
            let key = (*index, note);
            if !seen.contains(&key) {
                notes.push(key.1.clone());
                seen.push(key);
            }
        }
        tabs.extend(planned.tabs);
        pockets.extend(planned.pockets);
        // Material left describes the part: the copies leave the same.
        if number == 1 {
            material_left.extend(attach(*index, planned.left));
            ring_starts.extend(planned.starts.into_iter().map(|(ring, stored)| RingStart {
                operation: *index,
                ring,
                stored,
            }));
        }
        arcs.extend(planned.arcs);
        position = planned.end;
    }
    if let Some(grid) = &job.copies {
        check_copies_apart(grid, &moves, &op_spans, tool.diameter / 2.0)?;
    }
    // Chords of known arcs are tagged; the post writes them as G2/G3 if it does arcs.
    tag_moves(&mut moves, &arcs);
    Ok(Plan {
        program: Program {
            tool_id: tool.id,
            tool_diameter: tool.diameter,
            rpm,
            spindle: job.machine.spindle,
            safe_z: job.machine.safe_z,
            moves,
        },
        notes,
        tabs,
        pockets,
        material_left,
        op_spans,
        ring_starts,
    })
}

/// One operation's pieces, ordered by kind, then (min y, min x).
fn attach(operation: usize, mut pieces: Vec<LeftPiece>) -> Vec<MaterialLeft> {
    pieces.sort_by(|a, b| {
        let ((ax, ay), _) = bounds_of(&a.rings);
        let ((bx, by), _) = bounds_of(&b.rings);
        a.kind
            .cmp(&b.kind)
            .then(ay.total_cmp(&by))
            .then(ax.total_cmp(&bx))
    });
    pieces
        .into_iter()
        .map(|piece| MaterialLeft {
            operation,
            kind: piece.kind,
            rings: piece.rings,
        })
        .collect()
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
}

/// One operation's moves, warnings and tabs, and where the tool ends up (above that point).
struct Planned {
    moves: Vec<Move>,
    notes: Vec<String>,
    tabs: Vec<Vec<Point>>,
    pockets: Vec<PocketPiece>,
    left: Vec<LeftPiece>,
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
    let plan = plan_pocket_starting(
        &selection.contours,
        ctx.tool.diameter / 2.0,
        op.clearance,
        op.stepover * ctx.tool.diameter,
        op.direction,
        ctx.job.machine.spindle,
        op.keep_tool_down,
        start,
        op.strategy,
        Choices {
            starts,
            lead: op.lead.map(|l| l.radius),
            helix: match op.entry {
                Entry::Helix { radius, .. } => Some(radius),
                _ => None,
            },
        },
    )
    .map_err(|err| err.with_context(label))?;
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan.notes.iter().map(|n| format!("{label}: {n}")).collect(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        left: plan.left,
        arcs: plan.arcs,
        end: start,
        starts: Vec::new(),
    };
    for piece in &plan.pieces {
        for (k, ring) in piece.rings.iter().enumerate() {
            if piece.walls.get(k).copied().unwrap_or(false) {
                // Without the leads, if it has them.
                let plain = match piece.leads.get(k).cloned().flatten() {
                    Some(range) => ring.get(range).map(<[Point]>::to_vec).unwrap_or_default(),
                    None => ring.clone(),
                };
                out.starts
                    .push((plain, piece.chosen.get(k).copied().flatten()));
            }
        }
        out.moves.extend(
            pocket_moves(
                piece,
                ctx.pass_z,
                ctx.top_z,
                ctx.job.machine.safe_z,
                op.feed,
                op.plunge_feed,
                op.entry,
            )
            .map_err(|err| err.with_context(label))?,
        );
        // Retracted above the start of the wall ring.
        let last = piece.rings.len().checked_sub(1);
        if let Some(end) = last.and_then(|k| {
            crate::pocket::path_end(&piece.rings[k], piece.open.get(k).copied().unwrap_or(false))
        }) {
            out.end = end;
        }
    }
    out.pockets = plan.pieces;
    Ok(out)
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
        left: Vec::new(),
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
    let plan = plan_profile(
        op,
        label,
        &selection.contours,
        ctx.tool.diameter / 2.0,
        machine.spindle,
        start,
        starts,
    )?;
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan.notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        left: plan.left,
        arcs: plan.arcs,
        end: start,
        starts: plan
            .rings
            .iter()
            .zip(&plan.chosen)
            .map(|(ring, &stored)| (ring.points.clone(), stored))
            .collect(),
    };
    if let Some(tabs) = &op.tabs {
        let cut = op.end_depth - op.start_depth;
        if tabs.height >= cut {
            return Err(job_error(format!(
                "{label}: tab height {} must be less than the cut depth {cut} (end depth \
                 minus start depth)",
                tabs.height
            )));
        }
        if op.side == Side::Inside {
            out.notes.push(format!(
                "{label}: tabs on an inside profile will hold the cut-out slug"
            ));
        }
    }
    let mut curved: Vec<usize> = Vec::new();
    for ring in &plan.rings {
        let points = &ring.points;
        let leads = match op.lead {
            Some(lead) => profile_leads(op, ctx, ring, lead.radius, &mut out.notes),
            None => None,
        };
        if let Some(leads) = &leads {
            leads.tag(&mut out.arcs);
            let context = |err: ToolpathError| {
                err.with_context(&format!("{label}, contour #{}", ring.contour_id))
            };
            let mut layout = None;
            if let Some(tabs) = &op.tabs {
                // Validated: count >= 1.
                let count = usize::try_from(tabs.count).unwrap_or(0);
                let placed =
                    place_tabs(points, count, tabs.width, ctx.tool.diameter).map_err(context)?;
                if placed.on_curves > 0 {
                    curved.push(ring.contour_id);
                }
                out.tabs.extend(tab_polylines(points, &placed));
                let bottom = ctx.pass_z.last().copied().unwrap_or(0.0);
                layout = Some((placed, bottom + tabs.height));
            }
            out.moves.extend(
                led_contour_moves(
                    points,
                    leads,
                    layout.as_ref().map(|(layout, z_tab)| TabsAt {
                        layout,
                        z_tab: *z_tab,
                    }),
                    ctx.pass_z,
                    ctx.top_z,
                    machine.safe_z,
                    op.feed,
                    op.plunge_feed,
                    op.entry,
                )
                .map_err(context)?,
            );
            out.end = leads.lead_out.last().copied().unwrap_or(points[0]);
            continue;
        }
        match &op.tabs {
            None => out.moves.extend(
                contour_moves(
                    points,
                    ctx.pass_z,
                    ctx.top_z,
                    machine.safe_z,
                    op.feed,
                    op.plunge_feed,
                    op.entry,
                )
                .map_err(|err| {
                    err.with_context(&format!("{label}, contour #{}", ring.contour_id))
                })?,
            ),
            Some(tabs) => {
                // Validated: count >= 1.
                let count = usize::try_from(tabs.count).unwrap_or(0);
                let context = |err: ToolpathError| {
                    err.with_context(&format!("{label}, contour #{}", ring.contour_id))
                };
                let layout =
                    place_tabs(points, count, tabs.width, ctx.tool.diameter).map_err(context)?;
                if layout.on_curves > 0 {
                    curved.push(ring.contour_id);
                }
                let bottom = ctx.pass_z.last().copied().unwrap_or(0.0);
                out.moves.extend(
                    tabbed_contour_moves(
                        points,
                        &layout,
                        ctx.pass_z,
                        ctx.top_z,
                        bottom + tabs.height,
                        machine.safe_z,
                        op.feed,
                        op.plunge_feed,
                        op.entry,
                    )
                    .map_err(context)?,
                );
                out.tabs.extend(tab_polylines(points, &layout));
            }
        }
        out.end = points[0];
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

/// Leads for one profile ring that keep the tool centre at least the ring's offset from its
/// own contour and the tool radius from every other contour; the radius halves until they
/// fit, down to a tenth of the tool diameter (docs/specs/helix-and-leads.md part B). None
/// with a note if even that doesn't fit.
fn profile_leads(
    op: &ProfileOp,
    ctx: &Context,
    ring: &crate::profile::PathRing,
    radius: f64,
    notes: &mut Vec<String>,
) -> Option<Leads> {
    let tool_radius = ctx.tool.diameter / 2.0;
    let offset = tool_radius - op.clearance;
    let clockwise = cut_clockwise(op.side, op.direction, ctx.job.machine.spindle);
    // Climb with M3 keeps the part on the right: the waste is on the left.
    let waste_left = (op.side == Side::Outside) == clockwise;
    let prims = |own: bool| -> Vec<Primitive> {
        ctx.everything
            .iter()
            .filter(|c| (c.id == ring.contour_id) == own)
            .flat_map(|c| c.parts.iter().map(|p| p.segment.prim))
            .collect()
    };
    let cell = tool_radius.max(1.0);
    let own = PrimIndex::new(prims(true), cell);
    let others = PrimIndex::new(prims(false), cell);
    let min_radius = MIN_LEAD_DIAMETERS * ctx.tool.diameter;
    let label = ctx.label;
    let id = ring.contour_id;
    match fitted_leads(&ring.points, radius, min_radius, waste_left, |l| {
        l.clear_of(&own, offset) && l.clear_of(&others, tool_radius)
    }) {
        Some((leads, used)) => {
            if used < radius {
                notes.push(format!(
                    "{label}, contour #{id}: lead radius reduced to {used:.3} mm (no room for \
                     {radius:.3} mm)"
                ));
            }
            Some(leads)
        }
        None => {
            notes.push(format!(
                "{label}, contour #{id}: no room for a lead (not even {min_radius:.3} mm); cut \
                 without one"
            ));
            None
        }
    }
}

/// The XY extent the tool cuts in one copy (or in all, with None): every feed move's end
/// points, plus the tool radius. None if it cuts nothing.
pub fn cut_extent(
    moves: &[Move],
    spans: &[OpSpan],
    radius: f64,
    copy: Option<u32>,
) -> Option<(Point, Point)> {
    let mut extent: Option<(Point, Point)> = None;
    let mut at: (Option<f64>, Option<f64>) = (None, None);
    for span in spans {
        for m in &moves[span.moves.clone()] {
            let from = at;
            at = (m.x.or(at.0), m.y.or(at.1));
            if copy.is_some_and(|c| c != span.copy) || m.kind != MoveKind::Feed {
                continue;
            }
            for (x, y) in [from, at] {
                let (Some(x), Some(y)) = (x, y) else { continue };
                let ((x0, y0), (x1, y1)) = extent.unwrap_or(((x, y), (x, y)));
                extent = Some(((x0.min(x), y0.min(y)), (x1.max(x), y1.max(y))));
            }
        }
    }
    extent.map(|((x0, y0), (x1, y1))| ((x0 - radius, y0 - radius), (x1 + radius, y1 + radius)))
}

/// Neighbouring copies must not cut into each other (docs/specs/copies-grid.md §4): with
/// more than one column the pitch dx is at least the cut's width, likewise dy and height.
fn check_copies_apart(
    grid: &Copies,
    moves: &[Move],
    spans: &[OpSpan],
    radius: f64,
) -> Result<(), ToolpathError> {
    let Some(((x0, y0), (x1, y1))) = cut_extent(moves, spans, radius, Some(1)) else {
        return Ok(());
    };
    let axes = [
        ("columns", "dx", grid.columns, grid.dx, x1 - x0, "wide"),
        ("rows", "dy", grid.rows, grid.dy, y1 - y0, "high"),
    ];
    for (what, name, count, pitch, size, dimension) in axes {
        if count > 1 && pitch.abs() + CHAIN_TOL_MM < size {
            return Err(job_error(format!(
                "Copies overlap: with {what} the pitch {name} must be at least {size:.3} mm \
                 (the cut is {size:.3} mm {dimension}), got {:.3}",
                pitch.abs()
            )));
        }
    }
    Ok(())
}

fn job_error(message: String) -> ToolpathError {
    ToolpathError::Job(message)
}

/// Generation-time rules from the specs. Returns the single tool used and warnings.
pub fn validate_job(job: &Job) -> Result<(&Tool, Vec<String>), ToolpathError> {
    if job.operations.is_empty() {
        return Err(job_error("Job has no operations".into()));
    }
    if !job.operations.iter().any(Operation::enabled) {
        return Err(job_error(
            "Every operation is disabled: nothing to cut".into(),
        ));
    }
    let enabled = || {
        job.operations
            .iter()
            .enumerate()
            .filter(|(_, op)| op.enabled())
    };
    let mut used: Vec<i64> = enabled().map(|(_, op)| op.tool()).collect();
    used.sort_unstable();
    used.dedup();
    if used.len() > 1 {
        let list: Vec<String> = used.iter().map(i64::to_string).collect();
        return Err(job_error(format!(
            "Operations use tools [{}]; this version supports one tool per job",
            list.join(", ")
        )));
    }
    let tool = job
        .tools
        .iter()
        .find(|t| t.id == used[0])
        .ok_or_else(|| job_error(format!("Tool {} is not defined in the job", used[0])))?;
    if tool.rpm.is_none_or(|rpm| rpm <= 0.0) {
        return Err(job_error(format!(
            "Tool {} has no spindle speed (rpm)",
            tool.id
        )));
    }
    let (top, z_zero) = match job.stock.z_zero {
        ZZero::Top => (0.0, "top"),
        ZZero::Bottom => (job.stock.thickness, "bottom"),
    };
    if job.machine.safe_z <= top {
        return Err(job_error(format!(
            "safe_z {} must be above the stock top (Z {top}); z_zero is '{z_zero}'",
            job.machine.safe_z
        )));
    }
    let mut notes = Vec::new();
    for (index, op) in enabled() {
        let number = index + 1;
        let label = match op.name() {
            "" => format!("Operation {number}"),
            name => format!("Operation {number} ({name})"),
        };
        let (feed, plunge_feed) = match op {
            Operation::Profile(p) => (p.feed, p.plunge_feed),
            Operation::Pocket(p) => (p.feed, p.plunge_feed),
            Operation::Vcarve(v) => (v.feed, v.plunge_feed),
        };
        // Below the stock: a warning within the overcut allowance, an error beyond it
        // (docs/specs/depths-and-clearance.md section 2).
        let (_, end_depth) = op.depths();
        let thickness = job.stock.thickness;
        let below = end_depth - thickness;
        if below > job.stock.max_overcut + DEPTH_TOL_MM {
            return Err(job_error(format!(
                "{label}: end depth {end_depth} is {below:.3} mm below the stock \
                 ({thickness} mm thick), more than the allowed {} mm (max depth below stock)",
                job.stock.max_overcut
            )));
        }
        if below > DEPTH_TOL_MM {
            notes.push(format!(
                "{label}: cuts {below:.3} mm below the stock (into the spoilboard)"
            ));
        }
        if op.clearance() >= tool.diameter / 2.0 {
            return Err(job_error(format!(
                "{label}: clearance {} must be less than the tool radius {}",
                op.clearance(),
                tool.diameter / 2.0
            )));
        }
        if let Some(max_feed) = job.machine.max_feed {
            for (name, value) in [("feed", feed), ("plunge_feed", plunge_feed)] {
                if value > max_feed {
                    return Err(job_error(format!(
                        "Operation {number}: {name} {value} exceeds machine max_feed {max_feed}"
                    )));
                }
            }
        }
        if let Entry::Helix { radius, .. } = op.entry() {
            if let Operation::Profile(_) = op {
                return Err(job_error(format!(
                    "{label}: helix entry is for pockets; a profile can ramp in (entry: ramp)"
                )));
            }
            if radius >= tool.diameter / 2.0 {
                return Err(job_error(format!(
                    "{label}: helix radius {radius} must be less than the tool radius {}, or \
                     it leaves a core standing in the middle",
                    tool.diameter / 2.0
                )));
            }
        }
        if let Some(lead) = op.lead() {
            let longest = MAX_LEAD_DIAMETERS * tool.diameter;
            if lead.radius > longest {
                return Err(job_error(format!(
                    "{label}: lead radius {} is more than {MAX_LEAD_DIAMETERS} × the tool \
                     diameter ({longest:.3} mm)",
                    lead.radius
                )));
            }
            if let Operation::Profile(p) = op
                && p.side == Side::On
            {
                return Err(job_error(format!(
                    "{label}: a profile cut on the line has no waste side for a lead; cut \
                     inside or outside, or remove the lead"
                )));
            }
        }
        match op {
            Operation::Pocket(p) => notes.extend(pocket_tool_notes(number, tool, p.entry)?),
            Operation::Profile(p) => notes.extend(plunge_notes(number, "profile", tool, p.entry)?),
            Operation::Vcarve(_) => notes.extend(vcarve_tool_notes(&label, tool)?),
        }
    }
    Ok((tool, notes))
}

/// V-carving needs a V-bit with a known angle; a tip that isn't set is taken as pointed and
/// said so (docs/specs/v-carve.md §2, decision 11).
fn vcarve_tool_notes(label: &str, tool: &Tool) -> Result<Vec<String>, ToolpathError> {
    if tool.kind != ToolType::Vbit {
        return Err(job_error(format!(
            "{label}: V-carving needs a V-bit, tool {} is a {}",
            tool.id,
            type_name(tool.kind)
        )));
    }
    if tool.angle.is_none() {
        return Err(job_error(format!(
            "{label}: tool {} is a V-bit with no angle; set its angle",
            tool.id
        )));
    }
    if tool.tip_diameter.is_none() {
        return Ok(vec![format!(
            "{label}: tool {} has no tip diameter set; V-carving takes it as pointed",
            tool.id
        )]);
    }
    Ok(Vec::new())
}

fn type_name(kind: ToolType) -> &'static str {
    match kind {
        ToolType::Endmill => "endmill",
        ToolType::Drill => "drill",
        ToolType::Vbit => "vbit",
    }
}

/// Pockets need an endmill; a straight plunge also needs it to be centre-cutting.
fn pocket_tool_notes(
    number: usize,
    tool: &Tool,
    entry: Entry,
) -> Result<Vec<String>, ToolpathError> {
    if tool.kind != ToolType::Endmill {
        return Err(job_error(format!(
            "Operation {number}: pockets need an endmill, tool {} is a {}",
            tool.id,
            type_name(tool.kind)
        )));
    }
    plunge_notes(number, "pocket", tool, entry)
}

/// A straight plunge needs a centre-cutting tool (docs/specs/ramp-entry.md §6): refused
/// for a tool marked not centre-cutting, a note when it's unknown. A ramp needs neither.
fn plunge_notes(
    number: usize,
    what: &str,
    tool: &Tool,
    entry: Entry,
) -> Result<Vec<String>, ToolpathError> {
    if entry.ramp_angle().is_some() {
        return Ok(Vec::new());
    }
    match tool.center_cutting {
        Some(false) => Err(job_error(format!(
            "Operation {number}: tool {} is not centre-cutting and this {what} plunges \
             straight down; ramp in instead (entry: ramp) or use a centre-cutting tool",
            tool.id
        ))),
        None => Ok(vec![format!(
            "Operation {number}: {what} plunges straight down; tool {} isn't marked \
             center_cutting, make sure it is a centre-cutting tool",
            tool.id
        )]),
        Some(true) => Ok(Vec::new()),
    }
}
