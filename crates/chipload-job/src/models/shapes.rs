// SPDX-License-Identifier: GPL-3.0-or-later
//! Shapes: what each is (geometry and origin), how it is written in the file, its lines and
//! arcs, and the rules a shape must meet.

use super::*;

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
    /// A drawn shape's name in the Browser; None: shown by its id
    /// (docs/specs/drawn-shapes-box.md §4).
    pub name: Option<String>,
    pub geometry: Geometry,
    /// Construction geometry: shown and snapped to, never cut (drawn shapes only).
    // Spec: docs/specs/dxf-export.md, part B (schema 32).
    pub construction: bool,
}

/// Where a path comes from (docs/specs/source-model.md §2.1). Written flat in the file:
/// `dxf`, `dxf_handle`, `dxf_type`, `image`, `text`.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    /// Read from a DXF or SVG: its id in `drawing.dxfs`, the handle of the entity it came
    /// from (the INSERT's for block content), which keeps its id when the DXF is updated,
    /// and that entity's type, e.g. "LWPOLYLINE".
    Dxf {
        id: u64,
        handle: String,
        entity: String,
    },
    /// Traced from the picture with this id (docs/specs/image-trace.md §4).
    Image(u64),
    /// A letter of the text with this id (docs/specs/text.md §3).
    Text(u64),
}

impl Origin {
    /// What its segments carry.
    pub fn source(&self) -> ShapeFrom {
        match self {
            Origin::Dxf { id, .. } => ShapeFrom::Dxf(*id),
            Origin::Image(id) => ShapeFrom::Image(*id),
            Origin::Text(id) => ShapeFrom::Text(*id),
        }
    }
}

/// What a shape is. Drawn kinds store their defining values; their segments are derived.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// Lines and arcs: imported from a DXF exactly as read, traced from a picture, a
    /// text's letter, or (no origin) inserted from the shape library.
    Path {
        /// The last segment ends where the first starts.
        closed: bool,
        segments: Vec<PathSegment>,
        /// Where it comes from; None: nothing remakes it.
        origin: Option<Origin>,
    },
    Circle {
        center: [f64; 2],
        radius: f64,
    },
    /// `length` along the local X axis, `width` along local Y, rotated by `angle` (radians,
    /// CCW) about `center`; its corners rounded by `corner_radius` (0: sharp), at most half
    /// the shorter side (docs/specs/rectangle-corner-radius.md).
    Rectangle {
        center: [f64; 2],
        length: f64,
        width: f64,
        angle: f64,
        corner_radius: f64,
    },
    /// Segments through `points`, and back to the first if `closed`. Each segment bends by
    /// its bulge, as in DXF: tan(sweep / 4), 0 straight, ±1 a half circle (positive
    /// counter-clockwise); one per segment (docs/specs/drawing-follow-ups.md section 3).
    Polyline {
        closed: bool,
        points: Vec<[f64; 2]>,
        bulges: Vec<f64>,
    },
    /// A regular polygon: `sides` corners on the circle of `radius` about `center`, the first
    /// at `angle` (radians, CCW from +X) (docs/specs/drawing-input.md §5).
    Polygon {
        center: [f64; 2],
        radius: f64,
        sides: u32,
        angle: f64,
    },
}

