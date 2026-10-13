// SPDX-License-Identifier: GPL-3.0-or-later
//! DXF import: read -> Line/Arc primitives in mm -> shapes (one per entity) -> chain ->
//! classify -> nest -> report.
//!
//! The drawing is read as drawn, with as little repair as possible, and nothing is dropped
//! without an entry in the report. See ADR 0005 for what is converted and how.
//!
//! A project stores the shapes (docs/specs/project-file.md); [`contours_from_shapes`] turns
//! them into contours with the same chaining and report as a DXF import. [`import_dxf`]
//! itself goes DXF -> shapes -> contours, so inspecting a DXF and loading its project can't
//! differ.

#![forbid(unsafe_code)]

mod convert;
mod curves;
mod sanitize;

use std::collections::BTreeMap;
use std::path::Path;

use cavalier_contours::polyline::{PlineSource, PlineSourceMut, Polyline};
use chipload_core::Point;
use chipload_core::chain::{Contour, Segment, chain_segments};
use chipload_core::fold::fold_point;
use chipload_core::nesting;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_job::{
    ArcSegment, DrawingSource, Geometry, LineSegment, PathSegment, Placement, Shape, SourceIssue,
    Units,
};
use dxf::Drawing;
use dxf::enums::Units as DxfUnits;
use sha2::{Digest, Sha256};

pub use chipload_job::{IssueSeverity as Severity, UnitsFrom};
pub use convert::{Affine, MAX_BLOCK_DEPTH};

/// Millimetres per inch.
const MM_PER_INCH: f64 = 25.4;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ImportError {
    #[error("Cannot read DXF {path}: {reason}")]
    Unreadable { path: String, reason: String },
    #[error("DXF has no units ($INSUNITS=0); specify mm or inch explicitly")]
    UnitsMissing,
    #[error(
        "DXF $INSUNITS={0} is not supported (only mm, inch or unitless). \
         Re-export the drawing in mm or inch."
    )]
    UnsupportedUnits(i32),
    #[error(
        "DXF declares {dxf} but {expected} was expected. Fix the drawing or choose the units \
         explicitly; units are never guessed."
    )]
    UnitsMismatch {
        dxf: &'static str,
        expected: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub location: Option<Point>,
}

impl Issue {
    pub fn new(severity: Severity, code: &str, message: String) -> Self {
        Issue {
            severity,
            code: code.to_string(),
            message,
            location: None,
        }
    }

