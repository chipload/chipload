// SPDX-License-Identifier: GPL-3.0-or-later
//! DXF entities to Line/Arc segments in millimetres.
//!
//! The drawing is taken as it is: lines, arcs and bulges come through exactly, only splines
//! and ellipses are approximated. Everything is mapped through one 2D affine transform per
//! entity (units, a mirrored extrusion, block inserts). Entities that don't lie in the XY
//! plane are skipped and reported, never projected.

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::chain::Segment;
use chipload_core::primitives::{Arc, Line, Primitive, max_segment_angle};
use chipload_core::tolerances::{ANGLE_TOL_RAD, ARC_APPROX_TOL_MM, CHAIN_TOL_MM, Z_NONZERO_TOL_MM};
use dxf::entities::{Entity, EntityType};
use dxf::{Drawing, Point as DxfPoint, Vector};

use crate::curves::{Ellipse, Spline, Vec3};
use crate::{Issue, Severity};
use chipload_i18n::{Digits, Message, Value};

/// Guards against hostile, self-referencing or absurdly deep block nesting.
pub const MAX_BLOCK_DEPTH: usize = 32;
/// The largest coordinate, radius or extent an entity may have, mm (1 km): far beyond any
/// machine, and far inside what the contour joining's grid of `CHAIN_TOL_MM` cells can index.
/// An entity past it, or with a value that isn't a number (`inf`, `NaN`), is skipped and
/// reported; a hostile DXF with a 127-digit coordinate made the joining overflow (found by
/// fuzzing, 2026-10-13).
pub const MAX_COORDINATE_MM: f64 = 1.0e6;

/// `p ↦ m·p + t` in the XY plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    /// Row-major 2×2 matrix.
    pub m: [[f64; 2]; 2],
    pub t: Point,
}

impl Affine {
    pub fn scale(sx: f64, sy: f64) -> Self {
        Affine {
            m: [[sx, 0.0], [0.0, sy]],
            t: (0.0, 0.0),
        }
    }

    pub fn rotation(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Affine {
            m: [[c, -s], [s, c]],
            t: (0.0, 0.0),
        }
    }

    pub fn translation(x: f64, y: f64) -> Self {
        Affine {
            m: [[1.0, 0.0], [0.0, 1.0]],
            t: (x, y),
        }
    }

    pub fn apply(&self, p: Point) -> Point {
        (
            self.m[0][0] * p.0 + self.m[0][1] * p.1 + self.t.0,
            self.m[1][0] * p.0 + self.m[1][1] * p.1 + self.t.1,
        )
    }

    /// `self ∘ inner`: apply `inner` first.
    pub fn then_inner(&self, inner: &Affine) -> Affine {
        let a = &self.m;
        let b = &inner.m;
        let m = [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ];
        Affine {
            m,
            t: self.apply(inner.t),
        }
    }

    fn det(&self) -> f64 {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]
    }

    /// (stretch, distortion): the singular values are stretch ± distortion. Distortion is 0
    /// for a similarity (rotation, uniform scale, mirror) and says how far it bends circles.
    fn conformal_split(&self) -> (f64, f64) {
        let [[a, b], [c, d]] = self.m;
        // The rotation-like part (q) and the reflection-like part (r) of the matrix.
        let q = ((a + d) / 2.0).hypot((c - b) / 2.0);
        let r = ((a - d) / 2.0).hypot((c + b) / 2.0);
        (q.max(r), q.min(r))
    }

    /// Largest factor by which a length can grow.
    pub fn max_stretch(&self) -> f64 {
        let (stretch, distortion) = self.conformal_split();
        stretch + distortion
    }
}

/// The plane an entity is drawn in, from its extrusion direction: only ±Z is accepted.
/// −Z is the mirrored object coordinate system (x flipped), common for mirrored arcs.
fn extrusion_frame(normal: &Vector) -> Option<Affine> {
    let len = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    if len == 0.0 || normal.x.hypot(normal.y) / len > ANGLE_TOL_RAD {
        return None;
    }
    Some(if normal.z < 0.0 {
        Affine::scale(-1.0, 1.0)
    } else {
        Affine::scale(1.0, 1.0)
    })
}

