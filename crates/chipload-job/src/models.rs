// SPDX-License-Identifier: GPL-3.0-or-later
//! Job model: a declarative description of how to machine a drawing.
//!
//! Job file models (schema v4: a project with its drawing embedded, docs/specs/
//! project-file.md). All lengths are mm, all feeds mm/min, angles radians, `depth` is
//! measured down from the stock top. Unknown fields are rejected; value constraints are
//! checked by [`Job::validate`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The schema version this code reads and writes.
pub const CURRENT_VERSION: i64 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    pub drawing: Drawing,
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

/// The project's drawing: shapes in millimetres, or (only for jobs from before v4) a
/// reference to a DXF that still has to be embedded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drawing {
    pub shapes: Vec<Shape>,
    /// The id the next new shape gets. Ids are never reused.
    pub next_shape_id: u64,
    /// Where the imported shapes came from (the last import or update).
    #[serde(default)]
    pub source: Option<DrawingSource>,
    /// Legacy: the DXF a pre-v4 job referred to. Such a project must be embedded first.
    #[serde(default)]
    pub external: Option<ExternalDrawing>,
}

/// A pre-v4 job's drawing reference (was the top-level `source`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalDrawing {
    pub file: String,
    #[serde(default)]
    pub sha256: Option<String>,
    pub units: Units,
}

/// One shape: today always a path of lines and arcs (one per imported DXF entity).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub id: u64,
    pub layer: String,
    /// Handle of the DXF entity it was imported from (the INSERT's for block content).
    #[serde(default)]
    pub dxf_handle: Option<String>,
    /// DXF type of that entity, e.g. "LWPOLYLINE".
    #[serde(default)]
    pub dxf_type: Option<String>,
    pub kind: ShapeKind,
    /// The last segment ends where the first starts.
    pub closed: bool,
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Path,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathSegment {
    Line(LineSegment),
    Arc(ArcSegment),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineSegment {
    pub start: [f64; 2],
    pub end: [f64; 2],
}

/// From `start_angle` through `sweep` radians (positive = counter-clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArcSegment {
    pub center: [f64; 2],
    pub radius: f64,
    pub start_angle: f64,
    pub sweep: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitsFrom {
    /// Declared by the DXF ($INSUNITS).
    Dxf,
    /// The DXF declared none; the user chose.
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Warning,
    Error,
}

/// An import-time issue that can't be recomputed from the shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
}

/// Provenance of the imported shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawingSource {
    pub file: String,
    pub sha256: String,
    pub units: Units,
    pub units_from: UnitsFrom,
    #[serde(default)]
    pub imported: BTreeMap<String, usize>,
    #[serde(default)]
    pub flattened: BTreeMap<String, usize>,
    #[serde(default)]
    pub ignored: BTreeMap<String, usize>,
    #[serde(default)]
    pub issues: Vec<SourceIssue>,
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
    /// Shape ids of the project's drawing.
    #[serde(default)]
    pub shapes: Option<Vec<u64>>,
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

fn enabled_default() -> bool {
    true
}

impl Operation {
    pub fn name(&self) -> &str {
        match self {
            Operation::Profile(op) => &op.name,
            Operation::Pocket(op) => &op.name,
            Operation::Drill(op) => &op.name,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Operation::Profile(op) => op.enabled,
            Operation::Pocket(op) => op.enabled,
            Operation::Drill(op) => op.enabled,
        }
    }

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
    /// Disabled operations stay in the file and are skipped when generating.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
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
    /// Disabled operations stay in the file and are skipped when generating.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
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
    /// Disabled operations stay in the file and are skipped when generating.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    pub select: Selector,
    pub tool: i64,
    #[serde(default)]
    pub peck: Option<f64>,
}

impl Drawing {
    /// Structural rules of the drawing, as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.external.is_some() && !self.shapes.is_empty() {
            problems.push("drawing: an external drawing can't also have shapes".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for (i, shape) in self.shapes.iter().enumerate() {
            let at = format!("drawing.shapes.{i}");
            if shape.id == 0 || shape.id >= self.next_shape_id {
                problems.push(format!(
                    "{at}.id: must be between 1 and next_shape_id - 1 ({}), got {}",
                    self.next_shape_id.saturating_sub(1),
                    shape.id
                ));
            }
            if !seen.insert(shape.id) {
                problems.push(format!("{at}.id: {} is used twice", shape.id));
            }
            if shape.segments.is_empty() {
                problems.push(format!("{at}.segments: a shape needs at least one segment"));
            }
            for (j, segment) in shape.segments.iter().enumerate() {
                let finite = match segment {
                    PathSegment::Line(l) => l.start.iter().chain(&l.end).all(|v| v.is_finite()),
                    PathSegment::Arc(a) => {
                        a.center.iter().all(|v| v.is_finite())
                            && a.start_angle.is_finite()
                            && a.sweep.is_finite()
                            && a.radius.is_finite()
                            && a.radius > 0.0
                    }
                };
                if !finite {
                    problems.push(format!(
                        "{at}.segments.{j}: coordinates must be finite and radii positive"
                    ));
                }
            }
        }
        problems
    }
}

impl Job {
    /// Value constraints the serde types can't express.
    /// Returns every violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = self.drawing.validate();
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
            let has_shapes = selector.shapes.as_ref().is_some_and(|s| !s.is_empty());
            if selector.layer.is_none() && !has_handles && !has_shapes {
                problems.push(format!(
                    "operations.{i}.select: selector needs a layer, handles or shapes"
                ));
            }
            let external = self.drawing.external.is_some();
            if has_handles && !external {
                problems.push(format!(
                    "operations.{i}.select: handle selectors only apply to a drawing that is not \
                     embedded yet; select shapes instead"
                ));
            }
            if has_shapes && external {
                problems.push(format!(
                    "operations.{i}.select: shape selectors need an embedded drawing"
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
