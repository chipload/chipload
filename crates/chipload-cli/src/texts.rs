// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload text …`: text written into shapes for V-carving, pocketing or profiling
//! (docs/specs/text.md §6).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::{Job, TextAlign, TextAnchor, save_job};
use chipload_pipeline::texts::{
    NewText, TextReport, add_text, font_names, remake, remove_text, text_label, text_size,
    with_anchor,
};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{Value, json};

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum TextAction {
    /// Write a text into shapes on its own layer; prints its id. "\n" in the text starts a
    /// new line.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// What it says.
        text: String,
        /// The height of a capital letter, mm.
        #[arg(long)]
        height: f64,
        #[command(flatten)]
        settings: Settings,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Change a text (what it says, its font, size, place or turn) and write it again.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The text's id (as `text list` shows it).
        id: u64,
        /// What it says.
        #[arg(long)]
        text: Option<String>,
        /// The height of a capital letter, mm.
        #[arg(long)]
        height: Option<f64>,
        #[command(flatten)]
        settings: Settings,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// The project's texts: what they say, font, size, place and shapes.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Remove a text and its letters.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// The text's id.
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// The built-in fonts.
    Fonts,
}

/// How and where the text is written; unset: the default (add) or unchanged (set).
#[derive(Args)]
pub struct Settings {
    /// Where the text's anchor goes: X,Y in mm (default 0,0).
    #[arg(long, value_parser = parse_at, allow_hyphen_values = true)]
    at: Option<(f64, f64)>,
    /// What --at is: the lower-left corner of the text's box (default) or its centre. A
    /// centred text stays centred when it changes. Changed without --at, the text stays
    /// where it is.
    #[arg(long, value_enum)]
    anchor: Option<AnchorArg>,
    /// Turn it about its centre to this angle, degrees counter-clockwise (any angle).
    /// `--at` stays the corner before the turn.
    #[arg(long, allow_hyphen_values = true)]
    rotate: Option<f64>,
    /// A built-in font (`text fonts`; default FreeSans).
    #[arg(long)]
    font: Option<String>,
    /// How the lines line up (default left).
    #[arg(long, value_enum)]
    align: Option<AlignArg>,
    /// Extra space between letters, mm (default 0: the font's own).
    #[arg(long, allow_hyphen_values = true)]
    spacing: Option<f64>,
    /// From one line's base to the next, in capital heights (default 1.6).
    #[arg(long)]
    line_spacing: Option<f64>,
    /// The layer for its letters (default "text").
    #[arg(long)]
    layer: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum AlignArg {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum AnchorArg {
    Corner,
    Center,
}

impl From<AnchorArg> for TextAnchor {
    fn from(a: AnchorArg) -> Self {
        match a {
            AnchorArg::Corner => TextAnchor::Corner,
            AnchorArg::Center => TextAnchor::Center,
        }
    }
}

impl From<AlignArg> for TextAlign {
    fn from(a: AlignArg) -> Self {
        match a {
            AlignArg::Left => TextAlign::Left,
            AlignArg::Center => TextAlign::Center,
            AlignArg::Right => TextAlign::Right,
        }
    }
}

fn parse_at(text: &str) -> Result<(f64, f64), String> {
    let bad = || format!("expected X,Y in mm, got {text:?}");
    let (x, y) = text.split_once(',').ok_or_else(bad)?;
    Ok((
        x.trim().parse().map_err(|_| bad())?,
        y.trim().parse().map_err(|_| bad())?,
    ))
}

/// "\n" typed on the command line starts a new line.
fn lines(text: &str) -> String {
    text.replace("\\n", "\n")
}

/// Degrees in 0 to <360.
fn turned(degrees: f64) -> Result<f64, String> {
    if !degrees.is_finite() {
        return Err(format!("--rotate takes degrees, got {degrees}"));
    }
    let turned = degrees.rem_euclid(360.0);
    Ok(if turned >= 360.0 { 0.0 } else { turned })
}

pub fn text(out: &mut Out, action: TextAction) -> ExitCode {
    match action {
        TextAction::Add {
            project,
            text,
            height,
            settings,
            output,
        } => add(
            out,
            &project,
            &lines(&text),
            height,
            &settings,
            output.as_deref(),
        ),
        TextAction::Set {
            project,
            id,
            text,
            height,
            settings,
            output,
        } => {
            let change = |job: &mut Job| -> Result<(), String> {
                let t = job
                    .drawing
                    .texts
                    .iter_mut()
                    .find(|t| t.id == id)
                    .ok_or_else(|| format!("there is no text {id}"))?;
                if let Some(text) = &text {
                    t.text = lines(text);
                }
                if let Some(height) = height {
                    t.height = height;
                }
                // A new anchor keeps the text where it is, unless --at says where.
                if let Some(anchor) = settings.anchor {
                    let size = text_size(t).map_err(|e| e.to_string())?;
                    *t = with_anchor(t, anchor.into(), size);
                }
                if let Some((x, y)) = settings.at {
                    (t.x, t.y) = (x, y);
                }
                if let Some(degrees) = settings.rotate {
                    t.rotation = turned(degrees)?;
                }
                if let Some(font) = &settings.font {
                    t.font = font.clone();
                }
                if let Some(align) = settings.align {
                    t.align = align.into();
                }
                if let Some(spacing) = settings.spacing {
                    t.spacing = spacing;
                }
                if let Some(line_spacing) = settings.line_spacing {
                    t.line_spacing = line_spacing;
                }
                if let Some(layer) = &settings.layer {
                    t.layer = layer.clone();
                }
                Ok(())
            };
            set(out, &project, id, change, output.as_deref())
        }
        TextAction::List { project } => list(out, &project),
        TextAction::Remove {
            project,
            id,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            match remove_text(&job, id) {
                Ok(job) => {
                    out.line(format!("Removed text {id} and its letters."));
                    write(out, &job, output.as_deref().unwrap_or(&project))
                }
                Err(err) => out.fail(err),
            }
        }
        TextAction::Fonts => {
            let fonts: Vec<&str> = font_names().collect();
            for font in &fonts {
                out.line(*font);
            }
            out.set("fonts", json!(fonts));
            ExitCode::SUCCESS
        }
    }
}

fn add(
    out: &mut Out,
    project: &Path,
    text: &str,
    height: f64,
    settings: &Settings,
    output: Option<&Path>,
) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut new = NewText::new(text, height);
    if let Some((x, y)) = settings.at {
        (new.x, new.y) = (x, y);
    }
    if let Some(anchor) = settings.anchor {
        new.anchor = anchor.into();
    }
    if let Some(degrees) = settings.rotate {
        match turned(degrees) {
            Ok(r) => new.rotation = r,
            Err(err) => return out.fail(err),
        }
    }
    if let Some(font) = &settings.font {
        new.font = font;
    }
    if let Some(align) = settings.align {
        new.align = align.into();
    }
    new.spacing = settings.spacing.unwrap_or(new.spacing);
    new.line_spacing = settings.line_spacing.unwrap_or(new.line_spacing);
    if let Some(layer) = &settings.layer {
        new.layer = layer;
    }
    match add_text(&job, new) {
        Ok((job, report)) => {
            reported(out, &report);
            write(out, &job, output.unwrap_or(project))
        }
        Err(err) => out.fail(err),
    }
}

/// `place --text`: the text's position (its anchor: corner or centre) and turn, and its
/// height; written again, as `text set` does.
pub fn place_text(
    out: &mut Out,
    project: &Path,
    id: u64,
    at: Option<(f64, f64)>,
    rotate: Option<f64>,
    height: Option<f64>,
    output: Option<&Path>,
) -> ExitCode {
    let change = |job: &mut Job| -> Result<(), String> {
        let t = job
            .drawing
            .texts
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("there is no text {id}"))?;
        if let Some((x, y)) = at {
            (t.x, t.y) = (x, y);
        }
        if let Some(degrees) = rotate {
            t.rotation = turned(degrees)?;
        }
        if let Some(height) = height {
            t.height = height;
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
    match remake(&job, id) {
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
    if job.drawing.texts.is_empty() {
        out.line("No texts.");
    }
    let mut rows = Vec::new();
    for text in &job.drawing.texts {
        let shapes = job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.text() == Some(text.id))
            .count();
        out.line(format!(
            "{}: {} in {}, {} mm high, on layer {}, {} at ({}, {}){}; {shapes} {}",
            text.id,
            text_label(text),
            text.font,
            text.height,
            text.layer,
            match text.anchor {
                TextAnchor::Corner => "corner",
                TextAnchor::Center => "centre",
            },
            text.x,
            text.y,
            if text.rotation == 0.0 {
                String::new()
            } else {
                format!(
                    ", turned {}°",
                    chipload_pipeline::placement::degrees_text(text.rotation)
                )
            },
            if shapes == 1 { "shape" } else { "shapes" }
        ));
        rows.push(json!({
            "id": text.id,
            "text": text.text,
            "font": text.font,
            "height": text.height,
            "x": text.x,
            "y": text.y,
            "anchor": text.anchor,
            "rotation": text.rotation,
            "align": text.align,
            "spacing": text.spacing,
            "line_spacing": text.line_spacing,
            "layer": text.layer,
            "shapes": shapes,
        }));
    }
    out.set("texts", Value::Array(rows));
    ExitCode::SUCCESS
}

/// The report: its lines, and as JSON under `text`.
pub fn reported(out: &mut Out, report: &TextReport) {
    for line in &report.lines {
        out.line(line.clone());
    }
    out.set(
        "text",
        json!({
            "id": report.id,
            "shapes": report.shapes,
            "width": report.width,
            "height": report.height,
            "missing": report.missing.iter().collect::<String>(),
        }),
    );
}

fn write(out: &mut Out, job: &Job, target: &Path) -> ExitCode {
    if let Err(err) = save_job(job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}
