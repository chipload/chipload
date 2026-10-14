// SPDX-License-Identifier: GPL-3.0-or-later
//! Start points chosen by the user (docs/specs/helix-and-leads.md part A).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::model::MoveKind;
use chipload_toolpath::{Plan, plan_job};
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

fn profile(side: &str, starts: Value, tabs: Value) -> Value {
    json!([{"type": "profile", "select": {"layer": "L"}, "side": side, "direction": "climb",
            "tool": 1, "start_depth": 0, "end_depth": 12, "clearance": 0, "step_down": 6,
            "feed": 1500, "plunge_feed": 500, "tabs": tabs, "starts": starts}])
}

fn pocket(strategy: Value, starts: Value) -> Value {
    json!([{"type": "pocket", "round_corners": false, "select": {"layer": "L"}, "tool": 1, "start_depth": 0,
            "end_depth": 6, "clearance": 0, "step_down": 3, "stepover": 0.4,
            "direction": "climb", "feed": 1500, "plunge_feed": 500, "strategy": strategy,
            "starts": starts}])
}

/// 100 x 60 mm; the outside tool-centre ring runs from (7, 7) to (113, 73).
fn part() -> Vec<Contour> {
    vec![common::rect(0, 10.0, 10.0, 110.0, 70.0)]
}

/// Where each rapid in XY goes, in order.
fn rapids(plan: &Plan) -> Vec<Point> {
    plan.program
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid)
        .filter_map(|m| m.x.zip(m.y))
        .collect()
}

fn close(a: Point, b: Point) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
}

#[test]
fn a_profile_ring_starts_at_the_projection_of_its_point() {
    let plain = plan_job(&job(profile("outside", json!([]), Value::Null)), &part()).unwrap();
    assert_eq!(plain.ring_starts.len(), 1);
    assert_eq!(plain.ring_starts[0].stored, None);

    let plan = plan_job(
        &job(profile("outside", json!([[60, 76]]), Value::Null)),
        &part(),
    )
    .unwrap();
    assert!(close(rapids(&plan)[0], (60.0, 73.0)), "{:?}", rapids(&plan));
    let start = &plan.ring_starts[0];
    assert_eq!(start.stored, Some(0));
    assert!(close(start.ring[0], (60.0, 73.0)));
    // Climb outside with M3 is clockwise: from the top edge the ring goes on to the right.
    assert!(start.ring[1].0 > 60.0 && (start.ring[1].1 - 73.0).abs() < 1e-9);
    // The same ring, the same length of cut: only where it starts differs.
    assert!(plan.notes.iter().all(|n| !n.contains("start point")));
}

