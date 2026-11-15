// SPDX-License-Identifier: GPL-3.0-or-later
//! Flute (docs/specs/fluting.md §10): the depth along each open shape ramps in and out, each
//! pass is the profile cut off at its level, back and forth; what it can't cut is refused.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::arcs::merge_arcs;
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::plan::job_problems;
use chipload_toolpath::plan_job;
use proptest::prelude::*;
use serde_json::{Value, json};

const TOL: f64 = 0.01;

/// An open contour through `(x, y, bulge)` vertices.
fn open(id: usize, vertices: &[(f64, f64, f64)]) -> Contour {
    let parts = vertices
        .windows(2)
        .map(|w| {
            let ((x1, y1, b), (x2, y2, _)) = (w[0], w[1]);
            common::segment(match Arc::from_bulge((x1, y1), (x2, y2), b) {
                Some(arc) => Primitive::Arc(arc),
                None => Primitive::Line(Line {
                    start: (x1, y1),
                    end: (x2, y2),
                }),
            })
        })
        .collect();
    Contour {
        id,
        parts,
        closed: false,
        invalid_reason: None,
        nesting_depth: None,
    }
}

/// A project with ball nose 1 (6 mm), V-bit 2 (90°, 12.7 mm), drill 3, end mill 4 not
/// marked centre-cutting, and one flute on layer L with `extra` over: 3 mm deep in one
/// pass, straight 20 mm ramps.
fn job(extra: Value) -> Job {
    let mut data = migrate(json!({
        "schema_version": 21,
        "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
        "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                  "max_overcut": 0.5},
        "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
        "tools": [
            {"id": 1, "type": "ballnose", "diameter": 6, "rpm": 18000},
            {"id": 2, "type": "vbit", "diameter": 12.7, "rpm": 18000,
             "angle": 90f64.to_radians()},
            {"id": 3, "type": "drill", "diameter": 5, "rpm": 3000},
            {"id": 4, "diameter": 6, "rpm": 18000}
        ],
        "operations": [],
    }))
    .unwrap();
    let mut op = json!({"type": "flute", "part": 1, "select": {"layer": "L"}, "tool": 1,
        "start_depth": 0, "end_depth": 3, "step_down": null, "ramp_start": 20,
        "ramp_end": 20, "ramp_shape": "straight", "feed": 1500, "plunge_feed": 500});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    data["operations"] = json!([op]);
    parse_job(data, "test.json").unwrap()
}

/// A 100 mm line along X at y = 10, drawn from x = 10.
fn line() -> Vec<Contour> {
    vec![open(1, &[(10.0, 10.0, 0.0), (110.0, 10.0, 0.0)])]
}

/// The position after each move, from (0, 0, 0), with its kind and feed.
fn trace(moves: &[Move]) -> Vec<(f64, f64, f64, MoveKind, Option<f64>)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    moves
        .iter()
        .map(|m| {
            (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
            (x, y, z, m.kind, m.feed)
        })
        .collect()
}

/// The traversals along the line: each a run of feed moves at or below the top, as
/// (x, z) points, split where the direction along X turns.
fn traversals(moves: &[Move]) -> Vec<Vec<(f64, f64)>> {
    let mut out: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    let mut sense = 0.0;
    for (x, _, z, kind, _) in trace(moves) {
        if kind != MoveKind::Feed {
            continue;
        }
        if let Some(&(lx, _)) = run.last() {
            let step = x - lx;
            if step != 0.0 {
                if sense != 0.0 && step.signum() != sense {
                    let end = *run.last().unwrap();
                    out.push(std::mem::take(&mut run));
                    run.push(end);
                }
                sense = step.signum();
            }
        }
        run.push((x, z));
    }
    if !run.is_empty() {
        out.push(run);
    }
    out
}

/// The straight profile of the 100 mm line: 0 at the ends, `depth` from 20 to 80.
fn straight(s: f64, depth: f64, start: f64, end: f64) -> f64 {
    let a = if start > 0.0 {
        (s / start).min(1.0)
    } else {
        1.0
    };
    let b = if end > 0.0 {
        ((100.0 - s) / end).min(1.0)
    } else {
        1.0
    };
    depth * a.min(b)
}