impl Shape {
    pub fn kind_name(&self) -> &'static str {
        match self.geometry {
            Geometry::Path { .. } => "path",
            Geometry::Circle { .. } => "circle",
            Geometry::Rectangle { .. } => "rectangle",
            Geometry::Polyline { .. } => "polyline",
            Geometry::Polygon { .. } => "polygon",
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
            Geometry::Circle { .. } | Geometry::Rectangle { .. } | Geometry::Polygon { .. } => true,
        }
    }

    /// Where a path comes from; None for a drawn shape or an inserted path.
    pub fn origin(&self) -> Option<&Origin> {
        match &self.geometry {
            Geometry::Path { origin, .. } => origin.as_ref(),
            _ => None,
        }
    }

    /// What its segments carry: the DXF, picture or text it comes from.
    pub fn source(&self) -> Option<ShapeFrom> {
        self.origin().map(Origin::source)
    }

    /// The text this shape is a letter of.
    pub fn text(&self) -> Option<u64> {
        match self.origin() {
            Some(Origin::Text(id)) => Some(*id),
            _ => None,
        }
    }

    /// The DXF this shape was imported from.
    pub fn dxf(&self) -> Option<u64> {
        match self.origin() {
            Some(Origin::Dxf { id, .. }) => Some(*id),
            _ => None,
        }
    }

    /// The image this shape was traced from.
    pub fn image(&self) -> Option<u64> {
        match self.origin() {
            Some(Origin::Image(id)) => Some(*id),
            _ => None,
        }
    }

    /// The handle of the DXF entity it was imported from.
    pub fn dxf_handle(&self) -> Option<&str> {
        match self.origin() {
            Some(Origin::Dxf { handle, .. }) => Some(handle),
            _ => None,
        }
    }

    /// The type of the DXF entity it was imported from.
    pub fn dxf_type(&self) -> Option<&str> {
        match self.origin() {
            Some(Origin::Dxf { entity, .. }) => Some(entity),
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
                corner_radius,
            } => {
                let (sin, cos) = angle.sin_cos();
                let (hl, hw) = (length / 2.0, width / 2.0);
                let place =
                    |x: f64, y: f64| [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos];
                if *corner_radius == 0.0 {
                    // Counter-clockwise from the (-length/2, -width/2) corner.
                    let corners: Vec<[f64; 2]> = [(-hl, -hw), (hl, -hw), (hl, hw), (-hl, hw)]
                        .iter()
                        .map(|&(x, y)| place(x, y))
                        .collect();
                    lines_through(&corners, true)
                } else {
                    rounded_rectangle(hl, hw, *corner_radius, *angle, place)
                }
            }
            Geometry::Polyline {
                closed,
                points,
                bulges,
            } => polyline_segments(points, bulges, *closed),
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            } => lines_through(&polygon_corners(*center, *radius, *sides, *angle), true),
        }
    }
}

/// A regular polygon's corners, counter-clockwise from the one at `angle`.
pub fn polygon_corners(center: [f64; 2], radius: f64, sides: u32, angle: f64) -> Vec<[f64; 2]> {
    let n = f64::from(sides.max(1));
    (0..sides)
        .map(|k| {
            let (sin, cos) = (angle + f64::from(k) * std::f64::consts::TAU / n).sin_cos();
            [center[0] + radius * cos, center[1] + radius * sin]
        })
        .collect()
}

