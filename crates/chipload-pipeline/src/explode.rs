// SPDX-License-Identifier: GPL-3.0-or-later
//! Explode a DXF into drawn shapes, and join drawn polylines whose ends lie close
//! (docs/specs/explode-and-join.md). The CLI and the GUI both go through here.

use std::collections::{BTreeMap, BTreeSet};

use chipload_i18n::{Digits, Message, Value};
use chipload_job::explode::{DEFAULT_JOIN_GAP_MM, MAX_JOIN_GAP_MM};
use chipload_job::explode::{JoinOutcome, JoinPiece, gap_allowed, join, path_to_drawn};
use chipload_job::{Geometry, Job, PathSegment, Selector, Shape};

use crate::{PipelineError, ids_text, load_drawing, placement};

/// How many open ends a join's report names; the rest are counted.
const OPEN_ENDS_NAMED: usize = 5;

/// What exploding or joining did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShapesReport {
    /// The drawn shapes that came of it (exploded, or joined), in the drawing's order.
    pub shapes: Vec<u64>,
    /// Shapes that are gone: joined into another, or left with nothing to draw.
    pub removed: Vec<u64>,
    /// The report as messages, for the app.
    pub messages: Vec<Message>,
    /// The same in English, for the command line.
    pub lines: Vec<String>,
}

impl ShapesReport {
    fn finish(mut self) -> Self {
        self.lines = self.messages.iter().map(Message::english).collect();
        self
    }
}

/// "the gap must be 0.001 to 1 mm".
fn check_gap(gap: f64) -> Result<(), PipelineError> {
    if gap_allowed(gap) {
        return Ok(());
    }
    Err(PipelineError::Job(
        Message::new("pipeline-join-gap-range")
            .arg(
                "least",
                Value::Length(chipload_core::tolerances::CHAIN_TOL_MM, Digits::Shortest),
            )
            .arg("most", Value::Length(MAX_JOIN_GAP_MM, Digits::Shortest))
            .arg("got", Value::Length(gap, Digits::Shortest)),
    ))
}

/// The gap a join takes when none is given, mm.
pub fn default_gap() -> f64 {
    DEFAULT_JOIN_GAP_MM
}

/// The DXF to explode: `id`, or the project's only one.
pub fn which_dxf(job: &Job, id: Option<u64>) -> Result<u64, PipelineError> {
    match (id, job.drawing.dxfs.as_slice()) {
        (Some(id), _) => Ok(id),
        (None, [only]) => Ok(only.id),
        (None, []) => Err(PipelineError::Job(Message::new("pipeline-explode-no-dxf"))),
        (None, several) => Err(PipelineError::Job(
            Message::new("pipeline-explode-which-dxf")
                .arg("count", several.len())
                .arg("dxfs", placement::dxf_list(several)),
        )),
    }
}

