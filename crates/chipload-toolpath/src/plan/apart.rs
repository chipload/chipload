// SPDX-License-Identifier: GPL-3.0-or-later
//! Where each operation cuts, and parts and instances kept apart.

use chipload_core::Point;
use chipload_core::primitives::{Line, Primitive};
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Part};

use super::OpSpan;
use crate::ToolpathError;
use crate::geom::PrimIndex;
use crate::model::{Move, MoveKind};

/// The XY extent the tool cuts in one copy (or in all, with None): every feed move's end
/// points, plus the radius of the tool that made them. None if it cuts nothing.
pub fn cut_extent(moves: &[Move], spans: &[OpSpan], copy: Option<u32>) -> Option<(Point, Point)> {
    let mut extent: Option<(Point, Point)> = None;
    let mut at: (Option<f64>, Option<f64>) = (None, None);
    for span in spans {
        let r = span.tool_radius;
        for m in &moves[span.moves.clone()] {
            let from = at;
            at = (m.x.or(at.0), m.y.or(at.1));
            if copy.is_some_and(|c| c != span.copy) || m.kind != MoveKind::Feed {
                continue;
            }
            for (x, y) in [from, at] {
                let (Some(x), Some(y)) = (x, y) else { continue };
                let ((x0, y0), (x1, y1)) = extent.unwrap_or(((x - r, y - r), (x + r, y + r)));
                extent = Some((
                    (x0.min(x - r), y0.min(y - r)),
                    (x1.max(x + r), y1.max(y + r)),
                ));
            }
        }
    }
    extent
}

/// What one copy of a part cuts: every feed move as a segment, grouped by the radius of
/// the tool that made it, and the box round them widened by those radii.
struct CutArea {
    extent: (Point, Point),
    by_radius: Vec<(f64, PrimIndex)>,
}

/// The cut of `copy` among `spans`; None if it cuts nothing.
fn cut_area(moves: &[Move], spans: &[OpSpan], copy: u32) -> Option<CutArea> {
    let extent = cut_extent(moves, spans, Some(copy))?;
    let mut by_radius: std::collections::BTreeMap<u64, (f64, Vec<Primitive>)> = Default::default();
    let mut at: (Option<f64>, Option<f64>) = (None, None);
    for span in spans {
        for m in &moves[span.moves.clone()] {
            let from = at;
            at = (m.x.or(at.0), m.y.or(at.1));
            if span.copy != copy || m.kind != MoveKind::Feed {
                continue;
            }
            if let ((Some(x0), Some(y0)), (Some(x1), Some(y1))) = (from, at) {
                by_radius
                    .entry(span.tool_radius.to_bits())
                    .or_insert_with(|| (span.tool_radius, Vec::new()))
                    .1
                    .push(Primitive::Line(Line {
                        start: (x0, y0),
                        end: (x1, y1),
                    }));
            }
        }
    }
    let by_radius = by_radius
        .into_values()
        .map(|(radius, prims)| (radius, PrimIndex::new(prims, radius.max(1.0))))
        .collect();
    Some(CutArea { extent, by_radius })
}

/// Whether the tools cut into each other: a feed move of one copy comes closer to a feed
/// move of the other than the two radii (less the chaining tolerance, so touching is
/// allowed). The boxes decide first; the segments only where they overlap.
fn areas_overlap(a: &CutArea, b: &CutArea) -> bool {
    if !boxes_overlap(a.extent, b.extent) {
        return false;
    }
    a.by_radius.iter().any(|(ra, index_a)| {
        b.by_radius.iter().any(|(rb, index_b)| {
            let limit = ra + rb - CHAIN_TOL_MM;
            index_a.prims.iter().any(|prim| match prim {
                Primitive::Line(l) => index_b
                    .segment_distance_within(l.start, l.end, limit)
                    .is_some(),
                Primitive::Arc(_) => false,
            })
        })
    })
}

/// Copies must not cut into each other or into other parts (docs/specs/copies-grid.md
/// §4, docs/specs/parts.md §5). Copies 1 cut where drawn may overlap each other; a part
/// placed elsewhere is placed as an instance is (docs/specs/part-placement.md §3).
pub(super) fn check_parts_apart(
    job: &Job,
    moves: &[Move],
    spans: &[OpSpan],
) -> Result<(), ToolpathError> {
    // (part, copy, cut where drawn, its cut)
    let mut areas: Vec<(usize, u32, bool, CutArea)> = Vec::new();
    for (p, part) in job.parts.iter().enumerate() {
        // A surface covers the whole stock, every part's copies included
        // (docs/specs/surfacing.md §2): it overlaps them by design.
        let mine: Vec<OpSpan> = spans
            .iter()
            .filter(|s| {
                job.operations
                    .get(s.operation)
                    .is_some_and(|op| op.part() == part.id && !op.is_board_surface())
            })
            .cloned()
            .collect();
        check_instances_apart(part, moves, &mine)?;
        for (number, _) in part.in_cut_order() {
            if let Some(area) = cut_area(moves, &mine, number) {
                let as_drawn = number == 1 && !part.is_placed();
                areas.push((p, number, as_drawn, area));
            }
        }
    }
    for (k, (p, number, drawn, a)) in areas.iter().enumerate() {
        for (q, other, other_drawn, b) in &areas[..k] {
            let generated = !drawn || !other_drawn;
            if p == q || !generated || !areas_overlap(a, b) {
                continue;
            }
            let ((x0, y0), (x1, y1)) = a.extent;
            return Err(ToolpathError::Overlap(
                Message::new("toolpath-error-parts-overlap")
                    .arg("part", job.parts[*p].name.as_str())
                    .arg("copy", *number)
                    .arg("other_part", job.parts[*q].name.as_str())
                    .arg("other_copy", *other)
                    .arg("x0", Value::LengthNumber(x0, Digits::Fixed(3)))
                    .arg("x1", Value::Length(x1, Digits::Fixed(3)))
                    .arg("y0", Value::LengthNumber(y0, Digits::Fixed(3)))
                    .arg("y1", Value::Length(y1, Digits::Fixed(3))),
            ));
        }
    }
    Ok(())
}

/// Whether two boxes overlap by more than the chaining tolerance.
fn boxes_overlap(a: (Point, Point), b: (Point, Point)) -> bool {
    let ((ax0, ay0), (ax1, ay1)) = a;
    let ((bx0, by0), (bx1, by1)) = b;
    ax0 + CHAIN_TOL_MM < bx1
        && bx0 + CHAIN_TOL_MM < ax1
        && ay0 + CHAIN_TOL_MM < by1
        && by0 + CHAIN_TOL_MM < ay1
}

/// A part's instances must not cut into each other (docs/specs/instances.md §3): the
/// tools' paths are compared, so instances whose boxes overlap but whose cuts don't (shapes
/// that interleave) are allowed.
fn check_instances_apart(
    part: &Part,
    moves: &[Move],
    spans: &[OpSpan],
) -> Result<(), ToolpathError> {
    let cut: Vec<(u32, CutArea)> = part
        .spots()
        .into_iter()
        .filter_map(|(number, _)| cut_area(moves, spans, number).map(|a| (number, a)))
        .collect();
    for (k, (number, a)) in cut.iter().enumerate() {
        for (other, b) in &cut[..k] {
            if areas_overlap(a, b) {
                return Err(ToolpathError::Overlap(
                    Message::new("toolpath-error-instances-overlap")
                        .arg("part", part.name.as_str())
                        .arg("instance", *number)
                        .arg("other", *other),
                ));
            }
        }
    }
    Ok(())
}
