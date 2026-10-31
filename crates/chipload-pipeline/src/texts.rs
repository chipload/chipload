// SPDX-License-Identifier: GPL-3.0-or-later
//! Texts in a project (docs/specs/text.md): add one, make its letters again after it
//! changes, remove it. Its letters become `path` shapes on its layer that name the text,
//! placed and turned as a picture's shapes are; the CLI and the GUI both go through these.

use chipload_core::chain::ShapeFrom;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Shape, TEXT_DEFAULT_LINE_SPACING, Text, TextAlign, TextAnchor};
use chipload_text::{Align, FontFile, Layout, Lettering, letters, letters_in};

use crate::PipelineError;
use crate::fonts::FontSource;
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
pub fn add_text(
    job: &Job,
    new: NewText,
    fonts: &dyn FontSource,
) -> Result<(Job, TextReport), PipelineError> {
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
    remake(job, &out, id, fonts)
}

/// The letters of `text`, in mm with their box's lower-left corner at (0, 0); outlines
/// that overlap (a connected script's joins, a letter of overlapping strokes) merged into
/// ones that don't cross (docs/specs/more-fonts.md §4).
/// A built-in font is found by its name, any other through `fonts`; a font that is in
/// neither gives the note that the text keeps its letters (docs/specs/system-fonts.md §5).
pub fn lettering(text: &Text, fonts: &dyn FontSource) -> Result<Lettering, PipelineError> {
    let layout = Layout {
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
    };
    let mut lettering = if chipload_text::builtin(&text.font).is_some() {
        letters(&layout)?
    } else {
        let font = fonts.find(&text.font).ok_or_else(|| no_font(text))?;
        letters_in(
            &layout,
            FontFile {
                name: &text.font,
                data: &font.data,
                index: font.index,
            },
        )?
    };
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

/// The note for a text whose font isn't on this computer.
fn font_missing(text: &Text) -> PipelineError {
    PipelineError::Job(
        Message::new("pipeline-text-font-missing")
            .arg("font", text.font.as_str())
            .arg("text", text_label(text)),
    )
}

/// The error for a new text in a font that is neither built in nor on this computer.
fn no_font(text: &Text) -> PipelineError {
    PipelineError::Job(
        Message::new("pipeline-text-no-font")
            .arg("font", text.font.as_str())
            .arg(
                "fonts",
                chipload_text::font_names().collect::<Vec<_>>().join(", "),
            ),
    )
}

/// Whether two versions of a text have the same letters, wherever they are.
fn same_letters(a: &Text, b: &Text) -> bool {
    a.text == b.text
        && a.font == b.font
        && a.height == b.height
        && a.spacing == b.spacing
        && a.line_spacing == b.line_spacing
        && a.align == b.align
}

/// Make text `id`'s letters again with its values in `job`, replacing its shapes; `before`
/// is the project before it changed. A text whose font isn't on this computer can still
/// be moved, turned and put on another layer: its stored letters are (decision 3); other
/// changes give the note.
pub fn remake(
    before: &Job,
    job: &Job,
    id: u64,
    fonts: &dyn FontSource,
) -> Result<(Job, TextReport), PipelineError> {
    let (index, text) = find(job, id)?;
    crate::refuse_problems(&text.problems(index))?;
    let old = before.drawing.texts.iter().find(|t| t.id == id);
    if !crate::fonts::available(&text.font, fonts) {
        return match old {
            Some(old) if same_letters(old, text) => Ok(moved_letters(job, old, text)),
            // A text that has letters keeps them; a new one has none to keep.
            Some(old) if old.font == text.font => Err(font_missing(text)),
            _ => Err(no_font(text)),
        };
    }
    let lettering = lettering(text, fonts)?;
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
            construction: false,
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
pub fn remake_changed(
    before: &Job,
    after: &Job,
    fonts: &dyn FontSource,
) -> Result<(Job, Vec<TextReport>), PipelineError> {
    let mut out = after.clone();
    let mut reports = Vec::new();
    for text in &after.drawing.texts {
        let old = before.drawing.texts.iter().find(|t| t.id == text.id);
        if old != Some(text) {
            let (job, report) = remake(before, &out, text.id, fonts)?;
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

/// The size of a text's box before its turn, mm (what Position and the canvas use): from
/// its letters, or from its stored shapes in `job` when its font isn't on this computer.
pub fn text_size(
    job: &Job,
    text: &Text,
    fonts: &dyn FontSource,
) -> Result<(f64, f64), PipelineError> {
    if !crate::fonts::available(&text.font, fonts) {
        return stored_size(job, text).ok_or_else(|| font_missing(text));
    }
    let l = lettering(text, fonts)?;
    Ok((l.width, l.height))
}

/// The box round a text's stored letters, turned back by its turn: (width, height), and
/// its centre where the letters are. None without letters.
fn stored_box(job: &Job, text: &Text) -> Option<((f64, f64), [f64; 2])> {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for shape in job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(text.id))
    {
        let Geometry::Path { segments, .. } = &shape.geometry else {
            continue;
        };
        for segment in segments {
            let (a, b) = segment.rotated(-text.rotation).to_primitive().bounds();
            lo = [lo[0].min(a.0), lo[1].min(a.1)];
            hi = [hi[0].max(b.0), hi[1].max(b.1)];
        }
    }
    if !(lo[0].is_finite() && hi[0].is_finite()) {
        return None;
    }
    let size = (hi[0] - lo[0], hi[1] - lo[1]);
    // The centre in the turned-back frame, turned forward again.
    let middle = chipload_job::rotated(
        [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0],
        text.rotation,
    );
    Some((size, middle))
}

fn stored_size(job: &Job, text: &Text) -> Option<(f64, f64)> {
    stored_box(job, text).map(|(size, _)| size)
}

/// Text `new`'s stored letters, from where `old` put them to where `new` puts them, on its
/// layer: moved and turned as they are, without the font.
fn moved_letters(job: &Job, old: &Text, new: &Text) -> (Job, TextReport) {
    let mut out = job.clone();
    let mut ids = Vec::new();
    let mut size = (0.0, 0.0);
    if let Some((box_size, from)) = stored_box(job, old) {
        size = box_size;
        let corner = box_corner(new, box_size);
        let to = [corner.0 + box_size.0 / 2.0, corner.1 + box_size.1 / 2.0];
        let turn = new.rotation - old.rotation;
        for shape in out
            .drawing
            .shapes
            .iter_mut()
            .filter(|s| s.text() == Some(new.id))
        {
            ids.push(shape.id);
            shape.layer = new.layer.clone();
            if let Geometry::Path { segments, .. } = &mut shape.geometry {
                for segment in segments.iter_mut() {
                    *segment = segment
                        .translated(-from[0], -from[1])
                        .rotated(turn)
                        .translated(to[0], to[1]);
                }
            }
        }
    }
    let message = Message::new("pipeline-text-moved")
        .arg("text", text_label(new))
        .arg("count", ids.len());
    let report = TextReport {
        id: new.id,
        shapes: ids,
        width: size.0,
        height: size.1,
        missing: Vec::new(),
        lines: vec![message.english()],
        messages: vec![message],
    };
    (out, report)
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
