// SPDX-License-Identifier: GPL-3.0-or-later
//! Material-removal simulation (docs/specs/simulation.md): the stock as a height map, cut by
//! the planned moves with each tool's shape. Display and report only: it never changes or
//! checks a program.

use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_job::{Job, Tool, ToolType};
use chipload_toolpath::model::{MoveKind, Program};
use chipload_toolpath::{Cancel, ToolGroup};

use crate::PipelineError;

/// The cell sizes tried, smallest first, mm (§1).
pub const CELL_STEPS_MM: [f64; 6] = [0.1, 0.2, 0.25, 0.5, 1.0, 2.0];
/// The automatic cell size keeps the grid at most this many cells (§1).
pub const AUTO_MAX_CELLS: usize = 4_000_000;
/// A chosen cell size may make at most this many (§8).
pub const MAX_CELLS: usize = 16_000_000;
/// The range a chosen cell size must lie in, mm (§6).
pub const MIN_CELL_MM: f64 = 0.05;
pub const MAX_CELL_MM: f64 = 10.0;
/// The display grid's budget: the 3D surface and the playback's stock (docs/specs/
/// playback.md §3).
pub const DISPLAY_CELLS: usize = 250_000;
/// The most history entries kept; past it, playback shows no stock (playback.md §3).
pub const MAX_HISTORY_ENTRIES: usize = 64_000_000;
/// A drill's point angle, radians: the usual 118° (decision 3).
pub const DRILL_POINT_RAD: f64 = 118.0 * std::f64::consts::PI / 180.0;
/// A V-bit without an angle is simulated with this one.
const UNKNOWN_VBIT_RAD: f64 = std::f64::consts::FRAC_PI_2;

/// The stock after the program, as heights on a grid of square cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Simulation {
    /// The lower left corner of cell (0, 0), drawing coordinates, mm.
    pub origin: (f64, f64),
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    /// Row by row from the bottom (lowest Y), each from the left: the top of the material,
    /// program Z, mm.
    pub heights: Vec<f32>,
    /// Program Z of the stock top and bottom.
    pub top: f64,
    pub bottom: f64,
    /// The stock's size, mm (the last column and row may be partly outside it).
    pub size: (f64, f64),
    pub numbers: Numbers,
}

/// What the report shows (§5).
#[derive(Debug, Clone, PartialEq)]
pub struct Numbers {
    pub removed_cm3: f64,
    /// The lowest height of any cell, program Z; None when nothing was cut.
    pub deepest: Option<f64>,
    /// How far below the stock bottom the deepest cut goes, mm, if it does.
    pub below_bottom: Option<f64>,
    /// The program uses a drill, simulated with a 118° point (decision 3).
    pub drill_point_assumed: bool,
    /// A V-bit without an angle, simulated as 90°.
    pub vbit_angle_assumed: bool,
}

/// The shape of a tool's cutting end.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// Flat bottom of radius `radius`.
    Flat { radius: f64 },
    /// A cone rising `slope` mm per mm from a flat tip of radius `tip`, out to `radius`.
    Cone { radius: f64, tip: f64, slope: f64 },
}

impl Shape {
    fn radius(self) -> f64 {
        match self {
            Shape::Flat { radius } | Shape::Cone { radius, .. } => radius,
        }
    }
}

fn shape_of(tool: &Tool, numbers: &mut Numbers) -> Shape {
    let radius = tool.diameter / 2.0;
    let cone = |angle: f64, tip: f64| Shape::Cone {
        radius,
        tip: tip.clamp(0.0, radius),
        slope: 1.0 / (angle / 2.0).tan(),
    };
    match tool.kind {
        ToolType::Endmill => Shape::Flat { radius },
        ToolType::Drill => {
            numbers.drill_point_assumed = true;
            cone(DRILL_POINT_RAD, 0.0)
        }
        ToolType::Vbit => {
            let angle = tool.angle.unwrap_or_else(|| {
                numbers.vbit_angle_assumed = true;
                UNKNOWN_VBIT_RAD
            });
            cone(angle, tool.tip_diameter.unwrap_or(0.0) / 2.0)
        }
    }
}

