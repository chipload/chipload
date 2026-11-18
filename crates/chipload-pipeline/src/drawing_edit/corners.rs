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
            ..
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
    // A polyline's corner is rounded by its radius, so it stays editable
    // (docs/specs/polyline-radii.md §4, decision 3).
    if let (
        Geometry::Polyline {
            closed,
            points,
            bulges,
            radii,
        },
        Cut::Fillet(r),
    ) = (&shape.geometry, cut)
    {
        return fillet_polyline(job, id, (*closed, points, bulges, radii), corner, r);
    }
    let poly = corner_poly(&shape.geometry)
        .ok_or_else(|| refused(Message::new("pipeline-corner-none").arg("id", id)))?;
    // A chamfered polyline with rounded corners keeps them: Shift cuts only its sharp
    // corners, one at a time from the last so the others keep their numbers.
    if let (Geometry::Polyline { radii, .. }, Cut::Chamfer(_)) = (&shape.geometry, cut)
        && radii.iter().any(|&r| r != 0.0)
    {
        let chosen: Vec<usize> = match corner {
            Some(k) => vec![k],
            None => corner::corners(&poly)
                .into_iter()
                .filter(|&k| radii.get(k).copied().unwrap_or(0.0) == 0.0)
                .collect(),
        };
        if chosen.is_empty() {
            return Err(refused(Message::new("pipeline-corner-none").arg("id", id)));
        }
        let mut poly = poly;
        let mut rounds = radii.clone();
        for &k in chosen.iter().rev() {
            poly = corner::cut_corners(&poly, Some(k), cut).map_err(|e| corner_refusal(id, e))?;
            rounds.splice(k..=k, [0.0, 0.0]);
        }
        return reshape(
            job,
            id,
            Geometry::Polyline {
                closed: poly.closed,
                points: poly.points.iter().map(|&(x, y)| [x, y]).collect(),
                bulges: poly.bulges,
                radii: rounds,
            },
        );
    }
    let done = corner::cut_corners(&poly, corner, cut).map_err(|e| corner_refusal(id, e))?;
    reshape(
        job,
        id,
        Geometry::Polyline {
            closed: done.closed,
            points: done.points.iter().map(|&(x, y)| [x, y]).collect(),
            bulges: done.bulges,
            radii: Vec::new(),
        },
    )
}

/// Why corners of shape `id` couldn't be cut.
fn corner_refusal(id: u64, e: CornerError) -> PipelineError {
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
}

/// A polyline's corner `corner` (None: every corner) given radius `r`, refused with the
/// most that fits when its sides haven't the room.
fn fillet_polyline(
    job: &Job,
    id: u64,
    (closed, points, bulges, radii): (bool, &[[f64; 2]], &[f64], &[f64]),
    corner: Option<usize>,
    r: f64,
) -> Result<Job, PipelineError> {
    if !(r.is_finite() && r > 0.0) {
        return Err(refused(Message::new("pipeline-corner-size")));
    }
    let pts: Vec<(f64, f64)> = points.iter().map(|p| (p[0], p[1])).collect();
    let path = chipload_core::corners::Path {
        points: &pts,
        bulges,
        closed,
    };
    let corners: Vec<usize> = (0..pts.len()).filter(|&k| path.turn(k).is_some()).collect();
    let chosen = match corner {
        Some(k) if corners.contains(&k) => vec![k],
        Some(k) => {
            return Err(refused(
                Message::new("pipeline-corner-not")
                    .arg("id", id)
                    .arg("corner", k),
            ));
        }
        None if corners.is_empty() => {
            return Err(refused(Message::new("pipeline-corner-none").arg("id", id)));
        }
        None => corners,
    };
    let mut new_radii = if radii.len() == pts.len() {
        radii.to_vec()
    } else {
        vec![0.0; pts.len()]
    };
    for &k in &chosen {
        new_radii[k] = r;
    }
    if let Some(most) = chipload_core::corners::check(path, &new_radii)
        .into_iter()
        .find_map(|problem| match problem {
            chipload_core::corners::RadiusError::TooLarge { most, .. } => Some(most),
            _ => None,
        })
    {
        return Err(refused(
            Message::new("pipeline-corner-too-large")
                .arg("id", id)
                .arg("most", Value::Length(most, Digits::Fixed(3))),
        ));
    }
    reshape(
        job,
        id,
        Geometry::Polyline {
            closed,
            points: points.to_vec(),
            bulges: bulges.to_vec(),
            radii: new_radii,
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
