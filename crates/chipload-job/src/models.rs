// SPDX-License-Identifier: GPL-3.0-or-later
//! Job model: a declarative description of how to machine a drawing.
//!
//! Job file models (schema v9: a project with its drawing embedded, docs/specs/
//! project-file.md, shapes drawn in Chipload, docs/specs/drawing-tools.md, and the imported
//! drawing's placement, docs/specs/drawing-placement.md and drawing-rotation.md). All lengths are mm, all feeds
//! mm/min, angles radians, `depth` is
//! measured down from the stock top. Unknown fields are rejected; value constraints are
//! checked by [`Job::validate`].

use std::collections::BTreeMap;

use chipload_core::primitives::Arc;
use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM, DEPTH_TOL_MM};
use serde::{Deserialize, Serialize};

/// The schema version this code reads and writes.
pub const CURRENT_VERSION: i64 = 19;

/// The oldest schema version that still loads (decision 2026-10-10: versions from before
/// projects, 1-3, were never released and are no longer read).
pub const OLDEST_VERSION: i64 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// The version of the file format. Chipload upgrades older projects when it opens them,
    /// and refuses newer ones.
    pub schema_version: i64,
    /// The Chipload version that wrote the file, for reference.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The drawing: the shapes Chipload cuts, imported from a DXF or drawn in Chipload.
    pub drawing: Drawing,
    /// The material on the machine.
    pub stock: Stock,
    /// The machine: post-processor, safe height, limits and zero.
    pub machine: Machine,
    /// The tools the operations cut with.
    pub tools: Vec<Tool>,
    /// The operations, cut in this order.
    pub operations: Vec<Operation>,
    /// Cut the part several times in a grid. Null: once.
    // Spec: docs/specs/copies-grid.md.
    #[serde(default)]
    pub copies: Option<Copies>,
}

/// Most columns or rows of copies, and most copies in all.
pub const MAX_COPIES_PER_SIDE: u32 = 100;
pub const MAX_COPIES: u32 = 1000;

/// A grid of copies of the whole part: copy (column c, row r) is the drawing moved by
/// (c·dx, r·dy) mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Copies {
    /// Copies along X, at least 1.
    pub columns: u32,
    /// Copies along Y, at least 1.
    pub rows: u32,
    /// Distance from copy to copy along X, mm; negative grows to the left.
    pub dx: f64,
    /// Distance from copy to copy along Y, mm; negative grows downwards.
    pub dy: f64,
    /// `by_operation`: each operation on every copy, then the next; `by_part`: every
    /// operation on one copy, then the next copy.
    #[serde(default)]
    pub order: CopyOrder,
}

/// In which order the copies are cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CopyOrder {
    /// Each operation on every copy, then the next operation.
    #[default]
    ByOperation,
    /// Every operation on one copy, then the next copy.
    ByPart,
}

impl Copies {
    pub fn count(&self) -> u32 {
        self.columns.saturating_mul(self.rows)
    }

    /// Every copy as (number, shift), in cut order: row by row from copy 1, alternating
    /// direction (1, 2, 3, then 6, 5, 4). Copies are numbered row by row from 1, the
    /// drawing as drawn.
    pub fn in_cut_order(&self) -> Vec<(u32, (f64, f64))> {
        let mut out = Vec::new();
        for row in 0..self.rows {
            let mut columns: Vec<u32> = (0..self.columns).collect();
            if row % 2 == 1 {
                columns.reverse();
            }
            for column in columns {
                let number = row * self.columns + column + 1;
                out.push((
                    number,
                    (f64::from(column) * self.dx, f64::from(row) * self.dy),
                ));
            }
        }
        out
    }

    /// What is wrong with the grid, as "copies.field: message".
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (name, n) in [("columns", self.columns), ("rows", self.rows)] {
            if !(1..=MAX_COPIES_PER_SIDE).contains(&n) {
                problems.push(format!(
                    "copies.{name}: must be from 1 to {MAX_COPIES_PER_SIDE}, got {n}"
                ));
            }
        }
        if self.count() > MAX_COPIES {
            problems.push(format!(
                "copies: at most {MAX_COPIES} copies in all, got {}",
                self.count()
            ));
        }
        for (name, pitch, n) in [("dx", self.dx, self.columns), ("dy", self.dy, self.rows)] {
            if !pitch.is_finite() {
                problems.push(format!("copies.{name}: must be a number, got {pitch}"));
            } else if n > 1 && pitch == 0.0 {
                problems.push(format!(
                    "copies.{name}: must not be 0 with more than one copy that way"
                ));
            }
        }
        problems
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Mm,
    Inch,
}

/// The project's drawing: shapes in millimetres, or (only for jobs from before v4) a
/// reference to a DXF that still has to be embedded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Drawing {
    /// The shapes, imported and drawn.
    pub shapes: Vec<Shape>,
    /// The id the next new shape gets. Ids are never reused.
    pub next_shape_id: u64,
    /// The DXF the imported shapes came from, at the last import or update. Null: no DXF
    /// yet.
    #[serde(default)]
    pub source: Option<DrawingSource>,
    /// Pictures traced into shapes, for V-carving.
    // Spec: docs/specs/image-trace.md §4.
    #[serde(default)]
    pub images: Vec<Image>,
}

/// A picture traced into shapes: the picture itself, where it goes, and how it becomes black
/// and white. Its traced shapes are `path` shapes on its layer that name it in `image`.
// Spec: docs/specs/image-trace.md §2, §4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Image {
    /// The image's id, which its traced shapes name.
    pub id: u64,
    /// The picture's file name.
    pub file: String,
    /// The picture's SHA-256.
    pub sha256: String,
    /// The picture file itself (PNG, JPEG or BMP), base64-encoded.
    pub data: String,
    /// The layer its traced shapes go on.
    pub layer: String,
    /// X of the picture's lower-left corner on the stock, mm.
    pub x: f64,
    /// Y of the picture's lower-left corner on the stock, mm.
    pub y: f64,
    /// The picture's width on the stock, mm; its height follows from the picture.
    pub width: f64,
    /// Grey values below this (0–255) are black: 1 to 254, or "auto" to pick it from the
    /// picture (Otsu's method).
    pub threshold: ImageThreshold,
    /// Carve the light parts instead of the dark ones.
    pub invert: bool,
    /// Blur before the threshold, in pixels, 0 to 10; 0: none.
    pub blur: f64,
    /// Black or white specks smaller than this are dropped, mm², 0 to 100.
    pub min_area: f64,
    /// How far a traced edge may be simplified away from the picture's edge, mm, 0.01 to 1.
    pub smoothing: f64,
}

/// Where grey turns black: "auto" or a grey value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ImageThreshold {
    /// Picked from the picture.
    Auto(AutoThreshold),
    /// Grey values below this are black, 1 to 254.
    Level(u8),
}

/// The word "auto".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum AutoThreshold {
    Auto,
}

impl ImageThreshold {
    pub const AUTO: ImageThreshold = ImageThreshold::Auto(AutoThreshold::Auto);
}

/// The ranges of an image's settings (docs/specs/image-trace.md §2).
pub const IMAGE_MAX_BLUR_PX: f64 = 10.0;
pub const IMAGE_MAX_MIN_AREA_MM2: f64 = 100.0;
pub const IMAGE_MIN_SMOOTHING_MM: f64 = 0.01;
pub const IMAGE_MAX_SMOOTHING_MM: f64 = 1.0;

