// SPDX-License-Identifier: GPL-3.0-or-later
//! Adaptive pocket clearing (docs/specs/adaptive.md), designed and written for Chipload.
//!
//! The pocket's material is a grid of cells, one bit each (1 = still to cut). The tool
//! follows the edge of the uncut material, keeping it on one side, and at every step turns
//! so that the material it newly cuts is close to the target (load × step). When the
//! material near the path is gone it starts again at the nearest uncut cell. The path never
//! leaves the area the tool centre may be in; whatever this simulation decides, the moves are
//! verified against the walls afterwards like every other pocket.

use std::f64::consts::FRAC_PI_2;

use chipload_core::Point;
use chipload_core::primitives::distance;

use crate::geom::PrimIndex;

/// Most steps in one pocket, against runaway loops on absurd input.
const MAX_STEPS: usize = 2_000_000;
/// Bisection rounds when choosing a direction (angle resolution π / 2^6, under 3°).
const STEER_ROUNDS: usize = 6;
/// Below this share of the target a step counts as not cutting: the tool seeks material.
const SEEK_BELOW: f64 = 0.25;

/// Cells of the pocket, one bit each, row by row.
#[derive(Debug, Clone)]
pub struct Grid {
    x0: f64,
    y0: f64,
    /// Cell side, mm.
    h: f64,
    cols: usize,
    rows: usize,
    words: usize,
    bits: Vec<u64>,
}

impl Grid {
    /// An empty grid covering (lo, hi) with cells of side `h`.
    pub fn new(lo: Point, hi: Point, h: f64) -> Self {
        let cols = (((hi.0 - lo.0) / h).ceil() as usize).max(1) + 2;
        let rows = (((hi.1 - lo.1) / h).ceil() as usize).max(1) + 2;
        let words = cols.div_ceil(64);
        Grid {
            x0: lo.0 - h,
            y0: lo.1 - h,
            h,
            cols,
            rows,
            words,
            bits: vec![0; words * rows],
        }
    }

    pub fn cell_side(&self) -> f64 {
        self.h
    }

    fn centre(&self, col: usize, row: usize) -> Point {
        (
            self.x0 + (col as f64 + 0.5) * self.h,
            self.y0 + (row as f64 + 0.5) * self.h,
        )
    }

    /// Columns whose centres lie in [a, b] (clipped), on any row.
    fn cols_between(&self, a: f64, b: f64) -> Option<(usize, usize)> {
        let first = ((a - self.x0) / self.h - 0.5).ceil().max(0.0);
        let last = ((b - self.x0) / self.h - 0.5).floor();
        if last < first || first >= self.cols as f64 {
            return None;
        }
        let last = last.min(self.cols as f64 - 1.0);
        Some((first as usize, last as usize))
    }

    /// Rows whose centres lie in [a, b] (clipped).
    fn rows_between(&self, a: f64, b: f64) -> Option<(usize, usize)> {
        let first = ((a - self.y0) / self.h - 0.5).ceil().max(0.0);
        let last = ((b - self.y0) / self.h - 0.5).floor();
        if last < first || first >= self.rows as f64 {
            return None;
        }
        let last = last.min(self.rows as f64 - 1.0);
        Some((first as usize, last as usize))
    }

    /// Mask of bits `from..=to` within one word.
    fn mask(from: usize, to: usize) -> u64 {
        let high = if to == 63 {
            u64::MAX
        } else {
            (1u64 << (to + 1)) - 1
        };
        high & !((1u64 << from) - 1)
    }

    /// Visit the words of columns `a..=b` on `row` with their masks.
    fn span(&self, row: usize, a: usize, b: usize, mut visit: impl FnMut(usize, u64)) {
        let base = row * self.words;
        let (wa, wb) = (a / 64, b / 64);
        for w in wa..=wb {
            let from = if w == wa { a % 64 } else { 0 };
            let to = if w == wb { b % 64 } else { 63 };
            visit(base + w, Self::mask(from, to));
        }
    }

