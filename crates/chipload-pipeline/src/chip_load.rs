// SPDX-License-Identifier: GPL-3.0-or-later
//! Chip load: how thick a chip each cutting edge takes, feed / (rpm × flutes), mm per tooth
//! (docs/specs/small-items.md §5). Shown, not judged: a good value depends on the material,
//! the tool and the machine (decision 6, option A).

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
pub const CHIP_LOAD_HINT: &str = "Chip load: how thick a chip each cutting edge takes, \
    feed / (spindle speed × flutes). Too thin and the tool rubs and heats up; too thick and \
    it may break. It depends on the material, the tool and the machine; as a rough guide, \
    wood and plastics with 3–12 mm tools often take 0.05–0.25 mm/tooth, aluminium \
    0.02–0.08. Check the tool maker's figures.";

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
    }
}

/// The chip load of an operation at its cutting feed (the plunge feed has none: going down
/// the tool cuts with its tip).
pub fn chip_load(job: &Job, op: &Operation) -> ChipLoad {
    match job.tools.iter().find(|t| t.id == op.tool()) {
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
        match self {
            ChipLoad::Mm(mm) => format!("{mm:.3} mm/tooth"),
            ChipLoad::NoFlutes => "set the tool's flutes to see the chip load".into(),
            ChipLoad::NoRpm => "set the tool's spindle speed to see the chip load".into(),
            ChipLoad::NoTool => "no tool".into(),
        }
    }
}

/// One line per enabled operation, for `chipload plan`.
pub fn chip_load_lines(job: &Job) -> Vec<String> {
    let mut lines = vec!["Chip load (at the cutting feed):".to_string()];
    for (i, op) in job
        .operations
        .iter()
        .enumerate()
        .filter(|(_, op)| op.enabled())
    {
        let label = match op.name() {
            "" => format!("Operation {}", i + 1),
            name => format!("Operation {} ({name})", i + 1),
        };
        lines.push(format!("  {label}: {}", chip_load(job, op).text()));
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
