// SPDX-License-Identifier: GPL-3.0-or-later
//! A footprint (docs/specs/nesting.md §3, §4.1): the cells a sweep touches, its holes
//! filled, and the same grown by the gap.

use chipload_core::Point;
use chipload_core::math;

use super::grid::Grid;
use super::sweep::{Capsule, Sweep};

/// Where a grid lies in the drawing: the low corner of its cell (0, 0) and the side of a
/// cell, mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub origin: Point,
    pub cell: f64,
}

impl Frame {
    /// Cell (`col`, `row`) as its low and high corners.
    pub fn cell_box(&self, col: usize, row: usize) -> (Point, Point) {
        let (x, y) = (
            self.origin.0 + col as f64 * self.cell,
            self.origin.1 + row as f64 * self.cell,
        );
        ((x, y), (x + self.cell, y + self.cell))
    }

    /// Every cell of `grid` the capsule touches made occupied: a cell is touched when any
    /// point of it lies within the capsule's radius of its segment (a touch counts), so
    /// rounding only ever adds room.
    pub fn add_capsule(&self, grid: &mut Grid, capsule: &Capsule) {
        let Capsule { a, b, radius } = *capsule;
        let x = (a.0.min(b.0) - radius, a.0.max(b.0) + radius);
        let y = (a.1.min(b.1) - radius, a.1.max(b.1) + radius);
        let Some((c0, c1)) = self.span(x, self.origin.0, grid.cols()) else {
            return;
        };
        let Some((r0, r1)) = self.span(y, self.origin.1, grid.rows()) else {
            return;
        };
        for row in r0..=r1 {
            for col in c0..=c1 {
                if segment_box_distance(a, b, self.cell_box(col, row)) <= radius {
                    grid.set(col, row);
                }
            }
        }
    }

    /// Every capsule of `sweep` added.
    pub fn add_sweep(&self, grid: &mut Grid, sweep: &Sweep) {
        for capsule in &sweep.capsules {
            self.add_capsule(grid, capsule);
        }
    }

    /// The cells from `low` to `high` along one axis of `count` cells starting at
    /// `origin`, clamped to the grid; None when the range misses it.
    fn span(&self, (low, high): (f64, f64), origin: f64, count: usize) -> Option<(usize, usize)> {
        let index = |at: f64| ((at - origin) / self.cell).floor();
        let (first, last) = (index(low), index(high));
        if !(first.is_finite() && last.is_finite()) || count == 0 {
            return None;
        }
        let max = (count - 1) as f64;
        if last < 0.0 || first > max {
            return None;
        }
        Some((first.max(0.0) as usize, last.min(max) as usize))
    }
}

/// What one instance cuts at one turn, rasterised: `grid` in `frame`, whose origin is the
/// low corner of the sweep's exact box (`low`, `high`). Holes are filled: the part and
/// what falls out of it are inside its footprint (nesting.md §3).
#[derive(Debug, Clone, PartialEq)]
pub struct Footprint {
    pub low: Point,
    pub high: Point,
    pub frame: Frame,
    pub grid: Grid,
}

/// A footprint grown by a gap: its cell (`col`, `row`) is the footprint's cell
/// (`col − margin`, `row − margin`).
#[derive(Debug, Clone, PartialEq)]
pub struct Dilated {
    pub grid: Grid,
    pub margin: usize,
}

impl Footprint {
    /// The footprint of `sweep` on cells of side `cell` mm; None when it sweeps nothing
    /// or the cell isn't a positive size.
    pub fn new(sweep: &Sweep, cell: f64) -> Option<Footprint> {
        if !(cell.is_finite() && cell > 0.0) {
            return None;
        }
        let (low, high) = sweep.extent()?;
        let count = |size: f64| (size / cell).ceil().max(1.0);
        let (cols, rows) = (count(high.0 - low.0), count(high.1 - low.1));
        if !(cols.is_finite() && rows.is_finite()) {
            return None;
        }
        let frame = Frame { origin: low, cell };
        let mut grid = Grid::new(cols as usize, rows as usize);
        frame.add_sweep(&mut grid, sweep);
        fill_holes(&mut grid);
        Some(Footprint {
            low,
            high,
            frame,
            grid,
        })
    }

    /// Width and height of the exact box, mm.
    pub fn size(&self) -> (f64, f64) {
        (self.high.0 - self.low.0, self.high.1 - self.low.1)
    }

