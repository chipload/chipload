// SPDX-License-Identifier: GPL-3.0-or-later
//! Prism (docs/specs/prism.md §7): the tip runs along the outline as deep as the largest
//! touching circle over the tangent of the half angle, capped by the cone or a max depth, in
//! passes cut off at their levels; notes and refusals.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::plan::job_problems;
use chipload_toolpath::plan_job;
use proptest::prelude::*;
use serde_json::{Value, json};

const TOL: f64 = 0.02;

/// A project with V-bit 1 (90°, pointed, 12.7 mm: its cone reaches 6.35 mm), V-bit 2 (60°,
/// a 0.4 mm flat), end mill 3, and one prism on layer L with `extra` over: one pass, climb.
fn job(extra: Value) -> Job {
    let mut data = migrate(json!({
        "schema_version": 21,
        "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
        "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                  "max_overcut": 0.5},
        "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
        "tools": [
            {"id": 1, "type": "vbit", "diameter": 12.7, "rpm": 18000,
             "angle": 90f64.to_radians()},
            {"id": 2, "type": "vbit", "diameter": 12.7, "rpm": 18000,
             "angle": 60f64.to_radians(), "tip_diameter": 0.4},
            {"id": 3, "diameter": 6, "rpm": 18000, "center_cutting": true}
        ],
        "operations": [],
    }))
    .unwrap();
    let mut op = json!({"type": "prism", "part": 1, "select": {"layer": "L"}, "tool": 1,
        "start_depth": 0, "max_depth": null, "step_down": null, "direction": "climb",
        "feed": 1500, "plunge_feed": 500});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    data["operations"] = json!([op]);
    parse_job(data, "test.json").unwrap()
}

/// Every feed move's end, as (x, y, depth below the top).
fn cuts(moves: &[Move]) -> Vec<(f64, f64, f64)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    let mut out = Vec::new();
    for m in moves {
        (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        if m.kind != MoveKind::Rapid && z <= 0.0 {
            out.push((x, y, -z));
        }
    }
    out
}

fn plan(job: &Job, contours: &[Contour]) -> chipload_toolpath::Plan {
    plan_job(job, contours).unwrap_or_else(|e| panic!("{e}"))
}

/// The largest circle inside the w × h rectangle at (x0, y0) touching its outline at `p`
/// (on the outline): along its edge, as far as the nearer corner, at most half the other
/// side.
fn rect_radius(p: (f64, f64), x0: f64, y0: f64, w: f64, h: f64) -> f64 {
    let (x, y) = (p.0 - x0, p.1 - y0);
    let on_horizontal = y.abs() < 1e-6 || (y - h).abs() < 1e-6;
    if on_horizontal {
        x.min(w - x).min(h / 2.0).max(0.0)
    } else {
        y.min(h - y).min(w / 2.0).max(0.0)
    }
}

#[test]
fn along_a_rectangle_the_depth_is_the_touching_circle() {
    let contours = vec![common::rect(1, 10.0, 10.0, 22.0, 70.0)];
    let plan = plan(&job(json!({})), &contours);
    let points = cuts(&plan.program.moves);
    assert!(!points.is_empty());
    for &(x, y, d) in &points {
        let want = rect_radius((x, y), 10.0, 10.0, 12.0, 60.0);
        assert!((d - want).abs() < TOL, "at ({x}, {y}): {d}, not {want}");
    }
    // The long sides go down to half the width, the corners stay at the top.
    let deepest = points.iter().map(|p| p.2).fold(0.0, f64::max);
    assert!((deepest - 6.0).abs() < TOL, "{deepest}");
    assert!(
        points
            .iter()
            .any(|&(x, y, d)| x == 10.0 && y == 10.0 && d.abs() < TOL)
    );
}

#[test]
fn a_circle_is_a_cone_its_edge_as_deep_as_its_radius() {
    let contours = vec![common::circle_id(1, (50.0, 50.0), 5.0)];
    let plan = plan(&job(json!({})), &contours);
    for (x, y, d) in cuts(&plan.program.moves) {
        assert!((math::hypot(x - 50.0, y - 50.0) - 5.0).abs() < TOL);
        if d > TOL {
            assert!((d - 5.0).abs() < TOL, "{d}");
        }
    }
}

#[test]
fn a_flat_tip_runs_half_its_width_out_and_the_angle_sets_the_depth() {
    // 60°: tan 30°; a 10 mm wide strip: 5 mm circles, 5 / tan 30° = 8.66 deep, within the
    // cone (12.3 / 2 = 6.15 mm reach).
    let contours = vec![common::rect(1, 0.0, 0.0, 80.0, 10.0)];
    let plan = plan(&job(json!({"tool": 2})), &contours);
    let points = cuts(&plan.program.moves);
    let deepest = points.iter().map(|p| p.2).fold(0.0, f64::max);
    assert!(
        (deepest - 5.0 / math::tan(30f64.to_radians())).abs() < TOL,
        "{deepest}"
    );
    // Along the bottom, the tip's centre is 0.2 mm below the line.
    assert!(
        points
            .iter()
            .filter(|p| p.0 > 20.0 && p.0 < 60.0 && p.1 < 5.0)
            .all(|p| (p.1 + 0.2).abs() < TOL),
        "{points:?}"
    );
}

