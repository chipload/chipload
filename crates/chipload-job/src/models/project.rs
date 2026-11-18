// SPDX-License-Identifier: GPL-3.0-or-later
//! The project itself: the job, its setups and parts, copies, and the drawing's units.

use super::*;
use chipload_core::math;

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
    /// Set: this setup is the back of another, the same board turned over. Its stock and
    /// work zero then follow that setup's ([`Job::follow_fronts`]), and its cuts are
    /// mirrored about the board's centre line. Null: a board of its own. Since schema 36.
    // Spec: docs/specs/two-sided.md.
    #[serde(default)]
    pub back_of: Option<BackOf>,
    /// Set: the setup is cut as a puzzle, in pieces that each fit a board, joined by
    /// seams with joints; each piece is a program of its own. Null: one board. Since
    /// schema 37.
    // Spec: docs/specs/pieces.md.
    #[serde(default)]
    pub pieces: Option<Pieces>,
    /// How the setup was last nested: its gap and margin, kept so *Nest again* needs no
    /// questions. Missing: never nested. Left out when missing.
    // Spec: docs/specs/nesting.md §10 (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nest: Option<Nest>,
    /// The parts cut from this board, each placed: a part once where it is drawn, or moved,
    /// turned, mirrored, several times. Cut grouped by part in the project's order, each
    /// part's instances in this order. Since schema 41, in place of each part's own
    /// placement and instances.
    // Spec: docs/specs/parts-and-setups.md §2.
    pub instances: Vec<Instance>,
    /// A board Chipload placed itself for one run, its (min, max) corners in drawing
    /// coordinates: a piece's board, fitted round the piece (docs/specs/pieces.md §7). It
    /// overrides where the stock would be and its size; the thickness, material and the
    /// rest stay the stock's. Never saved, never read from a file.
    #[serde(skip)]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub placed: Option<((f64, f64), (f64, f64))>,
}

/// A setup's nesting settings (docs/specs/nesting.md §10).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Nest {
    /// The least distance between what two instances cut, mm.
    pub gap: f64,
    /// The least distance from what an instance cuts to the board's edge, mm.
    pub margin: f64,
    /// What the nest makes as small as it can (docs/specs/nesting-search.md §2, §6);
    /// missing in a file: compact.
    #[serde(default)]
    pub goal: NestGoal,
}

/// What a nest makes as small as it can, after placing as much as it can
/// (docs/specs/nesting-search.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NestGoal {
    /// The smallest box round the parts: a block in the board's lower-left corner.
    #[default]
    Compact,
    /// The lowest top edge: a strip across the top of the board left free.
    Low,
    /// The leftmost right edge: a strip down the right of the board left free.
    Left,
}

/// How a part may be turned when it is nested (docs/specs/nesting.md §3): wood grain is
/// the usual reason for none or half.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Turns {
    /// As it lies.
    None,
    /// As it lies or turned 180°.
    Half,
    /// A quarter turn at a time.
    #[default]
    Quarter,
    /// 15° at a time.
    #[serde(rename = "every_15")]
    Every15,
}

impl Turns {
    pub const ALL: [Turns; 4] = [Turns::None, Turns::Half, Turns::Quarter, Turns::Every15];

    /// The name in the file and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Turns::None => "none",
            Turns::Half => "half",
            Turns::Quarter => "quarter",
            Turns::Every15 => "every_15",
        }
    }

    /// The turns tried, radians counter-clockwise, as it lies first; each in −π…π
    /// (270° is −90°), counter-clockwise ones before clockwise.
    pub fn angles(self) -> Vec<f64> {
        let steps: u32 = match self {
            Turns::None => 1,
            Turns::Half => 2,
            Turns::Quarter => 4,
            Turns::Every15 => 24,
        };
        let step = std::f64::consts::TAU / f64::from(steps);
        (0..steps)
            .map(|k| {
                if 2 * k <= steps {
                    f64::from(k) * step
                } else {
                    -f64::from(steps - k) * step
                }
            })
            .collect()
    }
}

/// How a setup is cut in pieces (docs/specs/pieces.md §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Pieces {
    /// The boards the pieces are cut from: each piece, its joints and the setup's margin
    /// fit on one.
    pub board: PieceBoard,
    /// Columns and rows of pieces. Null: the fewest whose pieces fit the board.
    #[serde(default)]
    pub grid: Option<PieceGrid>,
    /// The joints along the seams.
    pub joint: Joint,
    /// The joint's size, mm: a knob's head diameter, a finger's length.
    pub size: f64,
    /// How far apart joints are along a seam, mm: knob to knob, or a finger and a gap.
    pub spacing: f64,
    /// The room between two pieces at a seam, mm, for glue: each side gives half.
    pub clearance: f64,
    /// Each inner seam between columns moved along X from its even position, mm, from the
    /// left. Missing ones: 0.
    #[serde(default)]
    pub shift_x: Vec<f64>,
    /// Each inner seam between rows moved along Y from its even position, mm, from the
    /// bottom. Missing ones: 0.
    #[serde(default)]
    pub shift_y: Vec<f64>,
}

