// SPDX-License-Identifier: GPL-3.0-or-later
//! Simplifying the traced loops (spec §3.4): Douglas–Peucker within the smoothing, then a
//! check that no simplified loop crosses itself or another. A loop that does, or that
//! collapses, is kept as traced; traced loops never cross, so that always ends.

use std::collections::BTreeMap;

use chipload_core::Point;
use chipload_core::primitives::signed_area;

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
        let crossing = crossing_loops(&loops, cell);
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
        .max_by(|&a, &b| distance(ring[0], ring[a]).total_cmp(&distance(ring[0], ring[b])))
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
            .max_by(|x, y| x.1.total_cmp(&y.1));
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
/// their own other than its neighbours), found through a grid of `cell` mm squares.
fn crossing_loops(loops: &[Vec<Point>], cell: f64) -> Vec<usize> {
    // (loop, segment index) per grid square; BTreeMap so the result never depends on hashing.
    let mut grid: BTreeMap<(i64, i64), Vec<(usize, usize)>> = BTreeMap::new();
    let square = |v: f64| (v / cell).floor() as i64;
    for (l, ring) in loops.iter().enumerate() {
        for s in 0..ring.len() {
            let (a, b) = (ring[s], ring[(s + 1) % ring.len()]);
            for x in square(a.0.min(b.0))..=square(a.0.max(b.0)) {
                for y in square(a.1.min(b.1))..=square(a.1.max(b.1)) {
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
                if segments_meet(a, b, c, d) {
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
