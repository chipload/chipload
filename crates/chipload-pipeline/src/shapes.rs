// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing in a project: add, change and delete drawn shapes (docs/specs/drawing-tools.md).
//! The CLI and the GUI both go through these, so they follow the same rules.

use chipload_core::math;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::Message;
use chipload_job::edit::{Mirror, mirrored, transformed, translated};
use chipload_job::{Geometry, Job, Operation, Shape};

use crate::{PipelineError, ids_text, load_drawing, op_label};

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// "there is no shape 7".
pub fn no_shape(id: u64) -> PipelineError {
    refused(Message::new("pipeline-no-shape").arg("id", id))
}

/// Add a drawn shape. It takes `next_shape_id`, which then grows (ids are never reused).
/// Returns the project and the new id.
pub fn add_shape(job: &Job, layer: &str, geometry: Geometry) -> Result<(Job, u64), PipelineError> {
    if matches!(geometry, Geometry::Path { .. }) {
        return Err(refused(Message::new("pipeline-shape-not-drawable")));
    }
    let mut out = job.clone();
    let id = out.drawing.next_shape_id;
    out.drawing.next_shape_id += 1;
    out.drawing.shapes.push(Shape {
        construction: false,
        id,
        layer: layer.to_string(),
        name: None,
        geometry,
    });
    checked(out).map(|job| (job, id))
}

/// Replace a drawn shape's layer and defining values, keeping its id, so operations that
/// select it follow the change. Its kind can't change; imported shapes can't be edited.
pub fn set_shape(
    job: &Job,
    id: u64,
    layer: &str,
    geometry: Geometry,
) -> Result<Job, PipelineError> {
    let mut out = job.clone();
    let shape = drawn_mut(&mut out, id)?;
    if std::mem::discriminant(&shape.geometry) != std::mem::discriminant(&geometry) {
        return Err(refused(
            Message::new("pipeline-shape-kind")
                .arg("id", id)
                .arg("kind", shape.kind_name()),
        ));
    }
    // A slot stays a slot when its size changes (docs/specs/drawing-input.md §4).
    let geometry = chipload_job::edit::kept_slot(&shape.geometry, geometry);
    shape.layer = layer.to_string();
    let old = std::mem::replace(&mut shape.geometry, geometry.clone());
    // The operations' points on it follow it (docs/specs/dogbone-per-corner.md §4).
    follow_points(&mut out, job, &[id], |p| {
        chipload_job::edit::followed_point(p, &old, &geometry)
    });
    checked(out)
}

/// Replace drawn shape `id`'s values, its kind too (an editing operation that turns a
/// rectangle into a polyline), keeping its id and layer so operations follow it. Points on
/// it follow where the kind stays (as [`set_shape`]).
pub fn reshape(job: &Job, id: u64, geometry: Geometry) -> Result<Job, PipelineError> {
    let mut out = job.clone();
    let shape = drawn_mut(&mut out, id)?;
    let old = std::mem::replace(&mut shape.geometry, geometry.clone());
    follow_points(&mut out, job, &[id], |p| {
        chipload_job::edit::followed_point(p, &old, &geometry)
    });
    checked(out)
}

/// Every start point, corner choice and tab placed by hand whose nearest shape (among
/// those its operation selects, in `before`) is one of `ids`, moved by `map` (None: it
/// stays).
pub(crate) fn follow_points(
    out: &mut Job,
    before: &Job,
    ids: &[u64],
    map: impl Fn([f64; 2]) -> Option<[f64; 2]>,
) {
    for op in &mut out.operations {
        let select = op.select().clone();
        let on_moved = |p: [f64; 2]| {
            crate::placement::nearest_is(&before.drawing.shapes, &select, (p[0], p[1]), |s| {
                ids.contains(&s.id)
            })
        };
        let mut follow = |p: &mut [f64; 2]| {
            if on_moved(*p)
                && let Some(q) = map(*p)
            {
                *p = q;
            }
        };
        op.starts_mut().into_iter().flatten().for_each(&mut follow);
        for choice in op.corner_choices_mut().into_iter().flatten() {
            follow(&mut choice.at);
        }
        op.tab_points_mut()
            .into_iter()
            .flatten()
            .for_each(&mut follow);
    }
}