/// The size of the boards pieces are cut from, mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PieceBoard {
    pub x: f64,
    pub y: f64,
}

/// A grid of pieces given by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PieceGrid {
    pub columns: u32,
    pub rows: u32,
}

/// The joint along a seam between two pieces (docs/specs/pieces.md §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Joint {
    /// Round heads on narrower necks, alternating in and out, as on a jigsaw puzzle.
    Knobs,
    /// Square teeth with rounded corners.
    Fingers,
}

impl Joint {
    pub fn name(self) -> &'static str {
        match self {
            Joint::Knobs => "knobs",
            Joint::Fingers => "fingers",
        }
    }

    /// The joint's default size and spacing, mm.
    pub fn defaults(self) -> (f64, f64) {
        match self {
            Joint::Knobs => (30.0, 120.0),
            Joint::Fingers => (15.0, 60.0),
        }
    }
}

/// The margin round each piece's board when the setup's board has none of its own (a
/// fixed size), mm (docs/specs/pieces.md §7).
pub const DEFAULT_PIECE_MARGIN: f64 = 10.0;

/// The room between pieces at a seam by default, mm (docs/specs/pieces.md §5).
pub const DEFAULT_PIECE_CLEARANCE: f64 = 0.2;

impl Pieces {
    /// Pieces with a joint's defaults on boards of this size.
    pub fn new(board: PieceBoard, joint: Joint) -> Pieces {
        let (size, spacing) = joint.defaults();
        Pieces {
            board,
            grid: None,
            joint,
            size,
            spacing,
            clearance: DEFAULT_PIECE_CLEARANCE,
            shift_x: Vec::new(),
            shift_y: Vec::new(),
        }
    }
}

/// Which setup a back setup turns over, and how (docs/specs/two-sided.md §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct BackOf {
    /// The id of the front setup: the board as it lies first.
    pub setup: u64,
    /// How the board is turned over.
    pub turn: Turn,
    /// How the turned board is put back in place.
    pub register: Register,
}

/// How a board is turned over: about its vertical or its horizontal centre line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Turn {
    /// About the vertical centre line: the left edge goes to the right, X is mirrored.
    LeftRight,
    /// About the horizontal centre line: the bottom edge goes to the top, Y is mirrored.
    TopBottom,
}

impl Turn {
    pub fn name(self) -> &'static str {
        match self {
            Turn::LeftRight => "left-right",
            Turn::TopBottom => "top-bottom",
        }
    }
}

/// How a turned board is located again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Register {
    /// Dowel pins through holes on the turn axis, drilled with the front.
    Pins,
    /// Pushed into the same corner of a fence: the board's size must be exact.
    Corner,
}

impl Register {
    pub fn name(self) -> &'static str {
        match self {
            Register::Pins => "pins",
            Register::Corner => "corner",
        }
    }
}

/// A part: a named group of operations, cut together (docs/specs/parts.md §2). It owns no
/// geometry, its operations select the shapes, and no placement: the setups hold its
/// instances (docs/specs/parts-and-setups.md §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// Its number, unique in the project: what operations and instances name it by.
    pub id: u64,
    /// Its name, unique in the project, shown in the Browser, the report and the sheet.
    pub name: String,
    /// False: the part and all its operations stay in the project but aren't cut, on any
    /// board.
    pub enabled: bool,
    /// `by_operation`: each operation on every instance, then the next; `by_instance`:
    /// every operation on one instance, then the next instance. Since schema 38.
    pub order: InstanceOrder,
    /// Cut mirrored about the centre line of its box, the drawing as drawn: every instance
    /// with it. Missing: as drawn. Left out when missing.
    // Spec: docs/specs/mirror-part.md (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<Mirror>,
    /// How it may be turned when nested. Missing: a quarter turn at a time. Left out when
    /// missing.
    // Spec: docs/specs/nesting.md §10 (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<Turns>,
    /// The stock it is cut from: a material, a thickness, or both. Setting it moves the
    /// part to a board that has it (docs/specs/parts-and-setups.md §4). Missing: any board.
    /// Left out when missing.
    // Spec: docs/specs/parts-and-setups.md §4 (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock: Option<PartStock>,
}

/// The stock a part is cut from (docs/specs/parts-and-setups.md §4): either may be missing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PartStock {
    /// The library material. Null: any.
    pub material: Option<MaterialRef>,
    /// The board's thickness, mm. Null: any.
    pub thickness: Option<f64>,
}

impl PartStock {
    /// Neither a material nor a thickness: the part goes on any board.
    pub fn is_empty(&self) -> bool {
        self.material.is_none() && self.thickness.is_none()
    }