/// The cell size for a stock of `x` × `y` mm: the smallest step that keeps the grid within
/// [`AUTO_MAX_CELLS`], else the largest step (§1).
pub fn auto_cell(x: f64, y: f64) -> f64 {
    for cell in CELL_STEPS_MM {
        if cells_for(x, y, cell) <= AUTO_MAX_CELLS {
            return cell;
        }
    }
    CELL_STEPS_MM[CELL_STEPS_MM.len() - 1]
}

/// Columns × rows for a stock of `x` × `y` mm at `cell`.
pub fn cells_for(x: f64, y: f64, cell: f64) -> usize {
    let (columns, rows) = grid_size(x, y, cell);
    columns.saturating_mul(rows)
}

fn grid_size(x: f64, y: f64, cell: f64) -> (usize, usize) {
    // Small positive whole numbers for any stock that passed validation.
    let count = |v: f64| ((v / cell).ceil().max(1.0)) as usize;
    (count(x), count(y))
}

/// Cut the stock with `program` (drawing coordinates, before the work-zero shift), each
/// move with the tool of its tool group. `cell`: None for the automatic size.
pub fn simulate(
    job: &Job,
    program: &Program,
    groups: &[ToolGroup],
    cell: Option<f64>,
    cancel: &Cancel,
) -> Result<Simulation, PipelineError> {
    run(job, program, groups, cell, cancel, false).map(|(sim, _)| sim)
}

/// [`simulate`], also recording every cell's history on the display grid (docs/specs/
/// playback.md §3). The history is None when it would pass [`MAX_HISTORY_ENTRIES`].
pub fn simulate_with_history(
    job: &Job,
    program: &Program,
    groups: &[ToolGroup],
    cell: Option<f64>,
    cancel: &Cancel,
) -> Result<(Simulation, Option<History>), PipelineError> {
    run(job, program, groups, cell, cancel, true)
}

fn run(
    job: &Job,
    program: &Program,
    groups: &[ToolGroup],
    cell: Option<f64>,
    cancel: &Cancel,
    with_history: bool,
) -> Result<(Simulation, Option<History>), PipelineError> {
    let stock = &job.stock;
    let cell = match cell {
        Some(cell) => {
            if !(MIN_CELL_MM..=MAX_CELL_MM).contains(&cell) {
                return Err(PipelineError::Job(format!(
                    "cell size must be between {MIN_CELL_MM} and {MAX_CELL_MM} mm, got {cell}"
                )));
            }
            let cells = cells_for(stock.x, stock.y, cell);
            if cells > MAX_CELLS {
                return Err(PipelineError::Job(format!(
                    "a {cell} mm cell makes {cells} cells for this stock, more than \
                     {MAX_CELLS}; use a larger cell"
                )));
            }
            cell
        }
        None => {
            let cell = auto_cell(stock.x, stock.y);
            if cells_for(stock.x, stock.y, cell) > MAX_CELLS {
                return Err(PipelineError::Job(format!(
                    "the stock is too large to simulate at {cell} mm cells (more than \
                     {MAX_CELLS} cells)"
                )));
            }
            cell
        }
    };
    let (columns, rows) = grid_size(stock.x, stock.y, cell);
    let (origin, _) = stock.xy_bounds();
    let top = stock.top_z();
    let mut numbers = Numbers {
        removed_cm3: 0.0,
        deepest: None,
        below_bottom: None,
        drill_point_assumed: false,
        vbit_angle_assumed: false,
    };
    let recorder = with_history.then(|| {
        let factor = factor_for_size(columns, rows, DISPLAY_CELLS, usize::MAX);
        Recorder::new(columns.div_ceil(factor), rows.div_ceil(factor), factor, top)
    });
    let mut grid = Grid {
        origin,
        cell,
        columns,
        rows,
        heights: vec![top as f32; columns * rows],
        recorder,
    };
    let mut here: (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    for (k, m) in program.moves.iter().enumerate() {
        if k % 256 == 0 {
            cancel.check()?;
        }
        let to = (m.x.or(here.0), m.y.or(here.1), m.z.or(here.2));
        let from = here;
        here = to;
        if m.kind == MoveKind::Rapid {
            continue;
        }
        let (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1)) =
            (from.0, from.1, from.2, to.0, to.1, to.2)
        else {
            continue;
        };
        let Some(tool) = tool_at(job, groups, k) else {
            continue;
        };
        let shape = shape_of(tool, &mut numbers);
        if let Some(recorder) = &mut grid.recorder {
            // Indexes of a program's moves fit in u32 (far fewer than 4 billion moves).
            recorder.current = k as u32;
            recorder.path = ((x0, y0), (x1, y1), shape.radius());
        }
        grid.cut((x0, y0, z0), (x1, y1, z1), shape);
    }
    let bottom = top - stock.thickness;
    numbers.removed_cm3 = grid.removed(top, stock.x, stock.y) / 1000.0;
    let lowest = grid
        .heights
        .iter()
        .fold(f64::INFINITY, |m, &h| m.min(f64::from(h)));
    if lowest < top {
        numbers.deepest = Some(lowest);
        if lowest < bottom {
            numbers.below_bottom = Some(bottom - lowest);
        }
    }
    let history = grid.recorder.take().and_then(|r| {
        let (factor, columns, rows) = (r.factor, r.columns, r.rows);
        r.finish(History {
            origin,
            cell: cell * factor as f64,
            columns,
            rows,
            top,
            bottom,
            size: (stock.x, stock.y),
            starts: Vec::new(),
            entries: Vec::new(),
        })
    });
    let sim = Simulation {
        origin,
        cell,
        columns,
        rows,
        heights: grid.heights,
        top,
        bottom,
        size: (stock.x, stock.y),
        numbers,
    };
    Ok((sim, history))
}