/// Make drawn shapes `ids` construction geometry (shown and snapped to, never cut) or
/// ordinary shapes again (docs/specs/dxf-export.md §B2). Returns the project and a note per
/// operation that names one of them by id: it can't run while they are construction.
pub fn set_construction(
    job: &Job,
    ids: &[u64],
    construction: bool,
) -> Result<(Job, Vec<Message>), PipelineError> {
    for id in ids {
        if job
            .drawing
            .shapes
            .iter()
            .any(|s| s.id == *id && !s.is_drawn())
        {
            return Err(refused(
                Message::new("pipeline-construction-imported").arg("id", *id),
            ));
        }
    }
    drawn_shapes(job, ids)?;
    let mut out = job.clone();
    for shape in out
        .drawing
        .shapes
        .iter_mut()
        .filter(|s| ids.contains(&s.id))
    {
        shape.construction = construction;
    }
    let notes = if construction {
        out.operations
            .iter()
            .enumerate()
            .filter(|(_, op)| {
                selected_ids(op).is_some_and(|sel| sel.iter().any(|i| ids.contains(i)))
            })
            .map(|(i, op)| {
                Message::new("pipeline-construction-used").arg("label", op.label_message(i))
            })
            .collect()
    } else {
        Vec::new()
    };
    checked(out).map(|job| (job, notes))
}

/// Name drawn shape `id` (docs/specs/drawn-shapes-box.md §4): trimmed; an empty name
/// takes its name away.
pub fn name_shape(job: &Job, id: u64, name: &str) -> Result<Job, PipelineError> {
    let mut out = job.clone();
    let shape = drawn_mut(&mut out, id)?;
    let name = name.trim();
    shape.name = (!name.is_empty()).then(|| name.to_string());
    checked(out)
}

/// Delete a drawn shape. Operations that selected it are flagged, never fixed: their
/// selector keeps the id and they can't run until their geometry is chosen again. Returns
/// the project and one line per flagged operation.
pub fn delete_shape(job: &Job, id: u64) -> Result<(Job, Vec<String>), PipelineError> {
    let mut out = job.clone();
    drawn_mut(&mut out, id)?;
    out.drawing.shapes.retain(|s| s.id != id);
    out.drawing.tidy_groups();
    let flagged = out
        .operations
        .iter()
        .enumerate()
        .filter(|(_, op)| selected_ids(op).is_some_and(|ids| ids.contains(&id)))
        .map(|(i, op)| {
            format!(
                "{}: shape {} no longer exists; choose its geometry again",
                op_label(i, op),
                ids_text(&[id])
            )
        })
        .collect();
    // Not `checked`: a flagged selector is allowed to point at a missing shape.
    Ok((out, flagged))
}

/// The drawn shapes `ids`, in the drawing's order: each must exist and be drawn, and none may
/// be named twice.
pub(crate) fn drawn_shapes<'a>(job: &'a Job, ids: &[u64]) -> Result<Vec<&'a Shape>, PipelineError> {
    if ids.is_empty() {
        return Err(refused(Message::new("pipeline-no-shapes-given")));
    }
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            return Err(refused(Message::new("pipeline-shape-twice").arg("id", *id)));
        }
        let shape = job
            .drawing
            .shapes
            .iter()
            .find(|s| s.id == *id)
            .ok_or_else(|| no_shape(*id))?;
        if !shape.is_drawn() {
            return Err(refused(
                Message::new("pipeline-shape-imported-copy").arg("id", *id),
            ));
        }
    }
    Ok(job
        .drawing
        .shapes
        .iter()
        .filter(|s| ids.contains(&s.id))
        .collect())
}

