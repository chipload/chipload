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
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left"},
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
            "tool": 1, "depth": 12, "step_down": 6, "feed": 1500, "plunge_feed": 500,
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
    assert!(plan.notes.is_empty());
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