/// DXF type name as written in the file (the `dxf` crate keeps its own mapping private).
pub fn type_name(specific: &EntityType) -> String {
    let debug = format!("{specific:?}");
    let variant: String = debug.chars().take_while(|c| c.is_alphanumeric()).collect();
    match variant.as_str() {
        "ModelPoint" => "POINT".into(),
        "Face3D" => "3DFACE".into(),
        "Solid3D" => "3DSOLID".into(),
        "AttributeDefinition" => "ATTDEF".into(),
        "Attribute" => "ATTRIB".into(),
        "ProxyEntity" => "ACAD_PROXY_ENTITY".into(),
        v if v.ends_with("Dimension") => "DIMENSION".into(),
        v => v.to_uppercase(),
    }
}

/// Collects segments, counts and issues while walking the drawing.
pub struct Converter<'a> {
    drawing: &'a Drawing,
    /// Drawing units to millimetres.
    units: f64,
    /// The scale the shapes are made at (the placement's, docs/specs/drawing-scale.md):
    /// tolerances hold at that size.
    size: f64,
    pub segments: Vec<Segment>,
    /// For each segment, the index of the entity it came from (one shape per entity).
    pub entity_of: Vec<usize>,
    entities: usize,
    pub issues: Vec<Issue>,
    pub imported: BTreeMap<String, usize>,
    pub flattened: BTreeMap<String, usize>,
    pub ignored: BTreeMap<String, usize>,
    pub nonzero_z: usize,
    pub max_abs_z: f64,
}

/// Where an entity sits: the transform to millimetres, its top-level handle and the
/// layer of the INSERT it came from (block content on layer "0" takes that layer).
#[derive(Clone)]
struct Placement {
    to_mm: Affine,
    root_handle: String,
    parent_layer: Option<String>,
    depth: usize,
}

impl<'a> Converter<'a> {
    /// `size` scales everything about the DXF origin, before any tolerance is applied.
    pub fn new(drawing: &'a Drawing, units: f64, size: f64) -> Self {
        Converter {
            drawing,
            units,
            size,
            segments: Vec::new(),
            entity_of: Vec::new(),
            entities: 0,
            issues: Vec::new(),
            imported: BTreeMap::new(),
            flattened: BTreeMap::new(),
            ignored: BTreeMap::new(),
            nonzero_z: 0,
            max_abs_z: 0.0,
        }
    }

    pub fn convert_modelspace(&mut self) {
        let drawing = self.drawing;
        for entity in drawing.entities() {
            let place = Placement {
                to_mm: Affine::scale(self.units * self.size, self.units * self.size),
                root_handle: entity.common.handle.as_string(),
                parent_layer: None,
                depth: 0,
            };
            self.convert(entity, &place);
        }
    }

    fn convert(&mut self, entity: &Entity, place: &Placement) {
        let kind = type_name(&entity.specific);
        let mut layer = entity.common.layer.clone();
        if let Some(parent) = &place.parent_layer
            && layer == "0"
        {
            layer = parent.clone();
        }
        if let EntityType::Insert(insert) = &entity.specific {
            self.insert(entity, insert, place, &layer);
            return;
        }
        let Some(pieces) = self.pieces(entity, &kind, &place.to_mm) else {
            return;
        };
        if !pieces.prims.iter().all(in_range) {
            self.skip(
                entity,
                &kind,
                Message::new("dxf-skip-coordinate")
                    .arg("most", Value::Length(MAX_COORDINATE_MM, Digits::Shortest)),
            );
            return;
        }
        *self.imported.entry(kind.clone()).or_default() += 1;
        self.track_z(pieces.max_abs_z);
        let entity = self.entities;
        self.entities += 1;
        for prim in pieces.prims {
            self.segments.push(Segment {
                construction: false,
                prim,
                layer: layer.clone(),
                handle: place.root_handle.clone(),
                source_type: kind.clone(),
                shape: None,
                from: None,
            });
            self.entity_of.push(entity);
        }
    }

