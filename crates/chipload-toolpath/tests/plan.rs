// SPDX-License-Identifier: GPL-3.0-or-later
//! Job-level rules of plan_job.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::chain::Contour;
use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Job, parse_job};
use chipload_toolpath::left::LeftKind;
use chipload_toolpath::{ToolpathError, plan_job};
use serde_json::{Value, json};

fn job(operations: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            // plan_job gets its contours directly; the drawing isn't used here.
            "drawing": {"shapes": [], "next_shape_id": 1, "source": null},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": operations,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn profile(side: &str, tabs: Value) -> Value {
    json!([{"type": "profile", "select": {"layer": "L"}, "side": side, "direction": "climb",
            "tool": 1, "start_depth": 0, "end_depth": 12, "clearance": 0, "step_down": 6, "feed": 1500, "plunge_feed": 500,
            "tabs": tabs}])
}

fn part() -> Vec<Contour> {
    vec![common::polygon(&[
        (10.0, 10.0),
        (110.0, 10.0),
        (110.0, 70.0),
        (10.0, 70.0),
    ])]
}

#[test]
fn tabs_lift_on_the_last_pass_and_are_reported_for_display() {
    let tabs = json!({"count": 4, "width": 8, "height": 3});
    let plan = plan_job(&job(profile("outside", tabs)), &part()).unwrap();
    assert_eq!(plan.tabs.len(), 4);
    let lifts = plan
        .program
        .moves
        .iter()
        .filter(|m| m.z == Some(-9.0))
        .count();
    assert_eq!(lifts, 4);
    // Only the reminder that a straight plunge needs a centre-cutting tool.
    assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
    assert!(plan.notes[0].contains("plunges straight down"));
}

#[test]
fn tab_as_high_as_the_cut_is_an_error() {
    let tabs = json!({"count": 4, "width": 8, "height": 12});
    let err = plan_job(&job(profile("outside", tabs)), &part()).unwrap_err();
    assert!(
        matches!(&err, ToolpathError::Job(m) if m.contains("tab height 12")),
        "{err}"
    );
}

#[test]
fn tabs_on_an_inside_profile_are_allowed_with_a_warning() {
    let tabs = json!({"count": 2, "width": 8, "height": 3});
    let plan = plan_job(&job(profile("inside", tabs)), &part()).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("hold the cut-out slug"))
    );
}

#[test]
fn too_many_tabs_names_the_contour() {
    let tabs = json!({"count": 40, "width": 8, "height": 3});
    let err = plan_job(&job(profile("outside", tabs)), &part()).unwrap_err();
    assert!(
        matches!(&err, ToolpathError::Geometry(m) if m.starts_with("Operation 1, contour #1:")),
        "{err}"
    );
}

// -- material left (docs/specs/material-left.md)

fn unreachable(side: &str, contours: &[Contour]) -> Vec<chipload_toolpath::left::MaterialLeft> {
    let plan = plan_job(&job(profile(side, Value::Null)), contours).unwrap();
    assert!(
        plan.material_left
            .iter()
            .all(|m| m.operation == 0 && m.kind == LeftKind::Unreachable)
    );
    plan.material_left
}

#[test]
fn inside_profile_leaves_the_corners_of_the_hole() {
    let left = unreachable("inside", &part());
    assert_eq!(left.len(), 4);
    // Ordered by (min y, min x): bottom left first.
    assert_eq!(left[0].bounds().0, (10.0, 10.0));
}

#[test]
fn outside_profile_of_a_convex_part_and_a_cut_on_the_line_leave_nothing() {
    assert!(unreachable("outside", &part()).is_empty());
    assert!(unreachable("on", &part()).is_empty());
}

#[test]
fn outside_profile_leaves_the_inside_corner_of_an_l() {
    let l = vec![common::polygon(&[
        (0.0, 0.0),
        (50.0, 0.0),
        (50.0, 20.0),
        (20.0, 20.0),
        (20.0, 40.0),
        (0.0, 40.0),
    ])];
    let left = unreachable("outside", &l);
    assert_eq!(left.len(), 1);
    let (x, y) = left[0].bounds().0;
    assert!(
        (x - 20.0).abs() < 1e-9 && (y - 20.0).abs() < 1e-9,
        "{x}, {y}"
    );
}

#[test]
fn tabs_are_not_material_left() {
    let tabs = json!({"count": 4, "width": 8, "height": 3});
    let plan = plan_job(&job(profile("outside", tabs)), &part()).unwrap();
    assert!(plan.material_left.is_empty());
}

