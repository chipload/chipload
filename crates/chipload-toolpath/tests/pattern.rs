// SPDX-License-Identifier: GPL-3.0-or-later
//! Fill pattern (docs/specs/fill-pattern.md §10): grooves clipped to the shape shrunk by
//! half their width, out of its holes, laid out from the drawing's origin; rings one spacing
//! apart; refusals and notes.
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

const TOL: f64 = 0.01;

/// A project with end mill 1 (2 mm, centre-cutting), V-bit 2 (90°, pointed), drill 3, and
/// one pattern on layer L with `extra` over: lines at 0°, 5 mm apart, 0.5 mm deep with the
/// end mill, inside the edge, square ends.
fn job(extra: Value) -> Job {
    let mut data = migrate(json!({
        "schema_version": 21,
        "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
        "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                  "max_overcut": 0.5},
        "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
        "tools": [
            {"id": 1, "diameter": 2, "rpm": 18000, "center_cutting": true},
            {"id": 2, "type": "vbit", "diameter": 12.7, "rpm": 18000,
             "angle": 90f64.to_radians()},
            {"id": 3, "type": "drill", "diameter": 5, "rpm": 3000}
        ],
        "operations": [],
    }))
    .unwrap();
    let mut op = json!({"type": "pattern", "part": 1, "select": {"layer": "L"}, "tool": 1,
        "pattern": {"kind": "lines", "angle": 0.0, "spacing": 5.0}, "inside": true,
        "start_depth": 0, "end_depth": 0.5, "step_down": null, "ramp": 0,
        "feed": 1500, "plunge_feed": 500});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    data["operations"] = json!([op]);
    parse_job(data, "test.json").unwrap()
}

/// The 60 × 40 rectangle at the origin.
fn rect() -> Vec<Contour> {
    vec![common::rect(1, 0.0, 0.0, 60.0, 40.0)]
}

/// Every cutting move's end below the top, as (x, y, z).
fn cuts(moves: &[Move]) -> Vec<(f64, f64, f64)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    let mut out = Vec::new();
    for m in moves {
        (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        if m.kind != MoveKind::Rapid && z < -TOL {
            out.push((x, y, z));
        }
    }
    out
}

