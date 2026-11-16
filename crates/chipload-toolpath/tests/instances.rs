// SPDX-License-Identifier: GPL-3.0-or-later
//! Turned and mirrored instances of a part (docs/specs/instances.md §3): each cuts the
//! part's toolpath put in its place, climb staying climb, and none may cut into another.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_job::migrations::migrate;
use chipload_job::{Instance, Job, Mirror, Spot, parse_job};
use chipload_toolpath::model::MoveKind;
use chipload_toolpath::place::Placement;
use chipload_toolpath::{Plan, plan_job};
use proptest::prelude::*;
use serde_json::{Value, json};

/// A big stock, so instances can go anywhere; the drawing is empty (plan_job gets its
/// contours directly), so the part's centre, what instances turn about, is (0, 0).
fn job(operations: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": 21,
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 1000, "y": 1000, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": operations,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn profile(side: &str, tabs: Value) -> Value {
    json!({"type": "profile", "select": {"layer": "L"}, "side": side, "direction": "climb",
           "tool": 1, "start_depth": 0, "end_depth": 12, "clearance": 0, "step_down": 6,
           "feed": 1500, "plunge_feed": 500, "tabs": tabs})
}

fn pocket() -> Value {
    json!({"type": "pocket", "select": {"layer": "L"}, "tool": 1, "start_depth": 0,
           "direction": "climb",
           "end_depth": 3, "step_down": 3, "stepover": 0.4, "feed": 1500, "plunge_feed": 500,
           "clearance": 0})
}

/// An L-shaped part with a round corner: not symmetric, so a mirror or a turn shows.
fn part() -> Vec<Contour> {
    vec![common::bulged(
        1,
        &[
            (10.0, 10.0, 0.0),
            (110.0, 10.0, 0.0),
            (110.0, 40.0, 0.0),
            (60.0, 40.0, 0.4),
            (40.0, 70.0, 0.0),
            (10.0, 70.0, 0.0),
        ],
    )]
}

fn with_instance(ops: Value, spot: Spot) -> Job {
    let mut job = job(ops);
    job.parts[0].instances = vec![Instance {
        id: 2,
        enabled: true,
        mirror: spot.mirror,
        angle: spot.angle,
        at: spot.at,
    }];
    job
}

/// Every feed move's end in `copy`, its X and Y filled in from the moves before.
fn feed_ends(plan: &Plan, copy: u32) -> Vec<Point> {
    feed_ends_of(plan, copy, None)
}

/// [`feed_ends`] of one operation, or of all with None.
fn feed_ends_of(plan: &Plan, copy: u32, operation: Option<usize>) -> Vec<Point> {
    let mut at = (0.0, 0.0);
    let mut ends = Vec::new();
    for span in &plan.op_spans {
        let counted = span.copy == copy && operation.is_none_or(|o| o == span.operation);
        for (k, m) in plan.program.moves.iter().enumerate() {
            if span.moves.contains(&k) {
                at = (m.x.unwrap_or(at.0), m.y.unwrap_or(at.1));
                if counted && m.kind != MoveKind::Rapid {
                    ends.push(at);
                }
            }
        }
    }
    ends
}

/// Twice the signed area the feed moves of `copy` sweep about their own middle: positive
/// counter-clockwise. About the middle, so the gaps between laps count for little.
fn swept_area(plan: &Plan, copy: u32) -> f64 {
    let ends = feed_ends(plan, copy);
    let n = ends.len() as f64;
    let c = (
        ends.iter().map(|p| p.0).sum::<f64>() / n,
        ends.iter().map(|p| p.1).sum::<f64>() / n,
    );
    ends.windows(2)
        .map(|w| (w[0].0 - c.0) * (w[1].1 - c.1) - (w[1].0 - c.0) * (w[0].1 - c.1))
        .sum()
}

fn near(p: Point, among: &[Point], tol: f64) -> bool {
    among
        .iter()
        .any(|q| (p.0 - q.0).abs() <= tol && (p.1 - q.1).abs() <= tol)
}

fn mirror_of(k: u8) -> Option<Mirror> {
    match k {
        0 => None,
        1 => Some(Mirror::LeftRight),
        _ => Some(Mirror::TopBottom),
    }
}

