// SPDX-License-Identifier: GPL-3.0-or-later
//! The placer (docs/specs/nesting-search.md §3, nesting.md §4): instances placed one by
//! one from the usable area's lower-left corner, each where the nest's goal wants it and
//! its footprint grown by the gap meets nothing placed; the layout scored by the goal (§2).
//!
//! The bottom-left rule is from Jakobs, *On genetic algorithms for the packing of polygons*,
//! EJOR 88 (1996), and Chazelle, *The bottom-left bin-packing heuristic: an efficient
//! implementation*, IEEE Transactions on Computers C-32 (1983).
//!
//! The board is kept twice as runs of occupied cells: along each column, and along each
//! row. A footprint is tried at one column (or row) by sliding it up (or right): wherever
//! one of its runs meets one of the board's, it jumps past that run, so a line full of
//! parts is crossed in a few steps instead of cell by cell.

use chipload_core::Point;
use chipload_core::tolerances::PLACEMENT_ROUNDING_MM;

use super::footprint::Footprint;
use super::grid::Grid;
use super::sweep::Sweep;
use crate::{Cancel, ToolpathError};

/// What a nest makes as small as it can (nesting-search.md §2), after placing as much as
/// it can.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Goal {
    /// The smallest box round the parts: a block in the lower-left corner.
    #[default]
    Compact,
    /// The lowest top edge: a strip across the top left free.
    Low,
    /// The leftmost right edge: a strip down the right left free.
    Left,
}

/// Runs of occupied cells along one line, (first, last) inclusive, sorted, apart.
type Runs = Vec<(i64, i64)>;

/// A footprint as runs, one list per line: along each column (cells are rows) or along
/// each row (cells are columns).
#[derive(Debug, Clone)]
struct View {
    /// The footprint grown by the gap, its cell (0, 0) the plain one's (−margin, −margin).
    grown: Vec<Runs>,
    /// The plain footprint: what it occupies once placed.
    plain: Vec<Runs>,
}

/// One way an item can lie: its turn and its footprint there, as runs both ways.
#[derive(Debug, Clone)]
pub struct Lie {
    /// Radians, counter-clockwise: what the caller turned the sweep by.
    pub angle: f64,
    pub footprint: Footprint,
    margin: i64,
    by_col: View,
    by_row: View,
    /// Its size in cells, columns and rows.
    cells: (i64, i64),
    /// How many cells it occupies.
    area: i64,
}

/// Something to place: each way it may lie (one for no turns, two for half, …).
#[derive(Debug, Clone)]
pub struct Item {
    pub lies: Vec<Lie>,
}

/// Where an item went: which of its lies, and where its footprint's low corner (`low` of
/// the footprint) is, mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Put {
    pub item: usize,
    pub lie: usize,
    pub low: Point,
}

/// A layout: what was placed, what didn't fit (indices into the items), and the size of
/// the box from the usable area's lower-left corner round every footprint placed, mm.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub placed: Vec<Put>,
    pub unplaced: Vec<usize>,
    pub extent: (f64, f64),
}

/// The usable area of a board (the board less its margin, within the machine's travel):
/// its low and high corners, mm, the cell side, and what may not be covered.
#[derive(Debug, Clone)]
pub struct Sheet {
    pub low: Point,
    pub high: Point,
    pub cell: f64,
    /// Keep-out areas (dowel holes), as runs per column and per row.
    keep_cols: Vec<Runs>,
    keep_rows: Vec<Runs>,
}

