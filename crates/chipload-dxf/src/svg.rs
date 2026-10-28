// SPDX-License-Identifier: GPL-3.0-or-later
//! SVG files as drawing files (docs/specs/svg-import.md): `chipload-svg`'s shapes as a
//! project's imported shapes, and what it read and left out in the drawing file's counts,
//! from which the import report's messages are made again in any language.

use std::collections::BTreeMap;
use std::path::Path;

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    DrawingFormat, DrawingSource, Geometry, PathSegment, Placement, Shape, SourceIssue, Units,
};
use chipload_svg::{SvgError, read_svg};

use crate::{ImportError, ImportedShapes, Issue, Severity, UnitsFrom, sha256_hex};

/// The drawing file's counts, by these names (`imported`, `flattened`, `ignored`).
const PATH: &str = "path";
const HIDDEN_PATH: &str = "hidden path";
const CURVE: &str = "curve";
const TEXT: &str = "text";
const IMAGE: &str = "image";
const CLIP: &str = "clip path or mask";
const EMPTY: &str = "empty subpath";

/// Whether `path` names an SVG (`.svg` or `.svgz`).
pub fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("svgz"))
}

/// An SVG already read into memory as shapes (ids 1, 2, … in file order) and its drawing
/// file; `name` is the file name. `px_per_inch` is needed only for a file without a real
/// size; `scale` as for [`crate::shapes_from_dxf_scaled`].
pub fn shapes_from_svg_bytes(
    bytes: &[u8],
    name: &str,
    px_per_inch: Option<f64>,
    scale: f64,
) -> Result<ImportedShapes, ImportError> {
    let read = read_svg(bytes, px_per_inch, scale).map_err(|e| match e {
        SvgError::NeedsScale => ImportError::ScaleMissing,
        other => ImportError::Unreadable {
            path: name.to_string(),
            reason: other.message().into(),
        },
    })?;
    let shapes = read
        .shapes
        .into_iter()
        .enumerate()
        .map(|(i, shape)| {
            let segments: Vec<PathSegment> = shape
                .prims
                .iter()
                .map(PathSegment::from_primitive)
                .collect();
            Shape {
                id: i as u64 + 1,
                layer: shape.layer,
                name: None,
                geometry: Geometry::Path {
                    dxf_handle: Some(shape.key),
                    dxf_type: Some(PATH.to_string()),
                    closed: PathSegment::closes(&segments),
                    segments,
                    image: None,
                    dxf: Some(1),
                    text: None,
                },
            }
        })
        .collect();
    let counts = read.counts;
    let nonzero = |pairs: &[(&str, usize)]| -> BTreeMap<String, usize> {
        pairs
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|(k, n)| ((*k).to_string(), *n))
            .collect()
    };
    let mut source = DrawingSource {
        id: 1,
        format: DrawingFormat::Svg,
        name: chipload_job::migrations::dxf_name(name),
        file: name.to_string(),
        sha256: sha256_hex(bytes),
        units: Units::Mm,
        units_from: if read.units_in_file {
            UnitsFrom::Dxf
        } else {
            UnitsFrom::User
        },
        imported: nonzero(&[
            (PATH, counts.paths - counts.hidden),
            (HIDDEN_PATH, counts.hidden),
        ]),
        flattened: nonzero(&[(CURVE, counts.curves)]),
        ignored: nonzero(&[
            (TEXT, counts.texts),
            (IMAGE, counts.images),
            (CLIP, counts.clips),
            (EMPTY, counts.dropped),
        ]),
        issues: Vec::new(),
        px_per_inch: (!read.units_in_file).then_some(read.px_per_inch),
        placement: Placement::default(),
    };
    source.issues = ISSUES
        .iter()
        .filter_map(|&(code, severity)| {
            issue_message(code, &source).map(|message| {
                let issue = Issue::new(severity, code, message);
                SourceIssue {
                    severity: issue.severity,
                    code: issue.code,
                    message: issue.message,
                }
            })
        })
        .collect();
    Ok(ImportedShapes { shapes, source })
}

/// The issues an SVG's import reports, in order, with their severity.
const ISSUES: [(&str, Severity); 6] = [
    ("svg_px_chosen", Severity::Warning),
    ("svg_text", Severity::Warning),
    ("svg_image", Severity::Warning),
    ("svg_clip", Severity::Warning),
    ("svg_hidden", Severity::Warning),
    ("svg_empty", Severity::Warning),
];

/// The message of an SVG's issue `code`, made from its drawing file's counts (so it is shown
/// in any language); None when there is nothing to say.
pub(crate) fn issue_message(code: &str, source: &DrawingSource) -> Option<Message> {
    let count = |map: &BTreeMap<String, usize>, key: &str| map.get(key).copied().unwrap_or(0);
    let counted = |id: &'static str, n: usize| (n > 0).then(|| Message::new(id).arg("count", n));
    match code {
        "svg_px_chosen" => source
            .px_per_inch
            .map(|px| Message::new("svg-px-chosen").arg("px", Value::Number(px, Digits::Shortest))),
        "svg_text" => counted("svg-text-left-out", count(&source.ignored, TEXT)),
        "svg_image" => counted("svg-images-left-out", count(&source.ignored, IMAGE)),
        "svg_clip" => counted("svg-clips-ignored", count(&source.ignored, CLIP)),
        "svg_hidden" => counted("svg-hidden-paths", count(&source.imported, HIDDEN_PATH)),
        "svg_empty" => counted("svg-empty-dropped", count(&source.ignored, EMPTY)),
        _ => None,
    }
}
