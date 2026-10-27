// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounding and cutting the corners of a polyline (docs/specs/drawing-editing.md §3). A
//! corner is a vertex between two straight segments. A fillet of radius r touches both
//! sides at r / tan(θ / 2) from the corner, θ the angle between them; a chamfer cuts
//! straight across at a distance along each side.

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};

/// A polyline as points and one bulge per segment (DXF meaning: 0 straight).
#[derive(Debug, Clone, PartialEq)]
pub struct Poly {
    pub points: Vec<Point>,
    pub bulges: Vec<f64>,
    pub closed: bool,
}

/// What is done to a corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cut {
    /// An arc of this radius, tangent to both sides.
    Fillet(f64),
    /// A straight cut this far back along each side.
    Chamfer(f64),
}

/// Why corners can't be done.
#[derive(Debug, Clone, PartialEq)]
pub enum CornerError {
    /// The vertex isn't between two straight segments (an end, or an arc beside it).
    NotACorner(usize),
    /// The two sides run straight on: there is no corner.
    Straight(usize),
    /// The polyline has no corner at all to do.
    NoCorners,
    /// The radius or distance needs more of a side than it has; the most that fits.
    TooLarge { most: f64 },
    /// The radius or distance isn't a positive number.
    BadSize,
}

/// The vertices of `poly` that are corners: between two straight segments that turn.
pub fn corners(poly: &Poly) -> Vec<usize> {
    (0..poly.points.len())
        .filter(|&k| corner_angle(poly, k).is_ok())
        .collect()
}

/// `poly` with corner `k` (or, None, every corner) cut. Refused when a cut needs more of a
/// side than it has, counting both corners on a side when both are cut.
pub fn cut_corners(poly: &Poly, corner: Option<usize>, cut: Cut) -> Result<Poly, CornerError> {
    let size = match cut {
        Cut::Fillet(r) | Cut::Chamfer(r) => r,
    };
    if !(size.is_finite() && size > 0.0) {
        return Err(CornerError::BadSize);
    }
    let chosen: Vec<usize> = match corner {
        Some(k) => {
            corner_angle(poly, k)?;
            vec![k]
        }
        None => corners(poly),
    };
    if chosen.is_empty() {
        return Err(CornerError::NoCorners);
    }
    // How far back along each side each corner reaches, per unit of size.
    let n = poly.points.len();
    let mut per_size = vec![0.0; n];
    for &k in &chosen {
        let turn = corner_angle(poly, k)?;
        per_size[k] = match cut {
            // The angle between the sides is π − turn; the tangent points lie
            // r / tan(that / 2) from the corner.
            Cut::Fillet(_) => 1.0 / ((std::f64::consts::PI - turn) / 2.0).tan(),
            Cut::Chamfer(_) => 1.0,
        };
    }
    // Each side holds the reaches of the corners at its two ends.
    let segments = if poly.closed { n } else { n - 1 };
    let mut most = f64::INFINITY;
    for i in 0..segments {
        let j = (i + 1) % n;
        let reach = per_size[i] + per_size[j];
        if reach > 0.0 {
            most = most.min(distance(poly.points[i], poly.points[j]) / reach);
        }
    }
    if size > most + CHAIN_TOL_MM / 2.0 {
        return Err(CornerError::TooLarge { most });
    }
    // From the last corner to the first, so earlier indices stay put.
    let mut out = poly.clone();
    let mut sorted = chosen;
    sorted.sort_unstable();
    for &k in sorted.iter().rev() {
        replace_corner(&mut out, k, size * per_size[k], cut);
    }
    drop_empty_segments(&mut out);
    Ok(out)
}

/// Where cuts use a whole side, the tangent points of its two ends meet (or one meets the
/// next vertex): the segment between them has no length, and goes with its start point,
/// so the segment before it ends where it began.
fn drop_empty_segments(poly: &mut Poly) {
    let mut i = 0;
    while poly.points.len() > 2 {
        let n = poly.points.len();
        let segments = if poly.closed { n } else { n - 1 };
        if i >= segments {
            break;
        }
        let next = (i + 1) % n;
        if distance(poly.points[i], poly.points[next]) < CHAIN_TOL_MM {
            poly.points.remove(i);
            poly.bulges.remove(i);
        } else {
            i += 1;
        }
    }
}

