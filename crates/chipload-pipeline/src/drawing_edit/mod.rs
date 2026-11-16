// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing operations on drawn shapes (docs/specs/drawing-editing.md): the geometry is
//! `chipload-draw`'s; here shapes become its chains and its results become shapes, keeping
//! a shape's kind where the result is exactly that kind, with ids as the spec says.

use chipload_draw::offset::Chain;
pub use chipload_draw::offset::Corners;
use chipload_i18n::Message;
use chipload_job::{Geometry, Job, Shape};

use crate::PipelineError;
use crate::shapes::{add_shape, no_shape};
pub use chipload_draw::boolean::Combine;
pub use chipload_draw::corner::Cut;
pub use chipload_draw::trim::End;

mod arrange;
mod arrays;
mod combine;
mod corners;
mod offsets;
#[cfg(test)]
mod tests;
mod trim;

pub use arrange::{
    AlignTo, AlignWith, ArrangeUnit, align_shapes, arrange_units, arrange_units_in,
    distribute_shapes,
};
pub use arrays::{ARRAY_MOST, Polar, array_grid, array_linear, array_polar};
pub use combine::combine_shapes;
pub use corners::{cut_shape_corners, shape_corners, shown_cutters};
pub(crate) use offsets::offset_geometry;
pub use offsets::{OffsetSide, offset_at, offset_shapes};
pub use trim::{
    break_shape, extend_shape, nearer_end, nearest_drawn, nearest_point, trim_preview, trim_shape,
};

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// What an editing operation did: the project, the new shapes' ids, and notes for the user.
#[derive(Debug, Clone)]
pub struct Edited {
    pub job: Job,
    pub new_ids: Vec<u64>,
    pub notes: Vec<Message>,
    /// After a trim: the shape its pieces joined into with shapes whose ends meet theirs
    /// (the others are gone).
    pub joined: Option<u64>,
}

/// The drawn shape `id`, refused when it is imported.
fn drawn(job: &Job, id: u64) -> Result<&Shape, PipelineError> {
    let shape = job
        .drawing
        .shapes
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| no_shape(id))?;
    if !shape.is_drawn() {
        return Err(refused(
            Message::new("pipeline-edit-imported").arg("id", id),
        ));
    }
    Ok(shape)
}

/// A shape's outline as a chain of lines and arcs.
pub fn chain_of(shape: &Shape) -> Chain {
    Chain {
        prims: shape.segments().iter().map(|s| s.to_primitive()).collect(),
        closed: shape.closed(),
    }
}

/// A chain as a drawn polyline.
pub fn polyline_of(chain: &Chain) -> Geometry {
    let (points, bulges) = chain.as_polyline();
    Geometry::Polyline {
        closed: chain.closed,
        points: points.iter().map(|&(x, y)| [x, y]).collect(),
        bulges,
    }
}

/// Notes naming the operations that select `layer` by its name, which now cut the new
/// shapes there too (decision 1).
pub(crate) fn layer_cutters(job: &Job, layer: &str, count: usize) -> Option<Message> {
    let ops: Vec<String> = job
        .operations
        .iter()
        .enumerate()
        .filter(|(_, op)| {
            let select = op.select();
            select
                .layer
                .as_deref()
                .is_some_and(|l| l.to_lowercase() == layer.to_lowercase())
                && select
                    .source
                    .and_then(chipload_job::Source::shape_from)
                    .is_none()
        })
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    (!ops.is_empty()).then(|| {
        Message::new("pipeline-new-shapes-cut")
            .arg("count", count)
            .arg("layer", layer.to_string())
            .arg("operations", ops.join(", "))
            .arg("ops", ops.len())
    })
}

/// Add `geometries` on `source`'s layer, construction geometry when it is, returning their
/// ids, with the note on the operations that cut that layer (none for construction).
fn add_all(
    mut job: Job,
    source: &Shape,
    geometries: Vec<Geometry>,
    new_ids: &mut Vec<u64>,
    notes: &mut Vec<Message>,
) -> Result<Job, PipelineError> {
    let layer = source.layer.as_str();
    let count = geometries.len();
    for geometry in geometries {
        let (mut next, id) = add_shape(&job, layer, geometry)?;
        if let Some(added) = next.drawing.shapes.iter_mut().find(|s| s.id == id) {
            added.construction = source.construction;
        }
        job = next;
        new_ids.push(id);
    }
    if !source.construction
        && let Some(note) = layer_cutters(&job, layer, count)
    {
        notes.push(note);
    }
    Ok(job)
}

// -- offset (§2)