    /// Set the cells whose centres lie in [a, b] on `row`.
    fn set_span(&mut self, row: usize, a: f64, b: f64) {
        if let Some((ca, cb)) = self.cols_between(a, b) {
            let mut words = Vec::new();
            self.span(row, ca, cb, |i, m| words.push((i, m)));
            for (i, m) in words {
                self.bits[i] |= m;
            }
        }
    }

    /// Visit the rows of the disk of radius `r` round `c` as column ranges.
    fn disk(&self, c: Point, r: f64, mut visit: impl FnMut(usize, usize, usize)) {
        let Some((first, last)) = self.rows_between(c.1 - r, c.1 + r) else {
            return;
        };
        for row in first..=last {
            let dy = self.y0 + (row as f64 + 0.5) * self.h - c.1;
            let half = (r * r - dy * dy).max(0.0).sqrt();
            if half > 0.0
                && let Some((ca, cb)) = self.cols_between(c.0 - half, c.0 + half)
            {
                visit(row, ca, cb);
            }
        }
    }

    /// Uncut cells within `r` of `c`.
    pub fn count_disk(&self, c: Point, r: f64) -> u32 {
        let mut count = 0;
        self.disk(c, r, |row, ca, cb| {
            self.span(row, ca, cb, |i, m| count += (self.bits[i] & m).count_ones());
        });
        count
    }

    /// Mark the cells within `r` of `c` as cut.
    pub fn clear_disk(&mut self, c: Point, r: f64) {
        let mut words: Vec<(usize, u64)> = Vec::new();
        self.disk(c, r, |row, ca, cb| {
            self.span(row, ca, cb, |i, m| words.push((i, m)))
        });
        for (i, m) in words {
            self.bits[i] &= !m;
        }
    }

    /// Sum of the positions of the uncut cells within `r` of `c`, and their number.
    fn centroid_sum(&self, c: Point, r: f64) -> (f64, f64, u32) {
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0);
        self.disk(c, r, |row, ca, cb| {
            let y = self.y0 + (row as f64 + 0.5) * self.h;
            self.span(row, ca, cb, |i, m| {
                let mut bits = self.bits[i] & m;
                let word_col = (i % self.words) * 64;
                while bits != 0 {
                    let col = word_col + bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    sx += self.x0 + (col as f64 + 0.5) * self.h;
                    sy += y;
                    n += 1;
                }
            });
        });
        (sx, sy, n)
    }

    /// Fill the inside (even-odd) of closed `rings`.
    pub fn fill(&mut self, rings: &[Vec<Point>]) {
        for row in 0..self.rows {
            let y = self.y0 + (row as f64 + 0.5) * self.h;
            let mut xs: Vec<f64> = Vec::new();
            for ring in rings {
                let n = ring.len();
                for i in 0..n {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    if (a.1 > y) != (b.1 > y) {
                        xs.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for [a, b] in xs.as_chunks::<2>().0 {
                self.set_span(row, *a, *b);
            }
        }
    }

    /// Cells set in either.
    pub fn or(&mut self, other: &Grid) {
        for (a, b) in self.bits.iter_mut().zip(&other.bits) {
            *a |= *b;
        }
    }

    /// Cells set in both.
    pub fn and(&mut self, other: &Grid) {
        for (a, b) in self.bits.iter_mut().zip(&other.bits) {
            *a &= *b;
        }
    }

    /// Cells set in `self` but not in `other`.
    pub fn and_not(&mut self, other: &Grid) {
        for (a, b) in self.bits.iter_mut().zip(&other.bits) {
            *a &= !*b;
        }
    }

    pub fn is_set(&self, p: Point) -> bool {
        let col = ((p.0 - self.x0) / self.h).floor();
        let row = ((p.1 - self.y0) / self.h).floor();
        if col < 0.0 || row < 0.0 || col >= self.cols as f64 || row >= self.rows as f64 {
            return false;
        }
        let (col, row) = (col as usize, row as usize);
        self.bits[row * self.words + col / 64] >> (col % 64) & 1 == 1
    }

    pub fn count(&self) -> u64 {
        self.bits.iter().map(|w| u64::from(w.count_ones())).sum()
    }

    /// The set cell nearest to `p` (ties: lowest row, then column).
    pub fn nearest(&self, p: Point) -> Option<Point> {
        let mut best: Option<(f64, Point)> = None;
        for row in 0..self.rows {
            let y = self.y0 + (row as f64 + 0.5) * self.h;
            if best.is_some_and(|(d, _)| (y - p.1).abs() > d) {
                continue;
            }
            for w in 0..self.words {
                let mut bits = self.bits[row * self.words + w];
                while bits != 0 {
                    let col = w * 64 + bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    let c = self.centre(col, row);
                    let d = distance(c, p);
                    if best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, c));
                    }
                }
            }
        }
        best.map(|(_, c)| c)
    }
}

