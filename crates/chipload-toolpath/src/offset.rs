// SPDX-License-Identifier: GPL-3.0-or-later
//! Tool-centre rings for profiles: offset with true arcs (cavalier_contours, ADR 0006), then
//! flatten to chords and verify.

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, TAU};
use std::panic::{AssertUnwindSafe, catch_unwind};

use cavalier_contours::core::math::Vector2;
use cavalier_contours::polyline::{PlineOrientation, PlineSource, PlineSourceMut, Polyline};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, OFFSET_CHECK_SLACK_MM};
use chipload_job::Side;

use crate::ToolpathError;
use crate::arcs::{ArcTable, Circle, arc_midpoint};

/// The contour as a closed polyline with true arcs (bulges). Arcs over half a turn are split,
/// since a bulge describes at most a full circle and loses precision near it.
pub fn contour_pline(contour: &Contour) -> Polyline<f64> {
    let mut pline = Polyline::new_closed();
    for part in &contour.parts {
        let prim = if part.reversed {
            reversed(&part.segment.prim)
        } else {
            part.segment.prim
        };
        match prim {
            Primitive::Line(line) => pline.add(line.start.0, line.start.1, 0.0),
            Primitive::Arc(arc) => {
                let pieces = if arc.sweep.abs() > std::f64::consts::PI {
                    2
                } else {
                    1
                };
                let sweep = arc.sweep / f64::from(pieces);
                for i in 0..pieces {
                    let piece = Arc {
                        start_angle: arc.start_angle + sweep * f64::from(i),
                        sweep,
                        ..arc
                    };
                    let start = piece.point_at(piece.start_angle);
                    pline.add(start.0, start.1, piece.bulge());
                }
            }
        }
    }
    pline
}

fn reversed(prim: &Primitive) -> Primitive {
    match *prim {
        Primitive::Line(l) => Primitive::Line(Line {
            start: l.end,
            end: l.start,
        }),
        Primitive::Arc(a) => Primitive::Arc(Arc {
            start_angle: a.start_angle + a.sweep,
            sweep: -a.sweep,
            ..a
        }),
    }
}

/// Segments of a closed polyline as Line/Arc primitives.
pub fn pline_segments(pline: &Polyline<f64>) -> Vec<Primitive> {
    let n = pline.vertex_count();
    (0..n)
        .map(|i| {
            let v = pline.at(i);
            let next = pline.at((i + 1) % n);
            let (a, b) = ((v.x, v.y), (next.x, next.y));
            match Arc::from_bulge(a, b, v.bulge) {
                Some(arc) => Primitive::Arc(arc),
                None => Primitive::Line(Line { start: a, end: b }),
            }
        })
        .collect()
}

/// Chords within ARC_APPROX_TOL_MM of a closed polyline, without a repeated end point.
/// Every arc is split at its lowest point, so the start point of a ring (lowest, then
/// leftmost) lies on the true curve rather than wherever a chord happens to end.
pub fn flatten(pline: &Polyline<f64>) -> Vec<Point> {
    flatten_tagged(pline, &mut ArcTable::default())
}

/// [`flatten`], also recording in `arcs` which chords lie on which arc, so they can be
/// written as G2/G3 later (docs/specs/arc-output.md). The points are exactly flatten's.
pub fn flatten_tagged(pline: &Polyline<f64>, arcs: &mut ArcTable) -> Vec<Point> {
    // Raw points, and for each the circle of the piece that ends there (None: a line).
    let mut points: Vec<Point> = Vec::new();
    let mut ending: Vec<Option<Circle>> = Vec::new();
    for prim in pline_segments(pline) {
        if points.is_empty() {
            points.push(prim.start());
            ending.push(None);
        }
        match prim {
            Primitive::Line(line) => {
                points.push(line.end);
                ending.push(None);
            }
            Primitive::Arc(arc) => {
                let circle = Circle {
                    center: arc.center,
                    radius: arc.radius,
                };
                for piece in split_at_bottom(&arc) {
                    for p in Primitive::Arc(piece).points().into_iter().skip(1) {
                        points.push(p);
                        ending.push(Some(circle));
                    }
                }
            }
        }
    }
    let ring = dedupe_ring(&points);
    // A chord of the final ring is on an arc if every raw piece it stands for is on that
    // circle. Points removed as too close (dedupe_ring), including the closing point that
    // repeats the start, are bridged this way rather than losing their arc.
    let index: BTreeMap<(u64, u64), usize> = points
        .iter()
        .enumerate()
        .rev()
        .map(|(i, p)| ((p.0.to_bits(), p.1.to_bits()), i))
        .collect();
    let raw = |p: Point| index.get(&(p.0.to_bits(), p.1.to_bits())).copied();
    let n = ring.len();
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        let (Some(i), Some(j)) = (raw(a), raw(b)) else {
            continue;
        };
        // Raw pieces from i to j, wrapping round to the start for the closing chord.
        let pieces: Vec<Option<Circle>> = if j > i {
            ending[i + 1..=j].to_vec()
        } else {
            ending[i + 1..]
                .iter()
                .chain(&ending[1..=j])
                .copied()
                .collect()
        };
        if let Some(Some(first)) = pieces.first()
            && pieces.iter().all(|c| *c == Some(*first))
        {
            arcs.insert(a, b, *first);
        }
    }
    ring
}

