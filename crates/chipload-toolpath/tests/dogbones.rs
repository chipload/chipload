// SPDX-License-Identifier: GPL-3.0-or-later
//! Dogbone and T-bone corners (docs/specs/dogbones.md §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::f64::consts::{FRAC_PI_2, PI};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::distance;
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::{Plan, plan_job};
use common::bulged;
use proptest::prelude::*;
use serde_json::{Value, json};

const DEG: f64 = PI / 180.0;
const TOL: f64 = 1e-6;
/// 6 mm tool.
const R: f64 = 3.0;

fn polygon(id: usize, points: &[(f64, f64)]) -> Contour {
    let vertices: Vec<(f64, f64, f64)> = points.iter().map(|&(x, y)| (x, y, 0.0)).collect();
    bulged(id, &vertices)
}

fn rect(id: usize, x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    polygon(id, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

fn job(operations: Value, copies: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 400, "y": 300, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 2.0 * R, "rpm": 18000, "center_cutting": true}],
            "operations": operations,
            "copies": copies,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn corners(kind: &str, max_deg: f64) -> Value {
    json!({"type": kind, "max_angle": max_deg * DEG})
}

fn profile(side: &str, corners: Value, extra: Value) -> Value {
    let mut op = json!({"type": "profile", "select": {"layer": "L"}, "side": side,
        "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 4, "clearance": 0,
        "step_down": 2, "feed": 1000, "plunge_feed": 300, "entry": {"type": "plunge"},
        "corners": corners});
    if let (Value::Object(op), Value::Object(extra)) = (&mut op, extra) {
        op.extend(extra);
    }
    json!([op])
}

fn pocket(corners: Value) -> Value {
    json!([{"type": "pocket", "select": {"layer": "L"}, "tool": 1, "start_depth": 0,
        "end_depth": 2, "clearance": 0, "step_down": 2, "stepover": 0.4,
        "direction": "climb", "feed": 1000, "plunge_feed": 300, "strategy": {"type": "rings"},
        "entry": {"type": "plunge"}, "round_corners": false, "corners": corners}])
}

fn plan(operations: Value, contours: &[Contour]) -> Plan {
    plan_job(&job(operations, Value::Null), contours).unwrap()
}

/// Every XY position the tool goes to and straight back from: the legs' tips, in cut order,
/// once each (a spiral or depth pass repeats them).
fn tips(moves: &[Move]) -> Vec<Point> {
    let mut at: Vec<Point> = Vec::new();
    let mut here = (f64::NAN, f64::NAN);
    for m in moves {
        if m.kind == MoveKind::Rapid {
            here = (m.x.unwrap_or(here.0), m.y.unwrap_or(here.1));
            at.push((f64::NAN, f64::NAN));
            continue;
        }
        if m.x.is_none() && m.y.is_none() {
            continue;
        }
        here = (m.x.unwrap_or(here.0), m.y.unwrap_or(here.1));
        at.push(here);
    }
    let mut found: Vec<Point> = Vec::new();
    for w in at.windows(3) {
        if distance(w[0], w[2]) < TOL
            && distance(w[0], w[1]) > TOL
            && !found.iter().any(|&f| distance(f, w[1]) < TOL)
        {
            found.push(w[1]);
        }
    }
    found
}

fn near(p: Point, q: Point) -> bool {
    distance(p, q) < 1e-3
}

/// A dogbone tip for a 90° corner at `c` whose tool side lies toward `into` (unit-ish).
fn dogbone_tip(c: Point, into: Point, clearance: f64) -> Point {
    let len = into.0.hypot(into.1);
    let b = (into.0 / len, into.1 / len);
    // The corner moved `clearance` into the part, then the tool radius toward the tool.
    let back = clearance / (PI / 4.0).sin();
    (c.0 + b.0 * (R - back), c.1 + b.1 * (R - back))
}

// -- which corners and where (§1, §3)

#[test]
fn a_hole_cut_inside_gets_four_dogbones_on_the_bisectors() {
    let plan = plan(
        profile("inside", corners("dogbone", 100.0), json!({})),
        &[rect(1, 0.0, 0.0, 40.0, 20.0)],
    );
    let found = tips(&plan.program.moves);
    assert_eq!(found.len(), 4, "{found:?}");
    for (c, into) in [
        ((0.0, 0.0), (1.0, 1.0)),
        ((40.0, 0.0), (-1.0, 1.0)),
        ((40.0, 20.0), (-1.0, -1.0)),
        ((0.0, 20.0), (1.0, -1.0)),
    ] {
        let want = dogbone_tip(c, into, 0.0);
        assert!(
            found.iter().any(|&t| near(t, want)),
            "{want:?} in {found:?}"
        );
        assert!(found.iter().any(|&t| (distance(t, c) - R).abs() < TOL));
    }
}

#[test]
fn t_bones_run_along_the_longer_walls() {
    let plan = plan(
        profile("inside", corners("tbone", 100.0), json!({})),
        &[rect(1, 0.0, 0.0, 40.0, 20.0)],
    );
    let found = tips(&plan.program.moves);
    // The tool centre stays 3 mm off the long walls and goes to the short walls' line.
    let want = [(0.0, 3.0), (40.0, 3.0), (40.0, 17.0), (0.0, 17.0)];
    assert_eq!(found.len(), 4, "{found:?}");
    for w in want {
        assert!(found.iter().any(|&t| near(t, w)), "{w:?} in {found:?}");
    }
    // A square: equal walls; each tip still touches its corner.
    let plan = plan_job(
        &job(
            profile("inside", corners("tbone", 100.0), json!({})),
            Value::Null,
        ),
        &[rect(1, 0.0, 0.0, 20.0, 20.0)],
    )
    .unwrap();
    let found = tips(&plan.program.moves);
    assert_eq!(found.len(), 4);
    for c in [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)] {
        assert!(found.iter().any(|&t| (distance(t, c) - R).abs() < TOL));
    }
}

