// SPDX-License-Identifier: GPL-3.0-or-later
//! Chamfer (docs/specs/chamfer.md): the V-bit's centre runs half its tip into the waste, as
//! deep as the width over the tangent of its half angle, in one pass or equal passes; the
//! tools and widths it can't make are refused.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_core::primitives::signed_area;
use chipload_job::migrations::migrate;
use chipload_job::{Job, Operation, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::plan::job_problems;
use chipload_toolpath::plan_job;
use serde_json::{Value, json};

const TOL: f64 = 0.01;

/// A project with V-bit 1 (`angle_deg`, `tip` mm flat, 12.7 mm wide), end mill 2, and one
/// chamfer with `extra` over its defaults: 2 mm wide outside the shapes on layer L.
fn job(angle_deg: f64, tip: Option<f64>, extra: Value) -> Job {
    let base = migrate(json!({
        "schema_version": 21,
        "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
        "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                  "max_overcut": 0.5},
        "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
        "tools": [
            {"id": 1, "type": "vbit", "diameter": 12.7, "rpm": 18000,
             "angle": angle_deg.to_radians(), "tip_diameter": tip},
            {"id": 2, "diameter": 6, "rpm": 18000, "center_cutting": true}
        ],
        "operations": [{"type": "path", "select": {"layer": "L"}, "tool": 1,
            "start_depth": 0, "end_depth": 1, "step_down": 1, "feed": 1500,
            "plunge_feed": 500, "entry": {"type": "plunge"}}],
    }))
    .unwrap();
    let mut data: Value = base;
    let mut op = json!({"type": "chamfer", "part": 1, "select": {"layer": "L"}, "tool": 1,
        "side": "outside", "width": 2.0, "start_depth": 0, "step_down": null,
        "direction": "climb", "feed": 1500, "plunge_feed": 500, "entry": {"type": "plunge"}});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    data["operations"] = json!([op]);
    parse_job(data, "test.json").unwrap()
}

/// Every cutting move's end and middle along X and Y below the stock top, with its Z.
fn cutting(moves: &[Move]) -> Vec<(Point, f64)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, f64::INFINITY);
    let mut out = Vec::new();
    for m in moves {
        let from = (x, y);
        (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        let travels = m.x.is_some() || m.y.is_some();
        if m.kind != MoveKind::Rapid && travels && z < -TOL {
            out.push(((x, y), z));
            out.push((((from.0 + x) / 2.0, (from.1 + y) / 2.0), z));
        }
    }
    out
}

fn cuts_of(job: &Job, contours: &[Contour]) -> Vec<(Point, f64)> {
    let plan = plan_job(job, contours).unwrap_or_else(|e| panic!("{e}"));
    cutting(&plan.program.moves)
}

fn depths(cuts: &[(Point, f64)]) -> Vec<f64> {
    let mut zs: Vec<f64> = cuts
        .iter()
        .map(|(_, z)| (z * 1000.0).round() / 1000.0)
        .collect();
    zs.sort_by(|a, b| b.total_cmp(a));
    zs.dedup();
    zs
}

/// Distance from `p` to the 60 × 40 rectangle's line, and whether it is outside it.
fn to_rect(p: Point) -> (f64, bool) {
    let dx = (0.0 - p.0).max(p.0 - 60.0);
    let dy = (0.0 - p.1).max(p.1 - 40.0);
    if dx > 0.0 || dy > 0.0 {
        (math::hypot(dx.max(0.0), dy.max(0.0)), true)
    } else {
        (-(dx.max(dy)), false)
    }
}

fn rect() -> Vec<Contour> {
    vec![common::rect(0, 0.0, 0.0, 60.0, 40.0)]
}

#[test]
fn a_pointed_bit_follows_the_line_as_deep_as_the_width_at_90_degrees() {
    let cuts = cuts_of(&job(90.0, None, json!({})), &rect());
    assert_eq!(depths(&cuts), [-2.0]);
    for &(p, _) in &cuts {
        let (d, _) = to_rect(p);
        assert!(d < TOL, "off the line at {p:?}: {d}");
    }
}

#[test]
fn a_flat_tip_runs_half_its_width_outside_and_rounds_the_corners() {
    let cuts = cuts_of(&job(90.0, Some(0.2), json!({})), &rect());
    assert_eq!(depths(&cuts), [-2.0]);
    for &(p, _) in &cuts {
        let (d, outside) = to_rect(p);
        assert!(outside, "inside the part at {p:?}");
        assert!((d - 0.1).abs() < TOL, "not 0.1 from the line at {p:?}: {d}");
    }
    // Round the corner: points beyond both edges, still 0.1 from the corner.
    assert!(
        cuts.iter()
            .any(|&(p, _)| p.0 > 60.0 + TOL && p.1 > 40.0 + TOL),
        "the corner is turned on an arc"
    );
}

