// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing operations on drawn shapes (docs/specs/drawing-editing.md): the geometry is
//! `chipload-draw`'s; here shapes become its chains and its results become shapes, keeping
//! a shape's kind where the result is exactly that kind, with ids as the spec says.

use std::f64::consts::PI;

use chipload_core::tolerances::CHAIN_TOL_MM;
pub use chipload_draw::offset::Corners;
use chipload_draw::offset::{self, Chain, OffsetError, Side};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Shape};

use crate::PipelineError;
use crate::shapes::{add_shape, no_shape};

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// What an editing operation did: the project, the new shapes' ids, and notes for the user.
#[derive(Debug, Clone)]
pub struct Edited {
    pub job: Job,
    pub new_ids: Vec<u64>,
    pub notes: Vec<Message>,
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
fn layer_cutters(job: &Job, layer: &str, count: usize) -> Option<Message> {
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
                && select.dxf.is_none()
                && select.image.is_none()
                && select.text.is_none()
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

/// Add `geometries` on `layer`, returning their ids, with the note on the operations that
/// cut that layer.
fn add_all(
    mut job: Job,
    layer: &str,
    geometries: Vec<Geometry>,
    new_ids: &mut Vec<u64>,
    notes: &mut Vec<Message>,
) -> Result<Job, PipelineError> {
    let count = geometries.len();
    for geometry in geometries {
        let (next, id) = add_shape(&job, layer, geometry)?;
        job = next;
        new_ids.push(id);
    }
    if let Some(note) = layer_cutters(&job, layer, count) {
        notes.push(note);
    }
    Ok(job)
}

// -- offset (§2)

/// Which side an offset goes: for a closed shape outside or inside, for an open one left
/// or right. None: outside, or left for an open shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetSide {
    Outside,
    Inside,
    Left,
    Right,
}

/// Offset drawn shapes `ids` by `distance` (mm), `count` times (each `distance` further),
/// adding the results on each shape's layer; the shapes stay. Kinds are kept where the
/// offset is exactly that kind: a circle, a rectangle (or slot), a polygon inward or with
/// sharp corners.
pub fn offset_shapes(
    job: &Job,
    ids: &[u64],
    distance: f64,
    side: Option<OffsetSide>,
    corners: Corners,
    count: u32,
) -> Result<Edited, PipelineError> {
    if !(distance.is_finite() && distance > 0.0) {
        return Err(refused(
            Message::new("pipeline-offset-distance")
                .arg("got", Value::Length(distance, Digits::Shortest)),
        ));
    }
    let mut out = job.clone();
    let mut new_ids = Vec::new();
    let mut notes = Vec::new();
    for &id in ids {
        let shape = drawn(job, id)?.clone();
        let side = match (side, shape.closed()) {
            (Some(side), _) => side,
            (None, true) => OffsetSide::Outside,
            (None, false) => OffsetSide::Left,
        };
        let mut geometries = Vec::new();
        for k in 1..=count.max(1) {
            let d = distance * f64::from(k);
            geometries.extend(offset_geometry(&shape, d, side, corners)?);
        }
        out = add_all(out, &shape.layer, geometries, &mut new_ids, &mut notes)?;
    }
    notes.dedup();
    Ok(Edited {
        job: out,
        new_ids,
        notes,
    })
}

/// The offset the cursor at `at` asks for, from the nearest of drawn shapes `ids`: its
/// distance to it and the side it is on. None when none of them is drawn.
pub fn offset_at(job: &Job, ids: &[u64], at: (f64, f64)) -> Option<(f64, OffsetSide)> {
    let nearest = ids
        .iter()
        .filter_map(|&id| drawn(job, id).ok())
        .map(|shape| {
            let chain = chain_of(shape);
            (offset::nearest_distance(&chain, at), chain)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))?;
    let side = match offset::side_at(&nearest.1, at) {
        Side::Outside => OffsetSide::Outside,
        Side::Inside => OffsetSide::Inside,
        Side::Left => OffsetSide::Left,
        Side::Right => OffsetSide::Right,
    };
    Some((nearest.0, side))
}

/// One shape offset by `d`: its own kind where that is exact, else polylines.
fn offset_geometry(
    shape: &Shape,
    d: f64,
    side: OffsetSide,
    corners: Corners,
) -> Result<Vec<Geometry>, PipelineError> {
    let id = shape.id;
    let vanishes = || {
        refused(
            Message::new("pipeline-offset-vanishes")
                .arg("id", id)
                .arg("distance", Value::Length(d, Digits::Shortest)),
        )
    };
    let outside = side == OffsetSide::Outside;
    let closed_side = matches!(side, OffsetSide::Outside | OffsetSide::Inside);
    match (&shape.geometry, closed_side) {
        (Geometry::Circle { center, radius }, true) => {
            let radius = if outside { radius + d } else { radius - d };
            if radius < CHAIN_TOL_MM {
                return Err(vanishes());
            }
            Ok(vec![Geometry::Circle {
                center: *center,
                radius,
            }])
        }
        (
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
                corner_radius,
            },
            true,
        ) => {
            let (length, width, corner_radius) = if outside {
                let r = if *corner_radius > 0.0 || corners == Corners::Round {
                    corner_radius + d
                } else {
                    0.0
                };
                (length + 2.0 * d, width + 2.0 * d, r)
            } else {
                let r = corner_radius - d;
                (
                    length - 2.0 * d,
                    width - 2.0 * d,
                    if r < CHAIN_TOL_MM { 0.0 } else { r },
                )
            };
            if length < CHAIN_TOL_MM || width < CHAIN_TOL_MM {
                return Err(vanishes());
            }
            Ok(vec![Geometry::Rectangle {
                center: *center,
                length,
                width,
                angle: *angle,
                corner_radius,
            }])
        }
        // Inward, and outward with sharp corners, a regular polygon's sides move by d: its
        // corners by d / cos(π / n).
        (
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            },
            true,
        ) if !outside || corners == Corners::Sharp => {
            let step = d / (PI / f64::from(*sides)).cos();
            let radius = if outside {
                radius + step
            } else {
                radius - step
            };
            if radius < CHAIN_TOL_MM {
                return Err(vanishes());
            }
            Ok(vec![Geometry::Polygon {
                center: *center,
                radius,
                sides: *sides,
                angle: *angle,
            }])
        }
        _ => {
            let side = match side {
                OffsetSide::Outside => Side::Outside,
                OffsetSide::Inside => Side::Inside,
                OffsetSide::Left => Side::Left,
                OffsetSide::Right => Side::Right,
            };
            let results =
                offset::offset(&chain_of(shape), d, side, corners).map_err(|e| match e {
                    OffsetError::Vanishes => vanishes(),
                    OffsetError::WrongSide => refused(
                        Message::new("pipeline-offset-side")
                            .arg("id", id)
                            .arg("closed", if shape.closed() { "closed" } else { "open" }),
                    ),
                    OffsetError::BadDistance => refused(
                        Message::new("pipeline-offset-distance")
                            .arg("got", Value::Length(d, Digits::Shortest)),
                    ),
                    OffsetError::Library => refused(
                        Message::new("pipeline-offset-failed")
                            .arg("id", id)
                            .arg("why", "the offset library failed"),
                    ),
                    OffsetError::Unverified(why) => refused(
                        Message::new("pipeline-offset-failed")
                            .arg("id", id)
                            .arg("why", why),
                    ),
                })?;
            Ok(results.iter().map(polyline_of).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::load_job;
    use std::path::Path;

    fn drawn_part() -> Job {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v5/drawn_part.chipload");
        load_job(&path).unwrap()
    }

    fn geometry(job: &Job, id: u64) -> &Geometry {
        &job.drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .geometry
    }

    #[test]
    fn offsets_keep_the_kind_where_exact_and_go_on_the_layer() {
        // S1 rectangle 120 x 80, S2 circle r 12, S3 a triangle, all on layer "drawn".
        let job = drawn_part();
        let edited = offset_shapes(&job, &[1, 2, 3], 2.0, None, Corners::Round, 1).unwrap();
        assert_eq!(edited.new_ids, [4, 5, 6]);
        assert!(matches!(
            geometry(&edited.job, 4),
            Geometry::Rectangle { length, width, corner_radius, .. }
                if *length == 124.0 && *width == 84.0 && *corner_radius == 2.0
        ));
        assert!(
            matches!(geometry(&edited.job, 5), Geometry::Circle { radius, .. } if *radius == 14.0)
        );
        assert!(matches!(
            geometry(&edited.job, 6),
            Geometry::Polyline { closed: true, .. }
        ));
        // The originals stay.
        assert_eq!(geometry(&edited.job, 1), geometry(&job, 1));
        // Sharp: the rectangle keeps sharp corners; inside, twice.
        let sharp = offset_shapes(&job, &[1], 2.0, None, Corners::Sharp, 1).unwrap();
        assert!(matches!(
            geometry(&sharp.job, 4),
            Geometry::Rectangle { corner_radius, .. } if *corner_radius == 0.0
        ));
        let rings =
            offset_shapes(&job, &[2], 5.0, Some(OffsetSide::Inside), Corners::Round, 2).unwrap();
        assert!(
            matches!(geometry(&rings.job, 5), Geometry::Circle { radius, .. } if *radius == 2.0)
        );
        // A third ring would vanish: refused, and nothing is added.
        assert!(
            offset_shapes(&job, &[2], 5.0, Some(OffsetSide::Inside), Corners::Round, 3).is_err()
        );
    }

    #[test]
    fn operations_that_select_the_layer_are_named() {
        let mut job = drawn_part();
        if let Some(select) = job.operations[0].select_mut() {
            select.layer = Some("Drawn".into());
            select.shapes = None;
        }
        let edited = offset_shapes(&job, &[2], 1.0, None, Corners::Round, 1).unwrap();
        assert_eq!(edited.notes.len(), 1);
        let text = edited.notes[0].render(chipload_i18n::Language::En, Default::default());
        assert!(text.contains("operation 1"), "{text}");
    }
}
