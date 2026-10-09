// SPDX-License-Identifier: GPL-3.0-or-later
//! DXF import: read -> Line/Arc primitives in mm -> chain -> classify -> nest -> report.
//!
//! Counterpart of `src/chipload/core/io/dxf_import.py`. The drawing is read as drawn, with
//! as little repair as possible, and nothing is dropped without an entry in the report.
//! Results match Python structurally (contours, layers, nesting, issue codes); flattened
//! splines and ellipses match within tolerance, not point for point (ADR 0005).

#![forbid(unsafe_code)]

mod convert;
mod curves;
mod nesting;
mod sanitize;

use std::collections::BTreeMap;
use std::path::Path;

use cavalier_contours::polyline::{PlineSource, PlineSourceMut, Polyline};
use chipload_core::Point;
use chipload_core::chain::{Contour, Segment, chain_segments};
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_job::Units;
use dxf::Drawing;
use dxf::enums::Units as DxfUnits;

pub use convert::{Affine, MAX_BLOCK_DEPTH};

/// Millimetres per inch.
const MM_PER_INCH: f64 = 25.4;

#[derive(Debug, thiserror::Error)]
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
        "DXF declares {dxf} but the job says {job}. Fix the job or the drawing; units are never guessed."
    )]
    UnitsMismatch {
        dxf: &'static str,
        job: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub location: Option<Point>,
}

impl Issue {
    pub fn new(severity: Severity, code: &'static str, message: String) -> Self {
        Issue {
            severity,
            code,
            message,
            location: None,
        }
    }

    fn at(mut self, location: Point) -> Self {
        self.location = Some(location);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitsFrom {
    Dxf,
    Job,
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

pub fn units_name(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm",
        Units::Inch => "inch",
    }
}

/// Import a DXF file.
///
/// `units` normally comes from the job and must agree with $INSUNITS if the file declares
/// it. None means "use the file's units"; a unitless file is then refused.
pub fn import_dxf(path: &Path, units: Option<Units>) -> Result<ImportResult, ImportError> {
    let unreadable = |reason: String| ImportError::Unreadable {
        path: path.display().to_string(),
        reason,
    };
    let bytes = std::fs::read(path).map_err(|err| unreadable(err.to_string()))?;
    let cleaned = sanitize::strip_unused(&bytes);
    // The parser is third-party code fed hostile input: a panic in it becomes an import
    // error instead of taking the application down.
    let parsed = std::panic::catch_unwind(|| Drawing::load(&mut cleaned.as_slice()))
        .map_err(|_| unreadable("the DXF parser crashed on this file".into()))?;
    let drawing = parsed.map_err(|err| unreadable(err.to_string()))?;
    import_drawing(&drawing, units)
}

pub fn import_drawing(
    drawing: &Drawing,
    units: Option<Units>,
) -> Result<ImportResult, ImportError> {
    let (file_units, units_from) = resolve_units(drawing, units)?;
    let scale = match file_units {
        Units::Mm => 1.0,
        Units::Inch => MM_PER_INCH,
    };
    let mut converter = convert::Converter::new(drawing, scale);
    converter.convert_modelspace();

    let mut issues = std::mem::take(&mut converter.issues);
    if units_from == UnitsFrom::Job {
        issues.push(Issue::new(
            Severity::Warning,
            "units_from_job",
            format!(
                "DXF has no units; using job units ({})",
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

    let chained = chain_segments(&converter.segments);
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
        units: file_units,
        units_from,
        bbox: bbox(&converter.segments),
        imported: converter.imported,
        flattened: converter.flattened,
        ignored: converter.ignored,
        issues,
    };
    Ok(ImportResult { contours, report })
}

fn resolve_units(drawing: &Drawing, job: Option<Units>) -> Result<(Units, UnitsFrom), ImportError> {
    let declared = match drawing.header.default_drawing_units {
        DxfUnits::Unitless => None,
        DxfUnits::Millimeters => Some(Units::Mm),
        DxfUnits::Inches => Some(Units::Inch),
        other => return Err(ImportError::UnsupportedUnits(other as i32)),
    };
    match (declared, job) {
        (None, None) => Err(ImportError::UnitsMissing),
        (None, Some(job)) => Ok((job, UnitsFrom::Job)),
        (Some(dxf), Some(job)) if dxf != job => Err(ImportError::UnitsMismatch {
            dxf: units_name(dxf),
            job: units_name(job),
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
