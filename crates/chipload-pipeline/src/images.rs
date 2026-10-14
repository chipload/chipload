// SPDX-License-Identifier: GPL-3.0-or-later
//! Pictures in a project (docs/specs/image-trace.md): add one, trace it again after its
//! settings change, remove it, and show it black and white. The traced outlines, or centre
//! lines (docs/specs/image-arcs-centrelines.md), become `path` shapes on the image's layer
//! that name the image; the CLI and the GUI both go through these.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chipload_core::Point;
use chipload_core::fit::fit_path;
use chipload_core::primitives::Primitive;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_image::{ImageError, MAX_LOOPS, Settings, Threshold};
use chipload_job::{
    ArcSegment, Geometry, Image, ImageThreshold, ImageTrace, Job, LineSegment, PathSegment, Shape,
};
use sha2::{Digest, Sha256};

use crate::PipelineError;

pub use chipload_image::{BlackWhite, Decoded};

/// What the image dialog shows for some settings (spec §6): the picture as black and white,
/// and what it traces into.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub black_white: BlackWhite,
    pub trace: Traced,
}

/// What a picture traces into, in mm from its lower-left corner.
#[derive(Debug, Clone, PartialEq)]
pub struct Traced {
    /// Outlines (closed, black inside) or centre lines.
    pub paths: Vec<TracedPath>,
    /// Traced as centre lines.
    pub centre_lines: bool,
    /// The threshold used (the one auto picked, or the one set).
    pub threshold: u8,
    /// Pieces dropped as smaller than the smallest piece.
    pub dropped: usize,
    /// Outlines kept as lines: their arcs came too near another outline.
    pub without_arcs: usize,
    /// Outlines kept as traced: simplifying made them cross.
    pub unsimplified: usize,
    /// Centre lines: pieces too small for one (dots), left out.
    pub dots: usize,
    /// Centre lines: the strokes' widths, mm (5th and 95th percentile).
    pub widths: Option<(f64, f64)>,
    pub black_at_border: bool,
    /// The picture's size on the stock, mm.
    pub width: f64,
    pub height: f64,
}

/// One traced path.
#[derive(Debug, Clone, PartialEq)]
pub struct TracedPath {
    /// Lines and arcs, end to end.
    pub pieces: Vec<Primitive>,
    /// The same as points (arcs as chords), for drawing.
    pub points: Vec<Point>,
    /// Ends where it starts.
    pub closed: bool,
}

/// Read a picture once, for previews with changing settings.
pub fn decode_picture(bytes: &[u8]) -> Result<Decoded, PipelineError> {
    Ok(chipload_image::decode(bytes)?)
}

/// `picture` with `image`'s settings and width (its other fields are unused).
pub fn preview(picture: &Decoded, image: &Image) -> Result<Preview, PipelineError> {
    let settings = settings(image);
    Ok(Preview {
        black_white: chipload_image::black_white(picture, &settings)?,
        trace: traced_picture(picture, image)?,
    })
}

/// A picture to add: its file name and bytes, where it goes, and its settings.
#[derive(Debug, Clone)]
pub struct NewImage<'a> {
    pub file: &'a str,
    pub bytes: &'a [u8],
    /// None: named after the file.
    pub layer: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub threshold: ImageThreshold,
    pub invert: bool,
    pub blur: f64,
    pub min_area: f64,
    pub smoothing: f64,
    pub trace: ImageTrace,
    pub arcs: bool,
}

impl<'a> NewImage<'a> {
    /// The spec's default settings (§2), at (0, 0).
    pub fn new(file: &'a str, bytes: &'a [u8], width: f64) -> Self {
        let d = Settings::default();
        NewImage {
            file,
            bytes,
            layer: None,
            x: 0.0,
            y: 0.0,
            width,
            threshold: ImageThreshold::AUTO,
            invert: d.invert,
            blur: d.blur,
            min_area: d.min_area,
            smoothing: d.smoothing,
            trace: ImageTrace::Outline,
            arcs: d.arcs,
        }
    }
}

