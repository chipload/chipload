// SPDX-License-Identifier: GPL-3.0-or-later
//! Nesting a setup through the pipeline (docs/specs/nesting.md §4-§6, §13): every
//! instance placed on the board, apart and inside it, the result planned and checked;
//! counts, turns, a board that followed the drawing, the machine's travel, what doesn't fit
//! switched off, and the setups nesting refuses.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_job::{Instance, Job, Mirror, StockOrigin, Travel, Turn, Turns};
use chipload_pipeline::nest::{NestRequest, NestResult, nest_setup};
use chipload_pipeline::run_job;
use chipload_toolpath::Cancel;
use common::{corpus_value, every_direction};

/// The five-operation part on a fixed 1000 × 600 board.
fn part_on_board(x: f64, y: f64) -> Job {
    let mut job = every_direction(None);
    let stock = &mut job.setups[0].stock;
    stock.fit = None;
    stock.origin = StockOrigin::BottomLeft;
    stock.x = x;
    stock.y = y;
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    job
}

fn nest(job: &Job, request: NestRequest) -> NestResult {
    nest_setup(job, &request, &Cancel::new()).unwrap()
}

fn refusal(job: &Job, request: NestRequest) -> String {
    nest_setup(job, &request, &Cancel::new())
        .unwrap_err()
        .to_string()
}

/// Every feed of the program, in drawing coordinates, within the board less `margin`.
fn assert_cut_inside(job: &Job, margin: f64) {
    let run = run_job(job).unwrap();
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let mut at = (None, None);
    for span in &run.op_spans {
        let r = span.tool_radius;
        for m in &run.program.moves[span.moves.clone()] {
            at = (m.x.or(at.0), m.y.or(at.1));
            let (Some(x), Some(y)) = at else { continue };
            if m.kind == chipload_toolpath::model::MoveKind::Rapid {
                continue;
            }
            let tol = 1e-6;
            assert!(
                x - r >= x0 + margin - tol
                    && y - r >= y0 + margin - tol
                    && x + r <= x1 - margin + tol
                    && y + r <= y1 - margin + tol,
                "a cut at {x:.3}, {y:.3} (tool radius {r}) leaves the board less {margin}"
            );
        }
    }
}

#[test]
fn five_of_a_part_are_nested_apart_on_the_board_and_checked() {
    let job = part_on_board(1000.0, 600.0);
    let nested = nest(
        &job,
        NestRequest {
            counts: vec![(1, 5)],
            ..Default::default()
        },
    );
    assert_eq!(nested.count(), 5);
    assert!(nested.unplaced.is_empty(), "{:?}", nested.unplaced);
    assert_eq!(nested.job.parts[0].count(), 5);
    // Kept for Nest again: the defaults.
    let kept = nested.job.setups[0].nest.unwrap();
    assert_eq!((kept.gap, kept.margin), (3.0, 10.0));
    // Planned with the overlap check and checked: the run passes; every cut on the board
    // less its margin.
    assert_cut_inside(&nested.job, 10.0);
    assert!(nested.used > 0.0 && nested.used <= nested.length);
    assert!((nested.length - 980.0).abs() < 1e-9);
    // The same project nested again: the same placements.
    let again = nest(
        &job,
        NestRequest {
            counts: vec![(1, 5)],
            ..Default::default()
        },
    );
    assert_eq!(again.placed, nested.placed);
    let text = nested.summary().english();
    assert!(
        text.starts_with("5 of 5 placed, the board used to "),
        "{text}"
    );
}