    /// Primitives in millimetres, or None if the entity was not imported.
    fn pieces(&mut self, entity: &Entity, kind: &str, to_mm: &Affine) -> Option<Pieces> {
        let mut out = Pieces::default();
        match &entity.specific {
            EntityType::Line(line) => {
                out.line(to_mm, &line.p1, &line.p2);
            }
            EntityType::Circle(circle) => {
                let frame = self.frame(entity, kind, &circle.normal, to_mm)?;
                self.positive_radius(entity, kind, circle.radius)?;
                out.arc(&frame, &circle.center, circle.radius, 0.0, TAU);
            }
            EntityType::Arc(arc) => {
                let frame = self.frame(entity, kind, &arc.normal, to_mm)?;
                self.positive_radius(entity, kind, arc.radius)?;
                let sweep = (arc.end_angle - arc.start_angle).rem_euclid(360.0);
                out.arc(
                    &frame,
                    &arc.center,
                    arc.radius,
                    arc.start_angle.to_radians(),
                    sweep.to_radians(),
                );
            }
            EntityType::LwPolyline(poly) => {
                let frame = self.frame(entity, kind, &poly.extrusion_direction, to_mm)?;
                let vertices: Vec<(f64, f64, f64)> =
                    poly.vertices.iter().map(|v| (v.x, v.y, v.bulge)).collect();
                out.bulge_polyline(&frame, &vertices, entity.common.elevation, poly.is_closed());
            }
            EntityType::Polyline(poly) => {
                if poly.is_polyface_mesh() || poly.is_3d_polygon_mesh() {
                    *self.ignored.entry("POLYLINE (mesh)".into()).or_default() += 1;
                    return None;
                }
                let vertices: Vec<&dxf::entities::Vertex> = poly.vertices().collect();
                if poly.is_3d_polyline() {
                    let mut pts: Vec<&DxfPoint> = vertices.iter().map(|v| &v.location).collect();
                    if poly.is_closed()
                        && let Some(first) = pts.first().copied()
                    {
                        pts.push(first);
                    }
                    for w in pts.windows(2) {
                        out.line(to_mm, w[0], w[1]);
                    }
                } else {
                    let frame = self.frame(entity, kind, &poly.normal, to_mm)?;
                    let verts: Vec<(f64, f64, f64)> = vertices
                        .iter()
                        .map(|v| (v.location.x, v.location.y, v.bulge))
                        .collect();
                    out.bulge_polyline(&frame, &verts, poly.location.z, poly.is_closed());
                }
            }
            EntityType::Spline(spline) => {
                let control: Vec<Vec3> = spline
                    .control_points
                    .iter()
                    .map(|p| [p.x, p.y, p.z])
                    .collect();
                if control.is_empty() {
                    self.skip(entity, kind, Message::new("dxf-skip-fit-points"));
                    return None;
                }
                let curve = Spline {
                    degree: usize::try_from(spline.degree_of_curve).unwrap_or(0),
                    knots: &spline.knot_values,
                    control: &control,
                    weights: &spline.weight_values,
                };
                if let Some(reason) = curve.invalid_reason() {
                    self.skip(entity, kind, reason);
                    return None;
                }
                out.polyline(
                    to_mm,
                    &curve.flatten(ARC_APPROX_TOL_MM / to_mm.max_stretch()),
                );
                *self.flattened.entry(kind.into()).or_default() += 1;
            }
            EntityType::Ellipse(ellipse) => {
                let n = &ellipse.normal;
                // Major axis and centre are in WCS; the minor axis is normal × major.
                if extrusion_frame(n).is_none() {
                    self.skip(entity, kind, Message::new("dxf-skip-not-flat"));
                    return None;
                }
                let side = if n.z < 0.0 { -1.0 } else { 1.0 };
                let major = &ellipse.major_axis;
                let ratio = ellipse.minor_axis_ratio;
                let curve = Ellipse {
                    center: [ellipse.center.x, ellipse.center.y, ellipse.center.z],
                    major: [major.x, major.y, 0.0],
                    minor: [-side * major.y * ratio, side * major.x * ratio, 0.0],
                    start: ellipse.start_parameter,
                    end: ellipse.end_parameter,
                };
                out.polyline(
                    to_mm,
                    &curve.flatten(ARC_APPROX_TOL_MM / to_mm.max_stretch()),
                );
                *self.flattened.entry(kind.into()).or_default() += 1;
            }
            _ => {
                *self.ignored.entry(kind.into()).or_default() += 1;
                return None;
            }
        }
        Some(out)
    }