/// Drawn shapes `ids` mirrored about the line through `a` and `b`
/// (docs/specs/drawing-improvements.md §1). In place (`copy` false) they keep their ids,
/// and the operations' start points, corner choices and hand-placed tabs on them follow.
/// As copies, each mirror image is a new shape on its original's layer, except that an
/// open polyline or spline with both ends on the line joins its image into one closed
/// shape that keeps its id. Returns the edit and the ids of the shapes joined so.
pub fn mirror_shapes_about(
    job: &Job,
    ids: &[u64],
    a: [f64; 2],
    b: [f64; 2],
    copy: bool,
) -> Result<(crate::drawing_edit::Edited, Vec<u64>), PipelineError> {
    drawn_shapes(job, ids)?;
    let finite = a.iter().chain(&b).all(|v| v.is_finite());
    if !finite || math::hypot(b[0] - a[0], b[1] - a[1]) < CHAIN_TOL_MM {
        return Err(refused(Message::new("pipeline-mirror-line")));
    }
    let mut out = job.clone();
    let mut new_ids = Vec::new();
    let mut joined = Vec::new();
    for &id in ids {
        let Some(index) = out.drawing.shapes.iter().position(|s| s.id == id) else {
            continue;
        };
        let shape = out.drawing.shapes[index].clone();
        let Some(image) = chipload_job::edit::mirrored_about(&shape.geometry, a, b) else {
            continue;
        };
        if !copy {
            out.drawing.shapes[index].geometry = image;
        } else if let Some(whole) = joined_with_image(&shape.geometry, &image, a, b) {
            out.drawing.shapes[index].geometry = whole;
            joined.push(id);
        } else {
            let new_id = out.drawing.next_shape_id;
            out.drawing.next_shape_id += 1;
            out.drawing.shapes.push(Shape {
                construction: shape.construction,
                id: new_id,
                layer: shape.layer.clone(),
                name: None,
                geometry: image,
            });
            new_ids.push(new_id);
        }
    }
    if !copy {
        follow_points(&mut out, job, ids, |p| {
            Some(chipload_job::edit::reflected_point(p, a, b))
        });
    }
    let mut notes = Vec::new();
    if !joined.is_empty() {
        notes.push(Message::new("pipeline-mirror-joined").arg("count", joined.len()));
    }
    let mut layers: Vec<(String, usize)> = Vec::new();
    for shape in out
        .drawing
        .shapes
        .iter()
        .filter(|s| new_ids.contains(&s.id))
    {
        if shape.construction {
            continue;
        }
        match layers.iter_mut().find(|(l, _)| *l == shape.layer) {
            Some((_, n)) => *n += 1,
            None => layers.push((shape.layer.clone(), 1)),
        }
    }
    notes.extend(
        layers
            .iter()
            .filter_map(|(layer, n)| crate::drawing_edit::layer_cutters(&out, layer, *n)),
    );
    let edited = crate::drawing_edit::Edited {
        job: checked(out)?,
        new_ids,
        notes,
        joined: None,
    };
    Ok((edited, joined))
}

/// An open polyline or spline whose two ends lie on the line through `a` and `b`, joined
/// with its mirror `image` into one closed shape: its own points, then the image's back
/// from its next-to-last to its second (the ends on the line are shared). A polyline's
/// returning segments bend as their originals did (an image's bulge is negated, and
/// running it backwards negates it again). None for anything else, or a shape of one
/// segment, which lies along the line.
fn joined_with_image(
    geometry: &Geometry,
    image: &Geometry,
    a: [f64; 2],
    b: [f64; 2],
) -> Option<Geometry> {
    let on_line = |p: [f64; 2]| {
        let len = math::hypot(b[0] - a[0], b[1] - a[1]);
        let cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        (cross / len).abs() <= CHAIN_TOL_MM
    };
    let ends_on = |points: &[[f64; 2]]| {
        points.len() >= 3
            && points.first().is_some_and(|&p| on_line(p))
            && points.last().is_some_and(|&p| on_line(p))
    };
    match (geometry, image) {
        (
            Geometry::Polyline {
                closed: false,
                points,
                bulges,
                radii,
            },
            Geometry::Polyline {
                points: mirrored, ..
            },
        ) if ends_on(points) => {
            let n = points.len();
            let mut all = points.clone();
            all.extend(mirrored[1..n - 1].iter().rev());
            let mut bends = bulges.clone();
            bends.extend(bulges.iter().rev());
            // Each rounded corner and its image keep the radius; the two on the line stay
            // sharp (docs/specs/polyline-radii.md §5).
            let rounds = if radii.len() == n {
                let mut rounds = radii.clone();
                rounds.extend(radii[1..n - 1].iter().rev());
                rounds
            } else {
                Vec::new()
            };
            Some(Geometry::Polyline {
                closed: true,
                points: all,
                bulges: bends,
                radii: rounds,
            })
        }
        (
            Geometry::Spline {
                closed: false,
                points,
            },
            Geometry::Spline {
                points: mirrored, ..
            },
        ) if ends_on(points) => {
            let n = points.len();
            let mut all = points.clone();
            all.extend(mirrored[1..n - 1].iter().rev());
            Some(Geometry::Spline {
                closed: true,
                points: all,
            })
        }
        _ => None,
    }
}

