// SPDX-License-Identifier: GPL-3.0-or-later
//! Tabs placed by hand (docs/specs/tabs-by-hand.md §7).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_core::primitives::distance;
use chipload_job::{Job, Operation, load_job};
use chipload_pipeline::run_job;
use chipload_pipeline::shapes::transform_shapes;
use chipload_pipeline::tabs::{add_tab, even_ring, remove_tab, reset_tabs, tabs_of};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// A drawn 120 × 80 mm rectangle turned 10°, cut outside (operation 2) with 4 tabs.
fn project() -> Job {
    load_job(&corpus("jobs/v26/straight_tabs.chipload")).unwrap()
}

const PROFILE: usize = 1;

/// The middle of the rectangle's right side on the tool path (3 mm out), between two tabs.
fn right_side() -> [f64; 2] {
    let (s, c) = 10f64.to_radians().sin_cos();
    [100.0 + 63.0 * c, 80.0 + 63.0 * s]
}

fn tab_points(job: &Job) -> Vec<[f64; 2]> {
    match &job.operations[PROFILE] {
        Operation::Profile(p) => p.tabs.as_ref().unwrap().at.clone(),
        _ => panic!("operation 2 is the profile"),
    }
}

#[test]
fn adding_a_tab_keeps_the_others_where_they_were() {
    let mut job = project();
    let before = tabs_of(&job, PROFILE).unwrap();
    assert_eq!(before.len(), 4);
    assert!(before.iter().all(|t| !t.by_hand));
    add_tab(&mut job, PROFILE, right_side()).unwrap();
    let after = tabs_of(&job, PROFILE).unwrap();
    assert_eq!(after.len(), 5);
    assert!(after.iter().all(|t| t.by_hand));
    for t in &before {
        assert!(
            after.iter().any(|u| distance(u.at, t.at) < 1e-6),
            "{t:?} kept"
        );
    }
    run_job(&job).unwrap();
}

#[test]
fn removing_spacing_and_resetting() {
    let mut job = project();
    let before = tabs_of(&job, PROFILE).unwrap();
    remove_tab(&mut job, PROFILE, [before[2].at.0, before[2].at.1]).unwrap();
    let after = tabs_of(&job, PROFILE).unwrap();
    assert_eq!(after.len(), 3);
    assert!(!after.iter().any(|t| distance(t.at, before[2].at) < 1e-6));
    run_job(&job).unwrap();

    assert_eq!(
        even_ring(&mut job, PROFILE, [before[0].at.0, before[0].at.1]).unwrap(),
        3
    );
    assert_eq!(tabs_of(&job, PROFILE).unwrap(), before);

    remove_tab(&mut job, PROFILE, [before[0].at.0, before[0].at.1]).unwrap();
    assert_eq!(reset_tabs(&mut job, PROFILE).unwrap(), 3);
    assert!(tab_points(&job).is_empty());
}

#[test]
fn a_tab_too_close_or_far_from_the_ring_is_refused() {
    let mut job = project();
    let before = tabs_of(&job, PROFILE).unwrap();
    // Next to a tab already there: they'd overlap. Adding stores it; generating refuses.
    let near = [before[0].at.0 + 1.0, before[0].at.1];
    add_tab(&mut job, PROFILE, near).unwrap();
    let err = run_job(&job).unwrap_err().message().english();
    assert!(err.contains("are too close"), "{err}");

    let mut job = project();
    let err = add_tab(&mut job, PROFILE, [500.0, 500.0])
        .unwrap_err()
        .message()
        .english();
    assert!(
        err.contains("has no contour within a tool diameter of (500.000, 500.000) mm"),
        "{err}"
    );
    // A point left far away (the drawing changed) stops generating.
    if let Operation::Profile(p) = &mut job.operations[PROFILE] {
        p.tabs.as_mut().unwrap().at = vec![[500.0, 500.0]];
    }
    let err = run_job(&job).unwrap_err().message().english();
    assert!(err.contains("is on none of its contours any more"), "{err}");
}

#[test]
fn tabs_by_hand_move_with_their_drawn_shape() {
    let mut job = project();
    let before = tabs_of(&job, PROFILE).unwrap();
    assert_eq!(before.len(), 4);
    add_tab(&mut job, PROFILE, right_side()).unwrap();
    let placed = tab_points(&job);
    let moved = transform_shapes(&job, &[1], (0.0, 0.0), (15.0, -5.0), 1.0, 0.0).unwrap();
    let after = tab_points(&moved);
    for (p, q) in placed.iter().zip(&after) {
        assert!((q[0] - p[0] - 15.0).abs() < 1e-9 && (q[1] - p[1] + 5.0).abs() < 1e-9);
    }
    run_job(&moved).unwrap();
}
