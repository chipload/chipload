// SPDX-License-Identifier: GPL-3.0-or-later
//! Holes: drill cycles and helical bores (docs/specs/holes.md §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::f64::consts::{PI, TAU};

use chipload_core::chain::Contour;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::holes::{PECK_CLEARANCE_MM, circle_of};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::{Plan, plan_job};
use common::{bulged, segment};
use proptest::prelude::*;
use serde_json::{Value, json};

const DEG: f64 = PI / 180.0;
const TOL: f64 = 1e-9;

/// A circle as one drawn arc of a full turn.
fn circle(id: usize, x: f64, y: f64, radius: f64) -> Contour {
    Contour {
        id,
        parts: vec![segment(Primitive::Arc(Arc {
            center: (x, y),
            radius,
            start_angle: 0.0,
            sweep: TAU,
        }))],
        closed: true,
        invalid_reason: None,
        nesting_depth: Some(1),
    }
}

/// A circle as two half arcs, as a bulged polyline.
fn halves(id: usize, x: f64, y: f64, radius: f64) -> Contour {
    bulged(id, &[(x + radius, y, 1.0), (x - radius, y, 1.0)])
}

fn square(id: usize, x: f64, y: f64, side: f64) -> Contour {
    let corners = [(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
    let parts = (0..4)
        .map(|i| {
            segment(Primitive::Line(Line {
                start: corners[i],
                end: corners[(i + 1) % 4],
            }))
        })
        .collect();
    Contour {
        id,
        parts,
        closed: true,
        invalid_reason: None,
        nesting_depth: Some(0),
    }
}

fn job_with(operations: Value, tools: Value, copies: Value, thickness: f64) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": thickness, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": tools,
            "operations": operations,
            "copies": copies,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn drill_tool(diameter: f64) -> Value {
    json!([{"id": 1, "type": "drill", "diameter": diameter, "rpm": 6000}])
}

fn endmill(diameter: f64) -> Value {
    json!([{"id": 1, "type": "endmill", "diameter": diameter, "rpm": 18000,
            "center_cutting": true}])
}

fn drill_op(end: f64, peck: Value) -> Value {
    json!([{"type": "holes", "select": {"layer": "L"}, "tool": 1, "start_depth": 0,
            "end_depth": end, "feed": 1000, "plunge_feed": 300,
            "method": {"type": "drill", "peck": peck}}])
}

fn bore_op(end: f64, step: f64, angle_deg: f64, direction: &str) -> Value {
    json!([{"type": "holes", "select": {"layer": "L"}, "tool": 1, "start_depth": 0,
            "end_depth": end, "feed": 1000, "plunge_feed": 300,
            "method": {"type": "bore", "step_down": step, "ramp_angle": angle_deg * DEG,
                       "direction": direction}}])
}

fn plan(operations: Value, tools: Value, contours: &[Contour]) -> Plan {
    plan_job(&job_with(operations, tools, Value::Null, 18.0), contours).unwrap()
}

fn plan_err(operations: Value, tools: Value, contours: &[Contour], thickness: f64) -> String {
    plan_job(
        &job_with(operations, tools, Value::Null, thickness),
        contours,
    )
    .unwrap_err()
    .to_string()
}

/// A move with where it starts and ends, (x, y, z).
type Step = (Move, (f64, f64, f64), (f64, f64, f64));

/// Every move with where it starts and ends.
fn walk(moves: &[Move]) -> Vec<Step> {
    let mut at = (f64::NAN, f64::NAN, f64::NAN);
    moves
        .iter()
        .map(|m| {
            let to = (
                m.x.unwrap_or(at.0),
                m.y.unwrap_or(at.1),
                m.z.unwrap_or(at.2),
            );
            let step = (*m, at, to);
            at = to;
            step
        })
        .collect()
}

// -- drill (§3)

#[test]
fn every_hole_is_drilled_at_its_centre_in_pecks() {
    let contours = [circle(1, 20.0, 20.0, 4.0), circle(2, 60.0, 20.0, 4.0)];
    let plan = plan(drill_op(10.0, json!(4.0)), drill_tool(8.0), &contours);
    let steps = walk(&plan.program.moves);
    let mut bottoms = Vec::new();
    for (m, from, to) in &steps {
        if m.kind == MoveKind::Feed {
            // Straight down only, at a hole's centre.
            assert_eq!((from.0, from.1), (to.0, to.1));
            assert!(to.2 < from.2);
            assert!([(20.0, 20.0), (60.0, 20.0)].contains(&(to.0, to.1)));
            assert_eq!(m.feed, Some(300.0));
            bottoms.push(to.2);
        }
    }
    // 10 mm in pecks of at most 4: three even pecks, the last exactly at the end depth.
    let third = 10.0 / 3.0;
    let expected = [-third, -2.0 * third, -10.0];
    assert_eq!(bottoms.len(), 6);
    for (got, want) in bottoms.iter().zip(expected.iter().chain(expected.iter())) {
        assert!((got - want).abs() < TOL, "{got} vs {want}");
    }
    // Between pecks: up to the safe height, back down to 0.5 mm above the drilled depth.
    let reentries: Vec<f64> = steps
        .iter()
        .filter(|(m, _, to)| m.kind == MoveKind::Rapid && m.x.is_none() && to.2 < 0.0)
        .map(|(_, _, to)| to.2)
        .collect();
    assert_eq!(reentries.len(), 4);
    assert!((reentries[0] - (-third + PECK_CLEARANCE_MM)).abs() < TOL);
    assert!((reentries[1] - (-2.0 * third + PECK_CLEARANCE_MM)).abs() < TOL);
    for (i, (m, from, _)) in steps.iter().enumerate() {
        if m.kind == MoveKind::Rapid && m.z.is_some_and(|z| z < 0.0) {
            let (up, _, _) = steps[i - 1];
            assert_eq!(
                up,
                Move::rapid_z(5.0),
                "always after rising to the safe height"
            );
            assert_eq!(from.2, 5.0);
        }
    }
}

#[test]
fn without_pecks_or_with_a_peck_deeper_than_the_cut_it_drills_in_one_go() {
    let contours = [circle(1, 20.0, 20.0, 4.0)];
    for peck in [Value::Null, json!(12.0), json!(10.0)] {
        let plan = plan(drill_op(10.0, peck.clone()), drill_tool(8.0), &contours);
        let feeds: Vec<&Move> = plan
            .program
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Feed)
            .collect();
        assert_eq!(feeds.len(), 1, "peck {peck}");
        assert_eq!(feeds[0].z, Some(-10.0));
    }
}

#[test]
fn a_drill_cuts_circles_within_a_tenth_of_its_size() {
    // 8 mm drill: 7.9 and 8.1 are drilled, 7.8 and 8.2 aren't.
    let contours = [
        circle(1, 20.0, 20.0, 3.95),
        circle(2, 40.0, 20.0, 4.05),
        circle(3, 60.0, 20.0, 3.9),
        circle(4, 80.0, 20.0, 4.1),
        circle(5, 100.0, 20.0, 4.1),
    ];
    let plan = plan(drill_op(5.0, Value::Null), drill_tool(8.0), &contours);
    let centres: Vec<(f64, f64)> = walk(&plan.program.moves)
        .iter()
        .filter(|(m, _, _)| m.kind == MoveKind::Feed)
        .map(|(_, _, to)| (to.0, to.1))
        .collect();
    assert_eq!(centres, vec![(20.0, 20.0), (40.0, 20.0)]);
    let notes = plan
        .notes
        .iter()
        .map(|n| n.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        notes.contains("hole 7.8 mm: not the 8 mm drill's size (within 0.1 mm); not drilled"),
        "{notes}"
    );
    assert!(
        notes.contains("2 holes of 8.2 mm: not the 8 mm drill's size"),
        "{notes}"
    );
}

// -- bore (§4)

/// The bore's moves for one hole, and its tool-centre radius.
fn bore_steps(
    hole_radius: f64,
    tool: f64,
    step: f64,
    angle_deg: f64,
    direction: &str,
) -> Vec<Step> {
    let plan = plan(
        bore_op(6.0, step, angle_deg, direction),
        endmill(tool),
        &[circle(1, 50.0, 50.0, hole_radius)],
    );
    walk(&plan.program.moves)
}

#[test]
fn the_helix_follows_the_circle_down_then_one_level_circle() {
    let (hole, tool, step) = (5.0, 6.0, 1.0);
    let rc = hole - tool / 2.0;
    let steps = bore_steps(hole, tool, step, 10.0, "climb");
    let cutting: Vec<_> = steps
        .iter()
        .filter(|(m, from, to)| m.kind == MoveKind::Feed && (from.0, from.1) != (to.0, to.1))
        .collect();
    // The last cutting move steps off the wall toward the centre; the rest are on the circle.
    let (leave, on_circle) = cutting.split_last().unwrap();
    let mut level = 0;
    let mut travelled = 0.0;
    let mut dropped = 0.0;
    for (m, from, to) in on_circle {
        assert!((distance((to.0, to.1), (50.0, 50.0)) - rc).abs() < TOL);
        assert_eq!(m.feed, Some(1000.0));
        let run = distance((from.0, from.1), (to.0, to.1));
        let drop = from.2 - to.2;
        assert!(drop >= -TOL, "never up");
        assert!(
            drop <= run * (10.0 * DEG).tan() + TOL,
            "never steeper than the angle"
        );
        travelled += run;
        dropped += drop;
        // At most a step per turn, measured over the whole helix.
        assert!(dropped <= step * (travelled / (TAU * rc)).ceil() + TOL);
        if to.2 == -6.0 && from.2 == -6.0 {
            level += 1;
        }
    }
    assert!(level > 0, "a level circle at the end depth");
    let level_length: f64 = on_circle
        .iter()
        .filter(|(_, from, to)| from.2 == -6.0 && to.2 == -6.0)
        .map(|(_, from, to)| distance((from.0, from.1), (to.0, to.1)))
        .sum();
    assert!(level_length > 0.99 * TAU * rc, "a whole level turn");
    // Off the wall by half the tool radius (the centre is cut: rc <= r), then up.
    let (_, from, to) = leave;
    assert!((distance((from.0, from.1), (to.0, to.1)) - 1.5).abs() < TOL);
    assert!(distance((to.0, to.1), (50.0, 50.0)) < rc);
    assert_eq!(steps.last().unwrap().0, Move::rapid_z(5.0));
}

/// Whether the helix turns clockwise seen from above.
fn turns_clockwise(direction: &str, spindle: &str) -> bool {
    let mut job = job_with(
        bore_op(2.0, 1.0, 10.0, direction),
        endmill(6.0),
        Value::Null,
        18.0,
    );
    job.machine.spindle = if spindle == "cw" {
        chipload_job::Spindle::Cw
    } else {
        chipload_job::Spindle::Ccw
    };
    let plan = plan_job(&job, &[circle(1, 50.0, 50.0, 5.0)]).unwrap();
    let steps = walk(&plan.program.moves);
    let (_, from, to) = steps
        .iter()
        .find(|(m, from, to)| m.kind == MoveKind::Feed && (from.0, from.1) != (to.0, to.1))
        .unwrap();
    // From the start at +X of the centre: up (+Y) is counter-clockwise.
    assert!((from.0 - 52.0).abs() < TOL && (from.1 - 50.0).abs() < TOL);
    to.1 < from.1
}

#[test]
fn the_helix_turns_as_an_inside_profile_does() {
    // Climb on an inside contour with a clockwise spindle: counter-clockwise.
    assert!(!turns_clockwise("climb", "cw"));
    assert!(turns_clockwise("conventional", "cw"));
    assert!(turns_clockwise("climb", "ccw"));
    assert!(!turns_clockwise("conventional", "ccw"));
}

#[test]
fn the_smallest_hole_bored_is_the_tool_plus_a_fifth() {
    // 6 mm tool: the helix radius must be at least 0.6 mm, so a 7.2 mm hole.
    let steps = bore_steps(3.6, 6.0, 1.0, 10.0, "climb");
    assert!(steps.iter().any(|(m, _, _)| m.kind == MoveKind::Feed));
    let err = plan_err(
        bore_op(6.0, 1.0, 10.0, "climb"),
        endmill(6.0),
        &[circle(1, 50.0, 50.0, 3.59)],
        18.0,
    );
    assert!(
        err.contains("no selected circle can be cut with tool 1 (6 mm endmill)"),
        "{err}"
    );
    let plan = plan(
        bore_op(6.0, 1.0, 10.0, "climb"),
        endmill(6.0),
        &[circle(1, 50.0, 50.0, 3.59), circle(2, 80.0, 50.0, 5.0)],
    );
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("hole 7.18 mm: too small for the 6 mm tool to bore")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn a_hole_twice_the_tool_leaves_no_core() {
    // rc = r exactly: the helix cuts the centre; it steps off the wall and no note.
    let plan = plan(
        bore_op(6.0, 1.0, 10.0, "climb"),
        endmill(6.0),
        &[circle(1, 50.0, 50.0, 6.0)],
    );
    assert!(
        !plan.notes.iter().any(|n| n.contains("core")),
        "{:?}",
        plan.notes
    );
    let steps = walk(&plan.program.moves);
    let (_, from, to) = steps
        .iter()
        .rev()
        .find(|(m, from, to)| m.kind == MoveKind::Feed && (from.0, from.1) != (to.0, to.1))
        .unwrap();
    assert!((distance((from.0, from.1), (to.0, to.1)) - 1.5).abs() < TOL);
}

#[test]
fn a_larger_hole_leaves_a_core_through_but_is_refused_blind() {
    let contours = [circle(1, 50.0, 50.0, 6.5)];
    // Through: the end depth is the stock thickness.
    let plan = plan_job(
        &job_with(
            bore_op(12.0, 2.0, 10.0, "climb"),
            endmill(6.0),
            Value::Null,
            12.0,
        ),
        &contours,
    )
    .unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("1 hole leaves a loose core (hole more than twice the tool)")),
        "{:?}",
        plan.notes
    );
    // No step toward the core: straight up at the wall after the level circle.
    let steps = walk(&plan.program.moves);
    let last_feed = steps
        .iter()
        .rev()
        .find(|(m, _, _)| m.kind == MoveKind::Feed)
        .unwrap();
    assert!((distance((last_feed.2.0, last_feed.2.1), (50.0, 50.0)) - 3.5).abs() < TOL);
    let err = plan_err(
        bore_op(6.0, 2.0, 10.0, "climb"),
        endmill(6.0),
        &contours,
        12.0,
    );
    assert!(
        err.contains("hole 13 mm is blind and more than twice the 6 mm tool; use a pocket"),
        "{err}"
    );
}

