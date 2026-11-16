// SPDX-License-Identifier: GPL-3.0-or-later
//! Turned and mirrored instances through the whole pipeline (docs/specs/instances.md §3):
//! every operation of the part, placed about the part's centre, climb staying climb, and a
//! program the independent checker passes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_job::{Instance, Job, Mirror, Spot};
use chipload_pipeline::run_job;
use chipload_toolpath::OpSpan;
use chipload_toolpath::model::Program;
use chipload_toolpath::place::Placement;
use common::{every_direction, loops, turning};
use std::f64::consts::FRAC_PI_2;

/// The part with five operations (a pocket, profiles, holes, a path) and one instance at
/// `spot`, on a stock with room for it.
fn with_instance(spot: Spot, part_mirror: Option<Mirror>) -> Job {
    let mut job = every_direction(None);
    job.parts[0].mirror = part_mirror;
    job.parts[0].instances = vec![Instance {
        id: 2,
        enabled: true,
        mirror: spot.mirror,
        angle: spot.angle,
        at: spot.at,
    }];
    job.setups[0].stock.fit = None;
    job.setups[0].stock.x = 1000.0;
    job.setups[0].stock.y = 1000.0;
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    job
}

fn spans_of(spans: &[OpSpan], copy: u32) -> Vec<OpSpan> {
    spans.iter().filter(|s| s.copy == copy).cloned().collect()
}

/// Each operation of `copy`: the share of its cut area that runs counter-clockwise, and
/// the area.
fn shares(program: &Program, spans: &[OpSpan], copy: u32) -> Vec<(usize, f64, f64)> {
    turning(&loops(program, &spans_of(spans, copy)))
        .into_iter()
        .map(|(op, ccw, cw)| (op, ccw / (ccw - cw), ccw - cw))
        .collect()
}

/// Every feed move's end in `copy`, its X and Y filled in from the moves before.
fn feed_ends(program: &Program, spans: &[OpSpan], copy: u32) -> Vec<Point> {
    let mut at = (0.0, 0.0);
    let mut ends = Vec::new();
    for (k, m) in program.moves.iter().enumerate() {
        at = (m.x.unwrap_or(at.0), m.y.unwrap_or(at.1));
        let counted = spans.iter().any(|s| s.copy == copy && s.moves.contains(&k));
        if counted && m.kind != chipload_toolpath::model::MoveKind::Rapid {
            ends.push(at);
        }
    }
    ends
}

fn extent(points: &[Point]) -> (f64, f64, f64, f64) {
    points.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(a, b, c, d), p| (a.min(p.0), b.min(p.1), c.max(p.0), d.max(p.1)),
    )
}

/// Instance 2 turns every operation the way the part does, over the same area, and lies
/// where its placement about the part's centre puts the part's cut.
fn assert_placed_like_the_part(spot: Spot, part_mirror: Option<Mirror>) {
    let what = format!("{spot:?}, part mirrored {part_mirror:?}");
    let job = with_instance(spot, part_mirror);
    // run_job refuses a program the independent checker doesn't pass.
    let result = run_job(&job).unwrap_or_else(|e| panic!("{what}: {e}"));
    let (part, instance) = (
        shares(&result.program, &result.op_spans, 1),
        shares(&result.program, &result.op_spans, 2),
    );
    assert_eq!(part.len(), 5, "{what}: every operation cut: {part:?}");
    assert_eq!(part.len(), instance.len(), "{what}");
    for (p, i) in part.iter().zip(&instance) {
        assert_eq!(p.0, i.0, "{what}");
        // Where each ring starts moves its ramp's overlap a little (mirror_part.rs).
        assert!((p.2 - i.2).abs() < 0.1 * p.2, "{what}: {p:?} vs {i:?}");
        assert!((p.1 - i.1).abs() < 0.01, "{what}: {p:?} vs {i:?}");
    }
    let centre = job.part_centre(1).unwrap();
    let place = Placement::new(&spot, centre);
    let back: Vec<Point> = feed_ends(&result.program, &result.op_spans, 2)
        .into_iter()
        .map(|p| place.inverse(p))
        .collect();
    let (got, want) = (
        extent(&back),
        extent(&feed_ends(&result.program, &result.op_spans, 1)),
    );
    for (g, w) in [
        (got.0, want.0),
        (got.1, want.1),
        (got.2, want.2),
        (got.3, want.3),
    ] {
        assert!((g - w).abs() < 0.05, "{what}: {got:?} vs {want:?}");
    }
}

