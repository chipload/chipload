// SPDX-License-Identifier: GPL-3.0-or-later
//! Finishing pass and onion skin on profiles (docs/specs/finish-and-skin.md).
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
const SAFE_Z: f64 = 5.0;

fn job_with(operations: Value, copies: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": SAFE_Z, "spindle": "cw",
                        "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000, "center_cutting": true}],
            "operations": operations,
            "copies": copies,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn job(operations: Value) -> Job {
    job_with(operations, Value::Null)
}

/// An outside profile 12 mm deep in 3 mm steps, with `extra` fields.
fn profile(side: &str, extra: Value) -> Value {
    let mut op = json!({"type": "profile", "select": {"layer": "L"}, "side": side,
        "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 12, "clearance": 0,
        "step_down": 3, "feed": 1500, "plunge_feed": 500});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    json!([op])
}

/// 100 x 60 mm.
const PART: (f64, f64, f64, f64) = (10.0, 10.0, 110.0, 70.0);

fn part() -> Vec<Contour> {
    vec![common::rect(0, PART.0, PART.1, PART.2, PART.3)]
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

/// Least distance from `p` to the rectangle's outline (outside it).
fn to_outline(p: Point) -> f64 {
    let (x0, y0, x1, y1) = PART;
    let dx = (x0 - p.0).max(p.0 - x1).max(0.0);
    let dy = (y0 - p.1).max(p.1 - y1).max(0.0);
    if dx > 0.0 || dy > 0.0 {
        dx.hypot(dy)
    } else {
        -(p.0 - x0).min(x1 - p.0).min(p.1 - y0).min(y1 - p.1)
    }
}

/// A feed move: (from, to), each (x, y, z).
type Feed = ((f64, f64, f64), (f64, f64, f64));

/// One stretch of cutting between retracts: the positions of its feed moves.
#[derive(Debug)]
struct Stretch {
    /// (from, to) of every feed move.
    feeds: Vec<Feed>,
}

impl Stretch {
    fn lowest(&self) -> f64 {
        self.feeds
            .iter()
            .map(|(_, b)| b.2)
            .fold(f64::INFINITY, f64::min)
    }

    /// The distance from the outline of the moves along it at the stretch's lowest Z,
    /// without the first and last move (a step across from or to the roughing slot).
    fn wall_distances(&self) -> Vec<f64> {
        let lowest = self.lowest();
        let inner = &self.feeds[2..self.feeds.len() - 1];
        inner
            .iter()
            .filter(|(a, b)| (a.2 - lowest).abs() < 1e-9 && (b.2 - lowest).abs() < 1e-9)
            .filter(|(a, b)| (a.0, a.1) != (b.0, b.1))
            .flat_map(|(a, b)| [to_outline((a.0, a.1)), to_outline((b.0, b.1))])
            .collect()
    }
}

fn stretches(plan: &Plan) -> Vec<Stretch> {
    let moves = &plan.program.moves;
    let at = positions(moves);
    let mut out = Vec::new();
    let mut current: Vec<Feed> = Vec::new();
    for (i, m) in moves.iter().enumerate() {
        if m.kind == MoveKind::Rapid && m.z == Some(SAFE_Z) {
            if !current.is_empty() {
                out.push(Stretch {
                    feeds: std::mem::take(&mut current),
                });
            }
            continue;
        }
        if m.kind == MoveKind::Feed && i > 0 {
            current.push((at[i - 1], at[i]));
        }
    }
    if !current.is_empty() {
        out.push(Stretch { feeds: current });
    }
    out
}

fn all_near(values: &[f64], target: f64) -> bool {
    !values.is_empty() && values.iter().all(|v| (v - target).abs() < 1e-6)
}

#[test]
fn roughing_then_finishing_then_skin_with_a_plunge() {
    let ops = profile(
        "outside",
        json!({"finish": {"stock": 0.3}, "skin": {"thickness": 0.5}}),
    );
    let plan = plan_job(&job(ops), &part()).unwrap();
    let parts = stretches(&plan);
    assert_eq!(parts.len(), 3, "{parts:#?}");
    // Roughing: passes down to 11.5 (4 passes of 2.875), 0.3 mm farther out than the wall.
    assert!((parts[0].lowest() + 11.5).abs() < 1e-9);
    assert!(all_near(&parts[0].wall_distances(), 3.3));
    let rough_levels: Vec<f64> = parts[0]
        .feeds
        .iter()
        .filter(|(a, b)| a.2 == b.2 && (a.0, a.1) != (b.0, b.1))
        .map(|(a, _)| a.2)
        .fold(Vec::new(), |mut v, z| {
            if !v.contains(&z) {
                v.push(z);
            }
            v
        });
    assert_eq!(rough_levels, [-2.875, -5.75, -8.625, -11.5]);
    // The finishing lap: one level lap at the wall, at the roughing's depth.
    assert!((parts[1].lowest() + 11.5).abs() < 1e-9);
    assert!(all_near(&parts[1].wall_distances(), 3.0));
    // The skin lap: at the wall, at the end depth.
    assert!((parts[2].lowest() + 12.0).abs() < 1e-9);
    assert!(all_near(&parts[2].wall_distances(), 3.0));
    // The finishing and skin laps go down in the roughing slot and step across.
    for part in &parts[1..] {
        let (a, b) = part.feeds[0];
        assert_eq!((a.0, a.1), (b.0, b.1), "first a descent: {part:?}");
        assert!((to_outline((b.0, b.1)) - 3.3).abs() < 0.011);
        assert!((b.2 + 11.5).abs() < 1e-9);
    }
}

#[test]
fn no_descent_onto_uncut_material() {
    for entry in [
        json!({"type": "plunge"}),
        json!({"type": "ramp", "ramp_angle": THREE_DEG}),
    ] {
        let ops = profile(
            "outside",
            json!({"finish": {"stock": 0.3}, "skin": {"thickness": 0.5}, "entry": entry}),
        );
        let plan = plan_job(&job(ops), &part()).unwrap();
        let parts = stretches(&plan);
        for part in &parts[1..] {
            for (a, b) in &part.feeds {
                if (a.0, a.1) != (b.0, b.1) || b.2 >= a.2 {
                    continue;
                }
                let d = to_outline((b.0, b.1));
                // In the roughing slot down to its depth, or at the wall through the skin.
                // (On a corner arc the slot point is on a chord: up to the arc tolerance in.)
                let in_slot = (d - 3.3).abs() < 0.011 && b.2 >= -11.5 - 1e-9;
                let skin = (d - 3.0).abs() < 1e-6 && a.2 >= -11.5 - 1e-9 && b.2 >= -12.0 - 1e-9;
                assert!(in_slot || skin, "descent {a:?} -> {b:?}");
            }
        }
    }
}

#[test]
fn a_ramped_skin_lap_spirals_down_then_cuts_the_whole_ring_level() {
    let ops = profile(
        "outside",
        json!({"skin": {"thickness": 0.5},
               "entry": {"type": "ramp", "ramp_angle": THREE_DEG}}),
    );
    let plan = plan_job(&job(ops), &part()).unwrap();
    let parts = stretches(&plan);
    assert_eq!(parts.len(), 2, "roughing, skin");
    let skin = &parts[1];
    // Without a finishing pass the lap goes down at the wall, at the roughing's depth.
    let (a, b) = skin.feeds[0];
    assert_eq!((a.0, a.1), (b.0, b.1));
    assert!((b.2 + 11.5).abs() < 1e-9);
    // Never steeper than the ramp, never straight down below the roughing.
    for (a, b) in &skin.feeds[1..] {
        let run = (b.0 - a.0).hypot(b.1 - a.1);
        assert!(run > 0.0, "no plunge: {a:?} -> {b:?}");
        assert!((a.2 - b.2) <= run * THREE_DEG.tan() + 1e-9);
    }
    // At the end depth for one whole lap of the 212 mm ring (plus its corner arcs).
    let level: f64 = skin
        .feeds
        .iter()
        .filter(|(a, b)| a.2 == -12.0 && b.2 == -12.0)
        .map(|(a, b)| (b.0 - a.0).hypot(b.1 - a.1))
        .sum();
    let ring = 2.0 * (100.0 + 60.0) + std::f64::consts::TAU * 3.0;
    assert!(
        level > ring - 0.05 && level < ring + 1e-6,
        "{level} vs {ring}"
    );
}

/// The middle of each tab on the wall ring.
fn tab_middles(plan: &Plan) -> Vec<Point> {
    plan.tabs
        .iter()
        .map(|line| {
            let total: f64 = line
                .windows(2)
                .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
                .sum();
            let mut left = total / 2.0;
            for w in line.windows(2) {
                let seg = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
                if left <= seg {
                    let t = left / seg;
                    return (
                        w[0].0 + (w[1].0 - w[0].0) * t,
                        w[0].1 + (w[1].1 - w[0].1) * t,
                    );
                }
                left -= seg;
            }
            line[line.len() - 1]
        })
        .collect()
}

#[test]
fn every_lap_leaves_the_tabs() {
    for flank in [std::f64::consts::FRAC_PI_2, 30f64.to_radians()] {
        for entry in [
            json!({"type": "plunge"}),
            json!({"type": "ramp", "ramp_angle": THREE_DEG}),
        ] {
            let ops = profile(
                "outside",
                json!({"finish": {"stock": 0.3}, "skin": {"thickness": 0.5}, "entry": entry,
                       "tabs": {"count": 4, "width": 6, "height": 2, "flank_angle": flank}}),
            );
            let plan = plan_job(&job(ops), &part()).unwrap();
            let middles = tab_middles(&plan);
            assert_eq!(middles.len(), 4);
            let z_tab = -12.0 + 2.0;
            let moves = &plan.program.moves;
            let at = positions(moves);
            let mut passed = vec![0; middles.len()];
            for (i, m) in moves.iter().enumerate().skip(1) {
                if m.kind != MoveKind::Feed {
                    continue;
                }
                let (a, b) = (at[i - 1], at[i]);
                for (k, &mid) in middles.iter().enumerate() {
                    // The point of the move nearest the tab's middle, and its Z there.
                    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                    let len2 = dx * dx + dy * dy;
                    let t = if len2 > 0.0 {
                        (((mid.0 - a.0) * dx + (mid.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let p = (a.0 + dx * t, a.1 + dy * t);
                    if (p.0 - mid.0).hypot(p.1 - mid.1) > 0.3 + 1e-6 {
                        continue;
                    }
                    passed[k] += 1;
                    let z = a.2 + (b.2 - a.2) * t;
                    assert!(
                        z >= z_tab - 1e-6,
                        "flank {flank}, {entry}: Z{z} over tab {k} at {p:?}"
                    );
                }
            }
            // Every tab is passed by the roughing, finishing and skin laps.
            assert!(passed.iter().all(|&n| n >= 3), "{passed:?}");
        }
    }
}

#[test]
fn the_finishing_and_skin_laps_start_between_tabs() {
    let ops = profile(
        "outside",
        json!({"finish": {"stock": 0.3}, "skin": {"thickness": 0.5},
               "tabs": {"count": 4, "width": 6, "height": 2,
                        "flank_angle": std::f64::consts::FRAC_PI_2}}),
    );
    let plan = plan_job(&job(ops), &part()).unwrap();
    let parts = stretches(&plan);
    for part in &parts[1..] {
        // Straight down in the slot, all the way to the lap's depth: no tab there.
        let (_, b) = part.feeds[0];
        assert!((b.2 + 11.5).abs() < 1e-9, "{b:?}");
    }
}

fn refused(extra: Value, side: &str, words: &str) {
    let err = match parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000, "center_cutting": true}],
            "operations": profile(side, extra),
        }))
        .unwrap(),
        "test.json",
    ) {
        Ok(job) => plan_job(&job, &part()).unwrap_err().to_string(),
        Err(err) => err.to_string(),
    };
    assert!(err.contains(words), "{err}");
}