/// The smallest factor that brings a `columns` × `rows` grid within `cells` cells and
/// `side` cells per side.
pub fn factor_for_size(columns: usize, rows: usize, cells: usize, side: usize) -> usize {
    let mut factor = 1;
    while columns.div_ceil(factor) * rows.div_ceil(factor) > cells.max(1)
        || columns.div_ceil(factor) > side.max(1)
        || rows.div_ceil(factor) > side.max(1)
    {
        factor += 1;
    }
    factor
}

/// Every cell's history on the display grid (docs/specs/playback.md §3): each time the
/// cell was lowered, the move and the new height, in move order.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    pub origin: (f64, f64),
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    pub top: f64,
    pub bottom: f64,
    pub size: (f64, f64),
    /// Cell `c`'s entries are `entries[starts[c]..starts[c + 1]]`.
    starts: Vec<usize>,
    entries: Vec<Entry>,
}

/// One lowering of a history cell: the move, how far along it the tool first reached the
/// cell (0 to [`ALONG_MAX`] for the move's start to its end), and the cell's height after it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    pub step: u32,
    pub along: u16,
    pub z: f32,
}

/// `Entry::along` at the end of a move.
pub const ALONG_MAX: u16 = u16::MAX;

impl History {
    /// The heights once the first `done` moves are cut (0: the stock as it comes).
    pub fn heights_after(&self, done: usize) -> Vec<f32> {
        self.heights_with(done, None)
    }

    /// The heights once the first `done` moves are cut and move `done` is cut for the
    /// fraction `along` (0 to 1) of its length: a cell that move reaches only further on
    /// is shown as before it. Long straight moves then cut the stock as the tool goes,
    /// not all at once at their end.
    pub fn heights_at(&self, done: usize, along: f64) -> Vec<f32> {
        self.heights_with(done, Some(along_units(along)))
    }

    /// The heights after `done` moves and, with `reached`, the part of move `done` up to it.
    fn heights_with(&self, done: usize, reached: Option<u16>) -> Vec<f32> {
        let cells = self.columns * self.rows;
        (0..cells)
            .map(|c| {
                let run = &self.entries[self.starts[c]..self.starts[c + 1]];
                let n = run.partition_point(|e| {
                    (e.step as usize) < done
                        || (e.step as usize == done && reached.is_some_and(|r| e.along <= r))
                });
                if n == 0 {
                    self.top as f32
                } else {
                    run[n - 1].z
                }
            })
            .collect()
    }

