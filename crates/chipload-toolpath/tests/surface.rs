// SPDX-License-Identifier: GPL-3.0-or-later
//! Surfacing (docs/specs/surfacing.md): passes that cover the stock, stay within it grown by
//! the tool radius, and go down only beside it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::Point;
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::surface::{
    SIDE_OVERLAP_DIAMETERS, SurfaceCut, pass_positions, surface_moves,
};
use proptest::prelude::*;

mod common;

const TOL: f64 = 1e-9;

fn cut(stock: (Point, Point), tool_radius: f64, stepover: f64) -> SurfaceCut {
    SurfaceCut {
        stock,
        tool_radius,
        stepover,
        safe_z: 5.0,
        feed: 3000.0,
        plunge_feed: 1000.0,
    }
}

/// Every move with the position it ends at, the position filled in from the moves before.
fn positions(moves: &[Move]) -> Vec<(Move, Point, f64)> {
    let (mut x, mut y, mut z) = (f64::NAN, f64::NAN, f64::NAN);
    moves
        .iter()
        .map(|m| {
            x = m.x.unwrap_or(x);
            y = m.y.unwrap_or(y);
            z = m.z.unwrap_or(z);
            (*m, (x, y), z)
        })
        .collect()
}

#[test]
fn passes_are_even_and_reach_past_the_sides() {
    // 100 mm across, a 10 mm tool, stepover 7.5: the first and last centres 2.5 mm in, so
    // the tool's edge passes each side by 2.5 mm (a quarter of the diameter).
    let v = pass_positions(0.0, 100.0, 5.0, 7.5);
    assert!((v[0] - 2.5).abs() < TOL && (v[v.len() - 1] - 97.5).abs() < TOL);
    let gaps: Vec<f64> = v.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.iter().all(|g| *g <= 7.5 + TOL), "{gaps:?}");
    assert!(gaps.iter().all(|g| (g - gaps[0]).abs() < TOL), "{gaps:?}");
    assert_eq!(v.len(), 14, "95 mm in steps of at most 7.5: 13 gaps");
    // A stepover that divides the span exactly gives no extra pass.
    assert_eq!(pass_positions(0.0, 100.0, 5.0, 9.5).len(), 11);
    // A tool that covers the stock with the overlap on both sides (the stock at most the
    // tool's radius wide): one pass in the middle.
    assert_eq!(pass_positions(0.0, 10.0, 12.5, 18.75), vec![5.0]);
    // Wider than that: two passes, each side passed by a quarter of the diameter.
    assert_eq!(pass_positions(0.0, 20.0, 12.5, 18.75), vec![6.25, 13.75]);
    assert_eq!(SIDE_OVERLAP_DIAMETERS, 0.25);
}

#[test]
fn two_depths_go_back_the_other_way_without_a_retract() {
    let (moves, end) = surface_moves(
        &cut(((0.0, 0.0), (120.0, 80.0)), 10.0, 15.0),
        &[-0.5, -1.0],
        (0.0, 0.0),
    );
    let at = positions(&moves);
    // Down at the corner nearest the start, beside the stock: the tool clear of its end.
    assert_eq!(at[1].1, (-10.0, 5.0));
    let rapids_up: Vec<_> = at
        .iter()
        .filter(|(m, _, z)| m.kind == MoveKind::Rapid && (*z - 5.0).abs() < TOL)
        .collect();
    // Up only at the start and at the end.
    assert_eq!(
        rapids_up.len(),
        3,
        "safe Z, the XY rapid there, and the last lift"
    );
    let second = at
        .iter()
        .position(|(m, _, z)| m.kind == MoveKind::Feed && (*z + 1.0).abs() < TOL)
        .unwrap();
    // The second depth starts where the first ended, and ends back at the first pass.
    assert_eq!(at[second].1, at[second - 1].1);
    assert!((end.1 - 5.0).abs() < TOL, "{end:?}");
}

#[test]
fn a_tall_stock_is_faced_along_y() {
    let (moves, _) = surface_moves(
        &cut(((0.0, 0.0), (60.0, 200.0)), 6.0, 9.0),
        &[-0.5],
        (0.0, 0.0),
    );
    let at = positions(&moves);
    let longest = at
        .windows(2)
        .filter(|w| w[1].0.kind == MoveKind::Feed)
        .map(|w| ((w[1].1.0 - w[0].1.0).abs(), (w[1].1.1 - w[0].1.1).abs()))
        .fold((0.0f64, 0.0f64), |a, b| (a.0.max(b.0), a.1.max(b.1)));
    assert!((longest.1 - 212.0).abs() < TOL, "{longest:?}");
    assert!(longest.0 < 60.0, "{longest:?}");
}

proptest! {
    #![proptest_config(common::proptest_config(200))]

    /// Every pass and move stays within the stock grown by the tool radius (where the
    /// checker counts a cut as on the stock), the passes cover the stock, and the tool goes
    /// down only where it is clear of the stock's ends.
    #[test]
    fn passes_cover_the_stock_and_go_down_beside_it(
        x0 in -300.0f64..300.0,
        y0 in -300.0f64..300.0,
        w in 5.0f64..600.0,
        h in 5.0f64..600.0,
        radius in 0.5f64..20.0,
        fraction in 0.1f64..0.9,
        depths in 1usize..4,
        start in (-400.0f64..900.0, -400.0f64..900.0),
    ) {
        let stock = ((x0, y0), (x0 + w, y0 + h));
        let stepover = fraction * 2.0 * radius;
        let pass_z: Vec<f64> = (1..=depths).map(|k| -0.5 * k as f64).collect();
        let (moves, _) = surface_moves(&cut(stock, radius, stepover), &pass_z, start);
        let at = positions(&moves);
        let along_x = w >= h;
        let tol = 1e-6;
        for (k, (m, (x, y), _)) in at.iter().enumerate() {
            if m.kind != MoveKind::Feed {
                continue;
            }
            prop_assert!(x0 - radius - tol <= *x && *x <= x0 + w + radius + tol, "x {x}");
            prop_assert!(y0 - radius - tol <= *y && *y <= y0 + h + radius + tol, "y {y}");
            // Going down: at a pass's end, the tool clear of the stock along the pass.
            if m.x.is_none() && m.y.is_none() {
                let u = if along_x { *x } else { *y };
                let (lo, hi) = if along_x { (x0, x0 + w) } else { (y0, y0 + h) };
                prop_assert!(u <= lo - radius + tol || u >= hi + radius - tol, "down at {u}, move {k}");
            }
        }
        let (lo, hi, r) = if along_x { (y0, y0 + h, radius) } else { (x0, x0 + w, radius) };
        let v = pass_positions(lo, hi, r, stepover);
        prop_assert!(v[0] - r <= lo + tol && v[v.len() - 1] + r >= hi - tol);
        for gap in v.windows(2).map(|p| p[1] - p[0]) {
            prop_assert!(gap <= stepover + tol, "gap {gap} over {stepover}");
        }
    }
}