/// The fewest and most sides a polygon may have.
pub const POLYGON_SIDES: std::ops::RangeInclusive<u32> = 3..=1000;

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
pub(super) fn polyline_segments(
    points: &[[f64; 2]],
    bulges: &[f64],
    closed: bool,
) -> Vec<PathSegment> {
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

/// A rectangle of half sizes `hl`, `hw` with corners of radius `r`, counter-clockwise from
/// `r` right of its lower-left corner: each side's line (left out when the arcs meet), then
/// the quarter circle round the next corner. `place` turns and moves local points.
pub(super) fn rounded_rectangle(
    hl: f64,
    hw: f64,
    r: f64,
    angle: f64,
    place: impl Fn(f64, f64) -> [f64; 2],
) -> Vec<PathSegment> {
    use std::f64::consts::FRAC_PI_2;
    // Per side: its line's start and end, then the centre of the arc that follows and the
    // arc's start angle (local).
    let (x, y) = (hl - r, hw - r);
    let sides = [
        ((-x, -hw), (x, -hw), (x, -y), -FRAC_PI_2),
        ((hl, -y), (hl, y), (x, y), 0.0),
        ((x, hw), (-x, hw), (-x, y), FRAC_PI_2),
        ((-hl, y), (-hl, -y), (-x, -y), 2.0 * FRAC_PI_2),
    ];
    let mut segments = Vec::with_capacity(8);
    for (start, end, centre, start_angle) in sides {
        if (end.0 - start.0).hypot(end.1 - start.1) > CHAIN_TOL_MM {
            segments.push(PathSegment::Line(LineSegment {
                start: place(start.0, start.1),
                end: place(end.0, end.1),
            }));
        }
        segments.push(PathSegment::Arc(ArcSegment {
            center: place(centre.0, centre.1),
            radius: r,
            start_angle: start_angle + angle,
            sweep: FRAC_PI_2,
        }));
    }
    segments
}

pub(super) fn lines_through(points: &[[f64; 2]], closed: bool) -> Vec<PathSegment> {
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
    Polygon,
}

/// A shape as it appears in the file: every field a kind may have. Fields of other kinds are
/// refused, never ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub(super) struct RawShape {
    /// The shape's id, which operations select it by.
    id: u64,
    /// Its layer.
    layer: String,
    /// circle, rectangle, polyline: its name in the Browser; left out when it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// circle, rectangle, polygon, polyline: construction geometry, shown and snapped to but
    /// never cut; left out when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    construction: bool,
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
    /// path: the DXF it was imported from (its id in `drawing.dxfs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf: Option<u64>,
    /// path: the text it is a letter of (its id in `drawing.texts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<u64>,
    /// path, polyline: whether it ends where it starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    closed: Option<bool>,
    /// path: its lines and arcs, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    segments: Option<Vec<PathSegment>>,
    /// circle, rectangle, polygon: the centre, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    center: Option<[f64; 2]>,
    /// circle: the radius, mm; polygon: the radius to its corners, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    radius: Option<f64>,
    /// polygon: how many sides (and corners) it has, 3 to 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sides: Option<u32>,
    /// rectangle: its size along its own X axis, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    length: Option<f64>,
    /// rectangle: its size along its own Y axis, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    /// rectangle: its rotation about the centre; polygon: the angle of its first corner from
    /// +X; radians, counter-clockwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    angle: Option<f64>,
    /// rectangle: the radius of its corners, mm; 0 for sharp ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    corner_radius: Option<f64>,
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
            name: None,
            construction: false,
            kind,
            dxf_handle: None,
            dxf_type: None,
            image: None,
            dxf: None,
            text: None,
            closed: None,
            segments: None,
            center: None,
            radius: None,
            sides: None,
            length: None,
            width: None,
            angle: None,
            corner_radius: None,
            points: None,
            bulges: None,
        }
    }

    /// Names of the fields that are set, for "not allowed for this kind" messages.
    fn present(&self) -> Vec<&'static str> {
        [
            ("name", self.name.is_some()),
            ("construction", self.construction),
            ("dxf_handle", self.dxf_handle.is_some()),
            ("dxf_type", self.dxf_type.is_some()),
            ("image", self.image.is_some()),
            ("dxf", self.dxf.is_some()),
            ("text", self.text.is_some()),
            ("closed", self.closed.is_some()),
            ("segments", self.segments.is_some()),
            ("center", self.center.is_some()),
            ("radius", self.radius.is_some()),
            ("sides", self.sides.is_some()),
            ("length", self.length.is_some()),
            ("width", self.width.is_some()),
            ("angle", self.angle.is_some()),
            ("corner_radius", self.corner_radius.is_some()),
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
            Geometry::Polygon { .. } => ShapeKind::Polygon,
        };
        let mut raw = RawShape::empty(shape.id, shape.layer, kind);
        raw.name = shape.name;
        raw.construction = shape.construction;
        match shape.geometry {
            Geometry::Path {
                closed,
                segments,
                origin,
            } => {
                // A path always writes its handle and type, null when it has none.
                raw.dxf_handle = Some(None);
                raw.dxf_type = Some(None);
                match origin {
                    Some(Origin::Dxf { id, handle, entity }) => {
                        raw.dxf = Some(id);
                        raw.dxf_handle = Some(Some(handle));
                        raw.dxf_type = Some(Some(entity));
                    }
                    Some(Origin::Image(id)) => raw.image = Some(id),
                    Some(Origin::Text(id)) => raw.text = Some(id),
                    None => {}
                }
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
                corner_radius,
            } => {
                raw.center = Some(center);
                raw.length = Some(length);
                raw.width = Some(width);
                raw.angle = Some(angle);
                raw.corner_radius = Some(corner_radius);
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
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            } => {
                raw.center = Some(center);
                raw.radius = Some(radius);
                raw.sides = Some(sides);
                raw.angle = Some(angle);
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
            ShapeKind::Path => &[
                "dxf_handle",
                "dxf_type",
                "image",
                "dxf",
                "text",
                "closed",
                "segments",
            ],
            ShapeKind::Circle => &["name", "construction", "center", "radius"],
            ShapeKind::Rectangle => &[
                "name",
                "construction",
                "center",
                "length",
                "width",
                "angle",
                "corner_radius",
            ],
            ShapeKind::Polyline => &["name", "construction", "closed", "points", "bulges"],
            ShapeKind::Polygon => &["name", "construction", "center", "radius", "sides", "angle"],
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
            ShapeKind::Path => {
                let origin = raw.origin()?;
                Geometry::Path {
                    closed: raw.closed.ok_or_else(|| missing("closed"))?,
                    segments: raw.segments.ok_or_else(|| missing("segments"))?,
                    origin,
                }
            }
            ShapeKind::Circle => Geometry::Circle {
                center: raw.center.ok_or_else(|| missing("center"))?,
                radius: raw.radius.ok_or_else(|| missing("radius"))?,
            },
            ShapeKind::Rectangle => Geometry::Rectangle {
                center: raw.center.ok_or_else(|| missing("center"))?,
                length: raw.length.ok_or_else(|| missing("length"))?,
                width: raw.width.ok_or_else(|| missing("width"))?,
                angle: raw.angle.ok_or_else(|| missing("angle"))?,
                corner_radius: raw.corner_radius.ok_or_else(|| missing("corner_radius"))?,
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
            ShapeKind::Polygon => Geometry::Polygon {
                center: raw.center.ok_or_else(|| missing("center"))?,
                radius: raw.radius.ok_or_else(|| missing("radius"))?,
                sides: raw.sides.ok_or_else(|| missing("sides"))?,
                angle: raw.angle.ok_or_else(|| missing("angle"))?,
            },
        };
        Ok(Shape {
            id: raw.id,
            layer: raw.layer,
            name: raw.name,
            geometry,
            construction: raw.construction,
        })
    }
}

