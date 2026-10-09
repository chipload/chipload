// SPDX-License-Identifier: GPL-3.0-or-later
//! Job + imported contours -> Program. The toolpath half of `core/pipeline.py::run_job`;
//! post-processing and the independent check come in a later step of the port.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_job::{Job, Operation, PocketOp, ProfileOp, Side, Tool, ToolType, ZZero};

use crate::ToolpathError;
use crate::model::{Move, Program};
use crate::pocket::{PocketPiece, plan_pocket, pocket_moves};
use crate::profile::{contour_moves, pass_depths, plan_profile};
use crate::select::{Selection, select_contours};
use crate::tabs::{place_tabs, tab_polylines, tabbed_contour_moves};

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub program: Program,
    /// Warnings for the user.
    pub notes: Vec<String>,
    /// Tool-path stretch of every tab, for display.
    pub tabs: Vec<Vec<Point>>,
    /// Every pocket piece in cut order (rings and links), for display and conformance tests.
    pub pockets: Vec<PocketPiece>,
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
    let mut position: Point = (0.0, 0.0);
    for (index, op) in job.operations.iter().enumerate() {
        let label = match op_name(op) {
            "" => format!("Operation {}", index + 1),
            name => format!("Operation {} ({name})", index + 1),
        };
        let (select, depth, step_down) = match op {
            Operation::Profile(p) => (&p.select, p.depth, p.step_down),
            Operation::Pocket(p) => (&p.select, p.depth, p.step_down),
            Operation::Drill(_) => return Err(not_ported(&label, op)),
        };
        let selection = select_contours(contours, select, &label)?;
        notes.extend(selection.notes.iter().cloned());
        let pass_z: Vec<f64> = pass_depths(depth, step_down)?
            .into_iter()
            .map(|d| top_z - d)
            .collect();
        let context = Context {
            label: &label,
            tool,
            job,
            pass_z: &pass_z,
        };
        let planned = match op {
            Operation::Pocket(pocket) => plan_pocket_op(pocket, &context, &selection, position)?,
            Operation::Profile(profile) => {
                plan_profile_op(profile, &context, &selection, position)?
            }
            Operation::Drill(_) => return Err(not_ported(&label, op)),
        };
        moves.extend(planned.moves);
        notes.extend(planned.notes);
        tabs.extend(planned.tabs);
        pockets.extend(planned.pockets);
        position = planned.end;
    }
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
    })
}

/// What every operation needs from the job.
struct Context<'a> {
    label: &'a str,
    tool: &'a Tool,
    job: &'a Job,
    pass_z: &'a [f64],
}

/// One operation's moves, warnings and tabs, and where the tool ends up (above that point).
struct Planned {
    moves: Vec<Move>,
    notes: Vec<String>,
    tabs: Vec<Vec<Point>>,
    pockets: Vec<PocketPiece>,
    end: Point,
}

fn plan_pocket_op(
    op: &PocketOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let label = ctx.label;
    let plan = plan_pocket(
        &selection.contours,
        ctx.tool.diameter / 2.0,
        op.stepover * ctx.tool.diameter,
        op.direction,
        ctx.job.machine.spindle,
        op.keep_tool_down,
        start,
    )
    .map_err(|err| err.with_context(label))?;
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan.notes.iter().map(|n| format!("{label}: {n}")).collect(),
        tabs: Vec::new(),
        pockets: Vec::new(),
        end: start,
    };
    for piece in &plan.pieces {
        out.moves.extend(pocket_moves(
            piece,
            ctx.pass_z,
            ctx.job.machine.safe_z,
            op.feed,
            op.plunge_feed,
        ));
        // Retracted above the start of the wall ring.
        if let Some(&end) = piece.rings.last().and_then(|r| r.first()) {
            out.end = end;
        }
    }
    out.pockets = plan.pieces;
    Ok(out)
}

fn plan_profile_op(
    op: &ProfileOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
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
    )?;
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan.notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        end: start,
    };
    if let Some(tabs) = &op.tabs {
        if tabs.height >= op.depth {
            return Err(job_error(format!(
                "{label}: tab height {} must be less than the cut depth {}",
                tabs.height, op.depth
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
        match &op.tabs {
            None => out.moves.extend(contour_moves(
                points,
                ctx.pass_z,
                machine.safe_z,
                op.feed,
                op.plunge_feed,
            )),
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
                        bottom + tabs.height,
                        machine.safe_z,
                        op.feed,
                        op.plunge_feed,
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

fn op_name(op: &Operation) -> &str {
    match op {
        Operation::Profile(p) => &p.name,
        Operation::Pocket(p) => &p.name,
        Operation::Drill(p) => &p.name,
    }
}

fn not_ported(label: &str, op: &Operation) -> ToolpathError {
    let kind = match op {
        Operation::Profile(_) => "profile",
        Operation::Pocket(_) => "pocket",
        Operation::Drill(_) => "drill",
    };
    job_error(format!(
        "{label}: '{kind}' is not ported to the Rust version yet"
    ))
}

fn job_error(message: String) -> ToolpathError {
    ToolpathError::Job(message)
}

/// Generation-time rules from the specs. Returns the single tool used and warnings.
pub fn validate_job(job: &Job) -> Result<(&Tool, Vec<String>), ToolpathError> {
    if job.operations.is_empty() {
        return Err(job_error("Job has no operations".into()));
    }
    for (index, op) in job.operations.iter().enumerate() {
        if let Operation::Drill(_) = op {
            return Err(job_error(format!(
                "Operation {}: 'drill' is not implemented in this version",
                index + 1
            )));
        }
    }
    let mut used: Vec<i64> = job.operations.iter().map(Operation::tool).collect();
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
    for (index, op) in job.operations.iter().enumerate() {
        let number = index + 1;
        let (depth, feed, plunge_feed) = match op {
            Operation::Profile(p) => (p.depth, p.feed, p.plunge_feed),
            Operation::Pocket(p) => (p.depth, p.feed, p.plunge_feed),
            Operation::Drill(_) => continue,
        };
        if depth > job.stock.thickness {
            return Err(job_error(format!(
                "Operation {number}: depth {depth} is deeper than the stock ({}); overcut is \
                 not supported yet",
                job.stock.thickness
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
        if let Operation::Pocket(_) = op {
            notes.extend(pocket_tool_notes(number, tool)?);
        }
    }
    Ok((tool, notes))
}

/// Pockets plunge straight down (no ramping yet), so they need a centre-cutting endmill.
fn pocket_tool_notes(number: usize, tool: &Tool) -> Result<Vec<String>, ToolpathError> {
    if tool.kind != ToolType::Endmill {
        let kind = match tool.kind {
            ToolType::Endmill => "endmill",
            ToolType::Drill => "drill",
            ToolType::Vbit => "vbit",
        };
        return Err(job_error(format!(
            "Operation {number}: pockets need an endmill, tool {} is a {kind}",
            tool.id
        )));
    }
    match tool.center_cutting {
        Some(false) => Err(job_error(format!(
            "Operation {number}: tool {} is not centre-cutting and pockets plunge straight \
             down; use a centre-cutting endmill",
            tool.id
        ))),
        None => Ok(vec![format!(
            "Operation {number}: pocket plunges straight down; tool {} isn't marked \
             center_cutting, make sure it is a centre-cutting endmill",
            tool.id
        )]),
        Some(true) => Ok(Vec::new()),
    }
}
