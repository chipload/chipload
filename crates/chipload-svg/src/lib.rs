// SPDX-License-Identifier: GPL-3.0-or-later
//! SVG import (docs/specs/svg-import.md, ADR 0024): an SVG's paths as shapes of lines and
//! arcs in mm, on layers, with what was left out counted. Bytes in, no file I/O.
//!
//! `usvg` resolves styles, `use`, transforms, units and basic shapes into plain paths; its
//! XML (`roxmltree`) is read here too, for the limits, the counts of what isn't imported,
//! and Inkscape's layer names, which usvg drops. Curves are flattened finely and fitted
//! with lines and arcs (`chipload_core::fit`), as text outlines are.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};

use chipload_core::Point;
use chipload_core::fit::fit_path;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use usvg::roxmltree;
use usvg::tiny_skia_path::PathSegment;

/// The largest SVG read, after a `.svgz` is unpacked, bytes.
pub const MAX_BYTES: usize = 50 * 1024 * 1024;
/// The most XML nodes read.
pub const MAX_NODES: u32 = 5_000_000;
/// The deepest nesting of elements read (deeper could overflow the stack while converting).
pub const MAX_DEPTH: usize = 256;
/// The most shapes (subpaths) one SVG gives.
pub const MAX_SHAPES: usize = 200_000;
/// The farthest a point may lie from the origin after scaling, mm (10 km).
pub const MAX_COORDINATE_MM: f64 = 10_000_000.0;
/// CSS pixels per inch: what usvg converts real units to.
pub const CSS_PX_PER_INCH: f64 = 96.0;

const MM_PER_INCH: f64 = 25.4;
const INKSCAPE: &str = "http://www.inkscape.org/namespaces/inkscape";
/// Units in the root's size that say how big the drawing really is.
const REAL_UNITS: [&str; 6] = ["mm", "cm", "in", "pt", "pc", "q"];

/// Why an SVG can't be read.
#[derive(Debug, Clone, PartialEq)]
pub enum SvgError {
    /// Larger than [`MAX_BYTES`] (unpacked).
    TooLarge,
    /// Not an SVG that can be read: the parser's own words.
    Unreadable(String),
    /// The parser crashed on it.
    Crashed,
    /// Nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// The file gives no real size: the pixels per inch must be chosen.
    NeedsScale,
    /// A scale or pixels per inch that isn't a positive number.
    BadScale(f64),
    /// More shapes than [`MAX_SHAPES`].
    TooMany(usize),
    /// A point of this element is not a number or lies beyond [`MAX_COORDINATE_MM`].
    FarOut(String),
}

impl SvgError {
    pub fn message(&self) -> Message {
        match self {
            SvgError::TooLarge => Message::new("svg-error-too-large").arg(
                "most",
                Value::Text(format!("{} MB", MAX_BYTES / 1024 / 1024)),
            ),
            SvgError::Unreadable(why) => {
                Message::new("svg-error-unreadable").arg("reason", Value::Text(why.clone()))
            }
            SvgError::Crashed => Message::new("svg-error-crashed"),
            SvgError::TooDeep => Message::new("svg-error-too-deep").arg("most", MAX_DEPTH),
            SvgError::NeedsScale => Message::new("svg-error-needs-scale"),
            SvgError::BadScale(got) => {
                Message::new("svg-error-scale").arg("got", Value::Number(*got, Digits::Shortest))
            }
            SvgError::TooMany(count) => Message::new("svg-error-too-many")
                .arg("count", *count)
                .arg("most", MAX_SHAPES),
            SvgError::FarOut(key) => Message::new("svg-error-far-out")
                .arg("element", Value::Text(key.clone()))
                .arg("most", Value::Length(MAX_COORDINATE_MM, Digits::Shortest)),
        }
    }
}

impl std::fmt::Display for SvgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for SvgError {}

/// One subpath: its layer, its key (the element's id, or `@n` for the n-th path without
/// one, then `#k` for its k-th subpath), and its lines and arcs in mm.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgShape {
    pub layer: String,
    pub key: String,
    pub closed: bool,
    pub prims: Vec<Primitive>,
}

/// What the import read and left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counts {
    /// Paths read (basic shapes included, as usvg turns them into paths).
    pub paths: usize,
    /// Their subpaths that became shapes.
    pub subpaths: usize,
    /// Bézier curves (and the SVG arcs and ellipses made of them) fitted with lines and arcs.
    pub curves: usize,
    /// Paths imported that don't show in the file (no fill and no stroke, or hidden).
    pub hidden: usize,
    /// Text elements left out.
    pub texts: usize,
    /// Pictures left out.
    pub images: usize,
    /// Elements clipped or masked, imported whole.
    pub clips: usize,
    /// Subpaths with no length left out.
    pub dropped: usize,
}

