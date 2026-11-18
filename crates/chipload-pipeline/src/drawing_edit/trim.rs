// SPDX-License-Identifier: GPL-3.0-or-later
//! Trimming, breaking and extending drawn shapes at other shapes.

use std::collections::BTreeSet;

use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_draw::offset::{self, Chain};
use chipload_i18n::Message;
use chipload_job::{Geometry, Job, Shape};

use crate::PipelineError;
use crate::shapes::reshape;

use super::corners::shown_cutters;
use super::{Edited, add_all, chain_of, drawn, polyline_of, refused};
pub use chipload_draw::trim::End;
use chipload_draw::trim::{self, TrimError};

fn trim_refusal(id: u64, why: TrimError) -> PipelineError {
    let key = match why {
        TrimError::NothingCrosses => "pipeline-trim-nothing-crosses",
        TrimError::EndNotStraight => "pipeline-extend-not-straight",
        TrimError::NothingAhead => "pipeline-extend-nothing-ahead",
        TrimError::AtAnEnd => "pipeline-break-at-end",
        TrimError::Closed => "pipeline-extend-closed",
    };
    refused(Message::new(key).arg("id", id))
}

/// New shapes `ids` that took the place of shape `of` join its group, if it is in one
/// (docs/specs/shape-groups.md §4: a trimmed polyline's pieces stay in its group).
pub(super) fn join_group_of(job: &mut Job, of: u64, ids: &[u64]) {
    let Some(id) = job.drawing.group_of(of) else {
        return;
    };
    if let Some(group) = job.drawing.groups.iter_mut().find(|g| g.id == id) {
        let new: Vec<u64> = ids
            .iter()
            .copied()
            .filter(|s| !group.shapes.contains(s))
            .collect();
        group.shapes.extend(new);
    }
}

/// Shape `id` replaced by `pieces`: the first keeps its id (operations follow it), the
/// others are new shapes on its layer.
fn replace_by_pieces(job: &Job, shape: &Shape, pieces: &[Chain]) -> Result<Edited, PipelineError> {
    let Some((first, rest)) = pieces.split_first() else {
        return Err(refused(
            Message::new("pipeline-trim-everything").arg("id", shape.id),
        ));
    };
    let out = reshape(job, shape.id, polyline_of(first))?;
    let mut new_ids = Vec::new();
    let mut notes = Vec::new();
    let mut out = if rest.is_empty() {
        out
    } else {
        add_all(
            out,
            shape,
            rest.iter().map(polyline_of).collect(),
            &mut new_ids,
            &mut notes,
        )?
    };
    join_group_of(&mut out, shape.id, &new_ids);
    Ok(Edited {
        job: out,
        new_ids,
        notes,
        joined: None,
    })
}

/// Trim drawn shape `id` at `at`: the piece under it, between the nearest crossings with
/// the shown shapes, goes. An open shape cut in its middle becomes two (the first keeps
/// the id); a closed one becomes one open polyline.
pub fn trim_shape(job: &Job, id: u64, at: (f64, f64)) -> Result<Edited, PipelineError> {
    let shape = drawn(job, id)?.clone();
    let pieces = trim::trim(&chain_of(&shape), &shown_cutters(job), at)
        .map_err(|why| trim_refusal(id, why))?;
    let edited = replace_by_pieces(job, &shape, &pieces)?;
    joined_where_ends_meet(edited, &shape)
}

/// The ends of an open polyline.
fn open_ends(geometry: &Geometry) -> Option<[[f64; 2]; 2]> {
    match geometry {
        Geometry::Polyline {
            closed: false,
            points,
            ..
        } if points.len() >= 2 => Some([points[0], points[points.len() - 1]]),
        _ => None,
    }
}

/// After a trim of `shape`: its pieces joined with the drawn open polylines on its layer
/// whose ends meet theirs (within the chaining tolerance), and with those meeting these, as
/// one shape (the user, 2026-10-29: a slot trimmed by a line stayed two polylines). Never
/// construction geometry; nothing joins where three ends meet.
fn joined_where_ends_meet(edited: Edited, shape: &Shape) -> Result<Edited, PipelineError> {
    let layer = shape.layer.to_lowercase();
    let candidates: Vec<(u64, [[f64; 2]; 2])> = edited
        .job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.is_drawn() && !s.construction && s.layer.to_lowercase() == layer)
        .filter_map(|s| open_ends(&s.geometry).map(|ends| (s.id, ends)))
        .collect();
    let meets = |a: &[[f64; 2]; 2], b: &[[f64; 2]; 2]| {
        a.iter().any(|p| {
            b.iter().any(|q| {
                chipload_core::primitives::distance((p[0], p[1]), (q[0], q[1])) <= CHAIN_TOL_MM
            })
        })
    };
    let mut group: BTreeSet<u64> = std::iter::once(shape.id)
        .chain(edited.new_ids.iter().copied())
        .filter(|id| candidates.iter().any(|c| c.0 == *id))
        .collect();
    loop {
        let more: Vec<u64> = candidates
            .iter()
            .filter(|c| !group.contains(&c.0))
            .filter(|c| {
                candidates
                    .iter()
                    .filter(|g| group.contains(&g.0))
                    .any(|g| meets(&g.1, &c.1))
            })
            .map(|c| c.0)
            .collect();
        if more.is_empty() {
            break;
        }
        group.extend(more);
    }
    if group.len() < 2 {
        // One piece whose own ends meet (a shape joined by an earlier trim, its last
        // protruding end trimmed now) closes.
        // Left open if closing would make no valid shape.
        let mut edited = edited;
        let mut job = edited.job.clone();
        if group.contains(&shape.id)
            && close_if_ends_meet(&mut job, shape.id)
            && job.drawing.validate().is_empty()
        {
            edited.job = job;
            edited.joined = Some(shape.id);
        }
        return Ok(edited);
    }
    let (job, report) = crate::explode::apply_join(&edited.job, &group, CHAIN_TOL_MM);
    let Some(&kept) = report.shapes.first() else {
        return Ok(edited);
    };
    crate::refuse_problems(&job.drawing.validate())?;
    let mut notes = edited.notes;
    notes.extend(report.messages.into_iter().take(1));
    Ok(Edited {
        job,
        new_ids: edited
            .new_ids
            .into_iter()
            .filter(|id| !report.removed.contains(id))
            .collect(),
        notes,
        joined: Some(kept),
    })
}

