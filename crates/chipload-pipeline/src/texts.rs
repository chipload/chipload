// SPDX-License-Identifier: GPL-3.0-or-later
//! Texts in a project (docs/specs/text.md): add one, make its letters again after it
//! changes, remove it. Its letters become `path` shapes on its layer that name the text,
//! placed and turned as a picture's shapes are; the CLI and the GUI both go through these.

use chipload_core::chain::ShapeFrom;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Shape, TEXT_DEFAULT_LINE_SPACING, Text, TextAlign, TextAnchor};
use chipload_text::{Align, Layout, Lettering, letters};

use crate::PipelineError;
use crate::images::{find_left_out_again, placed_box};

pub use chipload_text::{DEFAULT_FONT, font_names};

/// The layer a new text's letters go on.
pub const DEFAULT_LAYER: &str = "text";

/// A text to add: what it says and how, and where it goes.
#[derive(Debug, Clone)]
pub struct NewText<'a> {
    pub text: &'a str,
    pub font: &'a str,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub anchor: TextAnchor,
    pub rotation: f64,
    pub align: TextAlign,
    pub spacing: f64,
    pub line_spacing: f64,
    pub layer: &'a str,
}

impl<'a> NewText<'a> {
    /// The spec's defaults (§3), at (0, 0).
    pub fn new(text: &'a str, height: f64) -> Self {
        NewText {
            text,
            font: DEFAULT_FONT,
            height,
            x: 0.0,
            y: 0.0,
            anchor: TextAnchor::Corner,
            rotation: 0.0,
            align: TextAlign::Left,
            spacing: 0.0,
            line_spacing: TEXT_DEFAULT_LINE_SPACING,
            layer: DEFAULT_LAYER,
        }
    }
}

/// What making a text's letters gave.
#[derive(Debug, Clone, PartialEq)]
pub struct TextReport {
    pub id: u64,
    /// Its letters' shape ids.
    pub shapes: Vec<u64>,
    /// The box round its letters, mm.
    pub width: f64,
    pub height: f64,
    /// Characters the font has no letter for, left out.
    pub missing: Vec<char>,
    /// The report as sentences, in English, for the command line: `messages` in English.
    pub lines: Vec<String>,
    /// The report as messages, for the app (docs/specs/translations.md §7).
    pub messages: Vec<Message>,
}

/// Add a text and make its letters. Returns the project and what was made.
pub fn add_text(job: &Job, new: NewText) -> Result<(Job, TextReport), PipelineError> {
    let id = job.drawing.texts.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let text = Text {
        id,
        text: new.text.to_string(),
        font: new.font.to_string(),
        height: new.height,
        x: new.x,
        y: new.y,
        anchor: new.anchor,
        rotation: new.rotation,
        align: new.align,
        spacing: new.spacing,
        line_spacing: new.line_spacing,
        layer: new.layer.to_string(),
    };
    let mut out = job.clone();
    out.drawing.texts.push(text);
    remake(&out, id)
}

/// The letters of `text`, in mm with their box's lower-left corner at (0, 0); outlines
/// that overlap (a connected script's joins, a letter of overlapping strokes) merged into
/// ones that don't cross (docs/specs/more-fonts.md §4).
pub fn lettering(text: &Text) -> Result<Lettering, PipelineError> {
    let mut lettering = letters(&Layout {
        text: &text.text,
        font: &text.font,
        height: text.height,
        spacing: text.spacing,
        line_spacing: text.line_spacing,
        align: match text.align {
            TextAlign::Left => Align::Left,
            TextAlign::Center => Align::Center,
            TextAlign::Right => Align::Right,
        },
    })?;
    let merged = chipload_toolpath::merge::merge_overlaps(&lettering.outlines, &lettering.letter)
        .map_err(|err| {
        PipelineError::Job(
            Message::new("pipeline-text-join-failed")
                .arg("text", text_label(text))
                .arg("error", err.message()),
        )
    })?;
    (lettering.outlines, lettering.letter) = merged.into_iter().unzip();
    Ok(lettering)
}

