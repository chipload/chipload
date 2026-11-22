// SPDX-License-Identifier: GPL-3.0-or-later
//! A drawn shape stretched: a factor along X and another along Y, about a point that stays
//! (docs/specs/scale-and-stretch.md §3).
//!
//! Straight sides stretch exactly: their points move. A stretched arc is part of an
//! ellipse, so circles and the arc sides of polylines become lines and arcs fitted to the
//! stretched curve within `ARC_APPROX_TOL_MM`. Corner radii stay what they are (decision 4).

use chipload_core::fit::fit_path;
use chipload_core::math;
use chipload_core::primitives::{Arc, Primitive};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use std::f64::consts::{FRAC_PI_2, TAU};

use super::rectangle_corners;
use crate::models::{Geometry, polygon_corners};

/// The share of the arc tolerance the fitting may use; the rest is left for the stretched
/// curve between the points it is fitted through.
const FIT_SHARE: f64 = 0.875;
/// The most points an arc is sampled at before fitting.
const MAX_SAMPLES: usize = 20_000;

/// A shape after a stretch.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretched {
    pub geometry: Geometry,
    /// It became another kind (a polyline), or its arcs were fitted again: the report says
    /// so.
    pub changed: bool,
}

/// `geometry` stretched by `fx` along X and `fy` along Y about `about`. None for an
/// imported path or factors that aren't positive numbers.
pub fn stretched(geometry: &Geometry, about: [f64; 2], fx: f64, fy: f64) -> Option<Stretched> {
    if !(fx.is_finite() && fx > 0.0 && fy.is_finite() && fy > 0.0) {
        return None;
    }
    let map = |p: [f64; 2]| {
        [
            about[0] + fx * (p[0] - about[0]),
            about[1] + fy * (p[1] - about[1]),
        ]
    };
    let kept = |geometry: Geometry| {
        Some(Stretched {
            geometry,
            changed: false,
        })
    };
    match geometry {
        Geometry::Path { .. } => None,
        Geometry::Circle { center, radius } => {
            let arc = Arc {
                center: (center[0], center[1]),
                radius: *radius,
                start_angle: 0.0,
                sweep: TAU,
            };
            let pieces = fitted_arc(&arc, true, &map);
            Some(Stretched {
                geometry: polyline_of(&pieces, true),
                changed: true,
            })
        }
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
            radii,
        } => {
            if let Some(odd) = square_turn(*angle, length.max(*width)) {
                let (sx, sy) = if odd { (fy, fx) } else { (fx, fy) };
                return kept(Geometry::Rectangle {
                    center: map(*center),
                    length: length * sx,
                    width: width * sy,
                    angle: *angle,
                    corner_radius: *corner_radius,
                    radii: radii.clone(),
                });
            }
            let radii = if radii.is_empty() && *corner_radius > 0.0 {
                vec![*corner_radius; 4]
            } else {
                radii.clone()
            };
            let points = rectangle_corners(*center, *length, *width, *angle).map(map);
            Some(Stretched {
                geometry: Geometry::Polyline {
                    closed: true,
                    points: points.to_vec(),
                    bulges: vec![0.0; 4],
                    radii,
                },
                changed: true,
            })
        }
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
            radii,
        } => {
            let points: Vec<[f64; 2]> = polygon_corners(*center, *radius, *sides, *angle)
                .into_iter()
                .map(map)
                .collect();
            let bulges = vec![0.0; points.len()];
            Some(Stretched {
                geometry: Geometry::Polyline {
                    closed: true,
                    points,
                    bulges,
                    radii: radii.clone(),
                },
                changed: true,
            })
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
            radii,
        } => Some(stretched_polyline(*closed, points, bulges, radii, &map)),
        // The spline through the stretched points (docs/specs/splines.md §5).
        Geometry::Spline { closed, points } => kept(Geometry::Spline {
            closed: *closed,
            points: points.iter().map(|p| map(*p)).collect(),
        }),
    }
}

/// Whether a rectangle turned by `angle` is square to the axes, as far as its corners can
/// tell (`size` its longer side): Some(true) when turned an odd number of quarter turns, so
/// its length lies along Y.
fn square_turn(angle: f64, size: f64) -> Option<bool> {
    let quarters = (angle / FRAC_PI_2).round();
    let off = (angle - quarters * FRAC_PI_2).abs();
    if off * size / 2.0 > CHAIN_TOL_MM {
        return None;
    }
    Some(quarters.rem_euclid(2.0) == 1.0)
}

