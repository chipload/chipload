// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing in a project: add, change and delete drawn shapes (docs/specs/drawing-tools.md).
//! The CLI and the GUI both go through these, so they follow the same rules.

use chipload_job::{Geometry, Job, Operation, Shape};

use crate::{PipelineError, ids_text, op_label};

/// Add a drawn shape. It takes `next_shape_id`, which then grows (ids are never reused).
/// Returns the project and the new id.
pub fn add_shape(job: &Job, layer: &str, geometry: Geometry) -> Result<(Job, u64), PipelineError> {
    if matches!(geometry, Geometry::Path { .. }) {
        return Err(PipelineError::Job(
            "only circles, rectangles and polylines can be drawn".into(),
        ));
    }
    let mut out = job.clone();
    let id = out.drawing.next_shape_id;
    out.drawing.next_shape_id += 1;
    out.drawing.shapes.push(Shape {
        id,
        layer: layer.to_string(),
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
        return Err(PipelineError::Job(format!(
            "shape {id} is a {}; its kind can't change",
            shape.kind_name()
        )));
    }
    shape.layer = layer.to_string();
    shape.geometry = geometry;
    checked(out)
}

/// Delete a drawn shape. Operations that selected it are flagged, never fixed: their
/// selector keeps the id and they can't run until their geometry is chosen again. Returns
/// the project and one line per flagged operation.
pub fn delete_shape(job: &Job, id: u64) -> Result<(Job, Vec<String>), PipelineError> {
    let mut out = job.clone();
    drawn_mut(&mut out, id)?;
    out.drawing.shapes.retain(|s| s.id != id);
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

fn selected_ids(op: &Operation) -> Option<&Vec<u64>> {
    match op {
        Operation::Profile(p) => p.select.shapes.as_ref(),
        Operation::Pocket(p) => p.select.shapes.as_ref(),
        Operation::Drill(d) => d.select.shapes.as_ref(),
    }
}

fn drawn_mut(job: &mut Job, id: u64) -> Result<&mut Shape, PipelineError> {
    let shape = job
        .drawing
        .shapes
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| PipelineError::Job(format!("there is no shape {id}")))?;
    if !shape.is_drawn() {
        return Err(PipelineError::Job(format!(
            "shape {id} was imported from the DXF and can't be changed here; change the \
             drawing and use Update drawing from DXF"
        )));
    }
    Ok(shape)
}

/// The project's own rules (values, ids) must hold after every change.
fn checked(job: Job) -> Result<Job, PipelineError> {
    let problems = job.drawing.validate();
    if problems.is_empty() {
        Ok(job)
    } else {
        Err(PipelineError::Job(problems.join("; ")))
    }
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

    #[test]
    fn paths_cannot_be_drawn() {
        let path = Geometry::Path {
            dxf_handle: None,
            dxf_type: None,
            closed: false,
            segments: Vec::new(),
        };
        assert!(add_shape(&project(), "0", path).is_err());
    }
}