/// The grooves: runs of cutting moves between retracts, as their (x, y) points.
fn grooves(moves: &[Move]) -> Vec<Vec<(f64, f64)>> {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    let mut out: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    for m in moves {
        (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        if m.kind == MoveKind::Rapid && z > 0.0 {
            if !run.is_empty() {
                out.push(std::mem::take(&mut run));
            }
        } else if z < -TOL {
            run.push((x, y));
        }
    }
    if !run.is_empty() {
        out.push(run);
    }
    out
}

fn plan(job: &Job, contours: &[Contour]) -> chipload_toolpath::Plan {
    plan_job(job, contours).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn lines_fill_a_rectangle_inside_its_edge() {
    let plan = plan(&job(json!({})), &rect());
    let found = grooves(&plan.program.moves);
    // y = 5 … 35: y = 0 and 40 are within a groove's half width of the edge.
    let mut ys: Vec<i64> = found.iter().map(|g| g[0].1.round() as i64).collect();
    ys.sort_unstable();
    ys.dedup();
    assert_eq!(ys, [5, 10, 15, 20, 25, 30, 35]);
    for g in &found {
        let xs: Vec<f64> = g.iter().map(|p| p.0).collect();
        let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!((lo - 1.0).abs() < TOL && (hi - 59.0).abs() < TOL, "{g:?}");
    }
}

#[test]
fn on_the_line_the_grooves_reach_the_edge() {
    let plan = plan(&job(json!({"inside": false})), &rect());
    let found = grooves(&plan.program.moves);
    // y = 0 and 40 lie on the edge: no groove inside there.
    assert_eq!(found.len(), 7);
    let reach = cuts(&plan.program.moves)
        .iter()
        .any(|p| p.0.abs() < TOL || (p.0 - 60.0).abs() < TOL);
    assert!(reach, "a groove reaches the line");
}

#[test]
fn grooves_stay_out_of_a_selected_hole() {
    let contours = vec![
        common::rect(1, 0.0, 0.0, 60.0, 40.0),
        common::circle_id(2, (30.0, 20.0), 10.0),
    ];
    let plan = plan(
        &job(json!({"pattern": {"kind": "cross", "angle": 0.3,
        "spacing": 2.5, "between": 1.2}})),
        &contours,
    );
    let points = cuts(&plan.program.moves);
    assert!(!points.is_empty());
    for (x, y, _) in points {
        let d = math::hypot(x - 30.0, y - 20.0);
        assert!(d >= 11.0 - TOL, "in the hole at ({x}, {y}): {d}");
        assert!((1.0 - TOL..=59.0 + TOL).contains(&x) && (1.0 - TOL..=39.0 + TOL).contains(&y));
    }
}

#[test]
fn lines_line_up_across_two_shapes() {
    let angle: f64 = 30f64.to_radians();
    let contours = vec![
        common::rect(1, 0.0, 0.0, 40.0, 40.0),
        common::rect(2, 47.0, 3.0, 90.0, 41.0),
    ];
    let plan = plan(
        &job(json!({"pattern": {"kind": "lines", "angle": angle, "spacing": 4.0}})),
        &contours,
    );
    let (sin, cos) = math::sin_cos(angle);
    for g in grooves(&plan.program.moves) {
        for (x, y) in g {
            let v = -x * sin + y * cos;
            let off = (v / 4.0 - (v / 4.0).round()).abs() * 4.0;
            assert!(off < TOL, "({x}, {y}) is {off} off the lines");
        }
    }
}

#[test]
fn a_cross_hatch_cuts_both_directions() {
    let between: f64 = 60f64.to_radians();
    let plan = plan(
        &job(json!({"pattern": {"kind": "cross", "angle": 0.0,
        "spacing": 5.0, "between": between}})),
        &rect(),
    );
    let mut directions: Vec<i64> = grooves(&plan.program.moves)
        .iter()
        .map(|g| {
            let (a, b) = (g[0], g[g.len() - 1]);
            let d = math::atan2(b.1 - a.1, b.0 - a.0)
                .to_degrees()
                .rem_euclid(180.0);
            d.round() as i64
        })
        .collect();
    directions.sort_unstable();
    directions.dedup();
    assert_eq!(directions, [0, 60]);
}

#[test]
fn rings_are_offsets_one_spacing_apart() {
    let contours = vec![common::circle_id(1, (50.0, 50.0), 20.0)];
    let plan = plan(
        &job(json!({"pattern": {"kind": "rings", "spacing": 3.0}})),
        &contours,
    );
    let mut radii: Vec<i64> = cuts(&plan.program.moves)
        .iter()
        .map(|p| (math::hypot(p.0 - 50.0, p.1 - 50.0) * 10.0).round() as i64)
        .collect();
    radii.sort_unstable();
    radii.dedup();
    assert_eq!(radii, [20, 50, 80, 110, 140, 170]);
}

#[test]
fn ramped_ends_go_down_from_the_top_over_the_ramp() {
    let plan = plan(&job(json!({"ramp": 3.0, "end_depth": 1.0})), &rect());
    let (mut x, mut z) = (0.0, 0.0);
    let mut at_start = Vec::new();
    for m in &plan.program.moves {
        let before = z;
        (x, z) = (m.x.unwrap_or(x), m.z.unwrap_or(z));
        if m.kind == MoveKind::Feed && m.x.is_some() && before.abs() < TOL {
            at_start.push((x, z));
        }
    }
    // Every groove leaves its end (x = 1 or 59) at the top and is at full depth 3 mm in.
    assert_eq!(at_start.len(), 7);
    assert!(
        at_start
            .iter()
            .all(|&(x, z)| ((x - 4.0).abs() < TOL || (x - 56.0).abs() < TOL)
                && (z + 1.0).abs() < TOL),
        "{at_start:?}"
    );
}

#[test]
fn notes_and_refusals() {
    // Narrower than a groove: overlapping, a note.
    let plan = plan(
        &job(json!({"pattern": {"kind": "lines", "angle": 0.0, "spacing": 1.5}})),
        &rect(),
    );
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("the grooves overlap")),
        "{:?}",
        plan.notes
    );
    // A shape narrower than a groove inside its edge: nothing there, a note.
    let thin = vec![common::rect(1, 0.0, 0.0, 60.0, 1.5)];
    let plan = plan_job(&job(json!({})), &thin).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("too narrow for the pattern")),
        "{:?}",
        plan.notes
    );
    // Too many grooves.
    let err = plan_job(
        &job(json!({"pattern": {"kind": "lines", "angle": 0.0, "spacing": 0.0005}})),
        &rect(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("more than 50000 grooves"), "{err}");
    // A drill, a V-bit past its cone, a cross-hatch past 90°.
    let problem = |job: &Job| {
        job_problems(job)
            .into_iter()
            .map(|p| p.path)
            .collect::<Vec<_>>()
    };
    assert!(problem(&job(json!({"tool": 3}))).contains(&"operations.0.tool".to_string()));
    assert!(
        problem(&job(json!({"tool": 2, "end_depth": 7})))
            .contains(&"operations.0.end_depth".to_string())
    );
    let base = job(json!({}));
    let mut data = serde_json::to_value(&base).unwrap();
    data["operations"][0]["pattern"] =
        json!({"kind": "cross", "angle": 0.0, "spacing": 3.0, "between": 2.0});
    let err = parse_job(data, "test.json").unwrap_err().to_string();
    assert!(err.contains("pattern.between"), "{err}");
}

