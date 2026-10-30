// SPDX-License-Identifier: GPL-3.0-or-later
//! Closed outlines that fold back on themselves: edges running side by side, closer than the
//! arc tolerance, without crossing. See docs/specs/folded-contours.md.

use crate::Point;
use crate::primitives::distance;
use crate::tolerances::{ARC_APPROX_TOL_MM, FOLD_MIN_PATH_MM};

/// One edge of the ring with where it starts along the boundary.
#[derive(Debug, Clone, Copy)]
struct Edge {
    a: Point,
    b: Point,
    /// Distance along the boundary from ring[0] to `a`.
    start: f64,
    length: f64,
}

impl Edge {
    fn point_at(&self, t: f64) -> Point {
        (
            self.a.0 + (self.b.0 - self.a.0) * t,
            self.a.1 + (self.b.1 - self.a.1) * t,
        )
    }

    /// Parameter (0..1) of the point of the edge nearest `p`.
    fn nearest(&self, p: Point) -> f64 {
        let (dx, dy) = (self.b.0 - self.a.0, self.b.1 - self.a.1);
        let len2 = dx * dx + dy * dy;
        if len2 == 0.0 {
            return 0.0;
        }
        (((p.0 - self.a.0) * dx + (p.1 - self.a.1) * dy) / len2).clamp(0.0, 1.0)
    }

    fn min_x(&self) -> f64 {
        self.a.0.min(self.b.0)
    }

    fn max_x(&self) -> f64 {
        self.a.0.max(self.b.0)
    }

    fn min_y(&self) -> f64 {
        self.a.1.min(self.b.1)
    }

    fn max_y(&self) -> f64 {
        self.a.1.max(self.b.1)
    }
}

/// Where a closed ring folds back on itself, if it does: a point of the boundary closer than
/// `ARC_APPROX_TOL_MM` to another part of it that is more than `FOLD_MIN_PATH_MM` away along
/// the boundary (the shorter way round). The first such point in boundary order.
///
/// `ring` is closed implicitly (no repeated end point), arcs already flattened.
pub fn fold_point(ring: &[Point]) -> Option<Point> {
    let n = ring.len();
    if n < 3 {
        return None;
    }
    let mut edges = Vec::with_capacity(n);
    let mut along = 0.0;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let length = distance(a, b);
        edges.push(Edge {
            a,
            b,
            start: along,
            length,
        });
        along += length;
    }
    let perimeter = along;
    if perimeter <= 2.0 * FOLD_MIN_PATH_MM {
        return None; // no two points can be that far apart along it
    }
    let tol = ARC_APPROX_TOL_MM;

    // Broad phase: edges whose boxes, grown by the tolerance, overlap. Sorted by left edge.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| {
        edges[i]
            .min_x()
            .total_cmp(&edges[j].min_x())
            .then(i.cmp(&j))
    });
    let mut hit: Option<(usize, f64, Point)> = None;
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            let (ei, ej) = (edges[i], edges[j]);
            if ej.min_x() > ei.max_x() + tol {
                break;
            }
            if ej.min_y() > ei.max_y() + tol || ei.min_y() > ej.max_y() + tol {
                continue;
            }
            let (first, second) = if i < j { (i, j) } else { (j, i) };
            if let Some((t, p)) = near_and_far(&edges[first], &edges[second], perimeter) {
                let earlier = hit.is_none_or(|(e, s, _)| (first, t) < (e, s));
                if earlier {
                    hit = Some((first, t, p));
                }
            }
        }
    }
    hit.map(|(_, _, p)| p)
}

/// The first point of `e` (by parameter) within the tolerance of `other` and farther than
/// FOLD_MIN_PATH_MM from it along the boundary.
fn near_and_far(e: &Edge, other: &Edge, perimeter: f64) -> Option<(f64, Point)> {
    let tol = ARC_APPROX_TOL_MM;
    // Skip pairs that are close along the boundary everywhere. Positions differ by
    // Δ = s_e − s_other, in [lo, hi] ⊂ (−P, P); the path is more than L where Δ lies in
    // (L, P − L) or (−P + L, −L).
    let lo = e.start - (other.start + other.length);
    let hi = e.start + e.length - other.start;
    let limit = FOLD_MIN_PATH_MM;
    let overlaps = |a: f64, b: f64| lo < b && hi > a;
    if !overlaps(limit, perimeter - limit) && !overlaps(limit - perimeter, -limit) {
        return None;
    }
    // Sample `e` every tolerance; the narrow part of a fold is longer than that. Only
    // where `e` runs through `other`'s box grown by the tolerance: no point outside it is
    // that near. The same samples, so the same answer, without a hundred-metre edge's ten
    // million (an SVG found by fuzzing took 17 s, 2026-10-30).
    let steps = (e.length / tol).ceil().max(1.0);
    let (t0, t1) = clip(e, other, tol)?;
    let first = (t0 * steps).floor().max(0.0) as usize;
    let last = (t1 * steps).ceil().min(steps) as usize;
    for k in first..=last {
        let t = k as f64 / steps;
        let p = e.point_at(t);
        let u = other.nearest(p);
        let q = other.point_at(u);
        if distance(p, q) >= tol {
            continue;
        }
        let path = path_between(
            e.start + t * e.length,
            other.start + u * other.length,
            perimeter,
        );
        if path > FOLD_MIN_PATH_MM {
            return Some((t, p));
        }
    }
    None
}

