// SPDX-License-Identifier: GPL-3.0-or-later
//! Fitting lines and arcs to dense points, such as a traced picture's edges
//! (docs/specs/image-arcs-centrelines.md §2).
//!
//! Greedy, longest piece first: from the current point, the farthest point a line reaches
//! within the tolerance and the farthest an arc reaches; the longer wins (a line on a tie).
//! Every input point, and the path between them, lies within the tolerance of the piece
//! that spans it, and every piece starts and ends on an input point.

use std::f64::consts::{FRAC_PI_4, PI, TAU};

use crate::Point;
use crate::order::Ordered;
use crate::primitives::{Arc, Line, Primitive, distance};

/// Arcs flatter than this are lines: a nearly straight arc gains nothing and makes poor G2/G3.
pub const MAX_FIT_RADIUS_MM: f64 = 1000.0;
/// An arc turns at most this far.
pub const MAX_FIT_SWEEP: f64 = PI;
/// A closed path turning less than this at its sharpest point has no corner to start at.
const CORNER_TURN: f64 = FRAC_PI_4;
/// How many points either side measure the turn at a point (one pixel's step is noisy).
const TURN_REACH: usize = 3;

/// `points` as lines (and arcs, when `arcs`) within `tol`. A closed path ends where it
/// starts (the first point isn't repeated in `points`); it starts at its sharpest corner, or
/// with none, at its lowest-left point, so no piece is cut short at an arbitrary place.
/// Fewer than two points (three when closed) give no pieces.
pub fn fit_path(points: &[Point], closed: bool, tol: f64, arcs: bool) -> Vec<Primitive> {
    if points.len() < 2 || (closed && points.len() < 3) {
        return Vec::new();
    }
    let path: Vec<Point> = if closed {
        let start = start_of(points);
        let mut path: Vec<Point> = points[start..].to_vec();
        path.extend_from_slice(&points[..start]);
        path.push(points[start]);
        path
    } else {
        points.to_vec()
    };
    let mut pieces = Vec::new();
    let mut i = 0;
    while i + 1 < path.len() {
        let by_line = farthest(&path, i, 1, |j| line_fits(&path, i, j, tol));
        let by_arc = if arcs {
            farthest(&path, i, 2, |j| arc_through(&path, i, j, tol).is_some())
        } else {
            i
        };
        if by_arc > by_line
            && let Some(arc) = arc_through(&path, i, by_arc, tol)
        {
            pieces.push(Primitive::Arc(arc));
            i = by_arc;
        } else {
            pieces.push(Primitive::Line(Line {
                start: path[i],
                end: path[by_line],
            }));
            i = by_line;
        }
    }
    pieces
}

/// The farthest `j` after `i` for which `fits(j)`, searching by doubling the step from
/// `first` and then halving; `i` when even `i + first` doesn't fit (or is past the end).
fn farthest(path: &[Point], i: usize, first: usize, fits: impl Fn(usize) -> bool) -> usize {
    let last = path.len() - 1;
    if i + first > last || !fits(i + first) {
        return if first == 1 { (i + 1).min(last) } else { i };
    }
    let (mut good, mut step) = (i + first, first);
    let mut bad = None;
    while good < last {
        step *= 2;
        let next = (i + step).min(last);
        if fits(next) {
            good = next;
        } else {
            bad = Some(next);
            break;
        }
    }
    if let Some(mut bad) = bad {
        while bad - good > 1 {
            let mid = good + (bad - good) / 2;
            if fits(mid) {
                good = mid;
            } else {
                bad = mid;
            }
        }
    }
    good
}

/// Every point strictly between `i` and `j` lies within `tol` of the segment from `i` to `j`.
fn line_fits(path: &[Point], i: usize, j: usize, tol: f64) -> bool {
    (i + 1..j).all(|k| segment_distance(path[k], path[i], path[j]) <= tol)
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length2 = dx * dx + dy * dy;
    if length2 == 0.0 {
        return distance(p, a);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length2).clamp(0.0, 1.0);
    distance(p, (a.0 + t * dx, a.1 + t * dy))
}

/// The arc from point `i` to point `j` through their middle point, if every point between
/// lies within `tol` of it, in order along it, the middle of every chord between
/// neighbouring points does too (so the arc follows the path, not only its points), and it
/// keeps the limits (radius at least `tol` and at most [`MAX_FIT_RADIUS_MM`], sweep at most
/// [`MAX_FIT_SWEEP`]).
fn arc_through(path: &[Point], i: usize, j: usize, tol: f64) -> Option<Arc> {
    if j < i + 2 {
        return None;
    }
    let (a, m, b) = (path[i], path[i + (j - i) / 2], path[j]);
    let center = circumcentre(a, m, b)?;
    let radius = distance(center, a);
    if !(tol..=MAX_FIT_RADIUS_MM).contains(&radius) {
        return None;
    }
    let turn = (m.0 - a.0) * (b.1 - a.1) - (m.1 - a.1) * (b.0 - a.0);
    // The middle point right of a→b (a positive cross product of m − a and b − a): the arc
    // turns counter-clockwise.
    let direction = if turn > 0.0 { 1.0 } else { -1.0 };
    let angle = |p: Point| (p.1 - center.1).atan2(p.0 - center.0);
    let start_angle = angle(a);
    let along = |p: Point| (direction * (angle(p) - start_angle)).rem_euclid(TAU);
    let sweep = along(b);
    if sweep > MAX_FIT_SWEEP || sweep <= 0.0 {
        return None;
    }
    let mut before = 0.0;
    for &p in &path[i + 1..j] {
        if (distance(p, center) - radius).abs() > tol {
            return None;
        }
        let here = along(p);
        if here < before || here > sweep {
            return None;
        }
        before = here;
    }
    // The path between the points too: a chord is farthest from the arc at its middle.
    for k in i..j {
        let (a, b) = (path[k], path[k + 1]);
        let middle = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        if (distance(middle, center) - radius).abs() > tol {
            return None;
        }
    }
    Some(Arc {
        center,
        radius,
        start_angle,
        sweep: direction * sweep,
    })
}

