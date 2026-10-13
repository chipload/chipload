// SPDX-License-Identifier: GPL-3.0-or-later
//! Lead-in and lead-out (docs/specs/helix-and-leads.md part B).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Job, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::{Plan, plan_job};
use serde_json::{Value, json};

const THREE_DEG: f64 = 0.052_359_877_559_829_88;

fn job(operations: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            "drawing": {"shapes": [], "next_shape_id": 1, "source": null},
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
        "step_down": 6, "feed": 1500, "plunge_feed": 500, "lead": {"radius": 3}});
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

/// 100 x 60 mm; the outside tool-centre ring runs from (7, 7) to (113, 73).
fn part() -> Vec<Contour> {
    vec![common::rect(0, 10.0, 10.0, 110.0, 70.0)]
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

/// Where the tool goes straight down while cutting.
fn plunges(plan: &Plan) -> Vec<Point> {
    let moves = &plan.program.moves;
    let at = positions(moves);
    moves
        .iter()
        .enumerate()
        .filter(|(i, m)| {
            m.kind == MoveKind::Feed && m.x.is_none() && m.y.is_none() && *i > 0 && {
                let before = at[i - 1].2;
                m.z.is_some_and(|z| z < before)
            }
        })
        .map(|(i, _)| (at[i].0, at[i].1))
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
fn an_outside_profile_goes_down_in_the_waste_on_every_pass() {
    let plan = plan_job(&job(profile("outside", json!({}))), &part()).unwrap();
    let downs = plunges(&plan);
    assert_eq!(downs.len(), 2, "one per pass: {downs:?}");
    // With a lead the ring starts in the middle of its longest chord: the top edge, cut
    // clockwise (climb, M3) to the right. The lead-in starts 3 mm out and 3 mm back.
    for &p in &downs {
        assert!(
            (p.0 - 57.0).abs() < 1e-9 && (p.1 - 76.0).abs() < 1e-9,
            "{p:?}"
        );
    }
    assert_eq!(plan.ring_starts[0].ring[0], (60.0, 73.0));
    // Every cutting move keeps the tool radius from the part.
    let at = positions(&plan.program.moves);
    for (i, m) in plan.program.moves.iter().enumerate() {
        if m.kind != MoveKind::Rapid && at[i].2 < 0.0 {
            let d = to_outline((at[i].0, at[i].1), (10.0, 10.0, 110.0, 70.0));
            assert!(d >= 3.0 - 1e-9, "move {i} at {:?}: {d}", at[i]);
        }
    }
    // Between the passes the tool retracts: no XY move at depth from the lead-out's end
    // back to the lead-in's start.
    let rapids_to_start = plan
        .program
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid && m.x.is_some())
        .count();
    assert_eq!(rapids_to_start, 2);
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
}

#[test]
fn an_inside_profile_puts_its_leads_inside_the_hole() {
    let plan = plan_job(&job(profile("inside", json!({}))), &part()).unwrap();
    for p in plunges(&plan) {
        assert!(
            p.0 > 13.0 && p.0 < 107.0 && p.1 > 13.0 && p.1 < 67.0,
            "{p:?}"
        );
    }
}

#[test]
fn every_direction_and_spindle_keeps_the_leads_in_the_waste() {
    for (direction, spindle) in [
        ("climb", "cw"),
        ("conventional", "cw"),
        ("climb", "ccw"),
        ("conventional", "ccw"),
    ] {
        for side in ["outside", "inside"] {
            let mut job = job(profile(side, json!({"direction": direction})));
            job.machine.spindle = serde_json::from_value(json!(spindle)).unwrap();
            let plan = plan_job(&job, &part()).unwrap();
            for p in plunges(&plan) {
                let d = to_outline(p, (10.0, 10.0, 110.0, 70.0));
                let outside = !(p.0 > 10.0 && p.0 < 110.0 && p.1 > 10.0 && p.1 < 70.0);
                assert_eq!(
                    outside,
                    side == "outside",
                    "{side} {direction} {spindle}: {p:?}"
                );
                assert!(d > 3.0, "{side} {direction} {spindle}: {p:?} {d}");
            }
        }
    }
}

#[test]
fn a_lead_that_does_not_fit_shrinks_then_goes() {
    // A 10 mm hole: the inside ring is a 4 mm square starting at (5, 3); a lead of radius
    // r begins at (5 - r, 3 + r), which must stay 3 mm from the wall x = 0: r <= 2.
    let hole = vec![common::rect(0, 0.0, 0.0, 10.0, 10.0)];
    let plan = plan_job(&job(profile("inside", json!({}))), &hole).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("lead radius reduced to 1.500 mm")),
        "{:?}",
        plan.notes
    );
    // 6.4 mm: the ring is a 0.4 mm square, not even the 0.6 mm minimum fits.
    let tiny = vec![common::rect(0, 0.0, 0.0, 6.4, 6.4)];
    let plan = plan_job(&job(profile("inside", json!({}))), &tiny).unwrap();
    assert!(
        plan.notes.iter().any(|n| n.contains("no room for a lead")),
        "{:?}",
        plan.notes
    );
    // Straight down on the ring, still started in the middle of its longest chord.
    assert_eq!(plunges(&plan), vec![(3.2, 3.0), (3.2, 3.0)]);
}

