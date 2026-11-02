// SPDX-License-Identifier: GPL-3.0-or-later
//! Groups of drawn shapes and texts (docs/specs/shape-groups.md): made, added to, taken
//! from, renamed, ungrouped, deleted, moved, turned and scaled as one. Operations made from
//! a group follow it (decision 8).

use std::collections::BTreeSet;

use chipload_i18n::Message;
use chipload_job::edit::transformed_point;
use chipload_job::{ArcSegment, Geometry, Job, LineSegment, PathSegment, ShapeGroup, Text};

use crate::PipelineError;

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// What a group is made of: drawn shapes and texts. A letter stands for its whole text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Members {
    pub shapes: Vec<u64>,
    pub texts: Vec<u64>,
}

impl Members {
    /// The members shapes `ids` and texts `texts` make: letters become their texts; an
    /// imported shape (a DXF's, a picture's) is refused (decision 2).
    pub fn of(job: &Job, ids: &[u64], texts: &[u64]) -> Result<Members, PipelineError> {
        let mut members = Members::default();
        for id in ids {
            let shape = job
                .drawing
                .shapes
                .iter()
                .find(|s| s.id == *id)
                .ok_or_else(|| crate::shapes::no_shape(*id))?;
            if let Some(text) = shape.text() {
                if !members.texts.contains(&text) {
                    members.texts.push(text);
                }
            } else if shape.is_drawn() {
                if !members.shapes.contains(id) {
                    members.shapes.push(*id);
                }
            } else {
                return Err(refused(
                    Message::new("pipeline-group-imported").arg("shape", *id),
                ));
            }
        }
        for text in texts {
            if !job.drawing.texts.iter().any(|t| t.id == *text) {
                return Err(refused(Message::new("pipeline-no-text").arg("id", *text)));
            }
            if !members.texts.contains(text) {
                members.texts.push(*text);
            }
        }
        if members.shapes.is_empty() && members.texts.is_empty() {
            return Err(refused(Message::new("pipeline-no-shapes-given")));
        }
        Ok(members)
    }

    /// The groups any of them is in, lowest id first.
    fn groups(&self, job: &Job) -> Vec<u64> {
        let found: BTreeSet<u64> = self
            .shapes
            .iter()
            .filter_map(|id| job.drawing.group_of(*id))
            .chain(
                self.texts
                    .iter()
                    .filter_map(|id| job.drawing.group_of_text(*id)),
            )
            .collect();
        found.into_iter().collect()
    }
}

/// Group `members` (decision 9): none of them in a group, a new group, named `name` or
/// "Group N"; in one group, the rest added to it; in several, those merged into the
/// first (lowest id), the operations of the others following it. Returns the project and
/// the group's id.
pub fn group(
    job: &Job,
    members: &Members,
    name: Option<&str>,
) -> Result<(Job, u64), PipelineError> {
    let mut out = job.clone();
    let found = members.groups(job);
    let id = match found.first() {
        Some(first) => *first,
        None => {
            let id = job.drawing.groups.iter().map(|g| g.id).max().unwrap_or(0) + 1;
            out.drawing.groups.push(ShapeGroup {
                id,
                name: name.map_or_else(|| format!("Group {id}"), str::to_string),
                shapes: Vec::new(),
                texts: Vec::new(),
            });
            id
        }
    };
    // The other groups' members join the first; their operations follow it.
    let others: Vec<ShapeGroup> = out
        .drawing
        .groups
        .iter()
        .filter(|g| g.id != id && found.contains(&g.id))
        .cloned()
        .collect();
    out.drawing
        .groups
        .retain(|g| g.id == id || !found.contains(&g.id));
    for op in &mut out.operations {
        if let Some(select) = op.select_mut()
            && select.group.is_some_and(|g| found.contains(&g))
        {
            select.group = Some(id);
        }
    }
    let Some(target) = out.drawing.groups.iter_mut().find(|g| g.id == id) else {
        return Err(no_group(id));
    };
    let joining = others
        .iter()
        .flat_map(|g| g.shapes.iter())
        .chain(&members.shapes);
    for shape in joining {
        if !target.shapes.contains(shape) {
            target.shapes.push(*shape);
        }
    }
    for text in others
        .iter()
        .flat_map(|g| g.texts.iter())
        .chain(&members.texts)
    {
        if !target.texts.contains(text) {
            target.texts.push(*text);
        }
    }
    if let Some(name) = name {
        target.name = name.to_string();
    }
    crate::refuse_problems(&out.drawing.validate())?;
    Ok((out, id))
}

