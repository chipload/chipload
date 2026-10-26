// SPDX-License-Identifier: GPL-3.0-or-later
//! Dogbones chosen per corner (docs/specs/dogbone-per-corner.md §8), on the user's drawing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::{Bone, Job, Operation, Wall};
use chipload_pipeline::corners::{choose_corner, corners_of, reset_corners};
use chipload_pipeline::placement::{Move, place_dxf};
use chipload_pipeline::run_job;
use chipload_toolpath::corners::Gets;
use serde_json::json;

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// The user's drawing (2026-10-26) with a 6 mm dogbone pocket on contour #30: a 9 mm slot
/// with a 3.167 mm step at each end.
fn slot_job() -> Job {
    let shapes = chipload_dxf::shapes_from_dxf(&corpus("dxf/shared_edges.dxf"), None).unwrap();
    let mut job = chipload_pipeline::new_project(shapes);
    let shape = job
        .drawing
        .shapes
        .iter()
        .find(|s| s.dxf_handle() == Some("F5"))
        .unwrap()
        .id;
    let pocket: Operation = serde_json::from_value(json!({
        "type": "pocket", "name": "", "enabled": true, "part": job.parts[0].id,
        "select": {"layer": null, "shapes": [shape], "text": null},
        "tool": job.tools[0].id, "start_depth": 0.0, "end_depth": 1.0, "clearance": 0.0,
        "step_down": 1.0, "stepover": 0.4, "direction": "climb", "feed": 1500.0,
        "plunge_feed": 500.0, "entry": {"type": "plunge"}, "starts": [], "lead": null,
        "corners": {"type": "dogbone", "max_angle": 100.0_f64.to_radians()}, "rest": null
    }))
    .unwrap();
    job.operations.push(pocket);
    job
}

const STEPS: [[f64; 2]; 2] = [[11.214, 301.0], [511.214, 301.0]];

#[test]
fn the_slots_steps_get_t_bones_along_their_short_walls() {
    let mut job = slot_job();
    let err = run_job(&job).unwrap_err().message().english();
    assert!(err.contains("another wall of its own contour #30"), "{err}");

    for at in STEPS {
        choose_corner(&mut job, 0, at, Some((Bone::Tbone, Some(Wall::Shorter)))).unwrap();
    }
    let rows = corners_of(&job, 0).unwrap();
    assert_eq!(rows.len(), 6);
    let chosen: Vec<_> = rows.iter().filter(|r| r.chosen).collect();
    assert_eq!(chosen.len(), 2);
    assert!(
        chosen
            .iter()
            .all(|r| r.gets == Some(Gets::Tbone(Wall::Shorter)))
    );
    run_job(&job).unwrap();

    // Round there works too; the other corners keep the operation's dogbones.
    for at in STEPS {
        choose_corner(&mut job, 0, at, Some((Bone::None, None))).unwrap();
    }
    run_job(&job).unwrap();
    assert_eq!(reset_corners(&mut job, 0).unwrap(), 2);
    assert!(run_job(&job).is_err());
}

#[test]
fn a_choice_is_stored_at_the_corner_and_only_at_one() {
    let mut job = slot_job();
    let err = choose_corner(&mut job, 0, [12.0, 301.0], Some((Bone::None, None)))
        .unwrap_err()
        .message()
        .english();
    assert!(
        err.contains("has no corner at (12.000, 301.000) mm"),
        "{err}"
    );
    // Typed with three decimals, stored at the corner's own point, once.
    let at = choose_corner(&mut job, 0, STEPS[0], Some((Bone::Dogbone, None))).unwrap();
    choose_corner(&mut job, 0, at, Some((Bone::None, None))).unwrap();
    let choices = job.operations[0].corner_choices();
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].at, at);
    assert_eq!(choices[0].bone, Bone::None);
    // Back to the operation's.
    choose_corner(&mut job, 0, at, None).unwrap();
    assert!(job.operations[0].corner_choices().is_empty());
}

#[test]
fn placing_the_drawing_moves_the_choices() {
    let mut job = slot_job();
    for at in STEPS {
        choose_corner(&mut job, 0, at, Some((Bone::Tbone, Some(Wall::Shorter)))).unwrap();
    }
    let (moved, report) = place_dxf(&job, 1, Move::Rotate(30.0)).unwrap();
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("Moved 2 corner choice(s) with it.")),
        "{:?}",
        report.lines
    );
    let rows = corners_of(&moved, 0).unwrap();
    assert_eq!(rows.iter().filter(|r| r.chosen).count(), 2);
    run_job(&moved).unwrap();
}