/// What tracing an image gave.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageReport {
    pub id: u64,
    /// The traced shapes' ids.
    pub shapes: Vec<u64>,
    /// The threshold used (the one auto picked, or the one set).
    pub threshold: u8,
    pub dropped: usize,
    pub unsimplified: usize,
    pub without_arcs: usize,
    /// Traced as centre lines; then `dots` and `widths` as in [`Traced`].
    pub centre_lines: bool,
    pub dots: usize,
    pub widths: Option<(f64, f64)>,
    pub black_at_border: bool,
    /// The picture's size on the stock, mm.
    pub width: f64,
    pub height: f64,
    /// The report as sentences, for the CLI and the GUI.
    pub lines: Vec<String>,
}

/// Add a picture and trace it. Returns the project and what was traced.
pub fn add_image(job: &Job, new: NewImage) -> Result<(Job, ImageReport), PipelineError> {
    // Check the picture before it goes into the project.
    chipload_image::decode(new.bytes)?;
    let id = job.drawing.images.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    let layer = new
        .layer
        .unwrap_or_else(|| default_layer(job, new.file, id));
    let image = Image {
        id,
        file: new.file.to_string(),
        sha256: format!("{:x}", Sha256::digest(new.bytes)),
        data: STANDARD.encode(new.bytes),
        layer,
        x: new.x,
        y: new.y,
        width: new.width,
        rotation: 0.0,
        threshold: new.threshold,
        invert: new.invert,
        blur: new.blur,
        min_area: new.min_area,
        smoothing: new.smoothing,
        trace: new.trace,
        arcs: new.arcs,
    };
    let mut out = job.clone();
    out.drawing.images.push(image);
    retrace(&out, id)
}

/// Trace image `id` again with its current settings, replacing its shapes.
pub fn retrace(job: &Job, id: u64) -> Result<(Job, ImageReport), PipelineError> {
    let image = find(job, id)?;
    let problems = image_problems(job, id);
    if !problems.is_empty() {
        return Err(PipelineError::Job(problems.join("; ")));
    }
    let trace = traced(image)?;
    let mut out = job.clone();
    let old: Vec<Shape> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.image() == Some(id))
        .cloned()
        .collect();
    out.drawing.shapes.retain(|s| s.image() != Some(id));
    let mut ids = Vec::with_capacity(trace.paths.len());
    for path in &trace.paths {
        let shape_id = out.drawing.next_shape_id;
        out.drawing.next_shape_id += 1;
        ids.push(shape_id);
        out.drawing.shapes.push(Shape {
            id: shape_id,
            layer: image.layer.clone(),
            geometry: Geometry::Path {
                dxf_handle: None,
                dxf_type: None,
                closed: path.closed,
                segments: placed(&path.pieces, image, trace.height),
                image: Some(id),
                dxf: None,
            },
        });
    }
    let mut report = report(image, &trace, ids);
    report.lines.extend(find_left_out_again(&mut out, id, &old));
    Ok((out, report))
}

/// How close a left-out shape's centre must come, as a share of the picture's box, and
/// how near its size, to be found again (docs/specs/leave-out-shapes.md §4).
const FOUND_AGAIN_CENTRE: f64 = 0.02;
const FOUND_AGAIN_SIZE: f64 = 0.15;

/// A shape's box, from its segments.
fn shape_box(shape: &Shape) -> Option<(Point, Point)> {
    let Geometry::Path { segments, .. } = &shape.geometry else {
        return None;
    };
    let mut boxes = segments.iter().map(|segment| {
        let piece = match segment {
            PathSegment::Line(l) => Primitive::Line(chipload_core::primitives::Line {
                start: (l.start[0], l.start[1]),
                end: (l.end[0], l.end[1]),
            }),
            PathSegment::Arc(a) => Primitive::Arc(chipload_core::primitives::Arc {
                center: (a.center[0], a.center[1]),
                radius: a.radius,
                start_angle: a.start_angle,
                sweep: a.sweep,
            }),
        };
        piece.bounds()
    });
    let first = boxes.next()?;
    Some(boxes.fold(first, |(lo, hi), (a, b)| {
        (
            (lo.0.min(a.0), lo.1.min(a.1)),
            (hi.0.max(b.0), hi.1.max(b.1)),
        )
    }))
}