    fn at(mut self, location: Point) -> Self {
        self.location = Some(location);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportReport {
    pub units: Units,
    pub units_from: UnitsFrom,
    /// DXF type -> count, sorted by type.
    pub imported: BTreeMap<String, usize>,
    /// Splines and ellipses approximated by lines.
    pub flattened: BTreeMap<String, usize>,
    /// Entity types not machined.
    pub ignored: BTreeMap<String, usize>,
    pub issues: Vec<Issue>,
    /// (min, max) of all imported geometry, mm.
    pub bbox: Option<(Point, Point)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportResult {
    pub contours: Vec<Contour>,
    pub report: ImportReport,
}

/// A DXF read into shapes (ids 1, 2, … in import order), plus what only the import knows.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedShapes {
    pub shapes: Vec<Shape>,
    pub source: DrawingSource,
}

pub fn units_name(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm",
        Units::Inch => "inch",
    }
}

/// Lowercase hex sha256, as stored in a project's `drawing.source.sha256`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Import a DXF file as contours (shapes are an internal step; see the module docs).
///
/// `units` must agree with $INSUNITS if the file declares it; a unitless file needs them.
pub fn import_dxf(path: &Path, units: Option<Units>) -> Result<ImportResult, ImportError> {
    let imported = shapes_from_dxf(path, units)?;
    Ok(contours_from_shapes(
        &imported.shapes,
        Some(&imported.source),
    ))
}

pub fn import_drawing(
    drawing: &Drawing,
    units: Option<Units>,
) -> Result<ImportResult, ImportError> {
    let imported = shapes_from_drawing(drawing, units)?;
    Ok(contours_from_shapes(
        &imported.shapes,
        Some(&imported.source),
    ))
}

/// Read a DXF file into shapes; `source` records its name and sha256.
pub fn shapes_from_dxf(path: &Path, units: Option<Units>) -> Result<ImportedShapes, ImportError> {
    shapes_from_dxf_scaled(path, units, 1.0)
}

/// [`shapes_from_dxf`], with every shape scaled about the DXF origin by `scale` (> 0) and
/// splines, ellipses and distorted arcs approximated within the tolerance at that size
/// (docs/specs/drawing-scale.md).
pub fn shapes_from_dxf_scaled(
    path: &Path,
    units: Option<Units>,
    scale: f64,
) -> Result<ImportedShapes, ImportError> {
    let bytes = std::fs::read(path).map_err(|err| ImportError::Unreadable {
        path: path.display().to_string(),
        reason: err.to_string(),
    })?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    shapes_from_dxf_bytes(&bytes, &name, units, scale)
}

/// [`shapes_from_dxf_scaled`] for a DXF already read into memory; `name` is the file name
/// recorded in the source and used in errors. Also the entry point of the fuzzer
/// (fuzz/fuzz_targets/dxf_import.rs).
pub fn shapes_from_dxf_bytes(
    bytes: &[u8],
    name: &str,
    units: Option<Units>,
    scale: f64,
) -> Result<ImportedShapes, ImportError> {
    let unreadable = |reason: String| ImportError::Unreadable {
        path: name.to_string(),
        reason,
    };
    let cleaned = sanitize::strip_unused(bytes);
    // The parser is third-party code fed hostile input: a panic in it becomes an import
    // error instead of taking the application down.
    let parsed = std::panic::catch_unwind(|| Drawing::load(&mut cleaned.as_slice()))
        .map_err(|_| unreadable("the DXF parser crashed on this file".into()))?;
    let drawing = parsed.map_err(|err| unreadable(err.to_string()))?;
    let mut imported = shapes_from_drawing_scaled(&drawing, units, scale)?;
    imported.source.file = name.to_string();
    imported.source.sha256 = sha256_hex(bytes);
    Ok(imported)
}

/// Convert a parsed drawing into shapes, one per entity that produced geometry.
pub fn shapes_from_drawing(
    drawing: &Drawing,
    units: Option<Units>,
) -> Result<ImportedShapes, ImportError> {
    shapes_from_drawing_scaled(drawing, units, 1.0)
}

/// [`shapes_from_drawing`] at `scale` (see [`shapes_from_dxf_scaled`]).
pub fn shapes_from_drawing_scaled(
    drawing: &Drawing,
    units: Option<Units>,
    scale: f64,
) -> Result<ImportedShapes, ImportError> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(ImportError::Unreadable {
            path: String::new(),
            reason: format!("can't import at scale {scale}"),
        });
    }
    let (file_units, units_from) = resolve_units(drawing, units)?;
    let to_mm = match file_units {
        Units::Mm => 1.0,
        Units::Inch => MM_PER_INCH,
    };
    let mut converter = convert::Converter::new(drawing, to_mm, scale);
    converter.convert_modelspace();

    let mut issues = std::mem::take(&mut converter.issues);
    if units_from == UnitsFrom::User {
        issues.push(Issue::new(
            Severity::Warning,
            "units_chosen",
            format!(
                "DXF has no units; using {} as chosen",
                units_name(file_units)
            ),
        ));
    }
    if converter.nonzero_z > 0 {
        issues.push(Issue::new(
            Severity::Warning,
            "nonzero_z",
            format!(
                "{} entities have nonzero Z (max |Z| {:.3} mm); Z was dropped",
                converter.nonzero_z, converter.max_abs_z
            ),
        ));
    }

    // (layer, handle, type, segments) per DXF entity, in import order.
    let mut entities: Vec<(String, String, String, Vec<PathSegment>)> = Vec::new();
    for (segment, &entity) in converter.segments.iter().zip(&converter.entity_of) {
        if entities.len() <= entity {
            entities.push((
                segment.layer.clone(),
                segment.handle.clone(),
                segment.source_type.clone(),
                Vec::new(),
            ));
        }
        if let Some(last) = entities.last_mut() {
            last.3.push(path_segment(&segment.prim));
        }
    }
    let shapes: Vec<Shape> = entities
        .into_iter()
        .enumerate()
        .map(|(i, (layer, handle, dxf_type, segments))| Shape {
            id: i as u64 + 1,
            layer,
            geometry: Geometry::Path {
                dxf_handle: Some(handle),
                dxf_type: Some(dxf_type),
                closed: is_closed(&segments),
                segments,
            },
        })
        .collect();
    let source = DrawingSource {
        file: String::new(),
        sha256: String::new(),
        units: file_units,
        units_from,
        imported: converter.imported,
        flattened: converter.flattened,
        ignored: converter.ignored,
        issues: issues
            .into_iter()
            .map(|i| SourceIssue {
                severity: i.severity,
                code: i.code,
                message: i.message,
            })
            .collect(),
        placement: Placement::default(),
    };
    Ok(ImportedShapes { shapes, source })
}

