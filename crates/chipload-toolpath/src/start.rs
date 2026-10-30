// SPDX-License-Identifier: GPL-3.0-or-later
//! Start points chosen by the user (docs/specs/helix-and-leads.md, part A). Each stored point
//! belongs to the ring nearest to it, and that ring starts at its point nearest to the
//! stored one. Rings without a point keep the start the planner gave them.

use chipload_core::Point;
use chipload_core::order::Ordered;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::arcs::{ArcTable, Circle};

/// The items within CHAIN_TOL_MM of the nearest one, in their order: equally near as far as
/// geometry goes. A choice of "the nearest" takes one of these by a second key, never by
/// the exact distance: on a symmetric drawing two candidates are equally near up to the
/// last bits, which differ between platforms' maths libraries, and the G-code followed them
/// (golden v31_svg_badge on Windows, macOS and ARM, 2026-10-29).
pub(crate) fn nearest_ties<T>(items: impl IntoIterator<Item = (T, f64)>) -> Vec<T> {
    let items: Vec<(T, f64)> = items.into_iter().collect();
    let nearest = items.iter().map(|i| i.1).fold(f64::INFINITY, f64::min);
    items
        .into_iter()
        .filter(|i| i.1 <= nearest + CHAIN_TOL_MM)
        .map(|i| i.0)
        .collect()
}

/// The point of segment a-b nearest to `p`.
fn nearest_on_segment(a: Point, b: Point, p: Point) -> Point {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return a;
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    (a.0 + dx * t, a.1 + dy * t)
}

/// The closed ring's point nearest to `p`: the index of the chord it lies on (from
/// ring[i] to ring[i + 1], wrapping), the point, and its distance from `p`. Points equally
/// near (`nearest_ties`) go to the lowest, then the leftmost, then the first chord. None for
/// an empty ring.
pub fn nearest_on_ring(ring: &[Point], p: Point) -> Option<(usize, Point, f64)> {
    let n = ring.len();
    let found = (0..n).map(|i| {
        let q = nearest_on_segment(ring[i], ring[(i + 1) % n], p);
        ((i, q, distance(p, q)), distance(p, q))
    });
    nearest_ties(found)
        .into_iter()
        .min_by_key(|&(i, q, _)| (Ordered(q.1), Ordered(q.0), i))
}

/// The point a stored start chose on its ring, and how far the stored point was from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chosen {
    /// Index in the operation's `starts`.
    pub index: usize,
    pub stored: Point,
    pub distance: f64,
}

/// For each ring, the first stored point whose nearest ring it is. Later points on a ring
/// that already has one are returned as duplicates: (the point's index, the ring's).
pub fn assign(starts: &[Point], rings: &[&[Point]]) -> (Vec<Option<Chosen>>, Vec<(usize, usize)>) {
    let mut chosen: Vec<Option<Chosen>> = vec![None; rings.len()];
    let mut duplicates = Vec::new();
    for (index, &stored) in starts.iter().enumerate() {
        // Ties (`nearest_ties`) go to the earlier ring, so the result doesn't depend on
        // float noise.
        let found = rings
            .iter()
            .enumerate()
            .filter_map(|(r, ring)| nearest_on_ring(ring, stored).map(|(_, _, d)| ((r, d), d)));
        let best: Option<(usize, f64)> = nearest_ties(found).into_iter().next();
        let Some((r, d)) = best else { continue };
        if chosen[r].is_some() {
            duplicates.push((index, r));
        } else {
            chosen[r] = Some(Chosen {
                index,
                stored,
                distance: d,
            });
        }
    }
    (chosen, duplicates)
}

/// The closed ring, keeping its direction, starting at its point nearest to `at`.
///
/// A point within the chaining tolerance of a vertex starts there. Otherwise the chord is
/// split: on a chord of a known arc the new vertex goes onto the arc (so both halves are
/// chords of it again, and are recorded as such); on a straight one it is the projection.
pub fn restart(ring: &[Point], at: Point, arcs: &mut ArcTable) -> Vec<Point> {
    let n = ring.len();
    let Some((i, q, _)) = nearest_on_ring(ring, at) else {
        return ring.to_vec();
    };
    let (a, b) = (ring[i], ring[(i + 1) % n]);
    if distance(q, a) <= CHAIN_TOL_MM {
        return rotated(ring, i);
    }
    if distance(q, b) <= CHAIN_TOL_MM {
        return rotated(ring, (i + 1) % n);
    }
    let split = match arcs.get(a, b) {
        Some(circle) => onto(circle, q),
        None => q,
    };
    if let Some(circle) = arcs.get(a, b) {
        arcs.insert(a, split, circle);
        arcs.insert(split, b, circle);
    }
    let mut out = Vec::with_capacity(n + 1);
    out.push(split);
    out.extend((1..=n).map(|k| ring[(i + k) % n]));
    out
}

