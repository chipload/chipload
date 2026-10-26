// SPDX-License-Identifier: GPL-3.0-or-later
//! Exact distances between toolpath segments and contour primitives, and a grid index so
//! distance queries only look at nearby primitives.

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::primitives::{Arc, Line, Primitive, distance};

use crate::offset::point_distance;

fn sub(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

fn cross(a: Point, b: Point) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn dot(a: Point, b: Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

/// Closest point to `p` on the segment a–b.
pub(crate) fn closest_on_segment(p: Point, a: Point, b: Point) -> Point {
    let ab = sub(b, a);
    let len2 = dot(ab, ab);
    let t = if len2 == 0.0 {
        0.0
    } else {
        (dot(sub(p, a), ab) / len2).clamp(0.0, 1.0)
    };
    (a.0 + ab.0 * t, a.1 + ab.1 * t)
}

/// Whether `angle` lies within the arc's sweep.
fn on_arc(arc: &Arc, angle: f64) -> bool {
    let along = if arc.sweep > 0.0 {
        (angle - arc.start_angle).rem_euclid(TAU)
    } else {
        (arc.start_angle - angle).rem_euclid(TAU)
    };
    along <= arc.sweep.abs()
}

fn segments_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (r, s) = (sub(b, a), sub(d, c));
    let denom = cross(r, s);
    if denom == 0.0 {
        return false; // parallel: the endpoint distances below decide
    }
    let t = cross(sub(c, a), s) / denom;
    let u = cross(sub(c, a), r) / denom;
    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

/// Shortest distance between the segment a–b and a line or arc.
pub fn segment_distance(a: Point, b: Point, prim: &Primitive) -> f64 {
    match prim {
        Primitive::Line(Line { start, end }) => {
            if segments_cross(a, b, *start, *end) {
                return 0.0;
            }
            [
                distance(a, closest_on_segment(a, *start, *end)),
                distance(b, closest_on_segment(b, *start, *end)),
                distance(*start, closest_on_segment(*start, a, b)),
                distance(*end, closest_on_segment(*end, a, b)),
            ]
            .into_iter()
            .fold(f64::INFINITY, f64::min)
        }
        Primitive::Arc(arc) => {
            let ab = sub(b, a);
            let len2 = dot(ab, ab);
            // Where the segment's line meets the circle, if within both segment and arc.
            if len2 > 0.0 {
                let f = sub(a, arc.center);
                let (qa, qb, qc) = (len2, 2.0 * dot(f, ab), dot(f, f) - arc.radius * arc.radius);
                let disc = qb * qb - 4.0 * qa * qc;
                if disc >= 0.0 {
                    for t in [
                        (-qb - disc.sqrt()) / (2.0 * qa),
                        (-qb + disc.sqrt()) / (2.0 * qa),
                    ] {
                        if (0.0..=1.0).contains(&t) {
                            let p = (a.0 + ab.0 * t, a.1 + ab.1 * t);
                            if on_arc(arc, (p.1 - arc.center.1).atan2(p.0 - arc.center.0)) {
                                return 0.0;
                            }
                        }
                    }
                }
            }
            // Otherwise the minimum is at an end of one of them, or at the segment point
            // closest to the centre if the arc covers that direction.
            let mut best = point_distance(prim, a).min(point_distance(prim, b));
            let (s, e) = (prim.start(), prim.end());
            best = best
                .min(distance(s, closest_on_segment(s, a, b)))
                .min(distance(e, closest_on_segment(e, a, b)));
            let q = closest_on_segment(arc.center, a, b);
            if on_arc(arc, (q.1 - arc.center.1).atan2(q.0 - arc.center.0)) {
                best = best.min((distance(q, arc.center) - arc.radius).abs());
            }
            best
        }
    }
}

/// Axis-aligned bounds (min, max) of a primitive; an arc uses its whole circle.
fn bounds(prim: &Primitive) -> (Point, Point) {
    match prim {
        Primitive::Line(l) => (
            (l.start.0.min(l.end.0), l.start.1.min(l.end.1)),
            (l.start.0.max(l.end.0), l.start.1.max(l.end.1)),
        ),
        Primitive::Arc(a) => arc_bounds(a),
    }
}

/// The box of the arc itself: its ends and the circle's extremes its sweep passes. Not the
/// whole circle: a nearly straight arc of a 2 m radius would fill millions of index cells.
fn arc_bounds(arc: &Arc) -> (Point, Point) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let extremes = [0.0, 0.25 * TAU, 0.5 * TAU, 0.75 * TAU]
        .into_iter()
        .filter(|&angle| on_arc(arc, angle))
        .map(|angle| arc.point_at(angle));
    let ends = [
        arc.point_at(arc.start_angle),
        arc.point_at(arc.start_angle + arc.sweep),
    ];
    for p in ends.into_iter().chain(extremes) {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    (lo, hi)
}

/// Primitives bucketed on a square grid, for "everything near this box" queries.
pub struct PrimIndex {
    pub prims: Vec<Primitive>,
    cell: f64,
    grid: BTreeMap<(i64, i64), Vec<usize>>,
}

impl PrimIndex {
    pub fn new(prims: Vec<Primitive>, cell: f64) -> Self {
        let mut index = PrimIndex {
            prims: Vec::new(),
            cell,
            grid: BTreeMap::new(),
        };
        for prim in prims {
            let id = index.prims.len();
            let (lo, hi) = bounds(&prim);
            let ((x0, y0), (x1, y1)) = (index.key(lo), index.key(hi));
            for x in x0..=x1 {
                for y in y0..=y1 {
                    index.grid.entry((x, y)).or_default().push(id);
                }
            }
            index.prims.push(prim);
        }
        index
    }

    fn key(&self, p: Point) -> (i64, i64) {
        // Drawing coordinates divided by a cell of at least a few hundredths of a mm.
        (
            (p.0 / self.cell).floor() as i64,
            (p.1 / self.cell).floor() as i64,
        )
    }

    /// Indices of primitives that may lie within `radius` of the box lo–hi, each once.
    pub fn near(&self, lo: Point, hi: Point, radius: f64) -> Vec<usize> {
        let (x0, y0) = self.key((lo.0 - radius, lo.1 - radius));
        let (x1, y1) = self.key((hi.0 + radius, hi.1 + radius));
        let mut found: Vec<usize> = Vec::new();
        if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > self.grid.len() as i64 {
            // Bigger than the whole index: walk the occupied cells instead.
            for (&(x, y), ids) in &self.grid {
                if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
                    found.extend(ids);
                }
            }
        } else {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(ids) = self.grid.get(&(x, y)) {
                        found.extend(ids);
                    }
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// Whether any primitive lies within `limit` of `p`; stops at the first one found.
    pub fn any_within(&self, p: Point, limit: f64) -> bool {
        if limit < 0.0 {
            return false;
        }
        let (x0, y0) = self.key((p.0 - limit, p.1 - limit));
        let (x1, y1) = self.key((p.0 + limit, p.1 + limit));
        // Nearest cells first: the cell holding `p` is the most likely to have a hit.
        let home = self.key(p);
        if let Some(ids) = self.grid.get(&home)
            && ids
                .iter()
                .any(|&i| point_distance(&self.prims[i], p) <= limit)
        {
            return true;
        }
        let hit = |ids: &Vec<usize>| {
            ids.iter()
                .any(|&i| point_distance(&self.prims[i], p) <= limit)
        };
        if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > self.grid.len() as i64 {
            // Bigger than the whole index: walk the occupied cells instead.
            return self.grid.iter().any(|(&(x, y), ids)| {
                (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && hit(ids)
            });
        }
        for x in x0..=x1 {
            for y in y0..=y1 {
                if (x, y) != home && self.grid.get(&(x, y)).is_some_and(hit) {
                    return true;
                }
            }
        }
        false
    }

    /// Distance from `p` to the nearest primitive, if any lies within `limit`; None
    /// means "farther than `limit`".
    pub fn distance_within(&self, p: Point, limit: f64) -> Option<f64> {
        let d = self
            .near(p, p, limit)
            .into_iter()
            .map(|i| point_distance(&self.prims[i], p))
            .fold(f64::INFINITY, f64::min);
        (d <= limit).then_some(d)
    }

    /// Distance from the segment a–b to the nearest primitive, if any lies within `limit`.
    pub fn segment_distance_within(&self, a: Point, b: Point, limit: f64) -> Option<f64> {
        let lo = (a.0.min(b.0), a.1.min(b.1));
        let hi = (a.0.max(b.0), a.1.max(b.1));
        let d = self
            .near(lo, hi, limit)
            .into_iter()
            .map(|i| segment_distance(a, b, &self.prims[i]))
            .fold(f64::INFINITY, f64::min);
        (d <= limit).then_some(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arc(center: Point, radius: f64, start: f64, sweep: f64) -> Primitive {
        Primitive::Arc(Arc {
            center,
            radius,
            start_angle: start,
            sweep,
        })
    }

    #[test]
    fn an_arc_is_boxed_by_its_sweep_not_its_circle() {
        // A nearly straight arc from (0, 0) to (15, 0), radius about 2 m (bulge -0.0018).
        let Some(flat) = Arc::from_bulge((0.0, 0.0), (15.0, 0.0), -0.0018) else {
            panic!("an arc");
        };
        let (lo, hi) = arc_bounds(&flat);
        assert!(lo.0 > -1e-9 && hi.0 < 15.0 + 1e-9, "{lo:?} {hi:?}");
        assert!(lo.1 > -0.1 && hi.1 < 0.1, "{lo:?} {hi:?}");
        // Through an extreme: the half circle over the top reaches y = 1.
        let (lo, hi) = arc_bounds(&Arc {
            center: (0.0, 0.0),
            radius: 1.0,
            start_angle: 0.0,
            sweep: 0.5 * TAU,
        });
        assert!((lo.0 + 1.0).abs() < 1e-9 && (hi.0 - 1.0).abs() < 1e-9);
        assert!(lo.1.abs() < 1e-9 && (hi.1 - 1.0).abs() < 1e-9);
        // Clockwise through the bottom.
        let (lo, _) = arc_bounds(&Arc {
            center: (0.0, 0.0),
            radius: 1.0,
            start_angle: 0.0,
            sweep: -0.5 * TAU,
        });
        assert!((lo.1 + 1.0).abs() < 1e-9);
        // The index of the flat arc stays small.
        let index = PrimIndex::new(vec![Primitive::Arc(flat)], 0.3);
        assert!(index.grid.len() < 100, "{} cells", index.grid.len());
    }

    #[test]
    fn segment_to_line() {
        let line = Primitive::Line(Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        });
        assert!((segment_distance((2.0, 3.0), (8.0, 5.0), &line) - 3.0).abs() < 1e-12);
        assert_eq!(segment_distance((5.0, -1.0), (5.0, 1.0), &line), 0.0);
    }

    #[test]
    fn segment_to_arc() {
        // Upper half circle of radius 5.
        let upper = arc((0.0, 0.0), 5.0, 0.0, std::f64::consts::PI);
        // A horizontal segment above it: closest at the top of the arc.
        assert!((segment_distance((-1.0, 7.0), (1.0, 7.0), &upper) - 2.0).abs() < 1e-12);
        // Below the centre the arc isn't there: the nearest points are its ends.
        let d = segment_distance((-1.0, -7.0), (1.0, -7.0), &upper);
        assert!((d - (16.0f64 + 49.0).sqrt()).abs() < 1e-12);
        // Crossing the arc.
        assert_eq!(segment_distance((0.0, 0.0), (0.0, 9.0), &upper), 0.0);
        // Inside the circle, not touching: the ends are farthest from the centre, so nearest
        // to the arc.
        let d = segment_distance((-1.0, 1.0), (1.0, 1.0), &upper);
        assert!((d - (5.0 - 2.0f64.sqrt())).abs() < 1e-12);
    }

    #[test]
    fn index_finds_only_what_is_near() {
        let prims: Vec<Primitive> = (0..100)
            .map(|i| {
                let x = f64::from(i) * 10.0;
                Primitive::Line(Line {
                    start: (x, 0.0),
                    end: (x + 1.0, 0.0),
                })
            })
            .collect();
        let index = PrimIndex::new(prims, 5.0);
        assert_eq!(index.distance_within((500.5, 2.0), 3.0), Some(2.0));
        assert_eq!(index.distance_within((505.0, 50.0), 3.0), None);
        assert!(index.near((500.0, 0.0), (500.0, 0.0), 1.0).len() <= 2);
    }
}