/// An outline with a notch 20 wide and 20 deep in its top edge.
fn notched(width: f64) -> Contour {
    let (a, b) = (70.0 - width / 2.0, 70.0 + width / 2.0);
    polygon(
        1,
        &[
            (10.0, 10.0),
            (130.0, 10.0),
            (130.0, 90.0),
            (b, 90.0),
            (b, 70.0),
            (a, 70.0),
            (a, 90.0),
            (10.0, 90.0),
        ],
    )
}

#[test]
fn an_outline_cut_outside_gets_them_only_in_its_notch() {
    let plan = plan(
        profile("outside", corners("dogbone", 100.0), json!({})),
        &[notched(20.0)],
    );
    let found = tips(&plan.program.moves);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .any(|&t| near(t, dogbone_tip((60.0, 70.0), (1.0, 1.0), 0.0)))
    );
    assert!(
        found
            .iter()
            .any(|&t| near(t, dogbone_tip((80.0, 70.0), (-1.0, 1.0), 0.0)))
    );
}

#[test]
fn the_max_angle_includes_its_own_angle() {
    let contours = [rect(1, 0.0, 0.0, 40.0, 20.0)];
    let at = plan(
        profile("inside", corners("dogbone", 90.0), json!({})),
        &contours,
    );
    assert_eq!(tips(&at.program.moves).len(), 4);
    let under = plan(
        profile("inside", corners("dogbone", 89.9), json!({})),
        &contours,
    );
    assert_eq!(tips(&under.program.moves).len(), 0);
    // Obtuse corners (a hexagon's 120°) are left round at the default.
    let hexagon: Vec<(f64, f64)> = (0..6)
        .map(|k| {
            let a = k as f64 * PI / 3.0;
            (50.0 + 20.0 * a.cos(), 50.0 + 20.0 * a.sin())
        })
        .collect();
    let hex = plan(
        profile("inside", corners("dogbone", 100.0), json!({})),
        &[polygon(1, &hexagon)],
    );
    assert_eq!(tips(&hex.program.moves).len(), 0);
}

#[test]
fn a_corner_between_a_line_and_an_arc_gets_one() {
    // A "D": a straight side from (0, 0) up to (0, 40) and a half circle back. The half
    // circle meets the straight side at 90°.
    let d = bulged(1, &[(0.0, 0.0, 1.0), (0.0, 40.0, 0.0)]);
    let plan = plan(
        profile("inside", corners("dogbone", 100.0), json!({})),
        &[d],
    );
    let found = tips(&plan.program.moves);
    assert_eq!(found.len(), 2, "{found:?}");
    for c in [(0.0, 0.0), (0.0, 40.0)] {
        assert!(
            found.iter().any(|&t| (distance(t, c) - R).abs() < 1e-3),
            "{c:?}: {found:?}"
        );
    }
}

#[test]
fn the_clearance_moves_the_corner() {
    for clearance in [0.2, -0.2] {
        let plan = plan(
            profile(
                "inside",
                corners("dogbone", 100.0),
                json!({"clearance": clearance}),
            ),
            &[rect(1, 0.0, 0.0, 40.0, 20.0)],
        );
        let found = tips(&plan.program.moves);
        let want = dogbone_tip((0.0, 0.0), (1.0, 1.0), clearance);
        assert!(
            found.iter().any(|&t| near(t, want)),
            "clearance {clearance}: {want:?} in {found:?}"
        );
    }
}