    /// Whether a board of `stock` is what the part asks for: the same thickness (within
    /// the depth tolerance) and the same library material, where the part names them.
    pub fn fits(&self, stock: &Stock) -> bool {
        let thickness = self
            .thickness
            .is_none_or(|t| (t - stock.thickness).abs() <= DEPTH_TOL_MM);
        let material = self.material.as_ref().is_none_or(|m| {
            stock
                .material
                .as_ref()
                .is_some_and(|b| b.library_uid == m.library_uid)
        });
        thickness && material
    }
}

/// How a part is cut mirrored: about the vertical or the horizontal centre line of its box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Mirror {
    /// About the vertical centre line: left and right change places, X is mirrored.
    LeftRight,
    /// About the horizontal centre line: top and bottom change places, Y is mirrored.
    TopBottom,
}

impl Mirror {
    pub fn name(self) -> &'static str {
        match self {
            Mirror::LeftRight => "left-right",
            Mirror::TopBottom => "top-bottom",
        }
    }
}

/// A part placed on a board (docs/specs/parts-and-setups.md §2, instances.md §2): mirrored,
/// then turned, about the part's centre, then moved.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Instance {
    /// The part placed (`parts`).
    pub part: u64,
    /// Its number, from 1, unique among the part's instances on every board; kept when it
    /// moves to another board.
    pub id: u32,
    /// False: kept, but not cut.
    pub enabled: bool,
    /// Mirrored about the part's centre line first. Null: as drawn.
    pub mirror: Option<Mirror>,
    /// Then turned about the part's centre, radians, counter-clockwise.
    pub angle: f64,
    /// Then moved, mm.
    pub at: [f64; 2],
}

impl Instance {
    /// Part `part` placed where it is drawn, as instance `id`.
    pub fn drawn(part: u64, id: u32) -> Instance {
        Instance::moved(part, id, [0.0, 0.0])
    }

    /// An instance only moved.
    pub fn moved(part: u64, id: u32, at: [f64; 2]) -> Instance {
        Instance {
            part,
            id,
            enabled: true,
            mirror: None,
            angle: 0.0,
            at,
        }
    }

    pub fn spot(&self) -> Spot {
        Spot {
            mirror: self.mirror,
            angle: self.angle,
            at: self.at,
        }
    }

    /// Set to a placement.
    pub fn set_spot(&mut self, spot: Spot) {
        self.mirror = spot.mirror;
        self.angle = spot.angle;
        self.at = spot.at;
    }

    /// Cut where its part is drawn: not mirrored, turned or moved.
    pub fn is_drawn(&self) -> bool {
        self.spot() == Spot::default()
    }
}

/// Where an instance lies relative to its part: mirrored, then turned, about the part's
/// centre, then moved. The default leaves the part as drawn.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Spot {
    pub mirror: Option<Mirror>,
    pub angle: f64,
    pub at: [f64; 2],
}

impl Spot {
    /// Only moved: no mirror, no turn.
    pub fn is_move(&self) -> bool {
        self.mirror.is_none() && self.angle == 0.0
    }
}

/// In which order a part and its instances are cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum InstanceOrder {
    /// Each operation on every instance, then the next operation.
    #[default]
    ByOperation,
    /// Every operation on one instance, then the next instance. Called `by_copy` before
    /// schema 38, and `by_part` before parts came (schema 22).
    ByInstance,
}

/// Most columns or rows of a grid, and most instances of a part in all.
pub const MAX_COPIES_PER_SIDE: u32 = 100;
pub const MAX_COPIES: u32 = 1000;

/// A grid that makes instances (docs/specs/instances.md §5, `part instance array`): the
/// part at (column c, row r) moved by (c·dx, r·dy) mm. Not saved: the instances are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Copies {
    /// Along X, at least 1.
    pub columns: u32,
    /// Along Y, at least 1.
    pub rows: u32,
    /// From one to the next along X, mm; negative grows to the left.
    pub dx: f64,
    /// From one to the next along Y, mm; negative grows downwards.
    pub dy: f64,
}

impl Copies {
    /// The grid's instances, the part itself (copy 1) left out, numbered as the grid's
    /// copies were (row by row from 1) and listed in the order they are cut: row by row,
    /// alternating direction (2, 3, then 6, 5, 4).
    pub fn instances(&self, part: u64) -> Vec<Instance> {
        self.in_cut_order()
            .into_iter()
            .skip(1)
            .map(|(number, (dx, dy))| Instance::moved(part, number, [dx, dy]))
            .collect()
    }

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