/// What steering needs to know about the pocket.
pub struct Field<'a> {
    /// Material still to cut by the adaptive passes.
    pub material: Grid,
    /// Cells where the tool centre may be (the pocket shrunk by the tool-centre offset).
    pub allowed: Grid,
    /// Cells well inside `allowed` (shrunk a few cells more): no exact wall check needed.
    pub safe: Grid,
    /// Walls and islands, for the exact distance check.
    pub walls: &'a PrimIndex,
    /// Tool centre to walls, mm.
    pub offset: f64,
    pub tool_radius: f64,
    /// Target sideways cut, mm.
    pub load: f64,
    /// true: material on the right of the direction of travel (climb with M3).
    pub material_right: bool,
}

/// The adaptive passes (open paths, in cut order) and the engagement of every step as a
/// share of the target (for tests and diagnostics).
pub struct Passes {
    pub paths: Vec<Vec<Point>>,
    pub engagement: Vec<f64>,
}

impl Field<'_> {
    fn allowed(&self, p: Point) -> bool {
        self.safe.is_set(p) || (self.allowed.is_set(p) && !self.walls.any_within(p, self.offset))
    }

    /// The radius the model cuts with: cells whose whole square is within the tool's radius
    /// (centre within r − half a cell diagonal). The model never calls material cut that
    /// isn't, so nothing it thinks is done is left behind; counting and clearing use the
    /// same radius, so what it counts it can also clear.
    fn reach(&self) -> f64 {
        self.tool_radius - self.material.cell_side() * std::f64::consts::FRAC_1_SQRT_2
    }

    /// Cells the tool would cut with its centre at `p`.
    fn cut_at(&self, p: Point) -> u32 {
        self.material.count_disk(p, self.reach())
    }

    /// Mark the cells the tool at `p` cuts.
    fn cut(&mut self, p: Point) {
        let reach = self.reach();
        self.material.clear_disk(p, reach);
    }

    fn step(&self) -> f64 {
        2.0 * self.material.cell_side()
    }

    /// Target cells per step: the load times the step, in cells.
    fn target(&self) -> f64 {
        let h = self.material.cell_side();
        self.load * self.step() / (h * h)
    }

    /// The direction from `here` (heading `heading`) whose cut is closest to the target,
    /// keeping the material on its side; None when there is nothing left to follow.
    fn steer(&self, here: Point, heading: f64) -> Option<(Point, f64, f64)> {
        let side = if self.material_right { 1.0 } else { -1.0 };
        let s = self.step();
        let target = self.target();
        // t > 0 turns into the material, t < 0 away from it.
        let probe = |t: f64| -> (Point, f64, f64) {
            let angle = heading - side * t;
            let p = (here.0 + s * angle.cos(), here.1 + s * angle.sin());
            let score = if self.allowed(p) {
                f64::from(self.cut_at(p)) - target
            } else {
                f64::INFINITY // too close to a wall counts as "too much"
            };
            (p, angle, score)
        };
        let into = probe(FRAC_PI_2);
        let away = probe(-FRAC_PI_2);
        if into.2 < 0.0 {
            // Not enough material even turning into it.
            let cut = into.2 + target;
            return (cut > 0.1 * target).then_some((into.0, into.1, cut));
        }
        if away.2 > 0.0 {
            // Too much even turning away: take the least, if allowed at all.
            return away
                .2
                .is_finite()
                .then_some((away.0, away.1, away.2 + target));
        }
        let (mut lo, mut hi) = (-FRAC_PI_2, FRAC_PI_2);
        for _ in 0..STEER_ROUNDS {
            let mid = (lo + hi) / 2.0;
            if probe(mid).2 > 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let (p, angle, score) = probe(lo);
        Some((p, angle, score + target))
    }

    /// Where to look for material while seeking: within this of the tool centre.
    fn lookahead(&self) -> f64 {
        5.0 * self.tool_radius
    }

    /// A step from `here` towards the uncut material nearby, keeping inside the allowed area
    /// (turning up to 90° either way, nearest first); None if there's none or no way there.
    fn seek(&self, here: Point) -> Option<(Point, f64)> {
        let (sx, sy, n) = self.material.centroid_sum(here, self.lookahead());
        if n == 0 {
            return None;
        }
        let target = (sx / f64::from(n), sy / f64::from(n));
        let towards = (target.1 - here.1).atan2(target.0 - here.0);
        let s = self.step();
        for k in 0..=12 {
            for sign in [1.0, -1.0] {
                let angle = towards + sign * f64::from(k) * FRAC_PI_2 / 12.0;
                let p = (here.0 + s * angle.cos(), here.1 + s * angle.sin());
                if self.allowed(p) {
                    return Some((p, angle));
                }
                if k == 0 {
                    break;
                }
            }
        }
        None
    }

    /// Follow the material from `start`, heading `heading`, cutting as it goes. Where the
    /// cut drops (the material near the path is gone), seek the nearest material through
    /// what is cleared and follow again; stop when nothing is within reach.
    fn follow(
        &mut self,
        start: Point,
        mut heading: f64,
        path: &mut Vec<Point>,
        engagement: &mut Vec<f64>,
        budget: &mut usize,
    ) {
        let target = self.target();
        let mut here = start;
        let mut seeking = 0usize;
        let seek_limit = (4.0 * self.lookahead() / self.step()).ceil() as usize;
        while *budget > 0 {
            *budget -= 1;
            let followed = self
                .steer(here, heading)
                .filter(|&(_, _, cut)| cut >= SEEK_BELOW * target);
            let (p, angle) = match followed {
                Some((p, angle, cut)) => {
                    engagement.push(cut / target);
                    seeking = 0;
                    (p, angle)
                }
                None => {
                    seeking += 1;
                    if seeking > seek_limit {
                        break;
                    }
                    match self.seek(here) {
                        Some(step) => step,
                        None => break,
                    }
                }
            };
            self.cut(p);
            path.push(p);
            here = p;
            heading = angle;
        }
    }
}