#[test]
fn turns_are_kept_per_part_and_none_keeps_it_as_it_lies() {
    let job = part_on_board(1000.0, 600.0);
    let nested = nest(
        &job,
        NestRequest {
            counts: vec![(1, 4)],
            turns: Some(Turns::None),
            ..Default::default()
        },
    );
    assert_eq!(nested.job.parts[0].turns, Some(Turns::None));
    assert!(
        nested.placed.iter().all(|p| p.angle == 0.0),
        "{:?}",
        nested.placed
    );
    // Saved and read back: the turns and the settings stay.
    let text = serde_json::to_string_pretty(&nested.job).unwrap();
    assert!(text.contains("\"turns\": \"none\""), "{text}");
    assert!(text.contains("\"nest\": {"), "{text}");
}

#[test]
fn a_mirrored_instance_stays_mirrored() {
    let mut job = part_on_board(1000.0, 600.0);
    job.parts[0].instances = vec![Instance {
        id: 2,
        enabled: true,
        mirror: Some(Mirror::LeftRight),
        angle: 0.0,
        at: [0.0, 0.0],
    }];
    let nested = nest(&job, NestRequest::default());
    assert_eq!(nested.placed.len(), 2);
    assert_eq!(
        nested.job.parts[0].instances[0].mirror,
        Some(Mirror::LeftRight)
    );
    assert_cut_inside(&nested.job, 10.0);
}

#[test]
fn counts_add_and_remove_instances_never_the_part_itself() {
    let mut job = part_on_board(1000.0, 600.0);
    job.parts[0].instances = vec![
        Instance {
            id: 2,
            enabled: true,
            mirror: None,
            angle: 0.0,
            at: [300.0, 0.0],
        },
        Instance {
            id: 3,
            enabled: false,
            mirror: None,
            angle: 0.0,
            at: [600.0, 0.0],
        },
    ];
    // Fewer: the highest-numbered enabled one goes; the one that is off stays.
    let nested = nest(
        &job,
        NestRequest {
            counts: vec![(1, 1)],
            ..Default::default()
        },
    );
    let ids: Vec<u32> = nested.job.parts[0].instances.iter().map(|i| i.id).collect();
    assert_eq!(ids, vec![3]);
    assert_eq!(nested.job.parts[0].count(), 1);
    // None at all: refused.
    let err = refusal(
        &job,
        NestRequest {
            counts: vec![(1, 0)],
            ..Default::default()
        },
    );
    assert!(err.contains("is cut at least once"), "{err}");
}

#[test]
fn what_doesnt_fit_is_switched_off_and_the_part_stays_on() {
    // About two fit on a 300 × 200 board.
    let job = part_on_board(300.0, 200.0);
    let nested = nest(
        &job,
        NestRequest {
            counts: vec![(1, 6)],
            ..Default::default()
        },
    );
    assert!(!nested.unplaced.is_empty());
    assert_eq!(nested.placed.len() + nested.unplaced.len(), 6);
    let part = &nested.job.parts[0];
    // The part itself is among those placed: it stays on.
    assert!(part.enabled);
    assert!(
        nested.unplaced.iter().all(|&(_, n)| n != 1),
        "{:?}",
        nested.unplaced
    );
    for &(_, n) in &nested.unplaced {
        assert!(!part.instances.iter().find(|i| i.id == n).unwrap().enabled);
    }
    assert_eq!(part.count() as usize, nested.placed.len());
    assert_cut_inside(&nested.job, 10.0);
    let notes: Vec<String> = nested.notes().iter().map(|m| m.english()).collect();
    assert!(
        notes.iter().any(|n| n.contains("didn't fit: switched off")),
        "{notes:?}"
    );
}

#[test]
fn a_board_that_follows_the_drawing_needs_a_size_and_gets_it() {
    let mut job = every_direction(None);
    job.setups[0].stock.fit = Some(5.0);
    job.setups[0].stock.origin = StockOrigin::Drawing;
    let err = refusal(&job, NestRequest::default());
    assert!(err.contains("--board"), "{err}");
    let nested = nest(
        &job,
        NestRequest {
            board: Some((800.0, 400.0)),
            counts: vec![(1, 3)],
            ..Default::default()
        },
    );
    assert!(nested.board_fixed);
    let stock = &nested.job.setups[0].stock;
    assert_eq!((stock.fit, stock.origin), (None, StockOrigin::BottomLeft));
    assert_eq!((stock.x, stock.y), (800.0, 400.0));
    assert_eq!(nested.job.stock_bounds(), ((0.0, 0.0), (800.0, 400.0)));
    // The drawing doesn't move.
    assert_eq!(nested.job.drawing.shapes, job.drawing.shapes);
    assert_cut_inside(&nested.job, 10.0);
}