/// The ends of drawn shape `id` when it is one straight line (a mirror line,
/// docs/specs/drawing-improvements.md §1); the straight side of a polyline nearest `near`
/// when given. None when it has no straight piece there.
pub fn straight_line_of(
    job: &Job,
    id: u64,
    near: Option<(f64, f64)>,
) -> Option<([f64; 2], [f64; 2])> {
    let shape = job.drawing.shapes.iter().find(|s| s.id == id)?;
    let lines: Vec<([f64; 2], [f64; 2])> = shape
        .segments()
        .iter()
        .filter_map(|s| match s {
            chipload_job::PathSegment::Line(l) => Some((l.start, l.end)),
            chipload_job::PathSegment::Arc(_) => None,
        })
        .collect();
    match near {
        None => (shape.segments().len() == 1)
            .then(|| lines.first().copied())
            .flatten(),
        Some(at) => {
            // The side nearest the point.
            let off = |(p, q): &([f64; 2], [f64; 2])| {
                let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                let len2 = dx * dx + dy * dy;
                let t = if len2 > 0.0 {
                    (((at.0 - p[0]) * dx + (at.1 - p[1]) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                math::hypot(p[0] + t * dx - at.0, p[1] + t * dy - at.1)
            };
            lines.into_iter().min_by(|x, y| off(x).total_cmp(&off(y)))
        }
    }
}

/// An arc slot as a drawn closed polyline (docs/specs/drawing-improvements.md §3): its
/// centre line round `center` at `radius` from `start_deg` to `end_deg` (degrees,
/// counter-clockwise when the end is greater), `width` wide. None when it can't be one: no
/// width, wider than the radius allows, a sweep of nothing or a whole turn or more, or ends
/// that would meet.
pub fn arc_slot_geometry(
    center: [f64; 2],
    radius: f64,
    start_deg: f64,
    end_deg: f64,
    width: f64,
) -> Option<Geometry> {
    let sweep = (end_deg - start_deg).to_radians();
    if !(radius.is_finite() && radius > 0.0 && sweep.is_finite()) {
        return None;
    }
    let arc = chipload_core::primitives::Arc {
        center: (center[0], center[1]),
        radius,
        start_angle: start_deg.to_radians(),
        sweep,
    };
    let (points, bulges) = chipload_draw::construct::arc_slot(&arc, width)?;
    Some(Geometry::Polyline {
        closed: true,
        points: points.iter().map(|&(x, y)| [x, y]).collect(),
        bulges,
        radii: Vec::new(),
    })
}

/// The exact box round shapes `ids` (arcs included), mm: ((x0, y0), (x1, y1)).
pub fn shapes_bounds(job: &Job, ids: &[u64]) -> Option<((f64, f64), (f64, f64))> {
    let contours = load_drawing(job).ok()?.contours;
    contours
        .iter()
        .flat_map(|c| c.parts.iter())
        .filter(|p| p.segment.shape.is_some_and(|id| ids.contains(&id)))
        .map(|p| p.segment.prim.bounds())
        .reduce(|(lo, hi), (a, b)| {
            (
                (lo.0.min(a.0), lo.1.min(a.1)),
                (hi.0.max(b.0), hi.1.max(b.1)),
            )
        })
}

/// Copies of drawn shapes `ids`, moved by `by` (mm), on the same layers, with new ids
/// (docs/specs/small-items.md §4). Returns the project and the new ids, in the drawing's
/// order of the originals.
pub fn copy_shapes(
    job: &Job,
    ids: &[u64],
    by: (f64, f64),
) -> Result<(Job, Vec<u64>), PipelineError> {
    let copies = shapes_to_copy(job, ids)?;
    add_moved(job, &copies, by)
}

/// The drawn shapes a copy keeps (the GUI's clipboard): layer, geometry and construction
/// flag of each; ids and names are not kept when they are pasted.
pub fn shapes_to_copy(job: &Job, ids: &[u64]) -> Result<Vec<Shape>, PipelineError> {
    Ok(drawn_shapes(job, ids)?.into_iter().cloned().collect())
}

/// Paste copied shapes: with `at`, the middle of the box round them goes there; without,
/// they go 10 mm right and 10 mm up from where they were (§4). Returns the new ids.
pub fn paste_shapes(
    job: &Job,
    shapes: &[Shape],
    at: Option<(f64, f64)>,
) -> Result<(Job, Vec<u64>), PipelineError> {
    let by = match at {
        None => (PASTE_SHIFT_MM, PASTE_SHIFT_MM),
        Some((x, y)) => {
            // The box of the copies alone, in a project holding just them.
            let mut alone = job.clone();
            alone.drawing.shapes.clear();
            alone.drawing.next_shape_id = 1;
            let (alone, ids) = add_moved(&alone, shapes, (0.0, 0.0))?;
            let ((x0, y0), (x1, y1)) = shapes_bounds(&alone, &ids)
                .ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?;
            (x - (x0 + x1) / 2.0, y - (y0 + y1) / 2.0)
        }
    };
    add_moved(job, shapes, by)
}

/// What `clipboard_text` starts with; also its version.
const CLIPBOARD_KEY: &str = "chipload_shapes";

/// Copied shapes as text for the system clipboard, so they can be pasted (§4): JSON,
/// `{"chipload_shapes": 1, "shapes": [...]}`, each shape as in a project file (its id is
/// not used when pasting).
pub fn clipboard_text(shapes: &[Shape]) -> String {
    let list: Vec<Shape> = shapes
        .iter()
        .zip(1..)
        .map(|(shape, id)| Shape {
            id,
            name: None,
            ..shape.clone()
        })
        .collect();
    serde_json::json!({ CLIPBOARD_KEY: 1, "shapes": list }).to_string()
}

/// The shapes in clipboard text written by [`clipboard_text`]; None for any other text.
/// Their values are checked when they are pasted.
pub fn shapes_from_clipboard(text: &str) -> Option<Vec<Shape>> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    if value.get(CLIPBOARD_KEY)?.as_i64()? != 1 {
        return None;
    }
    serde_json::from_value(value.get("shapes")?.clone()).ok()
}

/// How far a paste without a place moves the copies, right and up, mm.
pub const PASTE_SHIFT_MM: f64 = 10.0;

/// Add `shapes` moved by `by`, with new ids and no names; a copy of construction geometry
/// is construction too. Imported paths are refused.
fn add_moved(
    job: &Job,
    shapes: &[Shape],
    by: (f64, f64),
) -> Result<(Job, Vec<u64>), PipelineError> {
    if shapes.is_empty() {
        return Err(refused(Message::new("pipeline-no-shapes-given")));
    }
    let mut out = job.clone();
    let mut new_ids = Vec::new();
    for shape in shapes {
        let geometry = translated(&shape.geometry, [by.0, by.1])
            .ok_or_else(|| refused(Message::new("pipeline-shapes-copy-imported")))?;
        let id = out.drawing.next_shape_id;
        out.drawing.next_shape_id += 1;
        out.drawing.shapes.push(Shape {
            construction: shape.construction,
            id,
            layer: shape.layer.clone(),
            name: None,
            geometry,
        });
        new_ids.push(id);
    }
    checked(out).map(|job| (job, new_ids))
}

/// Drawn shapes `ids` mirrored in place about the middle of the box round them. They keep
/// their ids, so the operations that cut them follow (docs/specs/small-items.md §4).
pub fn mirror_shapes(job: &Job, ids: &[u64], mirror: Mirror) -> Result<Job, PipelineError> {
    drawn_shapes(job, ids)?;
    let ((x0, y0), (x1, y1)) = shapes_bounds(job, ids)
        .ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?;
    let about = match mirror {
        Mirror::LeftRight => (x0 + x1) / 2.0,
        Mirror::UpDown => (y0 + y1) / 2.0,
    };
    let mut out = job.clone();
    for shape in out
        .drawing
        .shapes
        .iter_mut()
        .filter(|s| ids.contains(&s.id))
    {
        if let Some(geometry) = mirrored(&shape.geometry, mirror, about) {
            shape.geometry = geometry;
        }
    }
    checked(out)
}

/// The exact box round shapes `ids` (arcs included), from their own segments, mm:
/// ((x0, y0), (x1, y1)). Quick enough for every frame: no chaining.
pub fn drawn_box(job: &Job, ids: &[u64]) -> Option<((f64, f64), (f64, f64))> {
    job.drawing
        .shapes
        .iter()
        .filter(|s| ids.contains(&s.id))
        .flat_map(Shape::segments)
        .map(|s| s.to_primitive().bounds())
        .reduce(|(lo, hi), (a, b)| {
            (
                (lo.0.min(a.0), lo.1.min(a.1)),
                (hi.0.max(b.0), hi.1.max(b.1)),
            )
        })
}

/// The middle of the box round shapes `ids`, mm.
pub fn drawn_middle(job: &Job, ids: &[u64]) -> Result<(f64, f64), PipelineError> {
    let ((x0, y0), (x1, y1)) =
        drawn_box(job, ids).ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?;
    Ok(((x0 + x1) / 2.0, (y0 + y1) / 2.0))
}

/// Drawn shapes `ids` moved, turned and scaled: each point p goes to
/// `to + scale · turn(p − from)`, turned `degrees` counter-clockwise
/// (docs/specs/drawn-shapes-box.md §2). They keep their ids, so the operations that cut
/// them follow, and so do the operations' start points and corner choices on them
/// (docs/specs/dogbone-per-corner.md §4).
pub fn transform_shapes(
    job: &Job,
    ids: &[u64],
    from: (f64, f64),
    to: (f64, f64),
    scale: f64,
    degrees: f64,
) -> Result<Job, PipelineError> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(refused(Message::new("pipeline-place-scale").arg(
            "got",
            chipload_i18n::Value::Number(scale, chipload_i18n::Digits::Shortest),
        )));
    }
    if !(degrees.is_finite() && from.0.is_finite() && from.1.is_finite())
        || !(to.0.is_finite() && to.1.is_finite())
    {
        return Err(refused(Message::new("pipeline-transform-not-numbers")));
    }
    for id in ids {
        if job
            .drawing
            .shapes
            .iter()
            .any(|s| s.id == *id && !s.is_drawn())
        {
            return Err(refused(
                Message::new("pipeline-shape-imported-transform").arg("id", *id),
            ));
        }
    }
    drawn_shapes(job, ids)?;
    let mut out = job.clone();
    for shape in out
        .drawing
        .shapes
        .iter_mut()
        .filter(|s| ids.contains(&s.id))
    {
        if let Some(geometry) = transformed(
            &shape.geometry,
            [from.0, from.1],
            [to.0, to.1],
            scale,
            degrees,
        ) {
            shape.geometry = geometry;
        }
    }
    follow_points(&mut out, job, ids, |p| {
        Some(chipload_job::edit::transformed_point(
            p,
            [from.0, from.1],
            [to.0, to.1],
            scale,
            degrees,
        ))
    });
    checked(out)
}