/// `p` moved radially onto the circle.
pub(crate) fn onto(circle: Circle, p: Point) -> Point {
    let (dx, dy) = (p.0 - circle.center.0, p.1 - circle.center.1);
    let len = dx.hypot(dy);
    if len == 0.0 {
        return p;
    }
    (
        circle.center.0 + dx / len * circle.radius,
        circle.center.1 + dy / len * circle.radius,
    )
}

fn rotated(ring: &[Point], start: usize) -> Vec<Point> {
    let mut out = ring.to_vec();
    out.rotate_left(start);
    out
}

/// "(12.000, 3.500) mm", for notes.
pub fn point_value(p: Point) -> chipload_i18n::Value {
    chipload_i18n::Value::Point(p.0, p.1, chipload_i18n::Digits::Fixed(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Point> {
        vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    }

    #[test]
    fn starts_at_the_projection_on_a_straight_chord() {
        let mut arcs = ArcTable::default();
        let ring = restart(&square(), (12.0, 4.0), &mut arcs);
        assert_eq!(
            ring,
            vec![
                (10.0, 4.0),
                (10.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
                (10.0, 0.0)
            ]
        );
    }

    #[test]
    fn a_point_at_a_vertex_starts_there_without_a_split() {
        let mut arcs = ArcTable::default();
        let ring = restart(&square(), (10.0005, 10.0), &mut arcs);
        assert_eq!(ring.len(), 4);
        assert_eq!(ring[0], (10.0, 10.0));
        assert_eq!(ring[1], (0.0, 10.0));
    }

    #[test]
    fn an_arc_chord_is_split_on_the_arc_and_both_halves_stay_arc_chords() {
        let circle = Circle {
            center: (0.0, 0.0),
            radius: 10.0,
        };
        let a = (10.0, 0.0);
        let b = (0.0, 10.0);
        let ring = vec![a, b, (-10.0, 0.0), (0.0, -10.0)];
        let mut arcs = ArcTable::default();
        arcs.insert(a, b, circle);
        let out = restart(&ring, (8.0, 8.0), &mut arcs);
        let p = out[0];
        assert!((p.0.hypot(p.1) - 10.0).abs() < 1e-9);
        assert_eq!(arcs.get(a, p), Some(circle));
        assert_eq!(arcs.get(p, b), Some(circle));
        assert_eq!(out[1], b);
        assert_eq!(out[4], a);
    }

    /// Equally near up to float noise is a tie, whatever the noise: the lowest point wins.
    #[test]
    fn equally_near_points_go_to_the_lowest_whatever_the_noise() {
        for noise in [0.0, 1e-15, -1e-15, 1e-9] {
            let ring = vec![(0.0, 0.0), (10.0 + noise, 0.0), (10.0, 10.0), (0.0, 10.0)];
            // The middle is as near to all four sides: the bottom one wins.
            let (_, q, _) = nearest_on_ring(&ring, (5.0, 5.0)).unwrap();
            assert!(distance(q, (5.0, 0.0)) < 1e-6, "{noise}: {q:?}");
        }
        assert_eq!(
            nearest_ties([("a", 1.0), ("b", 1.0005), ("c", 1.01)]),
            vec!["a", "b"]
        );
    }

    #[test]
    fn each_point_goes_to_its_nearest_ring_and_the_first_wins() {
        let near: Vec<Point> = square();
        let far: Vec<Point> = square().iter().map(|p| (p.0 + 50.0, p.1)).collect();
        let starts = [(52.0, -1.0), (5.0, 11.0), (61.0, 5.0)];
        let (chosen, duplicates) = assign(&starts, &[&near, &far]);
        assert_eq!(chosen[0].map(|c| c.index), Some(1));
        assert_eq!(chosen[1].map(|c| c.index), Some(0));
        assert!((chosen[0].unwrap().distance - 1.0).abs() < 1e-12);
        assert_eq!(duplicates, vec![(2, 1)]);
    }
}
