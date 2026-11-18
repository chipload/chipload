// SPDX-License-Identifier: GPL-3.0-or-later
//! The part view (docs/specs/parts-and-setups.md §8a): a part shown alone, once, where it
//! is drawn, on the board it is cut from.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{Job, PartStock, load_job};
use chipload_pipeline::part_view::part_view;
use chipload_pipeline::run_job;

/// Two parts on one 12 mm board: "simple part" (operations 1-3) and "part B" (4), each
/// drawn in place.
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v22/two_parts.chipload"),
    )
    .unwrap()
}

/// The operations a run of `job` cut, by index.
fn cut(job: &Job) -> Vec<usize> {
    let result = run_job(job).unwrap();
    let mut ops: Vec<usize> = result.op_spans.iter().map(|s| s.operation).collect();
    ops.dedup();
    ops
}

#[test]
fn a_part_drawn_in_place_is_shown_alone_on_its_own_board() {
    let job = sample();
    let view = part_view(&job, 2).unwrap();
    assert_eq!(cut(&view), [3]);
    // The board as the setup has it: the simple case doesn't jump.
    assert_eq!(view.stock_bounds(), job.stock_bounds());
    assert_eq!(view.machine.travel, job.machine.travel);
    assert_eq!(view.active_part, Some(2));
    // The project itself is untouched.
    assert_eq!(cut(&job), [0, 1, 2, 3]);
}

#[test]
fn copies_and_a_moved_part_are_shown_once_where_drawn_on_a_fitted_board() {
    let mut job = sample();
    let drawn = job.part_box(2).unwrap();
    job.instance_mut(2, 1).unwrap().at = [900.0, 0.0];
    let view = part_view(&job, 2).unwrap();
    let here = view.setups[0]
        .instances
        .iter()
        .filter(|i| i.part == 2)
        .collect::<Vec<_>>();
    assert_eq!(here.len(), 1);
    assert!(here[0].is_drawn());
    // Fitted round its shapes with the default margin, travel not checked.
    let ((x0, y0), (x1, y1)) = view.stock_bounds();
    let ((dx0, dy0), (dx1, dy1)) = drawn;
    assert_eq!(
        (x0, y0, x1, y1),
        (dx0 - 10.0, dy0 - 10.0, dx1 + 10.0, dy1 + 10.0)
    );
    assert_eq!(view.machine.travel, None);
    let result = run_job(&view).unwrap();
    let mut xs = result.program.moves.iter().filter_map(|m| m.x);
    assert!(xs.all(|x| x < dx1 + 10.0), "cut where drawn");
}

#[test]
fn a_parts_own_thickness_is_the_boards_and_its_through_cuts_follow() {
    let mut job = sample();
    for op in &mut job.operations {
        if let Some(t) = op.through_mut() {
            *t = Some(0.0);
        }
    }
    job.follow_thickness();
    job.parts[1].stock = Some(PartStock {
        material: None,
        thickness: Some(6.0),
    });
    let view = part_view(&job, 2).unwrap();
    assert_eq!(view.setups[0].stock.thickness, 6.0);
    assert_eq!(view.operations[3].depths(&view.tools).1, 6.0);
}

#[test]
fn no_such_part_no_view() {
    assert!(part_view(&sample(), 99).is_none());
}