/// Each shape's centre and size relative to the box round all of them, with whether it is
/// closed: (id, (u, v), (w, h), closed).
type Relative = (u64, Point, Point, bool);

fn relative(shapes: &[&Shape]) -> Vec<Relative> {
    let boxes: Vec<(u64, (Point, Point), bool)> = shapes
        .iter()
        .filter_map(|s| shape_box(s).map(|b| (s.id, b, s.closed())))
        .collect();
    let Some(&(_, first, _)) = boxes.first() else {
        return Vec::new();
    };
    let ((x0, y0), (x1, y1)) = boxes.iter().fold(first, |(lo, hi), (_, (a, b), _)| {
        (
            (lo.0.min(a.0), lo.1.min(a.1)),
            (hi.0.max(b.0), hi.1.max(b.1)),
        )
    });
    let (w, h) = ((x1 - x0).max(CHAIN_TOL_MM), (y1 - y0).max(CHAIN_TOL_MM));
    boxes
        .into_iter()
        .map(|(id, ((a0, b0), (a1, b1)), closed)| {
            (
                id,
                (((a0 + a1) / 2.0 - x0) / w, ((b0 + b1) / 2.0 - y0) / h),
                ((a1 - a0) / w, (b1 - b0) / h),
                closed,
            )
        })
        .collect()
}

/// After image `id` is traced again: each operation's left-out shapes of it, found again
/// among the new shapes by where they sit in the picture (docs/specs/leave-out-shapes.md
/// §4). One that isn't found keeps its old id, so the project shows a problem until the
/// shapes are picked again: nothing left out is ever cut silently. Returns report lines.
fn find_left_out_again(job: &mut Job, id: u64, old: &[Shape]) -> Vec<String> {
    let old_refs: Vec<&Shape> = old.iter().collect();
    let new_refs: Vec<Shape> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.image() == Some(id))
        .cloned()
        .collect();
    let new_refs: Vec<&Shape> = new_refs.iter().collect();
    let before = relative(&old_refs);
    let after = relative(&new_refs);
    let mut lines = Vec::new();
    for (i, op) in job.operations.iter_mut().enumerate() {
        let select = op.select_mut();
        if select.image != Some(id) {
            continue;
        }
        let Some(exclude) = select.exclude.as_mut().filter(|e| !e.is_empty()) else {
            continue;
        };
        let mut taken: Vec<u64> = Vec::new();
        let mut lost = 0;
        for left_out in exclude.iter_mut() {
            let Some(&(_, centre, size, closed)) = before.iter().find(|r| r.0 == *left_out) else {
                lost += 1;
                continue;
            };
            let near = |r: &&Relative| {
                let d = ((r.1.0 - centre.0).powi(2) + (r.1.1 - centre.1).powi(2)).sqrt();
                let similar = |a: f64, b: f64| (a - b).abs() <= FOUND_AGAIN_SIZE * a.max(b);
                (d <= FOUND_AGAIN_CENTRE
                    && r.3 == closed
                    && similar(r.2.0, size.0)
                    && similar(r.2.1, size.1)
                    && !taken.contains(&r.0))
                .then_some(d)
            };
            let best = after
                .iter()
                .filter_map(|r| near(&r).map(|d| (d, r.0)))
                .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            match best {
                Some((_, found)) => {
                    taken.push(found);
                    *left_out = found;
                }
                None => lost += 1,
            }
        }
        let count = exclude.len();
        lines.push(if lost == 0 {
            format!(
                "Operation {}: its {count} left-out {} found again.",
                i + 1,
                if count == 1 { "shape" } else { "shapes" }
            )
        } else {
            format!(
                "Operation {}: {lost} left-out {} not found again: pick the shapes to leave \
                 out again.",
                i + 1,
                if lost == 1 { "shape" } else { "shapes" }
            )
        });
    }
    lines
}