    /// The stock after the first `done` moves as a (display-grid) simulation, shaded on
    /// `numbers`' scale (the final one, so colours don't shift while scrubbing).
    pub fn after(&self, done: usize, numbers: &Numbers) -> Simulation {
        self.at(done, 0.0, numbers)
    }

    /// As [`History::after`], with move `done` cut for the fraction `along` of its length.
    pub fn at(&self, done: usize, along: f64, numbers: &Numbers) -> Simulation {
        Simulation {
            origin: self.origin,
            cell: self.cell,
            columns: self.columns,
            rows: self.rows,
            heights: self.heights_at(done, along),
            top: self.top,
            bottom: self.bottom,
            size: self.size,
            numbers: numbers.clone(),
        }
    }

    /// Cell `c`'s entries, in move order.
    pub fn cell_entries(&self, c: usize) -> &[Entry] {
        &self.entries[self.starts[c]..self.starts[c + 1]]
    }

    /// How many entries are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Builds a [`History`] while the grid is cut.
struct Recorder {
    columns: usize,
    rows: usize,
    factor: usize,
    /// The display grid's current heights.
    heights: Vec<f32>,
    cells: Vec<Vec<Entry>>,
    count: usize,
    overflow: bool,
    /// The move being cut.
    current: u32,
    /// Its tool axis's start and end in XY, and the tool's radius.
    path: ((f64, f64), (f64, f64), f64),
}

/// A fraction of a move (0 to 1) in `Entry::along` units, rounded down.
fn along_units(along: f64) -> u16 {
    // Clamped to 0..=ALONG_MAX first, so the cast can't wrap.
    (along.clamp(0.0, 1.0) * f64::from(ALONG_MAX)).floor() as u16
}

/// How far along the XY path `a` to `b` (0 to 1) a tool of `radius` first reaches `p`:
/// where the distance first drops to the radius, else where it is closest. A move straight
/// up or down cuts at its end.
fn first_reach(p: (f64, f64), a: (f64, f64), b: (f64, f64), radius: f64) -> f64 {
    let d = (b.0 - a.0, b.1 - a.1);
    let w = (a.0 - p.0, a.1 - p.1);
    let qa = d.0 * d.0 + d.1 * d.1;
    if qa.sqrt() <= CHAIN_TOL_MM {
        return 1.0;
    }
    let qb = 2.0 * (d.0 * w.0 + d.1 * w.1);
    let qc = w.0 * w.0 + w.1 * w.1;
    let disc = qb * qb - 4.0 * qa * (qc - radius * radius);
    let t = if disc >= 0.0 {
        (-qb - disc.sqrt()) / (2.0 * qa)
    } else {
        -qb / (2.0 * qa)
    };
    t.clamp(0.0, 1.0)
}

impl Recorder {
    fn new(columns: usize, rows: usize, factor: usize, top: f64) -> Self {
        Recorder {
            columns,
            rows,
            factor,
            heights: vec![top as f32; columns * rows],
            cells: vec![Vec::new(); columns * rows],
            count: 0,
            overflow: false,
            current: 0,
            path: ((0.0, 0.0), (0.0, 0.0), 0.0),
        }
    }

    /// Full cell (i, j), centred on `centre`, went down to `z`.
    fn lowered(&mut self, i: usize, j: usize, centre: (f64, f64), z: f32) {
        if self.overflow {
            return;
        }
        let c = (j / self.factor) * self.columns + i / self.factor;
        if z >= self.heights[c] {
            return;
        }
        self.heights[c] = z;
        let cell = &mut self.cells[c];
        match cell.last_mut() {
            // Lowered again by the same move: the first reach stays.
            Some(last) if last.step == self.current => last.z = z,
            _ => {
                let (a, b, radius) = self.path;
                cell.push(Entry {
                    step: self.current,
                    along: along_units(first_reach(centre, a, b, radius)),
                    z,
                });
                self.count += 1;
                if self.count > MAX_HISTORY_ENTRIES {
                    self.overflow = true;
                    self.cells = Vec::new();
                }
            }
        }
    }

