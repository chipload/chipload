// SPDX-License-Identifier: GPL-3.0-or-later
//! A relief rim's loops filled out towards their convex hull (docs/specs/stl-relief.md
//! §16.7): a morphological closing by a disc whose radius grows with the share asked for,
//! and at the end the convex hull itself.

use cavalier_contours::polyline::{PlineSourceMut, Polyline};
use chipload_core::Point;
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::ToolpathError;
use crate::cavalier::{ccw, offset_loops};
use crate::offset::flatten;

/// The closing's radius at `hull` 0.5, as a share of the loops' diagonal.
const RADIUS_SHARE: f64 = 0.1;

/// The closing's radius for `hull` (0 to 1, below 1) and loops `diagonal` across, mm:
/// 0.1 · diagonal · hull / (1 − hull).
pub fn closing_radius(hull: f64, diagonal: f64) -> f64 {
    RADIUS_SHARE * diagonal * hull / (1.0 - hull)
}

/// `outer` (filled loops, each closed implicitly) filled out by `hull`, 0 to 1: as they are
/// at 0, their convex hull at 1, between them closed by a disc of [`closing_radius`]. Only
/// outer loops come back: voids the closing encloses are filled.
pub fn fuller(outer: &[Vec<Point>], hull: f64) -> Result<Vec<Vec<Point>>, ToolpathError> {
    if hull.is_nan() || hull <= 0.0 || outer.is_empty() {
        return Ok(outer.to_vec());
    }
    if hull >= 1.0 {
        let all: Vec<Point> = outer.iter().flatten().copied().collect();
        let hull = convex_hull(&all);
        return Ok(if hull.len() >= 3 {
            vec![hull]
        } else {
            outer.to_vec()
        });
    }
    let (lo, hi) = bounds(outer);
    let diagonal = (hi.0 - lo.0).hypot(hi.1 - lo.1);
    let r = closing_radius(hull, diagonal);
    if r.is_nan() || r <= CHAIN_TOL_MM {
        return Ok(outer.to_vec());
    }
    let plines: Vec<Polyline<f64>> = outer
        .iter()
        .filter(|l| l.len() >= 3)
        .map(|l| ccw(pline(l)))
        .collect();
    // Grown as one shape, so pieces closer than 2R merge; then each piece shrunk back.
    let grown = offset_loops(plines.into_iter(), -r)?;
    let mut out = Vec::new();
    for area in grown {
        for back in offset_loops(std::iter::once(area.outer), r)? {
            let points = flatten(&back.outer);
            if points.len() >= 3 {
                out.push(points);
            }
        }
    }
    // Shrinking back can't lose everything a loop of the outline lies in; if the library
    // says otherwise, the outline as it is stands.
    Ok(if out.is_empty() { outer.to_vec() } else { out })
}

/// A closed polyline through `points`, lines only.
fn pline(points: &[Point]) -> Polyline<f64> {
    let mut p = Polyline::new_closed();
    for &(x, y) in points {
        p.add(x, y, 0.0);
    }
    p
}

/// The lowest and highest corner of the loops' points.
fn bounds(loops: &[Vec<Point>]) -> (Point, Point) {
    loops.iter().flatten().fold(
        (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                (lo.0.min(p.0), lo.1.min(p.1)),
                (hi.0.max(p.0), hi.1.max(p.1)),
            )
        },
    )
}

/// The convex hull of `points`, counter-clockwise from the lowest-leftmost, no three in a
/// line (Andrew's monotone chain, 1979).
pub fn convex_hull(points: &[Point]) -> Vec<Point> {
    let mut sorted: Vec<Point> = points
        .iter()
        .copied()
        .filter(|p| p.0.is_finite() && p.1.is_finite())
        .collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    // Left of the turn o → a → b: positive.
    let turn = |o: Point, a: Point, b: Point| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let chain = |points: &mut dyn Iterator<Item = Point>| {
        let mut half: Vec<Point> = Vec::new();
        for p in points {
            while half.len() >= 2 && turn(half[half.len() - 2], half[half.len() - 1], p) <= 0.0 {
                half.pop();
            }
            half.push(p);
        }
        half.pop();
        half
    };
    let mut lower = chain(&mut sorted.iter().copied());
    let upper = chain(&mut sorted.iter().rev().copied());
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A U, 30 wide and 30 tall, its slot 10 wide and 20 deep from the top.
    fn u() -> Vec<Point> {
        vec![
            (0.0, 0.0),
            (30.0, 0.0),
            (30.0, 30.0),
            (20.0, 30.0),
            (20.0, 10.0),
            (10.0, 10.0),
            (10.0, 30.0),
            (0.0, 30.0),
        ]
    }

    fn area(l: &[Point]) -> f64 {
        let n = l.len();
        (0..n)
            .map(|i| {
                let (a, b) = (l[i], l[(i + 1) % n]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn the_hull_of_points_goes_round_them_counter_clockwise() {
        let hull = convex_hull(&u());
        assert_eq!(
            hull,
            vec![(0.0, 0.0), (30.0, 0.0), (30.0, 30.0), (0.0, 30.0)]
        );
        assert!(convex_hull(&[(0.0, 0.0), (1.0, 1.0)]).len() < 3);
    }

    #[test]
    fn nothing_leaves_the_outline_and_everything_gives_its_hull() {
        let outline = vec![u()];
        assert_eq!(fuller(&outline, 0.0).unwrap(), outline);
        let hull = fuller(&outline, 1.0).unwrap();
        assert_eq!(hull.len(), 1);
        assert!((area(&hull[0]) - 900.0).abs() < 1e-9);
    }

    #[test]
    fn halfway_fills_the_slot_and_keeps_the_outer_corners() {
        // Diagonal 42.4 mm: R = 4.24 mm at 0.5 does not bridge the 10 mm slot, at 0.7 (9.9
        // mm) it does.
        let some = fuller(&[u()], 0.5).unwrap();
        assert_eq!(some.len(), 1);
        let a = area(&some[0]).abs();
        assert!(a > 700.0 + 1.0 && a < 900.0 - 1.0, "{a}");
        let more = fuller(&[u()], 0.7).unwrap();
        let b = area(&more[0]).abs();
        assert!(b > a && b < 900.0, "{b}");
        // Never outside the hull, so never outside the outline's box.
        for p in more[0].iter().chain(&some[0]) {
            assert!((-1e-6..=30.0 + 1e-6).contains(&p.0), "{p:?}");
            assert!((-1e-6..=30.0 + 1e-6).contains(&p.1), "{p:?}");
        }
        // The bottom corners stay square: a closing keeps convex corners.
        let near = |q: Point| more[0].iter().any(|p| (p.0 - q.0).hypot(p.1 - q.1) < 0.05);
        assert!(near((0.0, 0.0)) && near((30.0, 0.0)));
    }

    #[test]
    fn two_pieces_close_together_merge() {
        let square = |x: f64| vec![(x, 0.0), (x + 10.0, 0.0), (x + 10.0, 10.0), (x, 10.0)];
        let pieces = vec![square(0.0), square(12.0)];
        // Diagonal 23.3 mm: R = 2.3 mm at 0.5 bridges the 2 mm gap.
        let merged = fuller(&pieces, 0.5).unwrap();
        assert_eq!(merged.len(), 1);
        assert!(area(&merged[0]).abs() > 220.0 - 1.0);
        // Hardly filled, they stay apart.
        assert_eq!(fuller(&pieces, 0.02).unwrap().len(), 2);
    }
}
