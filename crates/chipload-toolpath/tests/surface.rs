// SPDX-License-Identifier: GPL-3.0-or-later
//! Surfacing (docs/specs/surfacing.md): passes that cover the stock, stay within it grown by
//! the tool radius, and go down only beside it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::Point;
use chipload_core::tolerances::CHAIN_TOL_MM;
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
        // Along the longer side; a square, within the chaining tolerance, along X (as
        // surface_moves: CI found a stock 0.00012 mm taller than wide, run along X).
        let along_x = w >= h - CHAIN_TOL_MM;
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

// -- A part surface (docs/specs/surface-levels.md §3)

/// A part surface of the shapes on layer L: 6 mm endmill, 1 mm off, in `steps` passes.
fn part_surface_job(margin: f64, stepover: f64, step_down: f64) -> chipload_job::Job {
    use serde_json::json;
    chipload_job::parse_job(
        chipload_job::migrations::migrate(json!({
            "schema_version": 21,
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 1000, "y": 1000, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": [{"type": "pocket", "select": {"layer": "L"}, "tool": 1,
                "start_depth": 0, "direction": "climb", "end_depth": 1, "step_down": step_down,
                "stepover": stepover, "feed": 1500, "plunge_feed": 500, "clearance": 0}],
        }))
        .map(|mut data| {
            // The pocket made a part surface: the same shapes, its stepover and depths.
            let op = &mut data["operations"][0];
            *op = json!({"type": "surface", "part": op["part"].clone(), "tool": 1,
                "select": {"layer": "L", "shapes": null}, "margin": margin,
                "start_depth": 0, "end_depth": 1, "step_down": step_down,
                "stepover": stepover, "feed": 1500, "plunge_feed": 500});
            data
        })
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

proptest! {
    #![proptest_config(common::proptest_config(32))]

    /// A part surface of a rectangle (docs/specs/surface-levels.md §8): the whole
    /// rectangle grown by the margin is faced (every point within the tool's reach of a
    /// feed move at the end depth), the tool's edge never goes past the grown rectangle, and
    /// it never plunges (every move down also moves across).
    #[test]
    fn a_part_surface_faces_its_grown_area_and_nothing_past_it(
        w in 20.0f64..120.0,
        h in 20.0f64..80.0,
        margin in 0.0f64..6.0,
        stepover in 0.1f64..=0.5,
        two_steps in any::<bool>(),
    ) {
        let (x0, y0) = (100.0, 100.0);
        let (x1, y1) = (x0 + w, y0 + h);
        let job = part_surface_job(margin, stepover, if two_steps { 0.5 } else { 1.0 });
        let contours = vec![common::rect(1, x0, y0, x1, y1)];
        let plan = chipload_toolpath::plan_job(&job, &contours).unwrap();
        prop_assert!(
            !plan.notes.iter().any(|n| n.message.english().contains("uncut material")),
            "{:?}",
            plan.notes
        );
        let r = 3.0;
        let tol = 1e-3;
        let mut feeds: Vec<(Point, Point)> = Vec::new();
        let mut at = (0.0, 0.0, 5.0);
        // The deepest the tool has fed down to so far: the stock top at first.
        let mut deepest = 0.0_f64;
        for m in &plan.program.moves {
            let next = (m.x.unwrap_or(at.0), m.y.unwrap_or(at.1), m.z.unwrap_or(at.2));
            if m.kind == MoveKind::Feed {
                // The tool's edge within the grown rectangle (its corners rounded by the
                // margin lie inside the box too).
                prop_assert!(
                    next.0 - r >= x0 - margin - tol && next.0 + r <= x1 + margin + tol
                        && next.1 - r >= y0 - margin - tol && next.1 + r <= y1 + margin + tol,
                    "{:?} past the area", next
                );
                // Straight down only onto a floor already cleared, never into material.
                if next.2 < at.2 - 1e-9 && next.2 < deepest - 1e-9 {
                    prop_assert!(next.0 != at.0 || next.1 != at.1, "a plunge to {:?}", next);
                }
                deepest = deepest.min(next.2);
                if (next.2 + 1.0).abs() < 1e-9 && (at.2 + 1.0).abs() < 1e-9 {
                    feeds.push(((at.0, at.1), (next.0, next.1)));
                }
            }
            at = next;
        }
        // Every point of the rectangle (and of the margin along the sides' middles) is
        // within the tool's radius of a feed move at the end depth.
        let reach = |p: Point| {
            feeds.iter().any(|&(a, b)| {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len = dx * dx + dy * dy;
                let t = if len > 0.0 {
                    (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (qx, qy) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
                (qx * qx + qy * qy).sqrt() <= r + 0.02
            })
        };
        // A round tool leaves the corners of the grown area: with a margin under its radius,
        // the rectangle's own corners (within r − margin of two sides) can't all be reached,
        // as in any pocket. That is why the margin starts at the radius.
        let corner = (r - margin).max(0.0);
        let in_corner = |p: Point| {
            let near_x = p.0 - x0 < corner || x1 - p.0 < corner;
            let near_y = p.1 - y0 < corner || y1 - p.1 < corner;
            near_x && near_y
        };
        let n = 12;
        for i in 0..=n {
            for j in 0..=n {
                let p = (x0 + w * f64::from(i) / f64::from(n), y0 + h * f64::from(j) / f64::from(n));
                prop_assert!(in_corner(p) || reach(p), "{:?} not faced", p);
            }
        }
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        for p in [(cx, y0 - margin), (cx, y1 + margin), (x0 - margin, cy), (x1 + margin, cy)] {
            prop_assert!(reach(p), "{:?} at the margin not faced", p);
        }
    }
}

/// Above half the tool, a part surface's rings can leave material in the corners and the
/// middle, as a pocket's: said with the pocket's warning.
#[test]
fn a_part_surface_with_a_wide_stepover_says_it_leaves_material() {
    let job = part_surface_job(3.0, 0.9, 1.0);
    let plan =
        chipload_toolpath::plan_job(&job, &[common::rect(1, 100.0, 100.0, 190.0, 160.0)]).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("uncut material")),
        "{:?}",
        plan.notes
    );
}
