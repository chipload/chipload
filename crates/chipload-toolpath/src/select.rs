// SPDX-License-Identifier: GPL-3.0-or-later
//! Match job selectors against imported contours. See docs/specs/v0-profile-pipeline.md §3.

use chipload_core::chain::{Contour, Segment, ShapeFrom};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::Selector;

use crate::ToolpathError;

#[derive(Debug, Clone, PartialEq)]
pub struct Selection<'a> {
    pub contours: Vec<&'a Contour>,
    /// Open contours that matched and were skipped, without the operation's label.
    pub notes: Vec<Message>,
}

/// Closed, valid contours whose every segment matches.
///
/// Open contours that match (e.g. centrelines on a cutting layer) are skipped and listed in
/// `notes`. Partly matched or invalid contours are refused, and so is a selection with no
/// closed contour in it.
pub fn select_contours<'a>(
    contours: &'a [Contour],
    selector: &Selector,
    op: &Message,
) -> Result<Selection<'a>, ToolpathError> {
    select(contours, selector, op, false)
}

/// Every contour whose every segment matches, open or closed, for a follow path
/// (docs/specs/follow-path.md §1): it follows the drawing as drawn, so a closed contour that
/// can't be offset (it crosses itself) is followed too. Partly matched contours are refused.
pub fn select_paths<'a>(
    contours: &'a [Contour],
    selector: &Selector,
    op: &Message,
) -> Result<Selection<'a>, ToolpathError> {
    select(contours, selector, op, true)
}

fn select<'a>(
    contours: &'a [Contour],
    selector: &Selector,
    op: &Message,
    paths: bool,
) -> Result<Selection<'a>, ToolpathError> {
    let refused = |message: Message| ToolpathError::Selector(message).with_context(op.clone());
    // Every selected shape must be part of some contour; otherwise it was deleted, or chaining
    // dropped it (a duplicate or zero-length segment). Never skip it silently.
    if let Some(ids) = &selector.shapes {
        let present: std::collections::BTreeSet<u64> = contours
            .iter()
            .flat_map(|c| c.parts.iter().filter_map(|p| p.segment.shape))
            .collect();
        let missing: Vec<String> = ids
            .iter()
            .filter(|id| !present.contains(id))
            .map(|id| format!("S{id}"))
            .collect();
        if !missing.is_empty() {
            return Err(refused(
                Message::new("toolpath-error-shape-missing").arg("shapes", missing.join(", ")),
            ));
        }
    }
    let mut selected = Vec::new();
    let mut skipped: Vec<&Contour> = Vec::new();
    // Shapes left out of a whole DXF, image or text: their contours, whole, aren't selected
    // (docs/specs/leave-out-shapes.md §3).
    let left_out = |contour: &Contour| {
        selector.exclude.as_ref().is_some_and(|ids| {
            contour
                .parts
                .iter()
                .any(|p| p.segment.shape.is_some_and(|s| ids.contains(&s)))
        })
    };
    for contour in contours {
        if left_out(contour) {
            continue;
        }
        let matches: Vec<bool> = contour
            .parts
            .iter()
            .map(|p| matches(&p.segment, selector))
            .collect();
        if !matches.iter().any(|&m| m) {
            continue;
        }
        if !matches.iter().all(|&m| m) {
            return Err(refused(
                Message::new("toolpath-error-partly-selected")
                    .arg("contour", contour.id)
                    .arg("layers", contour.layers().join(", ")),
            ));
        }
        if paths {
            selected.push(contour);
            continue;
        }
        if !contour.closed {
            skipped.push(contour);
            continue;
        }
        if let Some(reason) = &contour.invalid_reason {
            return Err(refused(
                Message::new("toolpath-error-contour-invalid")
                    .arg("contour", contour.id)
                    .arg("reason", reason),
            ));
        }
        selected.push(contour);
    }
    if selected.is_empty() {
        let ids: Vec<String> = skipped.iter().map(|c| format!("#{}", c.id)).collect();
        return Err(refused(if skipped.is_empty() {
            Message::new("toolpath-error-selects-nothing").arg("selector", describe(selector))
        } else {
            Message::new("toolpath-error-selects-open")
                .arg("selector", describe(selector))
                .arg("contours", ids.join(", "))
        }));
    }
    let notes = skipped
        .iter()
        .map(|c| {
            let (s, e) = (c.start(), c.end());
            Message::new("toolpath-skipped-open")
                .arg("contour", c.id)
                .arg("layers", c.layers().join(", "))
                .arg("from", Value::Point(s.0, s.1, Digits::Fixed(3)))
                .arg("to", Value::Point(e.0, e.1, Digits::Fixed(3)))
        })
        .collect();
    Ok(Selection {
        contours: selected,
        notes,
    })
}

fn matches(segment: &Segment, selector: &Selector) -> bool {
    if let Some(dxf) = selector.dxf
        && segment.from != Some(ShapeFrom::Dxf(dxf))
    {
        return false;
    }
    if let Some(image) = selector.image
        && segment.from != Some(ShapeFrom::Image(image))
    {
        return false;
    }
    if let Some(text) = selector.text
        && segment.from != Some(ShapeFrom::Text(text))
    {
        return false;
    }
    // DXF layer names are case-insensitive.
    if let Some(layer) = &selector.layer
        && segment.layer.to_lowercase() != layer.to_lowercase()
    {
        return false;
    }
    if let Some(shapes) = &selector.shapes
        && !shapes.is_empty()
        && !segment.shape.is_some_and(|id| shapes.contains(&id))
    {
        return false;
    }
    true
}

/// The selector in words: "layer 'cut' and shapes S1, S2 of DXF 1 but S3"; no words for a
/// selector that names nothing.
fn describe(selector: &Selector) -> Value {
    let shape_list = |ids: &[u64]| {
        ids.iter()
            .map(|id| format!("S{id}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut parts = Vec::new();
    if let Some(layer) = &selector.layer {
        parts.push(Message::new("toolpath-error-selector-layer").arg("layer", layer));
    }
    if let Some(shapes) = &selector.shapes
        && !shapes.is_empty()
    {
        parts
            .push(Message::new("toolpath-error-selector-shapes").arg("shapes", shape_list(shapes)));
    }
    let text = parts.into_iter().reduce(|first, second| {
        Message::new("toolpath-error-selector-and")
            .arg("first", first)
            .arg("second", second)
    });
    let mut source = match (selector.dxf, selector.image, selector.text) {
        (Some(dxf), _, _) => Message::new("toolpath-error-selector-dxf").arg("id", dxf),
        (None, Some(image), _) => Message::new("toolpath-error-selector-image").arg("id", image),
        (None, None, Some(id)) => Message::new("toolpath-error-selector-text").arg("id", id),
        (None, None, None) => {
            return text.map_or_else(|| Value::Text(String::new()), Value::from);
        }
    };
    if let Some(exclude) = selector.exclude.as_ref().filter(|e| !e.is_empty()) {
        source = Message::new("toolpath-error-selector-but")
            .arg("source", source)
            .arg("shapes", shape_list(exclude));
    }
    match text {
        None => source.into(),
        Some(text) => Message::new("toolpath-error-selector-of")
            .arg("what", text)
            .arg("source", source)
            .into(),
    }
}