#[test]
fn a_neighbouring_contour_makes_the_lead_shrink() {
    // Another part 8 mm below, in the drawing but not selected: the ring (y = 7) keeps 5 mm
    // from its top edge (y = 2), but a lead of radius 3 would reach y = 4, 2 mm from it.
    let mut parts = part();
    parts.push(common::rect(1, 10.0, -20.0, 110.0, 2.0));
    for seg in &mut parts[1].parts {
        seg.segment.layer = "other".into();
    }
    let plan = plan_job(
        &job(profile("outside", json!({"starts": [[60, 7]]}))),
        &parts,
    )
    .unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("contour #0: lead radius reduced to 1.500 mm")),
        "{:?}",
        plan.notes
    );
    // Halved to 1.5 mm: the plunge is 1.5 mm below the ring, 3.5 mm off the neighbour.
    for p in plunges(&plan) {
        assert!((p.1 - 5.5).abs() < 1e-9, "{p:?}");
    }
}

#[test]
fn a_profile_on_the_line_or_a_huge_lead_is_refused() {
    let err = plan_job(&job(profile("on", json!({}))), &part())
        .unwrap_err()
        .to_string();
    assert!(err.contains("no waste side for a lead"), "{err}");
    let err = plan_job(
        &job(profile("outside", json!({"lead": {"radius": 60.1}}))),
        &part(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("lead radius 60.1 is more than 10"), "{err}");
}

#[test]
fn leads_work_with_tabs_and_a_ramp() {
    let mut job = job(profile(
        "outside",
        json!({"tabs": {"count": 4, "width": 8, "height": 3, "flank_angle": THREE_DEG},
               "entry": {"type": "ramp", "ramp_angle": THREE_DEG}}),
    ));
    job.tools[0].center_cutting = Some(false);
    let plan = plan_job(&job, &part()).unwrap();
    assert_eq!(plan.tabs.len(), 4);
    // One lead-in (docs/specs/profile-spiral.md §5): one travel, and the tool stays down
    // from there to the rise-out.
    let travels = plan
        .program
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid && m.x.is_some())
        .count();
    assert_eq!(travels, 1);
    // Every straight descent while cutting only reaches depths already cut, and every
    // sideways descent is within the ramp angle.
    let at = positions(&plan.program.moves);
    let mut deepest: f64 = 0.0;
    for (i, m) in plan.program.moves.iter().enumerate().skip(1) {
        let (before, now) = (at[i - 1], at[i]);
        if m.kind != MoveKind::Rapid && now.2 < before.2 - 1e-9 {
            let run = (now.0 - before.0).hypot(now.1 - before.1);
            if run == 0.0 {
                assert!(now.2 >= deepest - 1e-9, "move {i} plunges to {}", now.2);
            } else {
                assert!((before.2 - now.2) / run <= THREE_DEG.tan() + 1e-9);
            }
        }
        deepest = deepest.min(now.2);
    }
}