    fn finish(self, mut history: History) -> Option<History> {
        if self.overflow {
            return None;
        }
        history.starts = Vec::with_capacity(self.cells.len() + 1);
        history.entries = Vec::with_capacity(self.count);
        history.starts.push(0);
        for cell in self.cells {
            history.entries.extend(cell);
            history.starts.push(history.entries.len());
        }
        Some(history)
    }
}

/// The tool of the group that move `k` belongs to.
fn tool_at<'a>(job: &'a Job, groups: &[ToolGroup], k: usize) -> Option<&'a Tool> {
    let group = groups.iter().find(|g| g.moves.contains(&k))?;
    job.tool(group.tool.id)
}

struct Grid {
    origin: (f64, f64),
    cell: f64,
    columns: usize,
    rows: usize,
    heights: Vec<f32>,
    recorder: Option<Recorder>,
}

impl Grid {
    /// Lower every cell the tool reaches on its way from `a` to `b` (§2), in pieces of at
    /// most two tool diameters so each looks only at the cells near it.
    fn cut(&mut self, a: (f64, f64, f64), b: (f64, f64, f64), shape: Shape) {
        let radius = match shape {
            Shape::Flat { radius } | Shape::Cone { radius, .. } => radius,
        };
        let length = (b.0 - a.0).hypot(b.1 - a.1);
        let piece = (4.0 * radius).max(self.cell);
        // A small positive whole number: moves are bounded by the machine's travel.
        let pieces = ((length / piece).ceil().max(1.0)) as usize;
        let at = |t: f64| {
            (
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
                a.2 + (b.2 - a.2) * t,
            )
        };
        for p in 0..pieces {
            let (s, e) = (
                at(p as f64 / pieces as f64),
                at((p + 1) as f64 / pieces as f64),
            );
            self.cut_piece(s, e, shape, radius);
        }
    }

    fn cut_piece(&mut self, a: (f64, f64, f64), b: (f64, f64, f64), shape: Shape, radius: f64) {
        let lo = (a.0.min(b.0) - radius, a.1.min(b.1) - radius);
        let hi = (a.0.max(b.0) + radius, a.1.max(b.1) + radius);
        let Some((i0, i1)) = self.span(lo.0, hi.0, self.origin.0, self.columns) else {
            return;
        };
        let Some((j0, j1)) = self.span(lo.1, hi.1, self.origin.1, self.rows) else {
            return;
        };
        for j in j0..=j1 {
            let cy = self.origin.1 + (j as f64 + 0.5) * self.cell;
            for i in i0..=i1 {
                let cx = self.origin.0 + (i as f64 + 0.5) * self.cell;
                if let Some(z) = lowest_over((cx, cy), a, b, shape) {
                    let h = &mut self.heights[j * self.columns + i];
                    let z = z as f32;
                    if z < *h {
                        *h = z;
                        if let Some(recorder) = &mut self.recorder {
                            recorder.lowered(i, j, (cx, cy), z);
                        }
                    }
                }
            }
        }
    }

    /// The cells whose centres lie between `lo` and `hi` along one axis.
    fn span(&self, lo: f64, hi: f64, origin: f64, count: usize) -> Option<(usize, usize)> {
        let first = ((lo - origin) / self.cell - 0.5).ceil().max(0.0);
        let last = ((hi - origin) / self.cell - 0.5).floor();
        if last < 0.0 || first > last || first >= count as f64 {
            return None;
        }
        // Both within 0..count here.
        Some((first as usize, (last as usize).min(count - 1)))
    }

    /// mm³ removed, each cell counted for the part of it inside the stock.
    fn removed(&self, top: f64, x: f64, y: f64) -> f64 {
        let mut total = 0.0;
        for j in 0..self.rows {
            let height = (y - j as f64 * self.cell).min(self.cell).max(0.0);
            for i in 0..self.columns {
                let width = (x - i as f64 * self.cell).min(self.cell).max(0.0);
                let depth = top - f64::from(self.heights[j * self.columns + i]);
                total += depth.max(0.0) * width * height;
            }
        }
        total
    }
}

