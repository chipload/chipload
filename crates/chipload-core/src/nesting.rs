// SPDX-License-Identifier: GPL-3.0-or-later
//! Whether one closed contour lies inside another, with Shapely's `contains` semantics
//! (as the Python reference uses): touching or sharing the outer boundary from the inside
//! still counts as inside. Identical shapes contain each other.

use crate::Point;
use crate::primitives::distance;
use crate::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};
use cavalier_contours::polyline::{PlineContainsResult, PlineSource, Polyline};

/// A valid closed contour: its vertices (no repeated end point) and the same as a polyline.
pub struct Ring {
    pub points: Vec<Point>,
    pub pline: Polyline<f64>,
}

pub fn contains(outer: &Ring, inner: &Ring) -> bool {
    match outer.pline.contains(&inner.pline) {
        PlineContainsResult::Pline2InsidePline1 => true,
        // Touching boundaries show up as intersections; decide those exactly.
        PlineContainsResult::Intersected | PlineContainsResult::InvalidInput => {
            inside_or_on(&outer.points, &inner.points)
        }
        PlineContainsResult::Pline1InsidePline2 | PlineContainsResult::Disjoint => false,
    }
}

fn edges(ring: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    (0..ring.len()).map(move |i| (ring[i], ring[(i + 1) % ring.len()]))
}

fn cross(a: Point, b: Point) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn sub(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

/// Every point of `inner` is inside `outer` or on its boundary.
///
/// Each inner edge is split wherever it meets an outer edge; between those points it is
/// entirely inside, entirely outside or entirely on the boundary, so one midpoint per
/// piece decides.
fn inside_or_on(outer: &[Point], inner: &[Point]) -> bool {
    edges(inner).all(|(p, q)| {
        let r = sub(q, p);
        let mut params = vec![0.0, 1.0];
        for (a, b) in edges(outer) {
            params.extend(split_params(p, r, a, b));
        }
        params.sort_by(f64::total_cmp);
        params.windows(2).all(|w| {
            let t = (w[0] + w[1]) / 2.0;
            let mid = (p.0 + r.0 * t, p.1 + r.1 * t);
            on_boundary(outer, mid) || inside(outer, mid)
        })
    })
}

/// Parameters t in (0, 1) where segment p + t·r meets segment a–b, including the ends of a
/// collinear overlap.
fn split_params(p: Point, r: Point, a: Point, b: Point) -> Vec<f64> {
    let s = sub(b, a);
    let len_r = r.0.hypot(r.1);
    let len_s = s.0.hypot(s.1);
    if len_r == 0.0 || len_s == 0.0 {
        return Vec::new();
    }
    let denom = cross(r, s);
    let qp = sub(a, p);
    let in_open = |t: f64| t > 0.0 && t < 1.0;
    if denom.abs() <= ANGLE_TOL_RAD * len_r * len_s {
        // Parallel: only a collinear overlap splits the edge.
        if cross(qp, r).abs() / len_r > CHAIN_TOL_MM {
            return Vec::new();
        }
        let project = |x: Point| {
            let d = sub(x, p);
            (d.0 * r.0 + d.1 * r.1) / (len_r * len_r)
        };
        return [project(a), project(b)]
            .into_iter()
            .filter(|&t| in_open(t))
            .collect();
    }
    let t = cross(qp, s) / denom;
    let u = cross(qp, r) / denom;
    if in_open(t) && (0.0..=1.0).contains(&u) {
        vec![t]
    } else {
        Vec::new()
    }
}

fn on_boundary(ring: &[Point], x: Point) -> bool {
    edges(ring).any(|(a, b)| segment_distance(x, a, b) <= CHAIN_TOL_MM)
}

fn segment_distance(x: Point, a: Point, b: Point) -> f64 {
    let ab = sub(b, a);
    let len2 = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((x.0 - a.0) * ab.0 + (x.1 - a.1) * ab.1) / len2).clamp(0.0, 1.0)
    };
    distance(x, (a.0 + ab.0 * t, a.1 + ab.1 * t))
}

/// Even-odd ray cast; only called for points clearly off the boundary.
fn inside(ring: &[Point], x: Point) -> bool {
    let mut inside = false;
    for (a, b) in edges(ring) {
        if (a.1 > x.1) != (b.1 > x.1) {
            let at = a.0 + (x.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if x.0 < at {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use cavalier_contours::polyline::PlineSourceMut;

    use super::*;

    fn ring(points: &[Point]) -> Ring {
        let mut pline = Polyline::new_closed();
        for &(x, y) in points {
            pline.add(x, y, 0.0);
        }
        Ring {
            points: points.to_vec(),
            pline,
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Ring {
        ring(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    #[test]
    fn strictly_inside() {
        assert!(contains(
            &rect(0.0, 0.0, 10.0, 10.0),
            &rect(2.0, 2.0, 8.0, 8.0)
        ));
        assert!(!contains(
            &rect(2.0, 2.0, 8.0, 8.0),
            &rect(0.0, 0.0, 10.0, 10.0)
        ));
    }

    #[test]
    fn sharing_edges_from_inside_counts_as_inside() {
        // A bar spanning the full width of the outer shape (as in messy_2.dxf).
        assert!(contains(
            &rect(0.0, 0.0, 10.0, 10.0),
            &rect(0.0, 4.0, 10.0, 6.0)
        ));
        assert!(contains(
            &rect(0.0, 0.0, 10.0, 10.0),
            &rect(0.0, 0.0, 5.0, 5.0)
        ));
    }

    #[test]
    fn crossing_or_outside_is_not_inside() {
        assert!(!contains(
            &rect(0.0, 0.0, 10.0, 10.0),
            &rect(5.0, 4.0, 15.0, 6.0)
        ));
        assert!(!contains(
            &rect(0.0, 0.0, 10.0, 10.0),
            &rect(10.0, 0.0, 20.0, 10.0)
        ));
        // Touching from outside at a corner.
        assert!(!contains(
            &rect(0.0, 0.0, 10.0, 10.0),
            &rect(10.0, 10.0, 20.0, 20.0)
        ));
    }

    #[test]
    fn identical_shapes_contain_each_other() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        assert!(contains(&a, &rect(0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn concave_outer_with_inner_in_the_notch_is_not_inside() {
        // U shape; the inner square sits in the notch, touching both arms.
        let u = ring(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (7.0, 10.0),
            (7.0, 3.0),
            (3.0, 3.0),
            (3.0, 10.0),
            (0.0, 10.0),
        ]);
        assert!(!contains(&u, &rect(3.0, 4.0, 7.0, 8.0)));
        assert!(contains(&u, &rect(0.0, 0.0, 10.0, 3.0)));
    }
}
