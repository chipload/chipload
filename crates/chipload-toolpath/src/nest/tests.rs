// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::arcs::Circle;
use crate::model::{Move, MoveKind};
use crate::plan::OpSpan;

fn line(a: (f64, f64), b: (f64, f64), radius: f64) -> Capsule {
    Capsule { a, b, radius }
}

/// A square ring, 20 mm a side, cut with a 3 mm tool: a frame with its middle falling out.
fn ring() -> Sweep {
    let corners = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
    Sweep {
        capsules: (0..4)
            .map(|k| line(corners[k], corners[(k + 1) % 4], 1.5))
            .collect(),
    }
}

#[test]
fn a_small_board_gets_half_millimetre_cells_and_a_full_sheet_larger_ones() {
    assert_eq!(cell_size(1220.0, 610.0), CELL_MM);
    let side = cell_size(2440.0, 1220.0);
    assert!(side > CELL_MM);
    assert!((2440.0 / side).ceil() * (1220.0 / side).ceil() <= MAX_CELLS);
    let steps = side / 0.05;
    assert!((steps - steps.round()).abs() < 1e-9, "{side}");
    // Nothing to measure: the usual cell.
    assert_eq!(cell_size(0.0, 0.0), CELL_MM);
}

#[test]
fn a_ring_is_filled_its_box_is_exact() {
    let f = Footprint::new(&ring(), 0.5).unwrap();
    assert_eq!(f.low, (-1.5, -1.5));
    assert_eq!(f.high, (21.5, 21.5));
    assert_eq!((f.grid.cols(), f.grid.rows()), (46, 46));
    // The walls and the middle filled. The tool rounds the outer corners off, but less
    // than a cell's worth each: cell (0, 0) is 1.41 mm from the corner, within the 1.5 mm
    // radius, so a touch occupies it.
    let middle = (10.0 + 1.5) / 0.5;
    assert!(f.grid.get(middle as usize, middle as usize));
    assert!(f.grid.get(0, 0));
    let missing = f.grid.cols() * f.grid.rows() - f.grid.count();
    assert!(
        missing <= 4,
        "at most a cell at each rounded corner: {missing}"
    );
}

#[test]
fn an_open_c_keeps_its_mouth_free() {
    // Three sides of a square: the inside is reached through the open side.
    let corners = [(20.0, 0.0), (0.0, 0.0), (0.0, 20.0), (20.0, 20.0)];
    let sweep = Sweep {
        capsules: (0..3)
            .map(|k| line(corners[k], corners[k + 1], 1.0))
            .collect(),
    };
    let f = Footprint::new(&sweep, 0.5).unwrap();
    let middle = (10.0 + 1.0) / 0.5;
    assert!(!f.grid.get(middle as usize, middle as usize));
}

#[test]
fn a_gap_of_zero_grows_nothing() {
    let f = Footprint::new(&ring(), 0.5).unwrap();
    let grown = f.dilated(0.0);
    assert_eq!(grown.margin, 0);
    assert_eq!(grown.grid, f.grid);
    let grown = f.dilated(3.0);
    assert_eq!(grown.margin, 6);
    assert_eq!(grown.grid.cols(), f.grid.cols() + 12);
}

fn feed(x: f64, y: f64) -> Move {
    Move {
        kind: MoveKind::Feed,
        x: Some(x),
        y: Some(y),
        z: None,
        feed: Some(1000.0),
        circle: None,
        comment: None,
    }
}

fn span(operation: usize, copy: u32, moves: std::ops::Range<usize>, radius: f64) -> OpSpan {
    OpSpan {
        operation,
        pass: None,
        copy,
        moves,
        tool_radius: radius,
    }
}

#[test]
fn a_sweep_takes_the_feeds_of_its_copy_and_kept_operations() {
    let moves = vec![
        Move::rapid_xy((0.0, 0.0)),
        Move {
            kind: MoveKind::Feed,
            x: None,
            y: None,
            z: Some(-3.0),
            feed: Some(300.0),
            circle: None,
            comment: None,
        },
        feed(10.0, 0.0),
        // Operation 1: another tool, dropped by `keep`.
        feed(10.0, 10.0),
        // Copy 2: another copy.
        Move::rapid_xy((50.0, 0.0)),
        feed(60.0, 0.0),
    ];
    let spans = vec![
        span(0, 1, 0..3, 3.0),
        span(1, 1, 3..4, 1.0),
        span(0, 2, 4..6, 3.0),
    ];
    let sweep = Sweep::from_moves(&moves, &spans, 1, |s| s.operation == 0);
    assert_eq!(
        sweep.capsules,
        vec![
            line((0.0, 0.0), (0.0, 0.0), 3.0),
            line((0.0, 0.0), (10.0, 0.0), 3.0)
        ],
        "the plunge a disc, then the line"
    );
    assert_eq!(sweep.extent(), Some(((-3.0, -3.0), (13.0, 3.0))));
    // Copy 2 starts where the program is: from (50, 0).
    let second = Sweep::from_moves(&moves, &spans, 2, |_| true);
    assert_eq!(second.capsules, vec![line((50.0, 0.0), (60.0, 0.0), 3.0)]);
}

