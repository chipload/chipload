// SPDX-License-Identifier: GPL-3.0-or-later
//! Rest machining (docs/specs/rest-machining.md §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::distance;
use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Job, parse_job};
use chipload_toolpath::model::MoveKind;
use chipload_toolpath::{Plan, job_problems, plan_job, validate_job};
use common::{circle, polygon_id, rect};
use serde_json::{Value, json};

/// How far a cut may come short of a bound: chords and float slack.
const SLACK: f64 = 0.02;

fn job(tools: Value, operations: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            "drawing": {"shapes": [], "next_shape_id": 1, "source": null},
            "stock": {"x": 400, "y": 300, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": tools,
            "operations": operations,
            "copies": null,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn tools() -> Value {
    json!([
        {"id": 1, "diameter": 12.0, "rpm": 18000, "center_cutting": true},
        {"id": 2, "diameter": 3.0, "rpm": 18000, "center_cutting": true},
    ])
}

fn pocket(tool: i64, rest: Value) -> Value {
    json!({"type": "pocket", "select": {"layer": "L"}, "tool": tool, "start_depth": 0,
        "end_depth": 4, "clearance": 0, "step_down": 2, "stepover": 0.4,
        "direction": "climb", "feed": 1000, "plunge_feed": 300, "strategy": {"type": "rings"},
        "entry": {"type": "plunge"}, "round_corners": false, "corners": null, "rest": rest})
}

fn rest_after(tool: i64) -> Value {
    json!({"after_tool": tool})
}

/// The large pocket, then the rest pocket.
fn both() -> Value {
    json!([pocket(1, Value::Null), pocket(2, rest_after(1))])
}

fn plan(operations: Value, contours: &[Contour]) -> Plan {
    plan_job(&job(tools(), operations), contours).unwrap()
}

/// Where operation `op` cuts below the stock top: the end of every cutting move.
fn cuts(plan: &Plan, op: usize) -> Vec<Point> {
    let (mut x, mut y, mut z) = (0.0, 0.0, f64::INFINITY);
    let mut out = Vec::new();
    for span in plan.op_spans.iter() {
        for m in &plan.program.moves[span.moves.clone()] {
            x = m.x.unwrap_or(x);
            y = m.y.unwrap_or(y);
            z = m.z.unwrap_or(z);
            if span.operation == op && m.kind != MoveKind::Rapid && z < 0.0 {
                out.push((x, y));
            }
        }
    }
    out
}

/// Whether operation `op` cuts within `tol` of `p` anywhere along its moves.
fn passes(plan: &Plan, op: usize, p: Point, tol: f64) -> bool {
    let (mut x, mut y, mut z) = (0.0, 0.0, f64::INFINITY);
    for span in plan.op_spans.iter() {
        for m in &plan.program.moves[span.moves.clone()] {
            let from = (x, y);
            x = m.x.unwrap_or(x);
            y = m.y.unwrap_or(y);
            z = m.z.unwrap_or(z);
            if span.operation == op
                && m.kind != MoveKind::Rapid
                && z < 0.0
                && segment_distance(p, from, (x, y)) <= tol
            {
                return true;
            }
        }
    }
    false
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length2 = dx * dx + dy * dy;
    let t = if length2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    distance(p, (a.0 + t * dx, a.1 + t * dy))
}

fn nearest(p: Point, to: &[Point]) -> f64 {
    to.iter()
        .map(|&q| distance(p, q))
        .fold(f64::INFINITY, f64::min)
}

const SQUARE: [Point; 4] = [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)];

#[test]
fn a_square_pocket_gets_its_four_corners_and_nothing_else() {
    let plan = plan(both(), &[rect(1, 0.0, 0.0, 40.0, 40.0)]);
    let rest = cuts(&plan, 1);
    assert!(!rest.is_empty());
    for &p in &rest {
        // Inside the square shrunk by the small tool's radius.
        assert!(p.0 >= 1.5 - SLACK && p.0 <= 38.5 + SLACK, "{p:?}");
        assert!(p.1 >= 1.5 - SLACK && p.1 <= 38.5 + SLACK, "{p:?}");
        // The large tool leaves material within its radius of a corner; the rest area is
        // that grown by the small tool's diameter.
        assert!(
            nearest(p, &SQUARE) <= 6.0 + 3.0 + SLACK,
            "{p:?} is far from every corner"
        );
    }
    for corner in SQUARE {
        // The small tool's centre goes into the corner: 1.5·√2 from it.
        let closest = nearest(corner, &rest);
        assert!(
            closest <= 1.5 * 2f64.sqrt() + SLACK,
            "{corner:?}: {closest}"
        );
    }
    // Four pieces, one per corner.
    let pieces = plan.pockets.len();
    assert_eq!(pieces, 1 + 4, "{pieces} pocket pieces");
    // The rest pocket leaves only the small tool's corners: four, smaller than before.
    let left: Vec<_> = plan
        .material_left
        .iter()
        .filter(|m| m.operation == 1)
        .collect();
    assert_eq!(left.len(), 4);
    for piece in left {
        let ((x0, y0), (x1, y1)) = piece.bounds();
        assert!(x1 - x0 < 1.5 && y1 - y0 < 1.5, "{:?}", piece.bounds());
    }
}

#[test]
fn a_slot_narrower_than_the_large_tool_is_cleared_along_its_length() {
    // A 40 × 40 square with an 8 mm wide slot out of its right side, 30 mm long.
    let shape = polygon_id(
        1,
        &[
            (0.0, 0.0),
            (40.0, 0.0),
            (40.0, 16.0),
            (70.0, 16.0),
            (70.0, 24.0),
            (40.0, 24.0),
            (40.0, 40.0),
            (0.0, 40.0),
        ],
    );
    let plan = plan(both(), &[shape]);
    let rest = cuts(&plan, 1);
    // The small tool gets to the slot's end wall.
    let far = rest.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    assert!((far - 68.5).abs() <= SLACK, "reaches x {far}");
    // Along both of its walls at mid length.
    assert!(passes(&plan, 1, (55.0, 17.5), SLACK));
    assert!(passes(&plan, 1, (55.0, 22.5), SLACK));
}

#[test]
fn the_gap_between_an_island_and_the_wall_is_cleared() {
    // An island 4 mm from the left wall: too narrow for 12 mm, wide enough for 3 mm.
    let plan = plan(
        both(),
        &[
            rect(1, 0.0, 0.0, 60.0, 40.0),
            rect(2, 4.0, 10.0, 20.0, 30.0),
        ],
    );
    let rest = cuts(&plan, 1);
    assert!(
        passes(&plan, 1, (2.0, 20.0), 0.5 + SLACK),
        "the gap at x 0..4 is not cut"
    );
    // Never nearer the island than the tool radius.
    for &p in &rest {
        let dx = (4.0 - p.0).max(p.0 - 20.0).max(0.0);
        let dy = (10.0 - p.1).max(p.1 - 30.0).max(0.0);
        assert!(dx.hypot(dy) >= 1.5 - SLACK, "{p:?} cuts the island");
    }
}

#[test]
fn a_circle_the_large_tool_reaches_everywhere_leaves_nothing() {
    let plan = plan(both(), &[circle((50.0, 50.0), 20.0)]);
    assert!(cuts(&plan, 1).is_empty());
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("nothing is left to rest machine")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn leftovers_too_small_for_the_small_tool_too_are_skipped_with_a_note() {
    // A 2 mm slot: neither tool fits. Only the rest pocket, which is safe on its own.
    let plan = plan(
        json!([pocket(2, rest_after(1))]),
        &[rect(1, 0.0, 0.0, 40.0, 2.0)],
    );
    assert!(cuts(&plan, 0).is_empty());
    let note = plan
        .notes
        .iter()
        .find(|n| n.contains("too small for this tool too"))
        .unwrap_or_else(|| panic!("{:?}", plan.notes));
    assert!(note.contains("1 place "), "{note}");
    assert!(note.contains("40.00 × 2.00 mm"), "{note}");
}

#[test]
fn the_rest_pocket_plans_the_same_alone_as_after_the_large_one() {
    // It doesn't depend on the earlier pocket's toolpath (decision 2).
    let shape = [rect(1, 0.0, 0.0, 40.0, 40.0)];
    let alone = plan(json!([pocket(2, rest_after(1))]), &shape);
    let after = plan(both(), &shape);
    let mut a = cuts(&alone, 0);
    let b = cuts(&after, 1);
    a.dedup();
    assert!(!a.is_empty());
    assert!(
        alone
            .notes
            .iter()
            .any(|n| n.contains("no earlier pocket cuts"))
    );
    assert!(
        !after
            .notes
            .iter()
            .any(|n| n.contains("no earlier pocket cuts"))
    );
    // Same pieces; the order may differ with the start position.
    let mut sa: Vec<_> = a
        .iter()
        .map(|p| ((p.0 * 1e3).round() as i64, (p.1 * 1e3).round() as i64))
        .collect();
    let mut sb: Vec<_> = b
        .iter()
        .map(|p| ((p.0 * 1e3).round() as i64, (p.1 * 1e3).round() as i64))
        .collect();
    sa.sort_unstable();
    sa.dedup();
    sb.sort_unstable();
    sb.dedup();
    assert_eq!(sa, sb);
}

fn problems(tools: Value, operations: Value) -> Vec<String> {
    job_problems(&job(tools, operations))
        .into_iter()
        .map(|p| p.text)
        .collect()
}

#[test]
fn rest_after_a_missing_tool_is_a_problem() {
    let found = problems(tools(), json!([pocket(2, rest_after(5))]));
    assert!(
        found
            .iter()
            .any(|p| p.contains("rest after tool 5, which the project doesn't have")),
        "{found:?}"
    );
}

#[test]
fn rest_after_a_tool_no_larger_is_a_problem() {
    let found = problems(tools(), json!([pocket(1, rest_after(2))]));
    assert!(
        found.iter().any(|p| p
            .contains("rest after tool 2 (3 mm) needs a tool larger than this pocket's (12 mm)")),
        "{found:?}"
    );
    let same = problems(
        json!([
            {"id": 1, "diameter": 6.0, "rpm": 18000, "center_cutting": true},
            {"id": 2, "diameter": 6.0, "rpm": 18000, "center_cutting": true},
        ]),
        json!([pocket(2, rest_after(1))]),
    );
    assert!(
        same.iter().any(|p| p.contains("needs a tool larger")),
        "{same:?}"
    );
}

#[test]
fn rest_after_a_drill_is_a_problem() {
    let found = problems(
        json!([
            {"id": 1, "type": "drill", "diameter": 12.0, "rpm": 18000},
            {"id": 2, "diameter": 3.0, "rpm": 18000, "center_cutting": true},
        ]),
        json!([pocket(2, rest_after(1))]),
    );
    assert!(
        found
            .iter()
            .any(|p| p.contains("rest after tool 1, a drill; a pocket's tool is an endmill")),
        "{found:?}"
    );
}

#[test]
fn rest_with_dogbones_is_a_problem() {
    let mut op = pocket(2, rest_after(1));
    op["corners"] = json!({"type": "dogbone", "max_angle": 1.7});
    let found = problems(tools(), json!([pocket(1, Value::Null), op]));
    assert!(
        found
            .iter()
            .any(|p| p.contains("dogbones and rest machining can't be combined yet")),
        "{found:?}"
    );
}

#[test]
fn the_earlier_pocket_must_cut_the_same_shapes_as_deep() {
    let warned = |operations: Value| {
        let notes = validate_job(&job(tools(), operations)).unwrap();
        notes.iter().any(|n| {
            n.contains("rests after tool 1, but no earlier pocket cuts these shapes with it")
        })
    };
    assert!(!warned(both()));
    // Shallower.
    let mut shallow = pocket(1, Value::Null);
    shallow["end_depth"] = json!(2);
    assert!(warned(json!([shallow, pocket(2, rest_after(1))])));
    // After it instead of before.
    assert!(warned(json!([
        pocket(2, rest_after(1)),
        pocket(1, Value::Null)
    ])));
    // Disabled.
    let mut off = pocket(1, Value::Null);
    off["enabled"] = json!(false);
    assert!(warned(json!([off, pocket(2, rest_after(1))])));
    // Other shapes.
    let mut other = pocket(1, Value::Null);
    other["select"] = json!({"shapes": [7]});
    assert!(warned(json!([other, pocket(2, rest_after(1))])));
}

#[test]
fn a_pocket_without_rest_is_unchanged() {
    // `rest: null` and a missing field plan the same.
    let shape = [rect(1, 0.0, 0.0, 40.0, 40.0)];
    let mut missing = pocket(2, Value::Null);
    missing.as_object_mut().unwrap().remove("rest");
    let a = plan(json!([pocket(2, Value::Null)]), &shape);
    let b = plan(json!([missing]), &shape);
    assert_eq!(a.program, b.program);
}
