// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's stock (docs/specs/parts-and-setups.md §4): recorded on the part, a note where
//! its board differs, and a move to a board of that stock only when asked.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_job::{Job, MaterialRef, PartStock, load_job};
use chipload_pipeline::part_stock::{
    StockBoard, move_to_stock_board, set_part_stock, stock_board, stock_mismatch_notes,
};
use chipload_pipeline::run_setups;

/// Two parts, *simple part* (1) and *part B* (2), on one 12 mm board, each cut through
/// (`through` 0: to the board's bottom, whatever its thickness).
fn two_parts() -> Job {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/jobs/v22/two_parts.chipload");
    let mut job = load_job(&path).unwrap();
    assert_eq!(job.setups.len(), 1);
    for op in &mut job.operations {
        if let Some(through) = op.through_mut() {
            *through = Some(0.0);
        }
    }
    job.follow_thickness();
    job
}

fn thick(mm: f64) -> Option<PartStock> {
    Some(PartStock {
        material: None,
        thickness: Some(mm),
    })
}

fn birch() -> MaterialRef {
    MaterialRef {
        name: "Birch plywood".into(),
        library_uid: "00000000000000b1".into(),
    }
}

/// The end depths of part `part`'s operations.
fn depths(job: &Job, part: u64) -> Vec<f64> {
    job.operations
        .iter()
        .filter(|op| op.part() == part)
        .map(|op| op.depths(&job.tools).1)
        .collect()
}

#[test]
fn the_users_example_a_thinner_bottom_moves_to_a_board_of_its_own_when_asked() {
    // The user, 2026-11-17: a DXF with a side and a bottom, the bottom from 6 mm.
    let job = two_parts();
    let set = set_part_stock(&job, 2, thick(6.0)).unwrap();
    // Recorded; nothing moves, nothing is made (the user, 2026-11-17).
    assert_eq!(set.part(2).unwrap().stock, thick(6.0));
    assert_eq!(set.setups, job.setups);
    assert_eq!(stock_board(&set, 2), Some(StockBoard::New));
    // Asked: a board made like the first, 6 mm thick.
    let (moved, made) = move_to_stock_board(&set, 2).unwrap();
    assert!(made.made);
    assert_eq!(moved.setups.len(), 2);
    let board = moved.setup(made.to).unwrap();
    let first = &moved.setups[0];
    assert_eq!(board.stock.thickness, 6.0);
    assert_eq!(
        (
            board.stock.x,
            board.stock.y,
            board.stock.origin,
            board.stock.fit
        ),
        (
            first.stock.x,
            first.stock.y,
            first.stock.origin,
            first.stock.fit
        )
    );
    assert_eq!(
        (board.stock.z_zero, board.stock.max_overcut, board.work_zero),
        (first.stock.z_zero, first.stock.max_overcut, first.work_zero)
    );
    assert_eq!(moved.setups_of_part(2), [made.to]);
    assert_eq!(moved.setups_of_part(1), [first.id]);
    assert_eq!(moved.active_setup_id(), Some(made.to));
    assert_eq!(stock_board(&moved, 2), None);
    // Its cuts through the board go through 6 mm; the side's still through 12.
    assert!(
        depths(&moved, 2).iter().all(|d| *d == 6.0),
        "{:?}",
        depths(&moved, 2)
    );
    assert!(depths(&moved, 1).contains(&12.0));
    // Two programs, each passing the checker.
    assert_eq!(run_setups(&moved, None).unwrap().len(), 2);
    assert_eq!(
        made.message(&moved, 2).unwrap().english(),
        "Made Setup 2 (6 mm) for part B"
    );

    // A second part asking for 6 mm goes to that board; nothing new is made.
    let asked = set_part_stock(&moved, 1, thick(6.0)).unwrap();
    assert_eq!(stock_board(&asked, 1), Some(StockBoard::Existing(made.to)));
    let (both, second) = move_to_stock_board(&asked, 1).unwrap();
    assert_eq!((second.to, second.made), (made.to, false));
    assert_eq!(both.setups_of_part(1), [made.to]);
    // The first board stays, though empty: boards go only when the user removes them.
    assert_eq!(both.setups.len(), 2);

    // Cleared again: nothing moves.
    let cleared = set_part_stock(&moved, 2, None).unwrap();
    assert_eq!(cleared.setups_of_part(2), [made.to]);
    assert_eq!(stock_board(&cleared, 2), None);
    assert!(move_to_stock_board(&cleared, 2).is_err());
}

#[test]
fn a_part_whose_board_fits_stays_and_a_material_counts_too() {
    let job = two_parts();
    // 12 mm is its board: nowhere to go, the stock is kept.
    let same = set_part_stock(&job, 2, thick(12.0)).unwrap();
    assert_eq!(stock_board(&same, 2), None);
    assert_eq!(same.part(2).unwrap().stock, thick(12.0));
    // A material the board hasn't: a board of birch, 12 mm as the first.
    let birch_stock = PartStock {
        material: Some(birch()),
        thickness: None,
    };
    let asked = set_part_stock(&job, 2, Some(birch_stock)).unwrap();
    let (birch_job, moved) = move_to_stock_board(&asked, 2).unwrap();
    let board = birch_job.setup(moved.to).unwrap();
    assert_eq!(board.stock.material, Some(birch()));
    assert_eq!(board.stock.thickness, 12.0);
    // An empty stock is none.
    let none = set_part_stock(&job, 2, Some(PartStock::default())).unwrap();
    assert_eq!(none.part(2).unwrap().stock, None);
    // A thickness of 0 is refused.
    assert!(set_part_stock(&job, 2, thick(0.0)).is_err());
}

#[test]
fn a_mismatch_is_a_note() {
    let job = two_parts();
    let set = set_part_stock(&job, 2, thick(6.0)).unwrap();
    let here = job.setups[0].id;
    assert_eq!(set.stock_mismatches(here), [2]);
    let notes = stock_mismatch_notes(&set);
    assert_eq!(notes.len(), 1);
    assert_eq!(
        notes[0].text,
        "part B is to be cut from 6 mm, but this board is 12 mm"
    );
    // The run says so too.
    let result = chipload_pipeline::run_job(&set.for_setup(here)).unwrap();
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.text.contains("is to be cut from 6 mm"))
    );
}

#[test]
fn a_part_on_a_two_sided_board_is_not_moved() {
    let job = two_parts();
    let (two, _, _) = chipload_pipeline::setups::add_back_side(
        &job,
        job.setups[0].id,
        chipload_job::Turn::LeftRight,
        None,
    )
    .unwrap();
    let kept = set_part_stock(&two, 2, thick(6.0)).unwrap();
    assert_eq!(stock_board(&kept, 2), None);
    assert!(move_to_stock_board(&kept, 2).is_err());
    assert_eq!(kept.setups.len(), 2);
    assert_eq!(kept.stock_mismatches(job.setups[0].id), [2]);
}