#[test]
fn each_operation_owns_a_run_of_moves_in_cut_order() {
    // Three operations, the middle one disabled: the spans keep the list's numbering.
    let mut ops = profile("inside", Value::Null);
    let mut disabled = ops[0].clone();
    disabled["enabled"] = json!(false);
    let mut outside = ops[0].clone();
    outside["side"] = json!("outside");
    ops = json!([ops[0], disabled, outside]);
    let plan = plan_job(&job(ops), &part()).unwrap();
    let ops: Vec<usize> = plan.op_spans.iter().map(|s| s.operation).collect();
    assert_eq!(ops, [0, 2]);
    assert_eq!(plan.op_spans[0].moves.start, 0);
    assert_eq!(plan.op_spans[0].moves.end, plan.op_spans[1].moves.start);
    assert_eq!(plan.op_spans[1].moves.end, plan.program.moves.len());
}

// -- depths and clearance (docs/specs/depths-and-clearance.md)

fn profile_with(side: &str, changes: Value) -> Value {
    let mut ops = profile(side, Value::Null);
    for (k, v) in changes.as_object().unwrap() {
        ops[0][k] = v.clone();
    }
    ops
}

/// (min x, min y, max x, max y) of the XY positions the program visits.
fn xy_extent(plan: &chipload_toolpath::Plan) -> (f64, f64, f64, f64) {
    let (mut x, mut y) = (0.0, 0.0);
    let mut out = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for m in &plan.program.moves {
        x = m.x.unwrap_or(x);
        y = m.y.unwrap_or(y);
        if m.x.is_some() || m.y.is_some() {
            out = (out.0.min(x), out.1.min(y), out.2.max(x), out.3.max(y));
        }
    }
    out
}

#[test]
fn clearance_moves_a_profile_into_the_part() {
    // The part is 10..110 x 10..70; tool radius 3.
    let outside = |c: f64| {
        xy_extent(
            &plan_job(
                &job(profile_with("outside", json!({"clearance": c}))),
                &part(),
            )
            .unwrap(),
        )
    };
    assert_eq!(outside(0.0), (7.0, 7.0, 113.0, 73.0));
    assert_eq!(outside(0.5), (7.5, 7.5, 112.5, 72.5)); // smaller part
    assert_eq!(outside(-0.5), (6.5, 6.5, 113.5, 73.5)); // stock left on it
    let inside = |c: f64| {
        xy_extent(
            &plan_job(
                &job(profile_with("inside", json!({"clearance": c}))),
                &part(),
            )
            .unwrap(),
        )
    };
    assert_eq!(inside(0.5), (12.5, 12.5, 107.5, 67.5)); // larger hole
    let err = plan_job(
        &job(profile_with("outside", json!({"clearance": 3}))),
        &part(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("less than the tool radius 3"),
        "{err}"
    );
}

#[test]
fn passes_run_between_start_and_end_depth() {
    let ops = profile_with(
        "outside",
        json!({"start_depth": 4, "end_depth": 12, "step_down": 4}),
    );
    let plan = plan_job(&job(ops), &part()).unwrap();
    let mut cut_z: Vec<f64> = plan
        .program
        .moves
        .iter()
        .filter_map(|m| m.z)
        .filter(|&z| z < 0.0)
        .collect();
    cut_z.dedup();
    assert_eq!(cut_z, [-8.0, -12.0]);
}

#[test]
fn tabs_must_be_lower_than_the_cut_between_the_depths() {
    let tabs = json!({"count": 4, "width": 8, "height": 3});
    let cut = |start: f64| {
        plan_job(
            &job(profile_with(
                "outside",
                json!({"start_depth": start, "tabs": tabs.clone()}),
            )),
            &part(),
        )
    };
    assert!(cut(8.0).is_ok()); // 4 mm cut, 3 mm tabs
    let err = cut(9.0).unwrap_err(); // 3 mm cut
    assert!(
        err.to_string()
            .contains("tab height 3 must be less than the cut depth 3"),
        "{err}"
    );
}

#[test]
fn cuts_below_the_stock_warn_within_the_overcut_and_fail_beyond() {
    // Stock 18 mm thick, max_overcut 0.5.
    let plan = plan_job(
        &job(profile_with("outside", json!({"end_depth": 18.3}))),
        &part(),
    )
    .unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("cuts 0.300 mm below the stock")),
        "{:?}",
        plan.notes
    );
    let err = plan_job(
        &job(profile_with("outside", json!({"end_depth": 18.6}))),
        &part(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("more than the allowed 0.5 mm"),
        "{err}"
    );
    let flush = plan_job(
        &job(profile_with("outside", json!({"end_depth": 18}))),
        &part(),
    )
    .unwrap();
    assert!(!flush.notes.iter().any(|n| n.contains("below the stock")));
}

