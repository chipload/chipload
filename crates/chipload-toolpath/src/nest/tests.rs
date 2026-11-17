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
