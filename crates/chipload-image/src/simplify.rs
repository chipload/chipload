// SPDX-License-Identifier: GPL-3.0-or-later
//! Simplifying the traced loops (spec §3.4): Douglas–Peucker within the smoothing, then a
//! check that no simplified loop crosses itself or another. A loop that does, or that
//! collapses, is kept as traced; traced loops never cross, so that always ends.
//!
//! With arcs (docs/specs/image-arcs-centrelines.md §2), each loop is first fitted with lines
//! and arcs; a fitted loop that crosses, or comes nearer another loop than twice the
//! chaining tolerance, falls back to its Douglas–Peucker version, and then to the traced one.

use std::collections::BTreeMap;

use chipload_core::Point;
use chipload_core::fit::fit_path;
use chipload_core::order::Ordered;
use chipload_core::primitives::{Line, Primitive, max_segment_angle, signed_area};
use chipload_core::tolerances::CHAIN_TOL_MM;

/// The traced loops simplified, as points and as pieces.
pub(crate) struct Fitted {
    pub loops: Vec<Vec<Point>>,
    pub pieces: Vec<Vec<Primitive>>,
    pub without_arcs: usize,
    pub unsimplified: usize,
}

/// The loops simplified within `tolerance` mm: by Douglas–Peucker, or with `arcs` by lines
/// and arcs where that keeps them apart.
pub(crate) fn fitted(raw: Vec<Vec<Point>>, tolerance: f64, pixel: f64, arcs: bool) -> Fitted {
    let (lines, unsimplified) = simplified(raw.clone(), tolerance, pixel);
    if !arcs {
        let pieces = lines.iter().map(|ring| as_lines(ring)).collect();
        return Fitted {
            loops: lines,
            pieces,
            without_arcs: 0,
            unsimplified,
        };
    }
    // Each loop's stage: 0 its arcs, 1 its lines, 2 as traced.
    let mut stage = vec![0_u8; raw.len()];
    let mut pieces: Vec<Vec<Primitive>> = Vec::with_capacity(raw.len());
    let mut loops: Vec<Vec<Point>> = Vec::with_capacity(raw.len());
    for (k, ring) in raw.iter().enumerate() {
        let fit = fit_path(ring, true, tolerance, true);
        let flat = flattened(&fit);
        let area = signed_area(ring);
        let kept = signed_area(&flat);
        if flat.len() >= 3 && kept != 0.0 && kept.signum() == area.signum() {
            pieces.push(fit);
            loops.push(flat);
        } else {
            stage[k] = 1;
            pieces.push(as_lines(&lines[k]));
            loops.push(lines[k].clone());
        }
    }
    let cell = (4.0 * tolerance).max(2.0 * pixel);
    loop {
        let near: Vec<bool> = stage.iter().map(|&s| s == 0).collect();
        let crossing = crossing_loops(&loops, cell, &near);
        let mut changed = false;
        for k in crossing {
            let ring = match stage[k] {
                0 => &lines[k],
                1 => &raw[k],
                _ => continue,
            };
            stage[k] += 1;
            loops[k] = ring.clone();
            pieces[k] = as_lines(ring);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    let lines_kept = stage.iter().filter(|&&s| s > 0).count();
    let traced = stage.iter().filter(|&&s| s == 2).count();
    Fitted {
        loops,
        pieces,
        without_arcs: lines_kept,
        unsimplified: traced,
    }
}

/// A closed ring as lines, the last back to the first point.
fn as_lines(ring: &[Point]) -> Vec<Primitive> {
    (0..ring.len())
        .map(|k| {
            Primitive::Line(Line {
                start: ring[k],
                end: ring[(k + 1) % ring.len()],
            })
        })
        .collect()
}

/// A closed loop of pieces as points, arcs split into chords within the chaining tolerance;
/// the first point isn't repeated.
fn flattened(pieces: &[Primitive]) -> Vec<Point> {
    let mut points = Vec::new();
    for piece in pieces {
        match piece {
            Primitive::Line(l) => points.push(l.start),
            Primitive::Arc(a) => {
                let step = max_segment_angle(a.radius, CHAIN_TOL_MM);
                let n = (a.sweep.abs() / step).ceil().max(1.0) as usize;
                for i in 0..n {
                    points.push(a.point_at(a.start_angle + a.sweep * i as f64 / n as f64));
                }
            }
        }
    }
    points
}

/// The loops simplified within `tolerance` mm, and how many had to stay as traced.
pub(crate) fn simplified(
    raw: Vec<Vec<Point>>,
    tolerance: f64,
    pixel: f64,
) -> (Vec<Vec<Point>>, usize) {
    let mut loops: Vec<Vec<Point>> = raw
        .iter()
        .map(|ring| {
            let simple = simplify_ring(ring, tolerance);
            let area = signed_area(ring);
            let kept = signed_area(&simple);
            if simple.len() >= 3 && kept != 0.0 && kept.signum() == area.signum() {
                simple
            } else {
                ring.clone()
            }
        })
        .collect();
    let mut as_traced = vec![false; loops.len()];
    let cell = (4.0 * tolerance).max(2.0 * pixel);
    loop {
        let crossing = crossing_loops(&loops, cell, &vec![false; loops.len()]);
        let mut changed = false;
        for k in crossing {
            if !as_traced[k] {
                as_traced[k] = true;
                loops[k] = raw[k].clone();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let count = as_traced.iter().filter(|&&b| b).count();
    (loops, count)
}

/// Douglas–Peucker on a closed ring: split at the first point and the point farthest from
/// it, and simplify both halves as open paths.
fn simplify_ring(ring: &[Point], tolerance: f64) -> Vec<Point> {
    let n = ring.len();
    if n < 4 {
        return ring.to_vec();
    }
    let far = (1..n)
        .max_by_key(|&a| Ordered(distance(ring[0], ring[a])))
        .unwrap_or(n / 2);
    let mut path: Vec<Point> = ring.to_vec();
    path.push(ring[0]);
    let mut keep = vec![false; path.len()];
    keep[0] = true;
    keep[far] = true;
    keep[n] = true;
    let mut stack = vec![(0, far), (far, n)];
    while let Some((a, b)) = stack.pop() {
        let worst = (a + 1..b)
            .map(|k| (k, segment_distance(path[k], path[a], path[b])))
            .max_by_key(|&(_, d)| Ordered(d));
        if let Some((k, d)) = worst
            && d > tolerance
        {
            keep[k] = true;
            stack.push((a, k));
            stack.push((k, b));
        }
    }
    (0..n).filter(|&k| keep[k]).map(|k| path[k]).collect()
}

fn distance(a: Point, b: Point) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
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

/// The loops that have a segment crossing or touching another segment (of any loop, or of
/// their own other than its neighbours), found through a grid of `cell` mm squares. A loop
/// marked `near` (flattened arcs) also counts when it comes within twice the chaining
/// tolerance of another loop: its chords lie up to that tolerance off its arcs.
fn crossing_loops(loops: &[Vec<Point>], cell: f64, near: &[bool]) -> Vec<usize> {
    // (loop, segment index) per grid square; BTreeMap so the result never depends on hashing.
    let mut grid: BTreeMap<(i64, i64), Vec<(usize, usize)>> = BTreeMap::new();
    let square = |v: f64| (v / cell).floor() as i64;
    // Boxes grown by the nearness limit, so near segments always share a square.
    let reach = 2.0 * CHAIN_TOL_MM;
    for (l, ring) in loops.iter().enumerate() {
        for s in 0..ring.len() {
            let (a, b) = (ring[s], ring[(s + 1) % ring.len()]);
            for x in square(a.0.min(b.0) - reach)..=square(a.0.max(b.0) + reach) {
                for y in square(a.1.min(b.1) - reach)..=square(a.1.max(b.1) + reach) {
                    grid.entry((x, y)).or_default().push((l, s));
                }
            }
        }
    }
    let mut bad = vec![false; loops.len()];
    for members in grid.values() {
        for (m, &(l1, s1)) in members.iter().enumerate() {
            for &(l2, s2) in &members[m + 1..] {
                if (bad[l1] && bad[l2]) || neighbours(loops, l1, s1, l2, s2) {
                    continue;
                }
                let (a, b) = segment(loops, l1, s1);
                let (c, d) = segment(loops, l2, s2);
                let too_near = l1 != l2
                    && (near[l1] || near[l2])
                    && segment_gap(a, b, c, d) < 2.0 * CHAIN_TOL_MM;
                if too_near || segments_meet(a, b, c, d) {
                    bad[l1] = true;
                    bad[l2] = true;
                }
            }
        }
    }
    (0..loops.len()).filter(|&k| bad[k]).collect()
}

fn segment(loops: &[Vec<Point>], l: usize, s: usize) -> (Point, Point) {
    let ring = &loops[l];
    (ring[s], ring[(s + 1) % ring.len()])
}

/// The same segment, or two that follow each other in one loop (they share a point).
fn neighbours(loops: &[Vec<Point>], l1: usize, s1: usize, l2: usize, s2: usize) -> bool {
    if l1 != l2 {
        return false;
    }
    let n = loops[l1].len();
    s1 == s2 || (s1 + 1) % n == s2 || (s2 + 1) % n == s1
}

/// Whether segments ab and cd cross or touch (exact orientation tests).
fn segments_meet(a: Point, b: Point, c: Point, d: Point) -> bool {
    let o1 = orientation(a, b, c);
    let o2 = orientation(a, b, d);
    let o3 = orientation(c, d, a);
    let o4 = orientation(c, d, b);
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    (o1 == 0.0 && on_segment(a, b, c))
        || (o2 == 0.0 && on_segment(a, b, d))
        || (o3 == 0.0 && on_segment(c, d, a))
        || (o4 == 0.0 && on_segment(c, d, b))
}

/// The shortest distance between segments ab and cd.
fn segment_gap(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if segments_meet(a, b, c, d) {
        return 0.0;
    }
    segment_distance(a, c, d)
        .min(segment_distance(b, c, d))
        .min(segment_distance(c, a, b))
        .min(segment_distance(d, a, b))
}

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    let v = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    v.signum() * f64::from(u8::from(v != 0.0))
}

/// `p`, collinear with ab, lies within its bounding box.
fn on_segment(a: Point, b: Point, p: Point) -> bool {
    p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0) && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_run_becomes_its_corners() {
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
        let simple = simplify_ring(&ring, 0.01);
        assert_eq!(simple.len(), 4, "{simple:?}");
    }

    #[test]
    fn touching_and_crossing_segments_meet() {
        assert!(segments_meet(
            (0.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
            (2.0, 0.0)
        ));
        assert!(segments_meet(
            (0.0, 0.0),
            (2.0, 0.0),
            (1.0, 0.0),
            (1.0, 1.0)
        ));
        assert!(!segments_meet(
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (1.0, 1.0)
        ));
    }
}
