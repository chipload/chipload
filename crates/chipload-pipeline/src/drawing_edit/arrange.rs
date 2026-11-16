// SPDX-License-Identifier: GPL-3.0-or-later
//! Aligning and distributing shapes, as units of shapes that belong together.

use std::collections::{BTreeMap, BTreeSet};

use chipload_i18n::Message;
use chipload_job::Job;

use super::{drawn, refused};
use crate::PipelineError;
use crate::shapes::no_shape;

/// Which side or middle of the boxes lines up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignTo {
    Left,
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

/// What the shapes line up with (docs/specs/align-first.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignWith {
    /// The first shape named, which stays put: the others line up with its box.
    First,
    /// The box round them all (a box selection, where no shape came first).
    All,
    /// The stock's box; one shape is enough.
    Stock,
}

type Bounds = ((f64, f64), (f64, f64));

/// What moves as one when shapes are lined up or spaced out (docs/specs/arrange-whole.md):
/// shapes that chain into one contour (the pieces of a slot), and a group selected whole,
/// its texts with it.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangeUnit {
    /// Its shapes: drawn shapes, and the letters of a whole group's texts.
    pub shapes: Vec<u64>,
    /// The groups in it, selected whole: they move with their texts.
    pub groups: Vec<u64>,
}

/// Shapes `ids` as the units they line up as, in the order their first shape is named.
/// A group counts whole when all its shapes are among `ids`; then the letters of its texts
/// may be among them too. Any other shape must be a drawn one.
pub fn arrange_units(job: &Job, ids: &[u64]) -> Result<Vec<ArrangeUnit>, PipelineError> {
    let drawing = crate::load_drawing(job)?;
    arrange_units_in(job, &drawing.contours, &drawing.twins, ids)
}

/// [`arrange_units`] with the project's contours and twins (shapes left out as duplicates,
/// with the shapes they repeat) already at hand, as the window has them. A twin goes with
/// the shapes it repeats.
pub fn arrange_units_in(
    job: &Job,
    contours: &[chipload_core::chain::Contour],
    twins: &BTreeMap<u64, BTreeSet<u64>>,
    ids: &[u64],
) -> Result<Vec<ArrangeUnit>, PipelineError> {
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            return Err(refused(Message::new("pipeline-shape-twice").arg("id", *id)));
        }
    }
    let whole: Vec<(u64, Vec<u64>)> = job
        .drawing
        .groups
        .iter()
        .map(|g| (g.id, job.drawing.group_shapes(g.id)))
        .filter(|(_, members)| !members.is_empty() && members.iter().all(|m| ids.contains(m)))
        .collect();
    let group_at = |id: u64| whole.iter().position(|(_, m)| m.contains(&id));
    for &id in ids {
        if group_at(id).is_none() {
            drawn(job, id)?;
        }
    }
    // Each id starts as its own unit; ids of one whole group, or of one contour, join.
    let mut unit: Vec<usize> = (0..ids.len()).collect();
    let join = |unit: &mut Vec<usize>, a: usize, b: usize| {
        let (ra, rb) = (root(unit, a), root(unit, b));
        if ra != rb {
            unit[ra.max(rb)] = ra.min(rb);
        }
    };
    let index = |id: u64| ids.iter().position(|&i| i == id);
    for (_, members) in &whole {
        let at: Vec<usize> = members.iter().filter_map(|&m| index(m)).collect();
        for pair in at.windows(2) {
            join(&mut unit, pair[0], pair[1]);
        }
    }
    for (twin, repeats) in twins {
        let Some(t) = index(*twin) else {
            continue;
        };
        for r in repeats.iter().filter_map(|&r| index(r)) {
            join(&mut unit, t, r);
        }
    }
    for contour in contours {
        let mut at: Vec<usize> = contour
            .parts
            .iter()
            .filter_map(|p| p.segment.shape.and_then(index))
            .collect();
        at.sort_unstable();
        at.dedup();
        for pair in at.windows(2) {
            join(&mut unit, pair[0], pair[1]);
        }
    }
    let mut units: Vec<(usize, ArrangeUnit)> = Vec::new();
    for (i, &id) in ids.iter().enumerate() {
        let r = root(&mut unit, i);
        let at = match units.iter().position(|(k, _)| *k == r) {
            Some(at) => at,
            None => {
                units.push((
                    r,
                    ArrangeUnit {
                        shapes: Vec::new(),
                        groups: Vec::new(),
                    },
                ));
                units.len() - 1
            }
        };
        let found = &mut units[at].1;
        found.shapes.push(id);
        if let Some(g) = group_at(id)
            && !found.groups.contains(&whole[g].0)
        {
            found.groups.push(whole[g].0);
        }
    }
    Ok(units.into_iter().map(|(_, u)| u).collect())
}

/// The first index of the unit `i` is in (a union-find of indices, each pointing to one
/// that is in its unit, the first pointing to itself).
fn root(unit: &mut [usize], mut i: usize) -> usize {
    while unit[i] != i {
        unit[i] = unit[unit[i]];
        i = unit[i];
    }
    i
}