impl RawShape {
    /// A path's origin from its flat fields: one DXF with its handle and type, one picture,
    /// one text, or none. Anything else is refused (docs/specs/source-model.md §2.1).
    fn origin(&self) -> Result<Option<Origin>, String> {
        let handle = self.dxf_handle.clone().flatten();
        let entity = self.dxf_type.clone().flatten();
        let refused = |what: &str| Err(format!("shape {}: {what}", self.id));
        match (self.dxf, self.image, self.text) {
            (Some(id), None, None) => match (handle, entity) {
                (Some(handle), Some(entity)) => Ok(Some(Origin::Dxf { id, handle, entity })),
                _ => refused("a path from a DXF needs `dxf_handle` and `dxf_type`"),
            },
            (None, _, _) if handle.is_some() || entity.is_some() => {
                refused("`dxf_handle` and `dxf_type` belong to a path from a DXF (`dxf`)")
            }
            (None, Some(id), None) => Ok(Some(Origin::Image(id))),
            (None, None, Some(id)) => Ok(Some(Origin::Text(id))),
            (None, None, None) => Ok(None),
            _ => refused("a path comes from one DXF, image or text, not more"),
        }
    }

    fn from_kind_name(kind: ShapeKind) -> &'static str {
        match kind {
            ShapeKind::Path => "path",
            ShapeKind::Circle => "circle",
            ShapeKind::Rectangle => "rectangle",
            ShapeKind::Polyline => "polyline",
            ShapeKind::Polygon => "polygon",
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
    /// The primitive the stored segment describes (exact: the same f64 values).
    pub fn to_primitive(&self) -> Primitive {
        match self {
            PathSegment::Line(l) => Primitive::Line(Line {
                start: (l.start[0], l.start[1]),
                end: (l.end[0], l.end[1]),
            }),
            PathSegment::Arc(a) => Primitive::Arc(Arc {
                center: (a.center[0], a.center[1]),
                radius: a.radius,
                start_angle: a.start_angle,
                sweep: a.sweep,
            }),
        }
    }

    /// A primitive as a stored segment (exact: the same f64 values).
    pub fn from_primitive(prim: &Primitive) -> PathSegment {
        match prim {
            Primitive::Line(l) => PathSegment::Line(LineSegment {
                start: [l.start.0, l.start.1],
                end: [l.end.0, l.end.1],
            }),
            Primitive::Arc(a) => PathSegment::Arc(ArcSegment {
                center: [a.center.0, a.center.1],
                radius: a.radius,
                start_angle: a.start_angle,
                sweep: a.sweep,
            }),
        }
    }

    /// Whether the last of `segments` ends where the first starts (within the chaining
    /// tolerance).
    pub fn closes(segments: &[PathSegment]) -> bool {
        match (segments.first(), segments.last()) {
            (Some(first), Some(last)) => {
                distance(first.to_primitive().start(), last.to_primitive().end()) <= CHAIN_TOL_MM
            }
            _ => false,
        }
    }

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

    /// The segment turned about the origin by `degrees` counter-clockwise; multiples of 90
    /// as [`PathSegment::quarter_turned`], exactly. An arc's start angle is wrapped into
    /// [0, 2π).
    pub fn rotated(&self, degrees: f64) -> PathSegment {
        if let Some(k) = as_quarter_turns(degrees) {
            return self.quarter_turned(k);
        }
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: rotated(l.start, degrees),
                end: rotated(l.end, degrees),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: rotated(a.center, degrees),
                start_angle: (a.start_angle + degrees.to_radians())
                    .rem_euclid(std::f64::consts::TAU),
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

/// Value rules of one shape, as "field: message" (spec drawing-tools.md section 1).
/// What is wrong with a shape's values, as "field: message"; empty when it is valid.
pub fn shape_problems(geometry: &Geometry) -> Vec<Problem> {
    let mut problems = Vec::new();
    let finite = |p: &[f64; 2]| p.iter().all(|v| v.is_finite());
    let mut positive = |field: &str, value: f64| {
        if !(value.is_finite() && value > 0.0) {
            problems.push(Problem::at(
                field,
                Message::new("job-must-be-positive-number")
                    .arg("got", Value::Length(value, Digits::Shortest)),
            ));
        }
    };
    match geometry {
        Geometry::Path { segments, .. } => {
            if segments.is_empty() {
                problems.push(Problem::at(
                    "segments",
                    Message::new("job-path-no-segments"),
                ));
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
                    problems.push(Problem::at(
                        format!("segments.{j}"),
                        Message::new("job-segment-not-finite"),
                    ));
                }
            }
        }
        Geometry::Circle { center, radius } => {
            positive("radius", *radius);
            if !finite(center) {
                problems.push(Problem::at("center", Message::new("job-must-be-finite")));
            }
        }
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } => {
            positive("length", *length);
            positive("width", *width);
            let most = length.min(*width) / 2.0;
            if !corner_radius.is_finite() || *corner_radius < 0.0 {
                problems.push(Problem::at(
                    "corner_radius",
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::Length(*corner_radius, Digits::Shortest)),
                ));
            } else if *corner_radius > 0.0 && *corner_radius < CHAIN_TOL_MM {
                problems.push(Problem::at(
                    "corner_radius",
                    Message::new("job-corner-radius-tiny")
                        .arg("least", Value::Length(CHAIN_TOL_MM, Digits::Shortest))
                        .arg("got", Value::Length(*corner_radius, Digits::Shortest)),
                ));
            } else if *corner_radius > most {
                problems.push(Problem::at(
                    "corner_radius",
                    Message::new("job-corner-radius-too-large")
                        .arg("most", Value::Length(most, Digits::Shortest))
                        .arg("got", Value::Length(*corner_radius, Digits::Shortest)),
                ));
            }
            if !finite(center) {
                problems.push(Problem::at("center", Message::new("job-must-be-finite")));
            }
            if !angle.is_finite() {
                problems.push(Problem::at(
                    "angle",
                    Message::new("job-must-be-finite-got")
                        .arg("got", Value::Number(*angle, Digits::Shortest)),
                ));
            }
        }
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => {
            positive("radius", *radius);
            if !POLYGON_SIDES.contains(sides) {
                problems.push(Problem::at(
                    "sides",
                    Message::new("job-polygon-sides")
                        .arg("least", *POLYGON_SIDES.start())
                        .arg("most", *POLYGON_SIDES.end())
                        .arg("got", *sides),
                ));
            }
            if !finite(center) {
                problems.push(Problem::at("center", Message::new("job-must-be-finite")));
            }
            if !angle.is_finite() {
                problems.push(Problem::at(
                    "angle",
                    Message::new("job-must-be-finite-got")
                        .arg("got", Value::Number(*angle, Digits::Shortest)),
                ));
            }
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
        } => {
            // Two points close a loop when a segment bends: a DXF polyline circle of two
            // half circles (docs/specs/explode-and-join.md §2).
            let bends = bulges.iter().any(|b| *b != 0.0);
            let least = if *closed && !(bends && points.len() == 2) {
                3
            } else {
                2
            };
            if points.len() < least {
                problems.push(Problem::at(
                    "points",
                    Message::new("job-polyline-points")
                        .arg("closed", if *closed { "closed" } else { "open" })
                        .arg("least", least)
                        .arg("got", points.len()),
                ));
            }
            if !points.iter().all(finite) {
                problems.push(Problem::at("points", Message::new("job-must-be-finite")));
            }
            let n = points.len();
            let pairs = if *closed { n } else { n.saturating_sub(1) };
            for i in 0..pairs {
                let (a, b) = (points[i], points[(i + 1) % n]);
                if (a[0] - b[0]).hypot(a[1] - b[1]) <= CHAIN_TOL_MM {
                    problems.push(Problem::at(
                        format!("points.{}", (i + 1) % n),
                        Message::new("job-point-on-previous")
                            .arg("tolerance", Value::Length(CHAIN_TOL_MM, Digits::Shortest)),
                    ));
                }
            }
            if bulges.len() != pairs {
                problems.push(Problem::at(
                    "bulges",
                    Message::new("job-bulges-count")
                        .arg("wanted", pairs)
                        .arg("got", bulges.len()),
                ));
            }
            for (i, b) in bulges.iter().enumerate() {
                if !(b.is_finite() && b.abs() <= 1.0) {
                    problems.push(Problem::at(
                        format!("bulges.{i}"),
                        Message::new("job-bulge-range")
                            .arg("got", Value::Number(*b, Digits::Shortest)),
                    ));
                }
            }
        }
    }
    problems
}
