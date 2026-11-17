// SPDX-License-Identifier: GPL-3.0-or-later
//! Bottom-left fill (docs/specs/nesting.md §4.2-4.5): each instance, largest first, at
//! the lowest place along the board's longer side, then across it, where its footprint grown
//! by the gap meets nothing placed; several orders tried, the best kept.
//!
//! The bottom-left rule is from Jakobs, *On genetic algorithms for the packing of polygons*,
//! EJOR 88 (1996), and Chazelle, *The bottom-left bin-packing heuristic: an efficient
//! implementation*, IEEE Transactions on Computers C-32 (1983).
//!
//! The board is kept as runs of occupied cells along each line across it. A footprint is
//! tried at one place along the board by sliding it across: wherever one of its runs meets
//! one of the board's, it jumps past that run, so a line full of parts is crossed in a few
//! steps instead of cell by cell.

use chipload_core::Point;
use chipload_core::tolerances::PLACEMENT_ROUNDING_MM;

use super::footprint::{Footprint, Frame};
use super::grid::Grid;
use super::sweep::Sweep;
use crate::{Cancel, ToolpathError};

/// How much farther along the board a part may go to lie lower across it, mm
/// (nesting.md §4.3, as built).
pub const ALONG_SLACK_MM: f64 = 5.0;

/// Runs of occupied cells along one line, (first, last) inclusive, sorted, apart.
type Runs = Vec<(i64, i64)>;

/// One way an item can lie: its turn, its footprint there, and the runs of the footprint
/// grown by the gap, line by line along the board's longer side.
#[derive(Debug, Clone)]
pub struct Lie {
    /// Radians, counter-clockwise: what the caller turned the sweep by.
    pub angle: f64,
    pub footprint: Footprint,
    /// The grown footprint's runs, per line along the board, and its margin in cells.
    grown: Vec<Runs>,
    margin: i64,
    /// The plain footprint's runs, per line: what it occupies once placed.
    plain: Vec<Runs>,
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

/// A layout: what was placed, what didn't fit (indices into the items), and how far along
/// the board's longer side it reaches from the usable area's start, mm.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub placed: Vec<Put>,
    pub unplaced: Vec<usize>,
    pub used: f64,
}

/// The usable area of a board (the board less its margin, within the machine's travel):
/// its low and high corners, mm, the cell side, and what may not be covered.
#[derive(Debug, Clone)]
pub struct Sheet {
    pub low: Point,
    pub high: Point,
    pub cell: f64,
    /// Along X (true) or Y: the longer side, packed along first (decision 6).
    along_x: bool,
    /// Keep-out areas (dowel holes), as runs per line along.
    keep_out: Vec<Runs>,
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
        let frame = Frame { origin: low, cell };
        for sweep in keep_out {
            frame.add_sweep(&mut grid, sweep);
        }
        let along_x = width >= height;
        let keep_out = runs(&grid, along_x);
        Some(Sheet {
            low,
            high,
            cell,
            along_x,
            keep_out,
        })
    }

    /// The way `footprint` (turned by `angle`) lies on this sheet, with the gap.
    pub fn lie(&self, angle: f64, footprint: Footprint, gap: f64) -> Lie {
        let grown = footprint.dilated(gap);
        Lie {
            angle,
            grown: runs(&grown.grid, self.along_x),
            margin: grown.margin as i64,
            plain: runs(&footprint.grid, self.along_x),
            footprint,
        }
    }

    /// Size along and across of the usable area, mm.
    fn lengths(&self) -> (f64, f64) {
        let (w, h) = (self.high.0 - self.low.0, self.high.1 - self.low.1);
        if self.along_x { (w, h) } else { (h, w) }
    }

    /// A footprint's size along and across, mm.
    fn footprint_lengths(&self, f: &Footprint) -> (f64, f64) {
        let (w, h) = f.size();
        if self.along_x { (w, h) } else { (h, w) }
    }

    /// The low corner of a footprint whose cell (0, 0) is on cell (`along`, `across`).
    fn corner(&self, along: i64, across: i64) -> Point {
        let (a, c) = (along as f64 * self.cell, across as f64 * self.cell);
        if self.along_x {
            (self.low.0 + a, self.low.1 + c)
        } else {
            (self.low.0 + c, self.low.1 + a)
        }
    }

    /// The last cell a footprint `length` mm long may start at on a side `side` mm long,
    /// its far end within the side, compared in mm (nesting.md §4.5); None when it is
    /// longer than the side.
    fn last_start(&self, length: f64, side: f64) -> Option<i64> {
        let room = side - length + PLACEMENT_ROUNDING_MM;
        (room >= 0.0).then(|| (room / self.cell).floor() as i64)
    }
}