proptest! {
    #![proptest_config(common::proptest_config(24))]

    /// An instance cuts the part's toolpath put in its place. Turned and moved, its profile
    /// is the part's: every feed of instance 2, brought back, is one of the part's, and
    /// there are as many. Mirrored, it is planned the other way round (climb stays climb),
    /// so its ramps and tabs lie elsewhere: it is the same mirrored instance unturned,
    /// turned. Either way it cuts the part's area. (A pocket starts near where the tool
    /// is, as copies always have, so only its area is compared.)
    #[test]
    fn an_instance_cuts_the_parts_toolpath_put_in_its_place(
        angle in -3.2f64..3.2,
        mirror in 0u8..3,
        x in 300.0f64..700.0,
        y in 300.0f64..700.0,
    ) {
        let spot = Spot { mirror: mirror_of(mirror), angle, at: [x, y] };
        let tabs = json!({"count": 3, "width": 8, "height": 3, "flank_angle": 1.0});
        let ops = json!([pocket(), profile("outside", tabs)]);
        let plan = plan_job(&with_instance(ops.clone(), spot), &part()).unwrap();
        let place = Placement::new(&spot, (0.0, 0.0));
        // What instance 2's profile is compared with: the part's, or the same mirrored
        // instance unturned.
        let (reference_plan, reference_copy, reference_place) = match spot.mirror {
            None => (plan.clone(), 1, Placement::new(&Spot::default(), (0.0, 0.0))),
            Some(_) => {
                let unturned = Spot { angle: 0.0, ..spot };
                let other = plan_job(&with_instance(ops, unturned), &part()).unwrap();
                (other, 2, Placement::new(&unturned, (0.0, 0.0)))
            }
        };
        let reference = feed_ends_of(&reference_plan, reference_copy, Some(1));
        let profile_ends = feed_ends_of(&plan, 2, Some(1));
        prop_assert_eq!(reference.len(), profile_ends.len());
        for p in &profile_ends {
            let there = reference_place.apply(place.inverse(*p));
            prop_assert!(near(there, &reference, 1e-6), "{:?}", p);
        }
        let instance_ends = feed_ends(&plan, 2);
        // The part's area, mirrored or not: the same extent brought back.
        let extent = |points: &[Point]| points.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(a, b, c, d), p| (a.min(p.0), b.min(p.1), c.max(p.0), d.max(p.1)),
        );
        let back: Vec<Point> = instance_ends.iter().map(|p| place.inverse(*p)).collect();
        let (got, want) = (extent(&back), extent(&feed_ends(&plan, 1)));
        for (g, w) in [(got.0, want.0), (got.1, want.1), (got.2, want.2), (got.3, want.3)] {
            prop_assert!((g - w).abs() < 1e-6, "{:?} vs {:?}", got, want);
        }
        // Arcs stay arcs: as many tagged moves in each profile.
        let arcs = |plan: &Plan, copy: u32| plan.op_spans.iter()
            .filter(|s| s.copy == copy && s.operation == 1)
            .flat_map(|s| &plan.program.moves[s.moves.clone()])
            .filter(|m| m.circle.is_some())
            .count();
        prop_assert_eq!(arcs(&reference_plan, reference_copy), arcs(&plan, 2));
        // Climb stays climb: the profile goes round the same way, mirrored or not.
        let profile_only = |copy: u32| {
            let mut only = plan.clone();
            only.op_spans.retain(|s| s.operation == 1);
            swept_area(&only, copy)
        };
        prop_assert!(profile_only(1) * profile_only(2) > 0.0);
    }
}