#[test]
fn the_rules_at_their_boundaries() {
    // A skin as thick as the step down is fine; thicker is refused.
    let ops = profile("outside", json!({"skin": {"thickness": 3.0}}));
    plan_job(&job(ops), &part()).unwrap();
    refused(
        json!({"skin": {"thickness": 3.01}}),
        "outside",
        "at most the step down",
    );
    // A skin as thick as the tabs are high is refused.
    refused(
        json!({"skin": {"thickness": 2.0},
               "tabs": {"count": 2, "width": 6, "height": 2, "flank_angle": 1.0}}),
        "outside",
        "less than the tab height",
    );
    refused(
        json!({"skin": {"thickness": 0}}),
        "outside",
        "greater than 0",
    );
    // Finishing stock: just under the tool radius plans; the radius is refused.
    let ops = profile("outside", json!({"finish": {"stock": 2.99}}));
    plan_job(&job(ops), &part()).unwrap();
    refused(
        json!({"finish": {"stock": 3.0}}),
        "outside",
        "less than the tool radius",
    );
    // A profile on the line has no wall to finish; a skin is fine.
    refused(json!({"finish": {"stock": 0.3}}), "on", "no wall to finish");
    let ops = profile("on", json!({"skin": {"thickness": 1}}));
    plan_job(&job(ops), &part()).unwrap();
}