/// The primitive a stored path segment describes.
pub fn primitive(segment: &PathSegment) -> Primitive {
    match segment {
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

/// A primitive as a stored path segment (exact: same f64 values).
pub fn path_segment(prim: &Primitive) -> PathSegment {
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

/// The last segment ends where the first starts.
pub fn is_closed(segments: &[PathSegment]) -> bool {
    match (segments.first(), segments.last()) {
        (Some(first), Some(last)) => {
            distance(primitive(first).start(), primitive(last).end()) <= CHAIN_TOL_MM
        }
        _ => false,
    }
}

/// Contours and the import report for a project's shapes: `source.issues` first, then the
/// chaining and classification issues recomputed from the shapes.
pub fn contours_from_shapes(shapes: &[Shape], source: Option<&DrawingSource>) -> ImportResult {
    let segments: Vec<Segment> = shapes
        .iter()
        .flat_map(|shape| {
            let handle = shape
                .dxf_handle()
                .map_or_else(|| format!("S{}", shape.id), str::to_string);
            let source_type = shape
                .dxf_type()
                .map_or_else(|| shape.kind_name().to_uppercase(), str::to_string);
            shape.segments().into_iter().map(move |seg| Segment {
                prim: primitive(&seg),
                layer: shape.layer.clone(),
                handle: handle.clone(),
                source_type: source_type.clone(),
                shape: Some(shape.id),
            })
        })
        .collect();
    let mut issues: Vec<Issue> = source
        .map(|s| {
            s.issues
                .iter()
                .map(|i| Issue::new(i.severity, &i.code, i.message.clone()))
                .collect()
        })
        .unwrap_or_default();

    let chained = chain_segments(&segments);
    for seg in &chained.zero_length {
        issues.push(
            Issue::new(
                Severity::Warning,
                "zero_length",
                format!("Removed zero-length {}", describe(seg)),
            )
            .at(seg.prim.start()),
        );
    }
    for seg in &chained.duplicates {
        issues.push(
            Issue::new(
                Severity::Warning,
                "duplicate",
                format!("Removed duplicate {}", describe(seg)),
            )
            .at(seg.prim.start()),
        );
    }
    for point in &chained.branch_points {
        issues.push(
            Issue::new(
                Severity::Warning,
                "branch_point",
                "More than two segments meet at one point".into(),
            )
            .at(*point),
        );
    }

    let contours = classify(chained.contours, &mut issues);
    let report = ImportReport {
        units: source.map_or(Units::Mm, |s| s.units),
        units_from: source.map_or(UnitsFrom::User, |s| s.units_from),
        bbox: bbox(&segments),
        imported: source.map(|s| s.imported.clone()).unwrap_or_default(),
        flattened: source.map(|s| s.flattened.clone()).unwrap_or_default(),
        ignored: source.map(|s| s.ignored.clone()).unwrap_or_default(),
        issues,
    };
    ImportResult { contours, report }
}

fn resolve_units(
    drawing: &Drawing,
    units: Option<Units>,
) -> Result<(Units, UnitsFrom), ImportError> {
    let declared = match drawing.header.default_drawing_units {
        DxfUnits::Unitless => None,
        DxfUnits::Millimeters => Some(Units::Mm),
        DxfUnits::Inches => Some(Units::Inch),
        other => return Err(ImportError::UnsupportedUnits(other as i32)),
    };
    match (declared, units) {
        (None, None) => Err(ImportError::UnitsMissing),
        (None, Some(chosen)) => Ok((chosen, UnitsFrom::User)),
        (Some(dxf), Some(expected)) if dxf != expected => Err(ImportError::UnitsMismatch {
            dxf: units_name(dxf),
            expected: units_name(expected),
        }),
        (Some(dxf), _) => Ok((dxf, UnitsFrom::Dxf)),
    }
}

/// Mark invalid closed contours, compute nesting depth, report open contours.
fn classify(mut contours: Vec<Contour>, issues: &mut Vec<Issue>) -> Vec<Contour> {
    let mut rings: Vec<(usize, nesting::Ring)> = Vec::new();
    for (index, contour) in contours.iter_mut().enumerate() {
        if !contour.closed {
            let (s, e) = (contour.start(), contour.end());
            issues.push(
                Issue::new(
                    Severity::Warning,
                    "open_contour",
                    format!(
                        "Contour {} on {} is open: ends at ({:.3}, {:.3}) and ({:.3}, {:.3})",
                        contour.id,
                        contour.layers().join(", "),
                        s.0,
                        s.1,
                        e.0,
                        e.1
                    ),
                )
                .at(s),
            );
            continue;
        }
        let points = contour.points();
        match ring(&points) {
            Ok(pline) => rings.push((index, nesting::Ring { points, pline })),
            Err(reason) => {
                issues.push(
                    Issue::new(
                        Severity::Error,
                        "invalid_contour",
                        format!("Contour {}: {reason}", contour.id),
                    )
                    .at(contour.start()),
                );
                contour.invalid_reason = Some(reason);
            }
        }
    }
    for (index, inner) in &rings {
        let depth = rings
            .iter()
            .filter(|(other, outer)| other != index && nesting::contains(outer, inner))
            .count();
        contours[*index].nesting_depth = Some(depth);
    }
    contours
}

/// A closed polyline through the points, or why it can't be machined.
fn ring(points: &[Point]) -> Result<Polyline<f64>, String> {
    if points.len() < 3 {
        return Err("fewer than 3 distinct points".into());
    }
    let mut pline = Polyline::new_closed();
    for &(x, y) in points {
        pline.add(x, y, 0.0);
    }
    if pline.scan_for_self_intersect() {
        return Err("self-intersecting".into());
    }
    if let Some((x, y)) = fold_point(points) {
        return Err(format!(
            "folds back on itself: its edges come within {ARC_APPROX_TOL_MM:.3} mm of each \
             other near ({x:.3}, {y:.3})"
        ));
    }
    if pline.area().abs() < CHAIN_TOL_MM * CHAIN_TOL_MM {
        return Err("encloses no area".into());
    }
    Ok(pline)
}

fn describe(segment: &Segment) -> String {
    format!(
        "{} on layer {} ({})",
        segment.source_type, segment.layer, segment.handle
    )
}

fn bbox(segments: &[Segment]) -> Option<(Point, Point)> {
    let mut points = segments.iter().flat_map(|s| s.prim.points());
    let first = points.next()?;
    Some(points.fold((first, first), |(lo, hi), p| {
        (
            (lo.0.min(p.0), lo.1.min(p.1)),
            (hi.0.max(p.0), hi.1.max(p.1)),
        )
    }))
}