#[test]
fn copies_count_the_leads_in_their_size() {
    // Without leads the cut is 72 mm high (60 + 2 × 6). The leads, on the top edge where
    // the ring starts, reach 3 mm higher: the pitch that was enough is now too small.
    let mut plain = job(profile("outside", json!({})));
    plain.operations = serde_json::from_value(json!([{"type": "profile",
        "select": {"layer": "L"}, "side": "outside", "direction": "climb", "tool": 1,
        "start_depth": 0, "end_depth": 12, "clearance": 0, "step_down": 6, "feed": 1500,
        "plunge_feed": 500}]))
    .unwrap();
    plain.copies =
        serde_json::from_value(json!({"columns": 1, "rows": 2, "dx": 0, "dy": 72})).unwrap();
    plan_job(&plain, &part()).unwrap();
    let mut led = job(profile("outside", json!({})));
    led.copies = plain.copies;
    let err = plan_job(&led, &part()).unwrap_err().to_string();
    assert!(err.contains("Copies overlap"), "{err}");
}

#[test]
fn a_pocket_wall_gets_leads() {
    let parts = vec![common::rect(0, 0.0, 0.0, 60.0, 40.0)];
    {
        let strategy = json!({"type": "rings"});
        let plain = plan_job(
            &job(
                json!([{"type": "pocket", "round_corners": false, "select": {"layer": "L"}, "tool": 1,
                "start_depth": 0, "end_depth": 6, "clearance": 0, "step_down": 3,
                "stepover": 0.4, "direction": "climb", "feed": 1500, "plunge_feed": 500,
                "strategy": strategy}]),
            ),
            &parts,
        )
        .unwrap();
        let plan = plan_job(&job(pocket(strategy.clone(), json!({}))), &parts)
            .unwrap_or_else(|e| panic!("{strategy}: {e}"));
        let piece = &plan.pockets[0];
        let k = piece.rings.len() - 1;
        let range = piece.leads[k].clone().expect("the wall has leads");
        assert!(piece.open[k]);
        // The wall ring, starting in the middle of its longest chord (the bottom or the top
        // edge, as long as each other: the one nearer where the clearing ended), with the
        // leads around it; every corner of it is still there.
        let ring = &piece.rings[k][range.clone()];
        assert!(
            ring[0] == (30.0, 3.0) || ring[0] == (30.0, 37.0),
            "{strategy}: {:?}",
            ring[0]
        );
        for corner in &plain.pockets[0].rings[k] {
            assert!(ring.contains(corner), "{strategy}: {corner:?}");
        }
        let path = &piece.rings[k];
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
    let led: Vec<usize> = (0..piece.rings.len())
        .filter(|&k| piece.leads[k].is_some())
        .collect();
    assert_eq!(led.len(), 2, "the outline's and the island's walls");
    for &k in &led {
        for &p in &piece.rings[k] {
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

    /// With leads and start points anywhere, every cutting move of a profile keeps the
    /// tool radius from the part (docs/specs/helix-and-leads.md §6).
    #[test]
    fn profiles_with_leads_never_gouge(
        radii in proptest::collection::vec(25.0f64..45.0, 5..12),
        outside in proptest::bool::ANY,
        lead in 0.5f64..10.0,
        starts in proptest::collection::vec((40.0f64..160.0, 40.0f64..160.0), 0..2),
    ) {
        let parts = vec![common::polygon(&star(&radii))];
        let side = if outside { "outside" } else { "inside" };
        let starts: Vec<[f64; 2]> = starts.iter().map(|&(x, y)| [x, y]).collect();
        let job = job(profile(side, json!({"lead": {"radius": lead}, "starts": starts})));
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
