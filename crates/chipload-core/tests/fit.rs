// SPDX-License-Identifier: GPL-3.0-or-later
//! Fitting lines and arcs (docs/specs/image-arcs-centrelines.md §8).

use std::f64::consts::{PI, TAU};

use chipload_core::Point;
use chipload_core::fit::{MAX_FIT_RADIUS_MM, MAX_FIT_SWEEP, fit_path};
use chipload_core::primitives::{Primitive, distance};
use proptest::prelude::*;

/// Distance from `p` to a line or arc.
fn piece_distance(piece: &Primitive, p: Point) -> f64 {
    match piece {
        Primitive::Line(l) => {
            let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
            let length2 = dx * dx + dy * dy;
            if length2 == 0.0 {
                return distance(p, l.start);
            }
            let t = (((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / length2).clamp(0.0, 1.0);
            distance(p, (l.start.0 + t * dx, l.start.1 + t * dy))
        }
        Primitive::Arc(a) => {
            let angle = (p.1 - a.center.1).atan2(p.0 - a.center.0);
            let along = (a.sweep.signum() * (angle - a.start_angle)).rem_euclid(TAU);
            if along <= a.sweep.abs() {
                (distance(p, a.center) - a.radius).abs()
            } else {
                distance(p, piece.start()).min(distance(p, piece.end()))
            }
        }
    }
}

/// A wobbly closed curve: a circle with a few harmonics and per-point noise.
fn ring() -> impl Strategy<Value = Vec<Point>> {
    (
        20usize..400,
        2.0f64..30.0,
        prop::collection::vec((-0.3f64..0.3, 0.0f64..TAU), 0..4),
        0.0f64..0.05,
        prop::collection::vec(-1.0f64..1.0, 400),
    )
        .prop_map(|(n, r, harmonics, noise, jitter)| {
            (0..n)
                .map(|k| {
                    let t = TAU * k as f64 / n as f64;
                    let mut radius = r;
                    for (h, (amp, phase)) in harmonics.iter().enumerate() {
                        radius += r * amp * ((h as f64 + 2.0) * t + phase).sin() / 2.0;
                    }
                    radius += noise * jitter[k];
                    (radius * t.cos(), radius * t.sin())
                })
                .collect()
        })
}

fn check(
    points: &[Point],
    pieces: &[Primitive],
    closed: bool,
    tol: f64,
) -> Result<(), TestCaseError> {
    prop_assert!(!pieces.is_empty());
    let on_input = |q: Point| points.iter().any(|&p| distance(p, q) < 1e-9);
    for (k, piece) in pieces.iter().enumerate() {
        prop_assert!(on_input(piece.start()), "piece {} starts off the input", k);
        prop_assert!(on_input(piece.end()), "piece {} ends off the input", k);
        if let Some(next) = pieces.get(k + 1) {
            prop_assert!(distance(piece.end(), next.start()) < 1e-9);
        }
        if let Primitive::Arc(a) = piece {
            prop_assert!(a.sweep.abs() <= MAX_FIT_SWEEP + 1e-12);
            prop_assert!(a.radius >= tol && a.radius <= MAX_FIT_RADIUS_MM);
        }
    }
    let (first, last) = (pieces[0].start(), pieces[pieces.len() - 1].end());
    if closed {
        prop_assert!(distance(first, last) < 1e-9);
    } else {
        prop_assert!(distance(first, points[0]) < 1e-9);
        prop_assert!(distance(last, points[points.len() - 1]) < 1e-9);
    }
    // The points, and the path between them (its midpoints).
    let n = points.len();
    let pairs = if closed { n } else { n - 1 };
    let mids = (0..pairs).map(|k| {
        let (a, b) = (points[k], points[(k + 1) % n]);
        ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
    });
    let all: Vec<Point> = points.iter().copied().chain(mids).collect();
    for &p in &all {
        let d = pieces
            .iter()
            .map(|piece| piece_distance(piece, p))
            .fold(f64::INFINITY, f64::min);
        prop_assert!(d <= tol + 1e-9, "{:?} is {} from the fit", p, d);
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn closed_fits_stay_within_the_tolerance(
        points in ring(),
        tol in 0.01f64..0.5,
        arcs in any::<bool>(),
    ) {
        let pieces = fit_path(&points, true, tol, arcs);
        check(&points, &pieces, true, tol)?;
        if !arcs {
            prop_assert!(pieces.iter().all(|p| matches!(p, Primitive::Line(_))));
        }
    }

    #[test]
    fn open_fits_stay_within_the_tolerance_and_keep_their_ends(
        points in ring(),
        cut in 0.2f64..1.0,
        tol in 0.01f64..0.5,
    ) {
        let n = ((points.len() as f64 * cut) as usize).max(2);
        let open = &points[..n];
        let pieces = fit_path(open, false, tol, true);
        check(open, &pieces, false, tol)?;
    }
}

#[test]
fn half_a_circle_is_one_arc_and_more_is_split() {
    let half: Vec<Point> = (0..=60)
        .map(|k| {
            let t = PI * k as f64 / 60.0;
            (5.0 * t.cos(), 5.0 * t.sin())
        })
        .collect();
    assert_eq!(fit_path(&half, false, 0.01, true).len(), 1);
    let more: Vec<Point> = (0..=80)
        .map(|k| {
            let t = 1.3 * PI * k as f64 / 80.0;
            (5.0 * t.cos(), 5.0 * t.sin())
        })
        .collect();
    assert_eq!(fit_path(&more, false, 0.01, true).len(), 2);
}
