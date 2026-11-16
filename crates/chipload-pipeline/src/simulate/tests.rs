// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// A sampled reference for [`lowest_over`].
fn sampled(p: (f64, f64), a: (f64, f64, f64), b: (f64, f64, f64), shape: Shape) -> Option<f64> {
    let n = 200_000;
    (0..=n)
        .filter_map(|k| {
            let t = k as f64 / n as f64;
            let c = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            let d = math::hypot(c.0 - p.0, c.1 - p.1);
            let h = match shape {
                Shape::Flat { radius } => (d <= radius).then_some(0.0),
                Shape::Cone { radius, tip, slope } => {
                    (d <= radius).then(|| slope * (d - tip).max(0.0))
                }
                Shape::Round(cutter) => cutter.height_at(d),
            }?;
            Some(a.2 + (b.2 - a.2) * t + h)
        })
        .reduce(f64::min)
}

#[test]
fn the_exact_minimum_agrees_with_sampling() {
    let shapes = [
        Shape::Flat { radius: 3.0 },
        Shape::Cone {
            radius: 6.35,
            tip: 0.0,
            slope: 1.0 / math::tan(30.0f64.to_radians()),
        },
        Shape::Cone {
            radius: 6.35,
            tip: 0.5,
            slope: 1.0 / math::tan(45.0f64.to_radians()),
        },
        Shape::Cone {
            radius: 4.0,
            tip: 0.0,
            slope: 1.0 / math::tan(59.0f64.to_radians()),
        },
        Shape::Round(Cutter::Ball { radius: 3.0 }),
        Shape::Round(Cutter::TaperedBall {
            radius: 3.175,
            ball: 0.5,
            half_angle: 7.5f64.to_radians(),
        }),
        Shape::Round(Cutter::Bull {
            radius: 3.0,
            corner: 1.0,
        }),
    ];
    // A deterministic spread of moves and points.
    let mut seed = 12345u64;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 11) as f64) / ((1u64 << 53) as f64)
    };
    for shape in shapes {
        for _ in 0..300 {
            let a = (next() * 20.0, next() * 20.0, -next() * 5.0);
            let b = (next() * 20.0, next() * 20.0, -next() * 5.0);
            let p = (next() * 20.0, next() * 20.0);
            let exact = lowest_over(p, a, b, shape);
            let reference = sampled(p, a, b, shape);
            match (exact, reference) {
                (Some(e), Some(r)) => {
                    assert!(e <= r + 1e-9, "{shape:?} {a:?} {b:?} {p:?}: {e} > {r}");
                    assert!(r - e < 1e-3, "{shape:?} {a:?} {b:?} {p:?}: {e} vs {r}");
                }
                (None, None) => {}
                // Only at the very edge of reach, where sampling can miss it.
                (Some(_), None) => {}
                (None, Some(r)) => panic!("{shape:?} {a:?} {b:?} {p:?}: missed {r}"),
            }
        }
    }
}

#[test]
fn a_coarser_grid_keeps_the_lowest_cell() {
    let sim = Simulation {
        origin: (0.0, 0.0),
        cell: 1.0,
        columns: 3,
        rows: 2,
        heights: vec![0.0, -1.0, 0.0, 0.0, 0.0, -2.0],
        floors: vec![-10.0, -10.0, -9.0, -10.0, -10.0, -10.0],
        top: 0.0,
        bottom: -10.0,
        size: (3.0, 2.0),
        numbers: Numbers {
            removed_cm3: 0.0,
            deepest: Some(-2.0),
            below_bottom: None,
            drill_point_assumed: false,
            vbit_angle_assumed: false,
            through_cm2: None,
            meet_cm2: None,
        },
    };
    let coarse = sim.coarsened(2);
    assert_eq!((coarse.columns, coarse.rows, coarse.cell), (2, 1, 2.0));
    assert_eq!(coarse.heights, vec![-1.0, -2.0]);
    // And the highest floor, so a narrow cut from below stays visible.
    assert_eq!(coarse.floors, vec![-10.0, -9.0]);
    assert_eq!(sim.factor_for(2, 10), 2);
    assert_eq!(sim.factor_for(100, 2), 2);
    assert_eq!(sim.factor_for(100, 100), 1);
}

#[test]
fn the_automatic_cell_keeps_the_budget() {
    assert_eq!(auto_cell(200.0, 200.0), 0.1);
    assert_eq!(auto_cell(600.0, 400.0), 0.25);
    // 200 × 200 mm is exactly 4 million cells at 0.1 mm; a little more is over.
    assert_eq!(cells_for(200.0, 200.0, 0.1), 4_000_000);
    assert_eq!(auto_cell(200.1, 200.0), 0.2);
    assert_eq!(auto_cell(5000.0, 5000.0), 2.0);
}