/// An SVG read into shapes.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgDrawing {
    pub shapes: Vec<SvgShape>,
    pub counts: Counts,
    /// The root's width and height have real units (mm, in, …), so the size is exact.
    pub units_in_file: bool,
    /// The pixels per inch used: [`CSS_PX_PER_INCH`] when the file has real units, else the
    /// one chosen.
    pub px_per_inch: f64,
}

/// Read an SVG (or `.svgz`). `px_per_inch` is needed when the file gives no real size and
/// ignored when it does. The drawing is scaled by `scale` (> 0) about the page's lower-left
/// corner, which becomes the origin, with curves fitted at that size.
pub fn read_svg(
    bytes: &[u8],
    px_per_inch: Option<f64>,
    scale: f64,
) -> Result<SvgDrawing, SvgError> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(SvgError::BadScale(scale));
    }
    let raw = unpacked(bytes)?;
    let text = std::str::from_utf8(&raw)
        .map_err(|_| SvgError::Unreadable("the file is not UTF-8 text".into()))?;
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: MAX_NODES,
        ..Default::default()
    };
    // Third-party parsers fed hostile input: a panic is an import error, not a crash.
    let doc = catch_unwind(AssertUnwindSafe(|| {
        roxmltree::Document::parse_with_options(text, options)
    }))
    .map_err(|_| SvgError::Crashed)?
    .map_err(|e| SvgError::Unreadable(e.to_string()))?;
    check_depth(&doc)?;
    let scan = scan(&doc);
    let px_per_inch = if scan.units_in_file {
        CSS_PX_PER_INCH
    } else {
        match px_per_inch {
            Some(px) if px.is_finite() && px > 0.0 => px,
            Some(px) => return Err(SvgError::BadScale(px)),
            None => return Err(SvgError::NeedsScale),
        }
    };
    let mut opt = usvg::Options {
        dpi: CSS_PX_PER_INCH as f32,
        ..Default::default()
    };
    // Never read other files or decode pictures: links are not followed.
    opt.resources_dir = None;
    opt.image_href_resolver = usvg::ImageHrefResolver {
        resolve_data: Box::new(|_, _, _| None),
        resolve_string: Box::new(|_, _| None),
    };
    let tree = catch_unwind(AssertUnwindSafe(|| usvg::Tree::from_xmltree(&doc, &opt)))
        .map_err(|_| SvgError::Crashed)?
        .map_err(|e| SvgError::Unreadable(e.to_string()))?;
    let mm = MM_PER_INCH / px_per_inch * scale;
    let page_height = f64::from(tree.size().height());
    let mut reader = Reader {
        to_mm: |x: f32, y: f32| (f64::from(x) * mm, (page_height - f64::from(y)) * mm),
        layers: &scan.layers,
        shapes: Vec::new(),
        counts: Counts {
            texts: scan.texts,
            images: scan.images,
            clips: scan.clips,
            ..Counts::default()
        },
        keys: BTreeSet::new(),
        unnamed: 0,
    };
    reader.group(tree.root(), None)?;
    Ok(SvgDrawing {
        shapes: reader.shapes,
        counts: reader.counts,
        units_in_file: scan.units_in_file,
        px_per_inch,
    })
}

/// The SVG text: as it is, or unpacked from gzip (`.svgz`), at most [`MAX_BYTES`].
fn unpacked(bytes: &[u8]) -> Result<Vec<u8>, SvgError> {
    let limit = MAX_BYTES as u64 + 1;
    let out = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(limit)
            .read_to_end(&mut out)
            .map_err(|_| SvgError::Unreadable("the .svgz is not valid gzip".into()))?;
        out
    } else {
        bytes.to_vec()
    };
    if out.len() > MAX_BYTES {
        return Err(SvgError::TooLarge);
    }
    Ok(out)
}

/// Refuse elements nested deeper than [`MAX_DEPTH`] (walked without recursion).
fn check_depth(doc: &roxmltree::Document<'_>) -> Result<(), SvgError> {
    let mut stack = vec![(doc.root(), 0usize)];
    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            return Err(SvgError::TooDeep);
        }
        for child in node.children().filter(roxmltree::Node::is_element) {
            stack.push((child, depth + 1));
        }
    }
    Ok(())
}

/// What the XML itself says: real units, Inkscape layers (group id -> label), and what
/// won't be imported.
struct Scan {
    units_in_file: bool,
    layers: BTreeMap<String, String>,
    texts: usize,
    images: usize,
    clips: usize,
}

