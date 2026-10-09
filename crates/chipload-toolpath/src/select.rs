// SPDX-License-Identifier: GPL-3.0-or-later
//! Match job selectors against imported contours. See docs/specs/v0-profile-pipeline.md §3.

use chipload_core::chain::{Contour, Segment};
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
    for contour in contours {
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
                "{op}: skipped open contour #{} on {} ({:.3}, {:.3}) to ({:.3}, {:.3})",
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
    // DXF layer names and handles are case-insensitive.
    if let Some(layer) = &selector.layer
        && segment.layer.to_lowercase() != layer.to_lowercase()
    {
        return false;
    }
    if let Some(handles) = &selector.handles
        && !handles.is_empty()
        && !handles
            .iter()
            .any(|h| h.to_lowercase() == segment.handle.to_lowercase())
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
    if let Some(handles) = &selector.handles
        && !handles.is_empty()
    {
        parts.push(format!("handles {}", handles.join(", ")));
    }
    if let Some(shapes) = &selector.shapes
        && !shapes.is_empty()
    {
        let ids: Vec<String> = shapes.iter().map(|id| format!("S{id}")).collect();
        parts.push(format!("shapes {}", ids.join(", ")));
    }
    parts.join(" and ")
}