/// The placer's board while it fills: the occupied runs per line along.
struct Filling<'a> {
    sheet: &'a Sheet,
    occupied: Vec<Runs>,
}

impl Filling<'_> {
    /// The lowest place for `lie`: the first cell along where it fits anywhere across,
    /// and there the lowest across. None when it fits nowhere.
    fn place(&self, lie: &Lie) -> Option<(i64, i64)> {
        let (last_along, last_across) = self.limits(lie)?;
        (0..=last_along).find_map(|along| {
            self.lowest_across(lie, along, last_across)
                .map(|across| (along, across))
        })
    }

    /// The last cells along and across `lie` may start at; None when it is larger than the
    /// sheet.
    fn limits(&self, lie: &Lie) -> Option<(i64, i64)> {
        let (side_along, side_across) = self.sheet.lengths();
        let (length_along, length_across) = self.sheet.footprint_lengths(&lie.footprint);
        Some((
            self.sheet.last_start(length_along, side_along)?,
            self.sheet.last_start(length_across, side_across)?,
        ))
    }

    /// The best place for any of `lies` (nesting.md §4.3): the lowest along where one fits
    /// anywhere, but up to [`ALONG_SLACK_MM`] farther along when that is lower across, so a
    /// part doesn't go to the top of the board for a millimetre's gain. Then the lowest
    /// across, the lowest along, the first lie.
    fn best(&self, lies: &[Lie]) -> Option<((i64, i64), usize)> {
        let firsts: Vec<Option<(i64, i64)>> = lies.iter().map(|lie| self.place(lie)).collect();
        let first = firsts.iter().flatten().map(|&(along, _)| along).min()?;
        let reach = first + (ALONG_SLACK_MM / self.sheet.cell).floor() as i64;
        let mut best: Option<((i64, i64), usize)> = None;
        for (l, (lie, found)) in lies.iter().zip(&firsts).enumerate() {
            let (Some((start, _)), Some((last_along, last_across))) = (found, self.limits(lie))
            else {
                continue;
            };
            for along in *start..=reach.min(last_along) {
                let Some(across) = self.lowest_across(lie, along, last_across) else {
                    continue;
                };
                let better = best.is_none_or(|((a, c), b)| (across, along, l) < (c, a, b));
                if better {
                    best = Some(((along, across), l));
                }
            }
        }
        best
    }

    /// The lowest cell across, up to `last`, where `lie` fits at cell `along`.
    fn lowest_across(&self, lie: &Lie, along: i64, last: i64) -> Option<i64> {
        let mut across = 0;
        'slide: loop {
            if across > last {
                return None;
            }
            for (i, runs) in lie.grown.iter().enumerate() {
                let line = along + i as i64 - lie.margin;
                let Some(board) = usize::try_from(line)
                    .ok()
                    .and_then(|l| self.occupied.get(l))
                else {
                    continue;
                };
                for &(first, end) in runs {
                    let (low, high) = (across + first - lie.margin, across + end - lie.margin);
                    if let Some(blocked) = meeting(board, low, high) {
                        // Past the run it met: its first cell just beyond.
                        across = blocked + 1 - (first - lie.margin);
                        continue 'slide;
                    }
                }
            }
            return Some(across);
        }
    }

    /// `lie` placed with its cell (0, 0) at (`along`, `across`): its plain runs occupied.
    fn occupy(&mut self, lie: &Lie, along: i64, across: i64) {
        for (i, runs) in lie.plain.iter().enumerate() {
            let line = along + i as i64;
            let Some(board) = usize::try_from(line)
                .ok()
                .and_then(|l| self.occupied.get_mut(l))
            else {
                continue;
            };
            for &(first, end) in runs {
                insert(board, across + first, across + end);
            }
        }
    }
}

