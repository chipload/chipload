// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use proptest::prelude::*;

/// Reversals (a high or low) between two others that the radius reaches, or leaves, by
/// a swing smaller than `step` (a walk's first and last move are free).
fn small_reversals(r: &[f64], step: f64) -> usize {
    let mut flat: Vec<f64> = Vec::new();
    for &x in r {
        if flat.last() != Some(&x) {
            flat.push(x);
        }
    }
    let r = flat;
    let mut turns = vec![0];
    for i in 1..r.len().saturating_sub(1) {
        let peak = r[i] > r[i - 1] && r[i] > r[i + 1];
        let dip = r[i] < r[i - 1] && r[i] < r[i + 1];
        if peak || dip {
            turns.push(i);
        }
    }
    turns.push(r.len() - 1);
    (2..turns.len().saturating_sub(2))
        .filter(|&k| {
            let before = (r[turns[k]] - r[turns[k - 1]]).abs();
            let after = (r[turns[k + 1]] - r[turns[k]]).abs();
            before < step || after < step
        })
        .count()
}

proptest! {
    /// A wobbling radius: no reversal smaller than the step is left between others,
    /// nothing gets wider, and nothing narrower by the step or more.
    #[test]
    fn smoothing_leaves_no_small_wobble(
        base in prop::collection::vec(0.0f64..3.0, 2..12),
        noise in prop::collection::vec(-0.004f64..0.004, 200),
    ) {
        let step = 0.005;
        let n = noise.len();
        let mut walk: Vec<(Point, f64)> = (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64 * (base.len() - 1) as f64;
                let k = (t.floor() as usize).min(base.len() - 2);
                let f = t - k as f64;
                let r = base[k] + (base[k + 1] - base[k]) * f + noise[i];
                ((i as f64, 0.0), r.max(0.0))
            })
            .collect();
        let exact: Vec<f64> = walk.iter().map(|w| w.1).collect();
        smooth_radii(&mut walk, step);
        let smooth: Vec<f64> = walk.iter().map(|w| w.1).collect();
        for (s, e) in smooth.iter().zip(&exact) {
            prop_assert!(*s <= *e + 1e-12, "{s} wider than {e}");
            prop_assert!(e - s < step, "{s} too far below {e}");
        }
        prop_assert_eq!(small_reversals(&smooth, step), 0);
    }
}