/// Close drawn open polyline `id` when its ends meet (within the chaining tolerance): its
/// last point goes, the last segment's bulge closing it. Whether it did.
fn close_if_ends_meet(job: &mut Job, id: u64) -> bool {
    let Some(shape) = job.drawing.shapes.iter_mut().find(|s| s.id == id) else {
        return false;
    };
    let Geometry::Polyline {
        closed,
        points,
        bulges,
        ..
    } = &mut shape.geometry
    else {
        return false;
    };
    let n = points.len();
    if *closed || n < 3 {
        return false;
    }
    let (first, last) = (points[0], points[n - 1]);
    if chipload_core::primitives::distance((first[0], first[1]), (last[0], last[1])) > CHAIN_TOL_MM
    {
        return false;
    }
    points.pop();
    bulges.truncate(n - 1);
    *closed = true;
    true
}

/// The piece a trim of drawn shape `id` at `at` would remove, for a preview.
pub fn trim_preview(job: &Job, id: u64, at: (f64, f64)) -> Option<Geometry> {
    let shape = drawn(job, id).ok()?;
    trim::trimmed_piece(&chain_of(shape), &shown_cutters(job), at).map(|c| polyline_of(&c))
}

/// Break drawn shape `id` at the point of it nearest `at`: an open shape becomes two, a
/// closed one opens there.
pub fn break_shape(job: &Job, id: u64, at: (f64, f64)) -> Result<Edited, PipelineError> {
    let shape = drawn(job, id)?.clone();
    let pieces = trim::break_at(&chain_of(&shape), at).map_err(|why| trim_refusal(id, why))?;
    replace_by_pieces(job, &shape, &pieces)
}

/// Extend drawn open shape `id`'s straight `end` along itself to the nearest shown shape
/// ahead.
pub fn extend_shape(job: &Job, id: u64, end: End) -> Result<Edited, PipelineError> {
    let shape = drawn(job, id)?.clone();
    let longer = trim::extend(&chain_of(&shape), &shown_cutters(job), end)
        .map_err(|why| trim_refusal(id, why))?;
    Ok(Edited {
        job: reshape(job, id, polyline_of(&longer))?,
        new_ids: Vec::new(),
        notes: Vec::new(),
        joined: None,
    })
}

/// Of shapes `ids`, the drawn one nearest `at` (a contour can join several drawn
/// polylines end to end; a canvas tool works on the one under the cursor).
pub fn nearest_drawn(job: &Job, ids: &[u64], at: (f64, f64)) -> Option<u64> {
    ids.iter()
        .filter_map(|&id| drawn(job, id).ok())
        .map(|shape| (offset::nearest_distance(&chain_of(shape), at), shape.id))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// The point of drawn shape `id` nearest `at`: where a break there falls.
pub fn nearest_point(job: &Job, id: u64, at: (f64, f64)) -> Option<(f64, f64)> {
    let shape = drawn(job, id).ok()?;
    chain_of(shape)
        .prims
        .iter()
        .map(|prim| chipload_draw::intersect::nearest_on(prim, at))
        .min_by(|a, b| {
            let d = chipload_core::primitives::distance;
            d(*a, at).total_cmp(&d(*b, at))
        })
}

/// The end of open drawn shape `id` nearer `at`.
pub fn nearer_end(job: &Job, id: u64, at: (f64, f64)) -> Option<End> {
    let shape = drawn(job, id).ok()?;
    let chain = chain_of(shape);
    if chain.closed {
        return None;
    }
    let start = chain.prims.first()?.start();
    let end = chain.prims.last()?.end();
    let d = chipload_core::primitives::distance;
    Some(if d(at, start) <= d(at, end) {
        End::Start
    } else {
        End::End
    })
}

// -- union, subtract, intersect (§5)