fn no_group(id: u64) -> PipelineError {
    refused(Message::new("pipeline-no-group").arg("id", id))
}

fn find(job: &Job, id: u64) -> Result<&ShapeGroup, PipelineError> {
    job.drawing
        .groups
        .iter()
        .find(|g| g.id == id)
        .ok_or_else(|| no_group(id))
}

/// Rename group `id`.
pub fn rename_group(job: &Job, id: u64, name: &str) -> Result<Job, PipelineError> {
    find(job, id)?;
    if name.trim().is_empty() {
        return Err(refused(Message::new("pipeline-group-no-name")));
    }
    let mut out = job.clone();
    if let Some(group) = out.drawing.groups.iter_mut().find(|g| g.id == id) {
        group.name = name.trim().to_string();
    }
    Ok(out)
}

/// Add `members` to group `id`; any in another group move over.
pub fn add_to_group(job: &Job, id: u64, members: &Members) -> Result<Job, PipelineError> {
    find(job, id)?;
    let mut out = job.clone();
    for group in &mut out.drawing.groups {
        if group.id != id {
            group.shapes.retain(|s| !members.shapes.contains(s));
            group.texts.retain(|t| !members.texts.contains(t));
        }
    }
    out.drawing.tidy_groups();
    if let Some(group) = out.drawing.groups.iter_mut().find(|g| g.id == id) {
        let shapes: Vec<u64> = members
            .shapes
            .iter()
            .copied()
            .filter(|s| !group.shapes.contains(s))
            .collect();
        group.shapes.extend(shapes);
        let new: Vec<u64> = members
            .texts
            .iter()
            .copied()
            .filter(|t| !group.texts.contains(t))
            .collect();
        group.texts.extend(new);
    }
    crate::refuse_problems(&out.drawing.validate())?;
    Ok(out)
}

/// Take `members` out of whatever groups they are in; a group left empty goes. Its
/// operations then no longer cut them (decision 8).
pub fn remove_from_group(job: &Job, members: &Members) -> Result<Job, PipelineError> {
    let mut out = job.clone();
    for group in &mut out.drawing.groups {
        group.shapes.retain(|s| !members.shapes.contains(s));
        group.texts.retain(|t| !members.texts.contains(t));
    }
    out.drawing.tidy_groups();
    Ok(out)
}

/// Ungroup `id`: its shapes and texts stay where they are, free. An operation made from it
/// gets the group's shapes by id instead, so what it cuts doesn't change (decision 8). One
/// of a group with texts is refused: a selection can't name a text's letters by id.
pub fn ungroup(job: &Job, id: u64) -> Result<Job, PipelineError> {
    let group = find(job, id)?.clone();
    let mut out = job.clone();
    for (i, op) in out.operations.iter_mut().enumerate() {
        let Some(select) = op.select_mut() else {
            continue;
        };
        if select.group != Some(id) {
            continue;
        }
        if !group.texts.is_empty() {
            return Err(refused(
                Message::new("pipeline-ungroup-cut-texts")
                    .arg("op", i + 1)
                    .arg("name", group.name.clone()),
            ));
        }
        let left_out: Vec<u64> = select.exclude.take().unwrap_or_default();
        select.group = None;
        select.shapes = Some(
            group
                .shapes
                .iter()
                .copied()
                .filter(|s| !left_out.contains(s))
                .collect(),
        );
    }
    out.drawing.groups.retain(|g| g.id != id);
    Ok(out)
}

