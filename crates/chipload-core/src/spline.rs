// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawn splines (docs/specs/splines.md §4): the smooth curve through a list of points, as a
//! centripetal Catmull–Rom spline (Yuksel, Schaefer and Keyser, "Parameterization and
//! applications of Catmull–Rom curves", Computer-Aided Design 43(7), 2011), each piece
//! between two points one cubic Bézier; and that curve as lines and arcs within a tolerance,
//! for the toolpaths and the G-code.
//!
//! The centripetal knots (spaced by the square root of the distance between points) keep a
//! piece free of cusps and loops, and the curve close to the points' polygon. They depend
//! on distances only, so moving, turning, mirroring or evenly scaling the points does the
//! same to the curve.

use crate::Point;
use crate::fit::fit_path;
use crate::primitives::{Primitive, distance};
use crate::tolerances::CHAIN_TOL_MM;

/// A cubic Bézier: its start, two control points, its end.
pub type Bezier = [Point; 4];

/// The share of the tolerance the flattening may use; the fit gets the rest.
const FLATTEN_SHARE: f64 = 0.1;
/// How deep a piece is halved at most while flattening (2^20 parts: far below any
/// tolerance for a piece of sensible size).
const MAX_DEPTH: u32 = 20;

/// The spline through `points` as one cubic Bézier per piece: from each point to the next,
/// and from the last back to the first when `closed`. An open spline's ends continue its
/// first and last pieces straight on (the end points mirrored), so it starts and ends at
/// them; two points give a straight piece. Points closer than `CHAIN_TOL_MM` to the one
/// before are left out (validation refuses them in a project). Fewer than two points (three
/// when closed) give nothing.
pub fn beziers(points: &[Point], closed: bool) -> Vec<Bezier> {
    let mut pts: Vec<Point> = Vec::with_capacity(points.len());
    for &p in points {
        if pts
            .last()
            .is_none_or(|&last| distance(last, p) >= CHAIN_TOL_MM)
        {
            pts.push(p);
        }
    }
    if closed
        && pts.len() > 1
        && let (Some(&first), Some(&last)) = (pts.first(), pts.last())
        && distance(first, last) < CHAIN_TOL_MM
    {
        pts.pop();
    }
    let n = pts.len();
    if n < 2 || (closed && n < 3) {
        return Vec::new();
    }
    // Point i, with the ends of an open spline continued by mirroring.
    let at = |i: isize| -> Point {
        if closed {
            return pts[i.rem_euclid(n as isize) as usize];
        }
        if i < 0 {
            let (a, b) = (pts[0], pts[1]);
            (2.0 * a.0 - b.0, 2.0 * a.1 - b.1)
        } else if i as usize >= n {
            let (a, b) = (pts[n - 1], pts[n - 2]);
            (2.0 * a.0 - b.0, 2.0 * a.1 - b.1)
        } else {
            pts[i as usize]
        }
    };
    let pieces = if closed { n } else { n - 1 };
    (0..pieces as isize)
        .map(|i| piece(at(i - 1), at(i), at(i + 1), at(i + 2)))
        .collect()
}

/// The centripetal Catmull–Rom piece from `p1` to `p2`, with neighbours `p0` and `p3`, as a
/// cubic Bézier. The tangents are those of the non-uniform Catmull–Rom spline at the
/// centripetal knots, scaled to the piece's own knot interval.
fn piece(p0: Point, p1: Point, p2: Point, p3: Point) -> Bezier {
    let knot = |a: Point, b: Point| distance(a, b).sqrt().max(f64::MIN_POSITIVE);
    let (d0, d1, d2) = (knot(p0, p1), knot(p1, p2), knot(p2, p3));
    let tangent = |a: Point, b: Point, c: Point, da: f64, db: f64| -> Point {
        // (b − a)/da − (c − a)/(da + db) + (c − b)/db, times the piece's interval d1.
        let x = (b.0 - a.0) / da - (c.0 - a.0) / (da + db) + (c.0 - b.0) / db;
        let y = (b.1 - a.1) / da - (c.1 - a.1) / (da + db) + (c.1 - b.1) / db;
        (x * d1, y * d1)
    };
    let m1 = tangent(p0, p1, p2, d0, d1);
    let m2 = tangent(p1, p2, p3, d1, d2);
    [
        p1,
        (p1.0 + m1.0 / 3.0, p1.1 + m1.1 / 3.0),
        (p2.0 - m2.0 / 3.0, p2.1 - m2.1 / 3.0),
        p2,
    ]
}

/// The point of `b` at `t` (0 to 1).
pub fn point_at(b: &Bezier, t: f64) -> Point {
    let s = 1.0 - t;
    let (w0, w1, w2, w3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
    (
        w0 * b[0].0 + w1 * b[1].0 + w2 * b[2].0 + w3 * b[3].0,
        w0 * b[0].1 + w1 * b[1].1 + w2 * b[2].1 + w3 * b[3].1,
    )
}

/// How far the control points lie from the chord: the curve lies within this of it.
fn flatness(b: &Bezier) -> f64 {
    let off = |p: Point| {
        let len = distance(b[0], b[3]);
        if len < CHAIN_TOL_MM {
            return distance(b[0], p);
        }
        ((b[3].0 - b[0].0) * (p.1 - b[0].1) - (b[3].1 - b[0].1) * (p.0 - b[0].0)).abs() / len
    };
    off(b[1]).max(off(b[2]))
}

/// The two halves of `b` (de Casteljau at ½).
fn halves(b: &Bezier) -> (Bezier, Bezier) {
    let mid = |a: Point, c: Point| ((a.0 + c.0) / 2.0, (a.1 + c.1) / 2.0);
    let (ab, bc, cd) = (mid(b[0], b[1]), mid(b[1], b[2]), mid(b[2], b[3]));
    let (abc, bcd) = (mid(ab, bc), mid(bc, cd));
    let m = mid(abc, bcd);
    ([b[0], ab, abc, m], [m, bcd, cd, b[3]])
}

/// Points along `b` after its start, to its end, each chord within `tol` of the curve.
fn flatten_into(b: &Bezier, tol: f64, depth: u32, out: &mut Vec<Point>) {
    if depth >= MAX_DEPTH || flatness(b) <= tol {
        out.push(b[3]);
        return;
    }
    let (first, second) = halves(b);
    flatten_into(&first, tol, depth + 1, out);
    flatten_into(&second, tol, depth + 1, out);
}

/// The curve as points: the first point, then each piece flattened within `tol`; a closed
/// curve's start isn't repeated at its end.
pub fn flatten(beziers: &[Bezier], closed: bool, tol: f64) -> Vec<Point> {
    let Some(first) = beziers.first() else {
        return Vec::new();
    };
    let mut points = vec![first[0]];
    for b in beziers {
        flatten_into(b, tol, 0, &mut points);
    }
    if closed {
        points.pop();
    }
    points
}

/// The spline through `points` as lines and arcs, every point of the curve within `tol`
/// of them: flattened within a tenth of `tol`, then fitted within the rest. An open
/// spline's pieces start at its first point and end at its last.
pub fn segments(points: &[Point], closed: bool, tol: f64) -> Vec<Primitive> {
    let pieces = beziers(points, closed);
    let dense = flatten(&pieces, closed, tol * FLATTEN_SHARE);
    fit_path(&dense, closed, tol * (1.0 - FLATTEN_SHARE), true)
}

#[cfg(test)]
mod tests;
