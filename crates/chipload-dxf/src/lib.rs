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
mod svg;

use std::collections::BTreeMap;
use std::path::Path;

use cavalier_contours::polyline::{PlineSource, PlineSourceMut, Polyline};
use chipload_core::Point;
use chipload_core::chain::{Contour, Segment, ShapeFrom, chain_segments};
use chipload_core::fold::fold_point;
use chipload_core::nesting;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    DrawingFormat, DrawingSource, Geometry, PathSegment, Placement, Shape, SourceIssue, Units,
};
use dxf::Drawing;
use dxf::enums::Units as DxfUnits;
use sha2::{Digest, Sha256};

pub use chipload_job::{IssueSeverity as Severity, UnitsFrom};
pub use convert::{Affine, MAX_BLOCK_DEPTH};
pub use svg::{is_svg, shapes_from_svg_bytes};

/// Millimetres per inch.
const MM_PER_INCH: f64 = 25.4;

/// Why a DXF can't be imported. Its text is a message (`dxf.ftl`), shown in the app's
/// language; `Display` writes it in English for the command line.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    /// `reason` is the parser's or the file system's own words, or a message.
    Unreadable {
        path: String,
        reason: Value,
    },
    UnitsMissing,
    UnsupportedUnits(i32),
    UnitsMismatch {
        dxf: &'static str,
        expected: &'static str,
    },
    /// An SVG without a real size: its pixels per inch must be chosen
    /// (docs/specs/svg-import.md §7).
    ScaleMissing,
}

impl ImportError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            ImportError::Unreadable { path, reason } => Message::new("dxf-error-unreadable")
                .arg("path", path)
                .arg("reason", reason.clone()),
            ImportError::UnitsMissing => Message::new("dxf-error-units-missing"),
            ImportError::UnsupportedUnits(code) => Message::new("dxf-error-units-unsupported")
                .arg("code", Value::Text(code.to_string())),
            ImportError::UnitsMismatch { dxf, expected } => {
                Message::new("dxf-error-units-mismatch")
                    .arg("dxf", *dxf)
                    .arg("expected", *expected)
            }
            ImportError::ScaleMissing => Message::new("svg-error-needs-scale"),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for ImportError {}

/// `message` is `text_message` in English and mm, for the command line and `--json`; the
/// app shows `text_message` in its language and units (docs/specs/translations.md §7).
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub text_message: Message,
    pub location: Option<Point>,
}

impl Issue {
    pub fn new(severity: Severity, code: &str, message: Message) -> Self {
        Issue {
            severity,
            code: code.to_string(),
            message: message.english(),
            text_message: message,
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
    /// The first drawing file's format: where `units_from` says the units came from.
    pub format: DrawingFormat,
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
        std::slice::from_ref(&imported.source),
    ))
}

pub fn import_drawing(
    drawing: &Drawing,
    units: Option<Units>,
) -> Result<ImportResult, ImportError> {
    let imported = shapes_from_drawing(drawing, units)?;
    Ok(contours_from_shapes(
        &imported.shapes,
        std::slice::from_ref(&imported.source),
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
        reason: Value::Text(err.to_string()),
    })?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    shapes_from_dxf_bytes(&bytes, &name, units, scale)
}