fn scan(doc: &roxmltree::Document<'_>) -> Scan {
    let root = doc.root_element();
    let real = |name: &str| root.attribute(name).is_some_and(has_real_unit);
    let mut scan = Scan {
        units_in_file: real("width") && real("height"),
        layers: BTreeMap::new(),
        texts: 0,
        images: 0,
        clips: 0,
    };
    for node in doc.descendants().filter(roxmltree::Node::is_element) {
        match node.tag_name().name() {
            "text" => scan.texts += 1,
            "image" => scan.images += 1,
            "g" if node.attribute((INKSCAPE, "groupmode")) == Some("layer") => {
                if let Some(id) = node.attribute("id").filter(|id| !id.is_empty()) {
                    let label = node.attribute((INKSCAPE, "label")).unwrap_or(id);
                    scan.layers.insert(id.to_string(), label.to_string());
                }
            }
            _ => {}
        }
        let style = node.attribute("style").unwrap_or_default();
        let linked = |name: &str| {
            node.attribute(name).is_some_and(|v| v.trim() != "none")
                || style.contains(&format!("{name}:url"))
                || style.contains(&format!("{name}: url"))
        };
        if linked("clip-path") || linked("mask") {
            scan.clips += 1;
        }
    }
    scan
}

/// Whether a length such as `210mm` or `8.5in` has a real unit (not px, %, or none).
fn has_real_unit(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    REAL_UNITS.iter().any(|unit| {
        value
            .strip_suffix(unit)
            .is_some_and(|number| number.trim().parse::<f64>().is_ok())
    })
}

/// Walks usvg's tree, making shapes.
struct Reader<'a, F: Fn(f32, f32) -> Point> {
    to_mm: F,
    layers: &'a BTreeMap<String, String>,
    shapes: Vec<SvgShape>,
    counts: Counts,
    keys: BTreeSet<String>,
    unnamed: usize,
}

impl<F: Fn(f32, f32) -> Point> Reader<'_, F> {
    /// A group's paths, on `layer` (the outermost Inkscape layer it is in).
    fn group(&mut self, group: &usvg::Group, layer: Option<&str>) -> Result<(), SvgError> {
        let layer = layer.or_else(|| self.layers.get(group.id()).map(String::as_str));
        for node in group.children() {
            match node {
                usvg::Node::Group(g) => self.group(g, layer)?,
                usvg::Node::Path(p) => self.path(p, layer)?,
                // Counted from the XML (text isn't even read without fonts).
                usvg::Node::Image(_) | usvg::Node::Text(_) => {}
            }
        }
        Ok(())
    }

    fn path(&mut self, path: &usvg::Path, layer: Option<&str>) -> Result<(), SvgError> {
        self.counts.paths += 1;
        if !path.is_visible() {
            self.counts.hidden += 1;
        }
        let layer = match layer {
            Some(layer) => layer.to_string(),
            None if !self.layers.is_empty() => "0".to_string(),
            None => colour_layer(path),
        };
        let base = if path.id().is_empty() {
            self.unnamed += 1;
            format!("@{}", self.unnamed)
        } else {
            path.id().to_string()
        };
        let Some(data) = path.data().clone().transform(path.abs_transform()) else {
            self.counts.dropped += 1;
            return Ok(());
        };
        let mut pen = Pen::default();
        for segment in data.segments() {
            let at = |p: usvg::tiny_skia_path::Point| (self.to_mm)(p.x, p.y);
            match segment {
                PathSegment::MoveTo(p) => pen.move_to(at(p)),
                PathSegment::LineTo(p) => pen.line_to(at(p)),
                PathSegment::QuadTo(p1, p) => {
                    self.counts.curves += 1;
                    pen.quad_to(at(p1), at(p));
                }
                PathSegment::CubicTo(p1, p2, p) => {
                    self.counts.curves += 1;
                    pen.cubic_to(at(p1), at(p2), at(p));
                }
                PathSegment::Close => pen.close(),
            }
        }
        for (k, (points, closed)) in pen.finish().into_iter().enumerate() {
            if points
                .iter()
                .any(|p| !(p.0.abs() <= MAX_COORDINATE_MM && p.1.abs() <= MAX_COORDINATE_MM))
            {
                return Err(SvgError::FarOut(base));
            }
            let Some((points, closed)) = cleaned(points, closed) else {
                self.counts.dropped += 1;
                continue;
            };
            let prims = fit_path(&points, closed, ARC_APPROX_TOL_MM - FLAT_MM, true);
            if prims.is_empty() {
                self.counts.dropped += 1;
                continue;
            }
            if self.shapes.len() >= MAX_SHAPES {
                return Err(SvgError::TooMany(self.shapes.len() + 1));
            }
            let key = self.unique(format!("{base}#{}", k + 1));
            self.counts.subpaths += 1;
            self.shapes.push(SvgShape {
                layer: layer.clone(),
                key,
                closed,
                prims,
            });
        }
        Ok(())
    }

    /// `key`, or with `~2`, `~3`, … when it is taken (a path drawn twice by `use` keeps
    /// its id in each copy).
    fn unique(&mut self, key: String) -> String {
        let mut candidate = key.clone();
        let mut n = 1;
        while self.keys.contains(&candidate) {
            n += 1;
            candidate = format!("{key}~{n}");
        }
        self.keys.insert(candidate.clone());
        candidate
    }
}

