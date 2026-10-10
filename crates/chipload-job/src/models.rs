// SPDX-License-Identifier: GPL-3.0-or-later
//! Job model: a declarative description of how to machine a drawing.
//!
//! Job file models (schema v5: a project with its drawing embedded, docs/specs/
//! project-file.md, and shapes drawn in Chipload, docs/specs/drawing-tools.md). All lengths are mm, all feeds mm/min, angles radians, `depth` is
//! measured down from the stock top. Unknown fields are rejected; value constraints are
//! checked by [`Job::validate`].

use std::collections::BTreeMap;

use chipload_core::tolerances::CHAIN_TOL_MM;
use serde::{Deserialize, Serialize};

/// The schema version this code reads and writes.
pub const CURRENT_VERSION: i64 = 5;

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

/// One shape of the drawing: imported (a path of lines and arcs, one per DXF entity) or
/// drawn in Chipload (circle, rectangle, polyline; docs/specs/drawing-tools.md).
///
/// In the file it is one flat object: `id`, `layer`, `kind`, then the fields of that kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawShape", into = "RawShape")]
pub struct Shape {
    pub id: u64,
    pub layer: String,
    pub geometry: Geometry,
}

/// What a shape is. Drawn kinds store their defining values; their segments are derived.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// Imported: lines and arcs exactly as read from the DXF.
    Path {
        /// Handle of the DXF entity it was imported from (the INSERT's for block content).
        dxf_handle: Option<String>,
        /// DXF type of that entity, e.g. "LWPOLYLINE".
        dxf_type: Option<String>,
        /// The last segment ends where the first starts.
        closed: bool,
        segments: Vec<PathSegment>,
    },
    Circle {
        center: [f64; 2],
        radius: f64,
    },
    /// `length` along the local X axis, `width` along local Y, rotated by `angle` (radians,
    /// CCW) about `center`.
    Rectangle {
        center: [f64; 2],
        length: f64,
        width: f64,
        angle: f64,
    },
    /// Straight segments through `points`, and back to the first if `closed`.
    Polyline {
        closed: bool,
        points: Vec<[f64; 2]>,
    },
}

impl Shape {
    pub fn kind_name(&self) -> &'static str {
        match self.geometry {
            Geometry::Path { .. } => "path",
            Geometry::Circle { .. } => "circle",
            Geometry::Rectangle { .. } => "rectangle",
            Geometry::Polyline { .. } => "polyline",
        }
    }

    /// Drawn in Chipload (editable), as opposed to imported.
    pub fn is_drawn(&self) -> bool {
        !matches!(self.geometry, Geometry::Path { .. })
    }

    /// Whether the shape's segments form a loop (as stored for paths).
    pub fn closed(&self) -> bool {
        match &self.geometry {
            Geometry::Path { closed, .. } | Geometry::Polyline { closed, .. } => *closed,
            Geometry::Circle { .. } | Geometry::Rectangle { .. } => true,
        }
    }

    pub fn dxf_handle(&self) -> Option<&str> {
        match &self.geometry {
            Geometry::Path { dxf_handle, .. } => dxf_handle.as_deref(),
            _ => None,
        }
    }

    pub fn dxf_type(&self) -> Option<&str> {
        match &self.geometry {
            Geometry::Path { dxf_type, .. } => dxf_type.as_deref(),
            _ => None,
        }
    }

    /// The shape as lines and arcs (spec drawing-tools.md section 2). Exact and
    /// deterministic: nothing is rounded.
    pub fn segments(&self) -> Vec<PathSegment> {
        match &self.geometry {
            Geometry::Path { segments, .. } => segments.clone(),
            // As an imported DXF CIRCLE: one full arc starting at angle 0.
            Geometry::Circle { center, radius } => vec![PathSegment::Arc(ArcSegment {
                center: *center,
                radius: *radius,
                start_angle: 0.0,
                sweep: std::f64::consts::TAU,
            })],
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
            } => {
                let (sin, cos) = angle.sin_cos();
                let (hl, hw) = (length / 2.0, width / 2.0);
                // Counter-clockwise from the (-length/2, -width/2) corner.
                let corners: Vec<[f64; 2]> = [(-hl, -hw), (hl, -hw), (hl, hw), (-hl, hw)]
                    .iter()
                    .map(|&(x, y)| [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos])
                    .collect();
                lines_through(&corners, true)
            }
            Geometry::Polyline { closed, points } => lines_through(points, *closed),
        }
    }
}