impl Image {
    /// What is wrong with the image's values, as "drawing.images.N.field: message".
    pub fn problems(&self, index: usize) -> Vec<String> {
        let at = format!("drawing.images.{index}");
        let mut problems = Vec::new();
        let mut range = |field: &str, value: f64, low: f64, high: f64, unit: &str| {
            if !(value.is_finite() && (low..=high).contains(&value)) {
                problems.push(format!(
                    "{at}.{field}: must be {low} to {high}{unit}, got {value}"
                ));
            }
        };
        range("blur", self.blur, 0.0, IMAGE_MAX_BLUR_PX, " px");
        range(
            "min_area",
            self.min_area,
            0.0,
            IMAGE_MAX_MIN_AREA_MM2,
            " mm²",
        );
        range(
            "smoothing",
            self.smoothing,
            IMAGE_MIN_SMOOTHING_MM,
            IMAGE_MAX_SMOOTHING_MM,
            " mm",
        );
        if let ImageThreshold::Level(t) = self.threshold
            && !(1..=254).contains(&t)
        {
            problems.push(format!(
                "{at}.threshold: must be 1 to 254 or \"auto\", got {t}"
            ));
        }
        if !(self.width.is_finite() && self.width > 0.0) {
            problems.push(format!(
                "{at}.width: must be greater than 0, got {}",
                self.width
            ));
        }
        if !(self.x.is_finite() && self.y.is_finite()) {
            problems.push(format!(
                "{at}: x and y must be numbers, got ({}, {})",
                self.x, self.y
            ));
        }
        if self.layer.trim().is_empty() {
            problems.push(format!("{at}.layer: must not be empty"));
        }
        if self.data.is_empty() {
            problems.push(format!("{at}.data: the picture is missing"));
        }
        problems
    }
}

/// One shape of the drawing: imported from a DXF (`path`: lines and arcs, one shape per DXF
/// entity), or drawn in Chipload (`circle`, `rectangle`, `polyline`). One flat object:
/// `id`, `layer`, `kind`, then the fields of that kind.
// Spec: docs/specs/drawing-tools.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
        /// The image it was traced from (docs/specs/image-trace.md §4); None: from a DXF.
        image: Option<u64>,
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
    /// Segments through `points`, and back to the first if `closed`. Each segment bends by
    /// its bulge, as in DXF: tan(sweep / 4), 0 straight, ±1 a half circle (positive
    /// counter-clockwise); one per segment (docs/specs/drawing-follow-ups.md section 3).
    Polyline {
        closed: bool,
        points: Vec<[f64; 2]>,
        bulges: Vec<f64>,
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

    /// Imported from the DXF: a path not traced from an image. The DXF's placement moves
    /// these.
    pub fn from_dxf(&self) -> bool {
        matches!(self.geometry, Geometry::Path { image: None, .. })
    }

    /// The image this shape was traced from.
    pub fn image(&self) -> Option<u64> {
        match &self.geometry {
            Geometry::Path { image, .. } => *image,
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
            Geometry::Polyline {
                closed,
                points,
                bulges,
            } => polyline_segments(points, bulges, *closed),
        }
    }
}

impl Geometry {
    /// A polyline of straight segments (every bulge 0).
    pub fn straight_polyline(closed: bool, points: Vec<[f64; 2]>) -> Geometry {
        let bulges = vec![0.0; polyline_segment_count(points.len(), closed)];
        Geometry::Polyline {
            closed,
            points,
            bulges,
        }
    }
}

/// How many segments a polyline of `points` points has.
pub fn polyline_segment_count(points: usize, closed: bool) -> usize {
    if closed {
        points
    } else {
        points.saturating_sub(1)
    }
}

/// A polyline's segments: straight where the bulge is 0, otherwise the arc a DXF bulge of
/// that value describes (`Arc::from_bulge`). A missing bulge counts as straight.
fn polyline_segments(points: &[[f64; 2]], bulges: &[f64], closed: bool) -> Vec<PathSegment> {
    let n = points.len();
    (0..polyline_segment_count(n, closed))
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let bulge = bulges.get(i).copied().unwrap_or(0.0);
            match Arc::from_bulge((a[0], a[1]), (b[0], b[1]), bulge) {
                Some(arc) => PathSegment::Arc(ArcSegment {
                    center: [arc.center.0, arc.center.1],
                    radius: arc.radius,
                    start_angle: arc.start_angle,
                    sweep: arc.sweep,
                }),
                None => PathSegment::Line(LineSegment { start: a, end: b }),
            }
        })
        .collect()
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct RawShape {
    /// The shape's id, which operations select it by.
    id: u64,
    /// Its layer.
    layer: String,
    /// `path` (imported from a DXF), or a shape drawn in Chipload: `circle`, `rectangle`,
    /// `polyline`.
    kind: ShapeKind,
    /// path: the handle of the DXF entity it came from, which keeps its id when the DXF is
    /// updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf_handle: Option<Option<String>>,
    /// path: the DXF entity's type, e.g. LWPOLYLINE.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf_type: Option<Option<String>>,
    /// path: the image it was traced from; absent for shapes from a DXF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<u64>,
    /// path, polyline: whether it ends where it starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    closed: Option<bool>,
    /// path: its lines and arcs, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    segments: Option<Vec<PathSegment>>,
    /// circle, rectangle: the centre, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    center: Option<[f64; 2]>,
    /// circle: the radius, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    radius: Option<f64>,
    /// rectangle: its size along its own X axis, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    length: Option<f64>,
    /// rectangle: its size along its own Y axis, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    /// rectangle: its rotation about the centre, radians, counter-clockwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    angle: Option<f64>,
    /// polyline: its points, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    points: Option<Vec<[f64; 2]>>,
    /// polyline: one per segment, as in DXF: tan(sweep / 4); 0 straight, 1 a
    /// counter-clockwise half circle, -1 clockwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bulges: Option<Vec<f64>>,
}

impl RawShape {
    fn empty(id: u64, layer: String, kind: ShapeKind) -> Self {
        RawShape {
            id,
            layer,
            kind,
            dxf_handle: None,
            dxf_type: None,
            image: None,
            closed: None,
            segments: None,
            center: None,
            radius: None,
            length: None,
            width: None,
            angle: None,
            points: None,
            bulges: None,
        }
    }

    /// Names of the fields that are set, for "not allowed for this kind" messages.
    fn present(&self) -> Vec<&'static str> {
        [
            ("dxf_handle", self.dxf_handle.is_some()),
            ("dxf_type", self.dxf_type.is_some()),
            ("image", self.image.is_some()),
            ("closed", self.closed.is_some()),
            ("segments", self.segments.is_some()),
            ("center", self.center.is_some()),
            ("radius", self.radius.is_some()),
            ("length", self.length.is_some()),
            ("width", self.width.is_some()),
            ("angle", self.angle.is_some()),
            ("points", self.points.is_some()),
            ("bulges", self.bulges.is_some()),
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
                image,
            } => {
                raw.dxf_handle = Some(dxf_handle);
                raw.dxf_type = Some(dxf_type);
                raw.image = image;
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
            Geometry::Polyline {
                closed,
                points,
                bulges,
            } => {
                raw.closed = Some(closed);
                raw.points = Some(points);
                raw.bulges = Some(bulges);
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
            ShapeKind::Path => &["dxf_handle", "dxf_type", "image", "closed", "segments"],
            ShapeKind::Circle => &["center", "radius"],
            ShapeKind::Rectangle => &["center", "length", "width", "angle"],
            ShapeKind::Polyline => &["closed", "points", "bulges"],
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
                image: raw.image,
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
            ShapeKind::Polyline => {
                let closed = raw.closed.ok_or_else(|| missing("closed"))?;
                let points = raw.points.ok_or_else(|| missing("points"))?;
                // Optional: without it every segment is straight (as before bulges existed).
                let count = polyline_segment_count(points.len(), closed);
                let bulges = raw.bulges.unwrap_or_else(|| vec![0.0; count]);
                Geometry::Polyline {
                    closed,
                    points,
                    bulges,
                }
            }
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PathSegment {
    Line(LineSegment),
    Arc(ArcSegment),
}

impl PathSegment {
    /// The segment moved by (dx, dy) mm.
    pub fn translated(&self, dx: f64, dy: f64) -> PathSegment {
        let moved = |p: [f64; 2]| [p[0] + dx, p[1] + dy];
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: moved(l.start),
                end: moved(l.end),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: moved(a.center),
                ..*a
            }),
        }
    }