/// The angle the polyline turns at corner `k` (0 straight on, toward π a hairpin), or why
/// `k` isn't a corner.
fn corner_angle(poly: &Poly, k: usize) -> Result<f64, CornerError> {
    let n = poly.points.len();
    let has_in = poly.closed || k > 0;
    let has_out = poly.closed || k + 1 < n;
    if k >= n || n < 3 || !has_in || !has_out {
        return Err(CornerError::NotACorner(k));
    }
    let before = (k + n - 1) % n;
    let after = (k + 1) % n;
    let bulge = |i: usize| poly.bulges.get(i).copied().unwrap_or(0.0);
    if bulge(before) != 0.0 || bulge(k) != 0.0 {
        return Err(CornerError::NotACorner(k));
    }
    let (p, a, b) = (poly.points[k], poly.points[before], poly.points[after]);
    if distance(a, p) < CHAIN_TOL_MM || distance(p, b) < CHAIN_TOL_MM {
        return Err(CornerError::NotACorner(k));
    }
    let turn = corner_turn(a, p, b);
    if turn < ANGLE_TOL_RAD || std::f64::consts::PI - turn < ANGLE_TOL_RAD {
        return Err(CornerError::Straight(k));
    }
    Ok(turn)
}

/// Corner `k` replaced by two points `back` from it along each side, joined by the fillet's
/// arc or the chamfer's line.
fn replace_corner(poly: &mut Poly, k: usize, back: f64, cut: Cut) {
    let n = poly.points.len();
    let (p, a, b) = (
        poly.points[k],
        poly.points[(k + n - 1) % n],
        poly.points[(k + 1) % n],
    );
    let toward = |q: Point| {
        let d = distance(p, q);
        (p.0 + (q.0 - p.0) * back / d, p.1 + (q.1 - p.1) * back / d)
    };
    let (t1, t2) = (toward(a), toward(b));
    let bulge = match cut {
        Cut::Chamfer(_) => 0.0,
        Cut::Fillet(_) => {
            // The arc turns as the corner did: left (counter-clockwise) for a left turn.
            let cross = (p.0 - a.0) * (b.1 - p.1) - (p.1 - a.1) * (b.0 - p.0);
            let turn = corner_turn(a, p, b);
            let sweep = if cross > 0.0 { turn } else { -turn };
            (sweep / 4.0).tan()
        }
    };
    poly.points[k] = t1;
    poly.points.insert(k + 1, t2);
    poly.bulges.insert(k, bulge);
}