/// Trace again every image whose picture, position or settings differ in `after` from
/// `before` (or that is new), as after a `chipload set`.
pub fn retrace_changed(
    before: &Job,
    after: &Job,
) -> Result<(Job, Vec<ImageReport>), PipelineError> {
    let mut out = after.clone();
    let mut reports = Vec::new();
    for image in &after.drawing.images {
        let old = before.drawing.images.iter().find(|i| i.id == image.id);
        if old != Some(image) {
            let (job, report) = retrace(&out, image.id)?;
            out = job;
            reports.push(report);
        }
    }
    Ok((out, reports))
}

/// Remove image `id` and its shapes. Operations that select its layer stay; they then find
/// nothing, which the project's problems show.
pub fn remove_image(job: &Job, id: u64) -> Result<Job, PipelineError> {
    find(job, id)?;
    let mut out = job.clone();
    out.drawing.images.retain(|i| i.id != id);
    out.drawing.shapes.retain(|s| s.image() != Some(id));
    Ok(out)
}

/// Image `id` as black and white, as it is traced.
pub fn black_white(job: &Job, id: u64) -> Result<chipload_image::BlackWhite, PipelineError> {
    let image = find(job, id)?;
    Ok(chipload_image::black_white(
        &decoded(image)?,
        &settings(image),
    )?)
}

/// Image `id` as a black-and-white PNG.
pub fn black_white_png(job: &Job, id: u64) -> Result<Vec<u8>, PipelineError> {
    let bw = black_white(job, id)?;
    let pixels: Vec<u8> = bw.black.iter().map(|&b| if b { 0 } else { 255 }).collect();
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, bw.width, bw.height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| PipelineError::Job(format!("writing the picture: {e}")))?;
    writer
        .write_image_data(&pixels)
        .map_err(|e| PipelineError::Job(format!("writing the picture: {e}")))?;
    writer
        .finish()
        .map_err(|e| PipelineError::Job(format!("writing the picture: {e}")))?;
    Ok(bytes)
}

/// The picture's bytes, decoded from the project.
pub fn picture_bytes(image: &Image) -> Result<Vec<u8>, PipelineError> {
    STANDARD.decode(&image.data).map_err(|e| {
        PipelineError::Job(format!(
            "drawing.images: the picture of image {} can't be read: {e}",
            image.id
        ))
    })
}

fn find(job: &Job, id: u64) -> Result<&Image, PipelineError> {
    job.drawing
        .images
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| PipelineError::Job(format!("there is no image {id}")))
}

fn image_problems(job: &Job, id: u64) -> Vec<String> {
    job.drawing
        .images
        .iter()
        .enumerate()
        .filter(|(_, i)| i.id == id)
        .flat_map(|(k, i)| i.problems(k))
        .collect()
}

fn decoded(image: &Image) -> Result<Decoded, PipelineError> {
    Ok(chipload_image::decode(&picture_bytes(image)?)?)
}

fn settings(image: &Image) -> Settings {
    Settings {
        threshold: match image.threshold {
            ImageThreshold::Auto(_) => Threshold::Auto,
            ImageThreshold::Level(t) => Threshold::Level(t),
        },
        invert: image.invert,
        blur: image.blur,
        min_area: image.min_area,
        smoothing: image.smoothing,
        arcs: image.arcs,
    }
}

fn traced(image: &Image) -> Result<Traced, PipelineError> {
    traced_picture(&decoded(image)?, image)
}