fn lines_through(points: &[[f64; 2]], closed: bool) -> Vec<PathSegment> {
    let n = points.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    (0..count)
        .map(|i| {
            PathSegment::Line(LineSegment {
                start: points[i],
                end: points[(i + 1) % n],
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Path,
    Circle,
    Rectangle,
    Polyline,
}

/// A shape as it appears in the file: every field a kind may have. Fields of other kinds are
/// refused, never ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawShape {
    id: u64,
    layer: String,
    kind: ShapeKind,
    // path: dxf_handle and dxf_type are always written (null if unknown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf_handle: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf_type: Option<Option<String>>,
    // path, polyline
    #[serde(default, skip_serializing_if = "Option::is_none")]
    closed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    segments: Option<Vec<PathSegment>>,
    // circle, rectangle
    #[serde(default, skip_serializing_if = "Option::is_none")]
    center: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    angle: Option<f64>,
    // polyline
    #[serde(default, skip_serializing_if = "Option::is_none")]
    points: Option<Vec<[f64; 2]>>,
}

impl RawShape {
    fn empty(id: u64, layer: String, kind: ShapeKind) -> Self {
        RawShape {
            id,
            layer,
            kind,
            dxf_handle: None,
            dxf_type: None,
            closed: None,
            segments: None,
            center: None,
            radius: None,
            length: None,
            width: None,
            angle: None,
            points: None,
        }
    }

    /// Names of the fields that are set, for "not allowed for this kind" messages.
    fn present(&self) -> Vec<&'static str> {
        [
            ("dxf_handle", self.dxf_handle.is_some()),
            ("dxf_type", self.dxf_type.is_some()),
            ("closed", self.closed.is_some()),
            ("segments", self.segments.is_some()),
            ("center", self.center.is_some()),
            ("radius", self.radius.is_some()),
            ("length", self.length.is_some()),
            ("width", self.width.is_some()),
            ("angle", self.angle.is_some()),
            ("points", self.points.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, set)| set.then_some(name))
        .collect()
    }
}

impl From<Shape> for RawShape {
    fn from(shape: Shape) -> Self {
        let kind = match shape.geometry {
            Geometry::Path { .. } => ShapeKind::Path,
            Geometry::Circle { .. } => ShapeKind::Circle,
            Geometry::Rectangle { .. } => ShapeKind::Rectangle,
            Geometry::Polyline { .. } => ShapeKind::Polyline,
        };
        let mut raw = RawShape::empty(shape.id, shape.layer, kind);
        match shape.geometry {
            Geometry::Path {
                dxf_handle,
                dxf_type,
                closed,
                segments,
            } => {
                raw.dxf_handle = Some(dxf_handle);
                raw.dxf_type = Some(dxf_type);
                raw.closed = Some(closed);
                raw.segments = Some(segments);
            }
            Geometry::Circle { center, radius } => {
                raw.center = Some(center);
                raw.radius = Some(radius);
            }
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
            } => {
                raw.center = Some(center);
                raw.length = Some(length);
                raw.width = Some(width);
                raw.angle = Some(angle);
            }
            Geometry::Polyline { closed, points } => {
                raw.closed = Some(closed);
                raw.points = Some(points);
            }
        }
        raw
    }
}

impl TryFrom<RawShape> for Shape {
    type Error = String;

    fn try_from(raw: RawShape) -> Result<Self, String> {
        let kind = raw.kind;
        let allowed: &[&str] = match kind {
            ShapeKind::Path => &["dxf_handle", "dxf_type", "closed", "segments"],
            ShapeKind::Circle => &["center", "radius"],
            ShapeKind::Rectangle => &["center", "length", "width", "angle"],
            ShapeKind::Polyline => &["closed", "points"],
        };
        let name = RawShape::from_kind_name(kind);
        if let Some(extra) = raw.present().into_iter().find(|f| !allowed.contains(f)) {
            return Err(format!(
                "shape {}: field `{extra}` is not allowed for a {name}",
                raw.id
            ));
        }
        let missing = |field: &str| format!("shape {}: a {name} needs `{field}`", raw.id);
        let geometry = match kind {
            ShapeKind::Path => Geometry::Path {
                dxf_handle: raw.dxf_handle.flatten(),
                dxf_type: raw.dxf_type.flatten(),
                closed: raw.closed.ok_or_else(|| missing("closed"))?,
                segments: raw.segments.ok_or_else(|| missing("segments"))?,
            },
            ShapeKind::Circle => Geometry::Circle {
                center: raw.center.ok_or_else(|| missing("center"))?,
                radius: raw.radius.ok_or_else(|| missing("radius"))?,
            },
            ShapeKind::Rectangle => Geometry::Rectangle {
                center: raw.center.ok_or_else(|| missing("center"))?,
                length: raw.length.ok_or_else(|| missing("length"))?,
                width: raw.width.ok_or_else(|| missing("width"))?,
                angle: raw.angle.ok_or_else(|| missing("angle"))?,
            },
            ShapeKind::Polyline => Geometry::Polyline {
                closed: raw.closed.ok_or_else(|| missing("closed"))?,
                points: raw.points.ok_or_else(|| missing("points"))?,
            },
        };
        Ok(Shape {
            id: raw.id,
            layer: raw.layer,
            geometry,
        })
    }
}

impl RawShape {
    fn from_kind_name(kind: ShapeKind) -> &'static str {
        match kind {
            ShapeKind::Path => "path",
            ShapeKind::Circle => "circle",
            ShapeKind::Rectangle => "rectangle",
            ShapeKind::Polyline => "polyline",
        }
    }
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
            problems.extend(
                shape_problems(&shape.geometry)
                    .into_iter()
                    .map(|p| format!("{at} (S{}).{p}", shape.id)),
            );
        }
        problems
    }
}