    fn insert(
        &mut self,
        entity: &Entity,
        insert: &dxf::entities::Insert,
        place: &Placement,
        layer: &str,
    ) {
        if place.depth >= MAX_BLOCK_DEPTH {
            self.issues.push(Issue::new(
                Severity::Error,
                "block_depth",
                Message::new("dxf-block-depth").arg("most", MAX_BLOCK_DEPTH),
            ));
            return;
        }
        let drawing = self.drawing;
        let Some(block) = drawing.blocks().find(|b| b.name == insert.name) else {
            self.skip(
                entity,
                "INSERT",
                Message::new("dxf-skip-no-block").arg("block", format!("{:?}", insert.name)),
            );
            return;
        };
        let Some(ocs) = extrusion_frame(&insert.extrusion_direction) else {
            self.skip(entity, "INSERT", Message::new("dxf-skip-not-flat"));
            return;
        };
        let rotation = Affine::rotation(insert.rotation.to_radians());
        let local = rotation
            .then_inner(&Affine::scale(insert.x_scale_factor, insert.y_scale_factor))
            .then_inner(&Affine::translation(
                -block.base_point.x,
                -block.base_point.y,
            ));
        let columns = insert.column_count.max(1);
        let rows = insert.row_count.max(1);
        for row in 0..rows {
            for column in 0..columns {
                // Array offsets follow the insert's rotation.
                let offset = rotation.apply((
                    f64::from(column) * insert.column_spacing,
                    f64::from(row) * insert.row_spacing,
                ));
                let at =
                    Affine::translation(insert.location.x + offset.0, insert.location.y + offset.1);
                let child = Placement {
                    to_mm: place
                        .to_mm
                        .then_inner(&ocs)
                        .then_inner(&at)
                        .then_inner(&local),
                    root_handle: place.root_handle.clone(),
                    parent_layer: Some(layer.to_string()),
                    depth: place.depth + 1,
                };
                for child_entity in &block.entities {
                    self.convert(child_entity, &child);
                }
            }
        }
    }

    /// The entity's plane mapped to millimetres, or None (reported) if it isn't flat in XY.
    fn frame(
        &mut self,
        entity: &Entity,
        kind: &str,
        normal: &Vector,
        to_mm: &Affine,
    ) -> Option<Affine> {
        match extrusion_frame(normal) {
            Some(ocs) => Some(to_mm.then_inner(&ocs)),
            None => {
                self.skip(entity, kind, Message::new("dxf-skip-not-flat"));
                None
            }
        }
    }

    fn positive_radius(&mut self, entity: &Entity, kind: &str, radius: f64) -> Option<()> {
        if radius > 0.0 {
            return Some(());
        }
        self.skip(
            entity,
            kind,
            Message::new("dxf-skip-radius").arg("radius", Value::Number(radius, Digits::Shortest)),
        );
        None
    }

    fn skip(&mut self, entity: &Entity, kind: &str, reason: Message) {
        *self.ignored.entry(kind.into()).or_default() += 1;
        let handle = entity.common.handle.as_string();
        self.issues.push(Issue::new(
            Severity::Warning,
            "entity_skipped",
            Message::new("dxf-entity-skipped")
                .arg("kind", kind)
                .arg("handle", handle)
                .arg("reason", reason),
        ));
    }