/// Delete group `id` with its shapes and texts. Operations made from it are left with
/// nothing to cut, which the project shows as a problem, as for deleted shapes.
pub fn delete_group(job: &Job, id: u64) -> Result<Job, PipelineError> {
    let group = find(job, id)?.clone();
    let mut out = job.clone();
    out.drawing.shapes.retain(|s| {
        !group.shapes.contains(&s.id) && !s.text().is_some_and(|t| group.texts.contains(&t))
    });
    out.drawing.texts.retain(|t| !group.texts.contains(&t.id));
    out.drawing.groups.retain(|g| g.id != id);
    out.drawing.tidy_groups();
    Ok(out)
}

/// The box round group `id`'s shapes and letters, mm.
pub fn group_box(job: &Job, id: u64) -> Option<((f64, f64), (f64, f64))> {
    let ids = job.drawing.group_shapes(id);
    job.drawing
        .shapes
        .iter()
        .filter(|s| ids.contains(&s.id))
        .flat_map(chipload_job::Shape::segments)
        .map(|s| s.to_primitive().bounds())
        .reduce(|(lo, hi), (a, b)| {
            (
                (lo.0.min(a.0), lo.1.min(a.1)),
                (hi.0.max(b.0), hi.1.max(b.1)),
            )
        })
}

/// Group `id` moved, turned and scaled: each point p goes to `to + scale · turn(p − from)`,
/// turned `degrees` counter-clockwise. Its drawn shapes keep their ids; its texts move,
/// turn and scale with their letters (their height and spacing scaled), so a text in a
/// font this computer lacks goes along too.
pub fn transform_group(
    job: &Job,
    id: u64,
    from: (f64, f64),
    to: (f64, f64),
    scale: f64,
    degrees: f64,
) -> Result<Job, PipelineError> {
    let group = find(job, id)?.clone();
    let drawn: Vec<u64> = group.shapes.clone();
    let mut out = if drawn.is_empty() {
        job.clone()
    } else {
        crate::shapes::transform_shapes(job, &drawn, from, to, scale, degrees)?
    };
    let (from, to) = ([from.0, from.1], [to.0, to.1]);
    for text_id in &group.texts {
        let Some(text) = out.drawing.texts.iter().find(|t| t.id == *text_id).cloned() else {
            continue;
        };
        let Some(size) = crate::texts::stored_size(&out, &text) else {
            continue;
        };
        let box_ = crate::box_handles::text_box(&text, size);
        let centre = box_.center();
        let [cx, cy] = transformed_point([centre.0, centre.1], from, to, scale, degrees);
        let new_size = (size.0 * scale, size.1 * scale);
        let rotation = (text.rotation + degrees).rem_euclid(360.0);
        let (x, y) = match text.anchor {
            chipload_job::TextAnchor::Center => (cx, cy),
            chipload_job::TextAnchor::Corner => (cx - new_size.0 / 2.0, cy - new_size.1 / 2.0),
        };
        let moved = Text {
            x,
            y,
            rotation,
            height: text.height * scale,
            spacing: text.spacing * scale,
            ..text
        };
        for shape in &mut out.drawing.shapes {
            if shape.text() != Some(*text_id) {
                continue;
            }
            if let Geometry::Path { segments, .. } = &mut shape.geometry {
                for segment in segments.iter_mut() {
                    *segment = transformed_segment(segment, from, to, scale, degrees);
                }
            }
        }
        if let Some(slot) = out.drawing.texts.iter_mut().find(|t| t.id == *text_id) {
            *slot = moved;
        }
    }
    crate::refuse_problems(&out.drawing.validate())?;
    Ok(out)
}

/// A letter's segment moved, turned and scaled as [`transform_group`] says.
pub fn transformed_segment(
    segment: &PathSegment,
    from: [f64; 2],
    to: [f64; 2],
    scale: f64,
    degrees: f64,
) -> PathSegment {
    let at = |p: [f64; 2]| transformed_point(p, from, to, scale, degrees);
    match segment {
        PathSegment::Line(l) => PathSegment::Line(LineSegment {
            start: at(l.start),
            end: at(l.end),
        }),
        PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
            center: at(a.center),
            radius: a.radius * scale,
            start_angle: a.start_angle + degrees.to_radians(),
            sweep: a.sweep,
        }),
    }
}
