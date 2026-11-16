// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounding and chamfering a shape's corners.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Shape};

use crate::PipelineError;
use crate::shapes::reshape;

use super::{drawn, refused};
pub use chipload_draw::corner::Cut;
use chipload_draw::corner::{self, CornerError, Poly};

/// A drawn shape as a polyline of corners for filleting, with the kind it has: None for a
/// circle, or a rectangle whose corners are round already (no corners to cut).
fn corner_poly(geometry: &Geometry) -> Option<Poly> {
    let points = |corners: Vec<[f64; 2]>| corners.iter().map(|p| (p[0], p[1])).collect();
    match geometry {
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } if *corner_radius == 0.0 => Some(Poly {
            points: points(
                chipload_job::edit::rectangle_corners(*center, *length, *width, *angle).to_vec(),
            ),
            bulges: vec![0.0; 4],
            closed: true,
        }),
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => Some(Poly {
            points: points(chipload_job::polygon_corners(
                *center, *radius, *sides, *angle,
            )),
            bulges: vec![0.0; *sides as usize],
            closed: true,
        }),
        Geometry::Polyline {
            closed,
            points: p,
            bulges,
        } => Some(Poly {
            points: points(p.clone()),
            bulges: bulges.clone(),
            closed: *closed,
        }),
        _ => None,
    }
}

/// The corners of a drawn shape that can be filleted or chamfered: (vertex, where).
pub fn shape_corners(shape: &Shape) -> Vec<(usize, (f64, f64))> {
    corner_poly(&shape.geometry)
        .map(|poly| {
            corner::corners(&poly)
                .into_iter()
                .map(|k| (k, poly.points[k]))
                .collect()
        })
        .unwrap_or_default()
}

/// Fillet or chamfer corner `corner` of drawn shape `id` (None: every corner). The shape
/// keeps its id; a rectangle filleted all round keeps its kind (its corner radius), any
/// other becomes a polyline.
pub fn cut_shape_corners(
    job: &Job,
    id: u64,
    corner: Option<usize>,
    cut: Cut,
) -> Result<Job, PipelineError> {
    let shape = drawn(job, id)?;
    if let (
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            ..
        },
        None,
        Cut::Fillet(r),
    ) = (&shape.geometry, corner, cut)
    {
        return reshape(
            job,
            id,
            Geometry::Rectangle {
                center: *center,
                length: *length,
                width: *width,
                angle: *angle,
                corner_radius: r,
            },
        );
    }
    let poly = corner_poly(&shape.geometry)
        .ok_or_else(|| refused(Message::new("pipeline-corner-none").arg("id", id)))?;
    let done = corner::cut_corners(&poly, corner, cut).map_err(|e| {
        refused(match e {
            CornerError::NotACorner(k) => Message::new("pipeline-corner-not")
                .arg("id", id)
                .arg("corner", k),
            CornerError::Straight(k) => Message::new("pipeline-corner-straight")
                .arg("id", id)
                .arg("corner", k),
            CornerError::NoCorners => Message::new("pipeline-corner-none").arg("id", id),
            CornerError::TooLarge { most } => Message::new("pipeline-corner-too-large")
                .arg("id", id)
                .arg("most", Value::Length(most, Digits::Fixed(3))),
            CornerError::BadSize => Message::new("pipeline-corner-size"),
        })
    })?;
    reshape(
        job,
        id,
        Geometry::Polyline {
            closed: done.closed,
            points: done.points.iter().map(|&(x, y)| [x, y]).collect(),
            bulges: done.bulges,
        },
    )
}

// -- trim, extend, break (§4)

/// The cutting edges: the segments of every shape the drawing shows, drawn and imported.
pub fn shown_cutters(job: &Job) -> Vec<chipload_core::primitives::Primitive> {
    job.drawing
        .shapes
        .iter()
        .filter(|s| !job.drawing.is_hidden(s))
        .flat_map(|s| s.segments().into_iter().map(|seg| seg.to_primitive()))
        .collect()
}