    /// The footprint grown by `gap` mm: every cell nearer than `gap` to an occupied one is
    /// occupied too, the distance between two cells being the least between their squares.
    /// So two footprints whose grown and plain cells don't meet are at least `gap` apart.
    pub fn dilated(&self, gap: f64) -> Dilated {
        let offsets = disc_offsets(gap, self.frame.cell);
        let margin = offsets
            .iter()
            .map(|&(dx, dy)| dx.unsigned_abs().max(dy.unsigned_abs()))
            .max()
            .unwrap_or(0) as usize;
        let (cols, rows) = (self.grid.cols(), self.grid.rows());
        let mut grown = Grid::new(cols + 2 * margin, rows + 2 * margin);
        let m = margin as i64;
        grown.or_at(&self.grid, m, m);
        // Only cells on the edge need stamping: a cell near an inner one is nearer to,
        // or as near to, an edge cell between them.
        for row in 0..rows {
            for col in 0..cols {
                if !self.grid.get(col, row) || !on_edge(&self.grid, col, row) {
                    continue;
                }
                for &(dx, dy) in &offsets {
                    let (c, r) = (col as i64 + m + dx, row as i64 + m + dy);
                    if c >= 0 && r >= 0 {
                        grown.set(c as usize, r as usize);
                    }
                }
            }
        }
        Dilated {
            grid: grown,
            margin,
        }
    }
}

/// The cell offsets (dx, dy) nearer than `gap` to cell (0, 0), squares apart: the gap
/// between them is (|dx| − 1, |dy| − 1) cells, none when they touch. Always (0, 0); with a
/// gap of 0 only it, since footprints may then touch but not share a cell.
fn disc_offsets(gap: f64, cell: f64) -> Vec<(i64, i64)> {
    if !(gap.is_finite() && gap > 0.0 && cell > 0.0) {
        return vec![(0, 0)];
    }
    let k = (gap / cell).ceil() as i64;
    let apart = |d: i64| (d.abs() - 1).max(0) as f64 * cell;
    let mut offsets = Vec::new();
    for dy in -k..=k {
        for dx in -k..=k {
            if math::hypot(apart(dx), apart(dy)) < gap {
                offsets.push((dx, dy));
            }
        }
    }
    offsets
}

/// Whether occupied cell (`col`, `row`) has a free neighbour across a side, or the grid's
/// edge.
fn on_edge(grid: &Grid, col: usize, row: usize) -> bool {
    col == 0
        || row == 0
        || col + 1 == grid.cols()
        || row + 1 == grid.rows()
        || !grid.get(col - 1, row)
        || !grid.get(col + 1, row)
        || !grid.get(col, row - 1)
        || !grid.get(col, row + 1)
}

/// Free cells that can't be reached from outside the grid, stepping across sides only,
/// made occupied. A diagonal touch of two occupied cells counts as closed, which only ever
/// fills more.
fn fill_holes(grid: &mut Grid) {
    let (cols, rows) = (grid.cols(), grid.rows());
    let mut outside = Grid::new(cols, rows);
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let reach = |col: usize, row: usize, outside: &mut Grid, stack: &mut Vec<_>| {
        if !grid.get(col, row) && !outside.get(col, row) {
            outside.set(col, row);
            stack.push((col, row));
        }
    };
    for col in 0..cols {
        reach(col, 0, &mut outside, &mut stack);
        reach(col, rows.saturating_sub(1), &mut outside, &mut stack);
    }
    for row in 0..rows {
        reach(0, row, &mut outside, &mut stack);
        reach(cols.saturating_sub(1), row, &mut outside, &mut stack);
    }
    while let Some((col, row)) = stack.pop() {
        if col > 0 {
            reach(col - 1, row, &mut outside, &mut stack);
        }
        if col + 1 < cols {
            reach(col + 1, row, &mut outside, &mut stack);
        }
        if row > 0 {
            reach(col, row - 1, &mut outside, &mut stack);
        }
        if row + 1 < rows {
            reach(col, row + 1, &mut outside, &mut stack);
        }
    }
    for row in 0..rows {
        for col in 0..cols {
            if !outside.get(col, row) {
                grid.set(col, row);
            }
        }
    }
}

/// The least distance from the segment `a`–`b` to the box (`low`, `high`): 0 when they
/// meet. Apart, the nearest points include a corner of the box or an end of the segment.
pub(super) fn segment_box_distance(a: Point, b: Point, (low, high): (Point, Point)) -> f64 {
    if segment_meets_box(a, b, low, high) {
        return 0.0;
    }
    let corners = [low, (high.0, low.1), high, (low.0, high.1)];
    let to_corners = corners
        .iter()
        .map(|&c| point_segment_distance(c, a, b))
        .fold(f64::INFINITY, f64::min);
    to_corners
        .min(point_box_distance(a, low, high))
        .min(point_box_distance(b, low, high))
}

/// Whether the segment meets the closed box: clipped against each side in turn
/// (Liang–Barsky).
fn segment_meets_box(a: Point, b: Point, low: Point, high: Point) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.0 - low.0),
        (dx, high.0 - a.0),
        (-dy, a.1 - low.1),
        (dy, high.1 - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
        if t0 > t1 {
            return false;
        }
    }
    true
}

fn point_box_distance(p: Point, low: Point, high: Point) -> f64 {
    let dx = (low.0 - p.0).max(0.0).max(p.0 - high.0);
    let dy = (low.1 - p.1).max(0.0).max(p.1 - high.1);
    math::hypot(dx, dy)
}

pub(super) fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    math::hypot(p.0 - (a.0 + t * dx), p.1 - (a.1 + t * dy))
}