/// A path's layer in a file without Inkscape layers: its stroke colour, else its fill
/// colour, as `#rrggbb` (`none` for neither, `gradient` or `pattern` for those paints).
fn colour_layer(path: &usvg::Path) -> String {
    let paint = path
        .stroke()
        .map(usvg::Stroke::paint)
        .or_else(|| path.fill().map(usvg::Fill::paint));
    match paint {
        Some(usvg::Paint::Color(c)) => format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue),
        Some(usvg::Paint::LinearGradient(_) | usvg::Paint::RadialGradient(_)) => {
            "gradient".to_string()
        }
        Some(usvg::Paint::Pattern(_)) => "pattern".to_string(),
        None => "none".to_string(),
    }
}

/// Curves are flattened within this before fitting (with the rest of the arc tolerance).
const FLAT_MM: f64 = ARC_APPROX_TOL_MM / 4.0;

/// Subpaths as dense points, curves flattened within [`FLAT_MM`].
#[derive(Default)]
struct Pen {
    current: Vec<Point>,
    closed: bool,
    /// `current` holds only the point carried over after a Z.
    restart: bool,
    done: Vec<(Vec<Point>, bool)>,
}

impl Pen {
    fn move_to(&mut self, p: Point) {
        self.end();
        self.current.push(p);
    }

    fn line_to(&mut self, p: Point) {
        self.current.push(p);
    }

    fn last(&self) -> Point {
        self.current.last().copied().unwrap_or((0.0, 0.0))
    }

    fn quad_to(&mut self, p1: Point, p2: Point) {
        let p0 = self.last();
        // The second difference bounds how far the curve leaves its chords.
        let bend = 2.0 * distance((p0.0 - p1.0, p0.1 - p1.1), (p1.0 - p2.0, p1.1 - p2.1));
        self.curve(bend, |t| {
            let u = 1.0 - t;
            (
                u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
                u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
            )
        });
    }

    fn cubic_to(&mut self, p1: Point, p2: Point, p3: Point) {
        let p0 = self.last();
        let second =
            |a: Point, b: Point, c: Point| distance((a.0 - b.0, a.1 - b.1), (b.0 - c.0, b.1 - c.1));
        let bend = 6.0 * second(p0, p1, p2).max(second(p1, p2, p3));
        self.curve(bend, |t| {
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            (
                a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
                a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
            )
        });
    }

    /// Points along a curve at even steps of its parameter, enough that no chord strays
    /// more than [`FLAT_MM`] from it (`bend` bounds its second derivative).
    fn curve(&mut self, bend: f64, point: impl Fn(f64) -> Point) {
        let n = if bend.is_finite() {
            (bend / (8.0 * FLAT_MM)).sqrt().ceil().clamp(1.0, 1000.0) as usize
        } else {
            1
        };
        for k in 1..=n {
            self.current.push(point(k as f64 / n as f64));
        }
    }

    fn close(&mut self) {
        self.closed = true;
        let start = self.current.first().copied();
        self.end();
        // After Z, drawing goes on from the subpath's start.
        if let Some(start) = start {
            self.current.push(start);
            self.closed = false;
            self.restart = true;
        }
    }

    fn end(&mut self) {
        let points = std::mem::take(&mut self.current);
        if self.restart && points.len() == 1 {
            // Only the point carried over after a Z: nothing drawn from it.
        } else if !points.is_empty() {
            self.done.push((points, self.closed));
        }
        self.closed = false;
        self.restart = false;
    }

    fn finish(mut self) -> Vec<(Vec<Point>, bool)> {
        self.end();
        self.done
    }
}

/// A subpath without repeated points; closed also when its ends meet. None when it has no
/// length (fewer than two points apart, three when closed).
fn cleaned(points: Vec<Point>, closed: bool) -> Option<(Vec<Point>, bool)> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if out.last().is_none_or(|&q| distance(p, q) > CHAIN_TOL_MM) {
            out.push(p);
        }
    }
    let mut closed = closed;
    if out.len() > 2
        && let (Some(&first), Some(&last)) = (out.first(), out.last())
        && distance(first, last) <= CHAIN_TOL_MM
    {
        out.pop();
        closed = true;
    }
    let enough = if closed { 3 } else { 2 };
    (out.len() >= enough).then_some((out, closed))
}

#[cfg(test)]
mod tests;