#[test]
fn inside_a_hole_with_a_60_degree_bit_goes_deeper() {
    let hole = vec![common::circle_id(0, (30.0, 20.0), 10.0)];
    let job = job(60.0, Some(0.2), json!({"side": "inside", "width": 1.5}));
    let cuts = cuts_of(&job, &hole);
    let depth = 1.5 / math::tan(30f64.to_radians());
    assert_eq!(depths(&cuts), [-((depth * 1000.0).round() / 1000.0)]);
    for &(p, _) in &cuts {
        let r = math::hypot(p.0 - 30.0, p.1 - 20.0);
        assert!(
            (r - 9.9).abs() < TOL,
            "not 0.1 inside the hole at {p:?}: {r}"
        );
    }
}

#[test]
fn a_step_down_gives_equal_passes_the_last_at_the_depth() {
    let even = cuts_of(&job(90.0, None, json!({"step_down": 1.0})), &rect());
    assert_eq!(depths(&even), [-1.0, -2.0]);
    let uneven = cuts_of(&job(90.0, None, json!({"step_down": 0.8})), &rect());
    assert_eq!(depths(&uneven), [-0.667, -1.333, -2.0]);
}

#[test]
fn the_edge_of_a_step_starts_lower() {
    let cuts = cuts_of(&job(90.0, None, json!({"start_depth": 3.0})), &rect());
    assert_eq!(depths(&cuts), [-5.0]);
}

#[test]
fn climb_and_conventional_go_round_opposite_ways() {
    let way = |direction: &str| {
        let cuts = cuts_of(&job(90.0, None, json!({"direction": direction})), &rect());
        let ring: Vec<Point> = cuts.iter().step_by(2).map(|(p, _)| *p).collect();
        signed_area(&ring)
    };
    // Outside, climb, a clockwise spindle: the part on the right, clockwise.
    assert!(way("climb") < 0.0);
    assert!(way("conventional") > 0.0);
}

#[test]
fn the_depth_comes_from_the_tool() {
    let job = job(60.0, None, json!({"width": 2.0}));
    let (start, end) = job.operations[0].depths(&job.tools);
    assert_eq!(start, 0.0);
    assert!((end - 2.0 * 3f64.sqrt()).abs() < 1e-9);
}

#[test]
fn what_it_cant_make_is_refused_at_its_field() {
    let problem = |job: &Job| {
        job_problems(job)
            .into_iter()
            .map(|p| (p.path, p.short))
            .collect::<Vec<_>>()
    };
    // Wider than the cone: (12.7 - 0.2) / 2 = 6.25 at most.
    let wide = problem(&job(90.0, Some(0.2), json!({"width": 7.0})));
    assert!(
        wide.iter()
            .any(|(f, t)| f == "operations.0.width" && t.contains("6.25")),
        "{wide:?}"
    );
    let end_mill = problem(&job(90.0, None, json!({"tool": 2})));
    assert!(
        end_mill
            .iter()
            .any(|(f, t)| f == "operations.0.tool" && t.contains("V-bit")),
        "{end_mill:?}"
    );
    let helix = problem(&job(
        90.0,
        None,
        json!({"entry": {"type": "helix", "radius": 1.0, "ramp_angle": 0.1}}),
    ));
    assert!(
        helix.iter().any(|(f, _)| f == "operations.0.entry"),
        "{helix:?}"
    );
}

#[test]
fn a_width_of_zero_is_refused_when_read() {
    let base = job(90.0, None, json!({}));
    let mut data = serde_json::to_value(&base).unwrap();
    data["operations"][0]["width"] = json!(0.0);
    let err = parse_job(data, "test.json").unwrap_err().to_string();
    assert!(err.contains("width"), "{err}");
}

#[test]
fn open_shapes_are_skipped_with_a_note() {
    let open = common::polygon(&[(0.0, 0.0), (50.0, 0.0), (50.0, 30.0)]);
    let mut open = open;
    open.closed = false;
    let contours = vec![common::rect(0, 0.0, 0.0, 60.0, 40.0), {
        open.id = 1;
        open
    }];
    let plan = plan_job(&job(90.0, None, json!({})), &contours).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.message.english().contains("open")),
        "{:?}",
        plan.notes
    );
    assert!(matches!(
        job(90.0, None, json!({})).operations[0],
        Operation::Chamfer(_)
    ));
}