/// The centre of the circle through three points; None when they are (nearly) in line.
fn circumcentre(a: Point, b: Point, c: Point) -> Option<Point> {
    let (bx, by) = (b.0 - a.0, b.1 - a.1);
    let (cx, cy) = (c.0 - a.0, c.1 - a.1);
    let d = 2.0 * (bx * cy - by * cx);
    let scale = (bx * bx + by * by).max(cx * cx + cy * cy);
    if d.abs() <= f64::EPSILON * scale {
        return None;
    }
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;
    let ux = (cy * b2 - by * c2) / d;
    let uy = (bx * c2 - cx * b2) / d;
    if !(ux.is_finite() && uy.is_finite()) {
        return None;
    }
    Some((a.0 + ux, a.1 + uy))
}

/// Where a closed path starts: its sharpest corner, measured over [`TURN_REACH`] points
/// either side, if it turns at least [`CORNER_TURN`] there; otherwise its lowest-left point.
fn start_of(points: &[Point]) -> usize {
    let n = points.len();
    let reach = TURN_REACH.min((n - 1) / 2).max(1);
    let turn = |k: usize| {
        let (p, q, r) = (
            points[(k + n - reach) % n],
            points[k],
            points[(k + reach) % n],
        );
        let a = (q.1 - p.1).atan2(q.0 - p.0);
        let b = (r.1 - q.1).atan2(r.0 - q.0);
        let d = (b - a).rem_euclid(TAU);
        d.min(TAU - d)
    };
    let sharpest = (0..n).max_by(|&x, &y| turn(x).total_cmp(&turn(y)).then(y.cmp(&x)));
    if let Some(k) = sharpest
        && turn(k) >= CORNER_TURN
    {
        return k;
    }
    (0..n)
        .min_by_key(|&x| (Ordered(points[x].1), Ordered(points[x].0)))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle(n: usize, r: f64) -> Vec<Point> {
        (0..n)
            .map(|k| {
                let t = TAU * k as f64 / n as f64;
                (r * t.cos(), r * t.sin())
            })
            .collect()
    }

    #[test]
    fn a_circle_becomes_at_most_four_arcs() {
        let pieces = fit_path(&circle(400, 20.0), true, 0.01, true);
        assert!(
            (2..=4).contains(&pieces.len()),
            "{} pieces: {pieces:?}",
            pieces.len()
        );
        assert!(pieces.iter().all(|p| matches!(p, Primitive::Arc(_))));
        let swept: f64 = pieces
            .iter()
            .map(|p| match p {
                Primitive::Arc(a) => a.sweep,
                Primitive::Line(_) => 0.0,
            })
            .sum();
        assert!((swept - TAU).abs() < 1e-9, "{swept}");
    }

    #[test]
    fn a_square_becomes_its_four_sides() {
        let mut ring = Vec::new();
        for k in 0..10 {
            ring.push((k as f64, 0.0));
        }
        for k in 0..10 {
            ring.push((10.0, k as f64));
        }
        for k in 0..10 {
            ring.push((10.0 - k as f64, 10.0));
        }
        for k in 0..10 {
            ring.push((0.0, 10.0 - k as f64));
        }
        let pieces = fit_path(&ring, true, 0.01, true);
        assert_eq!(pieces.len(), 4, "{pieces:?}");
        assert!(pieces.iter().all(|p| matches!(p, Primitive::Line(_))));
    }

    #[test]
    fn without_arcs_only_lines() {
        let pieces = fit_path(&circle(100, 5.0), true, 0.01, false);
        assert!(pieces.iter().all(|p| matches!(p, Primitive::Line(_))));
        assert!(pieces.len() > 4);
    }

    #[test]
    fn an_open_path_keeps_its_ends() {
        let points: Vec<Point> = (0..=50)
            .map(|k| {
                let t = PI * k as f64 / 50.0;
                (10.0 * t.cos(), 10.0 * t.sin())
            })
            .collect();
        let pieces = fit_path(&points, false, 0.01, true);
        assert_eq!(pieces.len(), 1, "{pieces:?}");
        assert!(distance(pieces[0].start(), points[0]) < 1e-9);
        assert!(distance(pieces[0].end(), points[50]) < 1e-9);
    }

    #[test]
    fn too_few_points_give_nothing() {
        assert!(fit_path(&[(0.0, 0.0)], false, 0.01, true).is_empty());
        assert!(fit_path(&[(0.0, 0.0), (1.0, 0.0)], true, 0.01, true).is_empty());
        assert_eq!(
            fit_path(&[(0.0, 0.0), (1.0, 0.0)], false, 0.01, true).len(),
            1
        );
    }
}