/// Clear the field's material in passes, starting at `start` (where the tool plunges).
pub fn clear(field: &mut Field, start: Point) -> Passes {
    let mut paths: Vec<Vec<Point>> = Vec::new();
    let mut engagement = Vec::new();
    let mut budget = MAX_STEPS;
    let side = if field.material_right { 1.0 } else { -1.0 };

    // Open a start: a small circle round the plunge point, travelling so the material is
    // on the chosen side (counter-clockwise for material on the right).
    let mut path = vec![start];
    field.cut(start);
    let opening = field.load;
    let count = 24;
    for k in 0..=count {
        let a = side * std::f64::consts::TAU * k as f64 / count as f64;
        let p = (start.0 + opening * a.cos(), start.1 + opening * a.sin());
        if !field.allowed(p) {
            break;
        }
        field.cut(p);
        path.push(p);
    }
    let here = path.last().copied().unwrap_or(start);
    let heading = (here.1 - start.1).atan2(here.0 - start.0) + side * FRAC_PI_2;
    field.follow(here, heading, &mut path, &mut engagement, &mut budget);
    paths.push(path);

    // Then from the nearest uncut cell, entered from the cleared side, until nothing is left.
    while budget > 0 {
        let here = paths
            .last()
            .and_then(|p| p.last())
            .copied()
            .unwrap_or(start);
        let Some(cell) = field.material.nearest(here) else {
            break;
        };
        let Some(entry) = entry_point(field, cell, here) else {
            // No allowed place to reach it from: leave it to the wall pass and the uncut check.
            field.material.clear_disk(cell, field.material.cell_side());
            continue;
        };
        field.cut(entry);
        let heading = (cell.1 - entry.1).atan2(cell.0 - entry.0) + side * FRAC_PI_2;
        let mut path = vec![entry];
        field.follow(entry, heading, &mut path, &mut engagement, &mut budget);
        paths.push(path);
    }
    Passes { paths, engagement }
}

