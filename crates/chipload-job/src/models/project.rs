// SPDX-License-Identifier: GPL-3.0-or-later
//! The project itself: the job, its setups and parts, copies, and the drawing's units.

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// The version of the file format. Chipload upgrades older projects when it opens them,
    /// and refuses newer ones.
    pub schema_version: i64,
    /// The Chipload version that wrote the file, for reference.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The drawing: the shapes Chipload cuts, imported from a DXF or drawn in Chipload.
    pub drawing: Drawing,
    /// The setups: each a board clamped on the machine, with its stock, its work zero and
    /// the parts cut from it, written as a program of its own. At least one. Since schema 34.
    // Spec: docs/specs/setups.md.
    pub setups: Vec<Setup>,
    /// The setup shown, simulated and previewed, where new parts go (its id). Null or no
    /// such setup: the first. Only the app and the command line read it.
    #[serde(default)]
    pub active_setup: Option<u64>,
    /// The machine: post-processor, safe height, limits.
    pub machine: Machine,
    /// The tools the operations cut with.
    pub tools: Vec<Tool>,
    /// The parts: groups of operations, each cut with its copies before the next part.
    /// At least one.
    // Spec: docs/specs/parts.md.
    pub parts: Vec<Part>,
    /// The part new operations go into (its id). Null or no such part: the first part.
    /// Only the app and the command line read it; it changes nothing that is cut.
    #[serde(default)]
    pub active_part: Option<u64>,
    /// The operations, cut in this order: grouped by part, in the parts' order.
    pub operations: Vec<Operation>,
}

/// A setup: one board clamped once on the machine, with its stock and work zero, and the
/// parts cut from it (docs/specs/setups.md §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Setup {
    /// Its number, unique in the project: what parts name it by.
    pub id: u64,
    /// Its name, shown in the Browser, the report and the setup sheet.
    pub name: String,
    /// False: the setup and its parts stay in the project but aren't cut.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The board.
    pub stock: Stock,
    /// The point of the drawing that becomes X0 Y0 in its G-code. Missing: the drawing's
    /// origin.
    // Spec: docs/specs/work-zero.md.
    #[serde(default)]
    pub work_zero: WorkZero,
}

/// A part: a named group of operations, cut together, with its own copies
/// (docs/specs/parts.md §2). It owns no geometry: its operations select the shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// Its number, unique in the project: what operations name it by.
    pub id: u64,
    /// Its name, unique in the project, shown in the Browser, the report and the sheet.
    pub name: String,
    /// False: the part and all its operations stay in the project but aren't cut.
    pub enabled: bool,
    /// Cut the part several times in a grid. Null: once.
    // Spec: docs/specs/copies-grid.md.
    pub copies: Option<Copies>,
    /// The id of the setup it is cut in (`setups`). Since schema 34.
    // Spec: docs/specs/setups.md.
    pub setup: u64,
}

impl Part {
    /// How many times the part is cut.
    pub fn count(&self) -> u32 {
        self.copies.map_or(1, |c| c.count())
    }

    /// Its copies as (number, shift) in cut order; one, unshifted, without a grid.
    pub fn in_cut_order(&self) -> Vec<(u32, (f64, f64))> {
        self.copies
            .map_or_else(|| vec![(1, (0.0, 0.0))], |c| c.in_cut_order())
    }
}

/// Most columns or rows of copies, and most copies in all.
pub const MAX_COPIES_PER_SIDE: u32 = 100;
pub const MAX_COPIES: u32 = 1000;

/// A grid of copies of a part: copy (column c, row r) is the part moved by (c·dx, r·dy)
/// mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Copies {
    /// Copies along X, at least 1.
    pub columns: u32,
    /// Copies along Y, at least 1.
    pub rows: u32,
    /// Distance from copy to copy along X, mm; negative grows to the left.
    pub dx: f64,
    /// Distance from copy to copy along Y, mm; negative grows downwards.
    pub dy: f64,
    /// `by_operation`: each operation on every copy, then the next; `by_copy`: every
    /// operation on one copy, then the next copy.
    #[serde(default)]
    pub order: CopyOrder,
}

/// In which order the copies are cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CopyOrder {
    /// Each operation on every copy, then the next operation.
    #[default]
    ByOperation,
    /// Every operation on one copy, then the next copy. Called `by_part` before parts
    /// came (schema 22).
    ByCopy,
}

impl Copies {
    pub fn count(&self) -> u32 {
        self.columns.saturating_mul(self.rows)
    }