/// Value rules of one shape, as "field: message" (spec drawing-tools.md section 1).
fn shape_problems(geometry: &Geometry) -> Vec<String> {
    let mut problems = Vec::new();
    let finite = |p: &[f64; 2]| p.iter().all(|v| v.is_finite());
    let mut positive = |field: &str, value: f64| {
        if !(value.is_finite() && value > 0.0) {
            problems.push(format!("{field}: must be a positive number, got {value}"));
        }
    };
    match geometry {
        Geometry::Path { segments, .. } => {
            if segments.is_empty() {
                problems.push("segments: a shape needs at least one segment".into());
            }
            for (j, segment) in segments.iter().enumerate() {
                let ok = match segment {
                    PathSegment::Line(l) => finite(&l.start) && finite(&l.end),
                    PathSegment::Arc(a) => {
                        finite(&a.center)
                            && a.start_angle.is_finite()
                            && a.sweep.is_finite()
                            && a.radius.is_finite()
                            && a.radius > 0.0
                    }
                };
                if !ok {
                    problems.push(format!(
                        "segments.{j}: coordinates must be finite and radii positive"
                    ));
                }
            }
        }
        Geometry::Circle { center, radius } => {
            positive("radius", *radius);
            if !finite(center) {
                problems.push("center: must be finite".into());
            }
        }
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
        } => {
            positive("length", *length);
            positive("width", *width);
            if !finite(center) {
                problems.push("center: must be finite".into());
            }
            if !angle.is_finite() {
                problems.push(format!("angle: must be finite, got {angle}"));
            }
        }
        Geometry::Polyline { closed, points } => {
            let least = if *closed { 3 } else { 2 };
            if points.len() < least {
                problems.push(format!(
                    "points: a{} polyline needs at least {least} points, got {}",
                    if *closed { " closed" } else { "n open" },
                    points.len()
                ));
            }
            if !points.iter().all(finite) {
                problems.push("points: must be finite".into());
            }
            let n = points.len();
            let pairs = if *closed { n } else { n.saturating_sub(1) };
            for i in 0..pairs {
                let (a, b) = (points[i], points[(i + 1) % n]);
                if (a[0] - b[0]).hypot(a[1] - b[1]) <= CHAIN_TOL_MM {
                    problems.push(format!(
                        "points.{}: is on top of the previous point (closer than {} mm)",
                        (i + 1) % n,
                        CHAIN_TOL_MM
                    ));
                }
            }
        }
    }
    problems
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

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(geometry: Geometry) -> Shape {
        Shape {
            id: 1,
            layer: "0".into(),
            geometry,
        }
    }

    fn line_starts(shape: &Shape) -> Vec<[f64; 2]> {
        shape
            .segments()
            .iter()
            .map(|s| match s {
                PathSegment::Line(l) => l.start,
                PathSegment::Arc(_) => panic!("expected lines"),
            })
            .collect()
    }

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12
    }

    #[test]
    fn a_circle_is_one_full_arc_from_angle_zero() {
        let c = shape(Geometry::Circle {
            center: [5.0, 6.0],
            radius: 2.0,
        });
        assert_eq!(
            c.segments(),
            [PathSegment::Arc(ArcSegment {
                center: [5.0, 6.0],
                radius: 2.0,
                start_angle: 0.0,
                sweep: std::f64::consts::TAU,
            })]
        );
        assert!(c.closed() && c.is_drawn() && c.dxf_handle().is_none());
    }

    #[test]
    fn rectangle_corners_run_counter_clockwise_and_rotate_about_the_centre() {
        let rect = |angle: f64| {
            shape(Geometry::Rectangle {
                center: [10.0, 20.0],
                length: 8.0,
                width: 4.0,
                angle,
            })
        };
        let flat = line_starts(&rect(0.0));
        assert_eq!(flat, [[6.0, 18.0], [14.0, 18.0], [14.0, 22.0], [6.0, 22.0]]);
        // 90°: length now runs along Y.
        let up = line_starts(&rect(std::f64::consts::FRAC_PI_2));
        let expected = [[12.0, 16.0], [12.0, 24.0], [8.0, 24.0], [8.0, 16.0]];
        assert!(
            up.iter().zip(&expected).all(|(a, b)| close(*a, *b)),
            "{up:?}"
        );
        // 30°: the first corner is (-4, -2) rotated by 30° about the centre.
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let first = line_starts(&rect(30f64.to_radians()))[0];
        assert!(close(
            first,
            [10.0 - 4.0 * cos + 2.0 * sin, 20.0 - 4.0 * sin - 2.0 * cos]
        ));
        // Closed: the last line ends at the first corner.
        let segments = rect(0.3).segments();
        let (PathSegment::Line(first), PathSegment::Line(last)) = (&segments[0], &segments[3])
        else {
            panic!("lines");
        };
        assert_eq!(last.end, first.start);
    }

    #[test]
    fn polylines_close_only_when_asked() {
        let points = vec![[0.0, 0.0], [4.0, 0.0], [2.0, 3.0]];
        let open = shape(Geometry::Polyline {
            closed: false,
            points: points.clone(),
        });
        let closed = shape(Geometry::Polyline {
            closed: true,
            points,
        });
        assert_eq!(open.segments().len(), 2);
        assert_eq!(closed.segments().len(), 3);
        assert!(!open.closed() && closed.closed());
    }
}