// -- out of reach (§5) and checks (§6)

#[test]
fn a_notch_narrower_than_the_tool_is_skipped_with_a_note() {
    let plan = plan(
        profile("outside", corners("dogbone", 100.0), json!({})),
        &[notched(5.0)],
    );
    assert!(tips(&plan.program.moves).is_empty());
    let notes = plan
        .notes
        .iter()
        .map(|n| n.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        notes.contains("corner at (67.500, 70.000): the tool can't reach into it; no dogbone"),
        "{notes}"
    );
}

#[test]
fn a_leg_that_would_cut_into_another_contour_is_refused() {
    // A small circle (not selected) in the part, 0.3 mm beside the hole's wall near its
    // corner: the dogbone's notch reaches it.
    let circle = bulged(2, &[(-0.3, 1.5, 1.0), (-0.9, 1.5, 1.0)]);
    let mut contours = vec![rect(1, 0.0, 0.0, 40.0, 20.0), circle];
    contours[1]
        .parts
        .iter_mut()
        .for_each(|p| p.segment.layer = "other".into());
    let err = plan_job(
        &job(
            profile("inside", corners("dogbone", 100.0), json!({})),
            Value::Null,
        ),
        &contours,
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("the dogbone at corner (0.000, 0.000) would cut into contour #2"),
        "{err}"
    );
}

// -- with the rest (§4)

#[test]
fn tabs_never_sit_on_a_leg() {
    let plan = plan(
        profile(
            "outside",
            corners("dogbone", 100.0),
            json!({"end_depth": 6, "tabs": {"count": 8, "width": 4, "height": 2,
                   "flank_angle": FRAC_PI_2}}),
        ),
        &[notched(20.0)],
    );
    let found = tips(&plan.program.moves);
    assert_eq!(found.len(), 2);
    for tab in &plan.tabs {
        for &p in tab {
            for &t in &found {
                assert!(distance(p, t) > R, "a tab at {p:?} by the tip {t:?}");
            }
        }
    }
}

#[test]
fn a_spiral_cuts_the_legs_going_down() {
    let plan = plan(
        profile(
            "inside",
            corners("dogbone", 100.0),
            json!({"entry": {"type": "ramp", "ramp_angle": 5.0 * DEG}}),
        ),
        &[rect(1, 0.0, 0.0, 40.0, 20.0)],
    );
    let found = tips(&plan.program.moves);
    assert_eq!(found.len(), 4);
    // Each tip is reached at more than one depth.
    for &t in &found {
        let mut here = (f64::NAN, f64::NAN, f64::NAN);
        let mut depths: Vec<i64> = plan
            .program
            .moves
            .iter()
            .filter_map(|m| {
                here = (
                    m.x.unwrap_or(here.0),
                    m.y.unwrap_or(here.1),
                    m.z.unwrap_or(here.2),
                );
                // Depths within a micrometre count as one.
                near((here.0, here.1), t).then_some((here.2 * 1e3).round() as i64)
            })
            .collect();
        depths.sort_unstable();
        depths.dedup();
        assert!(
            depths.len() >= 2,
            "{t:?} cut at {depths:?} while going down"
        );
    }
}

#[test]
fn with_a_finishing_pass_the_roughing_stops_short() {
    let plan = plan(
        profile(
            "inside",
            corners("dogbone", 100.0),
            json!({"finish": {"stock": 0.3}}),
        ),
        &[rect(1, 0.0, 0.0, 40.0, 20.0)],
    );
    let found = tips(&plan.program.moves);
    let wall = dogbone_tip((0.0, 0.0), (1.0, 1.0), 0.0);
    let rough = dogbone_tip((0.0, 0.0), (1.0, 1.0), -0.3);
    assert!(found.iter().any(|&t| near(t, wall)), "{found:?}");
    assert!(found.iter().any(|&t| near(t, rough)), "{found:?}");
}