    /// Every copy as (number, shift), in cut order: row by row from copy 1, alternating
    /// direction (1, 2, 3, then 6, 5, 4). Copies are numbered row by row from 1, the
    /// drawing as drawn.
    pub fn in_cut_order(&self) -> Vec<(u32, (f64, f64))> {
        let mut out = Vec::new();
        for row in 0..self.rows {
            let mut columns: Vec<u32> = (0..self.columns).collect();
            if row % 2 == 1 {
                columns.reverse();
            }
            for column in columns {
                let number = row * self.columns + column + 1;
                out.push((
                    number,
                    (f64::from(column) * self.dx, f64::from(row) * self.dy),
                ));
            }
        }
        out
    }

    /// What is wrong with the grid, as "{at}.field: message" (`at` is its path, e.g.
    /// "parts.0.copies").
    pub fn problems(&self, at: &str) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (name, n) in [("columns", self.columns), ("rows", self.rows)] {
            if !(1..=MAX_COPIES_PER_SIDE).contains(&n) {
                problems.push(Problem::at(
                    format!("{at}.{name}"),
                    Message::new("job-copies-per-side")
                        .arg("most", MAX_COPIES_PER_SIDE)
                        .arg("got", n),
                ));
            }
        }
        if self.count() > MAX_COPIES {
            problems.push(Problem::at(
                at,
                Message::new("job-copies-too-many")
                    .arg("most", MAX_COPIES)
                    .arg("got", self.count()),
            ));
        }
        for (name, pitch, n) in [("dx", self.dx, self.columns), ("dy", self.dy, self.rows)] {
            if !pitch.is_finite() {
                problems.push(Problem::at(
                    format!("{at}.{name}"),
                    Message::new("job-must-be-number")
                        .arg("got", Value::LengthNumber(pitch, Digits::Shortest)),
                ));
            } else if n > 1 && pitch == 0.0 {
                problems.push(Problem::at(
                    format!("{at}.{name}"),
                    Message::new("job-copies-pitch-zero"),
                ));
            }
        }
        problems
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Mm,
    Inch,
}

impl Units {
    pub fn name(self) -> &'static str {
        match self {
            Units::Mm => "mm",
            Units::Inch => "inch",
        }
    }

    /// Millimetres in one of these units.
    pub fn mm_per_unit(self) -> f64 {
        match self {
            Units::Mm => 1.0,
            Units::Inch => chipload_core::MM_PER_INCH,
        }
    }
}

/// The units a message is shown in (docs/specs/translations.md §7).
impl From<Units> for chipload_i18n::Units {
    fn from(units: Units) -> Self {
        match units {
            Units::Mm => chipload_i18n::Units::Mm,
            Units::Inch => chipload_i18n::Units::Inch,
        }
    }
}

impl Job {
    /// Every operation cut through the stock ends its setup's thickness plus `through` down
    /// (docs/specs/job-settings-layout.md §5), a drill's point lower so its full diameter
    /// goes through: run wherever a project is read or changed, so no end depth is left
    /// from an older thickness.
    pub fn follow_thickness(&mut self) {
        let depths: Vec<(f64, f64)> = self
            .operations
            .iter()
            .map(|op| {
                let thickness = self.setup_of(op).map_or(0.0, |s| s.stock.thickness);
                (thickness, self.drill_point(op))
            })
            .collect();
        for (op, (thickness, point)) in self.operations.iter_mut().zip(depths) {
            op.follow_thickness(thickness, point);
        }
    }

    /// How far below its full diameter `op`'s cut reaches: a drilled hole's drill point
    /// (docs/specs/holes.md decision 10); 0 for everything else, flat at the bottom.
    pub fn drill_point(&self, op: &Operation) -> f64 {
        match op {
            Operation::Holes(h) if matches!(h.method, HoleMethod::Drill { .. }) => self
                .tool(h.tool)
                .filter(|t| t.kind == ToolType::Drill)
                .map_or(0.0, |t| DRILL_POINT_PER_DIAMETER * t.diameter),
            _ => 0.0,
        }
    }

    /// The setup with this id.
    pub fn setup(&self, id: u64) -> Option<&Setup> {
        self.setups.iter().find(|s| s.id == id)
    }

    /// The setup operation `op` is cut in: its part's.
    pub fn setup_of(&self, op: &Operation) -> Option<&Setup> {
        self.setup(self.part_of(op)?.setup)
    }

    /// The id of the active setup: `active_setup` when it names a setup, else the first.
    pub fn active_setup_id(&self) -> Option<u64> {
        self.active_setup
            .filter(|id| self.setup(*id).is_some())
            .or_else(|| self.setups.first().map(|s| s.id))
    }