fn split_at_bottom(arc: &Arc) -> Vec<Arc> {
    let bottom = -FRAC_PI_2;
    let delta = if arc.sweep > 0.0 {
        (bottom - arc.start_angle).rem_euclid(TAU)
    } else {
        (arc.start_angle - bottom).rem_euclid(TAU)
    };
    if delta <= 0.0 || delta >= arc.sweep.abs() {
        return vec![*arc];
    }
    let first = delta.copysign(arc.sweep);
    vec![
        Arc {
            sweep: first,
            ..*arc
        },
        Arc {
            start_angle: arc.start_angle + first,
            sweep: arc.sweep - first,
            ..*arc
        },
    ]
}

/// Drop consecutive points closer than CHAIN_TOL_MM, including a repeated closing point.
pub fn dedupe_ring(points: &[Point]) -> Vec<Point> {
    let mut ring: Vec<Point> = Vec::with_capacity(points.len());
    for &p in points {
        if ring
            .last()
            .is_none_or(|&last| distance(last, p) >= CHAIN_TOL_MM)
        {
            ring.push(p);
        }
    }
    while ring.len() > 1 && distance(ring[ring.len() - 1], ring[0]) < CHAIN_TOL_MM {
        ring.pop();
    }
    ring
}

/// Lowest point of a ring, ties (within CHAIN_TOL_MM in y) going to the smallest x.
pub fn start_index(ring: &[Point]) -> usize {
    let lowest = ring.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let mut best = 0;
    let mut best_x = f64::INFINITY;
    for (i, p) in ring.iter().enumerate() {
        if p.1 <= lowest + CHAIN_TOL_MM && p.0 < best_x {
            best = i;
            best_x = p.0;
        }
    }
    best
}

fn start_key(ring: &[Point]) -> (f64, f64) {
    let p = ring.get(start_index(ring)).copied().unwrap_or((0.0, 0.0));
    (p.1, p.0)
}

/// Tool-centre rings for one closed contour.
///
/// `side` is relative to the given loop itself. Returns one ring for outside/on; inside can
/// return several when the offset splits at a narrow neck. Fails if the tool does not fit
/// or the offset doesn't pass verification.
pub fn offset_rings(
    contour: &Contour,
    side: Side,
    tool_radius: f64,
) -> Result<Vec<Vec<Point>>, ToolpathError> {
    offset_rings_tagged(contour, side, tool_radius, &mut ArcTable::default())
}

/// [`offset_rings`], recording the rings' arcs in `arcs`.
pub fn offset_rings_tagged(
    contour: &Contour,
    side: Side,
    tool_radius: f64,
    arcs: &mut ArcTable,
) -> Result<Vec<Vec<Point>>, ToolpathError> {
    let mut pline = contour_pline(contour);
    if side == Side::On {
        return Ok(vec![flatten_tagged(&pline, arcs)]);
    }
    if pline.orientation() == PlineOrientation::Clockwise {
        pline.invert_direction_mut();
    }
    // Positive offsets go left of the direction of travel: inward on a CCW loop.
    let signed = if side == Side::Outside {
        -tool_radius
    } else {
        tool_radius
    };
    // A panic in the offset library becomes an error, never a crash or a toolpath.
    let raw = catch_unwind(AssertUnwindSafe(|| pline.parallel_offset(signed)))
        .map_err(|_| geometry("Offset failed inside the geometry library"))?;
    let mut results: Vec<Polyline<f64>> = raw
        .into_iter()
        .filter(|p| p.is_closed() && p.area().abs() > CHAIN_TOL_MM * CHAIN_TOL_MM)
        .collect();
    if results.is_empty() {
        return Err(geometry(&format!(
            "Tool does not fit inside the contour (its centre would run {tool_radius:.3} mm \
             inside the line)"
        )));
    }
    if side == Side::Outside {
        // Growing one loop gives one region; any further loops are holes it encloses (e.g. a
        // C shape closing up), which an outside profile doesn't cut.
        results.sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
        results.truncate(1);
    }
    let segments = pline_segments(&pline);
    let mut rings: Vec<Vec<Point>> = results
        .iter()
        .map(|pline| flatten_tagged(pline, arcs))
        .collect();
    for ring in &rings {
        verify_offset(&pline, &segments, ring, side, tool_radius, arcs)?;
    }
    rings.sort_by(|a, b| {
        let (ka, kb) = (start_key(a), start_key(b));
        ka.0.total_cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
    });
    Ok(rings)
}