impl Sheet {
    /// The usable area from `low` to `high` on cells of side `cell`, with `keep_out`
    /// sweeps (dowel holes) nothing may be placed on. None when the area is empty or the
    /// cell isn't a positive size.
    pub fn new(low: Point, high: Point, cell: f64, keep_out: &[Sweep]) -> Option<Sheet> {
        let (width, height) = (high.0 - low.0, high.1 - low.1);
        let sized = |v: f64| v.is_finite() && v > 0.0;
        if !(sized(width) && sized(height) && sized(cell)) {
            return None;
        }
        let (cols, rows) = ((width / cell).ceil(), (height / cell).ceil());
        let mut grid = Grid::new(cols as usize, rows as usize);
        let frame = super::footprint::Frame { origin: low, cell };
        for sweep in keep_out {
            frame.add_sweep(&mut grid, sweep);
        }
        Some(Sheet {
            low,
            high,
            cell,
            keep_cols: runs(&grid, true),
            keep_rows: runs(&grid, false),
        })
    }

    /// The way `footprint` (turned by `angle`) lies on this sheet, with the gap.
    pub fn lie(&self, angle: f64, footprint: Footprint, gap: f64) -> Lie {
        let grown = footprint.dilated(gap);
        let view = |by_col: bool| View {
            grown: runs(&grown.grid, by_col),
            plain: runs(&footprint.grid, by_col),
        };
        Lie {
            angle,
            margin: grown.margin as i64,
            by_col: view(true),
            by_row: view(false),
            cells: (footprint.grid.cols() as i64, footprint.grid.rows() as i64),
            area: footprint.grid.count() as i64,
            footprint,
        }
    }

    /// The low corner of a footprint whose cell (0, 0) is on cell (`col`, `row`).
    fn corner(&self, col: i64, row: i64) -> Point {
        (
            self.low.0 + col as f64 * self.cell,
            self.low.1 + row as f64 * self.cell,
        )
    }

    /// The last cell a footprint `length` mm long may start at on a side `side` mm long,
    /// its far end within the side, compared in mm (nesting.md §4.5); None when it is
    /// longer than the side.
    fn last_start(&self, length: f64, side: f64) -> Option<i64> {
        let room = side - length + PLACEMENT_ROUNDING_MM;
        (room >= 0.0).then(|| (room / self.cell).floor() as i64)
    }

    /// The last column and row `lie` may start at; None when it is larger than the sheet.
    fn limits(&self, lie: &Lie) -> Option<(i64, i64)> {
        let (w, h) = lie.footprint.size();
        Some((
            self.last_start(w, self.high.0 - self.low.0)?,
            self.last_start(h, self.high.1 - self.low.1)?,
        ))
    }
}

/// A place's key, a layout's score: compared in order, the smaller wins.
pub(super) type Key = [i64; 5];

/// The placer's board while it fills: the occupied runs per column and per row, and the
/// box round what is placed, in cells from the corner.
struct Filling<'a> {
    sheet: &'a Sheet,
    cols: Vec<Runs>,
    rows: Vec<Runs>,
    right: i64,
    top: i64,
}

impl<'a> Filling<'a> {
    fn new(sheet: &'a Sheet) -> Filling<'a> {
        Filling {
            sheet,
            cols: sheet.keep_cols.clone(),
            rows: sheet.keep_rows.clone(),
            right: 0,
            top: 0,
        }
    }

    /// The first place for `lie`, starting columns up to `last_col` and rows up to
    /// `last_row`: by columns (the leftmost column where it fits, there the lowest row)
    /// or by rows (the lowest row, there the leftmost column). None when it fits nowhere.
    fn first(&self, lie: &Lie, by_col: bool, last_col: i64, last_row: i64) -> Option<(i64, i64)> {
        if by_col {
            (0..=last_col).find_map(|col| {
                self.slide(&lie.by_col, &self.cols, lie.margin, col, last_row)
                    .map(|row| (col, row))
            })
        } else {
            (0..=last_row).find_map(|row| {
                self.slide(&lie.by_row, &self.rows, lie.margin, row, last_col)
                    .map(|col| (col, row))
            })
        }
    }

    /// The lowest cell across, up to `last`, where `view` fits starting at line `along`
    /// of `board`'s lines.
    fn slide(
        &self,
        view: &View,
        board: &[Runs],
        margin: i64,
        along: i64,
        last: i64,
    ) -> Option<i64> {
        let mut across = 0;
        'slide: loop {
            if across > last {
                return None;
            }
            for (i, runs) in view.grown.iter().enumerate() {
                let line = along + i as i64 - margin;
                let Some(line) = usize::try_from(line).ok().and_then(|l| board.get(l)) else {
                    continue;
                };
                for &(first, end) in runs {
                    let (low, high) = (across + first - margin, across + end - margin);
                    if let Some(blocked) = meeting(line, low, high) {
                        // Past the run it met: its first cell just beyond.
                        across = blocked + 1 - (first - margin);
                        continue 'slide;
                    }
                }
            }
            return Some(across);
        }
    }