#[test]
fn a_point_far_from_its_ring_gives_a_note_and_still_moves_the_start() {
    let plan = plan_job(
        &job(profile("outside", json!([[60, 90]]), Value::Null)),
        &part(),
    )
    .unwrap();
    assert!(close(rapids(&plan)[0], (60.0, 73.0)));
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("start point (60.000, 90.000) is 17.000 mm from contour #0")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn each_point_takes_the_nearest_ring_and_a_second_one_is_ignored() {
    let parts = vec![
        common::rect(0, 10.0, 10.0, 50.0, 50.0),
        common::rect(1, 100.0, 10.0, 140.0, 50.0),
    ];
    let starts = json!([[150, 30], [30, 5], [145, 40]]);
    let plan = plan_job(&job(profile("outside", starts, Value::Null)), &parts).unwrap();
    let by_stored: Vec<(Option<usize>, Point)> = plan
        .ring_starts
        .iter()
        .map(|s| (s.stored, s.ring[0]))
        .collect();
    assert!(by_stored.contains(&(Some(1), (30.0, 7.0))), "{by_stored:?}");
    assert!(
        by_stored.contains(&(Some(0), (143.0, 30.0))),
        "{by_stored:?}"
    );
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("(145.000, 40.000) is on contour #1 too")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn tabs_move_with_the_start() {
    let tabs =
        json!({"count": 2, "width": 8, "height": 3, "flank_angle": std::f64::consts::FRAC_PI_2});
    let plain = plan_job(&job(profile("outside", json!([]), tabs.clone())), &part()).unwrap();
    let moved = plan_job(&job(profile("outside", json!([[113, 40]]), tabs)), &part()).unwrap();
    assert_eq!(plain.tabs.len(), 2);
    assert_eq!(moved.tabs.len(), 2);
    assert_ne!(plain.tabs, moved.tabs);
    // Tabs are spaced from the start: neither covers it.
    for tab in &moved.tabs {
        assert!(tab.iter().all(|&p| !close(p, (113.0, 40.0))));
    }
}

#[test]
fn copies_use_the_point_moved_with_them() {
    let mut job = job(profile("outside", json!([[60, 76]]), Value::Null));
    job.parts[0].copies =
        serde_json::from_value(json!({"columns": 2, "rows": 1, "dx": 150, "dy": 0})).unwrap();
    let plan = plan_job(&job, &part()).unwrap();
    let rapids = rapids(&plan);
    assert!(rapids.iter().any(|&p| close(p, (60.0, 73.0))), "{rapids:?}");
    assert!(
        rapids.iter().any(|&p| close(p, (210.0, 73.0))),
        "{rapids:?}"
    );
    // The markers are for the part only.
    assert_eq!(plan.ring_starts.len(), 1);
}

#[test]
fn a_start_on_an_arc_lands_on_the_arc() {
    let parts = vec![common::circle_id(0, (50.0, 50.0), 20.0)];
    let plan = plan_job(
        &job(profile("outside", json!([[90, 90]]), Value::Null)),
        &parts,
    )
    .unwrap();
    let p = plan.ring_starts[0].ring[0];
    let r = (p.0 - 50.0).hypot(p.1 - 50.0);
    assert!((r - 23.0).abs() < 1e-9, "radius {r}");
    assert!((p.0 - p.1).abs() < 1e-6, "{p:?} not on the diagonal");
}

#[test]
fn a_pocket_moves_only_its_wall_ring() {
    let parts = vec![common::rect(0, 0.0, 0.0, 60.0, 40.0)];
    {
        let strategy = json!({"type": "rings"});
        let plain = plan_job(&job(pocket(strategy.clone(), json!([]))), &parts).unwrap();
        let plan = plan_job(&job(pocket(strategy.clone(), json!([[30, 45]]))), &parts)
            .unwrap_or_else(|e| panic!("{strategy}: {e}"));
        let (a, b) = (&plain.pockets[0], &plan.pockets[0]);
        assert_eq!(a.rings.len(), b.rings.len(), "{strategy}");
        let walls: Vec<usize> = (0..b.rings.len()).filter(|&k| b.rings[k].wall).collect();
        assert_eq!(walls, vec![b.rings.len() - 1], "{strategy}");
        for k in 0..b.rings.len() - 1 {
            assert_eq!(a.rings[k], b.rings[k], "{strategy}: ring {k} changed");
        }
        let wall = b.rings.last().unwrap();
        assert!(
            close(wall.points[0], (30.0, 37.0)),
            "{strategy}: {:?}",
            wall.points[0]
        );
        assert_eq!(wall.chosen, Some(0));
        assert_eq!(plan.ring_starts.len(), 1, "{strategy}");
    }
}

#[test]
fn island_walls_can_be_moved_too() {
    let parts = vec![
        common::rect(0, 0.0, 0.0, 80.0, 60.0),
        common::rect(1, 30.0, 20.0, 50.0, 40.0),
    ];
    let plan = plan_job(
        &job(pocket(json!({"type": "rings"}), json!([[40, 18]]))),
        &parts,
    )
    .unwrap();
    let piece = &plan.pockets[0];
    let island = piece
        .rings
        .iter()
        .find(|r| r.chosen == Some(0))
        .expect("a wall ring took the point");
    assert!(island.wall);
    // The island's ring runs 3 mm outside it: its bottom edge is at y = 17.
    assert!(close(island.points[0], (40.0, 17.0)));
}
