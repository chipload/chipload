// SPDX-License-Identifier: GPL-3.0-or-later
//! Job model: a declarative description of how to machine a drawing.
//!
//! Job file models (schema v3). All lengths are mm, all feeds
//! mm/min, `depth` is measured down from the stock top. Unknown fields are rejected, like
//! Pydantic's `extra="forbid"`; value constraints are checked by [`Job::validate`].

use serde::{Deserialize, Serialize};

/// The schema version this code reads and writes.
pub const CURRENT_VERSION: i64 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    pub source: Source,
    pub stock: Stock,
    pub machine: Machine,
    pub tools: Vec<Tool>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Mm,
    Inch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub file: String,
    #[serde(default)]
    pub sha256: Option<String>,
    pub units: Units,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockOrigin {
    #[default]
    BottomLeft,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ZZero {
    #[default]
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stock {
    pub x: f64,
    pub y: f64,
    pub thickness: f64,
    #[serde(default)]
    pub origin: StockOrigin,
    #[serde(default)]
    pub z_zero: ZZero,
}

impl Stock {
    /// (min, max) corners in work coordinates; work zero is the DXF origin.
    pub fn xy_bounds(&self) -> ((f64, f64), (f64, f64)) {
        match self.origin {
            StockOrigin::BottomLeft => ((0.0, 0.0), (self.x, self.y)),
            StockOrigin::Center => ((-self.x / 2.0, -self.y / 2.0), (self.x / 2.0, self.y / 2.0)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Travel {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Spindle {
    /// M3
    Cw,
    /// M4
    Ccw,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub post: String,
    pub safe_z: f64,
    pub spindle: Spindle,
    #[serde(default)]
    pub max_feed: Option<f64>,
    #[serde(default)]
    pub travel: Option<Travel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Endmill,
    Drill,
    Vbit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub id: i64,
    #[serde(default, rename = "type")]
    pub kind: ToolType,
    pub diameter: f64,
    #[serde(default)]
    pub flutes: Option<i64>,
    /// None = unknown. Pockets plunge straight down, so they need a centre-cutting endmill.
    #[serde(default)]
    pub center_cutting: Option<bool>,
    #[serde(default)]
    pub rpm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub handles: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tabs {
    pub count: i64,
    /// Bridge left on the part, mm (the tool lifts for width + tool diameter).
    pub width: f64,
    /// Bridge thickness up from the bottom of the cut, mm.
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Inside,
    Outside,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Climb,
    Conventional,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Operation {
    Profile(ProfileOp),
    Pocket(PocketOp),
    Drill(DrillOp),
}

impl Operation {
    pub fn tool(&self) -> i64 {
        match self {
            Operation::Profile(op) => op.tool,
            Operation::Pocket(op) => op.tool,
            Operation::Drill(op) => op.tool,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileOp {
    #[serde(default)]
    pub name: String,
    pub select: Selector,
    pub side: Side,
    pub direction: Direction,
    pub tool: i64,
    pub depth: f64,
    pub step_down: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    #[serde(default)]
    pub tabs: Option<Tabs>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PocketOp {
    #[serde(default)]
    pub name: String,
    pub select: Selector,
    pub tool: i64,
    pub depth: f64,
    pub step_down: f64,
    /// Fraction of the tool diameter, 0 < stepover <= 1.
    pub stepover: f64,
    pub direction: Direction,
    pub feed: f64,
    pub plunge_feed: f64,
    /// Stay at depth between rings where that is safe.
    #[serde(default)]
    pub keep_tool_down: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrillOp {
    #[serde(default)]
    pub name: String,
    pub select: Selector,
    pub tool: i64,
    #[serde(default)]
    pub peck: Option<f64>,
}

impl Job {
    /// Value constraints the serde types can't express.
    /// Returns every violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut positive = |path: String, value: f64| {
            if value <= 0.0 || value.is_nan() {
                problems.push(format!("{path}: must be greater than 0, got {value}"));
            }
        };
        positive("stock.x".into(), self.stock.x);
        positive("stock.y".into(), self.stock.y);
        positive("stock.thickness".into(), self.stock.thickness);
        if let Some(t) = &self.machine.travel {
            positive("machine.travel.x".into(), t.x);
            positive("machine.travel.y".into(), t.y);
            positive("machine.travel.z".into(), t.z);
        }
        for (i, tool) in self.tools.iter().enumerate() {
            positive(format!("tools.{i}.diameter"), tool.diameter);
        }
        for (i, op) in self.operations.iter().enumerate() {
            let at = |field: &str| format!("operations.{i}.{field}");
            match op {
                Operation::Profile(p) => {
                    positive(at("depth"), p.depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    if let Some(tabs) = &p.tabs {
                        positive(at("tabs.width"), tabs.width);
                        positive(at("tabs.height"), tabs.height);
                    }
                }
                Operation::Pocket(p) => {
                    positive(at("depth"), p.depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    positive(at("stepover"), p.stepover);
                }
                Operation::Drill(_) => {}
            }
        }
        for (i, op) in self.operations.iter().enumerate() {
            let selector = match op {
                Operation::Profile(p) => &p.select,
                Operation::Pocket(p) => &p.select,
                Operation::Drill(p) => &p.select,
            };
            let has_handles = selector.handles.as_ref().is_some_and(|h| !h.is_empty());
            if selector.layer.is_none() && !has_handles {
                problems.push(format!(
                    "operations.{i}.select: selector needs a layer or at least one handle"
                ));
            }
            if let Operation::Pocket(p) = op
                && p.stepover > 1.0
            {
                problems.push(format!(
                    "operations.{i}.stepover: must be at most 1 (a fraction of the tool \
                     diameter), got {}",
                    p.stepover
                ));
            }
            if let Operation::Profile(ProfileOp {
                tabs: Some(tabs), ..
            }) = op
                && tabs.count < 1
            {
                problems.push(format!("operations.{i}.tabs.count: must be at least 1"));
            }
        }
        problems
    }
}