#[test]
fn the_groove_width_is_a_fact() {
    let plan = plan(&job(json!({"tool": 2, "end_depth": 1.0})), &rect());
    assert!(
        plan.facts
            .iter()
            .any(|n| n.message.english().contains("2 mm wide at the top")),
        "{:?}",
        plan.facts
    );
}

proptest! {
    #![proptest_config(common::proptest_config(32))]

    /// Random rectangles with a round hole, angles, spacings and patterns: every cutting
    /// point inside the rectangle shrunk by half a groove, outside the hole grown by it.
    #[test]
    fn every_groove_stays_inside(
        w in 20.0f64..120.0,
        h in 20.0f64..80.0,
        hole in 2.0f64..8.0,
        angle in -3.2f64..3.2,
        spacing in 1.0f64..8.0,
        kind in 0usize..4,
    ) {
        let contours = vec![
            common::rect(1, 10.0, 10.0, 10.0 + w, 10.0 + h),
            common::circle_id(2, (10.0 + w / 2.0, 10.0 + h / 2.0), hole),
        ];
        let pattern = match kind {
            0 => json!({"kind": "lines", "angle": angle, "spacing": spacing}),
            1 => json!({"kind": "cross", "angle": angle, "spacing": spacing, "between": 1.0}),
            2 => json!({"kind": "rings", "spacing": spacing}),
            _ => json!({"kind": "waves", "angle": angle, "spacing": spacing,
                        "wavelength": 4.0 * spacing, "height": spacing}),
        };
        let plan = plan(&job(json!({"pattern": pattern})), &contours);
        for (x, y, _) in cuts(&plan.program.moves) {
            prop_assert!(x >= 11.0 - TOL && x <= 9.0 + w + TOL, "x {x}");
            prop_assert!(y >= 11.0 - TOL && y <= 9.0 + h + TOL, "y {y}");
            let d = math::hypot(x - 10.0 - w / 2.0, y - 10.0 - h / 2.0);
            prop_assert!(d >= hole + 1.0 - TOL, "in the hole at ({x}, {y})");
        }
    }
}

/// The length of a groove's path.
fn length_of(g: &[(f64, f64)]) -> f64 {
    g.windows(2)
        .map(|w| math::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1))
        .sum()
}

#[test]
fn whole_dashes_are_all_their_length_and_staggered() {
    // docs/specs/fill-pattern.md §14.
    let plan = plan(
        &job(
            json!({"pattern": {"kind": "dashes", "angle": 0.0, "spacing": 5.0,
            "length": 6.0, "gap": 4.0, "whole": true}}),
        ),
        &rect(),
    );
    let found = grooves(&plan.program.moves);
    assert!(!found.is_empty());
    for g in &found {
        assert!((length_of(g) - 6.0).abs() < TOL, "{g:?}");
        for &(x, y) in g {
            assert!((1.0 - TOL..=59.0 + TOL).contains(&x) && (1.0 - TOL..=39.0 + TOL).contains(&y));
        }
    }
    // Rows 5 apart; at y = 5 dashes start at multiples of 10 + 5, at y = 10 of 10.
    let starts = |y: f64| -> Vec<f64> {
        let mut xs: Vec<f64> = found
            .iter()
            .filter(|g| (g[0].1 - y).abs() < TOL)
            .map(|g| g[0].0.min(g[g.len() - 1].0))
            .collect();
        xs.sort_by(f64::total_cmp);
        xs
    };
    assert!(
        starts(5.0)
            .iter()
            .all(|x| ((x - 5.0) / 10.0).fract().abs() < 1e-6)
    );
    assert!(starts(10.0).iter().all(|x| (x / 10.0).fract().abs() < 1e-6));
}

#[test]
fn clipped_strokes_reach_the_edge() {
    let plan = plan(
        &job(
            json!({"pattern": {"kind": "dashes", "angle": 0.0, "spacing": 5.0,
            "length": 7.0, "gap": 4.0, "whole": false}}),
        ),
        &rect(),
    );
    let found = grooves(&plan.program.moves);
    assert!(found.iter().any(|g| length_of(g) < 7.0 - TOL));
    assert!(
        found
            .iter()
            .flatten()
            .any(|p| (p.0 - 1.0).abs() < TOL || (p.0 - 59.0).abs() < TOL)
    );
}