/// Make text `id`'s letters again with its current values, replacing its shapes.
pub fn remake(job: &Job, id: u64) -> Result<(Job, TextReport), PipelineError> {
    let (index, text) = find(job, id)?;
    crate::refuse_problems(&text.problems(index))?;
    let lettering = lettering(text)?;
    let mut out = job.clone();
    let old: Vec<Shape> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(id))
        .cloned()
        .collect();
    out.drawing.shapes.retain(|s| s.text() != Some(id));
    let mut ids = Vec::with_capacity(lettering.outlines.len());
    for outline in &lettering.outlines {
        let shape_id = out.drawing.next_shape_id;
        out.drawing.next_shape_id += 1;
        ids.push(shape_id);
        out.drawing.shapes.push(Shape {
            id: shape_id,
            layer: text.layer.clone(),
            name: None,
            geometry: Geometry::Path {
                dxf_handle: None,
                dxf_type: None,
                closed: true,
                segments: placed_box(
                    outline,
                    box_corner(text, (lettering.width, lettering.height)),
                    (lettering.width, lettering.height),
                    text.rotation,
                ),
                image: None,
                dxf: None,
                text: Some(id),
            },
        });
    }
    let mut report = report(text, &lettering, ids);
    let found = find_left_out_again(&mut out, ShapeFrom::Text(id), &old);
    report.lines.extend(found.iter().map(Message::english));
    report.messages.extend(found);
    Ok((out, report))
}

/// The lower-left corner of a text's box of `size`, before its turn: its position, or
/// half the box from it when it is the centre.
pub fn box_corner(text: &Text, size: (f64, f64)) -> (f64, f64) {
    match text.anchor {
        TextAnchor::Corner => (text.x, text.y),
        TextAnchor::Center => (text.x - size.0 / 2.0, text.y - size.1 / 2.0),
    }
}

/// `text` with its anchor changed to `anchor` and its position moved to match, so its
/// letters stay where they are (a box of `size`).
pub fn with_anchor(text: &Text, anchor: TextAnchor, size: (f64, f64)) -> Text {
    let (x0, y0) = box_corner(text, size);
    let (x, y) = match anchor {
        TextAnchor::Corner => (x0, y0),
        TextAnchor::Center => (x0 + size.0 / 2.0, y0 + size.1 / 2.0),
    };
    Text {
        x,
        y,
        anchor,
        ..text.clone()
    }
}

/// Make the letters again of every text that differs in `after` from `before` (or is new),
/// as after a `chipload set`.
pub fn remake_changed(before: &Job, after: &Job) -> Result<(Job, Vec<TextReport>), PipelineError> {
    let mut out = after.clone();
    let mut reports = Vec::new();
    for text in &after.drawing.texts {
        let old = before.drawing.texts.iter().find(|t| t.id == text.id);
        if old != Some(text) {
            let (job, report) = remake(&out, text.id)?;
            out = job;
            reports.push(report);
        }
    }
    Ok((out, reports))
}

/// Remove text `id` and its letters. Operations that select it then find nothing, which
/// the project's problems show.
pub fn remove_text(job: &Job, id: u64) -> Result<Job, PipelineError> {
    find(job, id)?;
    let mut out = job.clone();
    out.drawing.texts.retain(|t| t.id != id);
    out.drawing.shapes.retain(|s| s.text() != Some(id));
    Ok(out)
}

/// The size of text `id`'s box before its turn, mm (what Position and the canvas use).
pub fn text_size(text: &Text) -> Result<(f64, f64), PipelineError> {
    let l = lettering(text)?;
    Ok((l.width, l.height))
}

fn find(job: &Job, id: u64) -> Result<(usize, &Text), PipelineError> {
    job.drawing
        .texts
        .iter()
        .enumerate()
        .find(|(_, t)| t.id == id)
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-no-text").arg("id", id)))
}

/// The first line of a text, shortened, for reports and the Browser.
pub fn text_label(text: &Text) -> String {
    let first = text.text.lines().next().unwrap_or_default().trim();
    let mut label: String = first.chars().take(24).collect();
    if first.chars().count() > 24 || text.text.trim().lines().count() > 1 {
        label.push('…');
    }
    format!("\"{label}\"")
}

fn report(text: &Text, l: &Lettering, shapes: Vec<u64>) -> TextReport {
    let count = shapes.len();
    let mut lines = vec![
        Message::new("pipeline-text-wrote")
            .arg("text", text_label(text))
            .arg("font", text.font.as_str())
            .arg("height", Value::mm(text.height))
            .arg("width", Value::LengthNumber(l.width, Digits::Fixed(1)))
            .arg("tall", Value::Length(l.height, Digits::Fixed(1)))
            .arg("count", count)
            .arg("layer", text.layer.as_str()),
    ];
    if !l.missing.is_empty() {
        let missing: String = l.missing.iter().collect();
        lines.push(
            Message::new("pipeline-text-missing")
                .arg("font", text.font.as_str())
                .arg("missing", missing),
        );
    }
    if l.dropped > 0 {
        lines.push(Message::new("pipeline-text-dropped").arg("count", l.dropped));
    }
    TextReport {
        id: text.id,
        shapes,
        width: l.width,
        height: l.height,
        missing: l.missing.clone(),
        lines: lines.iter().map(Message::english).collect(),
        messages: lines,
    }
}