    /// Where the active setup is in `setups` (0 when there is none, which validation
    /// refuses).
    pub fn active_setup_index(&self) -> usize {
        let id = self.active_setup_id();
        self.setups
            .iter()
            .position(|s| Some(s.id) == id)
            .unwrap_or(0)
    }

    /// The active setup's stock: what planning, the simulation and the view use
    /// (docs/specs/setups.md §5). [`Job::for_setup`] makes a setup the active one.
    pub fn stock(&self) -> &Stock {
        &self.setups[self.active_setup_index()].stock
    }

    /// The active setup's stock, to change.
    pub fn stock_mut(&mut self) -> &mut Stock {
        let index = self.active_setup_index();
        &mut self.setups[index].stock
    }

    /// The active setup's work zero.
    pub fn work_zero(&self) -> WorkZero {
        self.setups[self.active_setup_index()].work_zero
    }

    /// The active setup's work zero, to change.
    pub fn work_zero_mut(&mut self) -> &mut WorkZero {
        let index = self.active_setup_index();
        &mut self.setups[index].work_zero
    }

    /// This project as setup `id` alone (docs/specs/setups.md §4): `id` is the active setup,
    /// and the parts of the other setups are off, so planning cuts only its parts with its
    /// stock and zero. Operation numbers stay as they are.
    pub fn for_setup(&self, id: u64) -> Job {
        let mut job = self.clone();
        job.active_setup = Some(id);
        for part in &mut job.parts {
            if part.setup != id {
                part.enabled = false;
            }
        }
        job
    }

    /// The part with this id.
    pub fn part(&self, id: u64) -> Option<&Part> {
        self.parts.iter().find(|p| p.id == id)
    }

    /// The project tool with this id.
    pub fn tool(&self, id: i64) -> Option<&Tool> {
        find_tool(&self.tools, id)
    }

    /// The part operation `op` belongs to.
    pub fn part_of(&self, op: &Operation) -> Option<&Part> {
        self.part(op.part())
    }

    /// Whether operation `index` is cut: it is on and so is its part (docs/specs/parts.md
    /// §3).
    pub fn cuts(&self, index: usize) -> bool {
        self.operations.get(index).is_some_and(|op| {
            op.enabled()
                && self.part_of(op).is_some_and(|p| p.enabled)
                && self.setup_of(op).is_some_and(|s| s.enabled)
        })
    }

    /// The id of the part new operations go into: `active_part` when it names a part, else
    /// the first part.
    pub fn active_part_id(&self) -> Option<u64> {
        self.active_part
            .filter(|id| self.part(*id).is_some())
            .or_else(|| self.parts.first().map(|p| p.id))
    }

    /// Where an operation added to part `id` goes: after the part's last operation, or,
    /// for a part without operations, after the last operation of the parts before it.
    pub fn insert_index(&self, id: u64) -> usize {
        let order: Vec<u64> = self.parts.iter().map(|p| p.id).collect();
        let rank = |part: u64| order.iter().position(|p| *p == part).unwrap_or(usize::MAX);
        let mine = rank(id);
        self.operations
            .iter()
            .rposition(|op| rank(op.part()) <= mine)
            .map_or(0, |i| i + 1)
    }

    /// The drawing a board is placed on (docs/specs/setups.md §3): with one setup the whole
    /// drawing, as always; with several, the shapes the active setup's operations select.
    pub fn board_extents(&self) -> Option<((f64, f64), (f64, f64))> {
        if self.setups.len() < 2 {
            return self.drawing.extents();
        }
        let setup = self.active_setup_id();
        let selectors: Vec<Selector> = self
            .operations
            .iter()
            .filter(|op| op.selects_shapes() && self.part_of(op).map(|p| p.setup) == setup)
            .map(|op| op.select().resolved(&self.drawing))
            .collect();
        self.drawing
            .extents_of(|shape| selectors.iter().any(|s| s.may_pick(shape)))
    }

    /// The stock's (min, max) corners in drawing coordinates, wherever it is placed: at the
    /// drawing origin, or centred on the drawing (its setup's part of it).
    pub fn stock_bounds(&self) -> ((f64, f64), (f64, f64)) {
        self.stock_bounds_around(self.board_extents())
    }

    /// [`Job::stock_bounds`] around these board extents ([`Job::board_extents`]).
    pub fn stock_bounds_around(
        &self,
        board: Option<((f64, f64), (f64, f64))>,
    ) -> ((f64, f64), (f64, f64)) {
        let centre = board.map_or((0.0, 0.0), |((x0, y0), (x1, y1))| {
            ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
        });
        self.stock()
            .xy_bounds_around(self.stock_size_around(board), centre)
    }