proptest! {
    #![proptest_config(common::proptest_config(24))]

    /// The part itself placed (docs/specs/part-placement.md §3) cuts its toolpath as drawn
    /// put in its place: every feed of the placed part, brought back, is one of the
    /// unplaced part's, as many, arcs still arcs and going round the same way; mirrored
    /// parts too, and its instances stay where they were.
    #[test]
    fn the_part_itself_placed_cuts_its_toolpath_put_in_its_place(
        angle in -3.2f64..3.2,
        mirror in 0u8..3,
        x in -300.0f64..300.0,
        y in -300.0f64..300.0,
    ) {
        let tabs = json!({"count": 3, "width": 8, "height": 3, "flank_angle": 1.0});
        let ops = json!([pocket(), profile("outside", tabs)]);
        let instance = Spot { mirror: None, angle: 0.0, at: [800.0, 0.0] };
        let mut drawn = with_instance(ops, instance);
        drawn.parts[0].mirror = mirror_of(mirror);
        let mut placed = drawn.clone();
        placed.parts[0].angle = angle;
        placed.parts[0].at = [x, y];
        let (before, after) = (
            plan_job(&drawn, &part()).unwrap(),
            plan_job(&placed, &part()).unwrap(),
        );
        let place = Placement::new(&placed.parts[0].spot(), (0.0, 0.0));
        let reference = feed_ends_of(&before, 1, Some(1));
        let profile_ends = feed_ends_of(&after, 1, Some(1));
        prop_assert_eq!(reference.len(), profile_ends.len());
        for p in &profile_ends {
            prop_assert!(near(place.inverse(*p), &reference, 1e-6), "{:?}", p);
        }
        let arcs = |plan: &Plan| plan.op_spans.iter()
            .filter(|s| s.copy == 1 && s.operation == 1)
            .flat_map(|s| &plan.program.moves[s.moves.clone()])
            .filter(|m| m.circle.is_some())
            .count();
        prop_assert_eq!(arcs(&before), arcs(&after));
        let profile_only = |plan: &Plan| {
            let mut only = plan.clone();
            only.op_spans.retain(|s| s.operation == 1);
            swept_area(&only, 1)
        };
        prop_assert!(profile_only(&before) * profile_only(&after) > 0.0);
        // Instance 2 is placed relative to the part as drawn, not to the part placed. (Its
        // pocket starts near where the tool is, so only its profile is compared.)
        prop_assert_eq!(feed_ends_of(&before, 2, Some(1)), feed_ends_of(&after, 2, Some(1)));
    }
}

#[test]
fn a_quarter_turned_instance_lies_turned_about_the_centre_and_moved() {
    let spot = Spot {
        mirror: None,
        angle: std::f64::consts::FRAC_PI_2,
        at: [500.0, 300.0],
    };
    let plan = plan_job(
        &with_instance(json!([profile("outside", Value::Null)]), spot),
        &part(),
    )
    .unwrap();
    // The part cuts X 4..116, Y 4..76 (the 6 mm tool's path outside, and the tool round
    // it); turned 90° about (0, 0), that is X -76..-4, Y 4..116, then moved.
    let ((x0, y0), (x1, y1)) =
        chipload_toolpath::plan::cut_extent(&plan.program.moves, &plan.op_spans, Some(2)).unwrap();
    for (got, want) in [(x0, 424.0), (y0, 304.0), (x1, 496.0), (y1, 416.0)] {
        assert!((got - want).abs() < 1e-9, "{got} vs {want}");
    }
}

#[test]
fn a_mirrored_instance_keeps_climb() {
    // Without the cutting sense turned before mirroring, the mirrored profile would run the
    // other way round: conventional.
    for mirror in [Mirror::LeftRight, Mirror::TopBottom] {
        let spot = Spot {
            mirror: Some(mirror),
            angle: 0.0,
            at: [400.0, 400.0],
        };
        let plan = plan_job(
            &with_instance(json!([profile("outside", Value::Null)]), spot),
            &part(),
        )
        .unwrap();
        let (one, two) = (swept_area(&plan, 1), swept_area(&plan, 2));
        assert!(one * two > 0.0, "{mirror:?}: {one} vs {two}");
    }
}

#[test]
fn a_turned_instance_over_the_part_is_refused() {
    // Turned half round about (0, 0) and moved back by (120, 80): X 10..110 maps onto
    // 10..110 again, and the L lands on the part's own corner.
    let spot = Spot {
        mirror: None,
        angle: std::f64::consts::PI,
        at: [120.0, 80.0],
    };
    let err = plan_job(
        &with_instance(json!([profile("outside", Value::Null)]), spot),
        &part(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("instances 1 and 2 cut into each other"),
        "{err}"
    );
}

#[test]
fn an_instance_that_is_off_is_not_cut() {
    let spot = Spot {
        mirror: Some(Mirror::LeftRight),
        angle: 0.5,
        at: [400.0, 400.0],
    };
    let mut job = with_instance(json!([profile("outside", Value::Null)]), spot);
    job.parts[0].instances[0].enabled = false;
    let plan = plan_job(&job, &part()).unwrap();
    assert!(plan.op_spans.iter().all(|s| s.copy == 1));
}