#[test]
fn an_arc_move_is_swept_round_its_bulge() {
    // A half circle round (0, 0) of radius 10, counter-clockwise from (10, 0) to (−10, 0):
    // its top at (0, 10) is beyond the chord.
    let moves = vec![
        Move::rapid_xy((10.0, 0.0)),
        Move {
            kind: MoveKind::ArcCcw,
            x: Some(-10.0),
            y: Some(0.0),
            z: None,
            feed: Some(1000.0),
            circle: Some(Circle {
                center: (0.0, 0.0),
                radius: 10.0,
            }),
            comment: None,
        },
    ];
    let sweep = Sweep::from_moves(&moves, &[span(0, 1, 0..2, 1.0)], 1, |_| true);
    let ((_, y0), (_, y1)) = sweep.extent().unwrap();
    assert!(y1 >= 11.0, "{y1}");
    assert!(y0 > -1.1, "nothing below the chord but the tool: {y0}");
}

// -- Step 2: the placer (nesting.md §4.2-4.5)

/// A rectangle's outline cut by a 3 mm tool: its footprint is (w + 3) × (h + 3).
fn rect(w: f64, h: f64) -> Sweep {
    let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    Sweep {
        capsules: (0..4)
            .map(|k| line(corners[k], corners[(k + 1) % 4], 1.5))
            .collect(),
    }
}

fn item(sheet: &Sheet, sweep: &Sweep, angles: &[f64], gap: f64) -> Item {
    let centre = sweep
        .extent()
        .map(|(l, h)| ((l.0 + h.0) / 2.0, (l.1 + h.1) / 2.0))
        .unwrap();
    Item {
        lies: angles
            .iter()
            .map(|&a| {
                let f = Footprint::new(&sweep.turned(a, centre), sheet.cell).unwrap();
                sheet.lie(a, f, gap)
            })
            .collect(),
    }
}

fn run(sheet: &Sheet, items: &[Item]) -> Layout {
    run_for(sheet, items, Goal::Compact)
}

fn run_for(sheet: &Sheet, items: &[Item], goal: Goal) -> Layout {
    fill(sheet, items, goal, &crate::Cancel::new()).unwrap()
}

#[test]
fn one_part_goes_in_the_low_corner() {
    let sheet = Sheet::new((10.0, 20.0), (610.0, 320.0), 0.5, &[]).unwrap();
    for goal in [Goal::Compact, Goal::Low, Goal::Left] {
        let layout = run_for(
            &sheet,
            &[item(&sheet, &rect(100.0, 50.0), &[0.0], 3.0)],
            goal,
        );
        assert_eq!(layout.placed.len(), 1);
        assert_eq!(layout.placed[0].low, (10.0, 20.0));
        assert!((layout.extent.0 - 103.0).abs() < 1e-9);
        assert!((layout.extent.1 - 53.0).abs() < 1e-9);
    }
}

/// Two 300 × 200 footprints, a quarter turn allowed, on a tall board (nesting-search.md §8).
fn two_on_a_tall_board(goal: Goal) -> Layout {
    let sheet = Sheet::new((0.0, 0.0), (650.0, 1200.0), 0.5, &[]).unwrap();
    let quarter = [0.0, std::f64::consts::FRAC_PI_2];
    let part = rect(297.0, 197.0);
    let items = vec![
        item(&sheet, &part, &quarter, 3.0),
        item(&sheet, &part, &quarter, 3.0),
    ];
    let layout = run_for(&sheet, &items, goal);
    assert_eq!(layout.placed.len(), 2, "{layout:?}");
    layout
}

#[test]
fn compact_never_puts_the_second_part_beside_the_first_turned() {
    let layout = two_on_a_tall_board(Goal::Compact);
    let (w, h) = layout.extent;
    // Beside it turned would be a 503.5 × 300 box; above or beside unturned about 120 000.
    assert!(w * h < 125_000.0, "{layout:?}");
    assert!(layout.placed.iter().all(|p| p.lie == 0), "{layout:?}");
}

#[test]
fn low_puts_both_on_the_bottom_and_left_stacks_them() {
    let low = two_on_a_tall_board(Goal::Low);
    assert!(low.placed.iter().all(|p| p.low.1 == 0.0), "{low:?}");
    assert!((low.extent.1 - 200.0).abs() < 1e-9, "{low:?}");
    let left = two_on_a_tall_board(Goal::Left);
    assert!(left.placed.iter().all(|p| p.low.0 == 0.0), "{left:?}");
    assert!((left.extent.0 - 200.0).abs() < 1e-9, "{left:?}");
}

