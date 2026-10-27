// SPDX-License-Identifier: GPL-3.0-or-later
//! Shapes from the points of a drawing tool (docs/specs/drawing-input.md §4): the arc
//! through three points, the arc of a radius between two, and an arc as polyline segments.

use std::f64::consts::{PI, TAU};

use chipload_core::Point;
use chipload_core::primitives::{Arc, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;

/// The arc from `start` through `through` to `end`: the circle through the three points,
/// swept the way that passes `through`. None when two of them coincide or the three lie
/// on one line (within `CHAIN_TOL_MM` of it).
pub fn arc_through(start: Point, through: Point, end: Point) -> Option<Arc> {
    let chord = distance(start, end);
    if chord < CHAIN_TOL_MM
        || distance(start, through) < CHAIN_TOL_MM
        || distance(through, end) < CHAIN_TOL_MM
    {
        return None;
    }
    // How far `through` lies off the chord.
    let (ex, ey) = (end.0 - start.0, end.1 - start.1);
    let (tx, ty) = (through.0 - start.0, through.1 - start.1);
    let cross = ex * ty - ey * tx;
    if cross.abs() / chord < CHAIN_TOL_MM {
        return None;
    }
    // The circumcentre, relative to `start`.
    let d = 2.0 * cross;
    let (e2, t2) = (ex * ex + ey * ey, tx * tx + ty * ty);
    let cx = (ty * e2 - ey * t2) / d;
    let cy = (ex * t2 - tx * e2) / d;
    let center = (start.0 + cx, start.1 + cy);
    let angle = |p: Point| (p.1 - center.1).atan2(p.0 - center.0);
    let start_angle = angle(start);
    let to_end = (angle(end) - start_angle).rem_euclid(TAU);
    let to_through = (angle(through) - start_angle).rem_euclid(TAU);
    // Counter-clockwise when `through` comes before `end` that way.
    let sweep = if to_through < to_end {
        to_end
    } else {
        to_end - TAU
    };
    Some(Arc {
        center,
        radius: distance(center, start),
        start_angle,
        sweep,
    })
}

/// The middle point of the shorter arc of `radius` from `start` to `end`, bowing to the
/// side of `side`. None when the radius is less than half the distance between the ends,
/// or `side` lies on the line through them.
pub fn arc_middle_for_radius(start: Point, end: Point, radius: f64, side: Point) -> Option<Point> {
    let chord = distance(start, end);
    let half = chord / 2.0;
    if chord < CHAIN_TOL_MM || radius < half {
        return None;
    }
    let (ux, uy) = ((end.0 - start.0) / chord, (end.1 - start.1) / chord);
    // The left normal, turned to the side's side.
    let off = (side.0 - start.0) * -uy + (side.1 - start.1) * ux;
    if off.abs() < CHAIN_TOL_MM {
        return None;
    }
    let (nx, ny) = if off > 0.0 { (-uy, ux) } else { (uy, -ux) };
    let sagitta = radius - (radius * radius - half * half).max(0.0).sqrt();
    let middle = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
    Some((middle.0 + nx * sagitta, middle.1 + ny * sagitta))
}

/// An arc as open polyline points and bulges: one segment, or two halves when it sweeps
/// more than a half circle (a bulge is at most 1).
pub fn arc_as_polyline(arc: &Arc) -> (Vec<Point>, Vec<f64>) {
    let start = arc.point_at(arc.start_angle);
    let end = arc.point_at(arc.start_angle + arc.sweep);
    if arc.sweep.abs() <= PI {
        return (vec![start, end], vec![(arc.sweep / 4.0).tan()]);
    }
    let middle = arc.point_at(arc.start_angle + arc.sweep / 2.0);
    let half = (arc.sweep / 8.0).tan();
    (vec![start, middle, end], vec![half, half])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-9
    }

    #[test]
    fn three_points_give_the_arc_through_them() {
        // The upper half of the circle r 5 at (5, 0): counter-clockwise from (10, 0).
        let arc = arc_through((10.0, 0.0), (5.0, 5.0), (0.0, 0.0)).unwrap();
        assert!(close(arc.center, (5.0, 0.0)));
        assert!((arc.radius - 5.0).abs() < 1e-9);
        assert!((arc.sweep - PI).abs() < 1e-9);
        // The other way round: clockwise.
        let back = arc_through((0.0, 0.0), (5.0, 5.0), (10.0, 0.0)).unwrap();
        assert!((back.sweep + PI).abs() < 1e-9);
        // Through a point below: the lower half, three quarters when it goes round.
        let big = arc_through((10.0, 0.0), (5.0, -5.0), (5.0, 5.0)).unwrap();
        assert!((big.sweep + 3.0 * FRAC_PI_2).abs() < 1e-9);
        // On a line, or a repeated point: none.
        assert!(arc_through((0.0, 0.0), (5.0, 0.0), (10.0, 0.0)).is_none());
        assert!(arc_through((0.0, 0.0), (0.0, 0.0), (10.0, 0.0)).is_none());
    }

    #[test]
    fn a_radius_gives_the_shorter_arc_on_the_cursors_side() {
        let up = arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 5.0, (3.0, 1.0)).unwrap();
        assert!(close(up, (5.0, 5.0)));
        let down = arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 13.0, (3.0, -1.0)).unwrap();
        assert!(close(down, (5.0, -1.0)));
        assert!(arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 4.0, (3.0, 1.0)).is_none());
        assert!(arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 6.0, (3.0, 0.0)).is_none());
    }

    #[test]
    fn arcs_past_a_half_circle_become_two_segments() {
        let half = arc_through((10.0, 0.0), (5.0, 5.0), (0.0, 0.0)).unwrap();
        let (points, bulges) = arc_as_polyline(&half);
        assert_eq!(points.len(), 2);
        assert!((bulges[0] - 1.0).abs() < 1e-9);
        let big = arc_through((10.0, 0.0), (5.0, -5.0), (5.0, 5.0)).unwrap();
        let (points, bulges) = arc_as_polyline(&big);
        assert_eq!(points.len(), 3);
        assert!(close(points[2], (5.0, 5.0)));
        assert!(bulges.iter().all(|b| b.abs() <= 1.0 && *b < 0.0));
        // The bulges give the same arc back.
        let first = Arc::from_bulge(points[0], points[1], bulges[0]).unwrap();
        assert!(close(first.center, big.center));
    }
}
