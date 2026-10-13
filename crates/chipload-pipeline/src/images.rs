// SPDX-License-Identifier: GPL-3.0-or-later
//! Pictures in a project (docs/specs/image-trace.md): add one, trace it again after its
//! settings change, remove it, and show it black and white. The traced loops become `path`
//! shapes on the image's layer that name the image; the CLI and the GUI both go through
//! these.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chipload_image::{Settings, Threshold};
use chipload_job::{Geometry, Image, ImageThreshold, Job, LineSegment, PathSegment, Shape};
use sha2::{Digest, Sha256};

use crate::PipelineError;

pub use chipload_image::{BlackWhite, Decoded, Trace};

/// What the image dialog shows for some settings (spec §6): the picture as black and white,
/// and the traced loops in mm from its lower-left corner.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub black_white: BlackWhite,
    pub trace: Trace,
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
        trace: chipload_image::trace(picture, &settings, image.width)?,
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
        threshold: new.threshold,
        invert: new.invert,
        blur: new.blur,
        min_area: new.min_area,
        smoothing: new.smoothing,
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
    out.drawing.shapes.retain(|s| s.image() != Some(id));
    let mut ids = Vec::with_capacity(trace.loops.len());
    for ring in &trace.loops {
        let shape_id = out.drawing.next_shape_id;
        out.drawing.next_shape_id += 1;
        ids.push(shape_id);
        out.drawing.shapes.push(Shape {
            id: shape_id,
            layer: image.layer.clone(),
            geometry: Geometry::Path {
                dxf_handle: None,
                dxf_type: None,
                closed: true,
                segments: segments(ring, image.x, image.y),
                image: Some(id),
            },
        });
    }
    let report = report(image, &trace, ids);
    Ok((out, report))
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
    }
}

fn traced(image: &Image) -> Result<Trace, PipelineError> {
    Ok(chipload_image::trace(
        &decoded(image)?,
        &settings(image),
        image.width,
    )?)
}

/// A closed ring of points, moved by (x, y), as lines.
fn segments(ring: &[chipload_core::Point], x: f64, y: f64) -> Vec<PathSegment> {
    let n = ring.len();
    (0..n)
        .map(|k| {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            PathSegment::Line(LineSegment {
                start: [a.0 + x, a.1 + y],
                end: [b.0 + x, b.1 + y],
            })
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

fn report(image: &Image, trace: &Trace, shapes: Vec<u64>) -> ImageReport {
    let mut lines = vec![format!(
        "Traced {} ({:.1} × {:.1} mm, threshold {}{}): {} {} on layer {}.",
        image.file,
        trace.width,
        trace.height,
        trace.threshold,
        if matches!(image.threshold, ImageThreshold::Auto(_)) {
            ", picked"
        } else {
            ""
        },
        trace.loops.len(),
        if trace.loops.len() == 1 {
            "shape"
        } else {
            "shapes"
        },
        image.layer
    )];
    if trace.loops.is_empty() {
        lines.push(
            "Nothing is black: change the threshold, or invert if the picture is light on dark."
                .into(),
        );
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
        black_at_border: trace.black_at_border,
        width: trace.width,
        height: trace.height,
        lines,
    }
}