#[test]
fn stock_left_on_purpose_is_not_shown_as_unreachable() {
    // Found with the v7 corpus sample: a negative clearance leaves a band of stock on the
    // part, and material left must be judged against the cleared outline, not the drawn one.
    let left = |side: &str, c: f64| {
        plan_job(&job(profile_with(side, json!({"clearance": c}))), &part())
            .unwrap()
            .material_left
    };
    assert!(left("outside", -0.5).is_empty()); // a square part: no inside corners
    assert!(left("outside", 0.5).is_empty());
    // A square hole: its four corners, whatever the clearance, never one band.
    for c in [-0.5, 0.0, 0.5] {
        assert_eq!(left("inside", c).len(), 4, "clearance {c}");
    }
}

// -- ramp entry (docs/specs/ramp-entry.md)

const THREE_DEG: f64 = 3.0 * std::f64::consts::PI / 180.0;

fn not_centre_cutting(operations: Value) -> Job {
    let mut job = job(operations);
    job.tools[0].center_cutting = Some(false);
    job
}

/// Straight descents only reach depths already cut (or the start depth); every move that
/// goes down sideways is no steeper than the ramp angle. Returns how many ramp moves.
fn assert_ramps_only(moves: &[chipload_toolpath::model::Move], top: f64, angle: f64) -> usize {
    use chipload_toolpath::model::MoveKind;
    let (mut x, mut y, mut z) = (0.0, 0.0, f64::INFINITY);
    let mut deepest = top;
    let mut ramps = 0;
    for m in moves {
        let (nx, ny, nz) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        let run = (nx - x).hypot(ny - y);
        if m.kind != MoveKind::Rapid && nz < z - 1e-9 {
            if run == 0.0 {
                assert!(
                    nz >= deepest - 1e-9,
                    "straight plunge to {nz} below {deepest}"
                );
            } else {
                assert!((z - nz) / run <= angle.tan() + 1e-9, "ramp too steep");
                ramps += 1;
            }
        }
        deepest = deepest.min(nz);
        (x, y, z) = (nx, ny, nz);
    }
    ramps
}

#[test]
fn a_profile_with_a_tool_that_cant_plunge_is_refused_unless_it_ramps() {
    let plunge = not_centre_cutting(profile("outside", Value::Null));
    let err = plan_job(&plunge, &part()).unwrap_err().to_string();
    assert!(
        err.contains("not centre-cutting") && err.contains("ramp"),
        "{err}"
    );

    let mut ops = profile("outside", json!({"count": 4, "width": 8, "height": 3}));
    ops[0]["entry"] = json!({"type": "ramp", "ramp_angle": THREE_DEG});
    let plan = plan_job(&not_centre_cutting(ops), &part()).unwrap();
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
    // Two passes, each ramped in, and a ramp after each of the 4 tabs on the last pass.
    let ramps = assert_ramps_only(&plan.program.moves, 0.0, THREE_DEG);
    assert!(ramps >= 2 * 2 + 4 * 2, "{ramps} ramp moves");
    // Every lap is still cut in full at its depth: the tab verification passed, and the
    // lifts are the same four.
    let lifts = plan
        .program
        .moves
        .iter()
        .filter(|m| m.z == Some(-9.0))
        .count();
    assert!(lifts >= 4);
}

#[test]
fn pockets_ramp_in_every_strategy() {
    let pocket_part = vec![common::polygon(&[
        (0.0, 0.0),
        (60.0, 0.0),
        (60.0, 40.0),
        (0.0, 40.0),
    ])];
    for strategy in [
        json!({"type": "rings"}),
        json!({"type": "trochoidal", "loop_radius": 1.5, "step": 0.6}),
        json!({"type": "adaptive", "load": 0.1}),
    ] {
        let ops = json!([{"type": "pocket", "select": {"layer": "L"}, "tool": 1,
            "start_depth": 1, "end_depth": 7, "clearance": 0, "step_down": 3, "stepover": 0.4,
            "direction": "climb", "feed": 1500, "plunge_feed": 500, "strategy": strategy,
            "entry": {"type": "ramp", "ramp_angle": THREE_DEG}}]);
        let plan = plan_job(&not_centre_cutting(ops), &pocket_part)
            .unwrap_or_else(|e| panic!("{strategy}: {e}"));
        // Ramps start at the start depth (Z -1): only that is taken as cut.
        let ramps = assert_ramps_only(&plan.program.moves, -1.0, THREE_DEG);
        assert!(ramps > 0, "{strategy}");
    }
}

#[test]
fn a_ramp_steeper_than_30_degrees_is_refused() {
    let mut ops = profile("outside", Value::Null);
    ops[0]["entry"] = json!({"type": "ramp", "ramp_angle": 0.6});
    let err = parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            "drawing": {"shapes": [], "next_shape_id": 1, "source": null},
            "stock": {"x": 300, "y": 200, "thickness": 18, "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": ops,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("entry.ramp_angle: must be more than 0 and at most 30"),
        "{err}"
    );
}