    fn track_z(&mut self, abs_z: f64) {
        let z_mm = abs_z * self.units;
        if z_mm > Z_NONZERO_TOL_MM {
            self.nonzero_z += 1;
            self.max_abs_z = self.max_abs_z.max(z_mm);
        }
    }
}

/// Primitives of one entity, in millimetres, plus its largest |Z| in drawing units.
#[derive(Default)]
struct Pieces {
    prims: Vec<Primitive>,
    max_abs_z: f64,
}

impl Pieces {
    fn z(&mut self, z: f64) {
        self.max_abs_z = self.max_abs_z.max(z.abs());
    }

    fn line(&mut self, to_mm: &Affine, a: &DxfPoint, b: &DxfPoint) {
        self.z(a.z);
        self.z(b.z);
        self.prims.push(Primitive::Line(Line {
            start: to_mm.apply((a.x, a.y)),
            end: to_mm.apply((b.x, b.y)),
        }));
    }

    fn polyline(&mut self, to_mm: &Affine, points: &[Vec3]) {
        for p in points {
            self.z(p[2]);
        }
        for w in points.windows(2) {
            self.prims.push(Primitive::Line(Line {
                start: to_mm.apply((w[0][0], w[0][1])),
                end: to_mm.apply((w[1][0], w[1][1])),
            }));
        }
    }

    fn arc(&mut self, frame: &Affine, center: &DxfPoint, radius: f64, start: f64, sweep: f64) {
        self.z(center.z);
        let arc = Arc {
            center: (center.x, center.y),
            radius,
            start_angle: start,
            sweep,
        };
        self.prims.extend(map_arc(frame, &arc));
    }

    /// LWPOLYLINE / 2D POLYLINE vertices (x, y, bulge) at `elevation`.
    fn bulge_polyline(
        &mut self,
        frame: &Affine,
        vertices: &[(f64, f64, f64)],
        elevation: f64,
        closed: bool,
    ) {
        self.z(elevation);
        let n = vertices.len();
        let count = if closed { n } else { n.saturating_sub(1) };
        for i in 0..count {
            let (x1, y1, bulge) = vertices[i];
            let (x2, y2, _) = vertices[(i + 1) % n];
            match Arc::from_bulge((x1, y1), (x2, y2), bulge) {
                Some(arc) => self.prims.extend(map_arc(frame, &arc)),
                None => self.prims.push(Primitive::Line(Line {
                    start: frame.apply((x1, y1)),
                    end: frame.apply((x2, y2)),
                })),
            }
        }
    }
}

/// An arc under an affine map. A similarity keeps it an arc (a mirror reverses the sweep);
/// anything that would distort the circle by more than the chaining tolerance turns it into
/// lines within the arc approximation tolerance.
fn map_arc(frame: &Affine, arc: &Arc) -> Vec<Primitive> {
    let (stretch, distortion) = frame.conformal_split();
    if arc.radius * distortion <= CHAIN_TOL_MM {
        let center = frame.apply(arc.center);
        let start = frame.apply(arc.point_at(arc.start_angle));
        let sweep = if frame.det() < 0.0 {
            -arc.sweep
        } else {
            arc.sweep
        };
        return vec![Primitive::Arc(Arc {
            center,
            radius: arc.radius * stretch,
            start_angle: (start.1 - center.1).atan2(start.0 - center.0),
            sweep,
        })];
    }
    let tol = ARC_APPROX_TOL_MM / frame.max_stretch();
    let pieces = (arc.sweep.abs() / max_segment_angle(arc.radius, tol))
        .ceil()
        .max(1.0);
    let step = arc.sweep / pieces;
    // `pieces` is a small positive whole number (sweep ≤ 2π).
    let points: Vec<Point> = (0..=pieces as usize)
        .map(|i| frame.apply(arc.point_at(arc.start_angle + step * i as f64)))
        .collect();
    points
        .windows(2)
        .map(|w| {
            Primitive::Line(Line {
                start: w[0],
                end: w[1],
            })
        })
        .collect()
}