    /// The places tried for `lie` (nesting-search.md §3): the lowest, the leftmost, and for
    /// compact also the lowest within the box's width and the leftmost within its height.
    fn candidates(&self, lie: &Lie, goal: Goal) -> Vec<(i64, i64)> {
        let Some((last_col, last_row)) = self.sheet.limits(lie) else {
            return Vec::new();
        };
        let (w, h) = lie.cells;
        let tries: &[(bool, bool)] = match goal {
            Goal::Low => &[(false, false)],
            Goal::Left => &[(true, false)],
            Goal::Compact => &[(false, true), (true, true), (false, false), (true, false)],
        };
        let mut out: Vec<(i64, i64)> = Vec::new();
        for &(by_col, within) in tries {
            let (cols, rows) = match (within, by_col) {
                (false, _) => (last_col, last_row),
                (true, false) => (last_col.min(self.right - w), last_row),
                (true, true) => (last_col, last_row.min(self.top - h)),
            };
            if let Some(place) = self.first(lie, by_col, cols, rows)
                && !out.contains(&place)
            {
                out.push(place);
            }
        }
        out
    }

    /// What placing `lie` at (`col`, `row`) scores for `goal`: the smaller the better.
    fn key(&self, lie: &Lie, goal: Goal, (col, row): (i64, i64)) -> Key {
        let right = self.right.max(col + lie.cells.0);
        let top = self.top.max(row + lie.cells.1);
        let area = right * top;
        match goal {
            Goal::Compact => [area, top, right, row, col],
            Goal::Low => [row, col, top, area, right],
            Goal::Left => [col, row, right, area, top],
        }
    }

    /// The best place for `lie`, and its key.
    fn best(&self, lie: &Lie, goal: Goal) -> Option<(Key, (i64, i64))> {
        let mut best: Option<(Key, (i64, i64))> = None;
        for place in self.candidates(lie, goal) {
            let key = self.key(lie, goal, place);
            if best.is_none_or(|(b, _)| key < b) {
                best = Some((key, place));
            }
        }
        best
    }

    /// `lie` placed with its cell (0, 0) at (`col`, `row`): its plain runs occupied.
    fn occupy(&mut self, lie: &Lie, col: i64, row: i64) {
        mark(&mut self.cols, &lie.by_col.plain, col, row);
        mark(&mut self.rows, &lie.by_row.plain, row, col);
        self.right = self.right.max(col + lie.cells.0);
        self.top = self.top.max(row + lie.cells.1);
    }
}

/// `plain`'s runs, its line 0 on `board`'s line `along`, shifted `across`, occupied.
fn mark(board: &mut [Runs], plain: &[Runs], along: i64, across: i64) {
    for (i, runs) in plain.iter().enumerate() {
        let line = along + i as i64;
        let Some(line) = usize::try_from(line).ok().and_then(|l| board.get_mut(l)) else {
            continue;
        };
        for &(first, end) in runs {
            insert(line, across + first, across + end);
        }
    }
}

/// A candidate for the search (nesting-search.md §4): the order the items are placed in,
/// and each item's lie, or None to let the placer pick the best as it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    pub order: Vec<usize>,
    pub turns: Vec<Option<usize>>,
}