/// `picture` traced with `image`'s settings: outlines, or centre lines fitted within the
/// smoothing (docs/specs/image-arcs-centrelines.md §3).
fn traced_picture(picture: &Decoded, image: &Image) -> Result<Traced, PipelineError> {
    let settings = settings(image);
    match image.trace {
        ImageTrace::Outline => {
            let t = chipload_image::trace(picture, &settings, image.width)?;
            let paths = t
                .pieces
                .into_iter()
                .zip(t.loops)
                .map(|(pieces, points)| TracedPath {
                    pieces,
                    points,
                    closed: true,
                })
                .collect();
            Ok(Traced {
                paths,
                centre_lines: false,
                threshold: t.threshold,
                dropped: t.dropped,
                without_arcs: t.without_arcs,
                unsimplified: t.unsimplified,
                dots: 0,
                widths: None,
                black_at_border: t.black_at_border,
                width: t.width,
                height: t.height,
            })
        }
        ImageTrace::CentreLine => {
            let raw = chipload_image::trace_raw(picture, &settings, image.width)?;
            let found = chipload_toolpath::centre::centre_lines(&raw.loops, image.smoothing)?;
            if found.paths.len() > MAX_LOOPS {
                return Err(ImageError::TooManyLoops(found.paths.len()).into());
            }
            let paths = found
                .paths
                .iter()
                .map(|path| {
                    let pieces = fit_path(&path.points, path.closed, image.smoothing, image.arcs);
                    TracedPath {
                        points: drawn(&pieces),
                        pieces,
                        closed: path.closed,
                    }
                })
                .filter(|path| !path.pieces.is_empty())
                .collect();
            Ok(Traced {
                paths,
                centre_lines: true,
                threshold: raw.threshold,
                dropped: raw.dropped,
                without_arcs: 0,
                unsimplified: 0,
                dots: found.dots,
                widths: found.widths,
                black_at_border: raw.black_at_border,
                width: raw.width,
                height: raw.height,
            })
        }
    }
}

/// Pieces as points along them (arcs as chords), without repeating joins.
fn drawn(pieces: &[Primitive]) -> Vec<Point> {
    let mut points = Vec::new();
    for piece in pieces {
        let along = piece.points();
        let skip = usize::from(!points.is_empty());
        points.extend_from_slice(&along[skip.min(along.len())..]);
    }
    points
}

/// Traced pieces where the picture is: turned about its centre by its rotation, then
/// moved to its corner (docs/specs/direct-placement.md §3). Unturned, only moved, as
/// before rotations.
fn placed(pieces: &[Primitive], image: &Image, height: f64) -> Vec<PathSegment> {
    if image.rotation == 0.0 {
        return segments(pieces, image.x, image.y);
    }
    let (cx, cy) = (image.width / 2.0, height / 2.0);
    segments(pieces, -cx, -cy)
        .iter()
        .map(|s| {
            s.rotated(image.rotation)
                .translated(image.x + cx, image.y + cy)
        })
        .collect()
}

/// The box round a picture of `width` × `height` mm with its lower-left corner at (x, y),
/// turned by `rotation` degrees about its centre: what the canvas shows and *Position*
/// edits (docs/specs/direct-placement.md §2).
pub fn turned_box(x: f64, y: f64, width: f64, height: f64, rotation: f64) -> (Point, Point) {
    let (cx, cy) = (x + width / 2.0, y + height / 2.0);
    let corners = [
        [-width / 2.0, -height / 2.0],
        [width / 2.0, -height / 2.0],
        [width / 2.0, height / 2.0],
        [-width / 2.0, height / 2.0],
    ]
    .map(|p| chipload_job::rotated(p, rotation));
    let xs = corners.map(|p| p[0]);
    let ys = corners.map(|p| p[1]);
    let min = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
    let max = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
    ((cx + min(xs), cy + min(ys)), (cx + max(xs), cy + max(ys)))
}

/// Lines and arcs moved by (x, y), as path segments.
fn segments(pieces: &[Primitive], x: f64, y: f64) -> Vec<PathSegment> {
    pieces
        .iter()
        .map(|piece| match piece {
            Primitive::Line(l) => PathSegment::Line(LineSegment {
                start: [l.start.0 + x, l.start.1 + y],
                end: [l.end.0 + x, l.end.1 + y],
            }),
            Primitive::Arc(a) => PathSegment::Arc(ArcSegment {
                center: [a.center.0 + x, a.center.1 + y],
                radius: a.radius,
                start_angle: a.start_angle,
                sweep: a.sweep,
            }),
        })
        .collect()
}