    /// What is wrong with the grid, as "{at}.field: message" (`at` is where it was given).
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
        self.follow_fronts();
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
        // A relief ends at its model's lowest point (docs/specs/stl-relief.md §5).
        let models: Vec<(u64, f64)> = self
            .drawing
            .models
            .iter()
            .map(|m| (m.id, m.depth))
            .collect();
        for op in &mut self.operations {
            if let Operation::Relief(r) = op
                && let Some((_, depth)) = models.iter().find(|(id, _)| *id == r.model)
            {
                r.end_depth = r.start_depth + depth;
            }
        }
    }

    /// How far below its full diameter `op`'s cut reaches: a drilled hole's drill point
    /// (docs/specs/holes.md decision 10); 0 for everything else, flat at the bottom.
    /// Give every helix that no longer fits its tool the default for that tool, keeping its
    /// angle (docs/specs/styles.md §2.4): one as wide as the tool's radius or wider leaves a
    /// core standing, as one made for a larger tool does after the tool changed. Call after
    /// an operation's tool, or a tool's diameter, changes.
    pub fn fit_helixes(&mut self) {
        for op in &mut self.operations {
            op.fit_helixes(&self.tools);
        }
    }

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

    /// The setup whose back setup `id` is, when it is one and its front exists.
    pub fn front_of(&self, id: u64) -> Option<&Setup> {
        let back = self.setup(id)?.back_of?;
        self.setup(back.setup)
    }

    /// The index of the front setup `front`'s dowel holes operation, if it has one.
    pub fn dowels_of(&self, front: u64) -> Option<usize> {
        self.operations
            .iter()
            .position(|op| op.is_dowels() && self.setup_of(op).is_some_and(|s| s.id == front))
    }

    /// Whether back setup `back` is put back by dowel pins and its front has no dowel holes
    /// to fit them, none or none that is on (docs/specs/two-sided-wizard.md §3).
    pub fn missing_dowels(&self, back: u64) -> bool {
        let Some(link) = self.setup(back).and_then(|s| s.back_of) else {
            return false;
        };
        link.register == Register::Pins
            && !self.operations.iter().any(|op| {
                op.is_dowels()
                    && op.enabled()
                    && self.setup_of(op).is_some_and(|s| s.id == link.setup)
            })
    }

    /// The back setup of setup `id`, if it has one.
    pub fn back_of(&self, id: u64) -> Option<&Setup> {
        self.setups
            .iter()
            .find(|s| s.back_of.is_some_and(|b| b.setup == id))
    }

    /// Every back setup takes its front's board and work zero (docs/specs/two-sided.md §3,
    /// §5): one board, so one stock and one zero, kept in step wherever a project is read
    /// or changed ([`Job::follow_thickness`] calls it first). The board is as thick as the
    /// front leaves it: thinner by what the front surfaces. A back whose front is missing,
    /// or is itself a back, keeps its own; validation refuses it.
    pub fn follow_fronts(&mut self) {
        let fronts: Vec<Option<(Stock, WorkZero)>> = self
            .setups
            .iter()
            .map(|s| {
                let front = self.front_of(s.id).filter(|f| f.back_of.is_none())?;
                let mut stock = front.stock.clone();
                stock.thickness -= self.surfaced_depth(front.id);
                Some((stock, front.work_zero))
            })
            .collect();
        for (setup, front) in self.setups.iter_mut().zip(fronts) {
            if let Some((stock, zero)) = front {
                setup.stock = stock;
                setup.work_zero = zero;
            }
        }
    }

    /// The active setup's front, when it is a back setup and its front exists
    /// (docs/specs/two-sided.md §4): its cuts are mirrored about the board's centre line.
    pub fn back_turn(&self) -> Option<Turn> {
        let id = self.active_setup_id()?;
        self.front_of(id)?;
        self.setup(id)?.back_of.map(|b| b.turn)
    }

    /// For a back setup, its deepest surface pass that is cut, as (operation index, depth
    /// below the back face, mm): the dowel pins must be set at least that far below the
    /// back face, and over them the pass may go that deep (docs/specs/two-sided.md §6).
    /// None for a front, or a back without a surface pass.
    pub fn surface_over_pins(&self) -> Option<(usize, f64)> {
        self.back_turn()?;
        let active = self.active_setup_id()?;
        let mut deepest: Option<(usize, f64)> = None;
        for (i, op) in self.operations.iter().enumerate() {
            // A board surface faces over the pins; a part's faces only its shapes.
            let Operation::Surface(surface) = op else {
                continue;
            };
            if !op.is_board_surface() {
                continue;
            }
            if !self.cuts(i) || self.setup_of(op).map(|s| s.id) != Some(active) {
                continue;
            }
            if deepest.is_none_or(|(_, depth)| surface.end_depth > depth) {
                deepest = Some((i, surface.end_depth));
            }
        }
        deepest
    }

    /// The spindle's turn as seen in the drawing's coordinates, which every choice of
    /// travel direction is made with (docs/specs/two-sided.md §4): the machine's, and the
    /// other way for a back setup, whose program is mirrored after planning. A path
    /// planned conventional and then mirrored cuts climb on the back face. The G-code's M3
    /// or M4 is always the machine's own spindle.
    pub fn cutting_sense(&self) -> Spindle {
        match (self.back_turn(), self.machine.spindle) {
            (None, spindle) => spindle,
            (Some(_), Spindle::Cw) => Spindle::Ccw,
            (Some(_), Spindle::Ccw) => Spindle::Cw,
        }
    }

    /// The box a mirrored part is mirrored in (docs/specs/mirror-part.md §2): round the
    /// shapes its mirrored operations select, on or off, so switching one off to cut the
    /// rest again doesn't move the others. None when it selects none.
    pub fn part_box(&self, id: u64) -> Option<((f64, f64), (f64, f64))> {
        let selectors: Vec<Selector> = self
            .operations
            .iter()
            .filter(|op| op.part() == id && Self::mirrors(op))
            .map(|op| op.select().resolved(&self.drawing))
            .collect();
        self.drawing
            .extents_of(|shape| selectors.iter().any(|s| s.may_pick(shape)))
    }

    /// The point a part's instances are mirrored and turned about (docs/specs/instances.md
    /// §2): the middle of [`Job::part_box`], worked out each time so the instances follow
    /// the part's shapes. None when the part selects no shapes.
    pub fn part_centre(&self, id: u64) -> Option<(f64, f64)> {
        let ((x0, y0), (x1, y1)) = self.part_box(id)?;
        Some(((x0 + x1) / 2.0, (y0 + y1) / 2.0))
    }

    /// Whether an operation goes with its part's mirror: everything but what belongs to the
    /// board, a surface pass and dowel holes (docs/specs/mirror-part.md §2).
    fn mirrors(op: &Operation) -> bool {
        op.selects_shapes() && !op.is_dowels()
    }

    /// How operation `op` is cut mirrored (docs/specs/mirror-part.md §3): its part's mirror
    /// and the line it mirrors about, X = at for left to right, Y = at for top to bottom,
    /// the centre line of the part's box. None: as drawn.
    pub fn mirror_of(&self, op: &Operation) -> Option<(Mirror, f64)> {
        let mirror = self.part_of(op)?.mirror?;
        if !Self::mirrors(op) {
            return None;
        }
        let ((x0, y0), (x1, y1)) = self.part_box(op.part())?;
        Some(match mirror {
            Mirror::LeftRight => (mirror, (x0 + x1) / 2.0),
            Mirror::TopBottom => (mirror, (y0 + y1) / 2.0),
        })
    }

    /// The cutting sense operation `op` is planned with: [`Job::cutting_sense`], the other
    /// way when its part is mirrored, whose toolpath is mirrored after planning
    /// (docs/specs/mirror-part.md §3). In a back setup the two turns cancel.
    pub fn cutting_sense_of(&self, op: &Operation) -> Spindle {
        match (self.mirror_of(op), self.cutting_sense()) {
            (None, sense) => sense,
            (Some(_), Spindle::Cw) => Spindle::Ccw,
            (Some(_), Spindle::Ccw) => Spindle::Cw,
        }
    }

    /// How deep the front setup `id` surfaces the board: the deepest end of its cut surface
    /// operations, 0 without one. Its back's board is that much thinner
    /// (docs/specs/two-sided.md §5).
    fn surfaced_depth(&self, id: u64) -> f64 {
        self.operations
            .iter()
            .enumerate()
            .filter(|(i, op)| {
                op.is_board_surface()
                    && self.cuts(*i)
                    && self.setups_of_part(op.part()).contains(&id)
            })
            .map(|(_, op)| op.depths(&self.tools).1)
            .fold(0.0, f64::max)
    }

    /// The setup operation `op` is cut in: its part's ([`Job::home_setup`]).
    pub fn setup_of(&self, op: &Operation) -> Option<&Setup> {
        self.setup(self.home_setup(op.part())?)
    }

    /// Every instance of part `id`, on every board, as (setup id, instance): setups in their
    /// order, each setup's in its order (docs/specs/parts-and-setups.md §2).
    pub fn instances_of(&self, id: u64) -> Vec<(u64, &Instance)> {
        self.setups
            .iter()
            .flat_map(|s| {
                s.instances
                    .iter()
                    .filter(move |i| i.part == id)
                    .map(move |i| (s.id, i))
            })
            .collect()
    }

    /// The setups that hold an instance of part `id`, in their order.
    pub fn setups_of_part(&self, id: u64) -> Vec<u64> {
        self.setups
            .iter()
            .filter(|s| s.instances.iter().any(|i| i.part == id))
            .map(|s| s.id)
            .collect()
    }

    /// Whether part `id` has an instance, on or off, on setup `setup`.
    pub fn part_on(&self, id: u64, setup: u64) -> bool {
        self.setup(setup)
            .is_some_and(|s| s.instances.iter().any(|i| i.part == id))
    }

    /// The ids of the parts with an instance on setup `setup`, in the project's order.
    pub fn parts_on(&self, setup: u64) -> Vec<u64> {
        self.parts
            .iter()
            .filter(|p| self.part_on(p.id, setup))
            .map(|p| p.id)
            .collect()
    }

    /// The board part `id` is cut from, where one board is meant: the active setup when it
    /// holds the part, else the first setup that does. None: the part isn't placed anywhere.
    pub fn home_setup(&self, id: u64) -> Option<u64> {
        let holds = self.setups_of_part(id);
        self.active_setup_id()
            .filter(|a| holds.contains(a))
            .or_else(|| holds.first().copied())
    }

    /// The instances of part `id` on the active setup, on or off, in their order.
    pub fn instances_here(&self, id: u64) -> Vec<&Instance> {
        self.setups
            .get(self.active_setup_index())
            .map(|s| s.instances.iter().filter(|i| i.part == id).collect())
            .unwrap_or_default()
    }

    /// The instances of part `id` that are on, on the active setup, as (number, placement),
    /// in cut order. [`Job::for_setup`] makes a setup the active one for planning.
    pub fn spots(&self, id: u64) -> Vec<(u32, Spot)> {
        self.instances_here(id)
            .into_iter()
            .filter(|i| i.enabled)
            .map(|i| (i.id, i.spot()))
            .collect()
    }

    /// [`Job::spots`] as (number, shift): for instances that are only moved.
    pub fn in_cut_order(&self, id: u64) -> Vec<(u32, (f64, f64))> {
        self.spots(id)
            .into_iter()
            .map(|(n, p)| (n, (p.at[0], p.at[1])))
            .collect()
    }

    /// How many times part `id` is cut on the active setup: its instances there that are on.
    pub fn count(&self, id: u64) -> u32 {
        u32::try_from(self.spots(id).len()).unwrap_or(u32::MAX)
    }

    /// How many instances part `id` has that are on, on every board.
    pub fn count_all(&self, id: u64) -> u32 {
        let on = self
            .instances_of(id)
            .iter()
            .filter(|(_, i)| i.enabled)
            .count();
        u32::try_from(on).unwrap_or(u32::MAX)
    }

    /// Whether part `id` is cut only elsewhere than drawn: none of its instances that are
    /// on, on any board, lies where the part is drawn (docs/specs/part-placement.md §4).
    pub fn is_placed(&self, id: u64) -> bool {
        !self
            .instances_of(id)
            .iter()
            .any(|(_, i)| i.enabled && i.is_drawn())
    }

    /// Instance `number` of part `id`, on whichever board it is.
    pub fn instance(&self, id: u64, number: u32) -> Option<&Instance> {
        self.setups
            .iter()
            .flat_map(|s| &s.instances)
            .find(|i| i.part == id && i.id == number)
    }

    /// Instance `number` of part `id`, to change.
    pub fn instance_mut(&mut self, id: u64, number: u32) -> Option<&mut Instance> {
        self.setups
            .iter_mut()
            .flat_map(|s| &mut s.instances)
            .find(|i| i.part == id && i.id == number)
    }

    /// A free number for a new instance of part `id`: one more than the largest on any
    /// board, from 1.
    pub fn next_instance_id(&self, id: u64) -> u32 {
        self.instances_of(id)
            .iter()
            .map(|(_, i)| i.id)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// The setup with this id, to change.
    pub fn setup_mut(&mut self, id: u64) -> Option<&mut Setup> {
        self.setups.iter_mut().find(|s| s.id == id)
    }

    /// Part `id` placed on setup `setup` where it is drawn, as a new instance; false when
    /// there is no such setup.
    pub fn place_part(&mut self, id: u64, setup: u64) -> bool {
        let number = self.next_instance_id(id);
        match self.setup_mut(setup) {
            Some(s) => {
                s.instances.push(Instance::drawn(id, number));
                true
            }
            None => false,
        }
    }

    /// Every instance of part `id` moved onto setup `setup`, keeping its number and
    /// placement, after the instances there; false when there is no such setup.
    pub fn move_part_to(&mut self, id: u64, setup: u64) -> bool {
        if self.setup(setup).is_none() {
            return false;
        }
        let mut moved = Vec::new();
        for s in &mut self.setups {
            if s.id != setup {
                moved.extend(s.instances.iter().filter(|i| i.part == id).copied());
                s.instances.retain(|i| i.part != id);
            }
        }
        if let Some(s) = self.setup_mut(setup) {
            s.instances.extend(moved);
        }
        true
    }

    /// Part `id` cut as a grid on the active setup (`part instance array`,
    /// docs/specs/instances.md §5): its first instance here stays as it is (made where drawn
    /// when it has none here), its other instances here are replaced by the grid's,
    /// numbered as the grid numbers them; a number already used gets a free one.
    pub fn set_grid_here(&mut self, id: u64, grid: &Copies) {
        let first = self
            .instances_here(id)
            .first()
            .map(|i| **i)
            .unwrap_or_else(|| Instance::drawn(id, self.next_instance_id(id)));
        let first = Some(first);
        let elsewhere: Vec<u32> = {
            let here = self.active_setup_id();
            self.instances_of(id)
                .iter()
                .filter(|(s, _)| Some(*s) != here)
                .map(|(_, i)| i.id)
                .collect()
        };
        let mut used: Vec<u32> = elsewhere.clone();
        used.extend(first.map(|f| f.id));
        let mut list: Vec<Instance> = first.into_iter().collect();
        for mut instance in grid.instances(id) {
            if used.contains(&instance.id) {
                instance.id = used.iter().max().copied().unwrap_or(0) + 1;
            }
            used.push(instance.id);
            list.push(instance);
        }
        self.set_instances_here(id, list);
    }

    /// The parts on setup `setup` whose stock the board isn't (docs/specs/parts-and-setups.md
    /// §4): placed there by hand, or the board changed since. In the project's order.
    pub fn stock_mismatches(&self, setup: u64) -> Vec<u64> {
        let Some(board) = self.setup(setup) else {
            return Vec::new();
        };
        self.parts
            .iter()
            .filter(|p| self.part_on(p.id, setup))
            .filter(|p| p.stock.as_ref().is_some_and(|s| !s.fits(&board.stock)))
            .map(|p| p.id)
            .collect()
    }

    /// Every instance of part `id` removed, from every board.
    pub fn remove_part_instances(&mut self, id: u64) {
        for s in &mut self.setups {
            s.instances.retain(|i| i.part != id);
        }
    }

    /// The instances of part `id` on the active setup replaced by `instances` (each given
    /// the part), where they were in the list; the instances of the part on other boards
    /// stay.
    pub fn set_instances_here(&mut self, id: u64, instances: Vec<Instance>) {
        let index = self.active_setup_index();
        let Some(setup) = self.setups.get_mut(index) else {
            return;
        };
        let at = setup
            .instances
            .iter()
            .position(|i| i.part == id)
            .unwrap_or(setup.instances.len());
        setup.instances.retain(|i| i.part != id);
        let at = at.min(setup.instances.len());
        let placed = instances.into_iter().map(|i| Instance { part: id, ..i });
        setup.instances.splice(at..at, placed);
    }

    /// A simple project (docs/specs/parts-and-setups.md §2a): one board, each part on it
    /// once, cut, where it is drawn. Anything more is setups' work.
    pub fn is_simple(&self) -> bool {
        self.setups.len() == 1
            && self.parts.iter().all(|p| {
                matches!(
                    self.instances_of(p.id).as_slice(),
                    [(_, i)] if i.enabled && i.is_drawn()
                )
            })
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
    /// and the parts with no instance on there are off, so planning cuts only its parts with its
    /// stock and zero. Operation numbers stay as they are.
    pub fn for_setup(&self, id: u64) -> Job {
        let mut job = self.clone();
        job.active_setup = Some(id);
        let holds: Vec<u64> = self
            .setup(id)
            .map(|s| {
                s.instances
                    .iter()
                    .filter(|i| i.enabled)
                    .map(|i| i.part)
                    .collect()
            })
            .unwrap_or_default();
        for part in &mut job.parts {
            if !holds.contains(&part.id) {
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
    /// A relief's rim reaches past its shapes, and a fitted board grows round it
    /// (docs/specs/stl-relief.md §16); so do the parts' instances.
    pub fn board_extents(&self) -> Option<((f64, f64), (f64, f64))> {
        // A part placed elsewhere isn't cut where its shapes are drawn: they don't count,
        // its placed box does (docs/specs/part-placement.md §4, decision 5).
        let elsewhere = self.cut_only_elsewhere();
        let drawing = if self.setups.len() < 2 {
            self.drawing.extents_of(|shape| !elsewhere(shape))
        } else {
            let board = self.board_setups();
            let selectors: Vec<Selector> = self
                .operations
                .iter()
                .filter(|op| {
                    // Dowel holes count too: a board fitted round the drawing grows round
                    // its pins (docs/specs/two-sided.md §6, as changed 2026-11-12).
                    op.selects_shapes() && self.on_board(op, &board)
                })
                .map(|op| op.select().resolved(&self.drawing))
                .collect();
            self.drawing.extents_of(|shape| {
                selectors.iter().any(|s| s.may_pick(shape)) && !elsewhere(shape)
            })
        };
        let board = self.board_setups();
        let rims = self.operations.iter().filter_map(|op| match op {
            Operation::Relief(r) if self.setups.len() < 2 || self.on_board(op, &board) => {
                let rim = r.rim?;
                let select = r.select.resolved(&self.drawing);
                let ((x0, y0), (x1, y1)) =
                    self.drawing.extents_of(|shape| select.may_pick(shape))?;
                let reach = rim.width + rim_run(r.end_depth, rim.ramp);
                Some(((x0 - reach, y0 - reach), (x1 + reach, y1 + reach)))
            }
            _ => None,
        });
        let instances = self.instance_boxes(&board);
        rims.chain(instances)
            .fold(drawing, |all, ((x0, y0), (x1, y1))| {
                Some(match all {
                    None => ((x0, y0), (x1, y1)),
                    Some(((a0, b0), (a1, b1))) => {
                        ((a0.min(x0), b0.min(y0)), (a1.max(x1), b1.max(y1)))
                    }
                })
            })
    }

    /// The box round each enabled instance of the board's parts, placed: its part's box
    /// turned about the part's centre and moved (docs/specs/instances.md §2), so a board that
    /// fits the drawing holds them too; the part itself too when it is placed. A mirror keeps the box. A free angle gives the box
    /// round the turned box, a little larger than the shapes need.
    fn instance_boxes(&self, board: &[u64]) -> Vec<((f64, f64), (f64, f64))> {
        let mut boxes = Vec::new();
        for part in &self.parts {
            if !part.enabled {
                continue;
            }
            let Some(((x0, y0), (x1, y1))) = self.part_box(part.id) else {
                continue;
            };
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            let spots = self
                .instances_of(part.id)
                .into_iter()
                .filter(|(s, i)| i.enabled && (self.setups.len() < 2 || board.contains(s)))
                .map(|(_, i)| i.spot());
            for spot in spots {
                if spot == Spot::default() {
                    // Cut where drawn: its shapes count already.
                    continue;
                }
                let (sin, cos) = (math::sin(spot.angle), math::cos(spot.angle));
                let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
                    let (dx, dy) = (x - cx, y - cy);
                    (
                        cx + dx * cos - dy * sin + spot.at[0],
                        cy + dx * sin + dy * cos + spot.at[1],
                    )
                });
                let (mut lo, mut hi) = (corners[0], corners[0]);
                for (x, y) in &corners[1..] {
                    lo = (lo.0.min(*x), lo.1.min(*y));
                    hi = (hi.0.max(*x), hi.1.max(*y));
                }
                boxes.push((lo, hi));
            }
        }
        boxes
    }

    /// Whether a shape is cut only by parts placed elsewhere than drawn: picked by an
    /// operation of a placed part and by none of a part cut where drawn. Always false when
    /// no part is placed.
    fn cut_only_elsewhere(&self) -> impl Fn(&Shape) -> bool + '_ {
        let mut placed = Vec::new();
        let mut drawn = Vec::new();
        if self.parts.iter().any(|p| self.is_placed(p.id)) {
            for op in self.operations.iter().filter(|op| op.selects_shapes()) {
                let select = op.select().resolved(&self.drawing);
                match self.part_of(op) {
                    // Dowel holes belong to the board: never placed.
                    Some(part) if self.is_placed(part.id) && !op.is_dowels() => placed.push(select),
                    _ => drawn.push(select),
                }
            }
        }
        move |shape: &Shape| {
            placed.iter().any(|s| s.may_pick(shape)) && !drawn.iter().any(|s| s.may_pick(shape))
        }
    }

    /// Whether `op` cuts one of the setups `board`.
    fn on_board(&self, op: &Operation, board: &[u64]) -> bool {
        self.setups_of_part(op.part())
            .iter()
            .any(|s| board.contains(s))
    }

    /// The setups that cut the active setup's board: it, and its front or its back
    /// (docs/specs/two-sided.md §3). A board that fits fits both sides' shapes, so the front
    /// and the back lie on the same rectangle.
    fn board_setups(&self) -> Vec<u64> {
        let Some(id) = self.active_setup_id() else {
            return Vec::new();
        };
        let mut board = vec![id];
        board.extend(self.front_of(id).map(|s| s.id));
        board.extend(self.back_of(id).map(|s| s.id));
        board
    }

    /// The active setup's board placed for one run ([`Setup::placed`]), if any.
    pub fn placed_board(&self) -> Option<((f64, f64), (f64, f64))> {
        self.setups.get(self.active_setup_index())?.placed
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
        if let Some(placed) = self.placed_board() {
            return placed;
        }
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
        if let Some(((x0, y0), (x1, y1))) = self.placed_board() {
            return (x1 - x0, y1 - y0);
        }
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
            math::hypot(x - at.0, y - at.1)
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

/// How far out a relief rim's ramp reaches the stock top from its foot, mm, for a relief
/// `depth` deep (docs/specs/stl-relief.md §16): 0 for a straight wall.
pub fn rim_run(depth: f64, ramp: f64) -> f64 {
    let tan = math::tan(ramp);
    if tan.is_finite() && tan > 0.0 {
        depth.max(0.0) / tan
    } else {
        0.0
    }
}