/// Turn every shape of DXF `id` into drawn shapes and remove the DXF from the project
/// (§2). Shapes keep their ids and layers; a path in several runs gets new ids for the
/// runs after the first. Selectors limited to the DXF become lists of the shapes they
/// matched. With `join`, the exploded polylines are then joined with that gap (§3).
pub fn explode_dxf(
    job: &Job,
    id: u64,
    join_gap: Option<f64>,
) -> Result<(Job, ShapesReport), PipelineError> {
    let Some(source) = job.drawing.dxfs.iter().find(|d| d.id == id) else {
        return Err(placement::no_dxf(id));
    };
    if let Some(gap) = join_gap {
        check_gap(gap)?;
    }
    let matched = selected_by_dxf(job, id)?;
    let mut out = job.clone();
    let mut report = ShapesReport::default();
    // Old id → every id it is now (empty: nothing of it was left).
    let mut became: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut shapes = Vec::with_capacity(out.drawing.shapes.len());
    // Circles that started elsewhere, or ran clockwise: a drawn circle doesn't.
    let mut restarted: Vec<u64> = Vec::new();
    for shape in std::mem::take(&mut out.drawing.shapes) {
        if shape.dxf() != Some(id) {
            shapes.push(shape);
            continue;
        }
        let segments = shape.segments();
        let drawn = path_to_drawn(&segments);
        if let ([Geometry::Circle { .. }], [PathSegment::Arc(arc)]) = (&drawn[..], &segments[..])
            && (arc.start_angle != 0.0 || arc.sweep < 0.0)
        {
            restarted.push(shape.id);
        }
        let mut ids = Vec::with_capacity(drawn.len());
        for (i, geometry) in drawn.into_iter().enumerate() {
            let new_id = if i == 0 {
                shape.id
            } else {
                out.drawing.next_shape_id += 1;
                out.drawing.next_shape_id - 1
            };
            ids.push(new_id);
            report.shapes.push(new_id);
            shapes.push(Shape {
                construction: false,
                id: new_id,
                layer: shape.layer.clone(),
                name: None,
                geometry,
            });
        }
        if ids.is_empty() {
            report.removed.push(shape.id);
        }
        became.insert(shape.id, ids);
    }
    out.drawing.shapes = shapes;
    out.drawing.dxfs.retain(|d| d.id != id);
    let key = format!("dxf:{id}");
    out.drawing
        .hidden
        .retain(|h| h != &key && !h.starts_with(&format!("{key}/")));

    report.messages.push(
        Message::new("pipeline-dxf-exploded")
            .arg("kind", source.format.name())
            .arg("id", id)
            .arg("name", source.name.as_str())
            .arg("count", report.shapes.len()),
    );
    let split: Vec<u64> = became
        .iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(old, _)| *old)
        .collect();
    if !split.is_empty() {
        report.messages.push(
            Message::new("pipeline-explode-split")
                .arg("count", split.len())
                .arg("shapes", ids_text(&split)),
        );
    }
    if !restarted.is_empty() {
        report.messages.push(
            Message::new("pipeline-explode-circle-start")
                .arg("count", restarted.len())
                .arg("shapes", ids_text(&restarted)),
        );
    }
    if !report.removed.is_empty() {
        report.messages.push(
            Message::new("pipeline-explode-dropped")
                .arg("count", report.removed.len())
                .arg(
                    "tolerance",
                    Value::Length(chipload_core::tolerances::CHAIN_TOL_MM, Digits::Shortest),
                )
                .arg("shapes", ids_text(&report.removed)),
        );
    }

    for (i, op) in out.operations.iter_mut().enumerate() {
        let label = op.label_message(i);
        let Some(select) = op.select_mut() else {
            continue;
        };
        if let Some(ids) = matched.get(&i) {
            let ids: Vec<u64> = ids
                .iter()
                .flat_map(|old| became.get(old).cloned().unwrap_or_else(|| vec![*old]))
                .collect();
            if !ids.is_empty() {
                report.messages.push(
                    Message::new("pipeline-explode-selects")
                        .arg("label", label)
                        .arg("shapes", ids_text(&ids)),
                );
                *select = Selector {
                    shapes: Some(ids),
                    ..Selector::default()
                };
            }
        } else if let Some(list) = select.shapes.as_mut() {
            *list = list
                .iter()
                .flat_map(|old| became.get(old).cloned().unwrap_or_else(|| vec![*old]))
                .collect();
        }
    }

    if let Some(gap) = join_gap {
        let exploded: BTreeSet<u64> = report.shapes.iter().copied().collect();
        let (joined, join_report) = apply_join(&out, &exploded, gap);
        out = joined;
        report.shapes.retain(|s| !join_report.removed.contains(s));
        report.removed.extend(&join_report.removed);
        report.messages.extend(join_report.messages);
    }
    crate::refuse_problems(&out.drawing.validate())?;
    Ok((out, report.finish()))
}

/// For each operation (by index) limited to DXF `id`, the ids of the shapes it cuts: the
/// DXF's shapes it matches, without those in a contour with a shape it leaves out.
fn selected_by_dxf(job: &Job, id: u64) -> Result<BTreeMap<usize, Vec<u64>>, PipelineError> {
    let mut out = BTreeMap::new();
    let limited: Vec<usize> = job
        .operations
        .iter()
        .enumerate()
        .filter(|(_, op)| op.select().dxf == Some(id))
        .map(|(i, _)| i)
        .collect();
    if limited.is_empty() {
        return Ok(out);
    }
    let contours = load_drawing(job)?.contours;
    for i in limited {
        let select = job.operations[i].select();
        let excluded: BTreeSet<u64> = select.exclude.iter().flatten().copied().collect();
        // Every shape that shares a contour with a left-out one is left out with it.
        let mut left_out = excluded.clone();
        for contour in &contours {
            let ids: Vec<u64> = contour
                .parts
                .iter()
                .filter_map(|p| p.segment.shape)
                .collect();
            if ids.iter().any(|s| excluded.contains(s)) {
                left_out.extend(ids);
            }
        }
        let ids: Vec<u64> = job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.dxf() == Some(id))
            .filter(|s| {
                select
                    .layer
                    .as_ref()
                    .is_none_or(|l| s.layer.to_lowercase() == l.to_lowercase())
            })
            .filter(|s| {
                select
                    .shapes
                    .as_ref()
                    .is_none_or(|ids| ids.is_empty() || ids.contains(&s.id))
            })
            .filter(|s| !left_out.contains(&s.id))
            .map(|s| s.id)
            .collect();
        out.insert(i, ids);
    }
    Ok(out)
}