    /// The segment scaled about the origin by `factor` (> 0): points and radius are
    /// multiplied, angles stay.
    pub fn scaled(&self, factor: f64) -> PathSegment {
        let times = |p: [f64; 2]| [p[0] * factor, p[1] * factor];
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: times(l.start),
                end: times(l.end),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: times(a.center),
                radius: a.radius * factor,
                ..*a
            }),
        }
    }

    /// The segment turned about the origin by `k` × 90° counter-clockwise. Points are
    /// exact; an arc's start angle is wrapped into [0, 2π).
    pub fn quarter_turned(&self, k: u8) -> PathSegment {
        if k.is_multiple_of(4) {
            return self.clone();
        }
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: quarter_turned(l.start, k),
                end: quarter_turned(l.end, k),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: quarter_turned(a.center, k),
                start_angle: (a.start_angle + f64::from(k % 4) * std::f64::consts::FRAC_PI_2)
                    .rem_euclid(std::f64::consts::TAU),
                ..*a
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LineSegment {
    /// Start point, mm.
    pub start: [f64; 2],
    /// End point, mm.
    pub end: [f64; 2],
}

/// From `start_angle` through `sweep` radians (positive = counter-clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ArcSegment {
    /// Centre, mm.
    pub center: [f64; 2],
    /// Radius, mm.
    pub radius: f64,
    /// Where the arc starts, radians from +X, counter-clockwise.
    pub start_angle: f64,
    /// How far it turns, radians: positive counter-clockwise.
    pub sweep: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum UnitsFrom {
    /// Declared by the DXF ($INSUNITS).
    Dxf,
    /// The DXF declared none; the user chose.
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Warning,
    Error,
}

/// An import-time issue that can't be recomputed from the shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SourceIssue {
    /// `warning` or `error`.
    pub severity: IssueSeverity,
    /// What kind of issue, e.g. `duplicate` or `open_contour` (see the import page of the
    /// guide).
    pub code: String,
    /// What it is, in words.
    pub message: String,
}

/// Provenance of the imported shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DrawingSource {
    /// The DXF's file name.
    pub file: String,
    /// The DXF's SHA-256, to tell whether it changed.
    pub sha256: String,
    /// The DXF's units: `mm` or `inch`. Inch drawings are converted to mm on import.
    pub units: Units,
    /// Where the units came from: the DXF itself, or the user's choice.
    pub units_from: UnitsFrom,
    /// How many entities of each type were imported.
    #[serde(default)]
    pub imported: BTreeMap<String, usize>,
    /// How many curves of each type (splines, ellipses) were turned into arcs.
    #[serde(default)]
    pub flattened: BTreeMap<String, usize>,
    /// How many entities of each type were left out (text, dimensions, …).
    #[serde(default)]
    pub ignored: BTreeMap<String, usize>,
    /// What the import found: the import report.
    #[serde(default)]
    pub issues: Vec<SourceIssue>,
    /// How the imported drawing was moved, turned and scaled on the stock.
    // Spec: docs/specs/drawing-placement.md.
    pub placement: Placement,
}

/// Where the imported drawing is put: a DXF point p goes to scale · turn(p) + (dx, dy),
/// turned and scaled about the DXF's origin.
// Spec: docs/specs/drawing-rotation.md; docs/specs/drawing-scale.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// Scaled about the DXF's origin by this factor, 0.001 to 1000; 1: as drawn.
    pub scale: f64,
    /// Turned counter-clockwise about the DXF's origin, in quarter turns (0–3), after the
    /// scale.
    pub quarter_turns: u8,
    /// Then moved along X, mm.
    pub dx: f64,
    /// Then moved along Y, mm.
    pub dy: f64,
}

impl Default for Placement {
    /// As the DXF has it.
    fn default() -> Self {
        Placement {
            scale: 1.0,
            quarter_turns: 0,
            dx: 0.0,
            dy: 0.0,
        }
    }
}

impl Placement {
    /// The smallest total scale of the imported drawing (docs/specs/drawing-scale.md §8.5).
    pub const MIN_SCALE: f64 = 0.001;
    /// The largest total scale.
    pub const MAX_SCALE: f64 = 1000.0;

    /// Whether `scale` is an allowed total scale.
    pub fn scale_in_range(scale: f64) -> bool {
        (Self::MIN_SCALE..=Self::MAX_SCALE).contains(&scale)
    }
}