/// A drawing file, DXF or SVG by its extension, read into shapes at `scale` (see
/// [`shapes_from_dxf_scaled`]). `units` is for a DXF without units, `px_per_inch` for an SVG
/// without a real size; each is ignored by the other format.
pub fn shapes_from_file(
    path: &Path,
    units: Option<Units>,
    px_per_inch: Option<f64>,
    scale: f64,
) -> Result<ImportedShapes, ImportError> {
    if !is_svg(path) {
        return shapes_from_dxf_scaled(path, units, scale);
    }
    let bytes = std::fs::read(path).map_err(|err| ImportError::Unreadable {
        path: path.display().to_string(),
        reason: Value::Text(err.to_string()),
    })?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    shapes_from_svg_bytes(&bytes, &name, px_per_inch, scale)
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
    let unreadable = |reason: Value| ImportError::Unreadable {
        path: name.to_string(),
        reason,
    };
    let cleaned = sanitize::strip_unused(bytes);
    // The parser is third-party code fed hostile input: a panic in it becomes an import
    // error instead of taking the application down.
    let parsed = std::panic::catch_unwind(|| Drawing::load(&mut cleaned.as_slice()))
        .map_err(|_| unreadable(Message::new("dxf-error-parser-crashed").into()))?;
    let drawing = parsed.map_err(|err| unreadable(Value::Text(err.to_string())))?;
    let mut imported = shapes_from_drawing_scaled(&drawing, units, scale)?;
    imported.source.file = name.to_string();
    imported.source.name = chipload_job::migrations::dxf_name(name);
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
            reason: Message::new("dxf-error-scale")
                .arg("scale", Value::Number(scale, Digits::Shortest))
                .into(),
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
            units_chosen(file_units),
        ));
    }
    if converter.nonzero_z > 0 {
        issues.push(Issue::new(
            Severity::Warning,
            "nonzero_z",
            Message::new("dxf-nonzero-z")
                .arg("count", converter.nonzero_z)
                .arg("max", Value::Length(converter.max_abs_z, Digits::Fixed(3))),
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
            last.3.push(PathSegment::from_primitive(&segment.prim));
        }
    }
    let shapes: Vec<Shape> = entities
        .into_iter()
        .enumerate()
        .map(|(i, (layer, handle, dxf_type, segments))| Shape {
            id: i as u64 + 1,
            layer,
            name: None,
            geometry: Geometry::Path {
                dxf_handle: Some(handle),
                dxf_type: Some(dxf_type),
                closed: PathSegment::closes(&segments),
                segments,
                image: None,
                dxf: Some(1),
                text: None,
            },
        })
        .collect();
    let source = DrawingSource {
        id: 1,
        format: DrawingFormat::Dxf,
        name: String::new(),
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
        px_per_inch: None,
        placement: Placement::default(),
    };
    Ok(ImportedShapes { shapes, source })
}

