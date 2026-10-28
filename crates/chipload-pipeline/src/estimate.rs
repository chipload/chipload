// SPDX-License-Identifier: GPL-3.0-or-later
//! How long a job takes, estimated from its moves (docs/specs/cycle-time.md): cutting
//! moves at their feed, rapids at the rapid rate, spindle waits at every spindle start.
//! No acceleration: a real machine takes longer.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Note};
use chipload_toolpath::model::{MoveKind, Program};
use chipload_toolpath::{OpSpan, ToolGroup};

/// The rapid rate used when the machine has no max feed, mm/min (§3).
pub const ASSUMED_RAPID_MM_MIN: f64 = 5000.0;

/// Seconds, per part of the job.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Estimate {
    pub total: f64,
    pub cutting: f64,
    pub rapid: f64,
    pub spindle: f64,
    /// (operation or rest pass, seconds), in the order they are listed, each pass after its
    /// pocket; the time of all of its copies.
    pub operations: Vec<(chipload_job::OpRef, f64)>,
    /// Each tool group's time with its spindle wait: one file each when written one file
    /// per tool.
    pub groups: Vec<f64>,
    pub tool_changes: usize,
    /// mm/min.
    pub rapid_rate: f64,
    /// No max feed was set: the rate is [`ASSUMED_RAPID_MM_MIN`].
    pub rapid_assumed: bool,
    /// The box the tool cuts in, its radius included, mm: the smallest stock that holds the
    /// job (docs/specs/stock-around-drawing.md). None when nothing is cut.
    pub outline: Option<((f64, f64), (f64, f64))>,
}

/// The seconds of each move: (cutting, rapid). Lengths in 3D from where the last move
/// ended; an axis not known yet (the first moves) adds nothing.
fn move_times(program: &Program, rapid_rate: f64) -> Vec<(f64, f64)> {
    let mut at: [Option<f64>; 3] = [None, None, None];
    program
        .moves
        .iter()
        .map(|m| {
            let mut squared = 0.0;
            for (axis, to) in [m.x, m.y, m.z].into_iter().enumerate() {
                if let Some(to) = to {
                    if let Some(from) = at[axis] {
                        squared += (to - from) * (to - from);
                    }
                    at[axis] = Some(to);
                }
            }
            let length = f64::sqrt(squared);
            match (m.kind, m.feed) {
                (MoveKind::Rapid, _) => (0.0, length / rapid_rate * 60.0),
                (_, Some(feed)) if feed > 0.0 => (length / feed * 60.0, 0.0),
                // The checker refuses a cut without a feed: nothing to time.
                _ => (0.0, 0.0),
            }
        })
        .collect()
}

/// The seconds each move of `program` takes, at the job's rapid rate (as the estimate
/// counts them), for the playback slider (docs/specs/playback.md §2).
pub fn move_seconds(job: &Job, program: &Program) -> Vec<f64> {
    let rapid_rate = match job.machine.max_feed {
        Some(max) if max > 0.0 => max,
        _ => ASSUMED_RAPID_MM_MIN,
    };
    move_times(program, rapid_rate)
        .into_iter()
        .map(|(cut, rapid)| cut + rapid)
        .collect()
}

/// Estimate a planned job: `dwell_s` is the post's spindle wait, at each tool group's
/// start (one per file, or one after each tool change in one file).
pub fn estimate(
    job: &Job,
    program: &Program,
    spans: &[OpSpan],
    groups: &[ToolGroup],
    dwell_s: f64,
) -> Estimate {
    let (rapid_rate, rapid_assumed) = match job.machine.max_feed {
        Some(max) if max > 0.0 => (max, false),
        _ => (ASSUMED_RAPID_MM_MIN, true),
    };
    let times = move_times(program, rapid_rate);
    let sum = |range: std::ops::Range<usize>| -> f64 {
        times
            .get(range)
            .map_or(0.0, |part| part.iter().map(|(c, r)| c + r).sum())
    };
    let cutting: f64 = times.iter().map(|(c, _)| c).sum();
    let rapid: f64 = times.iter().map(|(_, r)| r).sum();
    let starts = groups.len().max(1);
    let spindle = dwell_s.max(0.0) * starts as f64;
    let mut operations: Vec<(chipload_job::OpRef, f64)> = Vec::new();
    for span in spans {
        let seconds = sum(span.moves.clone());
        let at = chipload_job::OpRef {
            operation: span.operation,
            pass: span.pass,
        };
        match operations.iter_mut().find(|(op, _)| *op == at) {
            Some((_, total)) => *total += seconds,
            None => operations.push((at, seconds)),
        }
    }
    operations.sort_by_key(|(op, _)| *op);
    Estimate {
        total: cutting + rapid + spindle,
        cutting,
        rapid,
        spindle,
        operations,
        groups: groups
            .iter()
            .map(|g| sum(g.moves.clone()) + dwell_s.max(0.0))
            .collect(),
        tool_changes: groups.len().saturating_sub(1),
        rapid_rate,
        rapid_assumed,
        outline: chipload_toolpath::plan::cut_extent(&program.moves, spans, None),
    }
}

