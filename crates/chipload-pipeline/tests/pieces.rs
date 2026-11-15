// SPDX-License-Identifier: GPL-3.0-or-later
//! A setup cut in pieces (docs/specs/pieces.md §6, §7): a program per piece, each on its own
//! board with its own zero, checked on its own.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{Job, PieceGrid, WorkZero, load_job};
use chipload_pipeline::pieces::run_pieces;
use chipload_pipeline::{PipelineError, run_setups_pieces};

/// A 900 × 500 mm rounded board cut out, a pocket and the word PUZZLE V-carved across its
/// middle, in knob pieces on 600 × 600 mm boards.
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v37/pieces.chipload"),
    )
    .unwrap()
}

#[test]
fn each_piece_is_a_program_on_its_own_board() {
    let runs = run_pieces(&sample(), None).unwrap();
    let names: Vec<&str> = runs.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["A1", "A2"]);
    for run in &runs {
        let (x, y) = run.job.stock_size();
        assert!(x <= 600.0 && y <= 600.0, "{} on {x} × {y}", run.name);
        // Every operation is cut on every piece: the outline, the pocket and the word cross
        // the seam.
        let ops: std::collections::BTreeSet<usize> =
            run.result.op_spans.iter().map(|s| s.operation).collect();
        assert_eq!(
            ops.into_iter().collect::<Vec<_>>(),
            [0, 1, 2],
            "{}",
            run.name
        );
        // Two tools, two files, each checked.
        assert_eq!(run.result.files.len(), 2);
    }
    // The pieces' boards lie side by side, the second starting left of the seam (its knobs
    // reach into the first).
    let a1 = runs[0].job.stock_bounds();
    let a2 = runs[1].job.stock_bounds();
    assert!(
        a2.0.0 < 450.0 && a2.0.0 > 350.0 && a1.1.0 > 450.0,
        "{a1:?} {a2:?}"
    );
}

#[test]
fn files_are_named_after_their_piece() {
    let runs = run_setups_pieces(&sample(), None, None).unwrap();
    let suffixes: Vec<String> = runs
        .iter()
        .flat_map(|r| {
            r.result
                .files
                .iter()
                .map(|f| f.suffix.clone().unwrap_or_default())
        })
        .collect();
    assert_eq!(suffixes, ["-A1-1-T1", "-A1-2-T2", "-A2-1-T1", "-A2-2-T2"]);
    let one = run_setups_pieces(&sample(), None, Some("A2")).unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].piece.as_deref(), Some("A2"));
    assert!(matches!(
        run_setups_pieces(&sample(), None, Some("C7")),
        Err(PipelineError::Job(_))
    ));
}

#[test]
fn the_same_project_gives_the_same_gcode() {
    let a = run_pieces(&sample(), None).unwrap();
    let b = run_pieces(&sample(), None).unwrap();
    for (x, y) in a.iter().zip(&b) {
        for (f, g) in x.result.files.iter().zip(&y.result.files) {
            assert_eq!(f.gcode, g.gcode);
        }
    }
}

#[test]
fn a_zero_off_a_pieces_board_moves_to_its_corner() {
    let mut job = sample();
    job.setups[0].work_zero = WorkZero::DrawingOrigin {};
    let runs = run_pieces(&job, None).unwrap();
    // The drawing's origin is the first piece's outline corner, inside its board.
    assert_eq!(runs[0].job.work_zero(), WorkZero::DrawingOrigin {});
    assert!(matches!(runs[1].job.work_zero(), WorkZero::Stock { .. }));
    assert!(
        runs[1]
            .result
            .notes
            .iter()
            .any(|n| n.text.contains("off its board")),
        "{:?}",
        runs[1].result.notes
    );
}

#[test]
fn a_grid_given_by_hand_and_a_shifted_seam() {
    let mut job = sample();
    let pieces = job.setups[0].pieces.as_mut().unwrap();
    pieces.grid = Some(PieceGrid {
        columns: 3,
        rows: 1,
    });
    pieces.shift_x = vec![-40.0];
    let runs = run_pieces(&job, None).unwrap();
    assert_eq!(runs.len(), 3);
    // The first seam moved 40 mm left: the first board is narrower than the last.
    let width = |i: usize| runs[i].job.stock_size().0;
    assert!(width(0) + 30.0 < width(2), "{} {}", width(0), width(2));
}

#[test]
fn without_an_outside_profile_there_is_nothing_to_split() {
    let mut job = sample();
    job.operations.remove(0);
    let err = run_pieces(&job, None).unwrap_err();
    assert!(err.to_string().contains("outside profile"), "{err}");
}
