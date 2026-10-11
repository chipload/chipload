// SPDX-License-Identifier: GPL-3.0-or-later
//! Helical entry for pockets (docs/specs/helix-and-leads.md part C).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::chain::Contour;
use chipload_core::primitives::distance;
use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Job, parse_job};
use chipload_toolpath::arcs::merge_arcs;
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::{Plan, plan_job};
use serde_json::{Value, json};

const THREE_DEG: f64 = 0.052_359_877_559_829_88;

fn job(operations: Value) -> Job {
    let mut job = parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            "drawing": {"shapes": [], "next_shape_id": 1, "source": null},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": operations,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap();
    job.tools[0].center_cutting = Some(false);
    job
}

fn pocket(strategy: Value, entry: Value) -> Value {
    json!([{"type": "pocket", "select": {"layer": "L"}, "tool": 1, "start_depth": 1,
        "end_depth": 7, "clearance": 0, "step_down": 3, "stepover": 0.4,
        "direction": "climb", "feed": 1500, "plunge_feed": 500, "strategy": strategy,
        "entry": entry}])
}

fn helix(radius: f64) -> Value {
    json!({"type": "helix", "radius": radius, "ramp_angle": THREE_DEG})
}

fn rings() -> Value {
    json!({"type": "rings"})
}

/// (x, y, z) after every move.
fn positions(moves: &[Move]) -> Vec<(f64, f64, f64)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, f64::INFINITY);
    moves
        .iter()
        .map(|m| {
            (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
            (x, y, z)
        })
        .collect()
}

/// Straight descents while cutting only reach depths already cut (or the start depth,
/// `top`); every descent that moves sideways is no steeper than 3°. Returns how many
/// sideways descents there are.
fn assert_entries_safe(plan: &Plan, top: f64) -> usize {
    let at = positions(&plan.program.moves);
    let mut deepest = top;
    let mut sideways = 0;
    for (i, m) in plan.program.moves.iter().enumerate().skip(1) {
        let (before, now) = (at[i - 1], at[i]);
        if m.kind != MoveKind::Rapid && now.2 < before.2 - 1e-9 {
            let run = (now.0 - before.0).hypot(now.1 - before.1);
            if run == 0.0 {
                assert!(now.2 >= deepest - 1e-9, "move {i}: plunge to {}", now.2);
            } else {
                assert!(
                    (before.2 - now.2) / run <= THREE_DEG.tan() + 1e-9,
                    "move {i}"
                );
                sideways += 1;
            }
        }
        deepest = deepest.min(now.2);
    }
    sideways
}

fn rect(w: f64, h: f64) -> Vec<Contour> {
    vec![common::rect(0, 0.0, 0.0, w, h)]
}

#[test]
fn a_pocket_goes_down_round_a_helix_in_its_middle_on_every_pass() {
    let plan = plan_job(&job(pocket(rings(), helix(1.5))), &rect(60.0, 40.0)).unwrap();
    let piece = &plan.pockets[0];
    let h = piece.helix.expect("a helix fits");
    assert_eq!(h.circle.radius, 1.5);
    // In the middle band: at least a tool radius plus the helix from the long walls, and
    // more than that from the ends.
    let c = h.circle.center;
    assert!((c.1 - 20.0).abs() <= 3.0 + 1e-9, "{c:?}");
    assert!(c.0 >= 7.5 && c.0 <= 52.5, "{c:?}");
    // Two passes, each going down round the helix (whole turns), no plunge into material.
    let sideways = assert_entries_safe(&plan, -1.0);
    assert!(sideways > 0);
    let at = positions(&plan.program.moves);
    let starts = plan
        .program
        .moves
        .iter()
        .enumerate()
        .filter(|(_, m)| m.kind == MoveKind::Rapid && m.x.is_some())
        .filter(|(i, _)| distance((at[*i].0, at[*i].1), h.start) < 1e-9)
        .count();
    assert_eq!(starts, 2, "one helix per pass");
    // The helix's moves carry its circle, ready for G2/G3.
    let on_helix = plan
        .program
        .moves
        .iter()
        .filter(|m| m.circle == Some(h.circle))
        .count();
    assert!(on_helix > 0);
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
}