// -- circles (§1)

#[test]
fn circles_are_recognised_and_other_shapes_skipped() {
    assert!(circle_of(&circle(1, 0.0, 0.0, 5.0)).is_some());
    let two = circle_of(&halves(2, 10.0, 0.0, 5.0)).unwrap();
    assert!(distance(two.center, (10.0, 0.0)) < TOL && (two.radius - 5.0).abs() < TOL);
    assert!(circle_of(&square(3, 0.0, 0.0, 10.0)).is_none());
    // Two half arcs with centres 0.01 mm apart: not one circle.
    let mut off = halves(4, 0.0, 0.0, 5.0);
    if let Primitive::Arc(arc) = &mut off.parts[1].segment.prim {
        arc.center.0 += 0.01;
    }
    assert!(circle_of(&off).is_none());
    // Most of a circle closed by a line: not a circle.
    assert!(circle_of(&bulged(5, &[(5.0, 0.0, 1.0), (-5.0, 0.0, 0.5)])).is_none());

    let plan = plan(
        drill_op(5.0, Value::Null),
        drill_tool(8.0),
        &[circle(1, 20.0, 20.0, 4.0), square(2, 50.0, 50.0, 10.0)],
    );
    assert!(
        plan.notes
            .iter()
            .any(|n| n.ends_with("skipped 1 shape that isn't a circle")),
        "{:?}",
        plan.notes
    );
}

