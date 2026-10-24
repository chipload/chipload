// SPDX-License-Identifier: GPL-3.0-or-later
//! Copies of a part in a grid (docs/specs/copies-grid.md, parts.md): the text shown for a
//! grid and the pitch for a gap between copies.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Copies, CopyOrder, Job, Units};
use chipload_toolpath::plan::cut_extent;
use chipload_toolpath::plan_job;

use crate::{PipelineError, load_drawing};

/// "3 × 2 copies, pitch 120.000 × 80.000 mm, operation by operation".
pub fn grid_text(c: &Copies) -> String {
    grid_text_in(c, Units::Mm)
}

/// As [`grid_text`], the pitch in `units` (docs/specs/inch-units.md §5.1).
pub fn grid_text_in(c: &Copies, units: Units) -> String {
    let d = crate::units::decimals(units);
    let n = |mm: f64| format!("{:.d$}", crate::units::from_mm(mm, units));
    format!(
        "{} × {} copies, pitch {} × {} {}, {}",
        c.columns,
        c.rows,
        n(c.dx),
        n(c.dy),
        crate::units::length_unit(units),
        order_name(c.order)
    )
}

/// The copies of a project with one part, as before parts came: None without a grid or
/// with two parts or more (then [`parts_lines`] tells them).
pub fn copies_text(job: &Job) -> Option<String> {
    copies_text_in(job, Units::Mm)
}

/// As [`copies_text`], in `units`.
pub fn copies_text_in(job: &Job, units: Units) -> Option<String> {
    match job.parts.as_slice() {
        [part] => part.copies.as_ref().map(|c| grid_text_in(c, units)),
        _ => None,
    }
}

/// With two parts or more, one line each: "Part 2 box: 2 × 1 copies, …" or "Part 1 lid:
/// one, off" (docs/specs/parts.md §6). Empty with one part.
pub fn parts_lines(job: &Job) -> Vec<String> {
    parts_lines_in(job, Units::Mm)
}

/// As [`parts_lines`], in `units`.
pub fn parts_lines_in(job: &Job, units: Units) -> Vec<String> {
    if job.parts.len() < 2 {
        return Vec::new();
    }
    job.parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let copies = part
                .copies
                .as_ref()
                .map_or("one".to_string(), |c| grid_text_in(c, units));
            let off = if part.enabled { "" } else { ", off: not cut" };
            format!("Part {} {}: {copies}{off}", i + 1, part.name)
        })
        .collect()
}

pub fn order_name(order: CopyOrder) -> &'static str {
    match order {
        CopyOrder::ByOperation => "operation by operation",
        CopyOrder::ByCopy => "copy by copy",
    }
}

/// Width and height the tool cuts for one copy of part `part` (the toolpath plus the tool
/// radius): the project planned with that part alone, once.
pub fn cut_size(job: &Job, part: u64) -> Result<(f64, f64), PipelineError> {
    let mut one = job.clone();
    for p in &mut one.parts {
        p.enabled = p.id == part;
        p.copies = None;
    }
    let imported = load_drawing(&one)?;
    let plan = plan_job(&one, &imported.contours)?;
    let ((x0, y0), (x1, y1)) = cut_extent(&plan.program.moves, &plan.op_spans, None)
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-part-cuts-nothing")))?;
    Ok((x1 - x0, y1 - y0))
}

/// The pitch that leaves `gap` mm between the cuts of neighbouring copies (a one-off: the
/// project stores the pitch, decision 2). It keeps the direction the grid grows in now:
/// `current` is the pitch as it is.
pub fn pitch_for_gap(
    job: &Job,
    part: u64,
    gap: f64,
    current: (f64, f64),
) -> Result<(f64, f64), PipelineError> {
    if !(gap.is_finite() && gap >= 0.0) {
        return Err(PipelineError::Job(
            Message::new("pipeline-copies-gap").arg("got", Value::Length(gap, Digits::Shortest)),
        ));
    }
    let (width, height) = cut_size(job, part)?;
    let along = |size: f64, now: f64| {
        if now < 0.0 { -(size + gap) } else { size + gap }
    };
    Ok((along(width, current.0), along(height, current.1)))
}