/// Every number of `prim` is finite and within [`MAX_COORDINATE_MM`].
fn in_range(prim: &Primitive) -> bool {
    let numbers = match prim {
        Primitive::Line(l) => vec![l.start.0, l.start.1, l.end.0, l.end.1],
        Primitive::Arc(a) => vec![a.center.0, a.center.1, a.radius, a.start_angle, a.sweep],
    };
    numbers
        .iter()
        .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE_MM)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn close(a: Point, b: Point) -> bool {
        (a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() < EPS
    }

    #[test]
    fn bulge_one_is_a_ccw_half_circle_below_the_chord() {
        let arc = Arc::from_bulge((0.0, 0.0), (10.0, 0.0), 1.0).unwrap();
        assert!(close(arc.center, (5.0, 0.0)));
        assert!((arc.radius - 5.0).abs() < EPS);
        let p = Primitive::Arc(arc);
        assert!(close(p.start(), (0.0, 0.0)) && close(p.end(), (10.0, 0.0)));
        assert!(p.midpoint().1 < 0.0);
    }

    #[test]
    fn negative_bulge_is_clockwise_and_bigger_bulge_is_the_long_way() {
        let cw = Primitive::Arc(Arc::from_bulge((0.0, 0.0), (10.0, 0.0), -0.5).unwrap());
        assert!(close(cw.end(), (10.0, 0.0)) && cw.midpoint().1 > 0.0);
        let long = Arc::from_bulge((0.0, 0.0), (10.0, 0.0), 2.0).unwrap();
        assert!(long.sweep > std::f64::consts::PI);
        assert!(close(Primitive::Arc(long).end(), (10.0, 0.0)));
    }

    #[test]
    fn mirror_keeps_an_arc_and_reverses_its_sweep() {
        let arc = Arc {
            center: (2.0, 1.0),
            radius: 3.0,
            start_angle: 0.0,
            sweep: 1.0,
        };
        let mapped = map_arc(&Affine::scale(-1.0, 1.0), &arc);
        let [Primitive::Arc(m)] = mapped.as_slice() else {
            panic!("expected one arc")
        };
        assert!(m.sweep < 0.0);
        let before = Primitive::Arc(arc);
        let after = Primitive::Arc(*m);
        assert!(close(after.start(), (-before.start().0, before.start().1)));
        assert!(close(after.end(), (-before.end().0, before.end().1)));
    }

    #[test]
    fn non_uniform_scale_flattens_within_tolerance() {
        let arc = Arc {
            center: (0.0, 0.0),
            radius: 10.0,
            start_angle: 0.0,
            sweep: TAU,
        };
        let frame = Affine::scale(2.0, 1.0);
        let lines = map_arc(&frame, &arc);
        assert!(lines.len() > 8);
        for prim in &lines {
            // Chord midpoints lie within tolerance inside the ellipse x²/400 + y²/100 = 1.
            let (x, y) = prim.midpoint();
            let level = (x * x / 400.0 + y * y / 100.0).sqrt();
            assert!(level <= 1.0 && (1.0 - level) * 10.0 < ARC_APPROX_TOL_MM);
        }
    }

    #[test]
    fn composition_applies_the_inner_transform_first() {
        let t = Affine::translation(5.0, 0.0)
            .then_inner(&Affine::rotation(std::f64::consts::FRAC_PI_2));
        assert!(close(t.apply((1.0, 0.0)), (5.0, 1.0)));
    }

    #[test]
    fn tilted_extrusion_is_refused() {
        assert!(extrusion_frame(&Vector::new(0.0, 0.0, -1.0)).is_some());
        assert!(extrusion_frame(&Vector::new(0.0, 0.1, 1.0)).is_none());
    }
}
