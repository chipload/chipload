// SPDX-License-Identifier: GPL-3.0-or-later
//! Offsetting drawn shapes: by a distance, or to where the pointer is.

use std::f64::consts::PI;

use chipload_core::math;
use chipload_core::tolerances::CHAIN_TOL_MM;

use super::{Edited, add_all, chain_of, drawn, polyline_of, refused};
use crate::PipelineError;
pub use chipload_draw::offset::Corners;
use chipload_draw::offset::{self, OffsetError, Side};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Shape};

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
        out = add_all(out, &shape, geometries, &mut new_ids, &mut notes)?;
    }
    notes.dedup();
    Ok(Edited {
        job: out,
        new_ids,
        notes,
        joined: None,
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
pub(crate) fn offset_geometry(
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
                radii,
            },
            true,
        ) if radii.is_empty() => {
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
                radii: Vec::new(),
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
                radii,
            },
            true,
        ) if (!outside || corners == Corners::Sharp) && radii.is_empty() => {
            let step = d / math::cos(PI / f64::from(*sides));
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
                radii: Vec::new(),
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

// -- fillet and chamfer (§3)