#[test]
fn only_as_much_board_as_the_machine_travels_is_used() {
    let mut job = part_on_board(1200.0, 400.0);
    job.machine.travel = Some(Travel {
        x: 500.0,
        y: 800.0,
        z: 100.0,
    });
    let nested = nest(
        &job,
        NestRequest {
            counts: vec![(1, 6)],
            ..Default::default()
        },
    );
    assert_eq!(nested.travel.len(), 1);
    assert_eq!(nested.travel[0].axis, 'X');
    // From the board's left edge (work zero at the drawing origin): every placed cut ends
    // within X 500, and the checker's travel check passes.
    let run = run_job(&nested.job).unwrap();
    let max_x = run
        .program
        .moves
        .iter()
        .filter_map(|m| m.x)
        .fold(f64::MIN, f64::max);
    assert!(max_x <= 500.0, "{max_x}");
    assert!((nested.length - 490.0).abs() < 1e-9, "{}", nested.length);
}

#[test]
fn setups_in_pieces_or_two_sided_and_empty_ones_are_refused() {
    let pieces = chipload_job::parse_job(corpus_value("jobs/v37/pieces.chipload"), "t").unwrap();
    let err = refusal(&pieces, NestRequest::default());
    assert!(err.contains("in pieces can't be nested"), "{err}");
    let job = part_on_board(1000.0, 600.0);
    let (two, back, _) =
        chipload_pipeline::setups::add_back_side(&job, 1, Turn::LeftRight, None).unwrap();
    for setup in [1, back] {
        let err = refusal(
            &two,
            NestRequest {
                setup: Some(setup),
                ..Default::default()
            },
        );
        assert!(err.contains("two-sided setup can't be nested"), "{err}");
    }
    let mut off = job.clone();
    off.parts[0].enabled = false;
    let err = refusal(&off, NestRequest::default());
    assert!(err.contains("no part that is on"), "{err}");
    let err = refusal(
        &job,
        NestRequest {
            margin: Some(400.0),
            ..Default::default()
        },
    );
    assert!(err.contains("No room is left"), "{err}");
}

#[test]
fn the_setup_sheet_says_it_was_nested_and_how_much_board_is_used() {
    use chipload_pipeline::sheet::{SheetInfo, setup_sheet};
    let job = part_on_board(1000.0, 600.0);
    let info = SheetInfo {
        project: "t".into(),
        written_by: "chipload (test)".into(),
        units: chipload_job::Units::Mm,
        language: chipload_i18n::Language::En,
        parts: Vec::new(),
    };
    let out = std::path::Path::new("t.nc");
    // Not nested: no such fact.
    let sheet = setup_sheet(&job, &run_job(&job).unwrap(), out, &info);
    assert!(!sheet.contains("Nested"), "{sheet}");
    let nested = nest(
        &job,
        NestRequest {
            counts: vec![(1, 4)],
            ..Default::default()
        },
    );
    let sheet = setup_sheet(&nested.job, &run_job(&nested.job).unwrap(), out, &info);
    assert!(sheet.contains("Nested"), "{sheet}");
    assert!(
        sheet.contains("4 instances nested 3 mm apart and 10 mm from the edge; the cuts reach "),
        "{sheet}"
    );
    // The sheet is HTML: the apostrophe is escaped.
    assert!(sheet.contains(" of the board&#39;s 1000 mm."), "{sheet}");
}