/// A tool centre position that just reaches `cell`, from the side towards `from` if allowed,
/// otherwise from the allowed direction cutting the least.
fn entry_point(field: &Field, cell: Point, from: Point) -> Option<Point> {
    let reach = field.reach() - field.material.cell_side();
    let towards = (from.1 - cell.1).atan2(from.0 - cell.0);
    let candidates = (0..16).map(|k| {
        let a = towards + std::f64::consts::TAU * k as f64 / 16.0;
        (cell.0 + reach * a.cos(), cell.1 + reach * a.sin())
    });
    let mut best: Option<(u32, Point)> = None;
    for p in candidates {
        if !field.allowed(p) {
            continue;
        }
        let cut = field.cut_at(p);
        if best.is_none_or(|(c, _)| cut < c) {
            best = Some((cut, p));
        }
    }
    best.map(|(_, p)| p)
}

/// Drop points while the path stays within `tolerance` of the original (open polyline).
pub fn simplify(path: &[Point], tolerance: f64) -> Vec<Point> {
    if path.len() < 3 {
        return path.to_vec();
    }
    let mut keep = vec![false; path.len()];
    keep[0] = true;
    keep[path.len() - 1] = true;
    let mut stack = vec![(0, path.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let mut worst = (0.0, 0);
        for i in a + 1..b {
            let d = segment_distance_to(path[i], path[a], path[b]);
            if d > worst.0 {
                worst = (d, i);
            }
        }
        if worst.0 > tolerance {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    path.iter()
        .zip(keep)
        .filter_map(|(&p, k)| k.then_some(p))
        .collect()
}

fn segment_distance_to(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    distance(p, (a.0 + dx * t, a.1 + dy * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(lo: f64, hi: f64) -> Vec<Point> {
        vec![(lo, lo), (hi, lo), (hi, hi), (lo, hi)]
    }

    #[test]
    fn the_grid_counts_and_clears_disks_by_rows() {
        let mut grid = Grid::new((0.0, 0.0), (10.0, 10.0), 0.1);
        grid.fill(&[square(0.0, 10.0)]);
        assert_eq!(grid.count(), 100 * 100);
        // A disk of radius 1 holds about π cells of 0.01 mm² ... per mm²: π / 0.01.
        let disk = f64::from(grid.count_disk((5.0, 5.0), 1.0));
        assert!((disk - std::f64::consts::PI / 0.01).abs() < 20.0, "{disk}");
        grid.clear_disk((5.0, 5.0), 1.0);
        assert_eq!(grid.count_disk((5.0, 5.0), 1.0), 0);
        assert_eq!(grid.count(), 10_000 - disk as u64);
        // Even-odd: a hole stays empty.
        let mut ring = Grid::new((0.0, 0.0), (10.0, 10.0), 0.1);
        ring.fill(&[square(0.0, 10.0), square(4.0, 6.0)]);
        assert_eq!(ring.count(), 10_000 - 400);
        assert!(!ring.is_set((5.0, 5.0)) && ring.is_set((1.0, 1.0)));
    }

    #[test]
    fn nearest_finds_the_closest_set_cell() {
        let mut grid = Grid::new((0.0, 0.0), (10.0, 10.0), 0.5);
        grid.fill(&[square(6.0, 8.0)]);
        let n = grid.nearest((0.0, 0.0)).unwrap();
        assert!(
            (n.0 - 6.25).abs() < 1e-9 && (n.1 - 6.25).abs() < 1e-9,
            "{n:?}"
        );
        grid.clear_disk((7.0, 7.0), 5.0);
        assert!(grid.nearest((0.0, 0.0)).is_none());
    }

    #[test]
    fn simplify_keeps_corners_and_drops_straight_points() {
        let path: Vec<Point> = (0..=10)
            .map(|i| (f64::from(i), 0.0))
            .chain((1..=10).map(|i| (10.0, f64::from(i))))
            .collect();
        assert_eq!(
            simplify(&path, 0.01),
            [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        );
    }
}
