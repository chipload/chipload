// SPDX-License-Identifier: GPL-3.0-or-later
//! Job + imported contours -> Program. The toolpath half of `core/pipeline.py::run_job`;
//! post-processing and the independent check come in a later step of the port.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_job::{Job, Operation, Tool, ToolType, ZZero};

use crate::ToolpathError;
use crate::model::{Move, Program};
use crate::profile::{contour_moves, pass_depths, plan_profile};
use crate::select::select_contours;

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub program: Program,
    /// Warnings for the user.
    pub notes: Vec<String>,
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
    let mut position: Point = (0.0, 0.0);
    for (index, op) in job.operations.iter().enumerate() {
        let label = match op_name(op) {
            "" => format!("Operation {}", index + 1),
            name => format!("Operation {} ({name})", index + 1),
        };
        let Operation::Profile(profile) = op else {
            return Err(not_ported(&label, op));
        };
        if profile.tabs.is_some() {
            return Err(job_error(format!(
                "{label}: tabs are not ported to the Rust version yet"
            )));
        }
        let selection = select_contours(contours, &profile.select, &label)?;
        notes.extend(selection.notes);
        let pass_z: Vec<f64> = pass_depths(profile.depth, profile.step_down)?
            .into_iter()
            .map(|d| top_z - d)
            .collect();
        let plan = plan_profile(
            profile,
            &label,
            &selection.contours,
            tool.diameter / 2.0,
            job.machine.spindle,
            position,
        )?;
        notes.extend(plan.notes);
        for ring in &plan.rings {
            moves.extend(contour_moves(
                &ring.points,
                &pass_z,
                job.machine.safe_z,
                profile.feed,
                profile.plunge_feed,
            ));
            position = ring.points[0];
        }
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
    })
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
fn validate_job(job: &Job) -> Result<(&Tool, Vec<String>), ToolpathError> {
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