// -- plan (§2, §5)

#[test]
fn the_tool_must_suit_the_method() {
    let contours = [circle(1, 20.0, 20.0, 4.0)];
    let vbit = json!([{"id": 1, "type": "vbit", "diameter": 8, "angle": 1.0, "rpm": 18000}]);
    let err = plan_err(drill_op(5.0, Value::Null), vbit, &contours, 18.0);
    assert!(err.contains("is a V-bit, which can't cut holes"), "{err}");
    let err = plan_err(
        bore_op(5.0, 1.0, 3.0, "climb"),
        drill_tool(8.0),
        &contours,
        18.0,
    );
    assert!(
        err.contains("the method is bore, which needs an endmill; tool 1 is a drill"),
        "{err}"
    );
    let err = plan_err(drill_op(5.0, Value::Null), endmill(8.0), &contours, 18.0);
    assert!(
        err.contains("the method is drill, which needs a drill; tool 1 is an endmill"),
        "{err}"
    );
}

#[test]
fn holes_go_nearest_first_and_every_copy_gets_its_own() {
    let contours = [
        circle(1, 100.0, 20.0, 4.0),
        circle(2, 10.0, 20.0, 4.0),
        circle(3, 40.0, 20.0, 4.0),
    ];
    let job = job_with(
        drill_op(5.0, Value::Null),
        drill_tool(8.0),
        json!({"columns": 2, "rows": 1, "dx": 150, "dy": 0}),
        18.0,
    );
    let plan = plan_job(&job, &contours).unwrap();
    let centres: Vec<f64> = walk(&plan.program.moves)
        .iter()
        .filter(|(m, _, _)| m.kind == MoveKind::Feed)
        .map(|(_, _, to)| to.0)
        .collect();
    // From the origin: 10, 40, 100; the second copy from there: 160, 190, 250.
    assert_eq!(centres, vec![10.0, 40.0, 100.0, 160.0, 190.0, 250.0]);
}