fn geometry(message: &str) -> ToolpathError {
    ToolpathError::Geometry(message.to_string())
}

/// Post-condition: never trust the offset blindly. Every vertex must be r from the contour,
/// every chord midpoint at least r − tolerance (no gouge), all on the requested side.
fn verify_offset(
    contour: &Polyline<f64>,
    segments: &[Primitive],
    ring: &[Point],
    side: Side,
    tool_radius: f64,
    arcs: &ArcTable,
) -> Result<(), ToolpathError> {
    let lowest = tool_radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM;
    let highest = tool_radius + OFFSET_CHECK_SLACK_MM;
    let dist = |p: Point| {
        segments
            .iter()
            .map(|s| point_distance(s, p))
            .fold(f64::INFINITY, f64::min)
    };
    let n = ring.len();
    let ok = n >= 3
        && (0..n).all(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let d = dist(a);
            let inside = contour.winding_number(Vector2::new(a.0, a.1)) != 0;
            // A chord on an arc is cut as the arc when arcs are written: check where it
            // bulges most too (docs/specs/arc-output.md section 5).
            let arc_ok = arcs.get(a, b).is_none_or(|circle| {
                let top = arc_midpoint(a, b, circle);
                let d = dist(top);
                d >= lowest && d <= highest
            });
            d <= highest
                && d >= lowest
                && dist(mid) >= lowest
                && arc_ok
                && inside == (side == Side::Inside)
        });
    if ok {
        Ok(())
    } else {
        Err(geometry(
            "Offset failed verification (path not at tool radius from the contour); the \
             contour may be degenerate",
        ))
    }
}

/// Distance from `p` to a line segment or arc.
pub fn point_distance(prim: &Primitive, p: Point) -> f64 {
    match prim {
        Primitive::Line(l) => {
            let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
            let len2 = dx * dx + dy * dy;
            let t = if len2 == 0.0 {
                0.0
            } else {
                (((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / len2).clamp(0.0, 1.0)
            };
            distance(p, (l.start.0 + dx * t, l.start.1 + dy * t))
        }
        Primitive::Arc(a) => {
            let angle = (p.1 - a.center.1).atan2(p.0 - a.center.0);
            let along = if a.sweep > 0.0 {
                (angle - a.start_angle).rem_euclid(TAU)
            } else {
                (a.start_angle - angle).rem_euclid(TAU)
            };
            if along <= a.sweep.abs() {
                (distance(p, a.center) - a.radius).abs()
            } else {
                distance(p, prim.start()).min(distance(p, prim.end()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cavalier_contours::polyline::PlineSourceMut;

    fn square(size: f64) -> Polyline<f64> {
        let mut p = Polyline::new_closed();
        for (x, y) in [(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)] {
            p.add(x, y, 0.0);
        }
        p
    }

    #[test]
    fn an_arc_bulging_into_the_wall_fails_although_its_chord_passes() {
        // docs/specs/arc-output.md section 5. An inside ring 1 mm from a 10 mm square's walls.
        let contour = square(10.0);
        let segments = pline_segments(&contour);
        let ring = [(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)];
        let no_arcs = ArcTable::default();
        assert!(verify_offset(&contour, &segments, &ring, Side::Inside, 1.0, &no_arcs).is_ok());
        // Claim the bottom chord is part of a circle that sags 0.4 mm towards the wall.
        let mut arcs = ArcTable::default();
        let centre = (5.0, 20.0);
        arcs.insert(
            (1.0, 1.0),
            (9.0, 1.0),
            Circle {
                center: centre,
                radius: distance(centre, (1.0, 1.0)),
            },
        );
        assert!(verify_offset(&contour, &segments, &ring, Side::Inside, 1.0, &arcs).is_err());
    }

    #[test]
    fn flattening_records_the_chords_of_every_arc() {
        // A 10 mm circle as two half arcs (bulge 1).
        let mut circle = Polyline::new_closed();
        circle.add(0.0, -5.0, 1.0);
        circle.add(0.0, 5.0, 1.0);
        let mut arcs = ArcTable::default();
        let points = flatten_tagged(&circle, &mut arcs);
        assert_eq!(points, flatten(&circle), "same points as before");
        let n = points.len();
        for k in 0..n {
            let c = arcs.get(points[k], points[(k + 1) % n]).unwrap();
            assert!((c.radius - 5.0).abs() < 1e-9 && distance(c.center, (0.0, 0.0)) < 1e-9);
        }
    }
}