/// The lowest point of the tool over `p` while its axis moves from `a` to `b` (Z linear),
/// None if it never reaches `p` (§2). Exact: the minimum of a convex function of the move's
/// parameter, taken over its few candidate points.
fn lowest_over(p: (f64, f64), a: (f64, f64, f64), b: (f64, f64, f64), shape: Shape) -> Option<f64> {
    let (radius, tip, slope) = match shape {
        Shape::Flat { radius } => (radius, radius, 0.0),
        Shape::Cone { radius, tip, slope } => (radius, tip, slope),
    };
    let d = (b.0 - a.0, b.1 - a.1);
    let w = (a.0 - p.0, a.1 - p.1);
    // |P(t) − p|² = qa t² + qb t + qc.
    let qa = d.0 * d.0 + d.1 * d.1;
    let qb = 2.0 * (d.0 * w.0 + d.1 * w.1);
    let qc = w.0 * w.0 + w.1 * w.1;
    let dz = b.2 - a.2;
    let z = |t: f64| a.2 + dz * t;
    let dist = |t: f64| (qa * t * t + qb * t + qc).max(0.0).sqrt();
    let height = |t: f64| z(t) + slope * (dist(t) - tip).max(0.0);
    // Where the distance is at most `r`: an interval of t, clipped to [0, 1].
    let within = |r: f64| -> Option<(f64, f64)> {
        // A move shorter than the chaining tolerance in XY: straight up or down.
        if qa.sqrt() <= CHAIN_TOL_MM {
            return (qc <= r * r).then_some((0.0, 1.0));
        }
        let disc = qb * qb - 4.0 * qa * (qc - r * r);
        if disc < 0.0 {
            return None;
        }
        let root = disc.sqrt();
        let (t0, t1) = ((-qb - root) / (2.0 * qa), (-qb + root) / (2.0 * qa));
        let (t0, t1) = (t0.max(0.0), t1.min(1.0));
        (t0 <= t1).then_some((t0, t1))
    };
    let (t0, t1) = within(radius)?;
    let mut candidates = vec![t0, t1];
    if slope > 0.0 {
        if let Some((s0, s1)) = within(tip) {
            candidates.extend([s0, s1]);
        }
        // Where the cone's side and the move's slope cancel: dz + slope·(dist)' = 0.
        if qa > 0.0 {
            let length = qa.sqrt();
            let along = -qb / (2.0 * qa);
            let perpendicular = (qc - qb * qb / (4.0 * qa)).max(0.0).sqrt();
            let k2 = slope * slope * qa - dz * dz;
            if k2 > 0.0 {
                let s = -dz.signum() * dz.abs() * perpendicular / (length * k2.sqrt());
                candidates.push(along + s);
            }
            candidates.push(along);
        }
    }
    candidates
        .into_iter()
        .filter(|t| (t0..=t1).contains(t))
        .map(height)
        .reduce(f64::min)
}

/// The colours of the shaded image (§4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// The uncut stock top.
    pub top: [u8; 3],
    /// The deepest cut.
    pub deep: [u8; 3],
    /// Cells cut below the stock bottom.
    pub below: [u8; 3],
}

/// The CLI's colours: light wood, dark brown, red below the stock.
pub const PRINT_PALETTE: Palette = Palette {
    top: [222, 196, 150],
    deep: [74, 52, 34],
    below: [200, 40, 40],
};

impl Simulation {
    /// The height of cell (column `i`, row `j` from the bottom).
    pub fn height(&self, i: usize, j: usize) -> f64 {
        f64::from(self.heights[j * self.columns + i])
    }