/// A candidate placed: its layout and its score.
#[derive(Debug, Clone)]
pub(super) struct Decoded {
    pub layout: Layout,
    pub score: Key,
}

/// Place `candidate`'s items one by one (nesting-search.md §3): each in its given lie at
/// the best place for `goal`, else in the first other lie that fits; with no lie given, in
/// the lie whose place is best.
pub(super) fn decode(
    sheet: &Sheet,
    items: &[Item],
    candidate: &Candidate,
    goal: Goal,
    cancel: &Cancel,
) -> Result<Decoded, ToolpathError> {
    let mut filling = Filling::new(sheet);
    let (mut placed, mut unplaced) = (Vec::new(), Vec::new());
    let mut area = 0_i64;
    for &k in &candidate.order {
        if cancel.is_cancelled() {
            return Err(ToolpathError::Cancelled);
        }
        let Some(item) = items.get(k) else { continue };
        let given = candidate.turns.get(k).copied().flatten();
        let Some((l, (col, row))) = choose(&filling, item, given, goal) else {
            unplaced.push(k);
            continue;
        };
        let Some(lie) = item.lies.get(l) else {
            continue;
        };
        filling.occupy(lie, col, row);
        area += lie.area;
        placed.push(Put {
            item: k,
            lie: l,
            low: sheet.corner(col, row),
        });
    }
    unplaced.sort_unstable();
    let (right, top) = (filling.right, filling.top);
    let measures = match goal {
        Goal::Compact => [right * top, top, right],
        Goal::Low => [top, right * top, right],
        Goal::Left => [right, right * top, top],
    };
    let score = [
        -(placed.len() as i64),
        -area,
        measures[0],
        measures[1],
        measures[2],
    ];
    let extent = extent(sheet, items, &placed);
    Ok(Decoded {
        layout: Layout {
            placed,
            unplaced,
            extent,
        },
        score,
    })
}

/// The lie and place for `item`: `given`'s best place, else the first other lie's; with
/// none given, the lie with the best place, the first among equals.
fn choose(
    filling: &Filling,
    item: &Item,
    given: Option<usize>,
    goal: Goal,
) -> Option<(usize, (i64, i64))> {
    if let Some(g) = given {
        let first = item.lies.get(g).and_then(|lie| filling.best(lie, goal));
        if let Some((_, place)) = first {
            return Some((g, place));
        }
        return item.lies.iter().enumerate().find_map(|(l, lie)| {
            (l != g)
                .then(|| filling.best(lie, goal))
                .flatten()
                .map(|(_, place)| (l, place))
        });
    }
    let mut best: Option<(Key, usize, (i64, i64))> = None;
    for (l, lie) in item.lies.iter().enumerate() {
        if let Some((key, place)) = filling.best(lie, goal)
            && best.is_none_or(|(b, _, _)| key < b)
        {
            best = Some((key, l, place));
        }
    }
    best.map(|(_, l, place)| (l, place))
}

/// The box round the placed footprints from the sheet's low corner, mm.
fn extent(sheet: &Sheet, items: &[Item], placed: &[Put]) -> (f64, f64) {
    let mut out = (0.0_f64, 0.0_f64);
    for put in placed {
        let Some(lie) = items.get(put.item).and_then(|i| i.lies.get(put.lie)) else {
            continue;
        };
        let (w, h) = lie.footprint.size();
        out.0 = out.0.max(put.low.0 - sheet.low.0 + w);
        out.1 = out.1.max(put.low.1 - sheet.low.1 + h);
    }
    out
}

/// The search's starts (nesting-search.md §4.1): each of the fixed orders with the placer
/// picking the lies, and again with its first item in each of its lies.
pub(super) fn starts(items: &[Item]) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    for order in orders(items) {
        let free = Candidate {
            order: order.clone(),
            turns: vec![None; items.len()],
        };
        out.push(free.clone());
        let Some(&first) = order.first() else {
            continue;
        };
        let lies = items.get(first).map_or(0, |i| i.lies.len());
        if lies < 2 {
            continue;
        }
        for l in 0..lies {
            let mut turned = free.clone();
            if let Some(t) = turned.turns.get_mut(first) {
                *t = Some(l);
            }
            out.push(turned);
        }
    }
    out
}