proptest! {
    #![proptest_config(common::proptest_config(64))]

    /// Every bore move stays in its hole, on the helix circle while cutting, never below
    /// the end depth.
    #[test]
    fn bores_stay_in_their_holes(
        tool in 2.0f64..10.0,
        extra in 0.2f64..2.5,
        step in 0.3f64..4.0,
        angle in 1.0f64..30.0,
        end in 1.0f64..12.0,
    ) {
        let hole = tool * (1.0 + extra) / 2.0;
        let rc = hole - tool / 2.0;
        // A blind hole with a core is refused: only test those that are cut.
        prop_assume!(rc <= tool / 2.0 || end >= 12.0 - 1e-9);
        let plan = plan_job(
            &job_with(bore_op(end, step, angle, "climb"), endmill(tool), Value::Null, 12.0),
            &[circle(1, 50.0, 50.0, hole)],
        )
        .unwrap();
        for (m, from, to) in walk(&plan.program.moves) {
            if m.kind == MoveKind::Rapid {
                continue;
            }
            prop_assert!(to.2 >= -end - TOL);
            let d = distance((to.0, to.1), (50.0, 50.0));
            prop_assert!(d <= rc + TOL);
            if (from.0, from.1) != (to.0, to.1) && from.2 != to.2 {
                prop_assert!((d - rc).abs() < TOL);
            }
        }
    }
}