#[test]
fn a_finishing_lap_the_roughing_cant_reach_is_refused() {
    // A 6.4 mm hole for a 6 mm tool: the wall ring is 0.2 mm round, the roughing ring
    // 0.3 mm farther in doesn't exist.
    let hole = vec![common::circle((50.0, 50.0), 3.2)];
    let ops = profile("inside", json!({"finish": {"stock": 0.3}}));
    let err = plan_job(&job(ops), &hole).unwrap_err().to_string();
    assert!(err.contains("contour #"), "{err}");
}

#[test]
fn each_copy_is_finished_and_skinned_before_the_next_is_cut() {
    let ops = profile(
        "outside",
        json!({"finish": {"stock": 0.3}, "skin": {"thickness": 0.5}}),
    );
    let grid = json!({"columns": 2, "rows": 1, "dx": 120, "dy": 0, "order": "by_operation"});
    let plan = plan_job(&job_with(ops, grid), &part()).unwrap();
    let parts = stretches(&plan);
    assert_eq!(parts.len(), 6);
    let depths: Vec<f64> = parts.iter().map(Stretch::lowest).collect();
    assert_eq!(depths, [-11.5, -11.5, -12.0, -11.5, -11.5, -12.0]);
}

#[test]
fn without_finish_or_skin_nothing_changes() {
    let plain = plan_job(&job(profile("outside", json!({}))), &part()).unwrap();
    let nulls = plan_job(
        &job(profile("outside", json!({"finish": null, "skin": null}))),
        &part(),
    )
    .unwrap();
    assert_eq!(plain.program.moves, nulls.program.moves);
    assert_eq!(stretches(&plain).len(), 1);
}