/// Join drawn shapes `ids` (none: every drawn shape) whose ends lie within `gap` mm (§3).
/// Imported shapes can't take part: explode their DXF first.
pub fn join_shapes(job: &Job, ids: &[u64], gap: f64) -> Result<(Job, ShapesReport), PipelineError> {
    check_gap(gap)?;
    let chosen: BTreeSet<u64> = if ids.is_empty() {
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.is_drawn())
            .map(|s| s.id)
            .collect()
    } else {
        for id in ids {
            let shape = job
                .drawing
                .shapes
                .iter()
                .find(|s| s.id == *id)
                .ok_or_else(|| crate::shapes::no_shape(*id))?;
            if !shape.is_drawn() {
                return Err(PipelineError::Job(
                    Message::new("pipeline-join-imported").arg("id", *id),
                ));
            }
        }
        ids.iter().copied().collect()
    };
    let (out, report) = apply_join(job, &chosen, gap);
    crate::refuse_problems(&out.drawing.validate())?;
    Ok((out, report.finish()))
}

/// Join the shapes `chosen` of `job`: the kept shapes take the joined geometry, the others
/// go, and selectors naming them name the kept one.
fn apply_join(job: &Job, chosen: &BTreeSet<u64>, gap: f64) -> (Job, ShapesReport) {
    let pieces: Vec<JoinPiece> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| chosen.contains(&s.id))
        .map(|s| JoinPiece {
            id: s.id,
            layer: &s.layer,
            geometry: &s.geometry,
        })
        .collect();
    let outcome = join(&pieces, gap);
    let mut out = job.clone();
    let mut report = ShapesReport::default();
    let mut kept_for: BTreeMap<u64, u64> = BTreeMap::new();
    for joined in &outcome.joined {
        if let Some(shape) = out.drawing.shapes.iter_mut().find(|s| s.id == joined.id) {
            shape.geometry = joined.geometry.clone();
        }
        for &piece in &joined.pieces[1..] {
            kept_for.insert(piece, joined.id);
            report.removed.push(piece);
        }
    }
    out.drawing.shapes.retain(|s| !kept_for.contains_key(&s.id));
    for op in &mut out.operations {
        if let Some(list) = op.select_mut().and_then(|s| s.shapes.as_mut()) {
            let mut seen = BTreeSet::new();
            *list = list
                .iter()
                .map(|id| *kept_for.get(id).unwrap_or(id))
                .filter(|id| seen.insert(*id))
                .collect();
        }
    }
    report.shapes = out
        .drawing
        .shapes
        .iter()
        .filter(|s| outcome.joined.iter().any(|j| j.id == s.id))
        .map(|s| s.id)
        .collect();
    report.removed.sort_unstable();
    report.messages = join_messages(&outcome, gap);
    (out, report)
}

/// The join's report: what joined, what stays open and where ends branch.
fn join_messages(outcome: &JoinOutcome, gap: f64) -> Vec<Message> {
    let mut messages = Vec::new();
    if outcome.joined.is_empty() {
        messages.push(
            Message::new("pipeline-join-none").arg("gap", Value::Length(gap, Digits::Shortest)),
        );
    } else {
        let pieces: usize = outcome.joined.iter().map(|j| j.pieces.len()).sum();
        let closed = outcome.joined.iter().filter(|j| j.closed).count();
        // Ends that already met (within the chaining tolerance) closed no gap to speak of.
        let joined = if outcome.widest > chipload_core::tolerances::CHAIN_TOL_MM {
            Message::new("pipeline-joined")
                .arg("widest", Value::Length(outcome.widest, Digits::Fixed(3)))
        } else {
            Message::new("pipeline-joined-touching")
        };
        messages.push(
            joined
                .arg("pieces", pieces)
                .arg("count", outcome.joined.len())
                .arg("closed", closed),
        );
    }
    if !outcome.open_ends.is_empty() {
        messages.push(Message::new("pipeline-join-open").arg("count", outcome.open_ends.len()));
        for end in outcome.open_ends.iter().take(OPEN_ENDS_NAMED) {
            let at = Value::Point(end.at[0], end.at[1], Digits::Fixed(3));
            messages.push(match end.nearest {
                Some(d) => Message::new("pipeline-join-open-end")
                    .arg("at", at)
                    .arg("nearest", Value::Length(d, Digits::Fixed(3))),
                None => Message::new("pipeline-join-open-alone").arg("at", at),
            });
        }
    }
    for at in &outcome.branches {
        messages.push(
            Message::new("pipeline-join-branch")
                .arg("at", Value::Point(at[0], at[1], Digits::Fixed(3))),
        );
    }
    messages
}
