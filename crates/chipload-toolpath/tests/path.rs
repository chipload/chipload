// SPDX-License-Identifier: GPL-3.0-or-later
//! Follow path (docs/specs/follow-path.md §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::{Contour, OrientedSegment, Segment};
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::plan_job;
use proptest::prelude::*;
use serde_json::{Value, json};

const DEG: f64 = std::f64::consts::PI / 180.0;

fn segment(prim: Primitive) -> OrientedSegment {
    OrientedSegment {
        segment: Segment {
            prim,
            layer: "L".into(),
            handle: "1".into(),
            source_type: "LWPOLYLINE".into(),
            shape: None,
            from: None,
        },
        reversed: false,
    }
}

/// An open contour through `(x, y, bulge)` vertices (the last bulge is unused).
fn open(id: usize, vertices: &[(f64, f64, f64)]) -> Contour {
    let parts = vertices
        .windows(2)
        .map(|w| {
            let ((x1, y1, b), (x2, y2, _)) = (w[0], w[1]);
            segment(match Arc::from_bulge((x1, y1), (x2, y2), b) {
                Some(arc) => Primitive::Arc(arc),
                None => Primitive::Line(Line {
                    start: (x1, y1),
                    end: (x2, y2),
                }),
            })
        })
        .collect();
    Contour {
        id,
        parts,
        closed: false,
        invalid_reason: None,
        nesting_depth: None,
    }
}

fn job(operations: Value, tools: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": tools,
            "operations": operations,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn endmill(center_cutting: bool) -> Value {
    json!([{"id": 1, "diameter": 3, "rpm": 18000, "center_cutting": center_cutting}])
}

fn path_op(entry: Value) -> Value {
    json!([{"type": "path", "select": {"layer": "L"}, "tool": 1, "start_depth": 0,
            "end_depth": 3, "step_down": 1, "feed": 1000, "plunge_feed": 300,
            "entry": entry}])
}

/// The position after each move, from (0, 0, 0).
fn trace(moves: &[Move]) -> Vec<(f64, f64, f64, MoveKind)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    moves
        .iter()
        .map(|m| {
            x = m.x.unwrap_or(x);
            y = m.y.unwrap_or(y);
            z = m.z.unwrap_or(z);
            (x, y, z, m.kind)
        })
        .collect()
}

fn line() -> Vec<Contour> {
    vec![open(1, &[(10.0, 10.0, 0.0), (110.0, 10.0, 0.0)])]
}

#[test]
fn a_plunged_open_path_goes_back_and_forth_one_depth_at_a_time() {
    let plan = plan_job(
        &job(path_op(json!({"type": "plunge"})), endmill(true)),
        &line(),
    )
    .unwrap();
    let at = trace(&plan.program.moves);
    let cuts: Vec<(f64, f64, f64)> = at
        .iter()
        .filter(|p| p.3 == MoveKind::Feed)
        .map(|p| (p.0, p.1, p.2))
        .collect();
    assert_eq!(
        cuts,
        [
            (10.0, 10.0, -1.0),
            (110.0, 10.0, -1.0),
            (110.0, 10.0, -2.0),
            (10.0, 10.0, -2.0),
            (10.0, 10.0, -3.0),
            (110.0, 10.0, -3.0),
        ]
    );
    // One retract at the end, none in between: safe Z, the approach height, safe Z.
    let rapids_z: Vec<f64> = plan
        .program
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid)
        .filter_map(|m| m.z)
        .collect();
    assert_eq!(rapids_z, [5.0, 1.0, 5.0]);
}

#[test]
fn a_ramped_open_path_goes_down_as_it_travels_then_levels() {
    let entry = json!({"type": "ramp", "ramp_angle": 3.0 * DEG});
    let plan = plan_job(&job(path_op(entry), endmill(false)), &line()).unwrap();
    let at = trace(&plan.program.moves);
    let feeds: Vec<&(f64, f64, f64, MoveKind)> =
        at.iter().filter(|p| p.3 == MoveKind::Feed).collect();
    // Straight down only to the start depth.
    assert_eq!((feeds[0].0, feeds[0].1, feeds[0].2), (10.0, 10.0, 0.0));
    for w in feeds.windows(2) {
        let run = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        let drop = w[0].2 - w[1].2;
        if drop > 0.0 {
            assert!(run > 0.0, "a plunge to {}", w[1].2);
            assert!(drop / run <= (3.0 * DEG).tan() + 1e-9);
            assert!(drop <= 1.0 + 1e-9, "more than a step in one traversal");
        }
    }
    // 100 mm at 3° goes down 5.2 mm: one traversal per step, then a level one.
    let ends: Vec<(f64, f64)> = feeds.iter().skip(1).map(|p| (p.0, p.2)).collect();
    assert_eq!(
        ends,
        [(110.0, -1.0), (10.0, -2.0), (110.0, -3.0), (10.0, -3.0)]
    );
}

