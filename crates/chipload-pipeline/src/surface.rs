// SPDX-License-Identifier: GPL-3.0-or-later
//! Adding a surface (docs/specs/surfacing.md): the whole stock taken down flat, first in its
//! part.

use chipload_job::library::Cutting;
use chipload_job::{Job, Operation, SurfaceOp, ToolType};

use crate::PipelineError;
use crate::chip_load::{default_feed, default_plunge_feed};

/// How much a new surface takes off, mm (docs/specs/surfacing.md §4).
pub const DEFAULT_SURFACE_DEPTH: f64 = 0.5;

/// A new surface's stepover, as a fraction of the tool diameter (docs/specs/surfacing.md §4).
pub const DEFAULT_SURFACE_STEPOVER: f64 = 0.75;

/// The tool a new surface takes: the project's widest endmill.
pub fn surface_tool(job: &Job) -> Option<i64> {
    job.tools
        .iter()
        .filter(|t| t.kind == ToolType::Endmill)
        .max_by(|a, b| a.diameter.total_cmp(&b.diameter))
        .map(|t| t.id)
}

/// Where a surface goes in part `part`: before its first operation (it faces the stock
/// before anything is cut, docs/specs/surfacing.md §6), or where the part's operations go
/// when it has none.
pub fn surface_index(job: &Job, part: u64) -> usize {
    job.operations
        .iter()
        .position(|op| op.part() == part)
        .unwrap_or_else(|| job.insert_index(part))
}

/// A new surface in the active part, first among its operations, with `tool` (default: the
/// widest endmill) taking `depth` mm off (default 0.5 mm, in one pass). Feeds and step down
/// from `cutting` (the tool's library values), else from the tool's diameter. Returns its
/// index.
pub fn add_surface(
    job: &mut Job,
    tool: Option<i64>,
    depth: Option<f64>,
    cutting: Option<&Cutting>,
) -> Result<usize, PipelineError> {
    let id = match tool.or_else(|| surface_tool(job)) {
        Some(id) => id,
        None => {
            return Err(PipelineError::Job(
                "the project has no endmill: a surface needs one (copy one in with `tool use`)"
                    .into(),
            ));
        }
    };
    let found = job
        .tool(id)
        .ok_or_else(|| PipelineError::Job(format!("the project has no tool {id}")))?;
    if found.kind != ToolType::Endmill {
        return Err(PipelineError::Job(format!(
            "tool {id} is a {}; a surface needs an endmill",
            found.kind.name()
        )));
    }
    let diameter = found.diameter;
    let depth = depth.unwrap_or(DEFAULT_SURFACE_DEPTH);
    let (feed, plunge_feed, step_down) = match cutting {
        Some(c) => (c.feed, c.plunge_feed, c.step_down.max(depth)),
        None => (default_feed(diameter), default_plunge_feed(diameter), depth),
    };
    let part = job.active_part_id().unwrap_or(1);
    let number = job
        .operations
        .iter()
        .filter(|op| matches!(op, Operation::Surface(_)))
        .count()
        + 1;
    let index = surface_index(job, part);
    job.operations.insert(
        index,
        Operation::Surface(SurfaceOp {
            name: format!("Surface {number}"),
            enabled: true,
            part,
            tool: id,
            start_depth: 0.0,
            end_depth: depth,
            step_down,
            stepover: DEFAULT_SURFACE_STEPOVER,
            feed,
            plunge_feed,
        }),
    );
    Ok(index)
}
