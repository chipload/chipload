// SPDX-License-Identifier: GPL-3.0-or-later
//! How long a job takes, estimated from its moves (docs/specs/cycle-time.md): cutting
//! moves at their feed, rapids at the rapid rate, spindle waits at every spindle start.
//! No acceleration: a real machine takes longer.

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
    /// (index in `job.operations`, seconds), in the order the operations are listed; the
    /// time of all of an operation's copies.
    pub operations: Vec<(usize, f64)>,
    /// Each tool group's time with its spindle wait: one file each when written one file
    /// per tool.
    pub groups: Vec<f64>,
    pub tool_changes: usize,
    /// mm/min.
    pub rapid_rate: f64,
    /// No max feed was set: the rate is [`ASSUMED_RAPID_MM_MIN`].
    pub rapid_assumed: bool,
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
    let mut operations: Vec<(usize, f64)> = Vec::new();
    for span in spans {
        let seconds = sum(span.moves.clone());
        match operations.iter_mut().find(|(op, _)| *op == span.operation) {
            Some((_, total)) => *total += seconds,
            None => operations.push((span.operation, seconds)),
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
    }
}

/// `45 s`, `12 min 30 s`, `1 h 05 min` (§5).
pub fn duration_text(seconds: f64) -> String {
    let whole = if seconds.is_finite() && seconds > 0.0 {
        seconds.round() as u64
    } else {
        0
    };
    match whole {
        0..60 => format!("{whole} s"),
        60..3600 => format!("{} min {} s", whole / 60, whole % 60),
        _ => {
            let minutes = (whole + 30) / 60;
            format!("{} h {:02} min", minutes / 60, minutes % 60)
        }
    }
}

/// The report's lines: the total and its parts, each operation, each file of a job
/// written one file per tool, and what the estimate leaves out.
pub fn estimate_lines(job: &Job, estimate: &Estimate, several_files: bool) -> Vec<Note> {
    let changes = match estimate.tool_changes {
        0 => String::new(),
        1 => ", plus 1 tool change".into(),
        n => format!(", plus {n} tool changes"),
    };
    let mut lines = vec![Note::job(format!(
        "Estimated time: {} (cutting {}, rapids {}, spindle {}){changes}",
        duration_text(estimate.total),
        duration_text(estimate.cutting),
        duration_text(estimate.rapid),
        duration_text(estimate.spindle)
    ))];
    // With two parts or more, each part's time over its operations' (docs/specs/parts.md
    // §6); one part reads as before parts.
    let several_parts = job.parts.len() > 1;
    let mut part_shown: Option<u64> = None;
    for &(index, seconds) in &estimate.operations {
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
                    .filter(|(i, _)| job.operations.get(*i).is_some_and(|o| o.part() == part.id))
                    .map(|(_, s)| s)
                    .sum();
                lines.push(Note::job(format!(
                    "  Part {} ({}): {}",
                    number + 1,
                    part.name,
                    duration_text(total)
                )));
            }
        }
        let label = op.label(index);
        let indent = if several_parts { "    " } else { "  " };
        lines.push(Note::at(
            index,
            format!("{indent}{label}: {}", duration_text(seconds)),
        ));
    }
    if several_files {
        for (i, seconds) in estimate.groups.iter().enumerate() {
            lines.push(Note::job(format!(
                "  File {}: {}",
                i + 1,
                duration_text(*seconds)
            )));
        }
    }
    let rapids = if estimate.rapid_assumed {
        format!(
            "rapids at {ASSUMED_RAPID_MM_MIN} mm/min, assumed: set the machine's max feed for \
             a better estimate"
        )
    } else {
        format!(
            "rapids at the machine's max feed, {} mm/min",
            estimate.rapid_rate
        )
    };
    lines.push(Note::job(format!(
        "  At the programmed feeds ({rapids}); acceleration and corners make the real time \
         longer."
    )));
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
