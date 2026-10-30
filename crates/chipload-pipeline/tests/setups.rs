// SPDX-License-Identifier: GPL-3.0-or-later
//! Setups (docs/specs/setups.md): boards clamped one after the other, each a program of
//! its own with its stock and zero.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{Job, OpRef, StockOrigin, WorkZero, load_job};
use chipload_pipeline::run_setups;
use chipload_pipeline::setups::{
    activate_setup, add_setup, move_part_to_setup, remove_setup, set_setup,
};

/// Two parts: "simple part" (operations 1-3, 12 mm deep) and "part B" (operation 4).
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v22/two_parts.chipload"),
    )
    .unwrap()
}

/// The sample with part B in a 6 mm setup of its own, its outline 6 mm deep.
fn two_boards() -> Job {
    let (job, id, _) = add_setup(&sample(), Some("6 mm bottom"), Some(6.0)).unwrap();
    assert_eq!(id, 2);
    let job = move_part_to_setup(&job, 2, 2).unwrap();
    chipload_job::paths::set_all(&job, &[("operations.3.end_depth".into(), 6.0.into())]).unwrap()
}

#[test]
fn one_setup_writes_the_files_it_always_did() {
    let job = sample();
    let runs = run_setups(&job, None).unwrap();
    assert_eq!(runs.len(), 1);
    let single = chipload_pipeline::run_job(&job).unwrap();
    assert_eq!(runs[0].1.files, single.files);
}

#[test]
fn each_setup_is_a_program_of_its_own_with_its_board() {
    let job = two_boards();
    let runs = run_setups(&job, None).unwrap();
    let ids: Vec<u64> = runs.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, [1, 2]);
    // Setup 1: the first part's operations, two tools; setup 2: part B alone.
    let ops = |k: usize| -> Vec<OpRef> {
        runs[k]
            .1
            .files
            .iter()
            .flat_map(|f| f.operations.clone())
            .collect()
    };
    assert_eq!(ops(0), [OpRef::op(0), OpRef::op(1), OpRef::op(2)]);
    assert_eq!(ops(1), [OpRef::op(3)]);
    // File names say the setup first.
    let suffixes: Vec<String> = runs
        .iter()
        .flat_map(|(_, r)| r.files.iter().map(|f| f.suffix.clone().unwrap()))
        .collect();
    assert_eq!(suffixes, ["-S1-1-T1", "-S1-2-T2", "-S2"]);
    // Setup 2's program goes no deeper than its 6 mm board allows.
    let deepest = runs[1]
        .1
        .program
        .moves
        .iter()
        .filter_map(|m| m.z)
        .fold(f64::INFINITY, f64::min);
    assert!(deepest >= -6.0 - 1e-9, "{deepest}");
    // Only one, by id.
    let only = run_setups(&job, Some(2)).unwrap();
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].0, 2);
    // A setup that is off isn't cut.
    let off = set_setup(&job, 1, None, Some(false)).unwrap();
    assert_eq!(run_setups(&off, None).unwrap().len(), 1);
}

#[test]
fn a_board_that_fits_fits_its_own_parts() {
    // Both boards centred on their drawing and fitting it: each round its own part.
    let mut job = two_boards();
    for setup in &mut job.setups {
        setup.stock.origin = StockOrigin::Drawing;
        setup.stock.fit = Some(10.0);
    }
    let whole = job.drawing.extents().unwrap();
    let first = job.for_setup(1).stock_bounds();
    let second = job.for_setup(2).stock_bounds();
    assert_ne!(first, second);
    for ((x0, y0), (x1, y1)) in [first, second] {
        assert!(
            x1 - x0 < whole.1.0 - whole.0.0 + 20.0,
            "smaller than the whole drawing's"
        );
        assert!(y1 - y0 <= whole.1.1 - whole.0.1 + 20.0 + 1e-9);
    }
}

#[test]
fn setups_are_added_activated_moved_and_removed() {
    let job = two_boards();
    // A new setup starts like the active one, with a part of its own, both active.
    let (job, id, part) = add_setup(&job, None, None).unwrap();
    assert_eq!(id, 3);
    assert_eq!(job.setup(3).unwrap().name, "Setup 3");
    assert_eq!(job.stock().thickness, 6.0);
    assert_eq!(
        (job.active_setup_id(), job.active_part_id()),
        (Some(3), Some(part))
    );
    // Activating setup 1 makes its first part the active part.
    let active = activate_setup(&job, 1).unwrap();
    assert_eq!(active.active_part_id(), Some(1));
    // Its own zero.
    let mut zeroed = active.clone();
    *zeroed.work_zero_mut() = WorkZero::Point { x: 5.0, y: 5.0 };
    assert_eq!(
        zeroed.setup(1).unwrap().work_zero,
        WorkZero::Point { x: 5.0, y: 5.0 }
    );
    assert_eq!(
        zeroed.setup(2).unwrap().work_zero,
        job.setup(2).unwrap().work_zero
    );
    // Removing a setup with parts needs --with-parts; the last setup stays.
    assert!(remove_setup(&job, 2, false).is_err());
    let (fewer, _) = remove_setup(&job, 2, true).unwrap();
    assert_eq!(fewer.setups.len(), 2);
    assert!(fewer.parts.iter().all(|p| p.setup != 2));
    assert!(fewer.operations.iter().all(|op| op.part() != 2));
    let (one, _) = remove_setup(&fewer, 3, true).unwrap();
    assert!(remove_setup(&one, 1, true).is_err());
}

#[test]
fn a_setup_is_simulated_on_its_own_board() {
    let job = two_boards();
    let runs = run_setups(&job, Some(2)).unwrap();
    let one = job.for_setup(2);
    let sim = chipload_pipeline::simulate::simulate(
        &one,
        &runs[0].1.program,
        &runs[0].1.groups,
        Some(1.0),
        &chipload_pipeline::Cancel::new(),
    )
    .unwrap();
    // The 6 mm board, cut through to its bottom and no further.
    let deepest = sim.numbers.deepest.unwrap();
    assert!((deepest + 6.0).abs() < 1e-6, "{deepest}");
    assert!(sim.numbers.below_bottom.is_none_or(|b| b <= 1e-6));
}