/// A polyline stretched: its points move; each arc side becomes the lines and arcs fitted
/// to the stretched arc, with the points between them added (radius 0 there).
fn stretched_polyline(
    closed: bool,
    points: &[[f64; 2]],
    bulges: &[f64],
    radii: &[f64],
    map: &impl Fn([f64; 2]) -> [f64; 2],
) -> Stretched {
    let n = points.len();
    let sides = bulges.len();
    let mut new_points = Vec::with_capacity(n);
    let mut new_bulges = Vec::with_capacity(sides);
    let mut new_radii = Vec::new();
    let mut changed = false;
    for (i, &p) in points.iter().enumerate() {
        new_points.push(map(p));
        if !radii.is_empty() {
            new_radii.push(radii.get(i).copied().unwrap_or(0.0));
        }
        if i >= sides {
            continue;
        }
        let next = points[(i + 1) % n];
        let arc = Arc::from_bulge((p[0], p[1]), (next[0], next[1]), bulges[i]);
        let Some(arc) = arc else {
            new_bulges.push(0.0);
            continue;
        };
        changed = true;
        let pieces = fitted_arc(&arc, false, map);
        for (k, piece) in pieces.iter().enumerate() {
            if k > 0 {
                let s = piece.start();
                new_points.push([s.0, s.1]);
                if !radii.is_empty() {
                    new_radii.push(0.0);
                }
            }
            new_bulges.push(bulge_of(piece));
        }
        if pieces.is_empty() {
            new_bulges.push(0.0);
        }
    }
    Stretched {
        geometry: Geometry::Polyline {
            closed,
            points: new_points,
            bulges: new_bulges,
            radii: new_radii,
        },
        changed,
    }
}

/// A piece's bulge as a polyline side: 0 for a line.
fn bulge_of(piece: &Primitive) -> f64 {
    match piece {
        Primitive::Line(_) => 0.0,
        Primitive::Arc(arc) => arc.bulge(),
    }
}

/// The lines and arcs fitted to `arc` stretched by `map`, from its start to its end (a
/// whole circle when `closed`).
fn fitted_arc(arc: &Arc, closed: bool, map: &impl Fn([f64; 2]) -> [f64; 2]) -> Vec<Primitive> {
    let count = sample_count(arc, map);
    let last = if closed { count - 1 } else { count };
    let points: Vec<(f64, f64)> = (0..=last)
        .map(|k| {
            let angle = arc.start_angle + arc.sweep * k as f64 / count as f64;
            let p = arc.point_at(angle);
            let q = map([p.0, p.1]);
            (q[0], q[1])
        })
        .collect();
    fit_path(&points, closed, ARC_APPROX_TOL_MM * FIT_SHARE, true)
}

/// How many steps to sample a stretched arc in: dense enough that the stretched curve
/// strays from the line between two neighbouring points by an eighth of the tolerance at
/// most (the sagitta, chord² / 8ρ, at the ellipse's tightest bend ρ = r·min² / max).
fn sample_count(arc: &Arc, map: &impl Fn([f64; 2]) -> [f64; 2]) -> usize {
    let origin = map([0.0, 0.0]);
    let ux = map([1.0, 0.0]);
    let uy = map([0.0, 1.0]);
    let fx = math::hypot(ux[0] - origin[0], ux[1] - origin[1]);
    let fy = math::hypot(uy[0] - origin[0], uy[1] - origin[1]);
    let (low, high) = (fx.min(fy), fx.max(fy));
    let tightest = arc.radius * low * low / high;
    let leftover = ARC_APPROX_TOL_MM * (1.0 - FIT_SHARE);
    let step = (8.0 * leftover * tightest).sqrt() / (arc.radius * high);
    let steps = (arc.sweep.abs() / step).ceil();
    if steps.is_finite() {
        (steps as usize).clamp(8, MAX_SAMPLES)
    } else {
        MAX_SAMPLES
    }
}

/// A polyline of fitted pieces that follow one another.
fn polyline_of(pieces: &[Primitive], closed: bool) -> Geometry {
    let mut points: Vec<[f64; 2]> = pieces
        .iter()
        .map(|p| {
            let s = p.start();
            [s.0, s.1]
        })
        .collect();
    if !closed && let Some(last) = pieces.last() {
        let e = last.end();
        points.push([e.0, e.1]);
    }
    Geometry::Polyline {
        closed,
        points,
        bulges: pieces.iter().map(bulge_of).collect(),
        radii: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
