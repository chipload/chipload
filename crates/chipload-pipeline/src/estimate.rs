// SPDX-License-Identifier: GPL-3.0-or-later
//! How long a job takes, estimated from its moves (docs/specs/cycle-time.md): cutting
//! moves at their feed, rapids at the rapid rate, spindle waits at every spindle start.
//! No acceleration: a real machine takes longer. Beside the time, the lengths cut, plunged
//! and moved at rapid and the lifts, per operation (docs/specs/operation-statistics.md).

use chipload_core::tolerances::CHAIN_TOL_MM;
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
    /// The lengths and lifts of the whole job, the moves between operations included.
    pub counts: Counts,
    /// Each operation's (or rest pass's) lengths and lifts, in the order of `operations`.
    pub operation_counts: Vec<(chipload_job::OpRef, Counts)>,
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

/// The lengths and lifts of some moves, with their time
/// (docs/specs/operation-statistics.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Counts {
    /// Seconds of the feed moves, plunges included.
    pub cutting_s: f64,
    /// Seconds at rapid.
    pub rapid_s: f64,
    /// Feed moves that move in X or Y (lines, arcs, ramps, helices), in 3D, mm.
    pub cutting_mm: f64,
    /// Feed moves along Z alone, down or up, mm.
    pub plunge_mm: f64,
    /// Rapids, in 3D, mm.
    pub rapid_mm: f64,
    /// Rapids that move up: each time the tool leaves the work, a peck's too.
    pub lifts: usize,
}

impl Counts {
    /// The time of these moves.
    pub fn seconds(&self) -> f64 {
        self.cutting_s + self.rapid_s
    }

    fn add(&mut self, other: &Counts) {
        self.cutting_s += other.cutting_s;
        self.rapid_s += other.rapid_s;
        self.cutting_mm += other.cutting_mm;
        self.plunge_mm += other.plunge_mm;
        self.rapid_mm += other.rapid_mm;
        self.lifts += other.lifts;
    }

    fn sum<'a>(moves: impl IntoIterator<Item = &'a Counts>) -> Counts {
        let mut total = Counts::default();
        for m in moves {
            total.add(m);
        }
        total
    }
}

