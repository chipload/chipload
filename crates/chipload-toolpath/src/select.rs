// SPDX-License-Identifier: GPL-3.0-or-later
//! Match job selectors against imported contours. See docs/specs/v0-profile-pipeline.md §3.

use chipload_core::chain::{Contour, Segment, ShapeFrom};
use chipload_job::Selector;

use crate::ToolpathError;

#[derive(Debug, Clone, PartialEq)]
pub struct Selection<'a> {
    pub contours: Vec<&'a Contour>,
    /// Open contours that matched and were skipped.
    pub notes: Vec<String>,
}

/// Closed, valid contours whose every segment matches.
///
/// Open contours that match (e.g. centrelines on a cutting layer) are skipped and listed in
/// `notes`. Partly matched or invalid contours are refused, and so is a selection with no
/// closed contour in it.
pub fn select_contours<'a>(
    contours: &'a [Contour],
    selector: &Selector,
    op: &str,
) -> Result<Selection<'a>, ToolpathError> {
    select(contours, selector, op, false)
}

/// Every contour whose every segment matches, open or closed, for a follow path
/// (docs/specs/follow-path.md §1): it follows the drawing as drawn, so a closed contour that
/// can't be offset (it crosses itself) is followed too. Partly matched contours are refused.
pub fn select_paths<'a>(
    contours: &'a [Contour],
    selector: &Selector,
    op: &str,
) -> Result<Selection<'a>, ToolpathError> {
    select(contours, selector, op, true)
}

fn select<'a>(
    contours: &'a [Contour],
    selector: &Selector,
    op: &str,
    paths: bool,
) -> Result<Selection<'a>, ToolpathError> {
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
            return Err(ToolpathError::Selector(format!(
                "{op}: shape {} is not part of any contour (deleted, or a duplicate or \
                 zero-length segment the import removed)",
                missing.join(", ")
            )));
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
            return Err(ToolpathError::Selector(format!(
                "{op}: contour #{} is only partly selected (layers {}); fix the drawing or \
                 the selector",
                contour.id,
                contour.layers().join(", ")
            )));
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
            return Err(ToolpathError::Selector(format!(
                "{op}: contour #{} is invalid: {reason}",
                contour.id
            )));
        }
        selected.push(contour);
    }
    if selected.is_empty() {
        let ids: Vec<String> = skipped.iter().map(|c| format!("#{}", c.id)).collect();
        return Err(ToolpathError::Selector(if skipped.is_empty() {
            format!("{op}: selector {} matches nothing", describe(selector))
        } else {
            format!(
                "{op}: selector {} matches only open contours ({}); this operation needs \
                 closed ones",
                describe(selector),
                ids.join(", ")
            )
        }));
    }
    let notes = skipped
        .iter()
        .map(|c| {
            let (s, e) = (c.start(), c.end());
            format!(
                "{op}: skipped open contour #{} on {} ({:.3}, {:.3}) mm to ({:.3}, {:.3}) mm",
                c.id,
                c.layers().join(", "),
                s.0,
                s.1,
                e.0,
                e.1
            )
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

fn describe(selector: &Selector) -> String {
    let mut parts = Vec::new();
    if let Some(layer) = &selector.layer {
        parts.push(format!("layer '{layer}'"));
    }
    if let Some(shapes) = &selector.shapes
        && !shapes.is_empty()
    {
        let ids: Vec<String> = shapes.iter().map(|id| format!("S{id}")).collect();
        parts.push(format!("shapes {}", ids.join(", ")));
    }
    let text = parts.join(" and ");
    let mut source = match (selector.dxf, selector.image, selector.text) {
        (Some(dxf), _, _) => format!("DXF {dxf}"),
        (None, Some(image), _) => format!("image {image}"),
        (None, None, Some(id)) => format!("text {id}"),
        (None, None, None) => return text,
    };
    if let Some(exclude) = selector.exclude.as_ref().filter(|e| !e.is_empty()) {
        let ids: Vec<String> = exclude.iter().map(|id| format!("S{id}")).collect();
        source += &format!(" but {}", ids.join(", "));
    }
    if text.is_empty() {
        source
    } else {
        format!("{text} of {source}")
    }
}