/// Contours and the import report for a project's shapes: the DXFs' own issues first (named
/// by their DXF when there are several), then the chaining and classification issues
/// recomputed from the shapes. Units are the first DXF's; counts add up over all of them.
pub fn contours_from_shapes(shapes: &[Shape], sources: &[DrawingSource]) -> ImportResult {
    let segments: Vec<Segment> = shapes
        .iter()
        .flat_map(|shape| {
            let handle = shape
                .dxf_handle()
                .map_or_else(|| format!("S{}", shape.id), str::to_string);
            let source_type = shape
                .dxf_type()
                .map_or_else(|| shape.kind_name().to_uppercase(), str::to_string);
            let from = match (shape.dxf(), shape.image(), shape.text()) {
                (Some(dxf), _, _) => Some(ShapeFrom::Dxf(dxf)),
                (None, Some(image), _) => Some(ShapeFrom::Image(image)),
                (None, None, Some(text)) => Some(ShapeFrom::Text(text)),
                (None, None, None) => None,
            };
            shape.segments().into_iter().map(move |seg| Segment {
                prim: seg.to_primitive(),
                layer: shape.layer.clone(),
                handle: handle.clone(),
                source_type: source_type.clone(),
                shape: Some(shape.id),
                from,
            })
        })
        .collect();
    let several = sources.len() > 1;
    let mut issues: Vec<Issue> = sources
        .iter()
        .flat_map(|s| {
            s.issues.iter().map(move |i| {
                let message = svg::issue_message(&i.code, s)
                    .filter(|_| s.format == DrawingFormat::Svg)
                    .unwrap_or_else(|| stored_issue(i, s.units));
                let message = if several {
                    Message::new("dxf-source-issue")
                        .arg("source", s.name.as_str())
                        .arg("issue", message)
                } else {
                    message
                };
                Issue::new(i.severity, &i.code, message)
            })
        })
        .collect();

    let chained = chain_segments(&segments);
    for seg in &chained.zero_length {
        issues.push(
            Issue::new(
                Severity::Warning,
                "zero_length",
                describe(Message::new("dxf-zero-length"), seg),
            )
            .at(seg.prim.start()),
        );
    }
    for seg in &chained.duplicates {
        issues.push(
            Issue::new(
                Severity::Warning,
                "duplicate",
                describe(Message::new("dxf-duplicate"), seg),
            )
            .at(seg.prim.start()),
        );
    }
    for point in &chained.branch_points {
        issues.push(
            Issue::new(
                Severity::Warning,
                "branch_point",
                Message::new("dxf-branch-point"),
            )
            .at(*point),
        );
    }

    let contours = classify(chained.contours, &mut issues);
    let first = sources.first();
    let summed = |counts: fn(&DrawingSource) -> &BTreeMap<String, usize>| {
        let mut total: BTreeMap<String, usize> = BTreeMap::new();
        for source in sources {
            for (kind, n) in counts(source) {
                *total.entry(kind.clone()).or_default() += n;
            }
        }
        total
    };
    let report = ImportReport {
        units: first.map_or(Units::Mm, |s| s.units),
        units_from: first.map_or(UnitsFrom::User, |s| s.units_from),
        format: first.map_or(DrawingFormat::Dxf, |s| s.format),
        bbox: bbox(&segments),
        imported: summed(|s| &s.imported),
        flattened: summed(|s| &s.flattened),
        ignored: summed(|s| &s.ignored),
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
                    Message::new("dxf-open-contour")
                        .arg("contour", contour.id)
                        .arg("layers", contour.layers().join(", "))
                        .arg("start", Value::Point(s.0, s.1, Digits::Fixed(3)))
                        .arg("end", Value::Point(e.0, e.1, Digits::Fixed(3))),
                )
                .at(s),
            );
            continue;
        }
        let points = contour.points();
        match ring(&points) {
            Ok(pline) => rings.push((index, nesting::Ring { points, pline })),
            Err(reason) => {
                contour.invalid_reason = Some(reason.english());
                issues.push(
                    Issue::new(
                        Severity::Error,
                        "invalid_contour",
                        Message::new("dxf-invalid-contour")
                            .arg("contour", contour.id)
                            .arg("reason", reason),
                    )
                    .at(contour.start()),
                );
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
fn ring(points: &[Point]) -> Result<Polyline<f64>, Message> {
    if points.len() < 3 {
        return Err(Message::new("dxf-ring-few-points"));
    }
    let mut pline = Polyline::new_closed();
    for &(x, y) in points {
        pline.add(x, y, 0.0);
    }
    if pline.scan_for_self_intersect() {
        return Err(Message::new("dxf-ring-self-intersecting"));
    }
    if let Some((x, y)) = fold_point(points) {
        return Err(Message::new("dxf-ring-folds")
            .arg("within", Value::Length(ARC_APPROX_TOL_MM, Digits::Fixed(3)))
            .arg("x", Value::LengthNumber(x, Digits::Fixed(3)))
            .arg("y", Value::LengthNumber(y, Digits::Fixed(3))));
    }
    if pline.area().abs() < CHAIN_TOL_MM * CHAIN_TOL_MM {
        return Err(Message::new("dxf-ring-no-area"));
    }
    Ok(pline)
}

/// A message about one segment: its DXF type, layer and handle.
fn describe(message: Message, segment: &Segment) -> Message {
    message
        .arg("type", segment.source_type.as_str())
        .arg("layer", segment.layer.as_str())
        .arg("handle", segment.handle.as_str())
}

/// The issue a DXF's units were chosen for it.
fn units_chosen(units: Units) -> Message {
    Message::new("dxf-units-chosen").arg("units", units_name(units))
}

/// A message for an issue the project file keeps from the DXF's import. The file keeps its
/// words, in English, not the message: those words are shown as they are, except where
/// they are a message's English exactly.
fn stored_issue(issue: &SourceIssue, units: Units) -> Message {
    let chosen = units_chosen(units);
    if issue.code == "units_chosen" && chosen.english() == issue.message {
        return chosen;
    }
    Message::new("dxf-stored-issue").arg("text", issue.message.as_str())
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