#[test]
fn wider_than_the_cone_the_top_stays_flat_with_a_note() {
    let contours = vec![common::rect(1, 0.0, 0.0, 60.0, 40.0)];
    let plan = plan(&job(json!({})), &contours);
    let deepest = cuts(&plan.program.moves)
        .iter()
        .map(|p| p.2)
        .fold(0.0, f64::max);
    assert!((deepest - 6.35).abs() < TOL, "{deepest}");
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("its middle stays flat")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn a_max_depth_bevels_without_a_note() {
    let contours = vec![common::rect(1, 0.0, 0.0, 60.0, 40.0)];
    let plan = plan(&job(json!({"max_depth": 2.0})), &contours);
    let deepest = cuts(&plan.program.moves)
        .iter()
        .map(|p| p.2)
        .fold(0.0, f64::max);
    assert!((deepest - 2.0).abs() < TOL, "{deepest}");
    assert!(
        !plan
            .notes
            .iter()
            .any(|n| n.message.english().contains("stays flat")),
        "{:?}",
        plan.notes
    );
    assert!(
        plan.facts
            .iter()
            .any(|n| n.message.english().contains("the deepest edge is 2 mm")),
        "{:?}",
        plan.facts
    );
}

#[test]
fn passes_are_the_ring_cut_off_at_their_levels() {
    let contours = vec![common::rect(1, 10.0, 10.0, 22.0, 70.0)];
    let plan = plan(&job(json!({"step_down": 2.0})), &contours);
    // 6 mm deepest: passes to 2, 4 and 6, round and round, one retract.
    let points = cuts(&plan.program.moves);
    for level in [2.0, 4.0, 6.0] {
        assert!(
            points.iter().any(|p| (p.2 - level).abs() < TOL),
            "no point at {level}"
        );
    }
    let lifts = plan
        .program
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid && m.z == Some(5.0))
        .count();
    assert_eq!(lifts, 2, "safe Z before and after");
}

#[test]
fn a_ring_is_cut_on_both_lines_its_middle_a_ridge() {
    let contours = vec![
        common::circle_id(1, (60.0, 60.0), 20.0),
        common::circle_id(2, (60.0, 60.0), 10.0),
    ];
    let plan = plan(&job(json!({})), &contours);
    let points = cuts(&plan.program.moves);
    for (x, y, d) in &points {
        let r = math::hypot(x - 60.0, y - 60.0);
        assert!(
            (r - 20.0).abs() < TOL || (r - 10.0).abs() < TOL,
            "off both lines: {r}"
        );
        if *d > TOL {
            assert!((d - 5.0).abs() < TOL, "{d}");
        }
    }
}

#[test]
fn shapes_too_close_get_a_note() {
    // Two 10 mm strips 2 mm apart: each 5 mm deep, its trench 5 mm wide.
    let contours = vec![
        common::rect(1, 0.0, 0.0, 60.0, 10.0),
        common::rect(2, 0.0, 12.0, 60.0, 22.0),
    ];
    let plan = plan(&job(json!({})), &contours);
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("trench reaches")),
        "{:?}",
        plan.notes
    );
    let apart = vec![
        common::rect(1, 0.0, 0.0, 60.0, 10.0),
        common::rect(2, 0.0, 30.0, 60.0, 40.0),
    ];
    let plan = plan_job(&job(json!({})), &apart).unwrap();
    assert!(
        !plan
            .notes
            .iter()
            .any(|n| n.message.english().contains("trench reaches")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn only_a_v_bit_with_an_angle() {
    let problems: Vec<String> = job_problems(&job(json!({"tool": 3})))
        .into_iter()
        .map(|p| p.path)
        .collect();
    assert!(
        problems.contains(&"operations.0.tool".to_string()),
        "{problems:?}"
    );
    let base = job(json!({}));
    let mut data = serde_json::to_value(&base).unwrap();
    data["operations"][0]["max_depth"] = json!(0.0);
    let err = parse_job(data, "test.json").unwrap_err().to_string();
    assert!(err.contains("max_depth"), "{err}");
}

proptest! {
    #![proptest_config(common::proptest_config(32))]

    /// Random rectangles: every point of the tip's path is on the outline, as deep as the
    /// largest touching circle allows (or the cone).
    #[test]
    fn the_depth_is_the_touching_circle_everywhere(
        w in 2.0f64..40.0,
        h in 2.0f64..40.0,
        x0 in 0.0f64..50.0,
        y0 in 0.0f64..50.0,
    ) {
        let contours = vec![common::rect(1, x0, y0, x0 + w, y0 + h)];
        let plan = plan(&job(json!({})), &contours);
        for (x, y, d) in cuts(&plan.program.moves) {
            let want = rect_radius((x, y), x0, y0, w, h).min(6.35);
            prop_assert!((d - want).abs() < TOL, "at ({x}, {y}): {d}, not {want}");
        }
    }
}
