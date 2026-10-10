// SPDX-License-Identifier: GPL-3.0-or-later
//! Copies of the part in a grid (docs/specs/copies-grid.md): the text shown for a grid and
//! the pitch for a gap between copies.

use chipload_job::{CopyOrder, Job};
use chipload_toolpath::plan::cut_extent;
use chipload_toolpath::plan_job;

use crate::{PipelineError, load_drawing};

/// "3 × 2 copies, pitch 120.000 × 80.000 mm, operation by operation".
pub fn copies_text(job: &Job) -> Option<String> {
    let c = job.copies?;
    Some(format!(
        "{} × {} copies, pitch {:.3} × {:.3} mm, {}",
        c.columns,
        c.rows,
        c.dx,
        c.dy,
        order_name(c.order)
    ))
}

pub fn order_name(order: CopyOrder) -> &'static str {
    match order {
        CopyOrder::ByOperation => "operation by operation",
        CopyOrder::ByPart => "part by part",
    }
}

/// Width and height the tool cuts for one part (the toolpath plus the tool radius).
pub fn cut_size(job: &Job) -> Result<(f64, f64), PipelineError> {
    let mut one = job.clone();
    one.copies = None;
    let imported = load_drawing(&one)?;
    let plan = plan_job(&one, &imported.contours)?;
    let radius = plan.program.tool_diameter / 2.0;
    let ((x0, y0), (x1, y1)) = cut_extent(&plan.program.moves, &plan.op_spans, radius, None)
        .ok_or_else(|| PipelineError::Job("The project cuts nothing".into()))?;
    Ok((x1 - x0, y1 - y0))
}

/// The pitch that leaves `gap` mm between the cuts of neighbouring copies (a one-off: the
/// project stores the pitch, decision 2). It keeps the direction the grid grows in now:
/// `current` is the pitch as it is.
pub fn pitch_for_gap(
    job: &Job,
    gap: f64,
    current: (f64, f64),
) -> Result<(f64, f64), PipelineError> {
    if !(gap.is_finite() && gap >= 0.0) {
        return Err(PipelineError::Job(format!(
            "The gap must be 0 or more, got {gap}"
        )));
    }
    let (width, height) = cut_size(job)?;
    let along = |size: f64, now: f64| {
        if now < 0.0 { -(size + gap) } else { size + gap }
    };
    Ok((along(width, current.0), along(height, current.1)))
}