fn selected_ids(op: &Operation) -> Option<&Vec<u64>> {
    op.select().shapes.as_ref()
}

fn drawn_mut(job: &mut Job, id: u64) -> Result<&mut Shape, PipelineError> {
    let shape = job
        .drawing
        .shapes
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| no_shape(id))?;
    if !shape.is_drawn() {
        return Err(refused(
            Message::new("pipeline-shape-imported-change").arg("id", id),
        ));
    }
    Ok(shape)
}

/// The project's own rules (values, ids) must hold after every change.
pub(crate) fn checked(job: Job) -> Result<Job, PipelineError> {
    crate::refuse_problems(&job.drawing.validate())?;
    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Job {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/corpus/jobs/v5/drawn_part.chipload"
        );
        chipload_job::load_job(std::path::Path::new(path)).unwrap()
    }

    #[test]
    fn a_shape_keeps_its_kind_and_id_when_changed() {
        let job = project();
        let circle = Geometry::Circle {
            center: [0.0, 0.0],
            radius: 1.0,
        };
        let err = set_shape(&job, 1, "drawn", circle).unwrap_err();
        assert!(err.to_string().contains("shape 1 is a rectangle"), "{err}");
        let bigger = Geometry::Circle {
            center: [75.0, 80.0],
            radius: 14.0,
        };
        let changed = set_shape(&job, 2, "holes", bigger.clone()).unwrap();
        let shape = changed.drawing.shapes.iter().find(|s| s.id == 2).unwrap();
        assert_eq!((shape.layer.as_str(), &shape.geometry), ("holes", &bigger));
        assert_eq!(changed.drawing.next_shape_id, job.drawing.next_shape_id);
    }

    fn geometry(job: &Job, id: u64) -> &Geometry {
        &job.drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .geometry
    }

    fn arc_polyline() -> Geometry {
        Geometry::Polyline {
            closed: true,
            points: vec![[10.0, 10.0], [50.0, 10.0], [50.0, 40.0]],
            bulges: vec![0.0, 0.5, 0.0],
            radii: Vec::new(),
        }
    }

    #[test]
    fn copies_get_new_ids_and_their_layers_and_move() {
        // docs/specs/small-items.md §4
        let job = project();
        let next = job.drawing.next_shape_id;
        let (copied, ids) = copy_shapes(&job, &[2, 1], (10.0, -5.0)).unwrap();
        assert_eq!(
            ids,
            vec![next, next + 1],
            "in the drawing's order: 1 then 2"
        );
        assert_eq!(copied.drawing.next_shape_id, next + 2);
        for (old, new) in [(1, next), (2, next + 1)] {
            let (a, b) = (
                job.drawing.shapes.iter().find(|s| s.id == old).unwrap(),
                copied.drawing.shapes.iter().find(|s| s.id == new).unwrap(),
            );
            assert_eq!(a.layer, b.layer);
            assert_eq!(
                Some(b.geometry.clone()),
                translated(&a.geometry, [10.0, -5.0])
            );
        }
        assert_eq!(
            geometry(&copied, 1),
            geometry(&job, 1),
            "the original stays"
        );
        assert!(copy_shapes(&job, &[], (1.0, 1.0)).is_err());
        assert!(copy_shapes(&job, &[2, 2], (1.0, 1.0)).is_err());
        assert!(copy_shapes(&job, &[99], (1.0, 1.0)).is_err());
    }

    #[test]
    fn mirroring_flips_in_place_and_twice_gives_the_original() {
        let (job, id) = add_shape(&project(), "drawn", arc_polyline()).unwrap();
        let ids = [1, 2, id];
        let before = shapes_bounds(&job, &ids).unwrap();
        for mirror in [Mirror::LeftRight, Mirror::UpDown] {
            let once = mirror_shapes(&job, &ids, mirror).unwrap();
            // The box stays where it was: mirrored about its own middle.
            let after = shapes_bounds(&once, &ids).unwrap();
            let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
            assert!(
                close(after.0.0, before.0.0) && close(after.1.1, before.1.1),
                "{after:?}"
            );
            let twice = mirror_shapes(&once, &ids, mirror).unwrap();
            for i in ids {
                let (a, b) = (geometry(&job, i), geometry(&twice, i));
                assert_eq!(format!("{a:.9?}"), format!("{b:.9?}"), "shape {i}");
            }
        }
        // The arc bends the other way; a 0 angle or bulge stays 0, not -0.
        let flipped = mirror_shapes(&job, &[id], Mirror::LeftRight).unwrap();
        let Geometry::Polyline { points, bulges, .. } = geometry(&flipped, id) else {
            panic!()
        };
        assert_eq!(bulges, &vec![0.0, -0.5, 0.0]);
        assert!(bulges[0].is_sign_positive());
        // About the middle of the box round it, arc included (it bulges beyond x = 50).
        let ((x0, _), (x1, _)) = shapes_bounds(&job, &[id]).unwrap();
        assert!(x1 > 50.0);
        assert_eq!(points[0], [(x0 + x1) - 10.0, 10.0]);
        let flipped = mirror_shapes(&job, &[1], Mirror::UpDown).unwrap();
        if let Geometry::Rectangle { angle, .. } = geometry(&job, 1)
            && let Geometry::Rectangle { angle: turned, .. } = geometry(&flipped, 1)
        {
            assert_eq!(*turned, 0.0 - angle);
        } else {
            panic!("shape 1 is a rectangle");
        }
    }

    #[test]
    fn a_paste_goes_to_the_cursor_or_beside_the_original() {
        let job = project();
        let clip = shapes_to_copy(&job, &[2, 3]).unwrap();
        let ((x0, y0), (x1, _)) = shapes_bounds(&job, &[2, 3]).unwrap();
        let (beside, ids) = paste_shapes(&job, &clip, None).unwrap();
        let ((a0, b0), _) = shapes_bounds(&beside, &ids).unwrap();
        assert!((a0 - x0 - PASTE_SHIFT_MM).abs() < 1e-9 && (b0 - y0 - PASTE_SHIFT_MM).abs() < 1e-9);
        let (there, ids) = paste_shapes(&job, &clip, Some((200.0, 100.0))).unwrap();
        let ((a0, b0), (a1, b1)) = shapes_bounds(&there, &ids).unwrap();
        assert!(((a0 + a1) / 2.0 - 200.0).abs() < 1e-9 && ((b0 + b1) / 2.0 - 100.0).abs() < 1e-9);
        assert!(((a1 - a0) - (x1 - x0)).abs() < 1e-9, "same size");
        assert_eq!(there.drawing.next_shape_id, job.drawing.next_shape_id + 2);
    }

    #[test]
    fn a_pasted_construction_shape_stays_construction() {
        let (job, _) = set_construction(&project(), &[2], true).unwrap();
        let clip = shapes_to_copy(&job, &[2, 3]).unwrap();
        let through_text = shapes_from_clipboard(&clipboard_text(&clip)).unwrap();
        for shapes in [&clip, &through_text] {
            let (pasted, ids) = paste_shapes(&job, shapes, None).unwrap();
            let flags: Vec<bool> = ids
                .iter()
                .map(|id| pasted.drawing.shapes.iter().find(|s| s.id == *id).unwrap())
                .map(|s| s.construction)
                .collect();
            assert_eq!(flags, [true, false]);
        }
    }

    #[test]
    fn copied_shapes_go_through_the_clipboard_as_text() {
        let job = project();
        let clip = shapes_to_copy(&job, &[1, 2, 3]).unwrap();
        let text = clipboard_text(&clip);
        let kept = |shapes: &[Shape]| -> Vec<(String, Geometry, bool)> {
            shapes
                .iter()
                .map(|s| (s.layer.clone(), s.geometry.clone(), s.construction))
                .collect()
        };
        assert_eq!(
            shapes_from_clipboard(&text).as_deref().map(kept),
            Some(kept(&clip))
        );
        assert_eq!(shapes_from_clipboard("hello"), None);
        assert_eq!(shapes_from_clipboard(r#"{"layer": "x"}"#), None);
        assert_eq!(
            shapes_from_clipboard(r#"{"chipload_shapes": 2, "shapes": []}"#),
            None
        );
        // Values from the clipboard are checked when pasted.
        let bad = r#"{"chipload_shapes": 1, "shapes": [{"layer": "0",
            "kind": "circle", "id": 1, "center": [0, 0], "radius": -1}]}"#;
        if let Some(shapes) = shapes_from_clipboard(bad) {
            assert!(paste_shapes(&job, &shapes, None).is_err());
        }
    }

    #[test]
    fn imported_shapes_are_neither_copied_nor_mirrored() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/corpus/jobs/v4/simple_part_full.chipload"
        );
        let job = chipload_job::load_job(std::path::Path::new(path)).unwrap();
        let id = job.drawing.shapes[0].id;
        let err = copy_shapes(&job, &[id], (1.0, 1.0))
            .unwrap_err()
            .to_string();
        assert!(err.contains("imported"), "{err}");
        assert!(mirror_shapes(&job, &[id], Mirror::UpDown).is_err());
    }

    #[test]
    fn paths_cannot_be_drawn() {
        let path = Geometry::Path {
            closed: false,
            segments: Vec::new(),
            origin: None,
        };
        assert!(add_shape(&project(), "0", path).is_err());
    }
}