/// `45 s`, `12 min 30 s`, `1 h 05 min` (§5).
pub fn duration_text(seconds: f64) -> String {
    duration_message(seconds).english()
}

/// [`duration_text`] as a message.
pub fn duration_message(seconds: f64) -> Message {
    let whole = if seconds.is_finite() && seconds > 0.0 {
        seconds.round() as u64
    } else {
        0
    };
    match whole {
        0..60 => Message::new("pipeline-duration-seconds").arg("seconds", whole),
        60..3600 => Message::new("pipeline-duration-minutes")
            .arg("minutes", whole / 60)
            .arg("seconds", whole % 60),
        _ => {
            let minutes = (whole + 30) / 60;
            Message::new("pipeline-duration-hours")
                .arg("hours", minutes / 60)
                .arg("minutes", format!("{:02}", minutes % 60))
        }
    }
}

/// The report's lines: the total and its parts, each operation, each file of a job
/// written one file per tool, and what the estimate leaves out.
pub fn estimate_lines(job: &Job, estimate: &Estimate, several_files: bool) -> Vec<Note> {
    let mut lines = vec![Note::job(
        Message::new("pipeline-estimate")
            .arg("total", duration_message(estimate.total))
            .arg("cutting", duration_message(estimate.cutting))
            .arg("rapids", duration_message(estimate.rapid))
            .arg("spindle", duration_message(estimate.spindle))
            .arg("changes", estimate.tool_changes),
    )];
    if let Some(((x0, y0), (x1, y1))) = estimate.outline {
        let number = |mm: f64| Value::LengthNumber(mm, Digits::Fixed(3));
        let length = |mm: f64| Value::Length(mm, Digits::Fixed(3));
        lines.push(Note::job(
            Message::new("pipeline-cut-outline")
                .arg("width", number(x1 - x0))
                .arg("length", length(y1 - y0))
                .arg("x0", number(x0))
                .arg("x1", number(x1))
                .arg("y0", number(y0))
                .arg("y1", number(y1)),
        ));
    }
    // With two parts or more, each part's time over its operations' (docs/specs/parts.md
    // §6); one part reads as before parts.
    let several_parts = job.parts.len() > 1;
    let mut part_shown: Option<u64> = None;
    for &(at, seconds) in &estimate.operations {
        let index = at.operation;
        let Some(op) = job.operations.get(index) else {
            continue;
        };
        if several_parts && part_shown != Some(op.part()) {
            part_shown = Some(op.part());
            if let Some((number, part)) = job
                .parts
                .iter()
                .enumerate()
                .find(|(_, p)| p.id == op.part())
            {
                let total: f64 = estimate
                    .operations
                    .iter()
                    .filter(|(r, _)| {
                        job.operations
                            .get(r.operation)
                            .is_some_and(|o| o.part() == part.id)
                    })
                    .map(|(_, s)| s)
                    .sum();
                lines.push(Note::job(
                    Message::new("pipeline-estimate-part")
                        .arg("number", number + 1)
                        .arg("name", part.name.as_str())
                        .arg("time", duration_message(total)),
                ));
            }
        }
        let id = if several_parts {
            Message::new("pipeline-estimate-part-operation")
        } else {
            Message::new("pipeline-estimate-operation")
        };
        lines.push(Note::at(
            index,
            id.arg("label", job.label_message(at))
                .arg("time", duration_message(seconds)),
        ));
    }
    if several_files {
        for (i, seconds) in estimate.groups.iter().enumerate() {
            lines.push(Note::job(
                Message::new("pipeline-estimate-file")
                    .arg("number", i + 1)
                    .arg("time", duration_message(*seconds)),
            ));
        }
    }
    let rapids = if estimate.rapid_assumed {
        Message::new("pipeline-estimate-rapids-assumed")
    } else {
        Message::new("pipeline-estimate-rapids")
    };
    lines.push(Note::job(
        rapids.arg("rate", Value::Feed(estimate.rapid_rate, Digits::Shortest)),
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_naturally() {
        assert_eq!(duration_text(44.6), "45 s");
        assert_eq!(duration_text(750.0), "12 min 30 s");
        assert_eq!(duration_text(3900.0), "1 h 05 min");
        assert_eq!(duration_text(7199.0), "2 h 00 min");
        assert_eq!(duration_text(0.2), "0 s");
        assert_eq!(duration_text(f64::NAN), "0 s");
    }
}
