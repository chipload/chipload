// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's stock picks its board (docs/specs/parts-and-setups.md §4, decision 11).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_job::{Job, MaterialRef, PartStock, load_job};
use chipload_pipeline::part_stock::{set_part_stock, stock_mismatch_notes};
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
fn the_users_example_a_thinner_bottom_moves_to_a_board_of_its_own() {
    // The user, 2026-11-17: a DXF with a side and a bottom, the bottom from 6 mm.
    let job = two_parts();
    let (moved, made) = set_part_stock(&job, 2, thick(6.0)).unwrap();
    let made = made.unwrap();
    assert!(made.made && made.removed.is_empty());
    assert_eq!(moved.setups.len(), 2);
    let board = moved.setup(made.to).unwrap();
    // Made like the first: its size, origin, zero, Z zero and overcut; 6 mm thick.
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
    assert!(board.made_for_stock);
    assert_eq!(moved.setups_of_part(2), [made.to]);
    assert_eq!(moved.setups_of_part(1), [first.id]);
    assert_eq!(moved.active_setup_id(), Some(made.to));
    // Its cuts through the board go through 6 mm; the side's still through 12.
    assert!(
        depths(&moved, 2).iter().all(|d| *d == 6.0),
        "{:?}",
        depths(&moved, 2)
    );
    assert!(depths(&moved, 1).contains(&12.0));
    // Two programs, each passing the checker.
    assert_eq!(run_setups(&moved, None).unwrap().len(), 2);
    let text: Vec<String> = made
        .messages(&moved, 2)
        .iter()
        .map(|m| m.english())
        .collect();
    assert_eq!(text, ["Made Setup 2 (6 mm) for part B"]);

    // A second part asking for 6 mm joins that board; nothing new is made.
    let (both, second) = set_part_stock(&moved, 1, thick(6.0)).unwrap();
    let second = second.unwrap();
    assert_eq!((second.to, second.made), (made.to, false));
    assert_eq!(both.setups_of_part(1), [made.to]);
    // The first board is the user's: kept, though empty.
    assert_eq!(both.setups.len(), 2);

    // Cleared again: back to the first board, the made one removed once empty.
    let (back, gone) = set_part_stock(&moved, 2, None).unwrap();
    let gone = gone.unwrap();
    assert_eq!(gone.to, job.setups[0].id);
    assert_eq!(gone.removed, ["Setup 2"]);
    assert_eq!(back.setups.len(), 1);
    assert_eq!(back.setups_of_part(2), [job.setups[0].id]);
    assert!(depths(&back, 2).iter().all(|d| *d == 12.0));
    // Set to the first board's own thickness: back as well.
    let (again, _) = set_part_stock(&moved, 2, thick(12.0)).unwrap();
    assert_eq!(again.setups.len(), 1);
}

#[test]
fn a_part_whose_board_fits_stays_and_a_material_counts_too() {
    let job = two_parts();
    // 12 mm is its board: nothing moves, the stock is kept.
    let (same, moved) = set_part_stock(&job, 2, thick(12.0)).unwrap();
    assert!(moved.is_none());
    assert_eq!(same.part(2).unwrap().stock, thick(12.0));
    // A material the board hasn't: a board of birch, 12 mm as the first.
    let birch_stock = PartStock {
        material: Some(birch()),
        thickness: None,
    };
    let (birch_job, moved) = set_part_stock(&job, 2, Some(birch_stock)).unwrap();
    let moved = moved.unwrap();
    let board = birch_job.setup(moved.to).unwrap();
    assert_eq!(board.stock.material, Some(birch()));
    assert_eq!(board.stock.thickness, 12.0);
    // An empty stock is none.
    let (none, _) = set_part_stock(&job, 2, Some(PartStock::default())).unwrap();
    assert_eq!(none.part(2).unwrap().stock, None);
    // A thickness of 0 is refused.
    assert!(set_part_stock(&job, 2, thick(0.0)).is_err());
}

#[test]
fn a_made_board_changed_by_hand_is_kept_and_a_mismatch_is_a_note() {
    let job = two_parts();
    let (moved, made) = set_part_stock(&job, 2, thick(6.0)).unwrap();
    let to = made.unwrap().to;
    // Renamed by hand: the user's now, kept when the part goes back.
    let renamed = chipload_pipeline::setups::set_setup(&moved, to, Some("Bottoms"), None).unwrap();
    assert!(!renamed.setup(to).unwrap().made_for_stock);
    let (back, gone) = set_part_stock(&renamed, 2, None).unwrap();
    // Not on a made board any more: it stays where it is.
    assert!(gone.is_none());
    assert_eq!(back.setups.len(), 2);
    // Placed by hand on a board of another stock: it stays there, with a note.
    let mut by_hand = moved.clone();
    by_hand.move_part_to(2, job.setups[0].id);
    by_hand.active_setup = Some(job.setups[0].id);
    assert_eq!(by_hand.stock_mismatches(job.setups[0].id), [2]);
    let notes = stock_mismatch_notes(&by_hand);
    assert_eq!(notes.len(), 1);
    assert_eq!(
        notes[0].text,
        "part B is to be cut from 6 mm, but this board is 12 mm"
    );
    // The run says so too.
    let result = chipload_pipeline::run_job(&by_hand.for_setup(job.setups[0].id)).unwrap();
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
    let (two, back, _) = chipload_pipeline::setups::add_back_side(
        &job,
        job.setups[0].id,
        chipload_job::Turn::LeftRight,
        None,
    )
    .unwrap();
    let (kept, moved) = set_part_stock(&two, 2, thick(6.0)).unwrap();
    assert!(moved.is_none());
    assert_eq!(kept.setups.len(), 2);
    assert_eq!(kept.stock_mismatches(job.setups[0].id), [2]);
    let _ = back;
}