/// Fill `sheet` with `items` (nesting-search.md): every start placed, the best kept by its
/// score, the first among equals. Deterministic: the same items give the same layout.
pub fn fill(
    sheet: &Sheet,
    items: &[Item],
    goal: Goal,
    cancel: &Cancel,
) -> Result<Layout, ToolpathError> {
    let mut best: Option<Decoded> = None;
    for candidate in starts(items) {
        let decoded = decode(sheet, items, &candidate, goal, cancel)?;
        if best.as_ref().is_none_or(|b| decoded.score < b.score) {
            best = Some(decoded);
        }
    }
    Ok(best.map(|b| b.layout).unwrap_or(Layout {
        placed: Vec::new(),
        unplaced: Vec::new(),
        extent: (0.0, 0.0),
    }))
}

/// The orders tried: largest area first, then longest side, then widest in X, then
/// tallest in Y; ties in item order. Repeated orders are tried once.
fn orders(items: &[Item]) -> Vec<Vec<usize>> {
    let size = |k: usize| {
        items
            .get(k)
            .and_then(|item| item.lies.first())
            .map_or((0.0, 0.0), |lie| lie.footprint.size())
    };
    let keys: [&dyn Fn((f64, f64)) -> f64; 4] = [
        &|(w, h)| w * h,
        &|(w, h)| w.max(h),
        &|(w, _)| w,
        &|(_, h)| h,
    ];
    let mut orders: Vec<Vec<usize>> = Vec::new();
    for key in keys {
        let mut order: Vec<usize> = (0..items.len()).collect();
        // Largest first; a stable sort keeps item order among equals.
        order.sort_by(|&a, &b| key(size(b)).total_cmp(&key(size(a))));
        if !orders.contains(&order) {
            orders.push(order);
        }
    }
    orders
}

/// The runs of occupied cells of `grid`, one list per line across, lines taken along X
/// (`along_x`, a line per column) or along Y (a line per row).
fn runs(grid: &Grid, along_x: bool) -> Vec<Runs> {
    let (lines, cells) = if along_x {
        (grid.cols(), grid.rows())
    } else {
        (grid.rows(), grid.cols())
    };
    (0..lines)
        .map(|line| {
            let mut out: Runs = Vec::new();
            let mut start: Option<i64> = None;
            for cell in 0..=cells {
                let on = cell < cells
                    && if along_x {
                        grid.get(line, cell)
                    } else {
                        grid.get(cell, line)
                    };
                match (on, start) {
                    (true, None) => start = Some(cell as i64),
                    (false, Some(s)) => {
                        out.push((s, cell as i64 - 1));
                        start = None;
                    }
                    _ => {}
                }
            }
            out
        })
        .collect()
}

/// The last cell of the first run of `line` that meets `low..=high`, if any.
fn meeting(line: &Runs, low: i64, high: i64) -> Option<i64> {
    // The first run that ends at or after `low`.
    let k = line.partition_point(|&(_, end)| end < low);
    line.get(k)
        .filter(|&&(first, _)| first <= high)
        .map(|&(_, end)| end)
}

/// `first..=last` made occupied in `line`, merged with runs it meets or touches.
fn insert(line: &mut Runs, first: i64, last: i64) {
    let start = line.partition_point(|&(_, end)| end + 1 < first);
    let stop = line.partition_point(|&(begin, _)| begin <= last + 1);
    let (mut lo, mut hi) = (first, last);
    if let Some(merged) = line.get(start..stop) {
        for &(a, b) in merged {
            lo = lo.min(a);
            hi = hi.max(b);
        }
    }
    if start <= stop {
        line.splice(start..stop, [(lo, hi)]);
    }
}