/// The Z of a traversal at `x`, between its points.
fn z_at(run: &[(f64, f64)], x: f64) -> f64 {
    let mut sorted = run.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    for w in sorted.windows(2) {
        let ((x0, z0), (x1, z1)) = (w[0], w[1]);
        if x >= x0 - 1e-9 && x <= x1 + 1e-9 {
            if x1 - x0 < 1e-12 {
                return z0.min(z1);
            }
            return z0 + (z1 - z0) * (x - x0) / (x1 - x0);
        }
    }
    panic!("{x} outside {run:?}");
}

/// How far `p` is from a line segment, or from an arc's circle.
fn off(prim: &Primitive, p: (f64, f64)) -> f64 {
    match prim {
        Primitive::Arc(a) => (math::hypot(p.0 - a.center.0, p.1 - a.center.1) - a.radius).abs(),
        Primitive::Line(l) => {
            let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
            let t = (((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / (dx * dx + dy * dy))
                .clamp(0.0, 1.0);
            math::hypot(p.0 - l.start.0 - t * dx, p.1 - l.start.1 - t * dy)
        }
    }
}

#[test]
fn one_pass_follows_the_profile_down_along_and_up() {
    let plan = plan_job(&job(json!({})), &line()).unwrap();
    let runs = traversals(&plan.program.moves);
    assert_eq!(runs.len(), 1, "{runs:?}");
    for k in 0..=100 {
        let s = k as f64;
        let z = z_at(&runs[0], 10.0 + s);
        assert!(
            (z + straight(s, 3.0, 20.0, 20.0)).abs() < TOL,
            "at {s}: {z}"
        );
    }
    // The ramps' ends are points of the path.
    for x in [30.0, 90.0] {
        assert!(
            runs[0]
                .iter()
                .any(|p| (p.0 - x).abs() < 1e-9 && (p.1 + 3.0).abs() < 1e-9)
        );
    }
}

#[test]
fn passes_are_the_profile_cut_off_at_their_level_back_and_forth() {
    let plan = plan_job(&job(json!({"end_depth": 4, "step_down": 2})), &line()).unwrap();
    let runs = traversals(&plan.program.moves);
    assert_eq!(runs.len(), 2, "{runs:?}");
    for k in 0..=100 {
        let s = k as f64;
        let first = z_at(&runs[0], 10.0 + s);
        let last = z_at(&runs[1], 10.0 + s);
        let full = straight(s, 4.0, 20.0, 20.0);
        assert!(
            (first + full.min(2.0)).abs() < TOL,
            "pass 1 at {s}: {first}"
        );
        assert!((last + full).abs() < TOL, "pass 2 at {s}: {last}");
        assert!(first - last <= 2.0 + TOL, "more than a step at {s}");
    }
    // The second goes back: it starts where the first ended.
    assert!((runs[0].last().unwrap().0 - 110.0).abs() < 1e-9);
    assert!((runs[1].last().unwrap().0 - 10.0).abs() < 1e-9);
    // No retract between passes: safe Z, the approach height, safe Z.
    let rapid_z: Vec<f64> = plan
        .program
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid)
        .filter_map(|m| m.z)
        .collect();
    assert_eq!(rapid_z.first(), Some(&5.0));
    assert_eq!(rapid_z.last(), Some(&5.0));
    assert!(rapid_z.len() <= 3, "{rapid_z:?}");
}

#[test]
fn a_square_end_goes_straight_down_at_the_plunge_feed() {
    let job = job(json!({"ramp_start": 0, "end_depth": 4, "step_down": 2}));
    let plan = plan_job(&job, &line()).unwrap();
    let at = trace(&plan.program.moves);
    // Straight down at x = 10 to each level, at 500 mm/min.
    let downs: Vec<(f64, f64, Option<f64>)> = at
        .windows(2)
        .filter(|w| w[0].0 == w[1].0 && w[0].1 == w[1].1 && w[1].2 < w[0].2 - 1e-9)
        .filter(|w| w[1].3 == MoveKind::Feed && w[1].2 < 0.0)
        .map(|w| (w[1].0, w[1].2, w[1].4))
        .collect();
    assert!(downs.contains(&(10.0, -2.0, Some(500.0))), "{downs:?}");
    let runs = traversals(&plan.program.moves);
    assert!((z_at(runs.last().unwrap(), 10.0) + 4.0).abs() < TOL);
    assert!(z_at(runs.last().unwrap(), 110.0).abs() < TOL);
}

#[test]
fn ramps_belong_to_the_shapes_drawn_ends_whichever_end_is_cut_first() {
    // Drawn from x = 110 to x = 10; the tool starts near the origin, at the drawn last end.
    let drawn_back = vec![open(1, &[(110.0, 10.0, 0.0), (10.0, 10.0, 0.0)])];
    let job = job(json!({"ramp_start": 0, "ramp_end": 20}));
    let plan = plan_job(&job, &drawn_back).unwrap();
    let runs = traversals(&plan.program.moves);
    // A square end at x = 110 (the first drawn end), a ramp to x = 10.
    assert!((z_at(&runs[0], 110.0) + 3.0).abs() < TOL, "{runs:?}");
    assert!(z_at(&runs[0], 10.0).abs() < TOL);
    assert!((z_at(&runs[0], 20.0) + 1.5).abs() < TOL);
    assert!(
        (runs[0][0].0 - 10.0).abs() < 1e-9,
        "starts at the nearer end"
    );
}

#[test]
fn ramps_longer_than_the_shape_are_shortened_with_a_note() {
    let short = vec![open(1, &[(10.0, 10.0, 0.0), (40.0, 10.0, 0.0)])];
    let plan = plan_job(&job(json!({})), &short).unwrap();
    let runs = traversals(&plan.program.moves);
    // 30 mm long: 15 mm each, full depth at the middle only.
    assert!((z_at(&runs[0], 25.0) + 3.0).abs() < TOL, "{runs:?}");
    assert!((z_at(&runs[0], 17.5) + 1.5).abs() < TOL);
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("shorter than its two ramps")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn curved_ramps_are_within_the_arc_tolerance_of_the_curve() {
    let plan = plan_job(&job(json!({"ramp_shape": "curved"})), &line()).unwrap();
    let runs = traversals(&plan.program.moves);
    for k in 0..=2000 {
        let s = k as f64 * 0.05;
        let u = |r: f64| math::sin(std::f64::consts::FRAC_PI_2 * r.min(1.0));
        let want = 3.0 * u(s / 20.0).min(u((100.0 - s) / 20.0));
        let z = z_at(&runs[0], 10.0 + s);
        assert!((z + want).abs() <= 0.01 + 1e-6, "at {s}: {z} for {want}");
    }
}

#[test]
fn along_an_arc_the_tool_stays_on_the_circle_and_arcs_stay_arcs() {
    // A half circle of radius 30 about (60, 10).
    let arc = vec![open(1, &[(30.0, 10.0, 1.0), (90.0, 10.0, 0.0)])];
    for shape in ["straight", "curved"] {
        let plan = plan_job(&job(json!({"ramp_shape": shape})), &arc).unwrap();
        for (x, y, z, kind, _) in trace(&plan.program.moves) {
            if kind == MoveKind::Feed && z < -TOL {
                let r = math::hypot(x - 60.0, y - 10.0);
                assert!(
                    (r - 30.0).abs() < 0.002,
                    "{shape}: off the circle at ({x}, {y})"
                );
            }
        }
        let merged = merge_arcs(&plan.program).unwrap();
        assert!(
            merged
                .moves
                .iter()
                .any(|m| matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw)),
            "{shape}: no arc moves"
        );
    }
}

#[test]
fn closed_shapes_are_skipped_with_a_note() {
    let contours = vec![common::rect(2, 0.0, 0.0, 60.0, 40.0), line().remove(0)];
    let plan = plan_job(&job(json!({})), &contours).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("closed contour #2")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn the_width_at_the_top_is_a_fact() {
    let plan = plan_job(&job(json!({})), &line()).unwrap();
    // A 6 mm ball 3 mm deep: as wide as the ball.
    assert!(
        plan.facts
            .iter()
            .any(|n| n.message.english().contains("6 mm wide at the top")),
        "{:?}",
        plan.facts
    );
    let plan = plan_job(&job(json!({"tool": 2, "end_depth": 2})), &line()).unwrap();
    assert!(
        plan.facts
            .iter()
            .any(|n| n.message.english().contains("4 mm wide at the top")),
        "{:?}",
        plan.facts
    );
}

#[test]
fn what_it_cant_cut_is_refused() {
    let problem = |job: &Job| {
        job_problems(job)
            .into_iter()
            .map(|p| (p.path, p.short))
            .collect::<Vec<_>>()
    };
    let drill = problem(&job(json!({"tool": 3})));
    assert!(
        drill.iter().any(|(f, _)| f == "operations.0.tool"),
        "{drill:?}"
    );
    // A 90° 12.7 mm V-bit's cone is 6.35 deep.
    let deep = problem(&job(json!({"tool": 2, "end_depth": 7})));
    assert!(
        deep.iter().any(|(f, _)| f == "operations.0.end_depth"),
        "{deep:?}"
    );
    let base = job(json!({}));
    let mut data = serde_json::to_value(&base).unwrap();
    data["operations"][0]["ramp_end"] = json!(-1.0);
    let err = parse_job(data, "test.json").unwrap_err().to_string();
    assert!(err.contains("ramp_end"), "{err}");
}

#[test]
fn an_end_mill_of_unknown_centre_cutting_gets_a_note_only_with_a_square_end() {
    let notes = |job: &Job| plan_job(job, &line()).unwrap().notes;
    let ramped = notes(&job(json!({"tool": 4})));
    assert!(
        !ramped
            .iter()
            .any(|n| n.message.english().contains("centre")),
        "{ramped:?}"
    );
    let square = notes(&job(json!({"tool": 4, "ramp_end": 0})));
    assert!(
        square
            .iter()
            .any(|n| n.message.english().contains("centre")),
        "{square:?}"
    );
}

proptest! {
    #![proptest_config(common::proptest_config(48))]

    /// Random polylines with arcs, ramps and steps: every point on the shape, never more
    /// than a step removed by one pass, never deeper than the depth, the last pass the
    /// profile.
    #[test]
    fn passes_stay_on_the_shape_and_within_a_step(
        bulges in prop::collection::vec(-0.8f64..0.8, 1..4),
        ramp_start in 0.0f64..60.0,
        ramp_end in 0.0f64..60.0,
        depth in 0.5f64..6.0,
        step in 0.7f64..3.0,
        curved in any::<bool>(),
    ) {
        let mut vertices = Vec::new();
        for (k, b) in bulges.iter().enumerate() {
            vertices.push((20.0 + 40.0 * k as f64, 50.0 + 15.0 * (k % 2) as f64, *b));
        }
        vertices.push((20.0 + 40.0 * bulges.len() as f64, 50.0, 0.0));
        let contour = open(1, &vertices);
        let job = job(json!({"end_depth": depth, "step_down": step, "ramp_start": ramp_start,
            "ramp_end": ramp_end, "ramp_shape": if curved { "curved" } else { "straight" }}));
        let plan = plan_job(&job, std::slice::from_ref(&contour)).unwrap();
        let at = trace(&plan.program.moves);
        let near = |x: f64, y: f64| {
            contour.parts.iter().any(|p| off(&p.segment.prim, (x, y)) < 0.002)
        };
        let mut previous = f64::INFINITY;
        for (x, y, z, kind, _) in &at {
            if *kind == MoveKind::Feed && *z < -TOL {
                prop_assert!(near(*x, *y), "off the shape at ({x}, {y})");
                prop_assert!(*z >= -depth - 1e-9, "deeper than the depth: {z}");
            }
            if *kind == MoveKind::Feed && *z < previous - step - 1e-6 && previous.is_finite() {
                // Only a straight-down move can drop: at most one step at a time.
                prop_assert!(false, "dropped {} at once", previous - z);
            }
            if *kind == MoveKind::Feed {
                previous = *z;
            }
        }
        let deepest = at.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        prop_assert!((deepest + depth).abs() < TOL || ramp_start + ramp_end > 0.0);
    }
}