#[test]
fn a_short_path_takes_more_traversals_not_a_steeper_slope() {
    let short = vec![open(1, &[(10.0, 10.0, 0.0), (20.0, 10.0, 0.0)])];
    let entry = json!({"type": "ramp", "ramp_angle": 3.0 * DEG});
    let plan = plan_job(&job(path_op(entry), endmill(false)), &short).unwrap();
    let feeds: Vec<_> = trace(&plan.program.moves)
        .into_iter()
        .filter(|p| p.3 == MoveKind::Feed)
        .collect();
    // 10 mm at 3° goes down 0.52 mm: two traversals per 1 mm step.
    let traversals = feeds.len() - 1;
    assert_eq!(traversals, 3 * 2 + 1);
}

#[test]
fn each_path_starts_at_its_nearer_end() {
    let contours = vec![
        open(1, &[(100.0, 50.0, 0.0), (10.0, 50.0, 0.0)]),
        open(2, &[(200.0, 10.0, 0.0), (120.0, 10.0, 0.0)]),
    ];
    let plan = plan_job(
        &job(path_op(json!({"type": "plunge"})), endmill(true)),
        &contours,
    )
    .unwrap();
    let travels: Vec<(f64, f64)> = trace(&plan.program.moves)
        .into_iter()
        .zip(&plan.program.moves)
        .filter(|(_, m)| m.kind == MoveKind::Rapid && m.x.is_some())
        .map(|(p, _)| (p.0, p.1))
        .collect();
    // From the origin: contour 1 from (10, 50); it ends at (100, 50) after three passes,
    // and contour 2's nearer end from there is (120, 10).
    assert_eq!(travels, [(10.0, 50.0), (120.0, 10.0)]);
}

#[test]
fn a_closed_shape_is_cut_as_a_profile_on_the_line() {
    let square = vec![common::polygon(&[
        (10.0, 10.0),
        (60.0, 10.0),
        (60.0, 40.0),
        (10.0, 40.0),
    ])];
    for entry in [
        json!({"type": "plunge"}),
        json!({"type": "ramp", "ramp_angle": 3.0 * DEG}),
    ] {
        let path = plan_job(&job(path_op(entry.clone()), endmill(true)), &square).unwrap();
        let profile = plan_job(
            &job(
                json!([{"type": "profile", "select": {"layer": "L"}, "side": "on",
                        "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 3,
                        "clearance": 0, "step_down": 1, "feed": 1000, "plunge_feed": 300,
                        "entry": entry}]),
                endmill(true),
            ),
            &square,
        )
        .unwrap();
        assert_eq!(path.program.moves, profile.program.moves, "{entry}");
    }
}

#[test]
fn profiles_still_skip_open_contours_and_follow_paths_take_both() {
    let mut contours = line();
    contours.push(common::polygon(&[
        (10.0, 30.0),
        (60.0, 30.0),
        (60.0, 60.0),
        (10.0, 60.0),
    ]));
    contours[1].id = 2;
    let profile = plan_job(
        &job(
            json!([{"type": "profile", "select": {"layer": "L"}, "side": "outside",
                    "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 3,
                    "clearance": 0, "step_down": 1, "feed": 1000, "plunge_feed": 300}]),
            endmill(true),
        ),
        &contours,
    )
    .unwrap();
    assert!(
        profile
            .notes
            .iter()
            .any(|n| n.contains("skipped open contour #1")),
        "{:?}",
        profile.notes
    );
    let path = plan_job(
        &job(path_op(json!({"type": "plunge"})), endmill(true)),
        &contours,
    )
    .unwrap();
    assert!(path.notes.iter().all(|n| !n.contains("skipped")));
    let cut_y: Vec<f64> = trace(&path.program.moves)
        .into_iter()
        .filter(|p| p.3 == MoveKind::Feed)
        .map(|p| p.1)
        .collect();
    assert!(cut_y.contains(&10.0) && cut_y.contains(&60.0));
}