    /// RGBA pixels, one per cell, row 0 at the top (highest Y), shaded by depth with a light
    /// from the upper left (§4).
    pub fn shaded(&self, palette: Palette) -> Vec<u8> {
        let deepest = self.numbers.deepest.unwrap_or(self.top);
        let range = (self.top - deepest).max(f64::MIN_POSITIVE);
        // Light from the upper left, above: unit vector.
        let light = {
            let (x, y, z) = (-1.0f64, 1.0f64, 1.5f64);
            let n = (x * x + y * y + z * z).sqrt();
            (x / n, y / n, z / n)
        };
        let flat = light.2;
        let mut out = Vec::with_capacity(self.columns * self.rows * 4);
        for row in 0..self.rows {
            let j = self.rows - 1 - row;
            for i in 0..self.columns {
                let h = self.height(i, j);
                let at = |i: usize, j: usize| self.height(i, j);
                let left = at(i.saturating_sub(1), j);
                let right = at((i + 1).min(self.columns - 1), j);
                let down = at(i, j.saturating_sub(1));
                let up = at(i, (j + 1).min(self.rows - 1));
                let gx = (right - left) / (2.0 * self.cell);
                let gy = (up - down) / (2.0 * self.cell);
                let n = (gx * gx + gy * gy + 1.0).sqrt();
                let lit = ((-gx * light.0 - gy * light.1 + light.2) / n).max(0.0);
                // Flat ground keeps its colour; slopes go lighter or darker.
                let shade = (0.55 + 0.45 * lit / flat).min(1.25);
                let base = if h < self.bottom - DEPTH_TOL_MM {
                    palette.below
                } else {
                    let f = ((self.top - h) / range).clamp(0.0, 1.0);
                    mix(palette.top, palette.deep, f)
                };
                for c in base {
                    out.push((f64::from(c) * shade).round().clamp(0.0, 255.0) as u8);
                }
                out.push(255);
            }
        }
        out
    }

    /// The same stock on a grid `factor` times coarser, each cell the lowest of the cells it
    /// covers, so narrow cuts stay visible (§4). `factor` 1 is a copy.
    pub fn coarsened(&self, factor: usize) -> Simulation {
        let factor = factor.max(1);
        let columns = self.columns.div_ceil(factor);
        let rows = self.rows.div_ceil(factor);
        let mut heights = vec![self.top as f32; columns * rows];
        for j in 0..self.rows {
            for i in 0..self.columns {
                let h = self.heights[j * self.columns + i];
                let cell = &mut heights[(j / factor) * columns + i / factor];
                if h < *cell {
                    *cell = h;
                }
            }
        }
        Simulation {
            origin: self.origin,
            cell: self.cell * factor as f64,
            columns,
            rows,
            heights,
            top: self.top,
            bottom: self.bottom,
            size: self.size,
            numbers: self.numbers.clone(),
        }
    }

    /// The smallest factor that brings the grid within `cells` cells and `side` cells per
    /// side.
    pub fn factor_for(&self, cells: usize, side: usize) -> usize {
        factor_for_size(self.columns, self.rows, cells, side)
    }

    /// The shaded image as PNG bytes, fixed settings (§6).
    pub fn png(&self, palette: Palette) -> Result<Vec<u8>, PipelineError> {
        let width = u32::try_from(self.columns).map_err(|e| PipelineError::Job(e.to_string()))?;
        let height = u32::try_from(self.rows).map_err(|e| PipelineError::Job(e.to_string()))?;
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Balanced);
        let png_error = |e: png::EncodingError| PipelineError::Job(format!("PNG: {e}"));
        let mut writer = encoder.write_header().map_err(png_error)?;
        writer
            .write_image_data(&self.shaded(palette))
            .map_err(png_error)?;
        writer.finish().map_err(png_error)?;
        Ok(bytes)
    }

    /// The report's lines (§5).
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "Simulation: {:.1} cm³ removed, cells {} mm ({} × {})",
            self.numbers.removed_cm3,
            trim(self.cell),
            self.columns,
            self.rows
        )];
        match self.numbers.deepest {
            Some(z) => lines.push(format!(
                "  Deepest cut: Z{z:.3} ({:.3} mm below the stock top)",
                self.top - z
            )),
            None => lines.push("  Nothing cut".to_string()),
        }
        if let Some(below) = self.numbers.below_bottom {
            lines.push(format!(
                "  Cuts {below:.3} mm below the stock bottom (into the spoilboard)"
            ));
        }
        if self.numbers.drill_point_assumed {
            lines.push("  Drills simulated with a 118° point".to_string());
        }
        if self.numbers.vbit_angle_assumed {
            lines.push("  A V-bit without an angle simulated as 90°".to_string());
        }
        lines
    }

    /// A hash of the heights rounded to 1 µm, for golden files (FNV-1a). Rounded, so the
    /// last bit of a platform's trigonometry doesn't change it.
    pub fn heights_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for h in &self.heights {
            // Heights are within the stock's few hundred mm: far inside i64.
            let microns = (f64::from(*h) * 1000.0).round() as i64;
            for byte in microns.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        }
        hash
    }
}