#[test]
fn herringbone_strokes_lean_both_ways_at_45_degrees() {
    let plan = plan(
        &job(
            json!({"pattern": {"kind": "herringbone", "angle": 0.0, "spacing": 3.0,
            "length": 7.0, "whole": true}}),
        ),
        &rect(),
    );
    let mut leans: Vec<i64> = grooves(&plan.program.moves)
        .iter()
        .map(|g| {
            let (a, b) = (g[0], g[g.len() - 1]);
            let d = math::atan2(b.1 - a.1, b.0 - a.0)
                .to_degrees()
                .rem_euclid(180.0);
            d.round() as i64
        })
        .collect();
    leans.sort_unstable();
    leans.dedup();
    assert_eq!(leans, [45, 135]);
}

#[test]
fn scales_are_half_circles_of_the_radius() {
    let plan = plan(
        &job(
            json!({"pattern": {"kind": "scales", "angle": 0.0, "spacing": 4.0,
            "radius": 5.0, "whole": true}}),
        ),
        &rect(),
    );
    let found = grooves(&plan.program.moves);
    assert!(!found.is_empty());
    for g in &found {
        let (a, b) = (g[0], g[g.len() - 1]);
        assert!(
            (math::hypot(b.0 - a.0, b.1 - a.1) - 10.0).abs() < TOL,
            "{g:?}"
        );
        let c = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        for p in g {
            assert!(
                (math::hypot(p.0 - c.0, p.1 - c.1) - 5.0).abs() < 0.02,
                "{p:?}"
            );
        }
    }
}

#[test]
fn a_sunburst_points_out_from_the_middle() {
    let contours = vec![common::circle_id(1, (50.0, 50.0), 30.0)];
    let plan = plan(
        &job(
            json!({"pattern": {"kind": "sunburst", "spacing": 7.0, "length": 5.0,
            "pitch": 5.0, "whole": true}}),
        ),
        &contours,
    );
    let found = grooves(&plan.program.moves);
    assert!(found.len() > 20);
    for g in &found {
        let (a, b) = (g[0], g[g.len() - 1]);
        let ra = math::hypot(a.0 - 50.0, a.1 - 50.0);
        let rb = math::hypot(b.0 - 50.0, b.1 - 50.0);
        assert!(((ra - rb).abs() - 5.0).abs() < TOL, "radial, 5 long: {g:?}");
        let cross = (a.0 - 50.0) * (b.1 - 50.0) - (a.1 - 50.0) * (b.0 - 50.0);
        assert!(cross.abs() < 1e-6 * ra.max(rb).powi(2) + TOL, "{g:?}");
    }
}

proptest! {
    #![proptest_config(common::proptest_config(24))]

    /// Random rectangles with a round hole and stroke patterns, whole strokes: every
    /// cutting point inside the rectangle shrunk by half a groove, outside the hole grown
    /// by it, and every dash its whole length.
    #[test]
    fn every_whole_stroke_stays_inside(
        w in 20.0f64..100.0,
        h in 20.0f64..80.0,
        hole in 2.0f64..8.0,
        angle in -3.2f64..3.2,
        spacing in 2.0f64..8.0,
        length in 3.0f64..12.0,
        kind in 0usize..4,
    ) {
        let contours = vec![
            common::rect(1, 10.0, 10.0, 10.0 + w, 10.0 + h),
            common::circle_id(2, (10.0 + w / 2.0, 10.0 + h / 2.0), hole),
        ];
        let pattern = match kind {
            0 => json!({"kind": "dashes", "angle": angle, "spacing": spacing,
                        "length": length, "gap": length / 2.0, "whole": true}),
            1 => json!({"kind": "herringbone", "angle": angle, "spacing": spacing,
                        "length": length, "whole": true}),
            2 => json!({"kind": "scales", "angle": angle, "spacing": spacing,
                        "radius": length / 2.0, "whole": true}),
            _ => json!({"kind": "sunburst", "spacing": spacing + length, "length": length,
                        "pitch": length, "whole": true}),
        };
        // Square ends for dashes, so their full length is below the top to measure.
        let ramp = if kind == 0 { 0.0 } else { 1.0 };
        let plan = plan(&job(json!({"pattern": pattern, "ramp": ramp})), &contours);
        for (x, y, _) in cuts(&plan.program.moves) {
            prop_assert!(x >= 11.0 - TOL && x <= 9.0 + w + TOL, "x {x}");
            prop_assert!(y >= 11.0 - TOL && y <= 9.0 + h + TOL, "y {y}");
            let d = math::hypot(x - 10.0 - w / 2.0, y - 10.0 - h / 2.0);
            prop_assert!(d >= hole + 1.0 - TOL, "in the hole at ({x}, {y})");
        }
        if kind == 0 {
            for g in grooves(&plan.program.moves) {
                prop_assert!((length_of(&g) - length).abs() < TOL, "{g:?}");
            }
        }
    }
}
