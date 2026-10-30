// SPDX-License-Identifier: GPL-3.0-or-later
//! Chip load: how thick a chip each cutting edge takes, feed / (rpm × flutes), mm per tooth
//! (docs/specs/small-items.md §5). Shown, not judged: a good value depends on the material,
//! the tool and the machine (decision 6, option A).

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{HoleMethod, Job, Operation, Tool};

/// An operation's chip load, or what is missing to know it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChipLoad {
    /// mm per tooth.
    Mm(f64),
    /// The tool has no number of flutes.
    NoFlutes,
    /// The tool has no spindle speed.
    NoRpm,
    /// The operation's tool isn't in the project.
    NoTool,
}

/// Where typical values lie, for the hover text next to the number.
pub fn chip_load_hint() -> Message {
    let number = |mm: f64| Value::LengthNumber(mm, Digits::Trim(2));
    Message::new("pipeline-chip-load-hint")
        .arg("small_tool", number(3.0))
        .arg("large_tool", Value::Length(12.0, Digits::Trim(2)))
        .arg("wood_low", number(0.05))
        .arg("wood_high", Value::ChipLoad(0.25, Digits::Fixed(2)))
        .arg("aluminium_low", number(0.02))
        .arg("aluminium_high", number(0.08))
}

/// The feeds without a tool to go by (no tool chosen yet).
pub const DEFAULT_FEED: f64 = 500.0;
pub const DEFAULT_PLUNGE_FEED: f64 = 150.0;

/// The feed a new operation starts with when the tool has no cutting values in the library
/// and there is no operation above to copy from: 250 mm/min per mm of diameter (6 mm:
/// 1500), within 300 to 3000, to the nearest 10. A starting point for wood and plastics
/// at a hobby spindle's speed, to be checked against the chip load.
pub fn default_feed(diameter: f64) -> f64 {
    if diameter <= 0.0 || diameter.is_nan() {
        return DEFAULT_FEED;
    }
    ((diameter * 250.0).clamp(300.0, 3000.0) / 10.0).round() * 10.0
}

/// A third of [`default_feed`], to the nearest 10.
pub fn default_plunge_feed(diameter: f64) -> f64 {
    if diameter <= 0.0 || diameter.is_nan() {
        return DEFAULT_PLUNGE_FEED;
    }
    (default_feed(diameter) / 30.0).round() * 10.0
}
/// The chip load of cutting at `feed` (mm/min) with `tool`.
pub fn chip_load_of(feed: f64, tool: &Tool) -> ChipLoad {
    let Some(flutes) = tool.flutes.filter(|n| *n > 0) else {
        return ChipLoad::NoFlutes;
    };
    let Some(rpm) = tool.rpm.filter(|r| *r > 0.0) else {
        return ChipLoad::NoRpm;
    };
    ChipLoad::Mm(feed / (rpm * flutes as f64))
}

/// The cutting feed of an operation, mm/min.
pub fn cutting_feed(op: &Operation) -> f64 {
    match op {
        Operation::Profile(p) => p.feed,
        Operation::Pocket(p) => p.feed,
        Operation::Vcarve(v) => v.feed,
        Operation::Path(p) => p.feed,
        // A drill cuts going down (docs/specs/holes.md §3).
        Operation::Holes(h) => match h.method {
            HoleMethod::Drill { .. } => h.plunge_feed,
            HoleMethod::Bore { .. } => h.feed,
        },
        Operation::Surface(f) => f.feed,
    }
}

/// The chip load of an operation at its cutting feed (the plunge feed has none: going down
/// the tool cuts with its tip).
pub fn chip_load(job: &Job, op: &Operation) -> ChipLoad {
    match job.tool(op.tool()) {
        Some(tool) => chip_load_of(cutting_feed(op), tool),
        None => ChipLoad::NoTool,
    }
}

impl ChipLoad {
    /// mm per tooth, if known.
    pub fn mm(self) -> Option<f64> {
        match self {
            ChipLoad::Mm(mm) => Some(mm),
            _ => None,
        }
    }

    /// `0.042 mm/tooth`, or what to set to know it.
    pub fn text(self) -> String {
        self.text_in(chipload_job::Units::Mm)
    }

    /// [`ChipLoad::text`] as a message.
    pub fn message(self) -> Message {
        match self {
            ChipLoad::Mm(mm) => Message::new("pipeline-chip-load")
                .arg("load", Value::ChipLoad(mm, Digits::Fixed(3))),
            ChipLoad::NoFlutes => Message::new("pipeline-chip-load-no-flutes"),
            ChipLoad::NoRpm => Message::new("pipeline-chip-load-no-rpm"),
            ChipLoad::NoTool => Message::new("pipeline-chip-load-no-tool"),
        }
    }

    /// As [`ChipLoad::text`], in `units`: `0.0017 in/tooth` (docs/specs/inch-units.md §5.1).
    pub fn text_in(self, units: chipload_job::Units) -> String {
        match self {
            ChipLoad::Mm(mm) => match units {
                chipload_job::Units::Mm => format!("{mm:.3} mm/tooth"),
                chipload_job::Units::Inch => {
                    format!("{:.4} in/tooth", mm / chipload_core::MM_PER_INCH)
                }
            },
            other => other
                .message()
                .render(chipload_i18n::Language::En, units.into()),
        }
    }
}

/// One line per operation that is cut, for `chipload plan`.
pub fn chip_load_lines(job: &Job) -> Vec<String> {
    chip_load_messages(job)
        .iter()
        .map(Message::english)
        .collect()
}

/// [`chip_load_lines`] as messages.
pub fn chip_load_messages(job: &Job) -> Vec<Message> {
    let mut lines = vec![Message::new("pipeline-chip-load-title")];
    for (i, op) in job
        .operations
        .iter()
        .enumerate()
        .filter(|(i, _)| job.cuts(*i))
    {
        lines.push(
            Message::new("pipeline-chip-load-line")
                .arg("label", op.label_message(i))
                .arg("load", chip_load(job, op).message()),
        );
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::ToolType;

    fn tool(flutes: Option<i64>, rpm: Option<f64>) -> Tool {
        Tool {
            id: 1,
            name: None,
            library_uid: None,
            kind: ToolType::Endmill,
            diameter: 6.0,
            flutes,
            center_cutting: None,
            rpm,
            angle: None,
            tip_diameter: None,
        }
    }

    #[test]
    fn feed_over_rpm_times_flutes() {
        let load = chip_load_of(1800.0, &tool(Some(2), Some(18000.0)));
        assert_eq!(load, ChipLoad::Mm(0.05));
        assert_eq!(load.text(), "0.050 mm/tooth");
        assert_eq!(load.text_in(chipload_job::Units::Inch), "0.0020 in/tooth");
        assert_eq!(
            chip_load_of(1800.0, &tool(None, Some(18000.0))),
            ChipLoad::NoFlutes
        );
        assert_eq!(
            chip_load_of(1800.0, &tool(Some(0), Some(18000.0))),
            ChipLoad::NoFlutes
        );
        assert_eq!(chip_load_of(1800.0, &tool(Some(2), None)), ChipLoad::NoRpm);
        assert_eq!(ChipLoad::NoRpm.mm(), None);
    }
}