fn mix(a: [u8; 3], b: [u8; 3], f: f64) -> [u8; 3] {
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * f).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

fn trim(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sampled reference for [`lowest_over`].
    fn sampled(p: (f64, f64), a: (f64, f64, f64), b: (f64, f64, f64), shape: Shape) -> Option<f64> {
        let (radius, tip, slope) = match shape {
            Shape::Flat { radius } => (radius, radius, 0.0),
            Shape::Cone { radius, tip, slope } => (radius, tip, slope),
        };
        let n = 200_000;
        (0..=n)
            .filter_map(|k| {
                let t = k as f64 / n as f64;
                let c = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                let d = (c.0 - p.0).hypot(c.1 - p.1);
                (d <= radius).then(|| a.2 + (b.2 - a.2) * t + slope * (d - tip).max(0.0))
            })
            .reduce(f64::min)
    }

    #[test]
    fn the_exact_minimum_agrees_with_sampling() {
        let shapes = [
            Shape::Flat { radius: 3.0 },
            Shape::Cone {
                radius: 6.35,
                tip: 0.0,
                slope: 1.0 / (30.0f64.to_radians()).tan(),
            },
            Shape::Cone {
                radius: 6.35,
                tip: 0.5,
                slope: 1.0 / (45.0f64.to_radians()).tan(),
            },
            Shape::Cone {
                radius: 4.0,
                tip: 0.0,
                slope: 1.0 / (59.0f64.to_radians()).tan(),
            },
        ];
        // A deterministic spread of moves and points.
        let mut seed = 12345u64;
        let mut next = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 11) as f64) / ((1u64 << 53) as f64)
        };
        for shape in shapes {
            for _ in 0..300 {
                let a = (next() * 20.0, next() * 20.0, -next() * 5.0);
                let b = (next() * 20.0, next() * 20.0, -next() * 5.0);
                let p = (next() * 20.0, next() * 20.0);
                let exact = lowest_over(p, a, b, shape);
                let reference = sampled(p, a, b, shape);
                match (exact, reference) {
                    (Some(e), Some(r)) => {
                        assert!(e <= r + 1e-9, "{shape:?} {a:?} {b:?} {p:?}: {e} > {r}");
                        assert!(r - e < 1e-3, "{shape:?} {a:?} {b:?} {p:?}: {e} vs {r}");
                    }
                    (None, None) => {}
                    // Only at the very edge of reach, where sampling can miss it.
                    (Some(_), None) => {}
                    (None, Some(r)) => panic!("{shape:?} {a:?} {b:?} {p:?}: missed {r}"),
                }
            }
        }
    }

    #[test]
    fn a_coarser_grid_keeps_the_lowest_cell() {
        let sim = Simulation {
            origin: (0.0, 0.0),
            cell: 1.0,
            columns: 3,
            rows: 2,
            heights: vec![0.0, -1.0, 0.0, 0.0, 0.0, -2.0],
            top: 0.0,
            bottom: -10.0,
            size: (3.0, 2.0),
            numbers: Numbers {
                removed_cm3: 0.0,
                deepest: Some(-2.0),
                below_bottom: None,
                drill_point_assumed: false,
                vbit_angle_assumed: false,
            },
        };
        let coarse = sim.coarsened(2);
        assert_eq!((coarse.columns, coarse.rows, coarse.cell), (2, 1, 2.0));
        assert_eq!(coarse.heights, vec![-1.0, -2.0]);
        assert_eq!(sim.factor_for(2, 10), 2);
        assert_eq!(sim.factor_for(100, 2), 2);
        assert_eq!(sim.factor_for(100, 100), 1);
    }

    #[test]
    fn the_automatic_cell_keeps_the_budget() {
        assert_eq!(auto_cell(200.0, 200.0), 0.1);
        assert_eq!(auto_cell(600.0, 400.0), 0.25);
        // 200 × 200 mm is exactly 4 million cells at 0.1 mm; a little more is over.
        assert_eq!(cells_for(200.0, 200.0, 0.1), 4_000_000);
        assert_eq!(auto_cell(200.1, 200.0), 0.2);
        assert_eq!(auto_cell(5000.0, 5000.0), 2.0);
    }
}