#[test]
fn a_pocket_gets_them_in_its_wall_rings_and_round_its_islands() {
    // A pocket round an L-shaped island: four corners at the wall, one in the L's inside
    // corner (seen from the pocket).
    let wall = rect(1, 0.0, 0.0, 80.0, 60.0);
    let island = polygon(
        2,
        &[
            (20.0, 20.0),
            (50.0, 20.0),
            (50.0, 30.0),
            (30.0, 30.0),
            (30.0, 40.0),
            (20.0, 40.0),
        ],
    );
    let plan = plan(pocket(corners("dogbone", 100.0)), &[wall, island]);
    // The tips in the wall rings: points reached from a vertex and left back to it.
    let mut found: Vec<Point> = Vec::new();
    for piece in &plan.pockets {
        for ring in &piece.rings {
            if ring.wall {
                let ring = &ring.points;
                let n = ring.len();
                for i in 0..n {
                    let (a, t, b) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
                    if distance(a, b) < TOL && distance(a, t) > TOL {
                        found.push(t);
                    }
                }
            }
        }
    }
    assert_eq!(found.len(), 5, "{found:?}");
    for (c, into) in [
        ((0.0, 0.0), (1.0, 1.0)),
        ((80.0, 0.0), (-1.0, 1.0)),
        ((80.0, 60.0), (-1.0, -1.0)),
        ((0.0, 60.0), (1.0, -1.0)),
        ((30.0, 30.0), (1.0, 1.0)),
    ] {
        let want = dogbone_tip(c, into, 0.0);
        assert!(
            found.iter().any(|&t| near(t, want)),
            "{want:?} in {found:?}"
        );
    }
    // Only wall rings have them, and every move along a wall ring stays off the walls
    // except the legs.
    for piece in &plan.pockets {
        for ring in &piece.rings {
            if !ring.wall {
                assert!(
                    !ring
                        .points
                        .iter()
                        .any(|&p| found.iter().any(|&t| near(p, t)))
                );
            }
        }
    }
    let program = tips(&plan.program.moves);
    for t in &found {
        assert!(program.iter().any(|p| near(*p, *t)), "{t:?} is cut");
    }
}

#[test]
fn crossing_pockets_skip_the_corners_inside_each_other() {
    // Two rectangles overlapping at a corner, each cleared as a piece of its own: each has
    // one corner inside the other, in waste the other clears (no dogbone, and the other's
    // wall doesn't stop the rest); the six other corners get one.
    let first = rect(1, 0.0, 20.0, 60.0, 40.0);
    let second = rect(2, 50.0, 30.0, 70.0, 50.0);
    let plan = plan(pocket(corners("dogbone", 100.0)), &[first, second]);
    let mut wall: Vec<Point> = Vec::new();
    for piece in &plan.pockets {
        for ring in &piece.rings {
            if ring.wall {
                let ring = &ring.points;
                let n = ring.len();
                for i in 0..n {
                    let (a, t, b) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
                    if distance(a, b) < TOL && distance(a, t) > TOL {
                        wall.push(t);
                    }
                }
            }
        }
    }
    assert_eq!(wall.len(), 6, "{wall:?}");
    for c in [
        (0.0, 20.0),
        (60.0, 20.0),
        (70.0, 30.0),
        (70.0, 50.0),
        (50.0, 50.0),
        (0.0, 40.0),
    ] {
        assert!(
            wall.iter().any(|&t| (distance(t, c) - R).abs() < TOL),
            "{c:?}"
        );
    }
}

#[test]
fn every_copy_gets_its_own() {
    let job = job(
        profile("inside", corners("dogbone", 100.0), json!({})),
        json!({"columns": 2, "rows": 1, "dx": 100, "dy": 0}),
    );
    let plan = plan_job(&job, &[rect(1, 0.0, 0.0, 40.0, 20.0)]).unwrap();
    let found = tips(&plan.program.moves);
    assert_eq!(found.len(), 8);
    let first = dogbone_tip((0.0, 0.0), (1.0, 1.0), 0.0);
    assert!(found.iter().any(|&t| near(t, (first.0 + 100.0, first.1))));
}

#[test]
fn without_corners_nothing_changes() {
    let contours = [rect(1, 0.0, 0.0, 40.0, 20.0)];
    let with_null = plan(profile("inside", Value::Null, json!({})), &contours);
    assert!(tips(&with_null.program.moves).is_empty());
}

proptest! {
    #![proptest_config(common::proptest_config(48))]

    /// Random rotated rectangles cut inside: four tips, each the tool radius from its corner.
    #[test]
    fn every_tip_touches_its_corner(
        w in 8.0f64..60.0,
        h in 8.0f64..60.0,
        turn in 0.0f64..PI,
        tbone in any::<bool>(),
    ) {
        let (c, s) = (turn.cos(), turn.sin());
        let at = |x: f64, y: f64| (100.0 + x * c - y * s, 100.0 + x * s + y * c);
        let corners_at = [at(0.0, 0.0), at(w, 0.0), at(w, h), at(0.0, h)];
        let kind = if tbone { "tbone" } else { "dogbone" };
        let plan = plan_job(
            &job(profile("inside", corners(kind, 100.0), json!({})), Value::Null),
            &[polygon(1, &corners_at)],
        )
        .unwrap();
        let found = tips(&plan.program.moves);
        prop_assert_eq!(found.len(), 4);
        for corner in corners_at {
            prop_assert!(found.iter().any(|&t| (distance(t, corner) - R).abs() < 1e-6));
        }
    }
}