#[test]
fn tools_a_follow_path_takes() {
    let entry = json!({"type": "plunge"});
    // A drill is refused.
    let mut drill = job(path_op(entry.clone()), endmill(true));
    drill.tools[0].kind = chipload_job::ToolType::Drill;
    let err = plan_job(&drill, &line()).unwrap_err().to_string();
    assert!(err.contains("is a drill"), "{err}");
    // An endmill that can't plunge is refused with a plunge, fine with a ramp.
    let err = plan_job(&job(path_op(entry.clone()), endmill(false)), &line())
        .unwrap_err()
        .to_string();
    assert!(err.contains("not centre-cutting"), "{err}");
    // A V-bit plunges, and the groove's width at the top is reported.
    let vbit = job(
        path_op(entry),
        json!([{"id": 1, "type": "vbit", "diameter": 12.7, "angle": 90.0 * DEG,
                "tip_diameter": 0.0, "rpm": 18000}]),
    );
    let plan = plan_job(&vbit, &line()).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("V-groove is 6 mm wide")),
        "{:?}",
        plan.notes
    );
    // A helix is for pockets.
    let helix = job(
        path_op(json!({"type": "helix", "radius": 1.0, "ramp_angle": 3.0 * DEG})),
        endmill(true),
    );
    let err = plan_job(&helix, &line()).unwrap_err().to_string();
    assert!(err.contains("helix entry is for pockets"), "{err}");
}

#[test]
fn arcs_on_an_open_path_are_written_as_arcs() {
    // A half circle (bulge 1) between two lines.
    let contours = vec![open(
        1,
        &[
            (0.0, 0.0, 0.0),
            (20.0, 0.0, 1.0),
            (40.0, 0.0, 0.0),
            (60.0, 0.0, 0.0),
        ],
    )];
    let plan = plan_job(
        &job(path_op(json!({"type": "plunge"})), endmill(true)),
        &contours,
    )
    .unwrap();
    let tagged = plan
        .program
        .moves
        .iter()
        .filter(|m| m.circle.is_some())
        .count();
    assert!(tagged > 0, "arc chords keep their circle");
}

// -- properties

fn vertices() -> impl Strategy<Value = Vec<(f64, f64, f64)>> {
    prop::collection::vec(
        (
            -80.0f64..80.0,
            -80.0f64..80.0,
            prop_oneof![Just(0.0), -1.0f64..1.0],
        ),
        2..8,
    )
}

proptest! {
    #![proptest_config(common::proptest_config(100))]

    #[test]
    fn every_cut_lies_on_the_path_and_no_traversal_goes_down_more_than_a_step(
        vs in vertices(),
        ramp_deg in 1.0f64..30.0,
        step in 0.5f64..3.0,
        ramped in any::<bool>(),
    ) {
        let contour = open(1, &vs);
        // Along the chords the tool follows (arcs flattened within the arc tolerance).
        let length: f64 = contour
            .points()
            .windows(2)
            .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
            .sum();
        prop_assume!(length > 1.0);
        let entry = if ramped {
            json!({"type": "ramp", "ramp_angle": ramp_deg * DEG})
        } else {
            json!({"type": "plunge"})
        };
        let ops = json!([{"type": "path", "select": {"layer": "L"}, "tool": 1,
            "start_depth": 0, "end_depth": 3.0 * step, "step_down": step, "feed": 1000,
            "plunge_feed": 300, "entry": entry}]);
        let plan = match plan_job(&job(ops, endmill(!ramped)), std::slice::from_ref(&contour)) {
            Ok(plan) => plan,
            Err(err) => {
                prop_assert!(err.to_string().contains("too short"), "{}", err);
                return Ok(());
            }
        };
        let points: Vec<Point> = contour.points();
        // Distance from a point to the drawn polyline (arcs within the arc tolerance).
        let off = |p: (f64, f64)| {
            points
                .windows(2)
                .map(|w| {
                    let (a, b) = (w[0], w[1]);
                    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                    let l2 = dx * dx + dy * dy;
                    let t = if l2 == 0.0 { 0.0 } else {
                        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
                    };
                    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
                })
                .fold(f64::INFINITY, f64::min)
        };
        let at = trace(&plan.program.moves);
        let mut deepest = 0.0f64;
        for (k, p) in at.iter().enumerate() {
            if p.3 != MoveKind::Feed {
                continue;
            }
            prop_assert!(off((p.0, p.1)) < 1e-3, "({}, {}) is off the path", p.0, p.1);
            let before = at[k - 1];
            let run = (p.0 - before.0).hypot(p.1 - before.1);
            let drop = before.2 - p.2;
            if drop > 1e-9 {
                if run == 0.0 {
                    // Straight down: a ramp only to the start depth, a plunge one step at most
                    // below what is cut.
                    if ramped {
                        prop_assert!(p.2 >= -1e-9);
                    } else {
                        prop_assert!(p.2 >= deepest - step - 1e-9);
                    }
                } else {
                    // Descending along the path: at most one step per traversal of it, never
                    // steeper than the ramp.
                    prop_assert!(drop / run <= step / length + 1e-6, "{} over {}", drop, run);
                    prop_assert!(drop / run <= (ramp_deg * DEG).tan() + 1e-9);
                }
            }
            deepest = deepest.min(p.2);
        }
        prop_assert!((deepest + 3.0 * step).abs() < 1e-9);
    }
}