#[test]
fn every_strategy_can_start_with_a_helix() {
    for strategy in [
        rings(),
        json!({"type": "trochoidal", "loop_radius": 1.5, "step": 0.6}),
        json!({"type": "adaptive"}),
    ] {
        let ops = pocket(strategy.clone(), helix(1.5));
        let plan =
            plan_job(&job(ops), &rect(60.0, 40.0)).unwrap_or_else(|e| panic!("{strategy}: {e}"));
        assert!(plan.pockets[0].helix.is_some(), "{strategy}");
        assert_entries_safe(&plan, -1.0);
    }
}

#[test]
fn in_a_narrow_slot_the_helix_shrinks_then_gives_way_to_a_ramp() {
    // 8 mm wide: the tool centre has 1 mm either side of the middle, so radius 1.5 shrinks.
    let plan = plan_job(&job(pocket(rings(), helix(1.5))), &rect(60.0, 8.0)).unwrap();
    let h = plan.pockets[0].helix.unwrap();
    assert!(
        h.circle.radius < 1.0 && h.circle.radius > 0.99,
        "{}",
        h.circle.radius
    );
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("helix radius reduced to 1.000 mm (no room for 1.500 mm)")),
        "{:?}",
        plan.notes
    );
    // 6.8 mm: 0.4 mm either side, less than the 0.6 mm minimum: it ramps in instead.
    let plan = plan_job(&job(pocket(rings(), helix(1.5))), &rect(60.0, 6.8)).unwrap();
    assert!(plan.pockets[0].helix.is_none());
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("no room for a helix (not even 0.600 mm); ramping in")),
        "{:?}",
        plan.notes
    );
    assert!(assert_entries_safe(&plan, -1.0) > 0);
}

#[test]
fn a_helix_on_a_profile_or_as_wide_as_the_tool_is_refused() {
    let profile = json!([{"type": "profile", "select": {"layer": "L"}, "side": "outside",
        "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 6, "clearance": 0,
        "step_down": 3, "feed": 1500, "plunge_feed": 500, "entry": helix(1.5)}]);
    let err = plan_job(&job(profile), &rect(60.0, 40.0))
        .unwrap_err()
        .to_string();
    assert!(err.contains("helix entry is for pockets"), "{err}");
    let err = plan_job(&job(pocket(rings(), helix(3.0))), &rect(60.0, 40.0))
        .unwrap_err()
        .to_string();
    assert!(err.contains("must be less than the tool radius 3"), "{err}");
}

#[test]
fn a_step_down_as_deep_as_the_cut_is_one_helix() {
    let mut ops = pocket(rings(), helix(1.5));
    ops[0]["start_depth"] = json!(0);
    ops[0]["end_depth"] = json!(18);
    ops[0]["step_down"] = json!(18);
    let plan = plan_job(&job(ops), &rect(60.0, 40.0)).unwrap();
    let h = plan.pockets[0].helix.unwrap();
    // 18 mm at 0.494 mm per turn: 37 turns, all the way down at once.
    let deepest = positions(&plan.program.moves)
        .iter()
        .map(|p| p.2)
        .fold(f64::INFINITY, f64::min);
    assert_eq!(deepest, -18.0);
    assert_entries_safe(&plan, 0.0);
    assert_eq!(h.circle.radius, 1.5);
}

#[test]
fn the_helix_is_written_as_helical_arcs_of_at_most_half_a_turn() {
    let plan = plan_job(&job(pocket(rings(), helix(1.5))), &rect(60.0, 40.0)).unwrap();
    let merged = merge_arcs(&plan.program).unwrap();
    let h = plan.pockets[0].helix.unwrap();
    let helical: Vec<&Move> = merged
        .moves
        .iter()
        .filter(|m| matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw) && m.z.is_some())
        .collect();
    // Climb with M3 cuts the walls counter-clockwise; the helix turns the same way.
    assert!(helical.iter().all(|m| m.kind == MoveKind::ArcCcw));
    assert!(helical.iter().all(|m| m.circle == Some(h.circle)));
    // Pass 1 goes 3 mm down from Z-1 in 7 turns, pass 2 another 3: 14 turns, 2 pieces each.
    assert_eq!(helical.len(), 28, "{}", helical.len());
    // The ends of the pieces are on the circle and go down evenly.
    let at = positions(&merged.moves);
    for (i, m) in merged.moves.iter().enumerate().skip(1) {
        if matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw) && m.z.is_some() {
            let (a, b) = (at[i - 1], at[i]);
            assert!((distance((b.0, b.1), h.circle.center) - 1.5).abs() < 1e-9);
            assert!(b.2 < a.2, "goes down");
        }
    }
}