/// How far the direction turns at `p` going from `a` to `b`, radians, 0 to π.
fn corner_turn(a: Point, p: Point, b: Point) -> f64 {
    let incoming = (p.1 - a.1).atan2(p.0 - a.0);
    let outgoing = (b.1 - p.1).atan2(b.0 - p.0);
    ((outgoing - incoming + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI)
        .abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_core::primitives::Arc;

    fn square() -> Poly {
        Poly {
            points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            bulges: vec![0.0; 4],
            closed: true,
        }
    }

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-9
    }

    #[test]
    fn a_fillet_is_tangent_to_both_sides() {
        let done = cut_corners(&square(), Some(1), Cut::Fillet(2.0)).unwrap();
        assert_eq!(done.points.len(), 5);
        assert!(close(done.points[1], (8.0, 0.0)));
        assert!(close(done.points[2], (10.0, 2.0)));
        // A quarter circle, counter-clockwise (the square turns left), about (8, 2).
        let arc = Arc::from_bulge(done.points[1], done.points[2], done.bulges[1]).unwrap();
        assert!(close(arc.center, (8.0, 2.0)));
        assert!((arc.radius - 2.0).abs() < 1e-9);
        assert!((arc.sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert_eq!(done.bulges.len(), 5);
    }

    #[test]
    fn every_corner_shares_its_sides() {
        // 5 on every corner uses each whole side: the tangent points meet, and no segment
        // of no length is left between them (a circle of four quarters).
        let all = cut_corners(&square(), None, Cut::Fillet(5.0)).unwrap();
        assert_eq!((all.points.len(), all.bulges.len()), (4, 4));
        assert!(all.bulges.iter().all(|&b| b > 0.0));
        let some = cut_corners(&square(), None, Cut::Fillet(2.0)).unwrap();
        assert_eq!(some.points.len(), 8);
        // 5 is all a 10 mm side holds with both ends rounded.
        let Err(CornerError::TooLarge { most }) = cut_corners(&square(), None, Cut::Fillet(6.0))
        else {
            panic!("too large")
        };
        assert!((most - 5.0).abs() < 1e-9);
        // One corner alone may take the whole side; the point it reaches is the next one.
        let whole = cut_corners(&square(), Some(0), Cut::Fillet(10.0)).unwrap();
        let n = whole.points.len();
        assert_eq!(whole.bulges.len(), n);
        for i in 0..n {
            assert!(distance(whole.points[i], whole.points[(i + 1) % n]) >= CHAIN_TOL_MM);
        }
        let chamfered = cut_corners(&square(), Some(2), Cut::Chamfer(3.0)).unwrap();
        assert!(close(chamfered.points[2], (10.0, 7.0)));
        assert!(close(chamfered.points[3], (7.0, 10.0)));
        assert_eq!(chamfered.bulges[2], 0.0);
    }

    #[test]
    fn ends_arcs_and_straight_runs_are_not_corners() {
        let open = Poly {
            points: vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (20.0, 10.0)],
            bulges: vec![0.0, 0.0, 0.5],
            closed: false,
        };
        assert_eq!(
            cut_corners(&open, Some(0), Cut::Fillet(1.0)),
            Err(CornerError::NotACorner(0))
        );
        assert_eq!(
            cut_corners(&open, Some(1), Cut::Fillet(1.0)),
            Err(CornerError::Straight(1))
        );
        assert_eq!(
            cut_corners(&open, Some(2), Cut::Fillet(1.0)),
            Err(CornerError::NotACorner(2))
        );
        assert_eq!(
            cut_corners(&open, None, Cut::Fillet(1.0)),
            Err(CornerError::NoCorners)
        );
        assert_eq!(
            cut_corners(&square(), Some(1), Cut::Fillet(0.0)),
            Err(CornerError::BadSize)
        );
    }

    #[test]
    fn a_sharp_corner_takes_far_more_side_than_a_blunt_one() {
        // A thin wedge: 10° at the tip.
        let tip = (10f64.to_radians() / 2.0).tan() * 100.0;
        let wedge = Poly {
            points: vec![(0.0, 0.0), (100.0, -tip), (100.0, tip)],
            bulges: vec![0.0; 3],
            closed: true,
        };
        let done = cut_corners(&wedge, Some(0), Cut::Fillet(1.0)).unwrap();
        // r / tan(5°) ≈ 11.43 back along each side.
        let back = distance((0.0, 0.0), done.points[0]);
        assert!((back - 1.0 / 5f64.to_radians().tan()).abs() < 1e-9);
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// A fillet's arc starts and ends along the sides: its centre is r from both.
            #[test]
            fn fillets_are_tangent(angle in 0.2..3.0f64, r in 0.1..5.0f64) {
                let b = (100.0 * angle.cos(), 100.0 * angle.sin());
                let poly = Poly {
                    points: vec![(-100.0, 0.0), (0.0, 0.0), b],
                    bulges: vec![0.0, 0.0],
                    closed: false,
                };
                let done = cut_corners(&poly, Some(1), Cut::Fillet(r)).unwrap();
                let arc = Arc::from_bulge(done.points[1], done.points[2], done.bulges[1]).unwrap();
                prop_assert!((arc.radius - r).abs() < 1e-6);
                // The centre is r from both lines (y = 0 and the line through 0 and b).
                prop_assert!((arc.center.1.abs() - r).abs() < 1e-6);
                let off = (b.0 * arc.center.1 - b.1 * arc.center.0).abs() / 100.0;
                prop_assert!((off - r).abs() < 1e-6);
            }
        }
    }
}
