// SPDX-License-Identifier: GPL-3.0-or-later
//! A pocket's finishing pass (docs/specs/pocket-finish.md): the roughing leaves the
//! allowances, then a floor pass and a lap of each wall take them, and nothing goes past the
//! finished wall.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::math;
mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::plan_job;
use serde_json::{Value, json};

const TOL: f64 = 0.01;
const RADIUS: f64 = 3.0;

fn job(extra: Value) -> Job {
    let mut op = json!({"type": "pocket", "select": {"layer": "L"}, "tool": 1,
        "start_depth": 0, "end_depth": 6, "clearance": 0, "step_down": 3, "stepover": 0.4,
        "direction": "climb", "feed": 1500, "plunge_feed": 500, "strategy": {"type": "rings"}});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    parse_job(
        migrate(json!({
            "schema_version": 21,
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000, "center_cutting": true}],
            "operations": [op],
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

/// A 60 × 40 pocket with a 10 × 10 island in the middle.
fn with_island() -> Vec<Contour> {
    vec![
        common::rect(0, 0.0, 0.0, 60.0, 40.0),
        common::rect(1, 25.0, 15.0, 35.0, 25.0),
    ]
}

/// The tool centre's distance to the nearest wall: the outline or the island.
fn to_walls(p: Point) -> f64 {
    let outline = (p.0).min(60.0 - p.0).min(p.1).min(40.0 - p.1);
    let dx = (25.0 - p.0).max(p.0 - 35.0).max(0.0);
    let dy = (15.0 - p.1).max(p.1 - 25.0).max(0.0);
    outline.min(math::hypot(dx, dy))
}

/// Every cutting move's end and middle along X and Y, with its Z, below the stock top (the
/// plunges straight down are left out).
fn cutting(moves: &[Move]) -> Vec<(Point, f64)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, f64::INFINITY);
    let mut out = Vec::new();
    for m in moves {
        let from = (x, y);
        (x, y, z) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        let travels = m.x.is_some() || m.y.is_some();
        if m.kind != MoveKind::Rapid && travels && z < -TOL {
            out.push(((x, y), z));
            out.push((((from.0 + x) / 2.0, (from.1 + y) / 2.0), z));
        }
    }
    out
}

fn moves_of(extra: Value, contours: &[Contour]) -> Vec<Move> {
    let plan = plan_job(&job(extra), contours).unwrap_or_else(|e| panic!("{e}"));
    plan.program.moves
}

fn depths(cuts: &[(Point, f64)]) -> Vec<f64> {
    let mut zs: Vec<f64> = cuts
        .iter()
        .map(|(_, z)| (z * 1000.0).round() / 1000.0)
        .collect();
    zs.sort_by(|a, b| b.total_cmp(a));
    zs.dedup();
    zs
}

#[test]
fn both_allowances_rough_then_finish_floor_and_every_wall() {
    let finish = json!({"finish": {"wall": 0.3, "floor": 0.2}});
    let cuts = cutting(&moves_of(finish, &with_island()));
    // Roughing to 5.8 in two equal passes (lifted 0.5 off the first to go back), then the
    // floor and the walls at 6.
    assert_eq!(depths(&cuts), [-2.4, -2.9, -5.8, -6.0]);
    for &(p, z) in &cuts {
        let d = to_walls(p);
        assert!(
            d >= RADIUS - TOL,
            "past the finished wall at {p:?} z {z}: {d}"
        );
        if z > -6.0 + TOL {
            assert!(d >= RADIUS + 0.3 - TOL, "the roughing at {p:?} z {z}: {d}");
        }
    }
    // The laps run on the finished walls, outline and island alike.
    let at_wall = |island: bool| {
        cuts.iter().any(|&(p, z)| {
            let on_island = (20.0..=40.0).contains(&p.0) && (10.0..=30.0).contains(&p.1);
            z <= -6.0 + TOL && on_island == island && (to_walls(p) - RADIUS).abs() < TOL
        })
    };
    assert!(at_wall(false), "a lap on the outline");
    assert!(at_wall(true), "a lap round the island");
}

#[test]
fn the_floor_alone_leaves_the_walls_where_they_are() {
    let cuts = cutting(&moves_of(
        json!({"finish": {"wall": 0, "floor": 0.5}}),
        &with_island(),
    ));
    assert_eq!(depths(&cuts), [-2.25, -2.75, -5.5, -6.0]);
    assert!(cuts.iter().all(|&(p, _)| to_walls(p) >= RADIUS - TOL));
    // The floor pass is the roughing's rings once more: up to the wall.
    assert!(
        cuts.iter()
            .any(|&(p, z)| z <= -6.0 + TOL && (to_walls(p) - RADIUS).abs() < TOL)
    );
}

#[test]
fn the_walls_alone_rough_to_the_end_depth() {
    let cuts = cutting(&moves_of(
        json!({"finish": {"wall": 0.5, "floor": 0}}),
        &with_island(),
    ));
    assert_eq!(depths(&cuts), [-2.5, -3.0, -6.0]);
    // The first pass is all roughing (at 6 the laps cross the allowance onto the wall).
    let roughing_nearest = cuts
        .iter()
        .filter(|(_, z)| *z > -6.0 + TOL)
        .map(|(p, _)| to_walls(*p))
        .fold(f64::INFINITY, f64::min);
    assert!(roughing_nearest >= RADIUS + 0.5 - TOL, "{roughing_nearest}");
}

#[test]
fn a_lap_with_leads_goes_down_in_the_roughed_area() {
    let moves = moves_of(
        json!({"finish": {"wall": 0.3, "floor": 0.2}, "lead": {"radius": 3}}),
        &with_island(),
    );
    let cuts = cutting(&moves);
    assert!(cuts.iter().all(|&(p, _)| to_walls(p) >= RADIUS - TOL));
    // Every plunge to the end depth happens at least the allowance in from the wall.
    let (mut x, mut y) = (0.0, 0.0);
    for m in &moves {
        (x, y) = (m.x.unwrap_or(x), m.y.unwrap_or(y));
        if m.x.is_none() && m.y.is_none() && m.z.is_some_and(|z| z <= -6.0 + TOL) {
            assert!(
                to_walls((x, y)) >= RADIUS + 0.3 - TOL,
                "plunge at ({x}, {y})"
            );
        }
    }
}

/// An island leaving a gap of `gap` mm below and above it, 10 mm at the sides.
fn necks(gap: f64) -> Vec<Contour> {
    vec![
        common::rect(0, 0.0, 0.0, 60.0, 40.0),
        common::rect(1, 10.0, gap, 50.0, 40.0 - gap),
    ]
}

#[test]
fn too_narrow_to_rough_is_refused_and_just_wide_enough_is_cut() {
    let finish = json!({"finish": {"wall": 0.5, "floor": 0}});
    let err = plan_job(&job(finish.clone()), &necks(6.6))
        .unwrap_err()
        .to_string();
    assert!(err.contains("too narrow near"), "{err}");
    assert!(err.contains("0.5 mm left on the walls"), "{err}");
    plan_job(&job(finish), &necks(7.2)).unwrap();
}

#[test]
fn climb_and_conventional_both_finish_without_gouging() {
    for direction in ["climb", "conventional"] {
        let cuts = cutting(&moves_of(
            json!({"finish": {"wall": 0.3, "floor": 0.2}, "direction": direction}),
            &with_island(),
        ));
        assert!(
            cuts.iter().all(|&(p, _)| to_walls(p) >= RADIUS - TOL),
            "{direction}"
        );
    }
}

#[test]
fn the_rules_are_checked_at_their_fields() {
    let problems = |extra: Value| -> Vec<String> {
        let mut job = job(json!({}));
        let chipload_job::Operation::Pocket(p) = &mut job.operations[0] else {
            panic!()
        };
        let extra: chipload_job::PocketOp = {
            let mut v = serde_json::to_value(&*p).unwrap();
            for (k, val) in extra.as_object().unwrap() {
                v[k] = val.clone();
            }
            serde_json::from_value(v).unwrap()
        };
        *p = extra;
        let mut out: Vec<String> = job
            .validate()
            .into_iter()
            .map(|p| format!("{}: {}", p.path, p.short))
            .collect();
        out.extend(
            chipload_toolpath::plan::job_problems(&job)
                .into_iter()
                .map(|p| format!("{}: {}", p.path, p.short)),
        );
        out
    };
    let has = |extra: Value, want: &str| {
        let got = problems(extra.clone());
        assert!(got.iter().any(|p| p.contains(want)), "{extra}: {got:?}");
    };
    has(
        json!({"finish": {"wall": 0, "floor": 0}}),
        "operations.0.finish: leaves nothing",
    );
    has(
        json!({"finish": {"wall": -1, "floor": 0}}),
        "finish.wall: must be 0 or more",
    );
    has(
        json!({"finish": {"wall": 3, "floor": 0}}),
        "finish.wall: finishing stock 3 mm",
    );
    has(
        json!({"finish": {"wall": 0, "floor": 3.5}}),
        "finish.floor: must be at most the step down",
    );
    has(
        json!({"finish": {"wall": 0, "floor": 0.2}, "through": 0.2, "end_depth": 18.2}),
        "finish.floor: a pocket cut through",
    );
    assert!(problems(json!({"finish": {"wall": 0.3, "floor": 0.2}})).is_empty());
}

#[test]
fn dogbones_are_roughed_short_and_reached_by_the_lap() {
    let outline = [common::rect(0, 0.0, 0.0, 60.0, 40.0)];
    let corners = json!({"type": "dogbone", "max_angle": 100f64.to_radians()});
    let cuts = cutting(&moves_of(
        json!({"finish": {"wall": 0.3, "floor": 0.2}, "corners": corners}),
        &outline,
    ));
    let to_corner = |p: Point, z_ok: &dyn Fn(f64) -> bool| {
        cuts.iter()
            .filter(|(_, z)| z_ok(*z))
            .map(|(q, _)| math::hypot(q.0 - p.0, q.1 - p.1))
            .fold(f64::INFINITY, f64::min)
    };
    for corner in [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)] {
        // The lap at the end depth reaches into the corner: the tool's edge on it.
        let lap = to_corner(corner, &|z| z <= -6.0 + TOL);
        assert!((lap - RADIUS).abs() < 0.02, "{corner:?}: {lap}");
        // The roughing's bone stops the allowance short.
        let rough = to_corner(corner, &|z| z > -6.0 + TOL);
        assert!(rough >= RADIUS + 0.3 - 0.02, "{corner:?}: {rough}");
    }
    // Nothing past the walls.
    for &(p, z) in &cuts {
        let d = (p.0).min(60.0 - p.0).min(p.1).min(40.0 - p.1);
        assert!(d >= -TOL, "outside at {p:?} z {z}");
        let corner = [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)]
            .iter()
            .map(|c| math::hypot(p.0 - c.0, p.1 - c.1))
            .fold(f64::INFINITY, f64::min);
        assert!(corner >= RADIUS - 0.02, "past a corner at {p:?}");
    }
}

#[test]
fn a_rest_pocket_takes_no_finishing_pass() {
    let finish = json!({"finish": {"wall": 0.3, "floor": 0.2}});
    let mut file = serde_json::to_value(job(finish)).unwrap();
    file["operations"][0]["rest"] = json!({"after_tool": 1});
    let err = parse_job(file, "test.json").unwrap_err().to_string();
    assert!(err.contains("operations.0.finish: a rest pocket"), "{err}");
}