#[test]
fn parts_go_a_gap_apart() {
    for (high, low_goal) in [((600.0, 300.0), true), ((300.0, 600.0), false)] {
        let sheet = Sheet::new((0.0, 0.0), high, 0.5, &[]).unwrap();
        // Each 103 × 203, the long side across the board's short one: two in a row.
        let tall = if low_goal {
            rect(100.0, 200.0)
        } else {
            rect(200.0, 100.0)
        };
        let items = vec![
            item(&sheet, &tall, &[0.0], 3.0),
            item(&sheet, &tall, &[0.0], 3.0),
        ];
        let goal = if low_goal { Goal::Low } else { Goal::Left };
        for goal in [goal, Goal::Compact] {
            let layout = run_for(&sheet, &items, goal);
            assert_eq!(layout.placed.len(), 2);
            let (a, b) = (layout.placed[0].low, layout.placed[1].low);
            let (step, beside) = if low_goal {
                (b.0 - a.0, b.1 - a.1)
            } else {
                (b.1 - a.1, b.0 - a.0)
            };
            assert_eq!(beside, 0.0, "{a:?} {b:?}");
            // 103 + a 3 mm gap, up to one cell more for the rounding.
            assert!((106.0..=106.5).contains(&step), "{step}");
        }
    }
}

#[test]
fn a_part_too_large_is_left_out_and_one_exactly_as_wide_fits() {
    let sheet = Sheet::new((0.0, 0.0), (400.0, 203.0), 0.5, &[]).unwrap();
    let items = vec![
        // Small, but no room is left beside the exact fit.
        item(&sheet, &rect(50.0, 50.0), &[0.0], 3.0),
        // 200.2 + 3 across: wider than the board.
        item(&sheet, &rect(397.0, 200.2), &[0.0], 3.0),
        // 397 + 3 = 400 and 200 + 3 = 203: exactly the board.
        item(&sheet, &rect(397.0, 200.0), &[0.0], 3.0),
    ];
    let layout = run(&sheet, &items);
    assert_eq!(layout.unplaced, vec![0, 1]);
    assert_eq!(layout.placed.len(), 1, "{layout:?}");
    assert_eq!(
        (layout.placed[0].item, layout.placed[0].low),
        (2, (0.0, 0.0))
    );
}

#[test]
fn a_part_that_fits_only_turned_is_turned() {
    let sheet = Sheet::new((0.0, 0.0), (600.0, 120.0), 0.5, &[]).unwrap();
    let quarter = [0.0, std::f64::consts::FRAC_PI_2];
    let layout = run(&sheet, &[item(&sheet, &rect(60.0, 200.0), &quarter, 3.0)]);
    assert_eq!(layout.placed.len(), 1);
    assert_eq!(layout.placed[0].lie, 1);
    // Not allowed to turn: it doesn't fit.
    let layout = run(&sheet, &[item(&sheet, &rect(60.0, 200.0), &[0.0], 3.0)]);
    assert_eq!(layout.unplaced, vec![0]);
}

#[test]
fn a_dowel_hole_is_kept_out_of() {
    // A 6 mm hole drilled at (20, 20): the part goes beside it.
    let hole = Sweep {
        capsules: vec![line((20.0, 20.0), (20.0, 20.0), 3.0)],
    };
    let sheet = Sheet::new((0.0, 0.0), (600.0, 300.0), 0.5, &[hole]).unwrap();
    let layout = run(&sheet, &[item(&sheet, &rect(50.0, 50.0), &[0.0], 3.0)]);
    let low = layout.placed[0].low;
    // The hole reaches 23 along X and across; 3 mm gap: 26 at least, one way or the other.
    assert!(low.0 >= 26.0 || low.1 >= 26.0, "{low:?}");
    // Beside it or above it make the same box: the lower one wins.
    assert_eq!(low.1, 0.0, "{low:?}");
}

#[test]
fn small_parts_fill_in_beside_large_ones() {
    // A 300 deep board: a 203 deep part leaves 94 mm across, enough for the small ones.
    let sheet = Sheet::new((0.0, 0.0), (800.0, 300.0), 0.5, &[]).unwrap();
    let items = vec![
        item(&sheet, &rect(400.0, 200.0), &[0.0], 3.0),
        item(&sheet, &rect(80.0, 80.0), &[0.0], 3.0),
        item(&sheet, &rect(80.0, 80.0), &[0.0], 3.0),
    ];
    let layout = run(&sheet, &items);
    assert!(layout.unplaced.is_empty());
    // The two small ones beside the large one, one above the other: no taller than it.
    assert!((layout.extent.1 - 203.0).abs() < 1e-9, "{layout:?}");
    assert!(layout.extent.0 <= 490.0, "{layout:?}");
}

#[test]
fn a_cancelled_fill_stops() {
    let sheet = Sheet::new((0.0, 0.0), (600.0, 300.0), 0.5, &[]).unwrap();
    let cancel = crate::Cancel::new();
    cancel.cancel();
    let items = [item(&sheet, &rect(10.0, 10.0), &[0.0], 3.0)];
    assert!(matches!(
        fill(&sheet, &items, Goal::Compact, &cancel),
        Err(crate::ToolpathError::Cancelled)
    ));
}
