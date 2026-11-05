// SPDX-License-Identifier: GPL-3.0-or-later
//! Two-sided work (docs/specs/two-sided.md): a back setup is the same board as its front,
//! turned over.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{BackOf, Job, Register, StockOrigin, Turn, WorkZero, load_job};
use chipload_pipeline::setups::{
    BoardChange, add_back_side, move_part_to_setup, remove_setup, set_board,
};

/// Two parts: "simple part" (operations 1-3) and "part B" (operation 4), one setup.
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v22/two_parts.chipload"),
    )
    .unwrap()
}

/// The sample with a back side, setup 2, holding part B.
fn with_back() -> Job {
    let (job, id, _) = add_back_side(&sample(), 1, Turn::LeftRight, None).unwrap();
    assert_eq!(id, 2);
    move_part_to_setup(&job, 2, 2).unwrap()
}

#[test]
fn a_back_side_shares_its_fronts_board_and_zero() {
    let job = with_back();
    let back = job.setup(2).unwrap();
    assert_eq!(back.name, "Setup 1 back");
    assert_eq!(
        back.back_of,
        Some(BackOf {
            setup: 1,
            turn: Turn::LeftRight,
            register: Register::Pins,
        })
    );
    assert_eq!(back.stock, job.setup(1).unwrap().stock);
    assert_eq!(back.work_zero, job.setup(1).unwrap().work_zero);
    // The front changes, the back follows wherever a project is read or changed.
    let mut changed = job.clone();
    changed.setups[0].stock.thickness = 18.0;
    changed.setups[0].work_zero = WorkZero::Point { x: 3.0, y: 4.0 };
    // Set on the back by hand: not its own, so it goes.
    changed.setups[1].stock.thickness = 5.0;
    changed.follow_thickness();
    assert_eq!(changed.setup(2).unwrap().stock.thickness, 18.0);
    assert_eq!(
        changed.setup(2).unwrap().work_zero,
        WorkZero::Point { x: 3.0, y: 4.0 }
    );
}

#[test]
fn a_board_that_fits_fits_both_sides() {
    let mut job = with_back();
    for setup in &mut job.setups {
        setup.stock.origin = StockOrigin::Drawing;
        setup.stock.fit = Some(10.0);
    }
    job.follow_thickness();
    let front = job.for_setup(1).stock_bounds();
    let back = job.for_setup(2).stock_bounds();
    assert_eq!(front, back, "one board");
    // Unlinked, each setup fits its own part, so the front's board is smaller.
    let own = set_board(
        &job,
        2,
        BoardChange {
            back_of: Some(None),
            ..BoardChange::default()
        },
    )
    .unwrap();
    let alone = own.for_setup(1).stock_bounds();
    let area = |((x0, y0), (x1, y1)): ((f64, f64), (f64, f64))| (x1 - x0) * (y1 - y0);
    assert!(area(alone) < area(front), "{alone:?} {front:?}");
}

#[test]
fn one_board_has_one_back_and_a_back_is_not_turned_again() {
    let job = with_back();
    // A second back of setup 1.
    assert!(add_back_side(&job, 1, Turn::TopBottom, None).is_err());
    // The back of the back.
    assert!(add_back_side(&job, 2, Turn::LeftRight, None).is_err());
    // A setup turned over onto itself.
    let itself = BoardChange {
        back_of: Some(Some(2)),
        ..BoardChange::default()
    };
    assert!(set_board(&job, 2, itself).is_err());
    // A turn for a setup that is not a back.
    let turn = BoardChange {
        turn: Some(Turn::TopBottom),
        ..BoardChange::default()
    };
    let err = set_board(&job, 1, turn).unwrap_err();
    assert!(err.to_string().contains("not the back"), "{err}");
    // A back's turn and way of locating change; its front stays.
    let changed = set_board(
        &job,
        2,
        BoardChange {
            turn: Some(Turn::TopBottom),
            register: Some(Register::Corner),
            ..BoardChange::default()
        },
    )
    .unwrap();
    assert_eq!(
        changed.setup(2).unwrap().back_of,
        Some(BackOf {
            setup: 1,
            turn: Turn::TopBottom,
            register: Register::Corner,
        })
    );
}

#[test]
fn removing_the_front_leaves_the_back_its_board() {
    let mut job = with_back();
    job.setups[0].stock.thickness = 15.0;
    job.follow_thickness();
    let (job, lines) = remove_setup(&job, 1, true).unwrap();
    assert!(
        lines.iter().any(|l| l.contains("board of its own")),
        "{lines:?}"
    );
    let back = job.setup(2).unwrap();
    assert_eq!(back.back_of, None);
    assert_eq!(back.stock.thickness, 15.0);
}

#[test]
fn a_back_side_is_saved_and_read_back() {
    let job = with_back();
    let dir = std::env::temp_dir().join(format!("chipload-two-sided-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("back.chipload");
    chipload_job::save_job(&job, &path).unwrap();
    let read = load_job(&path).unwrap();
    assert_eq!(
        read.setup(2).unwrap().back_of,
        job.setup(2).unwrap().back_of
    );
    assert_eq!(read.schema_version, 36);
    std::fs::remove_dir_all(&dir).unwrap();
}