#[test]
fn turned_and_mirrored_instances_cut_every_operation_like_the_part() {
    for (mirror, angle) in [
        (None, 0.5),
        (None, FRAC_PI_2),
        (Some(Mirror::LeftRight), 0.0),
        (Some(Mirror::TopBottom), -0.7),
        (Some(Mirror::LeftRight), FRAC_PI_2),
    ] {
        let spot = Spot {
            mirror,
            angle,
            at: [400.0, 300.0],
        };
        assert_placed_like_the_part(spot, None);
    }
}

#[test]
fn a_mirrored_parts_instances_are_the_mirrored_part_placed() {
    // The part's own mirror goes with every instance; an instance's mirror on top of it
    // brings the drawing back, still cut climb.
    for (part_mirror, mirror) in [
        (Mirror::LeftRight, None),
        (Mirror::LeftRight, Some(Mirror::LeftRight)),
        (Mirror::TopBottom, Some(Mirror::LeftRight)),
    ] {
        let spot = Spot {
            mirror,
            angle: 0.4,
            at: [400.0, 300.0],
        };
        assert_placed_like_the_part(spot, Some(part_mirror));
    }
}

#[test]
fn a_turned_instance_may_not_cut_into_another_part() {
    // Part 1's box is 0 … 110 square, its centre (55, 55): turned half round and moved
    // 200 right, the hole's corner at (0, 0) lies at (310, 110), and the circle part 2
    // cuts at (310.6, 108.5) is right beside it. The parts as drawn are far apart.
    let mut job = common::bones_beside_another_part([310.6, 108.5]);
    job.setups[0].stock.fit = None;
    job.setups[0].stock.x = 1000.0;
    job.setups[0].stock.y = 1000.0;
    assert!(run_job(&job).is_ok());
    job.parts[0].instances = vec![Instance {
        id: 2,
        enabled: true,
        mirror: None,
        angle: std::f64::consts::PI,
        at: [200.0, 0.0],
    }];
    let err = run_job(&job).unwrap_err().to_string();
    assert!(
        err.contains("Part \"Dot\" copy 1 overlaps part \"Box\" copy 2"),
        "{err}"
    );
}

#[test]
fn turned_instances_are_drawn_where_they_are_cut() {
    let spot = Spot {
        mirror: Some(Mirror::TopBottom),
        angle: 0.9,
        at: [400.0, 300.0],
    };
    let job = with_instance(spot, None);
    let contours = run_job(&job).unwrap().contours;
    let outlines = chipload_pipeline::parts::placed_outlines(&job, &contours);
    assert!(!outlines.is_empty());
    // Each outline brought back is one of the part's contours as drawn.
    let place = Placement::new(&spot, job.part_centre(1).unwrap());
    for (points, _) in &outlines {
        let back: Vec<Point> = points.iter().map(|&p| place.inverse(p)).collect();
        let found = contours.iter().any(|c| {
            let drawn = c.points();
            drawn.len() == back.len()
                && drawn
                    .iter()
                    .zip(&back)
                    .all(|(a, b)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9)
        });
        assert!(found, "{points:?}");
    }
    // The part alone and its moved instances are drawn at their shifts, not here.
    let mut moved = job.clone();
    moved.parts[0].instances[0].mirror = None;
    moved.parts[0].instances[0].angle = 0.0;
    assert!(chipload_pipeline::parts::placed_outlines(&moved, &contours).is_empty());
}

#[test]
fn a_new_instance_goes_beside_what_the_part_cuts() {
    use chipload_pipeline::instances::{INSTANCE_GAP_MM, add_instance};
    use chipload_toolpath::plan::cut_extent;
    let mut job = with_instance(Spot::default(), None);
    job.parts[0].instances.clear();
    // A quarter turn: the turned box is exactly the turned cut's.
    let (job, two) = add_instance(&job, 1, None, FRAC_PI_2, None).unwrap();
    let (job, three) = add_instance(&job, 1, None, 0.0, Some(Mirror::TopBottom)).unwrap();
    assert_eq!((two, three), (2, 3));
    let result = run_job(&job).unwrap();
    let extent = |copy| cut_extent(&result.program.moves, &result.op_spans, Some(copy)).unwrap();
    let (one, two, three) = (extent(1), extent(2), extent(3));
    // Each 10 mm right of what was cut before it, its bottom level with the part's.
    assert!(
        (two.0.0 - (one.1.0 + INSTANCE_GAP_MM)).abs() < 1e-6,
        "{one:?} {two:?}"
    );
    assert!(
        (three.0.0 - (two.1.0 + INSTANCE_GAP_MM)).abs() < 1e-6,
        "{two:?} {three:?}"
    );
    for low in [two.0.1, three.0.1] {
        assert!((low - one.0.1).abs() < 1e-6, "{one:?} {low}");
    }
}
