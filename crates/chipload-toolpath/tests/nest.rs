// SPDX-License-Identifier: GPL-3.0-or-later
//! Nesting's footprints (docs/specs/nesting.md §4.1, §13): rounding only ever adds room,
//! the gap holds when grown footprints don't meet, the bit grid shifts as cell by cell.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::Point;
use chipload_core::math;
use chipload_toolpath::nest::{Capsule, Footprint, Frame, Grid, Sweep};
use proptest::prelude::*;

fn point_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    math::hypot(p.0 - (a.0 + t * dx), p.1 - (a.1 + t * dy))
}

fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// The least distance between two segments: 0 when they cross.
fn segment_segment(a: Point, b: Point, c: Point, d: Point) -> f64 {
    let crosses = cross(a, b, c) * cross(a, b, d) < 0.0 && cross(c, d, a) * cross(c, d, b) < 0.0;
    if crosses {
        return 0.0;
    }
    point_segment(a, c, d)
        .min(point_segment(b, c, d))
        .min(point_segment(c, a, b))
        .min(point_segment(d, a, b))
}

/// The least distance between what two sweeps cover, the second moved by `by`.
fn sweep_distance(a: &Sweep, b: &Sweep, by: Point) -> f64 {
    let moved = |p: Point| (p.0 + by.0, p.1 + by.1);
    let mut least = f64::INFINITY;
    for p in &a.capsules {
        for q in &b.capsules {
            let d = segment_segment(p.a, p.b, moved(q.a), moved(q.b)) - p.radius - q.radius;
            least = least.min(d);
        }
    }
    least
}

fn capsule() -> impl Strategy<Value = Capsule> {
    (
        (-40.0..40.0f64, -40.0..40.0f64),
        (-40.0..40.0f64, -40.0..40.0f64),
        0.2..4.0f64,
        any::<bool>(),
    )
        .prop_map(|(a, b, radius, plunge)| Capsule {
            a,
            b: if plunge { a } else { b },
            radius,
        })
}

fn sweep() -> impl Strategy<Value = Sweep> {
    prop::collection::vec(capsule(), 1..6).prop_map(|capsules| Sweep { capsules })
}