fn next_id(job: &Job) -> u64 {
    job.drawing.images.iter().map(|i| i.id).max().unwrap_or(0) + 1
}

/// The layer a new image from `file` gets: the file's name without its extension.
pub fn new_layer(job: &Job, file: &str) -> String {
    default_layer(job, file, next_id(job))
}

/// The file's name without its extension, unless another image already uses that layer.
fn default_layer(job: &Job, file: &str, id: u64) -> String {
    let stem = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("image")
        .to_string();
    if job.drawing.images.iter().any(|i| i.layer == stem) {
        format!("{stem}-{id}")
    } else {
        stem
    }
}

fn report(image: &Image, trace: &Traced, shapes: Vec<u64>) -> ImageReport {
    let count = trace.paths.len();
    let what = match (trace.centre_lines, count == 1) {
        (false, true) => "shape",
        (false, false) => "shapes",
        (true, true) => "centre line",
        (true, false) => "centre lines",
    };
    let mut lines = vec![format!(
        "Traced {} ({:.1} × {:.1} mm, threshold {}{}): {count} {what} on layer {}.",
        image.file,
        trace.width,
        trace.height,
        trace.threshold,
        if matches!(image.threshold, ImageThreshold::Auto(_)) {
            ", picked"
        } else {
            ""
        },
        image.layer
    )];
    if count == 0 && trace.dots == 0 {
        lines.push(
            "Nothing is black: change the threshold, or invert if the picture is light on dark."
                .into(),
        );
    }
    if let Some((narrow, wide)) = trace.widths {
        lines.push(format!(
            "Strokes {narrow:.2} to {wide:.2} mm wide: cut them with a follow path."
        ));
    }
    // A closed centre line (an outline drawn by hand) is an area as well.
    let closed = trace.paths.iter().filter(|p| p.closed).count();
    if trace.centre_lines && closed > 0 {
        lines.push(format!(
            "{closed} closed {}: a profile or pocket can cut round {}, as round any outline.",
            if closed == 1 { "line" } else { "lines" },
            if closed == 1 { "it" } else { "them" }
        ));
    }
    if trace.dots > 0 {
        lines.push(format!(
            "{} {} too small for a centre line left out.",
            trace.dots,
            if trace.dots == 1 { "dot" } else { "dots" }
        ));
    }
    if trace.dropped > 0 {
        lines.push(format!(
            "{} {} smaller than {} mm² dropped.",
            trace.dropped,
            if trace.dropped == 1 {
                "speck"
            } else {
                "specks"
            },
            image.min_area
        ));
    }
    if trace.black_at_border {
        lines.push(
            "Black touches the picture's edge and is closed along it; if that's the background, \
             invert."
                .into(),
        );
    }
    if trace.without_arcs > 0 {
        lines.push(format!(
            "{} {} kept as lines: {} arcs came too near another shape.",
            trace.without_arcs,
            if trace.without_arcs == 1 {
                "shape"
            } else {
                "shapes"
            },
            if trace.without_arcs == 1 {
                "its"
            } else {
                "their"
            }
        ));
    }
    if trace.unsimplified > 0 {
        lines.push(format!(
            "{} {} kept as traced: smoothing made them cross.",
            trace.unsimplified,
            if trace.unsimplified == 1 {
                "shape"
            } else {
                "shapes"
            }
        ));
    }
    ImageReport {
        id: image.id,
        shapes,
        threshold: trace.threshold,
        dropped: trace.dropped,
        unsimplified: trace.unsimplified,
        without_arcs: trace.without_arcs,
        centre_lines: trace.centre_lines,
        dots: trace.dots,
        widths: trace.widths,
        black_at_border: trace.black_at_border,
        width: trace.width,
        height: trace.height,
        lines,
    }
}