/// The point turned about the origin by `k` × 90° counter-clockwise. Exact: no trigonometry.
pub fn quarter_turned(p: [f64; 2], k: u8) -> [f64; 2] {
    match k % 4 {
        0 => p,
        1 => [-p[1], p[0]],
        2 => [-p[0], -p[1]],
        _ => [p[1], -p[0]],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StockOrigin {
    #[default]
    BottomLeft,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ZZero {
    #[default]
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Stock {
    /// Size along X, mm.
    pub x: f64,
    /// Size along Y, mm.
    pub y: f64,
    /// Thickness, mm. Depths count down from the top.
    pub thickness: f64,
    /// Where the stock lies around the drawing's origin: its `bottom_left` corner or its
    /// `center` there.
    #[serde(default)]
    pub origin: StockOrigin,
    /// Where Z0 is: the stock `top` or `bottom`.
    #[serde(default)]
    pub z_zero: ZZero,
    /// How far a cut may go below the stock bottom, into the spoilboard, mm.
    pub max_overcut: f64,
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Travel {
    /// Travel along X, mm.
    pub x: f64,
    /// Travel along Y, mm.
    pub y: f64,
    /// Travel along Z, mm.
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Spindle {
    /// M3
    Cw,
    /// M4
    Ccw,
}

/// How a job with several tools is written.
// Spec: docs/specs/tool-per-operation.md §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ToolChange {
    /// One G-code file per tool group; the user changes the tool and sets Z zero between.
    #[default]
    Files,
    /// One file with `T<n> M6` at each change, for a sender that handles it.
    Program,
}

impl ToolChange {
    pub fn name(self) -> &'static str {
        match self {
            ToolChange::Files => "files",
            ToolChange::Program => "program",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Machine {
    /// The machine profile these values came from, for reference; nothing reads it.
    // Spec: docs/specs/tool-library-and-machines.md §4.
    #[serde(default)]
    pub name: Option<String>,
    /// The post-processor: `grbl`, `linuxcnc`, `mach3`, or the name of your own.
    pub post: String,
    /// The height for rapid moves sideways, mm above Z0. Above the stock top.
    pub safe_z: f64,
    /// The spindle's direction: `cw` (M3) or `ccw` (M4).
    pub spindle: Spindle,
    /// The machine's fastest feed, mm/min: higher feeds are refused, and the cycle time
    /// uses it for rapids. Null: unknown.
    #[serde(default)]
    pub max_feed: Option<f64>,
    /// The machine's travel, mm. When set, a program that moves farther is refused. Null:
    /// not checked.
    #[serde(default)]
    pub travel: Option<Travel>,
    /// The point of the drawing that becomes X0 Y0 in the G-code. Missing: the drawing's
    /// origin.
    // Spec: docs/specs/work-zero.md.
    #[serde(default)]
    pub work_zero: WorkZero,
    /// With several tools: `files` (one file per tool) or `program` (one file with tool
    /// changes).
    pub tool_change: ToolChange,
    /// Your own post-processor, copied into the project when chosen, so it gives the same
    /// G-code anywhere. Null: `post` is a built-in one.
    // Spec: docs/specs/user-posts.md §3.
    #[serde(default)]
    pub custom_post: Option<CustomPost>,
}

/// A user post-processor's config and template, exactly as read from the posts folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CustomPost {
    /// The post's settings file (TOML), as copied.
    pub config: String,
    /// The post's template, as copied.
    pub template: String,
}

/// Where the G-code's X0 Y0 is. The geometry never moves; only the output is translated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "at", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkZero {
    /// The drawing's own origin (the only choice before work_zero existed). An empty struct
    /// variant, not a unit one, so serde refuses stray fields here too.
    DrawingOrigin {},
    /// A corner or the centre of the stock, as placed by `stock.origin`.
    Stock {
        /// `center`, `bottom_left`, `bottom_right`, `top_left` or `top_right` of the stock.
        corner: StockCorner,
    },
    /// Any point, in drawing coordinates (mm).
    Point {
        /// X of the point, drawing coordinates, mm.
        x: f64,
        /// Y of the point, drawing coordinates, mm.
        y: f64,
    },
}

impl Default for WorkZero {
    fn default() -> Self {
        WorkZero::DrawingOrigin {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StockCorner {
    Center,
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

impl StockCorner {
    pub fn name(self) -> &'static str {
        match self {
            StockCorner::Center => "center",
            StockCorner::BottomLeft => "bottom left corner",
            StockCorner::BottomRight => "bottom right corner",
            StockCorner::TopLeft => "top left corner",
            StockCorner::TopRight => "top right corner",
        }
    }
}

impl Job {
    /// The work zero in drawing coordinates: subtract it from every X, Y of the toolpath to
    /// get program coordinates.
    pub fn work_zero_xy(&self) -> (f64, f64) {
        let ((x0, y0), (x1, y1)) = self.stock.xy_bounds();
        match self.machine.work_zero {
            WorkZero::DrawingOrigin {} => (0.0, 0.0),
            WorkZero::Point { x, y } => (x, y),
            WorkZero::Stock { corner } => match corner {
                StockCorner::Center => ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
                StockCorner::BottomLeft => (x0, y0),
                StockCorner::BottomRight => (x1, y0),
                StockCorner::TopLeft => (x0, y1),
                StockCorner::TopRight => (x1, y1),
            },
        }
    }

    /// "stock center = drawing X150.000 Y110.000, Z0 = stock top", for the G-code header,
    /// the CLI and the GUI. No parentheses or semicolons, so it fits in a G-code comment.
    pub fn work_zero_text(&self) -> String {
        let (x, y) = self.work_zero_xy();
        let at = match self.machine.work_zero {
            WorkZero::DrawingOrigin {} => "drawing origin".to_string(),
            WorkZero::Stock { corner } => format!("stock {}", corner.name()),
            WorkZero::Point { .. } => "point".to_string(),
        };
        let z = match self.stock.z_zero {
            ZZero::Top => "stock top",
            ZZero::Bottom => "stock bottom",
        };
        format!("{at} = drawing X{x:.3} Y{y:.3}, Z0 = {z}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Endmill,
    Drill,
    Vbit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Tool {
    /// The tool's number: what operations name it by, and its T number in the G-code.
    pub id: i64,
    /// The library tool it came from, for reference.
    // Spec: docs/specs/tool-library-and-machines.md §4.
    #[serde(default)]
    pub name: Option<String>,
    /// `endmill`, `drill` or `vbit`.
    #[serde(default, rename = "type")]
    pub kind: ToolType,
    /// Diameter, mm. For a V-bit, its widest cutting diameter.
    pub diameter: f64,
    /// Number of flutes, for the chip load. Null: unknown.
    #[serde(default)]
    pub flutes: Option<i64>,
    /// Whether it can plunge straight down. False: profiles ramp in and pockets go down
    /// round a helix. Null: unknown.
    #[serde(default)]
    pub center_cutting: Option<bool>,
    /// Spindle speed, rpm. Needed to generate.
    #[serde(default)]
    pub rpm: Option<f64>,
    /// V-bits: the included angle, radians. Null: unknown; a V-carve then refuses the tool.
    // Spec: docs/specs/v-carve.md §2.
    #[serde(default)]
    pub angle: Option<f64>,
    /// V-bits: the flat tip's diameter, mm. Null: pointed.
    #[serde(default)]
    pub tip_diameter: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Selector {
    /// Every shape on this layer (upper or lower case don't matter).
    #[serde(default)]
    pub layer: Option<String>,
    /// These shapes, by id.
    #[serde(default)]
    pub shapes: Option<Vec<u64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Tabs {
    /// Tabs per contour, at least 1.
    pub count: i64,
    /// The bridge left on the part, along its edge, mm (the flat top of a sloped tab).
    pub width: f64,
    /// The bridge's thickness, up from the bottom of the cut, mm.
    pub height: f64,
    /// The slope of the tab's sides, radians: π/2 (90°) straight, less for sloped tabs a
    /// ramped profile follows.
    // Spec: docs/specs/profile-spiral.md §4.
    pub flank_angle: f64,
}

/// Straight tabs: up and down vertically.
pub const STRAIGHT_FLANK_RAD: f64 = std::f64::consts::FRAC_PI_2;
/// The flank angle new tabs get with a ramp entry and a tool that can plunge: 30°.
pub const DEFAULT_RAMPED_FLANK_RAD: f64 = std::f64::consts::PI / 6.0;

/// A flank angle of 90° (within the angle tolerance): a straight tab.
pub fn flank_is_straight(flank_angle: f64) -> bool {
    flank_angle >= STRAIGHT_FLANK_RAD - ANGLE_TOL_RAD
}

impl Tabs {
    /// Straight up and down (90°, within the angle tolerance).
    pub fn is_straight(&self) -> bool {
        flank_is_straight(self.flank_angle)
    }

    /// The flank angle new tabs get (docs/specs/profile-spiral.md §4): straight with a
    /// plunge entry; with a ramp, 30°, or the ramp angle when the tool is marked not
    /// centre-cutting (a flank's way down is a ramp).
    pub fn default_flank(entry: Entry, center_cutting: Option<bool>) -> f64 {
        match entry.ramp_angle() {
            None => STRAIGHT_FLANK_RAD,
            Some(ramp_angle) if center_cutting == Some(false) => ramp_angle,
            Some(_) => DEFAULT_RAMPED_FLANK_RAD,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Inside,
    Outside,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Climb,
    Conventional,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Operation {
    Profile(ProfileOp),
    Pocket(PocketOp),
    Vcarve(VcarveOp),
    /// Follow path: the tool centre on the selected shapes, open or closed.
    // Spec: docs/specs/follow-path.md.
    Path(PathOp),
    /// Holes: drilled with a drill, bored with a helix with an endmill.
    // Spec: docs/specs/holes.md.
    Holes(HolesOp),
}

fn enabled_default() -> bool {
    true
}

impl Operation {
    pub fn name(&self) -> &str {
        match self {
            Operation::Profile(op) => &op.name,
            Operation::Pocket(op) => &op.name,
            Operation::Vcarve(op) => &op.name,
            Operation::Path(op) => &op.name,
            Operation::Holes(op) => &op.name,
        }
    }

    pub fn name_mut(&mut self) -> &mut String {
        match self {
            Operation::Profile(op) => &mut op.name,
            Operation::Pocket(op) => &mut op.name,
            Operation::Vcarve(op) => &mut op.name,
            Operation::Path(op) => &mut op.name,
            Operation::Holes(op) => &mut op.name,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        match self {
            Operation::Profile(op) => op.enabled = enabled,
            Operation::Pocket(op) => op.enabled = enabled,
            Operation::Vcarve(op) => op.enabled = enabled,
            Operation::Path(op) => op.enabled = enabled,
            Operation::Holes(op) => op.enabled = enabled,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Operation::Profile(op) => op.enabled,
            Operation::Pocket(op) => op.enabled,
            Operation::Vcarve(op) => op.enabled,
            Operation::Path(op) => op.enabled,
            Operation::Holes(op) => op.enabled,
        }
    }

    pub fn tool(&self) -> i64 {
        match self {
            Operation::Profile(op) => op.tool,
            Operation::Pocket(op) => op.tool,
            Operation::Vcarve(op) => op.tool,
            Operation::Path(op) => op.tool,
            Operation::Holes(op) => op.tool,
        }
    }

    pub fn set_tool(&mut self, tool: i64) {
        match self {
            Operation::Profile(op) => op.tool = tool,
            Operation::Pocket(op) => op.tool = tool,
            Operation::Vcarve(op) => op.tool = tool,
            Operation::Path(op) => op.tool = tool,
            Operation::Holes(op) => op.tool = tool,
        }
    }

    /// (start_depth, end_depth), mm down from the stock top.
    pub fn depths(&self) -> (f64, f64) {
        match self {
            Operation::Profile(op) => (op.start_depth, op.end_depth),
            Operation::Pocket(op) => (op.start_depth, op.end_depth),
            Operation::Vcarve(op) => (op.start_depth, op.end_depth),
            Operation::Path(op) => (op.start_depth, op.end_depth),
            Operation::Holes(op) => (op.start_depth, op.end_depth),
        }
    }

    pub fn entry(&self) -> Entry {
        match self {
            Operation::Profile(p) => p.entry,
            Operation::Pocket(p) => p.entry,
            // Paths start at the ends of the axis, mostly at depth 0; others plunge.
            Operation::Vcarve(_) => Entry::Plunge {},
            Operation::Path(p) => p.entry,
            // Holes go down their own way (docs/specs/holes.md §3, §4).
            Operation::Holes(_) => Entry::Plunge {},
        }
    }

    pub fn starts(&self) -> &[[f64; 2]] {
        match self {
            Operation::Profile(p) => &p.starts,
            Operation::Pocket(p) => &p.starts,
            Operation::Vcarve(_) => &[],
            Operation::Path(_) | Operation::Holes(_) => &[],
        }
    }

    /// None for an operation without start points (a V-carve, a follow path, holes).
    pub fn starts_mut(&mut self) -> Option<&mut Vec<[f64; 2]>> {
        match self {
            Operation::Profile(p) => Some(&mut p.starts),
            Operation::Pocket(p) => Some(&mut p.starts),
            Operation::Vcarve(_) | Operation::Path(_) | Operation::Holes(_) => None,
        }
    }

    /// Dogbones or T-bones (docs/specs/dogbones.md): profiles and pockets only.
    pub fn corners(&self) -> Option<Corners> {
        match self {
            Operation::Profile(p) => p.corners,
            Operation::Pocket(p) => p.corners,
            _ => None,
        }
    }

    pub fn lead(&self) -> Option<Lead> {
        match self {
            Operation::Profile(p) => p.lead,
            Operation::Pocket(p) => p.lead,
            Operation::Vcarve(_) | Operation::Path(_) | Operation::Holes(_) => None,
        }
    }

    pub fn clearance(&self) -> f64 {
        match self {
            Operation::Profile(op) => op.clearance,
            Operation::Pocket(op) => op.clearance,
            // A V-carve's cut is set by the outline and the angle alone.
            Operation::Vcarve(_) => 0.0,
            // On the line: no offset.
            Operation::Path(_) => 0.0,
            // Cut to the drawn size (docs/specs/holes.md decision 9).
            Operation::Holes(_) => 0.0,
        }
    }

    pub fn select(&self) -> &Selector {
        match self {
            Operation::Profile(op) => &op.select,
            Operation::Pocket(op) => &op.select,
            Operation::Vcarve(op) => &op.select,
            Operation::Path(op) => &op.select,
            Operation::Holes(op) => &op.select,
        }
    }

    pub fn select_mut(&mut self) -> &mut Selector {
        match self {
            Operation::Profile(op) => &mut op.select,
            Operation::Pocket(op) => &mut op.select,
            Operation::Vcarve(op) => &mut op.select,
            Operation::Path(op) => &mut op.select,
            Operation::Holes(op) => &mut op.select,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ProfileOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// Which side of the line the tool cuts: `outside` (cutting a part out), `inside` (a
    /// hole) or `on` the line.
    pub side: Side,
    /// `climb` or `conventional` milling.
    pub direction: Direction,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    // Spec: docs/specs/depths-and-clearance.md.
    pub start_depth: f64,
    /// Where the cut ends, mm down from the stock top. More than the start depth; below the
    /// stock bottom only within `stock.max_overcut`.
    pub end_depth: f64,
    /// How far the cut moves into the material that is kept, mm: positive removes more (a
    /// larger pocket, a smaller part), negative leaves material. Less than the tool radius.
    pub clearance: f64,
    /// The deepest each pass goes, mm. The passes are equal, and the last one ends exactly
    /// at the end depth.
    pub step_down: f64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// Holding tabs left at the bottom of the cut, so a part cut out doesn't come loose.
    /// Null: none.
    #[serde(default)]
    pub tabs: Option<Tabs>,
    /// How the tool goes down at the start of each pass: `plunge` (straight down; needs a
    /// centre-cutting tool) or `ramp` along the path; pockets also `helix`. Missing:
    /// plunge.
    // Spec: docs/specs/ramp-entry.md.
    #[serde(default)]
    pub entry: Entry,
    /// Where the cut starts, as points in drawing coordinates (mm): each moves the start of
    /// the ring (contour) nearest to it. Empty: chosen by Chipload.
    // Spec: docs/specs/helix-and-leads.md, part A.
    #[serde(default)]
    pub starts: Vec<[f64; 2]>,
    /// A quarter-circle lead-in and lead-out from the waste side onto the path. Null: none.
    // Spec: docs/specs/helix-and-leads.md, part B.
    #[serde(default)]
    pub lead: Option<Lead>,
    /// A finishing pass: the roughing passes leave `stock` mm on the wall, and one last lap
    /// at full depth takes it off. Null: none.
    // Spec: docs/specs/finish-and-skin.md.
    #[serde(default)]
    pub finish: Option<Finish>,
    /// An onion skin: the passes stop `thickness` mm above the end depth, and one last lap
    /// cuts through it. Null: none.
    // Spec: docs/specs/finish-and-skin.md.
    #[serde(default)]
    pub skin: Option<Skin>,
    /// Dogbones or T-bones in the inside corners, so square parts fit. Null: the corners
    /// stay round.
    // Spec: docs/specs/dogbones.md.
    #[serde(default)]
    pub corners: Option<Corners>,
}

/// How the tool reaches into the inside corners it would leave round, so a square part
/// fits: every corner on the tool's side up to `max_angle`.
// Spec: docs/specs/dogbones.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Corners {
    /// In along the corner's bisector.
    Dogbone {
        /// Only corners this sharp or sharper, radians (1.745 = 100°).
        max_angle: f64,
    },
    /// In along the longer wall: the notch is in the shorter one.
    Tbone {
        /// Only corners this sharp or sharper, radians (1.745 = 100°).
        max_angle: f64,
    },
}

impl Corners {
    pub fn max_angle(&self) -> f64 {
        match *self {
            Corners::Dogbone { max_angle } | Corners::Tbone { max_angle } => max_angle,
        }
    }
}

/// The largest corner angle new dogbones get: 100°, so 90° corners drawn slightly off count.
pub const DEFAULT_CORNER_ANGLE_RAD: f64 = 100.0 * std::f64::consts::PI / 180.0;

/// A finishing pass on a profile.
// Spec: docs/specs/finish-and-skin.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Finish {
    /// Left on the wall by the roughing passes, mm: more than 0, less than the tool radius.
    pub stock: f64,
}

/// An onion skin on a profile.
// Spec: docs/specs/finish-and-skin.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Skin {
    /// Left above the end depth until the last lap, mm: more than 0, at most the step down.
    pub thickness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PocketOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    // Spec: docs/specs/depths-and-clearance.md.
    pub start_depth: f64,
    /// Where the cut ends, mm down from the stock top. More than the start depth; below the
    /// stock bottom only within `stock.max_overcut`.
    pub end_depth: f64,
    /// How far the cut moves into the material that is kept, mm: positive removes more (a
    /// larger pocket, a smaller part), negative leaves material. Less than the tool radius.
    pub clearance: f64,
    /// The deepest each pass goes, mm. The passes are equal, and the last one ends exactly
    /// at the end depth.
    pub step_down: f64,
    /// The distance between the clearing rings, as a fraction of the tool diameter: more
    /// than 0, at most 1.
    pub stepover: f64,
    /// `climb` or `conventional` milling.
    pub direction: Direction,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// Between rings, feed across at depth instead of lifting, where the move stays clear
    /// of the walls and is at most a tool diameter. Missing: false.
    #[serde(default)]
    pub keep_tool_down: bool,
    /// How the pocket is cleared: `{"type": "rings"}`. (`trochoidal` and `adaptive` were
    /// removed in 0.14: such projects load but don't generate.) Missing: rings.
    // Spec: docs/specs/trochoidal.md.
    #[serde(default)]
    pub strategy: PocketStrategy,
    /// How the tool goes down at the start of each pass: `plunge` (straight down; needs a
    /// centre-cutting tool) or `ramp` along the path; pockets also `helix`. Missing:
    /// plunge.
    // Spec: docs/specs/ramp-entry.md.
    #[serde(default)]
    pub entry: Entry,
    /// Where each wall ring starts, as points in drawing coordinates (mm): each moves the
    /// start of the wall ring nearest to it. Empty: chosen by Chipload.
    // Spec: docs/specs/helix-and-leads.md, part A.
    #[serde(default)]
    pub starts: Vec<[f64; 2]>,
    /// A quarter-circle lead-in and lead-out from the cleared side onto the wall. Null:
    /// none.
    // Spec: docs/specs/helix-and-leads.md, part B.
    #[serde(default)]
    pub lead: Option<Lead>,
    /// Round the corners of every ring but the one along the wall, for smoother motion.
    // Spec: docs/specs/pocket-round-corners.md.
    pub round_corners: bool,
    /// Dogbones or T-bones in the inside corners, so square parts fit. Null: the corners
    /// stay round.
    // Spec: docs/specs/dogbones.md.
    #[serde(default)]
    pub corners: Option<Corners>,
    /// Rest machining: clear only what a larger tool leaves in these shapes. Null: the
    /// whole pocket.
    // Spec: docs/specs/rest-machining.md.
    #[serde(default)]
    pub rest: Option<Rest>,
}

/// Rest machining: the pocket clears only the material the larger tool `after_tool` can't
/// reach in its shapes.
// Spec: docs/specs/rest-machining.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Rest {
    /// The id of the larger project tool whose leftovers this pocket clears.
    pub after_tool: i64,
}

/// V-carve: a V-bit follows the centre line of closed shapes, as deep as makes the cut
/// exactly as wide as the shape.
// Spec: docs/specs/v-carve.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct VcarveOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// The surface carved, mm down from the stock top: 0 for the top, more to carve on a
    /// floor an earlier pocket cut.
    pub start_depth: f64,
    /// The deepest the V-bit's tip may go, mm down from the stock top. Strokes that need
    /// more are carved to it, narrower than drawn, with a warning.
    pub end_depth: f64,
    /// Carve in passes no deeper than this, mm. Null: one pass.
    #[serde(default)]
    pub step_down: Option<f64>,
    /// The feed, mm/min, for moves no steeper than 45°.
    pub feed: f64,
    /// The feed for moves steeper than 45°, mm/min.
    pub plunge_feed: f64,
}

/// Follow path: the tool centre on the selected shapes, open or closed, in depth passes.
// Spec: docs/specs/follow-path.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PathOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    pub start_depth: f64,
    /// Where the cut ends, mm down from the stock top. More than the start depth; below the
    /// stock bottom only within `stock.max_overcut`.
    pub end_depth: f64,
    /// The deepest each pass goes, mm. The passes are equal, and the last one ends exactly
    /// at the end depth.
    pub step_down: f64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// How the tool goes down at the start of each pass: `plunge` or `ramp` along the path.
    pub entry: Entry,
}

/// Holes: every selected circle, drilled with a drill or bored with a helix with an
/// endmill.
// Spec: docs/specs/holes.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HolesOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// What the operation cuts: the circles among the shapes on a layer, or shapes by id.
    /// Other shapes are skipped with a note.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    pub start_depth: f64,
    /// Where the hole ends, mm down from the stock top. For a drill, the depth of its
    /// point.
    pub end_depth: f64,
    /// A bore's feed round its helix and bottom circle, mm/min. A drill doesn't use it.
    pub feed: f64,
    /// A drill's feed down, or a bore's feed straight down to the start depth, mm/min.
    pub plunge_feed: f64,
    /// How the holes are cut: `drill` with a drill, or `bore` (a helix) with an endmill.
    pub method: HoleMethod,
}

/// How a Holes operation cuts.
// Spec: docs/specs/holes.md §2.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum HoleMethod {
    /// Straight down at the hole's centre, `peck` mm at a time.
    Drill {
        /// Drill this many mm at a time, lifting out between pecks to clear chips. Null: in
        /// one go.
        peck: Option<f64>,
    },
    /// A helix at the wall, at most `step_down` mm per turn and never steeper than
    /// `ramp_angle` (radians, 0 < angle <= 30°), then one level circle.
    Bore {
        /// The most the helix goes down per turn, mm.
        step_down: f64,
        /// The steepest the helix goes, radians: more than 0, at most 30° (0.5236).
        ramp_angle: f64,
        /// `climb` or `conventional`, as for a profile inside the hole.
        direction: Direction,
    },
}

impl HoleMethod {
    /// The method a tool of `kind` and `diameter` takes, with its defaults
    /// (docs/specs/holes.md §7); None for a V-bit.
    pub fn default_for(kind: ToolType, diameter: f64) -> Option<HoleMethod> {
        match kind {
            ToolType::Drill => Some(HoleMethod::Drill { peck: None }),
            ToolType::Endmill => Some(HoleMethod::Bore {
                step_down: 0.5 * diameter,
                ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                direction: Direction::Climb,
            }),
            ToolType::Vbit => None,
        }
    }

    /// The tool type this method needs.
    pub fn tool_type(&self) -> ToolType {
        match self {
            HoleMethod::Drill { .. } => ToolType::Drill,
            HoleMethod::Bore { .. } => ToolType::Endmill,
        }
    }
}

/// Quarter-circle lead-in and lead-out.
// Spec: docs/specs/helix-and-leads.md part B.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Lead {
    /// Radius of the lead arcs, mm: more than 0, at most 10 tool diameters. Smaller where
    /// there's no room.
    pub radius: f64,
}

/// The longest lead allowed, in tool diameters.
pub const MAX_LEAD_DIAMETERS: f64 = 10.0;

/// How the tool goes down into the material at the start of each pass.
// Spec: docs/specs/ramp-entry.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Entry {
    /// Straight down: needs a centre-cutting tool. An empty struct variant, as `Rings`.
    Plunge {},
    /// Down along the path and back, never steeper than `ramp_angle` (radians,
    /// 0 < angle <= 30°).
    Ramp {
        /// The steepest the ramp goes, radians: more than 0, at most 30° (0.5236).
        ramp_angle: f64,
    },
    /// Pockets only: down round a circle of `radius` (the tool centre's), never steeper
    /// than `ramp_angle`.
    // Spec: docs/specs/helix-and-leads.md part C.
    Helix {
        /// The helix's radius, mm: less than the tool radius.
        radius: f64,
        /// The steepest the helix goes, radians: more than 0, at most 30° (0.5236).
        ramp_angle: f64,
    },
}

impl Entry {
    /// How steeply a ramp or helix goes down; None for a straight plunge.
    pub fn ramp_angle(&self) -> Option<f64> {
        match *self {
            Entry::Plunge {} => None,
            Entry::Ramp { ramp_angle } | Entry::Helix { ramp_angle, .. } => Some(ramp_angle),
        }
    }

    /// A helix for a tool of `diameter`, with the default radius and angle.
    pub fn default_helix(diameter: f64) -> Entry {
        Entry::Helix {
            radius: DEFAULT_HELIX_DIAMETERS * diameter,
            ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
        }
    }
}

/// The helix radius a new helix gets, in tool diameters.
pub const DEFAULT_HELIX_DIAMETERS: f64 = 0.25;

impl Default for Entry {
    fn default() -> Self {
        Entry::Plunge {}
    }
}

/// The steepest ramp allowed: 30°.
pub const MAX_RAMP_ANGLE_RAD: f64 = std::f64::consts::PI / 6.0;
/// The ramp angle a new ramp gets: 3°.
pub const DEFAULT_RAMP_ANGLE_RAD: f64 = 3.0 * std::f64::consts::PI / 180.0;

/// Clearing strategy of a pocket.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum PocketStrategy {
    /// Rings parallel to the walls, cut from the inside out. An empty struct variant (not a
    /// unit one) so serde refuses stray fields here too.
    Rings {},
    /// Removed in 0.14: still read, so older projects load, but refused when generating
    /// until switched to rings.
    // Spec: docs/decisions/0017-scope.md.
    Trochoidal {
        /// Removed in 0.14; kept so old projects load.
        loop_radius: f64,
        /// Removed in 0.14; kept so old projects load.
        step: f64,
    },
    /// Removed in 0.14, as `trochoidal`.
    // Spec: docs/specs/adaptive-fronts.md.
    Adaptive {},
}

impl PocketStrategy {
    /// The name of a strategy that was removed, None for rings.
    pub fn removed_name(&self) -> Option<&'static str> {
        match self {
            PocketStrategy::Rings {} => None,
            PocketStrategy::Trochoidal { .. } => Some("trochoidal"),
            PocketStrategy::Adaptive {} => Some("adaptive"),
        }
    }
}

impl Default for PocketStrategy {
    fn default() -> Self {
        PocketStrategy::Rings {}
    }
}

impl Drawing {
    /// Structural rules of the drawing, as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
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
            if let Some(image) = shape.image()
                && !self.images.iter().any(|i| i.id == image)
            {
                problems.push(format!("{at}.image: no image {image} in drawing.images"));
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        for (i, image) in self.images.iter().enumerate() {
            if !ids.insert(image.id) {
                problems.push(format!("drawing.images.{i}.id: {} is used twice", image.id));
            }
            problems.extend(image.problems(i));
        }
        problems
    }
}

/// Value rules of one shape, as "field: message" (spec drawing-tools.md section 1).
/// What is wrong with a shape's values, as "field: message"; empty when it is valid.
pub fn shape_problems(geometry: &Geometry) -> Vec<String> {
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
        Geometry::Polyline {
            closed,
            points,
            bulges,
        } => {
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
            if bulges.len() != pairs {
                problems.push(format!(
                    "bulges: one per segment, {pairs} for this polyline, got {}",
                    bulges.len()
                ));
            }
            for (i, b) in bulges.iter().enumerate() {
                if !(b.is_finite() && b.abs() <= 1.0) {
                    problems.push(format!(
                        "bulges.{i}: must be between -1 and 1 (at most a half circle), got {b}"
                    ));
                }
            }
        }
    }
    problems
}

/// "<at>.<field>: must be greater than 0, got <value>" unless value > 0.
fn positive_problem(at: &str, field: &str, value: f64) -> Option<String> {
    (value <= 0.0 || value.is_nan())
        .then(|| format!("{at}.{field}: must be greater than 0, got {value}"))
}

/// The rule for `max_overcut`, kept under `stock` in projects and in machine profiles.
pub(crate) fn max_overcut_problems(at: &str, overcut: f64) -> Vec<String> {
    if overcut.is_finite() && overcut >= 0.0 {
        Vec::new()
    } else {
        vec![format!(
            "{at}.max_overcut: must be 0 or more, got {overcut}"
        )]
    }
}

impl Machine {
    /// Value rules of a machine, as "<at>.field: message"; shared by projects and machine
    /// profiles.
    pub fn problems(&self, at: &str) -> Vec<String> {
        let mut problems = Vec::new();
        if let WorkZero::Point { x, y } = self.work_zero
            && !(x.is_finite() && y.is_finite())
        {
            problems.push(format!(
                "{at}.work_zero: x and y must be finite, got {x}, {y}"
            ));
        }
        if let Some(t) = &self.travel {
            for (axis, value) in [("x", t.x), ("y", t.y), ("z", t.z)] {
                problems.extend(positive_problem(&format!("{at}.travel"), axis, value));
            }
        }
        problems
    }
}

impl Tool {
    /// Value rules of a tool, as "<at>.field: message"; shared by projects and the tool
    /// library.
    pub fn problems(&self, at: &str) -> Vec<String> {
        let mut problems: Vec<String> = positive_problem(at, "diameter", self.diameter)
            .into_iter()
            .collect();
        if let Some(angle) = self.angle
            && !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::PI)
        {
            problems.push(format!(
                "{at}.angle: must be more than 0 and less than 180 degrees, got {:.3} degrees",
                angle.to_degrees()
            ));
        }
        if let Some(tip) = self.tip_diameter
            && !(tip.is_finite() && tip >= 0.0 && tip < self.diameter)
        {
            problems.push(format!(
                "{at}.tip_diameter: must be 0 or more and less than the diameter ({}), got {tip}",
                self.diameter
            ));
        }
        problems
    }
}

impl Job {
    /// Every value rule the project breaks, with the field it is about
    /// (docs/specs/live-problems.md).
    pub fn problems(&self) -> Vec<crate::Problem> {
        self.validate()
            .iter()
            .map(|text| crate::Problem::from_path_text(text))
            .collect()
    }

    /// Value constraints the serde types can't express.
    /// Returns every violation as "path: message".
    pub fn validate(&self) -> Vec<String> {
        let mut problems = self.drawing.validate();
        problems.extend(self.machine.problems("machine"));
        if let Some(source) = &self.drawing.source {
            let Placement {
                scale,
                quarter_turns,
                dx,
                dy,
            } = source.placement;
            if !Placement::scale_in_range(scale) {
                problems.push(format!(
                    "drawing.source.placement.scale: must be {} to {}, got {scale}",
                    Placement::MIN_SCALE,
                    Placement::MAX_SCALE
                ));
            }
            if !(dx.is_finite() && dy.is_finite()) {
                problems.push(format!(
                    "drawing.source.placement: must be numbers, got ({dx}, {dy})"
                ));
            }
            if quarter_turns > 3 {
                problems.push(format!(
                    "drawing.source.placement.quarter_turns: must be 0 to 3, got {quarter_turns}"
                ));
            }
        }
        let mut positive = |path: String, value: f64| {
            if value <= 0.0 || value.is_nan() {
                problems.push(format!("{path}: must be greater than 0, got {value}"));
            }
        };
        positive("stock.x".into(), self.stock.x);
        positive("stock.y".into(), self.stock.y);
        positive("stock.thickness".into(), self.stock.thickness);
        for (i, tool) in self.tools.iter().enumerate() {
            problems.extend(tool.problems(&format!("tools.{i}")));
        }
        let mut positive = |path: String, value: f64| {
            if value <= 0.0 || value.is_nan() {
                problems.push(format!("{path}: must be greater than 0, got {value}"));
            }
        };
        for (i, op) in self.operations.iter().enumerate() {
            let at = |field: &str| format!("operations.{i}.{field}");
            match op {
                Operation::Profile(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    if let Some(tabs) = &p.tabs {
                        positive(at("tabs.width"), tabs.width);
                        positive(at("tabs.height"), tabs.height);
                    }
                    if let Some(finish) = &p.finish {
                        positive(at("finish.stock"), finish.stock);
                    }
                    if let Some(skin) = &p.skin {
                        positive(at("skin.thickness"), skin.thickness);
                    }
                }
                Operation::Pocket(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    positive(at("stepover"), p.stepover);
                }
                Operation::Vcarve(v) => {
                    positive(at("end_depth"), v.end_depth);
                    if let Some(step_down) = v.step_down {
                        positive(at("step_down"), step_down);
                    }
                    positive(at("feed"), v.feed);
                    positive(at("plunge_feed"), v.plunge_feed);
                }
                Operation::Path(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                }
                Operation::Holes(h) => {
                    positive(at("end_depth"), h.end_depth);
                    positive(at("feed"), h.feed);
                    positive(at("plunge_feed"), h.plunge_feed);
                    match h.method {
                        HoleMethod::Drill { peck } => {
                            if let Some(peck) = peck {
                                positive(at("method.peck"), peck);
                            }
                        }
                        HoleMethod::Bore { step_down, .. } => {
                            positive(at("method.step_down"), step_down);
                        }
                    }
                }
            }
        }
        problems.extend(max_overcut_problems("stock", self.stock.max_overcut));
        if let Some(copies) = &self.copies {
            problems.extend(copies.problems());
        }
        for (i, op) in self.operations.iter().enumerate() {
            let (start, end) = op.depths();
            if !(start.is_finite() && start >= 0.0) {
                problems.push(format!(
                    "operations.{i}.start_depth: must be 0 or more, got {start}"
                ));
            } else if start >= end {
                problems.push(format!(
                    "operations.{i}.start_depth: must be less than end_depth ({end}), got \
                     {start}"
                ));
            }
            if !op.clearance().is_finite() {
                problems.push(format!(
                    "operations.{i}.clearance: must be a number, got {}",
                    op.clearance()
                ));
            }
            if let Operation::Profile(p) = op
                && p.side == Side::On
                && p.clearance != 0.0
            {
                problems.push(format!(
                    "operations.{i}.clearance: a profile on the line has no side to clear \
                     towards; it must be 0, got {}",
                    p.clearance
                ));
            }
        }
        for (i, op) in self.operations.iter().enumerate() {
            let selector = op.select();
            let has_shapes = selector.shapes.as_ref().is_some_and(|s| !s.is_empty());
            if selector.layer.is_none() && !has_shapes {
                problems.push(format!(
                    "operations.{i}.select: selector needs a layer or shapes"
                ));
            }
            // Traced shapes get new ids each time their image is traced again
            // (docs/specs/image-trace.md §4, decision 6).
            let traced = selector.shapes.iter().flatten().find_map(|id| {
                self.drawing
                    .shapes
                    .iter()
                    .find(|s| s.id == *id && s.image().is_some())
            });
            if let Some(shape) = traced {
                problems.push(format!(
                    "operations.{i}.select: shape {} is traced from an image and changes \
                     each time the image is traced again: select its layer \"{}\" instead",
                    shape.id, shape.layer
                ));
            }
            if let Entry::Helix { radius, .. } = op.entry()
                && !(radius.is_finite() && radius > 0.0)
            {
                problems.push(format!(
                    "operations.{i}.entry.radius: must be more than 0, got {radius}"
                ));
            }
            if let Some(ramp_angle) = op.entry().ramp_angle()
                && !(ramp_angle.is_finite() && ramp_angle > 0.0 && ramp_angle <= MAX_RAMP_ANGLE_RAD)
            {
                problems.push(format!(
                    "operations.{i}.entry.ramp_angle: must be more than 0 and at most 30 \
                     degrees, got {:.3} degrees",
                    ramp_angle.to_degrees()
                ));
            }
            if let Some(corners) = op.corners() {
                let angle = corners.max_angle();
                if !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::PI) {
                    problems.push(format!(
                        "operations.{i}.corners.max_angle: must be more than 0 and less than \
                         180 degrees, got {:.3} degrees",
                        angle.to_degrees()
                    ));
                }
                if let Operation::Profile(p) = op
                    && p.side == Side::On
                {
                    problems.push(format!(
                        "operations.{i}.corners: a profile on the line has no inside corners \
                         to clear"
                    ));
                }
            }
            if let Operation::Holes(HolesOp {
                method: HoleMethod::Bore { ramp_angle, .. },
                ..
            }) = op
                && !(ramp_angle.is_finite()
                    && *ramp_angle > 0.0
                    && *ramp_angle <= MAX_RAMP_ANGLE_RAD)
            {
                problems.push(format!(
                    "operations.{i}.method.ramp_angle: must be more than 0 and at most 30 \
                     degrees, got {:.3} degrees",
                    ramp_angle.to_degrees()
                ));
            }
            if let Some(lead) = op.lead()
                && !(lead.radius.is_finite() && lead.radius > 0.0)
            {
                problems.push(format!(
                    "operations.{i}.lead.radius: must be more than 0, got {}",
                    lead.radius
                ));
            }
            if op
                .starts()
                .iter()
                .any(|p| !(p[0].is_finite() && p[1].is_finite()))
            {
                problems.push(format!(
                    "operations.{i}.starts: every start point needs finite x and y"
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
            if let Operation::Profile(ProfileOp {
                tabs: Some(tabs), ..
            }) = op
                && !(tabs.flank_angle.is_finite()
                    && tabs.flank_angle > 0.0
                    && tabs.flank_angle <= STRAIGHT_FLANK_RAD + ANGLE_TOL_RAD)
            {
                problems.push(format!(
                    "operations.{i}.tabs.flank_angle: must be more than 0 and at most 90 \
                     degrees, got {:.3} degrees",
                    tabs.flank_angle.to_degrees()
                ));
            }
            if let Operation::Profile(p) = op {
                problems.extend(finish_skin_problems(p, &format!("operations.{i}")));
            }
        }
        problems
    }
}

/// The rules for a profile's finishing pass and skin that need no tool
/// (docs/specs/finish-and-skin.md §1, §6); the tool radius is checked when planning.
fn finish_skin_problems(p: &ProfileOp, at: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if p.finish.is_some() && p.side == Side::On {
        problems.push(format!(
            "{at}.finish: a profile on the line has no wall to finish; use inside or outside"
        ));
    }
    let Some(skin) = p.skin else {
        return problems;
    };
    let t = skin.thickness;
    if !(t.is_finite() && t > 0.0) {
        return problems; // reported as not positive
    }
    if t > p.step_down + DEPTH_TOL_MM {
        problems.push(format!(
            "{at}.skin.thickness: must be at most the step down ({}), got {t}",
            p.step_down
        ));
    }
    let cut = p.end_depth - p.start_depth;
    if t >= cut {
        problems.push(format!(
            "{at}.skin.thickness: must be less than the cut ({cut}, end depth minus start \
             depth), got {t}"
        ));
    }
    if let Some(tabs) = &p.tabs
        && t >= tabs.height
    {
        problems.push(format!(
            "{at}.skin.thickness: must be less than the tab height ({}), got {t}",
            tabs.height
        ));
    }
    problems
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
            bulges: vec![0.0; 2],
        });
        let closed = shape(Geometry::Polyline {
            closed: true,
            points,
            bulges: vec![0.0; 3],
        });
        assert_eq!(open.segments().len(), 2);
        assert_eq!(closed.segments().len(), 3);
        assert!(!open.closed() && closed.closed());
    }

    #[test]
    fn a_bulged_segment_is_the_arc_a_dxf_bulge_describes() {
        let bulged = shape(Geometry::Polyline {
            closed: true,
            points: vec![[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]],
            bulges: vec![0.0, 0.4, 0.0],
        });
        let segments = bulged.segments();
        assert!(matches!(segments[0], PathSegment::Line(_)));
        assert!(matches!(segments[2], PathSegment::Line(_)));
        let PathSegment::Arc(arc) = segments[1] else {
            panic!("arc");
        };
        let expected = Arc::from_bulge((40.0, 0.0), (20.0, 30.0), 0.4).unwrap();
        assert_eq!(arc.center, [expected.center.0, expected.center.1]);
        assert_eq!(arc.radius, expected.radius);
        assert_eq!(arc.start_angle, expected.start_angle);
        assert_eq!(arc.sweep, 4.0 * 0.4f64.atan());
        // It ends on the next point.
        let end_angle = arc.start_angle + arc.sweep;
        let end = [
            arc.center[0] + arc.radius * end_angle.cos(),
            arc.center[1] + arc.radius * end_angle.sin(),
        ];
        assert!((end[0] - 20.0).abs() < 1e-9 && (end[1] - 30.0).abs() < 1e-9);
    }
}