/// The parameters (0..1) of `e` between which it lies in `other`'s box grown by `grow`
/// (clipping, as Liang and Barsky), or None if it never does.
fn clip(e: &Edge, other: &Edge, grow: f64) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    let (dx, dy) = (e.b.0 - e.a.0, e.b.1 - e.a.1);
    let sides = [
        (-dx, e.a.0 - (other.min_x() - grow)),
        (dx, other.max_x() + grow - e.a.0),
        (-dy, e.a.1 - (other.min_y() - grow)),
        (dy, other.max_y() + grow - e.a.1),
    ];
    for (p, q) in sides {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else if p < 0.0 {
            t0 = t0.max(q / p);
        } else {
            t1 = t1.min(q / p);
        }
    }
    (t0 <= t1).then_some((t0, t1))
}

/// Distance along a closed boundary between two positions, the shorter way round.
fn path_between(s: f64, t: f64, perimeter: f64) -> f64 {
    let d = (s - t).abs() % perimeter;
    d.min(perimeter - d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_with(extra: &[Point]) -> Vec<Point> {
        // A 40 x 40 square; `extra` is inserted on the bottom edge.
        let mut ring = vec![(0.0, 0.0)];
        ring.extend_from_slice(extra);
        ring.extend([(40.0, 0.0), (40.0, 40.0), (0.0, 40.0)]);
        ring
    }

    #[test]
    fn the_outline_found_by_the_property_test_folds() {
        let ring = [
            (43.423305, 0.0),
            (30.490104, 0.005322),
            (41.937581, 0.014639),
            (54.764142, 0.028674),
            (33.94698, 0.023699),
            (57.769392, 0.050413),
            (55.74214, 0.058373),
            (31.934908, 0.039016),
            (46.263775, 13.79183),
            (-7.399768, 35.513573),
            (12.470305, -38.840453),
        ];
        let p = fold_point(&ring).unwrap();
        assert!(p.1.abs() < 0.1 && (30.0..58.0).contains(&p.0), "{p:?}");
    }

    #[test]
    fn a_needle_folds() {
        // A 10 mm spike, 0.005 mm wide, up from the bottom edge.
        let ring = square_with(&[(20.0, 0.0), (20.0, 10.0), (20.005, 0.0)]);
        assert!(fold_point(&ring).is_some());
    }

    #[test]
    fn a_v_notch_folds_only_when_narrower_than_about_a_degree() {
        let notch = |degrees: f64| {
            let half = (degrees / 2.0).to_radians().tan() * 10.0;
            square_with(&[(20.0 - half, 0.0), (20.0, 10.0), (20.0 + half, 0.0)])
        };
        assert!(fold_point(&notch(2.0)).is_none());
        assert!(fold_point(&notch(0.5)).is_some());
    }

    #[test]
    fn tiny_segments_and_jogs_do_not_fold() {
        // A 0.005 mm jog at a corner.
        assert!(fold_point(&square_with(&[(0.005, 0.0), (0.005, 0.005), (0.01, 0.005)])).is_none());
        // Many 0.002 mm segments along an edge, and one going back over them.
        let mut steps: Vec<Point> = (1..500).map(|i| (f64::from(i) * 0.002, 0.0)).collect();
        steps.extend([(0.997, 0.0), (1.0, 0.0)]);
        assert!(fold_point(&square_with(&steps)).is_none());
    }

    #[test]
    fn flattened_circles_and_thin_but_real_walls_do_not_fold() {
        let circle: Vec<Point> = (0..720)
            .map(|i| {
                let a = f64::from(i) / 720.0 * std::f64::consts::TAU;
                (5.0 * a.cos(), 5.0 * a.sin())
            })
            .collect();
        assert!(fold_point(&circle).is_none());
        // A slot 0.02 mm wide is thin, but wider than the tolerance.
        let slot = square_with(&[(20.0, 0.0), (20.0, 10.0), (20.02, 10.0), (20.02, 0.0)]);
        assert!(fold_point(&slot).is_none());
    }

    fn edge(a: Point, b: Point) -> Edge {
        Edge {
            a,
            b,
            start: 0.0,
            length: distance(a, b),
        }
    }

    proptest::proptest! {
        /// Every point of `e` outside the clipped range is at least the tolerance from
        /// `other`: the samples left out could never have been near.
        #[test]
        fn the_clip_leaves_out_only_points_far_from_the_other_edge(
            a in (-50.0f64..50.0, -50.0f64..50.0),
            b in (-50.0f64..50.0, -50.0f64..50.0),
            c in (-50.0f64..50.0, -50.0f64..50.0),
            d in (-50.0f64..50.0, -50.0f64..50.0),
        ) {
            let (e, other) = (edge(a, b), edge(c, d));
            let range = clip(&e, &other, ARC_APPROX_TOL_MM);
            for k in 0..=200 {
                let t = f64::from(k) / 200.0;
                let p = e.point_at(t);
                let near = distance(p, other.point_at(other.nearest(p))) < ARC_APPROX_TOL_MM;
                let inside = range.is_some_and(|(t0, t1)| t >= t0 - 1e-12 && t <= t1 + 1e-12);
                proptest::prop_assert!(!near || inside, "t {} near but outside {:?}", t, range);
            }
        }
    }

    /// Edges a hundred metres long that cross: answered at once, not sampled every 0.01 mm
    /// (an SVG found by fuzzing took 17 s, 2026-10-30).
    #[test]
    fn long_crossing_edges_are_checked_quickly() {
        let ring = [
            (0.0, 0.0),
            (100_000.0, 100_000.0),
            (100_000.0, 0.0),
            (0.0, 100_000.0),
        ];
        let start = std::time::Instant::now();
        let _ = fold_point(&ring);
        assert!(start.elapsed().as_secs_f64() < 5.0, "{:?}", start.elapsed());
    }
}