/// Fill `sheet` with `items` in each of the orders (nesting.md §4.4) and keep the best: the
/// most placed, then the least board used along, then the smallest box round them, then
/// the first order. Deterministic: the same items give the same layout.
pub fn fill(sheet: &Sheet, items: &[Item], cancel: &Cancel) -> Result<Layout, ToolpathError> {
    let mut best: Option<(Layout, f64)> = None;
    for order in orders(sheet, items) {
        let (layout, area) = fill_in_order(sheet, items, &order, cancel)?;
        let better = match &best {
            None => true,
            Some((b, b_area)) => {
                let key = |l: &Layout, area: f64| (std::cmp::Reverse(l.placed.len()), l.used, area);
                let (k, bk) = (key(&layout, area), key(b, *b_area));
                k.0 < bk.0 || (k.0 == bk.0 && (k.1 < bk.1 || (k.1 == bk.1 && k.2 < bk.2)))
            }
        };
        if better {
            best = Some((layout, area));
        }
    }
    Ok(best.map(|(l, _)| l).unwrap_or(Layout {
        placed: Vec::new(),
        unplaced: Vec::new(),
        used: 0.0,
    }))
}

/// The orders tried: largest area first, then longest side, then widest along, then widest
/// across; ties in item order. Repeated orders are tried once.
fn orders(sheet: &Sheet, items: &[Item]) -> Vec<Vec<usize>> {
    let size = |k: usize| {
        items
            .get(k)
            .and_then(|item| item.lies.first())
            .map_or((0.0, 0.0), |lie| sheet.footprint_lengths(&lie.footprint))
    };
    let keys: [&dyn Fn((f64, f64)) -> f64; 4] = [
        &|(a, c)| a * c,
        &|(a, c)| a.max(c),
        &|(a, _)| a,
        &|(_, c)| c,
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

/// One fill in `order`, and the area of the box round what it placed.
fn fill_in_order(
    sheet: &Sheet,
    items: &[Item],
    order: &[usize],
    cancel: &Cancel,
) -> Result<(Layout, f64), ToolpathError> {
    let mut filling = Filling {
        sheet,
        occupied: sheet.keep_out.clone(),
    };
    let (mut placed, mut unplaced) = (Vec::new(), Vec::new());
    let (mut used, mut across_used) = (0.0_f64, 0.0_f64);
    for &k in order {
        if cancel.is_cancelled() {
            return Err(ToolpathError::Cancelled);
        }
        let Some(item) = items.get(k) else { continue };
        let Some(((along, across), l)) = filling.best(&item.lies) else {
            unplaced.push(k);
            continue;
        };
        let Some(lie) = item.lies.get(l) else {
            continue;
        };
        filling.occupy(lie, along, across);
        let (length_along, length_across) = sheet.footprint_lengths(&lie.footprint);
        used = used.max(along as f64 * sheet.cell + length_along);
        across_used = across_used.max(across as f64 * sheet.cell + length_across);
        placed.push(Put {
            item: k,
            lie: l,
            low: sheet.corner(along, across),
        });
    }
    unplaced.sort_unstable();
    Ok((
        Layout {
            placed,
            unplaced,
            used,
        },
        used * across_used,
    ))
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
