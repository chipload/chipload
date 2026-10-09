// SPDX-License-Identifier: GPL-3.0-or-later
//! Job-level rules of plan_job.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::chain::Contour;
use chipload_job::{Job, parse_job};
use chipload_toolpath::{ToolpathError, plan_job};
use serde_json::{Value, json};

fn job(operations: Value) -> Job {
    parse_job(
        json!({
            "schema_version": 3,
            "source": {"file": "x.dxf", "units": "mm"},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left"},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": operations,
        }),
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
