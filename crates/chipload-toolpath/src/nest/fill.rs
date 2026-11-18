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
    /// The plain footprint's longest run, and its line: a board line with no free gap as
    /// long, that far along, can't take the footprint there.
    widest: (usize, i64),
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
        let view = |by_col: bool| {
            let plain = runs(&footprint.grid, by_col);
            View {
                grown: runs(&grown.grid, by_col),
                widest: widest_run(&plain),
                plain,
            }
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
        // A line's free gap within the cells the footprint may use, compared with its
        // longest run on that line: too short, and it can't lie there.
        let room = |board: &[Runs], view: &View, along: i64, end: i64| {
            let (line, length) = view.widest;
            usize::try_from(along)
                .ok()
                .and_then(|a| board.get(a + line))
                .is_none_or(|runs| longest_gap(runs, end) >= length)
        };
        let (w, h) = lie.cells;
        if by_col {
            (0..=last_col).find_map(|col| {
                if !room(&self.cols, &lie.by_col, col, last_row + h) {
                    return None;
                }
                self.slide(&lie.by_col, &self.cols, lie.margin, col, last_row)
                    .map(|row| (col, row))
            })
        } else {
            (0..=last_row).find_map(|row| {
                if !room(&self.rows, &lie.by_row, row, last_col + w) {
                    return None;
                }
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
        let lines = view.grown.len();
        let mut across = 0;
        // The lines are checked round and round from where the last jump came from: a
        // jump only moves the footprint on, so it fits once every line has passed in a
        // row since the last jump, without starting again from the first line.
        let (mut i, mut clear) = (0, 0);
        while clear < lines {
            if across > last {
                return None;
            }
            let row = along + i as i64 - margin;
            let board_line = usize::try_from(row).ok().and_then(|l| board.get(l));
            let runs = view.grown.get(i).map_or(&[][..], Vec::as_slice);
            let mut jumped = false;
            if let Some(line) = board_line {
                for &(first, end) in runs {
                    let (low, high) = (across + first - margin, across + end - margin);
                    if let Some(blocked) = meeting(line, low, high) {
                        // Past the run it met: its first cell just beyond.
                        across = blocked + 1 - (first - margin);
                        jumped = true;
                        break;
                    }
                }
            }
            if jumped {
                // This line is checked again where the footprint is now.
                clear = 0;
                continue;
            }
            clear += 1;
            i = (i + 1) % lines;
        }
        (across <= last).then_some(across)
    }

    /// The places tried for `lie` (nesting-search.md §3, as built): for low the lowest,
    /// for left the leftmost. For compact one place growing the box upward and one growing
    /// it rightward: the lowest within the box's width, else (the part wider than the box,
    /// or no room in it) the leftmost anywhere; and the leftmost within the box's height,
    /// else the lowest anywhere. A place inside the box is the lowest within its width.
    fn candidates(&self, lie: &Lie, goal: Goal) -> Vec<(i64, i64)> {
        let Some((last_col, last_row)) = self.sheet.limits(lie) else {
            return Vec::new();
        };
        let (w, h) = lie.cells;
        let lowest = |cols: i64| self.first(lie, false, cols, last_row);
        let leftmost = |rows: i64| self.first(lie, true, last_col, rows);
        let found = match goal {
            Goal::Low => vec![lowest(last_col)],
            Goal::Left => vec![leftmost(last_row)],
            Goal::Compact => {
                let up = lowest(last_col.min(self.right - w)).or_else(|| leftmost(last_row));
                let right = leftmost(last_row.min(self.top - h)).or_else(|| lowest(last_col));
                vec![up, right]
            }
        };
        let mut out: Vec<(i64, i64)> = Vec::new();
        for place in found.into_iter().flatten() {
            if !out.contains(&place) {
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

    /// Whether `lie` fits with its cell (0, 0) at (`col`, `row`): within the sheet, its
    /// footprint grown by the gap meeting nothing placed.
    fn fits_at(&self, lie: &Lie, col: i64, row: i64) -> bool {
        let Some((last_col, last_row)) = self.sheet.limits(lie) else {
            return false;
        };
        if !(0..=last_col).contains(&col) || !(0..=last_row).contains(&row) {
            return false;
        }
        lie.by_col.grown.iter().enumerate().all(|(i, runs)| {
            let line = usize::try_from(col + i as i64 - lie.margin).ok();
            let Some(line) = line.and_then(|l| self.cols.get(l)) else {
                return true;
            };
            runs.iter().all(|&(first, end)| {
                meeting(line, row + first - lie.margin, row + end - lie.margin).is_none()
            })
        })
    }

    /// `lie` placed with its cell (0, 0) at (`col`, `row`): its plain runs occupied.
    fn occupy(&mut self, lie: &Lie, col: i64, row: i64) {
        mark(&mut self.cols, &lie.by_col.plain, (col, row));
        mark(&mut self.rows, &lie.by_row.plain, (row, col));
        self.right = self.right.max(col + lie.cells.0);
        self.top = self.top.max(row + lie.cells.1);
    }
}

/// `plain`'s runs, its line 0 on `board`'s line `along`, shifted `across`, occupied.
fn mark(board: &mut [Runs], plain: &[Runs], (along, across): (i64, i64)) {
    for (i, runs) in plain.iter().enumerate() {
        let line = usize::try_from(along + i as i64).ok();
        let Some(line) = line.and_then(|l| board.get_mut(l)) else {
            continue;
        };
        for &(first, end) in runs {
            insert(line, across + first, across + end);
        }
    }
}

/// The longest stretch of free cells in `line` before cell `cells`.
fn longest_gap(line: &Runs, cells: i64) -> i64 {
    let (mut longest, mut next) = (0, 0);
    for &(first, end) in line {
        if first >= cells {
            break;
        }
        longest = longest.max(first - next);
        next = end + 1;
    }
    longest.max(cells - next)
}

/// The longest run of `lines`, and its line: (0, 0) when there is none.
fn widest_run(lines: &[Runs]) -> (usize, i64) {
    let mut widest = (0, 0);
    for (i, line) in lines.iter().enumerate() {
        for &(first, end) in line {
            if end - first + 1 > widest.1 {
                widest = (i, end - first + 1);
            }
        }
    }
    widest
}

/// A candidate for the search (nesting-search.md §4): the order the items are placed in,
/// and each item's lie, or None to let the placer pick the best as it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    pub order: Vec<usize>,
    pub turns: Vec<Option<usize>>,
}

/// A candidate placed: its layout, the lie each item took (its given one where it
/// wasn't placed), and its score.
#[derive(Debug, Clone)]
pub(super) struct Decoded {
    pub layout: Layout,
    pub turns: Vec<Option<usize>>,
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
    let mut turns = candidate.turns.clone();
    turns.resize(items.len(), None);
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
        if let Some(t) = turns.get_mut(k) {
            *t = Some(l);
        }
        placed.push(Put {
            item: k,
            lie: l,
            low: sheet.corner(col, row),
        });
    }
    unplaced.sort_unstable();
    let score = score(goal, placed.len(), area, (filling.right, filling.top));
    let extent = extent(sheet, items, &placed);
    Ok(Decoded {
        layout: Layout {
            placed,
            unplaced,
            extent,
        },
        turns,
        score,
    })
}

/// A layout's score (nesting-search.md §2): the most placed, the most area placed, then
/// the goal's measures of the box (`right`, `top`, in cells); the smaller the better.
fn score(goal: Goal, placed: usize, area: i64, (right, top): (i64, i64)) -> Key {
    let measures = match goal {
        Goal::Compact => [right * top, top, right],
        Goal::Low => [top, right * top, right],
        Goal::Left => [right, right * top, top],
    };
    [
        -(placed as i64),
        -area,
        measures[0],
        measures[1],
        measures[2],
    ]
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

/// Where an item lies now (nesting-search.md §5): one of its lies, and its footprint's low
/// corner, mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Now {
    pub lie: usize,
    pub low: Point,
}

/// A nest: the layout, and whether it is better than the board as it was. When it isn't,
/// the layout is the board as it was, with what doesn't fit there unplaced.
#[derive(Debug, Clone, PartialEq)]
pub struct Filled {
    pub layout: Layout,
    pub better: bool,
}

/// Fill `sheet` with `items` (nesting-search.md): the starts placed, then the search
/// (§4) from the best of them; the best layout seen. With `now` (where each item lies,
/// §5), the board as it is is scored too and is one of the starts, the search's random
/// numbers seeded from it; the search's layout is taken only when it scores better.
/// Deterministic: the same items, goal and board give the same layout.
pub fn fill(
    sheet: &Sheet,
    items: &[Item],
    goal: Goal,
    now: Option<&[Option<Now>]>,
    cancel: &Cancel,
) -> Result<Filled, ToolpathError> {
    let empty = Layout {
        placed: Vec::new(),
        unplaced: Vec::new(),
        extent: (0.0, 0.0),
    };
    let Some(now) = now else {
        let best = super::search::search(sheet, items, goal, budget(items), None, 0, cancel)?;
        return Ok(Filled {
            layout: best.map_or(empty, |b| b.layout),
            better: true,
        });
    };
    let (as_it_is, candidate) = as_it_is(sheet, items, goal, now);
    let seed = seed_of(now);
    let best = super::search::search(
        sheet,
        items,
        goal,
        budget(items),
        Some(candidate),
        seed,
        cancel,
    )?;
    Ok(match best {
        Some(b) if b.score < as_it_is.score => Filled {
            layout: b.layout,
            better: true,
        },
        _ => Filled {
            layout: as_it_is.layout,
            better: false,
        },
    })
}

/// The board as it is (nesting-search.md §5), scored: each item that lies in one of its
/// lies, on the sheet, its footprint grown by the gap meeting none placed before it, is
/// placed where it is (taken lowest first, or leftmost first for left); the rest are
/// unplaced. And the candidate that gives about the same layout: those placed in that
/// order, in their lies, then the rest.
pub(super) fn as_it_is(
    sheet: &Sheet,
    items: &[Item],
    goal: Goal,
    now: &[Option<Now>],
) -> (Decoded, Candidate) {
    let cell_of = |mm: f64, from: f64| ((mm - from) / sheet.cell).round() as i64;
    let mut lying: Vec<(usize, usize, i64, i64, Point)> = Vec::new();
    for (k, n) in now.iter().enumerate() {
        let Some(n) = n else { continue };
        if items.get(k).and_then(|i| i.lies.get(n.lie)).is_none() {
            continue;
        }
        let (col, row) = (cell_of(n.low.0, sheet.low.0), cell_of(n.low.1, sheet.low.1));
        lying.push((k, n.lie, col, row, n.low));
    }
    lying.sort_by_key(|&(k, _, col, row, _)| match goal {
        Goal::Left => (col, row, k),
        Goal::Compact | Goal::Low => (row, col, k),
    });
    let mut filling = Filling::new(sheet);
    let mut turns: Vec<Option<usize>> = vec![None; items.len()];
    let (mut placed, mut order) = (Vec::new(), Vec::new());
    let mut area = 0_i64;
    for &(k, l, col, row, low) in &lying {
        let Some(lie) = items.get(k).and_then(|i| i.lies.get(l)) else {
            continue;
        };
        if let Some(t) = turns.get_mut(k) {
            *t = Some(l);
        }
        if !filling.fits_at(lie, col, row) {
            continue;
        }
        filling.occupy(lie, col, row);
        area += lie.area;
        order.push(k);
        placed.push(Put {
            item: k,
            lie: l,
            low,
        });
    }
    let unplaced: Vec<usize> = (0..items.len()).filter(|k| !order.contains(k)).collect();
    order.extend(&unplaced);
    let extent = extent(sheet, items, &placed);
    let score = score(goal, placed.len(), area, (filling.right, filling.top));
    let decoded = Decoded {
        layout: Layout {
            placed,
            unplaced,
            extent,
        },
        turns: turns.clone(),
        score,
    };
    (decoded, Candidate { order, turns })
}

/// A number from where the items lie now (FNV-1a over their lies and corners), so that a
/// search from another board takes other random steps.
fn seed_of(now: &[Option<Now>]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut add = |v: u64| {
        for byte in v.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for n in now {
        match n {
            Some(n) => {
                add(n.lie as u64);
                add(n.low.0.to_bits());
                add(n.low.1.to_bits());
            }
            None => add(u64::MAX),
        }
    }
    hash
}

/// How many layouts the search places after its starts (nesting-search.md §4.5): none when
/// there is nothing to change, else [`super::search::budget`].
fn budget(items: &[Item]) -> usize {
    let turns = items.iter().any(|i| i.lies.len() > 1);
    if items.len() < 2 && !turns {
        0
    } else {
        super::search::budget(items.len())
    }
}

/// The orders tried: largest area first, then longest side, then widest in X, then
/// tallest in Y; ties in item order. Repeated orders are tried once.
pub(super) fn orders(items: &[Item]) -> Vec<Vec<usize>> {
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
