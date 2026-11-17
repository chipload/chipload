// SPDX-License-Identifier: GPL-3.0-or-later
//! Adding a board surface (docs/specs/surfacing.md, surface-levels.md): the whole stock taken
//! down flat, first in its setup.

use chipload_i18n::Message;
use chipload_job::library::Cutting;
use chipload_job::{Job, Operation, SurfaceOp, ToolType};

use crate::PipelineError;
use crate::chip_load::{default_feed, default_plunge_feed};

/// How much a new surface takes off, mm (docs/specs/surfacing.md §4).
pub const DEFAULT_SURFACE_DEPTH: f64 = 0.5;

/// A new surface's stepover, as a fraction of the tool diameter (docs/specs/surfacing.md §4).
pub const DEFAULT_SURFACE_STEPOVER: f64 = 0.75;

/// A new part surface's stepover: its rings leave nothing uncut up to half the tool, as a
/// pocket's (docs/specs/pocket.md §3, surface-levels.md §3).
pub const DEFAULT_PART_SURFACE_STEPOVER: f64 = 0.5;

/// The tool a new surface takes: the project's widest endmill.
pub fn surface_tool(job: &Job) -> Option<i64> {
    job.tools
        .iter()
        .filter(|t| t.kind == ToolType::Endmill)
        .max_by(|a, b| a.diameter.total_cmp(&b.diameter))
        .map(|t| t.id)
}

/// Where a board surface of setup `setup` goes: its part (the setup's first) and its index,
/// before the setup's first operation, so it faces the stock before anything is cut
/// (docs/specs/surface-levels.md §2, decision 3). None for a setup without parts.
pub fn board_surface_place(job: &Job, setup: u64) -> Option<(u64, usize)> {
    let part = *job.parts_on(setup).first()?;
    let index = job
        .operations
        .iter()
        .position(|op| job.part_on(op.part(), setup))
        .unwrap_or_else(|| job.insert_index(part));
    Some((part, index))
}

/// A new board surface in the active setup: [`add_board_surface`].
pub fn add_surface(
    job: &mut Job,
    tool: Option<i64>,
    depth: Option<f64>,
    cutting: Option<&Cutting>,
) -> Result<usize, PipelineError> {
    let setup = job.active_setup_id().unwrap_or(1);
    add_board_surface(job, setup, tool, depth, cutting)
}

/// A new board surface in setup `setup`, first among its operations, with `tool` (default:
/// the widest endmill) taking `depth` mm off (default 0.5 mm, in one pass). Feeds and step
/// down from `cutting` (the tool's library values), else from the tool's diameter. Returns
/// its index.
pub fn add_board_surface(
    job: &mut Job,
    setup: u64,
    tool: Option<i64>,
    depth: Option<f64>,
    cutting: Option<&Cutting>,
) -> Result<usize, PipelineError> {
    if job.setup(setup).is_none() {
        return Err(PipelineError::Job(
            Message::new("pipeline-no-setup").arg("id", setup),
        ));
    }
    let id = match tool.or_else(|| surface_tool(job)) {
        Some(id) => id,
        None => {
            return Err(PipelineError::Job(Message::new(
                "pipeline-surface-no-endmill",
            )));
        }
    };
    let found = job
        .tool(id)
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-no-tool").arg("tool", id)))?;
    if found.kind != ToolType::Endmill {
        return Err(PipelineError::Job(
            Message::new("pipeline-surface-not-endmill")
                .arg("tool", id)
                .arg("type", found.kind.name()),
        ));
    }
    let diameter = found.diameter;
    let depth = depth.unwrap_or(DEFAULT_SURFACE_DEPTH);
    let (feed, plunge_feed, step_down) = match cutting {
        Some(c) => (c.feed, c.plunge_feed, c.step_down.max(depth)),
        None => (default_feed(diameter), default_plunge_feed(diameter), depth),
    };
    let (part, index) = board_surface_place(job, setup)
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-setup-no-parts")))?;
    let number = job
        .operations
        .iter()
        .filter(|op| matches!(op, Operation::Surface(_)))
        .count()
        + 1;
    job.operations.insert(
        index,
        Operation::Surface(SurfaceOp {
            name: format!("Surface {number}"),
            enabled: true,
            part,
            select: chipload_job::Selector::default(),
            margin: diameter / 2.0,
            material: None,
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