fn grid(max_cols: usize) -> impl Strategy<Value = Grid> {
    (1..max_cols, 1..12usize).prop_flat_map(|(cols, rows)| {
        prop::collection::vec(any::<bool>(), cols * rows).prop_map(move |cells| {
            let mut grid = Grid::new(cols, rows);
            for (k, on) in cells.into_iter().enumerate() {
                if on {
                    grid.set(k % cols, k / cols);
                }
            }
            grid
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Every point a capsule covers lies in an occupied cell.
    #[test]
    fn every_point_swept_is_in_the_footprint(
        sweep in sweep(),
        cell in prop::sample::select(vec![0.5, 0.75, 1.3]),
        samples in prop::collection::vec((0.0..1.0f64, 0.0..1.0f64, 0.0..1.0f64), 40),
    ) {
        let footprint = Footprint::new(&sweep, cell).unwrap();
        for (k, (t, turn, out)) in samples.into_iter().enumerate() {
            let c = &sweep.capsules[k % sweep.capsules.len()];
            let on = (c.a.0 + t * (c.b.0 - c.a.0), c.a.1 + t * (c.b.1 - c.a.1));
            let (sin, cos) = math::sin_cos(turn * std::f64::consts::TAU);
            let p = (on.0 + out * c.radius * cos, on.1 + out * c.radius * sin);
            let col = ((p.0 - footprint.low.0) / cell).floor();
            let row = ((p.1 - footprint.low.1) / cell).floor();
            prop_assert!(col >= 0.0 && row >= 0.0, "{p:?} below the box");
            prop_assert!(
                footprint.grid.get(col as usize, row as usize),
                "{p:?} in cell {col}, {row} not occupied"
            );
        }
    }

    /// A capsule occupies only cells it touches: none whose centre is farther than its
    /// radius and half a cell's diagonal.
    #[test]
    fn a_capsule_occupies_only_cells_it_touches(
        c in capsule(),
        cell in prop::sample::select(vec![0.5, 1.0, 2.5]),
    ) {
        let frame = Frame { origin: (-50.0, -50.0), cell };
        let n = (100.0 / cell) as usize;
        let mut grid = Grid::new(n, n);
        frame.add_capsule(&mut grid, &c);
        let reach = c.radius + cell * std::f64::consts::SQRT_2 / 2.0;
        for row in 0..n {
            for col in 0..n {
                if grid.get(col, row) {
                    let ((x0, y0), (x1, y1)) = frame.cell_box(col, row);
                    let centre = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                    prop_assert!(point_segment(centre, c.a, c.b) <= reach + 1e-9);
                }
            }
        }
    }

    /// Grown by the gap, a footprint keeps any other whose plain cells it doesn't meet
    /// at least the gap away, measured exactly on the capsules. Along a row every column
    /// is tried, so the nearest places that don't meet are among them.
    #[test]
    fn grown_footprints_that_dont_meet_are_a_gap_apart(
        a in sweep(),
        b in sweep(),
        gap in prop::sample::select(vec![0.0, 0.3, 3.0, 6.2]),
        cell in prop::sample::select(vec![0.5, 1.0]),
        row in 0.0..1.0f64,
    ) {
        let fa = Footprint::new(&a, cell).unwrap();
        let fb = Footprint::new(&b, cell).unwrap();
        let grown = fa.dilated(gap);
        let m = grown.margin as i64;
        let (gc, gr) = (grown.grid.cols() as i64, grown.grid.rows() as i64);
        let (bc, br) = (fb.grid.cols() as i64, fb.grid.rows() as i64);
        // B's cell (0, 0) on grown A's row `r`: one where they share rows.
        let r = -br + 1 + ((gr + br - 2) as f64 * row) as i64;
        let mut tried = 0;
        for c in -bc..=gc {
            if grown.grid.overlaps_at(&fb.grid, c, r) {
                continue;
            }
            let by = (
                fa.low.0 + (c - m) as f64 * cell - fb.low.0,
                fa.low.1 + (r - m) as f64 * cell - fb.low.1,
            );
            let d = sweep_distance(&a, &b, by);
            prop_assert!(d >= gap - 1e-9, "{d} < {gap} at {c}, {r}");
            tried += 1;
        }
        prop_assert!(tried > 0);
    }

    /// Growing from the edge cells only gives what growing from every cell gives.
    #[test]
    fn growing_from_the_edge_is_growing_from_every_cell(
        sweep in sweep(),
        gap in prop::sample::select(vec![0.0, 0.4, 1.0, 2.6]),
    ) {
        let cell = 1.0;
        let f = Footprint::new(&sweep, cell).unwrap();
        let grown = f.dilated(gap);
        let m = grown.margin as i64;
        let apart = |d: i64| (d.abs() - 1).max(0) as f64 * cell;
        for row in 0..grown.grid.rows() as i64 {
            for col in 0..grown.grid.cols() as i64 {
                let (fc, fr) = (col - m, row - m);
                let near = (0..f.grid.rows() as i64).any(|r| {
                    (0..f.grid.cols() as i64).any(|c| {
                        f.grid.get(c as usize, r as usize)
                            && ((c, r) == (fc, fr)
                                || math::hypot(apart(c - fc), apart(r - fr)) < gap)
                    })
                });
                prop_assert_eq!(grown.grid.get(col as usize, row as usize), near);
            }
        }
    }

    /// A grid laid on another word by word meets and marks the same cells as cell by cell.
    #[test]
    fn shifted_grids_meet_and_mark_as_cell_by_cell(
        board in grid(150),
        part in grid(140),
        at in (-150i64..160, -14i64..14),
    ) {
        let naive_hit = (0..part.rows()).any(|r| {
            (0..part.cols()).any(|c| {
                let (bc, br) = (c as i64 + at.0, r as i64 + at.1);
                part.get(c, r)
                    && bc >= 0
                    && br >= 0
                    && board.get(bc as usize, br as usize)
            })
        });
        prop_assert_eq!(board.overlaps_at(&part, at.0, at.1), naive_hit);
        let mut marked = board.clone();
        marked.or_at(&part, at.0, at.1);
        for r in 0..board.rows() {
            for c in 0..board.cols() {
                let (pc, pr) = (c as i64 - at.0, r as i64 - at.1);
                let from_part = pc >= 0 && pr >= 0 && part.get(pc as usize, pr as usize);
                prop_assert_eq!(marked.get(c, r), board.get(c, r) || from_part);
            }
        }
        // Nothing beyond the last column: the count is the cells' count.
        let cells = (0..board.rows())
            .flat_map(|r| (0..board.cols()).map(move |c| (c, r)))
            .filter(|&(c, r)| marked.get(c, r))
            .count();
        prop_assert_eq!(marked.count(), cells);
    }

    /// A quarter turn swaps the footprint's width and height exactly.
    #[test]
    fn a_quarter_turn_swaps_width_and_height(sweep in sweep()) {
        let f = Footprint::new(&sweep, 0.5).unwrap();
        let turned = Footprint::new(&sweep.turned(std::f64::consts::FRAC_PI_2, (3.0, -7.0)), 0.5)
            .unwrap();
        let ((w, h), (tw, th)) = (f.size(), turned.size());
        prop_assert!((w - th).abs() < 1e-9 && (h - tw).abs() < 1e-9);
    }
}
