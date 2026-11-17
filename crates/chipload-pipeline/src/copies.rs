// SPDX-License-Identifier: GPL-3.0-or-later
//! Copies of a part in a grid (docs/specs/copies-grid.md, parts.md): the text shown for a
//! grid and the pitch for a gap between copies.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Instance, InstanceOrder, Job, Part, Units};
use chipload_toolpath::plan::cut_extent;
use chipload_toolpath::plan_job;

use crate::{PipelineError, load_drawing};

/// "3 in all, operation by operation": a part with its instances on the active setup
/// (docs/specs/instances.md §7).
pub fn instances_text(job: &Job, part: &Part) -> String {
    instances_message(job, part).english()
}

/// [`instances_text`] as a message.
pub fn instances_message(job: &Job, part: &Part) -> Message {
    // The part itself counts as turned when it is (docs/specs/part-placement.md).
    let spots = job.spots(part.id);
    Message::new("pipeline-instances")
        .arg("count", spots.len())
        .arg(
            "turned",
            spots.iter().filter(|(_, s)| s.angle != 0.0).count(),
        )
        .arg(
            "mirrored",
            spots.iter().filter(|(_, s)| s.mirror.is_some()).count(),
        )
        .arg("order", order_message(part.order))
}

/// The instances of a project with one part: None when it has none, or with two parts or
/// more (then [`parts_lines`] tells them).
pub fn copies_text(job: &Job) -> Option<String> {
    copies_message(job, Units::Mm).map(|m| m.english())
}

/// [`copies_text`] as a message.
pub fn copies_message(job: &Job, _units: Units) -> Option<Message> {
    match job.parts.as_slice() {
        [part] if several(job, part) => Some(instances_message(job, part)),
        _ => None,
    }
}

/// With two parts or more, one line each: "Part 2 box: 2 in all, …" or "Part 1 lid: one,
/// off" (docs/specs/parts.md §6). Empty with one part.
pub fn parts_lines(job: &Job) -> Vec<String> {
    parts_messages(job, Units::Mm)
        .iter()
        .map(Message::english)
        .collect()
}

/// [`parts_lines`] as messages.
pub fn parts_messages(job: &Job, _units: Units) -> Vec<Message> {
    if job.parts.len() < 2 {
        return Vec::new();
    }
    job.parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let copies = if several(job, part) {
                instances_message(job, part)
            } else {
                Message::new("pipeline-copies-one")
            };
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

/// Whether the part has instances on the active setup besides one, on or off: what used to
/// be "instances listed on the part".
fn several(job: &Job, part: &Part) -> bool {
    job.instances_here(part.id).len() > 1
}

/// How a part's instances are cut: "operation by operation", "instance by instance".
pub fn order_message(order: InstanceOrder) -> Message {
    Message::new(match order {
        InstanceOrder::ByOperation => "pipeline-copies-by-operation",
        InstanceOrder::ByInstance => "pipeline-instances-by-instance",
    })
}

/// Width and height the tool cuts for one copy of part `part` (the toolpath plus the tool
/// radius): the project planned with that part alone, once.
pub fn cut_size(job: &Job, part: u64) -> Result<(f64, f64), PipelineError> {
    let home = job.home_setup(part).or_else(|| job.active_setup_id());
    let mut one = home.map_or_else(|| job.clone(), |id| job.for_setup(id));
    for p in &mut one.parts {
        p.enabled = p.id == part;
    }
    // One copy as drawn: a turn would change its width and height.
    one.set_instances_here(part, vec![Instance::drawn(part, 1)]);
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