/// The box round a unit's shapes.
fn unit_box(job: &Job, unit: &ArrangeUnit) -> Result<Bounds, PipelineError> {
    crate::shapes::shapes_bounds(job, &unit.shapes)
        .ok_or_else(|| no_shape(unit.shapes.first().copied().unwrap_or_default()))
}

/// Move a unit by `by`: its whole groups with their texts, its other shapes as they are.
/// They keep their ids (operations and their points follow).
fn shift(job: Job, unit: &ArrangeUnit, by: (f64, f64)) -> Result<Job, PipelineError> {
    if by == (0.0, 0.0) {
        return Ok(job);
    }
    let mut out = job;
    let mut in_groups: Vec<u64> = Vec::new();
    for &group in &unit.groups {
        in_groups.extend(out.drawing.group_shapes(group));
        out = crate::groups::transform_group(&out, group, (0.0, 0.0), by, 1.0, 0.0)?;
    }
    let free: Vec<u64> = unit
        .shapes
        .iter()
        .copied()
        .filter(|id| !in_groups.contains(id))
        .collect();
    if free.is_empty() {
        return Ok(out);
    }
    crate::shapes::transform_shapes(&out, &free, (0.0, 0.0), by, 1.0, 0.0)
}

/// Line up shapes `ids`: each unit's box side or middle `to` that of the reference
/// `with`: the first unit's (which stays put), the box round them all, or the stock's. A
/// unit is a contour's shapes or a whole group ([`arrange_units`]), in the order first
/// named; it moves whole and its shapes keep their ids. Without the stock, two units or
/// more.
pub fn align_shapes(
    job: &Job,
    ids: &[u64],
    to: AlignTo,
    with: AlignWith,
) -> Result<Job, PipelineError> {
    let units = arrange_units(job, ids)?;
    if units.is_empty() || (units.len() < 2 && with != AlignWith::Stock) {
        return Err(refused(Message::new("pipeline-align-two")));
    }
    let boxes: Vec<Bounds> = units
        .iter()
        .map(|u| unit_box(job, u))
        .collect::<Result<_, _>>()?;
    let target = match with {
        AlignWith::Stock => job.stock_bounds(),
        AlignWith::First => boxes.first().copied().unwrap_or_default(),
        AlignWith::All => boxes
            .iter()
            .copied()
            .reduce(|(lo, hi), (a, b)| {
                (
                    (lo.0.min(a.0), lo.1.min(a.1)),
                    (hi.0.max(b.0), hi.1.max(b.1)),
                )
            })
            .unwrap_or_default(),
    };
    let mut out = job.clone();
    for (unit, (lo, hi)) in units.iter().zip(boxes) {
        let ((tlo, thi), mid) = (target, |a: f64, b: f64| (a + b) / 2.0);
        let by = match to {
            AlignTo::Left => (tlo.0 - lo.0, 0.0),
            AlignTo::Right => (thi.0 - hi.0, 0.0),
            AlignTo::Centre => (mid(tlo.0, thi.0) - mid(lo.0, hi.0), 0.0),
            AlignTo::Bottom => (0.0, tlo.1 - lo.1),
            AlignTo::Top => (0.0, thi.1 - hi.1),
            AlignTo::Middle => (0.0, mid(tlo.1, thi.1) - mid(lo.1, hi.1)),
        };
        out = shift(out, unit, by)?;
    }
    Ok(out)
}

/// Even out the gaps between the boxes of shapes `ids`' units ([`arrange_units`]), along X
/// (`horizontal`) or Y: the first and last (by where their boxes start) stay, the others
/// move along that axis only. Three units or more.
pub fn distribute_shapes(job: &Job, ids: &[u64], horizontal: bool) -> Result<Job, PipelineError> {
    let units = arrange_units(job, ids)?;
    if units.len() < 3 {
        return Err(refused(Message::new("pipeline-distribute-three")));
    }
    let axis = |p: (f64, f64)| if horizontal { p.0 } else { p.1 };
    let mut spans: Vec<(usize, f64, f64)> = Vec::new();
    for (i, unit) in units.iter().enumerate() {
        let (lo, hi) = unit_box(job, unit)?;
        spans.push((i, axis(lo), axis(hi)));
    }
    spans.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let (first, last) = (spans[0], spans[spans.len() - 1]);
    let sizes: f64 = spans.iter().map(|(_, lo, hi)| hi - lo).sum();
    let gap = (last.2 - first.1 - sizes) / (spans.len() - 1) as f64;
    let mut out = job.clone();
    let mut at = first.2 + gap;
    for &(i, lo, hi) in &spans[1..spans.len() - 1] {
        let d = at - lo;
        out = shift(out, &units[i], if horizontal { (d, 0.0) } else { (0.0, d) })?;
        at += hi - lo + gap;
    }
    Ok(out)
}
