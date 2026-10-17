// SPDX-License-Identifier: GPL-3.0-or-later
//! Lead-in and lead-out on pocket walls (docs/specs/helix-and-leads.md part B); profiles
//! have none since they always spiral (docs/specs/profile-always-spiral.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::plan_job;
use serde_json::{Value, json};

fn job(operations: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000, "center_cutting": true}],
            "operations": operations,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn profile(side: &str, extra: Value) -> Value {
    let mut op = json!({"type": "profile", "select": {"layer": "L"}, "side": side,
        "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 12, "clearance": 0,
        "step_down": 6, "feed": 1500, "plunge_feed": 500});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    json!([op])
}

fn pocket(strategy: Value, extra: Value) -> Value {
    let mut op = json!({"type": "pocket", "round_corners": false, "select": {"layer": "L"}, "tool": 1,
        "start_depth": 0, "end_depth": 6, "clearance": 0, "step_down": 3, "stepover": 0.4,
        "direction": "climb", "feed": 1500, "plunge_feed": 500, "strategy": strategy,
        "lead": {"radius": 3}});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    json!([op])
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

/// Least distance from `p` to the rectangle's outline.
fn to_outline(p: Point, (x0, y0, x1, y1): (f64, f64, f64, f64)) -> f64 {
    let dx = (x0 - p.0).max(p.0 - x1).max(0.0);
    let dy = (y0 - p.1).max(p.1 - y1).max(0.0);
    if dx > 0.0 || dy > 0.0 {
        dx.hypot(dy)
    } else {
        (p.0 - x0).min(x1 - p.0).min(p.1 - y0).min(y1 - p.1)
    }
}

#[test]
fn a_huge_lead_is_refused() {
    let parts = vec![common::rect(0, 0.0, 0.0, 60.0, 40.0)];
    let strategy = json!({"type": "rings"});
    let err = plan_job(
        &job(pocket(strategy, json!({"lead": {"radius": 60.1}}))),
        &parts,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("lead radius 60.1 mm is more than 10"), "{err}");
}

#[test]
fn a_pocket_wall_gets_leads() {
    let parts = vec![common::rect(0, 0.0, 0.0, 60.0, 40.0)];
    {
        let strategy = json!({"type": "rings"});
        let plan = plan_job(&job(pocket(strategy.clone(), json!({}))), &parts)
            .unwrap_or_else(|e| panic!("{strategy}: {e}"));
        let piece = &plan.pockets[0];
        let k = piece.rings.len() - 1;
        let range = piece.rings[k].lead.clone().expect("the wall has leads");
        assert!(piece.rings[k].open);
        // The wall ring, starting in the middle of its longest chord (the bottom or the top
        // edge, as long as each other: the one nearer where the clearing ended), with the
        // leads around it; every corner of it is still there.
        let ring = &piece.rings[k].points[range.clone()];
        assert!(
            ring[0] == (30.0, 3.0) || ring[0] == (30.0, 37.0),
            "{strategy}: {:?}",
            ring[0]
        );
        for corner in [(3.0, 3.0), (57.0, 3.0), (57.0, 37.0), (3.0, 37.0)] {
            assert!(ring.contains(&corner), "{strategy}: {corner:?}");
        }
        let path = &piece.rings[k].points;
        assert!(range.start > 0 && range.end < path.len());
        // The leads stay at least the tool radius from the walls: inside (3, 3)-(57, 37).
        for &p in path {
            assert!(
                p.0 >= 3.0 - 1e-9 && p.0 <= 57.0 + 1e-9 && p.1 >= 3.0 - 1e-9 && p.1 <= 37.0 + 1e-9,
                "{strategy}: {p:?}"
            );
        }
        assert_eq!(plan.ring_starts[0].ring, ring, "{strategy}");
        assert!(
            plan.notes.iter().all(|n| !n.contains("lead")),
            "{:?}",
            plan.notes
        );
    }
}

#[test]
fn island_walls_get_leads_on_their_cleared_side() {
    let parts = vec![
        common::rect(0, 0.0, 0.0, 80.0, 60.0),
        common::rect(1, 30.0, 20.0, 50.0, 40.0),
    ];
    let plan = plan_job(&job(pocket(json!({"type": "rings"}), json!({}))), &parts).unwrap();
    let piece = &plan.pockets[0];
    let led: Vec<&chipload_toolpath::pocket::PocketRing> =
        piece.rings.iter().filter(|r| r.lead.is_some()).collect();
    assert_eq!(led.len(), 2, "the outline's and the island's walls");
    for ring in &led {
        for &p in &ring.points {
            // Never closer than the tool radius to the island.
            let d = to_outline(p, (30.0, 20.0, 50.0, 40.0));
            let in_island = p.0 > 30.0 && p.0 < 50.0 && p.1 > 20.0 && p.1 < 40.0;
            assert!(!in_island && d >= 3.0 - 1e-6, "{p:?}");
        }
    }
}

/// A star-shaped polygon: `radii.len()` evenly spaced angles, each at its own radius.
fn star(radii: &[f64]) -> Vec<Point> {
    let n = radii.len() as f64;
    radii
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let a = i as f64 / n * std::f64::consts::TAU;
            (100.0 + r * a.cos(), 100.0 + r * a.sin())
        })
        .collect()
}

proptest::proptest! {
    #![proptest_config(common::proptest_config(60))]

    /// With start points anywhere, every cutting move of a profile keeps the tool radius
    /// from the part (docs/specs/helix-and-leads.md §6).
    #[test]
    fn profiles_with_start_points_never_gouge(
        radii in proptest::collection::vec(25.0f64..45.0, 5..12),
        outside in proptest::bool::ANY,
        starts in proptest::collection::vec((40.0f64..160.0, 40.0f64..160.0), 0..2),
    ) {
        let parts = vec![common::polygon(&star(&radii))];
        let side = if outside { "outside" } else { "inside" };
        let starts: Vec<[f64; 2]> = starts.iter().map(|&(x, y)| [x, y]).collect();
        let job = job(profile(side, json!({"starts": starts})));
        let plan = plan_job(&job, &parts).unwrap();
        let walls: Vec<chipload_core::primitives::Primitive> = parts[0]
            .parts
            .iter()
            .map(|p| p.segment.prim)
            .collect();
        let at = positions(&plan.program.moves);
        let limit = 3.0 - chipload_core::tolerances::ARC_APPROX_TOL_MM - 1e-6;
        for (i, m) in plan.program.moves.iter().enumerate().skip(1) {
            if m.kind == MoveKind::Rapid || at[i].2 >= 0.0 {
                continue;
            }
            let (a, b) = ((at[i - 1].0, at[i - 1].1), (at[i].0, at[i].1));
            for wall in &walls {
                let d = chipload_toolpath::geom::segment_distance(a, b, wall);
                proptest::prop_assert!(d >= limit, "move {i} {a:?}-{b:?} is {d} from the part");
            }
        }
    }
}