/// Why `t` bulges can't be worked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TangentWhy {
    /// The first side of an open polyline has no side before it.
    First,
    /// Side `0` follows a tangent side that is worked out after it.
    AfterTangent(usize),
    /// Side `0`'s end lies straight behind its start's direction.
    Behind(usize),
    /// More bulges than sides.
    Points,
}

/// A polyline's bulges with each tangent arc (None, `t` on the command line) worked out:
/// the arc from its side's start, leaving in the direction the side before it ends, to its
/// end (docs/specs/polyline-tangent-arcs.md §2, §4). Worked out in order; a closed one's
/// first side may follow its last when that is a number.
pub fn tangent_bulges(
    points: &[[f64; 2]],
    closed: bool,
    given: &[Option<f64>],
) -> Result<Vec<f64>, TangentWhy> {
    use chipload_draw::construct::{side_end_heading, tangent_arc_bulge};
    let n = points.len();
    let sides = chipload_job::polyline_segment_count(n, closed);
    if given.iter().any(Option::is_none) && given.len() > sides {
        return Err(TangentWhy::Points);
    }
    let p = |i: usize| (points[i % n][0], points[i % n][1]);
    let mut out: Vec<Option<f64>> = given.to_vec();
    for i in 0..out.len() {
        if out[i].is_some() {
            continue;
        }
        let before = match (i, closed) {
            (0, false) => return Err(TangentWhy::First),
            (0, true) => out.len() - 1,
            _ => i - 1,
        };
        let bulge = out[before].ok_or(TangentWhy::AfterTangent(i))?;
        let heading = side_end_heading(p(before), p(before + 1), bulge);
        out[i] = Some(tangent_arc_bulge(p(i), heading, p(i + 1)).ok_or(TangentWhy::Behind(i))?);
    }
    Ok(out.into_iter().map(|b| b.unwrap_or(0.0)).collect())
}
