// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload image …`: pictures traced into shapes for V-carving
//! (docs/specs/image-trace.md §5).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{ImageThreshold, ImageTrace, Job, save_job};
use chipload_pipeline::images::{
    ImageReport, NewImage, add_image, black_white_png, remove_image, retrace,
};
use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum ImageAction {
    /// Add a picture (PNG, JPEG or BMP) and trace it into shapes on its own layer.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// The picture.
        picture: PathBuf,
        /// The picture's width on the stock, mm; its height follows.
        #[arg(value_parser = crate::lengths::length, long)]
        width: f64,
        /// The layer for its shapes; default: the picture's file name.
        #[arg(long)]
        layer: Option<String>,
        /// Carve the light parts instead of the dark ones.
        #[arg(long)]
        invert: bool,
        #[command(flatten)]
        settings: Settings,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Change an image's settings, position or size, and trace it again.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The image's id (as `image list` shows it).
        id: u64,
        /// The picture's width on the stock, mm.
        #[arg(value_parser = crate::lengths::length, long)]
        width: Option<f64>,
        /// Turn it about its centre to this angle, degrees counter-clockwise (any angle; 0:
        /// as the picture is). `--at` stays the corner before the turn.
        #[arg(long, allow_hyphen_values = true)]
        rotate: Option<f64>,
        /// The layer for its shapes.
        #[arg(long)]
        layer: Option<String>,
        /// Carve the light parts instead (true or false).
        #[arg(long)]
        invert: Option<bool>,
        #[command(flatten)]
        settings: Settings,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// The project's images: picture, place, settings and shapes.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Write an image as black and white, as it is traced (PNG).
    Preview {
        /// The project (.chipload).
        project: PathBuf,
        /// The image's id.
        id: u64,
        /// The PNG to write.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Remove an image and its shapes.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// The image's id.
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// How the picture becomes black and white and is traced (spec §2); unset: the default
/// (add) or unchanged (set).
#[derive(Args)]
pub struct Settings {
    /// Where the picture's lower-left corner goes: X,Y in mm (default 0,0).
    #[arg(long, value_parser = parse_at)]
    at: Option<(f64, f64)>,
    /// Grey values (0–255) below this are black: 1 to 254, or auto (default).
    #[arg(long, value_parser = parse_threshold)]
    threshold: Option<ImageThreshold>,
    /// Blur before the threshold, px, 0 to 10 (default 1).
    #[arg(long)]
    blur: Option<f64>,
    /// Drop specks smaller than this, mm², 0 to 100 (default 0.5).
    #[arg(long)]
    min_area: Option<f64>,
    /// How far an edge may be simplified, mm, 0.01 to 1 (default 0.05).
    #[arg(value_parser = crate::lengths::length, long)]
    smoothing: Option<f64>,
    /// What to trace: outline (for a V-carve, default) or centre-line (strokes, for a
    /// follow path).
    #[arg(long, value_parser = parse_trace)]
    trace: Option<ImageTrace>,
    /// Fit arcs as well as lines (true or false; default true).
    #[arg(long)]
    arcs: Option<bool>,
}

fn parse_trace(text: &str) -> Result<ImageTrace, String> {
    match text {
        "outline" => Ok(ImageTrace::Outline),
        "centre-line" | "centre_line" | "center-line" | "center_line" => Ok(ImageTrace::CentreLine),
        _ => Err(format!("expected outline or centre-line, got {text:?}")),
    }
}

fn parse_at(text: &str) -> Result<(f64, f64), String> {
    crate::lengths::point(text).map(|[x, y]| (x, y))
}

fn parse_threshold(text: &str) -> Result<ImageThreshold, String> {
    if text.eq_ignore_ascii_case("auto") {
        return Ok(ImageThreshold::AUTO);
    }
    match text.parse::<u8>() {
        Ok(t) if (1..=254).contains(&t) => Ok(ImageThreshold::Level(t)),
        _ => Err(format!("expected 1 to 254 or auto, got {text:?}")),
    }
}

pub fn image(out: &mut Out, action: ImageAction) -> ExitCode {
    match action {
        ImageAction::Add {
            project,
            picture,
            width,
            layer,
            invert,
            settings,
            output,
        } => {
            let new = New {
                width,
                layer,
                invert,
            };
            add(out, &project, &picture, new, &settings, output.as_deref())
        }
        ImageAction::Set {
            project,
            id,
            width,
            rotate,
            layer,
            invert,
            settings,
            output,
        } => {
            let change = |job: &mut Job| -> Result<(), String> {
                let image = job
                    .drawing
                    .images
                    .iter_mut()
                    .find(|i| i.id == id)
                    .ok_or_else(|| format!("there is no image {id}"))?;
                if let Some(width) = width {
                    image.width = width;
                }
                if let Some(degrees) = rotate {
                    if !degrees.is_finite() {
                        return Err(format!("--rotate takes degrees, got {degrees}"));
                    }
                    let turned = degrees.rem_euclid(360.0);
                    image.rotation = if turned >= 360.0 { 0.0 } else { turned };
                }
                if let Some(layer) = &layer {
                    image.layer = layer.clone();
                }
                if let Some(invert) = invert {
                    image.invert = invert;
                }
                apply(&settings, image);
                Ok(())
            };
            set(out, &project, id, change, output.as_deref())
        }
        ImageAction::List { project } => list(out, &project),
        ImageAction::Preview {
            project,
            id,
            output,
        } => preview(out, &project, id, &output),
        ImageAction::Remove {
            project,
            id,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            match remove_image(&job, id) {
                Ok(job) => {
                    out.line(format!("Removed image {id} and its shapes."));
                    write(out, &job, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
    }
}

fn apply(settings: &Settings, image: &mut chipload_job::Image) {
    if let Some((x, y)) = settings.at {
        (image.x, image.y) = (x, y);
    }
    if let Some(threshold) = settings.threshold {
        image.threshold = threshold;
    }
    if let Some(blur) = settings.blur {
        image.blur = blur;
    }
    if let Some(min_area) = settings.min_area {
        image.min_area = min_area;
    }
    if let Some(smoothing) = settings.smoothing {
        image.smoothing = smoothing;
    }
    if let Some(trace) = settings.trace {
        image.trace = trace;
    }
    if let Some(arcs) = settings.arcs {
        image.arcs = arcs;
    }
}

/// What `image add` takes besides the settings.
struct New {
    width: f64,
    layer: Option<String>,
    invert: bool,
}

fn add(
    out: &mut Out,
    project: &Path,
    picture: &Path,
    given: New,
    settings: &Settings,
    output: Option<&Path>,
) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let bytes = match std::fs::read(picture) {
        Ok(bytes) => bytes,
        Err(err) => return out.fail(format!("{}: {err}", picture.display())),
    };
    let file = picture
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut new = NewImage::new(&file, &bytes, given.width);
    new.layer = given.layer;
    new.invert = given.invert;
    new.x = settings.at.map_or(0.0, |a| a.0);
    new.y = settings.at.map_or(0.0, |a| a.1);
    new.threshold = settings.threshold.unwrap_or(new.threshold);
    new.blur = settings.blur.unwrap_or(new.blur);
    new.min_area = settings.min_area.unwrap_or(new.min_area);
    new.smoothing = settings.smoothing.unwrap_or(new.smoothing);
    new.trace = settings.trace.unwrap_or(new.trace);
    new.arcs = settings.arcs.unwrap_or(new.arcs);
    match add_image(&job, new) {
        Ok((job, report)) => {
            reported(out, &report);
            write(out, &job, output.unwrap_or(project))
        }
        Err(err) => out.fail(err),
    }
}

/// `place --image`: the picture's lower-left corner, its turn (degrees counter-clockwise,
/// any angle) and its width; traced again, as `image set` does.
pub fn place_image(
    out: &mut Out,
    project: &Path,
    id: u64,
    at: Option<(f64, f64)>,
    rotate: Option<f64>,
    width: Option<f64>,
    output: Option<&Path>,
) -> ExitCode {
    let change = |job: &mut Job| -> Result<(), String> {
        let image = job
            .drawing
            .images
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| format!("there is no image {id}"))?;
        if let Some((x, y)) = at {
            (image.x, image.y) = (x, y);
        }
        if let Some(degrees) = rotate {
            if !degrees.is_finite() {
                return Err(format!("--rotate takes degrees, got {degrees}"));
            }
            image.rotation = degrees.rem_euclid(360.0);
        }
        if let Some(width) = width {
            image.width = width;
        }
        Ok(())
    };
    set(out, project, id, change, output)
}

fn set(
    out: &mut Out,
    project: &Path,
    id: u64,
    change: impl Fn(&mut Job) -> Result<(), String>,
    output: Option<&Path>,
) -> ExitCode {
    let mut job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    if let Err(err) = change(&mut job) {
        return out.fail(err);
    }
    match retrace(&job, id) {
        Ok((job, report)) => {
            reported(out, &report);
            write(out, &job, output.unwrap_or(project))
        }
        Err(err) => out.fail(err),
    }
}

fn list(out: &mut Out, project: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    if job.drawing.images.is_empty() {
        out.line("No images.");
    }
    let mut rows = Vec::new();
    for image in &job.drawing.images {
        let shapes = job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.image() == Some(image.id))
            .count();
        out.line(format!(
            "{}: {} on layer {}, {} mm wide at ({}, {}){}; threshold {}, invert {}, blur {} px, \
             smallest piece {} mm², smoothing {} mm, {}, arcs {}; {shapes} {}",
            image.id,
            image.file,
            image.layer,
            image.width,
            image.x,
            image.y,
            if image.rotation == 0.0 {
                String::new()
            } else {
                format!(
                    ", turned {}°",
                    chipload_pipeline::placement::degrees_text(image.rotation)
                )
            },
            threshold_text(image.threshold),
            if image.invert { "on" } else { "off" },
            image.blur,
            image.min_area,
            image.smoothing,
            trace_text(image.trace),
            if image.arcs { "on" } else { "off" },
            if shapes == 1 { "shape" } else { "shapes" }
        ));
        rows.push(json!({
            "id": image.id,
            "file": image.file,
            "sha256": image.sha256,
            "layer": image.layer,
            "x": image.x,
            "y": image.y,
            "width": image.width,
            "rotation": image.rotation,
            "threshold": image.threshold,
            "invert": image.invert,
            "blur": image.blur,
            "min_area": image.min_area,
            "smoothing": image.smoothing,
            "trace": image.trace,
            "arcs": image.arcs,
            "shapes": shapes,
        }));
    }
    out.set("images", Value::Array(rows));
    ExitCode::SUCCESS
}

fn preview(out: &mut Out, project: &Path, id: u64, output: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let png = match black_white_png(&job, id) {
        Ok(png) => png,
        Err(err) => return out.fail(err),
    };
    if let Err(err) = std::fs::write(output, png) {
        return out.fail(format!("{}: {err}", output.display()));
    }
    out.wrote(output, "");
    ExitCode::SUCCESS
}

fn trace_text(trace: ImageTrace) -> &'static str {
    match trace {
        ImageTrace::Outline => "outlines",
        ImageTrace::CentreLine => "centre lines",
    }
}

fn threshold_text(threshold: ImageThreshold) -> String {
    match threshold {
        ImageThreshold::Auto(_) => "auto".into(),
        ImageThreshold::Level(t) => t.to_string(),
    }
}

/// The trace report: its lines, and as JSON under `image`.
pub fn reported(out: &mut Out, report: &ImageReport) {
    for line in &report.lines {
        out.line(line.clone());
    }
    out.set("image", report_json(report));
}

pub fn report_json(report: &ImageReport) -> Value {
    json!({
        "id": report.id,
        "shapes": report.shapes,
        "threshold": report.threshold,
        "width": report.width,
        "height": report.height,
        "dropped": report.dropped,
        "unsimplified": report.unsimplified,
        "without_arcs": report.without_arcs,
        "centre_lines": report.centre_lines,
        "dots": report.dots,
        "stroke_widths": report.widths.map(|(narrow, wide)| [narrow, wide]),
        "black_at_border": report.black_at_border,
    })
}

fn write(out: &mut Out, job: &Job, target: &Path) -> ExitCode {
    if let Err(err) = save_job(job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}