/// Each move's [`Counts`]. Lengths in 3D from where the last move ended; an axis not known
/// yet (the first moves) adds nothing.
fn move_counts(program: &Program, rapid_rate: f64) -> Vec<Counts> {
    let mut at: [Option<f64>; 3] = [None, None, None];
    program
        .moves
        .iter()
        .map(|m| {
            let mut step = [0.0; 3];
            for (axis, to) in [m.x, m.y, m.z].into_iter().enumerate() {
                if let Some(to) = to {
                    if let Some(from) = at[axis] {
                        step[axis] = to - from;
                    }
                    at[axis] = Some(to);
                }
            }
            let [dx, dy, dz] = step;
            let length = f64::sqrt(dx * dx + dy * dy + dz * dz);
            let in_xy = f64::hypot(dx, dy) > CHAIN_TOL_MM;
            match (m.kind, m.feed) {
                (MoveKind::Rapid, _) => Counts {
                    rapid_s: length / rapid_rate * 60.0,
                    rapid_mm: length,
                    lifts: usize::from(dz > CHAIN_TOL_MM),
                    ..Counts::default()
                },
                (_, Some(feed)) if feed > 0.0 => Counts {
                    cutting_s: length / feed * 60.0,
                    cutting_mm: if in_xy { length } else { 0.0 },
                    plunge_mm: if in_xy { 0.0 } else { length },
                    ..Counts::default()
                },
                // The checker refuses a cut without a feed: nothing to count.
                _ => Counts::default(),
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
    move_counts(program, rapid_rate)
        .iter()
        .map(Counts::seconds)
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
    let counts = move_counts(program, rapid_rate);
    let sum = |range: std::ops::Range<usize>| -> Counts {
        counts.get(range).map_or(Counts::default(), Counts::sum)
    };
    let all = Counts::sum(&counts);
    let starts = groups.len().max(1);
    let spindle = dwell_s.max(0.0) * starts as f64;
    let mut operation_counts: Vec<(chipload_job::OpRef, Counts)> = Vec::new();
    for span in spans {
        let part = sum(span.moves.clone());
        let at = chipload_job::OpRef {
            operation: span.operation,
            pass: span.pass,
        };
        match operation_counts.iter_mut().find(|(op, _)| *op == at) {
            Some((_, total)) => total.add(&part),
            None => operation_counts.push((at, part)),
        }
    }
    operation_counts.sort_by_key(|(op, _)| *op);
    let operations = operation_counts
        .iter()
        .map(|(at, c)| (*at, c.seconds()))
        .collect();
    Estimate {
        total: all.seconds() + spindle,
        cutting: all.cutting_s,
        rapid: all.rapid_s,
        spindle,
        operations,
        counts: all,
        operation_counts,
        groups: groups
            .iter()
            .map(|g| sum(g.moves.clone()).seconds() + dwell_s.max(0.0))
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

/// Lengths and lifts in a line: `cutting 12.4 m, plunging 86 mm, rapids 3.1 m, 23 lifts`.
pub fn counts_message(counts: &Counts) -> Message {
    Message::new("pipeline-counts")
        .arg("cutting", Value::Distance(counts.cutting_mm))
        .arg("plunging", Value::Distance(counts.plunge_mm))
        .arg("rapids", Value::Distance(counts.rapid_mm))
        .arg("lifts", counts.lifts)
}

/// Which lengths and lifts the estimate's lines give (docs/specs/operation-statistics.md
/// §3.2-3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lengths {
    /// None: the setup sheet.
    Without,
    /// The whole job's, in one line: the Report, `chipload run`.
    Totals,
    /// The whole job's and each operation's: `chipload plan`.
    PerOperation,
}

/// One row of the estimate's breakdown, for a table (the app's report,
/// docs/specs/look-and-feel.md §6) or a line ([`estimate_lines`]).
#[derive(Debug, Clone, PartialEq)]
pub struct EstimateRow {
    pub kind: RowKind,
    /// Its time, copies included.
    pub seconds: f64,
}

/// What an [`EstimateRow`] is the time of.
#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    /// A part, with two parts or more: its operations follow (docs/specs/parts.md §6).
    Part { number: usize, name: String },
    /// An operation or a rest pass; `in_part` under a part's row.
    Operation {
        at: chipload_job::OpRef,
        in_part: bool,
    },
    /// A file of a job written one file per tool.
    File { number: usize },
}

/// The estimate's breakdown in order: each part (with two or more) before its operations,
/// each operation, then each file when the job is written one file per tool.
pub fn estimate_rows(job: &Job, estimate: &Estimate, several_files: bool) -> Vec<EstimateRow> {
    let mut rows = Vec::new();
    // With two parts or more, each part's time over its operations'; one part reads as
    // before parts.
    let several_parts = job.parts.len() > 1;
    let mut part_shown: Option<u64> = None;
    for &(at, seconds) in &estimate.operations {
        let Some(op) = job.operations.get(at.operation) else {
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
                rows.push(EstimateRow {
                    kind: RowKind::Part {
                        number: number + 1,
                        name: part.name.clone(),
                    },
                    seconds: total,
                });
            }
        }
        rows.push(EstimateRow {
            kind: RowKind::Operation {
                at,
                in_part: several_parts,
            },
            seconds,
        });
    }
    if several_files {
        for (i, seconds) in estimate.groups.iter().enumerate() {
            rows.push(EstimateRow {
                kind: RowKind::File { number: i + 1 },
                seconds: *seconds,
            });
        }
    }
    rows
}

/// The report's lines: the total and its parts, each operation, each file of a job
/// written one file per tool, the lengths and lifts as `lengths` says, and what the
/// estimate leaves out.
pub fn estimate_lines(
    job: &Job,
    estimate: &Estimate,
    several_files: bool,
    lengths: Lengths,
) -> Vec<Note> {
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
    for row in estimate_rows(job, estimate, several_files) {
        let time = duration_message(row.seconds);
        match row.kind {
            RowKind::Part { number, name } => lines.push(Note::job(
                Message::new("pipeline-estimate-part")
                    .arg("number", number)
                    .arg("name", name)
                    .arg("time", time),
            )),
            RowKind::Operation { at, in_part } => {
                let counts = estimate
                    .operation_counts
                    .iter()
                    .find(|(r, _)| *r == at)
                    .map(|(_, c)| c)
                    .filter(|_| lengths == Lengths::PerOperation);
                let id = match (in_part, counts.is_some()) {
                    (true, false) => "pipeline-estimate-part-operation",
                    (false, false) => "pipeline-estimate-operation",
                    (true, true) => "pipeline-estimate-part-operation-counted",
                    (false, true) => "pipeline-estimate-operation-counted",
                };
                let mut line = Message::new(id)
                    .arg("label", job.label_message(at))
                    .arg("time", time);
                if let Some(counts) = counts {
                    line = line.arg("counts", counts_message(counts));
                }
                lines.push(Note::at(at.operation, line));
            }
            RowKind::File { number } => lines.push(Note::job(
                Message::new("pipeline-estimate-file")
                    .arg("number", number)
                    .arg("time", time),
            )),
        }
    }
    if lengths != Lengths::Without {
        lines.push(Note::job(
            Message::new("pipeline-estimate-counts")
                .arg("counts", counts_message(&estimate.counts)),
        ));
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
