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
    grid_message(c, Units::Mm).english()
}

/// [`grid_text`] as a message, the pitch in `units` (docs/specs/inch-units.md §5.1): 3
/// decimals in mm and 4 in inches, kept even when they are zeros, so the pitch is converted
/// here.
pub fn grid_message(c: &Copies, units: Units) -> Message {
    let d = crate::units::decimals(units);
    let n = |mm: f64| Value::Number(crate::units::from_mm(mm, units), Digits::Fixed(d));
    Message::new("pipeline-copies-grid")
        .arg("columns", c.columns)
        .arg("rows", c.rows)
        .arg("dx", n(c.dx))
        .arg("dy", n(c.dy))
        .arg("unit", crate::units::length_unit(units))
        .arg("order", order_message(c.order))
}

/// The copies of a project with one part, as before parts came: None without a grid or
/// with two parts or more (then [`parts_lines`] tells them).
pub fn copies_text(job: &Job) -> Option<String> {
    copies_message(job, Units::Mm).map(|m| m.english())
}

/// [`copies_text`] as a message, in `units`.
pub fn copies_message(job: &Job, units: Units) -> Option<Message> {
    match job.parts.as_slice() {
        [part] => part.copies.as_ref().map(|c| grid_message(c, units)),
        _ => None,
    }
}

/// With two parts or more, one line each: "Part 2 box: 2 × 1 copies, …" or "Part 1 lid:
/// one, off" (docs/specs/parts.md §6). Empty with one part.
pub fn parts_lines(job: &Job) -> Vec<String> {
    parts_messages(job, Units::Mm)
        .iter()
        .map(Message::english)
        .collect()
}

/// [`parts_lines`] as messages, in `units`.
pub fn parts_messages(job: &Job, units: Units) -> Vec<Message> {
    if job.parts.len() < 2 {
        return Vec::new();
    }
    job.parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let copies = part.copies.as_ref().map_or_else(
                || Message::new("pipeline-copies-one"),
                |c| grid_message(c, units),
            );
            let id = if part.enabled {
                "pipeline-copies-part"
            } else {
                "pipeline-copies-part-off"
            };
            Message::new(id)
                .arg("number", i + 1)
                .arg("name", part.name.as_str())
                .arg("copies", copies)
        })
        .collect()
}

/// How the copies are cut: "operation by operation", "copy by copy".
pub fn order_message(order: CopyOrder) -> Message {
    Message::new(match order {
        CopyOrder::ByOperation => "pipeline-copies-by-operation",
        CopyOrder::ByCopy => "pipeline-copies-by-copy",
    })
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