    /// The stock's size, mm: the drawing's outline plus the margin each side when it fits
    /// the drawing, otherwise as set.
    pub fn stock_size(&self) -> (f64, f64) {
        self.stock_size_around(self.board_extents())
    }

    /// [`Job::stock_size`] around these board extents ([`Job::board_extents`]).
    pub fn stock_size_around(&self, board: Option<((f64, f64), (f64, f64))>) -> (f64, f64) {
        match (self.stock().origin, self.stock().fit, board) {
            (StockOrigin::Drawing, Some(margin), Some(((x0, y0), (x1, y1)))) => {
                (x1 - x0 + 2.0 * margin, y1 - y0 + 2.0 * margin)
            }
            _ => (self.stock().x, self.stock().y),
        }
    }

    /// The work zero in drawing coordinates: subtract it from every X, Y of the toolpath to
    /// get program coordinates.
    pub fn work_zero_xy(&self) -> (f64, f64) {
        self.work_zero_point(self.work_zero())
    }

    /// Where work zero `zero` would be, in drawing coordinates.
    pub fn work_zero_point(&self, zero: WorkZero) -> (f64, f64) {
        Self::work_zero_on(zero, self.stock_bounds())
    }

    /// Where work zero `zero` would be on a stock with these (min, max) corners.
    pub fn work_zero_on(zero: WorkZero, stock: ((f64, f64), (f64, f64))) -> (f64, f64) {
        match zero {
            WorkZero::DrawingOrigin {} => (0.0, 0.0),
            WorkZero::Point { x, y } => (x, y),
            WorkZero::Stock { corner } => Self::corner_of(stock, corner),
        }
    }

    /// A corner or the centre of the stock, in drawing coordinates.
    pub fn stock_corner_xy(&self, corner: StockCorner) -> (f64, f64) {
        Self::corner_of(self.stock_bounds(), corner)
    }

    fn corner_of(
        ((x0, y0), (x1, y1)): ((f64, f64), (f64, f64)),
        corner: StockCorner,
    ) -> (f64, f64) {
        match corner {
            StockCorner::Center => ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
            StockCorner::BottomLeft => (x0, y0),
            StockCorner::BottomRight => (x1, y0),
            StockCorner::TopLeft => (x0, y1),
            StockCorner::TopRight => (x1, y1),
        }
    }

    /// The work zero a point dropped at `at` means (docs/specs/work-zero-marker.md §3):
    /// within `reach` mm of a stock corner or its centre, that corner, which follows the
    /// stock; of the drawing origin, the origin; elsewhere the point itself. The nearest
    /// wins.
    pub fn work_zero_at(&self, at: (f64, f64), reach: f64) -> WorkZero {
        Self::work_zero_at_on(self.stock_bounds(), at, reach)
    }

    /// [`Job::work_zero_at`] on a stock with these (min, max) corners.
    pub fn work_zero_at_on(
        stock: ((f64, f64), (f64, f64)),
        at: (f64, f64),
        reach: f64,
    ) -> WorkZero {
        let mut named: Vec<WorkZero> = StockCorner::ALL
            .into_iter()
            .map(|corner| WorkZero::Stock { corner })
            .collect();
        named.push(WorkZero::DrawingOrigin {});
        let distance = |zero: WorkZero| {
            let (x, y) = Self::work_zero_on(zero, stock);
            (x - at.0).hypot(y - at.1)
        };
        named
            .into_iter()
            .map(|zero| (zero, distance(zero)))
            .filter(|&(_, d)| d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(WorkZero::Point { x: at.0, y: at.1 }, |(zero, _)| zero)
    }

    /// "stock center = drawing X150.000 Y110.000, Z0 = stock top", for the G-code header,
    /// the CLI and the GUI. No parentheses or semicolons, so it fits in a G-code comment.
    pub fn work_zero_text(&self) -> String {
        let (x, y) = self.work_zero_xy();
        let at = match self.work_zero() {
            WorkZero::DrawingOrigin {} => "drawing origin".to_string(),
            WorkZero::Stock { corner } => format!("stock {}", corner.name()),
            WorkZero::Point { .. } => "point".to_string(),
        };
        let z = match self.stock().z_zero {
            ZZero::Top => "stock top",
            ZZero::Bottom => "stock bottom",
        };
        format!("{at} = drawing X{x:.3} Y{y:.3}, Z0 = {z}")
    }
}
