// SPDX-License-Identifier: GPL-3.0-or-later
//! New operations and what they start with (docs/specs/styles.md §2): a profile's side,
//! depth and tabs from what is selected, depths from the last operation of the kind, cutting
//! values from the tool's cutting data. The window, `chipload op add` and styles all make
//! their operations here.

use std::collections::BTreeSet;

use chipload_core::chain::Contour;
use chipload_i18n::{Language, Message, Units, Value};
use chipload_job::library::{Cutting, ToolLibrary};
use chipload_job::{
    ChamferOp, DEFAULT_RAMP_ANGLE_RAD, Direction, EdgeSide, Entry, FillPattern, FluteOp,
    HoleMethod, HolesOp, Job, MouldEnds, MouldProfile, MouldSide, MouldingOp, Operation, PathOp,
    PatternOp, PocketOp, PrismOp, ProfileOp, RampShape, ReliefOp, SURFACE_STEPOVER, Selector, Side,
    SurfaceOp, Tabs, Tool, ToolType, VcarveOp,
};

/// A new relief's lines, as a fraction of the tool, with an end mill or a V-bit
/// (docs/specs/stl-relief.md §5); a ball nose's come from [`RELIEF_RIDGE_MM`].
pub const DEFAULT_RELIEF_STEPOVER: f64 = 0.1;

/// The ridges a new relief's ball nose leaves between its lines on flat ground, mm: below
/// what wood shows (docs/specs/stl-relief.md §15).
pub const RELIEF_RIDGE_MM: f64 = 0.02;

/// The stepover found from the ridge stays within these, as fractions of the tool.
pub const RELIEF_STEPOVER_RANGE: (f64, f64) = (0.05, 0.4);

/// Halvings in finding the stepover: far finer than the whole percent it is rounded to.
const RIDGE_HALVINGS: u32 = 40;

/// The ridge `tool` leaves between relief lines `stepover` (a fraction of it) apart on flat
/// ground, mm: the height of its cutting surface halfway between two lines. Only for a ball
/// nose or a bull nose (none within its flat); a square end mill leaves none on flat ground
/// but steps on slopes, and a V-bit's grooves are its own.
pub fn relief_ridge(tool: &Tool, stepover: f64) -> Option<f64> {
    if !tool.is_round() {
        return None;
    }
    tool.cutter()?.height_at(stepover * tool.diameter / 2.0)
}

/// A new relief's stepover for `tool`: with a ball or bull nose, the widest that leaves ridges of
/// [`RELIEF_RIDGE_MM`], to the nearest whole percent and within [`RELIEF_STEPOVER_RANGE`];
/// otherwise [`DEFAULT_RELIEF_STEPOVER`].
pub fn relief_stepover(tool: Option<&Tool>) -> f64 {
    ridge_stepover(tool, RELIEF_RIDGE_MM).unwrap_or(DEFAULT_RELIEF_STEPOVER)
}

/// With a ball or bull nose, the widest stepover that leaves ridges of `ridge` mm on flat
/// ground, to the nearest whole percent and within [`RELIEF_STEPOVER_RANGE`]; None for
/// another tool.
pub fn ridge_stepover(tool: Option<&Tool>, ridge: f64) -> Option<f64> {
    let tool = tool.filter(|t| t.is_round() && t.diameter > 0.0)?;
    let (lo, hi) = RELIEF_STEPOVER_RANGE;
    // The ridge grows with the stepover: halve towards the widest within the target.
    let (mut below, mut above) = (0.0, 1.0);
    for _ in 0..RIDGE_HALVINGS {
        let mid = (below + above) / 2.0;
        match relief_ridge(tool, mid) {
            Some(h) if h <= ridge => below = mid,
            _ => above = mid,
        }
    }
    Some(((below * 100.0).round() / 100.0).clamp(lo, hi))
}

use crate::chip_load::{default_feed, default_plunge_feed};
use crate::surface::{DEFAULT_PART_SURFACE_STEPOVER, DEFAULT_SURFACE_DEPTH};

/// The depth of a new pocket, V-carve or path with nothing to take it from, and of a
/// profile of open shapes: a shallow test cut (decision 2026-10-09).
pub const DEFAULT_DEPTH: f64 = 1.0;
/// A new chamfer's width at the surface, mm (docs/specs/chamfer.md §6).
pub const DEFAULT_CHAMFER_WIDTH: f64 = 1.0;
/// A new flute's depth with a tool other than a ball nose, mm (docs/specs/fluting.md §8).
pub const DEFAULT_FLUTE_DEPTH: f64 = 3.0;
/// A new flute's ramp at each end, mm (docs/specs/fluting.md §8).
pub const DEFAULT_FLUTE_RAMP: f64 = 20.0;
/// A new pattern's depth, mm (docs/specs/fill-pattern.md §8).
pub const DEFAULT_PATTERN_DEPTH: f64 = 0.5;
/// A new pattern's lines, from the X axis (docs/specs/fill-pattern.md §8).
pub const DEFAULT_PATTERN_ANGLE_DEG: f64 = 45.0;
/// The narrowest spacing a new pattern takes, mm.
pub const MIN_PATTERN_SPACING: f64 = 1.0;
/// A new moulding's cove radius, mm (docs/specs/moulding.md §9).
pub const DEFAULT_MOULDING_RADIUS: f64 = 6.0;
/// A new moulding's stepover with an endmill, as a share of its diameter.
pub const MOULDING_ENDMILL_STEPOVER: f64 = 0.25;
/// A new pocket's stepover, as a share of the tool's diameter, when nothing gives one.
pub const DEFAULT_STEPOVER: f64 = crate::materials::DEFAULT_STEPOVER;

/// The kinds of operation that can be added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Profile,
    Pocket,
    Vcarve,
    /// Follow path (docs/specs/follow-path.md).
    Path,
    /// Chamfer: a V-bit breaks the top edge of closed shapes (docs/specs/chamfer.md).
    Chamfer,
    /// Flute: a groove along open shapes that ramps in and out (docs/specs/fluting.md).
    Flute,
    /// Pattern: grooves engraved inside closed shapes (docs/specs/fill-pattern.md).
    Pattern,
    /// Prism: closed shapes standing as roofs with a ridge (docs/specs/prism.md).
    Prism,
    /// Moulding: a profile swept along a path (docs/specs/moulding.md).
    Moulding,
    /// Holes (docs/specs/holes.md).
    Holes,
    /// Surface: the whole stock (docs/specs/surfacing.md).
    Surface,
    /// Relief: a 3D model carved from the top (docs/specs/stl-relief.md §5).
    Relief,
}

impl OpKind {
    /// Whether it cuts selected shapes: every kind, a surface too, which faces the selected
    /// shapes of its part (the whole board is the setup's *Face the board…*,
    /// docs/specs/surface-levels.md §5).
    pub fn needs_selection(self) -> bool {
        true
    }

    /// Whether `op` is of this kind.
    pub fn is(self, op: &Operation) -> bool {
        matches!(
            (op, self),
            (Operation::Profile(_), OpKind::Profile)
                | (Operation::Pocket(_), OpKind::Pocket)
                | (Operation::Vcarve(_), OpKind::Vcarve)
                | (Operation::Path(_), OpKind::Path)
                | (Operation::Chamfer(_), OpKind::Chamfer)
                | (Operation::Flute(_), OpKind::Flute)
                | (Operation::Pattern(_), OpKind::Pattern)
                | (Operation::Prism(_), OpKind::Prism)
                | (Operation::Moulding(_), OpKind::Moulding)
                | (Operation::Holes(_), OpKind::Holes)
                | (Operation::Surface(_), OpKind::Surface)
                | (Operation::Relief(_), OpKind::Relief)
        )
    }

    fn name_id(self) -> &'static str {
        match self {
            OpKind::Profile => "pipeline-name-profile",
            OpKind::Pocket => "pipeline-name-pocket",
            OpKind::Vcarve => "pipeline-name-vcarve",
            OpKind::Path => "pipeline-name-path",
            OpKind::Chamfer => "pipeline-name-chamfer",
            OpKind::Flute => "pipeline-name-flute",
            OpKind::Pattern => "pipeline-name-pattern",
            OpKind::Prism => "pipeline-name-prism",
            OpKind::Moulding => "pipeline-name-moulding",
            OpKind::Holes => "pipeline-name-holes",
            OpKind::Surface => "pipeline-name-surface",
            OpKind::Relief => "pipeline-name-relief",
        }
    }
}

/// What to make: the kind, its shapes, and a tool other than the one it would get.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRequest {
    pub kind: OpKind,
    pub select: Selector,
    /// The project tool to cut with; None: the one `tool_for` gives.
    pub tool: Option<i64>,
}

/// One new operation, and whether its side is chosen (false only for a profile whose
/// selection doesn't tell: the window then asks).
#[derive(Debug, Clone, PartialEq)]
pub struct NewOp {
    pub op: Operation,
    pub side_chosen: bool,
}

/// The operations made for one request, to be inserted at `index` in this order, with what
/// the report should say about them.
#[derive(Debug, Clone, PartialEq)]
pub struct NewOps {
    pub index: usize,
    pub ops: Vec<NewOp>,
    pub messages: Vec<Message>,
}

/// The tool a new operation gets: the last operation's, else the project's first
/// (docs/specs/tool-per-operation.md §8).
pub fn next_tool_id(job: &Job) -> i64 {
    let known = |id: &i64| job.tools.iter().any(|t| t.id == *id);
    job.operations
        .last()
        .map(|e| e.tool())
        .filter(known)
        .or(job.tools.first().map(|t| t.id))
        .unwrap_or(1)
}

/// The tool a new operation of `kind` starts with: the last operation's tool when it
/// suits the kind, else the project's first that does. A V-carve and a chamfer take a V-bit;
/// other kinds don't (user report 2026-10-14). A surface takes the widest endmill
/// (docs/specs/surfacing.md §4).
pub fn tool_for(job: &Job, kind: OpKind) -> i64 {
    if kind == OpKind::Surface
        && let Some(widest) = crate::surface::surface_tool(job)
    {
        return widest;
    }
    // A relief takes a ball nose when there is one (docs/specs/stl-relief.md §5).
    if kind == OpKind::Relief
        && let Some(ball) = job.tools.iter().find(|t| t.kind == ToolType::Ballnose)
    {
        return ball.id;
    }
    // A moulding takes the first ball nose, else the first endmill (docs/specs/moulding.md
    // §9).
    if kind == OpKind::Moulding
        && let Some(round) = [ToolType::Ballnose, ToolType::Endmill]
            .iter()
            .find_map(|kind| job.tools.iter().find(|t| t.kind == *kind))
    {
        return round.id;
    }
    // A flute takes the first ball nose, else the first V-bit (docs/specs/fluting.md §8).
    if kind == OpKind::Flute
        && let Some(round) = [ToolType::Ballnose, ToolType::Vbit]
            .iter()
            .find_map(|kind| job.tools.iter().find(|t| t.kind == *kind))
    {
        return round.id;
    }
    let suits = |id: i64| {
        job.tools.iter().find(|t| t.id == id).is_some_and(|t| {
            (t.kind == ToolType::Vbit)
                == matches!(
                    kind,
                    OpKind::Vcarve | OpKind::Chamfer | OpKind::Pattern | OpKind::Prism
                )
        })
    };
    let next = next_tool_id(job);
    if suits(next) {
        return next;
    }
    job.tools
        .iter()
        .map(|t| t.id)
        .find(|id| suits(*id))
        .unwrap_or(next)
}

/// Where an operation of `kind` added to the active part goes: after the part's
/// operations; a part surface first in it, after any board surface (it faces the part's top
/// before anything of it is cut, docs/specs/surface-levels.md §2).
pub fn insert_index(job: &Job, kind: OpKind) -> usize {
    let part = job.active_part_id().unwrap_or(1);
    match kind {
        OpKind::Surface => job
            .operations
            .iter()
            .position(|e| e.part() == part && !e.is_board_surface())
            .unwrap_or_else(|| job.insert_index(part)),
        _ => job.insert_index(part),
    }
}

/// The cutting values project tool `tool` has for the active setup's material: its library
/// tool's row for the material, else for any material (docs/specs/cutting-data.md §6). None
/// for a tool not from `library`.
pub fn tool_cutting(job: &Job, library: Option<&ToolLibrary>, tool: i64) -> Option<Cutting> {
    let material = job
        .stock()
        .material
        .as_ref()
        .map(|m| m.library_uid.as_str());
    crate::materials::library_tool(job, library?, tool)?.cutting_for(material)
}

/// The operations `request` makes in `job` (docs/specs/styles.md §2), named in `lang`. Not
/// inserted: the caller puts them at `index`, in order.
pub fn new_operations(
    job: &Job,
    request: &NewRequest,
    contours: &[Contour],
    library: Option<&ToolLibrary>,
    lang: Language,
) -> NewOps {
    let kind = request.kind;
    let index = insert_index(job, kind);
    let tool = request
        .tool
        .or_else(|| match kind {
            OpKind::Holes => crate::matching_drill(job, &request.select, contours),
            _ => None,
        })
        .unwrap_or_else(|| tool_for(job, kind));
    let start = Start::new(job, index, tool, library);
    let number = job.operations.iter().filter(|e| kind.is(e)).count() + 1;
    let name = |n: usize| -> String {
        Message::new(kind.name_id())
            .arg("number", Value::Count(i64::try_from(n).unwrap_or(i64::MAX)))
            .render(lang, Units::Mm)
    };
    let mut messages = Vec::new();
    let made: Vec<(Operation, bool)> = match kind {
        OpKind::Profile => {
            let sides = profile_sides(job, start.part, &request.select, contours);
            match sides {
                Sides::One(side) => vec![(
                    start.profile(name(number), &request.select, side, true),
                    true,
                )],
                Sides::Split { holes, outlines } => {
                    let suffixed = |id: &'static str, n: usize| {
                        Message::new(id)
                            .arg("name", name(n))
                            .render(lang, Units::Mm)
                    };
                    let hole_name = suffixed("pipeline-name-holes-of", number);
                    let outline_name = suffixed("pipeline-name-outline-of", number + 1);
                    if follows_drawing(&request.select) {
                        messages.push(by_shapes(&hole_name, &outline_name));
                    }
                    vec![
                        (start.profile(hole_name, &holes, Side::Inside, true), true),
                        (
                            start.profile(outline_name, &outlines, Side::Outside, true),
                            true,
                        ),
                    ]
                }
                Sides::Unknown => vec![(
                    start.profile(name(number), &request.select, Side::Outside, false),
                    false,
                )],
            }
        }
        // A moulding's profile lies where a profile's waste would be: round outlines,
        // within holes; left of open paths (docs/specs/moulding.md §4).
        OpKind::Moulding => {
            let side = match profile_sides(job, start.part, &request.select, contours) {
                Sides::One(Side::Inside) => MouldSide::Inside,
                Sides::One(_) | Sides::Split { .. } => MouldSide::Outside,
                Sides::Unknown => MouldSide::Left,
            };
            vec![(start.moulding(name(number), &request.select, side), true)]
        }
        // A chamfer's waste is where a profile's would be: around outlines, within holes
        // (docs/specs/chamfer.md §6).
        OpKind::Chamfer => match profile_sides(job, start.part, &request.select, contours) {
            Sides::One(side) => vec![(
                start.chamfer(name(number), &request.select, edge_side(side)),
                true,
            )],
            Sides::Split { holes, outlines } => {
                let suffixed = |id: &'static str, n: usize| {
                    Message::new(id)
                        .arg("name", name(n))
                        .render(lang, Units::Mm)
                };
                let hole_name = suffixed("pipeline-name-holes-of", number);
                let outline_name = suffixed("pipeline-name-outline-of", number + 1);
                if follows_drawing(&request.select) {
                    messages.push(by_shapes(&hole_name, &outline_name));
                }
                vec![
                    (start.chamfer(hole_name, &holes, EdgeSide::Inside), true),
                    (
                        start.chamfer(outline_name, &outlines, EdgeSide::Outside),
                        true,
                    ),
                ]
            }
            Sides::Unknown => vec![(
                start.chamfer(name(number), &request.select, EdgeSide::Outside),
                true,
            )],
        },
        _ => vec![(start.other(kind, name(number), &request.select), true)],
    };
    let ops = made
        .into_iter()
        .map(|(mut op, side_chosen)| {
            start.finish(job, &mut op);
            NewOp { op, side_chosen }
        })
        .collect();
    NewOps {
        index,
        ops,
        messages,
    }
}

/// A new flute's depth with `tool`: a ball nose's radius (a half-round flute), otherwise
/// `DEFAULT_FLUTE_DEPTH`, no deeper than a V-bit's cone (docs/specs/fluting.md §8).
fn flute_depth(tool: &Tool) -> f64 {
    match tool.kind {
        ToolType::Ballnose => tool.diameter / 2.0,
        _ => chipload_toolpath::plan::vbit_cone_depth(tool)
            .map_or(DEFAULT_FLUTE_DEPTH, |deepest| {
                DEFAULT_FLUTE_DEPTH.min(deepest)
            }),
    }
}

/// The kinds of fill pattern, in the order the window lists them
/// (docs/specs/fill-pattern.md §3, §14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    Lines,
    Cross,
    Rings,
    Waves,
    Dashes,
    Herringbone,
    Scales,
    Sunburst,
}

impl PatternKind {
    pub const ALL: [PatternKind; 8] = [
        PatternKind::Lines,
        PatternKind::Cross,
        PatternKind::Rings,
        PatternKind::Waves,
        PatternKind::Dashes,
        PatternKind::Herringbone,
        PatternKind::Scales,
        PatternKind::Sunburst,
    ];

    /// The kind `pattern` is.
    pub fn of(pattern: &FillPattern) -> PatternKind {
        match pattern {
            FillPattern::Lines { .. } => PatternKind::Lines,
            FillPattern::Cross { .. } => PatternKind::Cross,
            FillPattern::Rings { .. } => PatternKind::Rings,
            FillPattern::Waves { .. } => PatternKind::Waves,
            FillPattern::Dashes { .. } => PatternKind::Dashes,
            FillPattern::Herringbone { .. } => PatternKind::Herringbone,
            FillPattern::Scales { .. } => PatternKind::Scales,
            FillPattern::Sunburst { .. } => PatternKind::Sunburst,
        }
    }
}

/// `op`'s pattern switched to `kind`, keeping its angle and spacing where they carry over,
/// the rest from the groove's width with `tool` (docs/specs/fill-pattern.md §14): strokes
/// three widths long (at least 4 mm), gaps half a stroke, a pitch of a stroke, scales of
/// twice the spacing, sunburst rings a stroke and a width apart. A stroke pattern with
/// square ends gets ramps of a third of a stroke, so it is fluted. The same kind: as it is.
pub fn switched_pattern(op: &PatternOp, kind: PatternKind, tool: Option<&Tool>) -> PatternOp {
    let mut out = op.clone();
    if PatternKind::of(&op.pattern) == kind {
        return out;
    }
    let depth = op.end_depth - op.start_depth;
    let width = tool
        .and_then(|t| chipload_toolpath::flute::flute_width_mm(t, depth))
        .or(tool.map(|t| t.diameter))
        .unwrap_or(1.0)
        .max(0.1);
    let spacing = op.pattern.spacing();
    let angle = op
        .pattern
        .angle()
        .unwrap_or(DEFAULT_PATTERN_ANGLE_DEG.to_radians());
    let length = (3.0 * width).max(4.0);
    let whole = true;
    out.pattern = match kind {
        PatternKind::Lines => FillPattern::Lines { angle, spacing },
        PatternKind::Cross => FillPattern::Cross {
            angle,
            spacing,
            between: std::f64::consts::FRAC_PI_2,
        },
        PatternKind::Rings => FillPattern::Rings { spacing },
        PatternKind::Waves => FillPattern::Waves {
            angle,
            spacing,
            wavelength: 4.0 * spacing,
            height: spacing,
        },
        PatternKind::Dashes => FillPattern::Dashes {
            angle,
            spacing,
            length,
            gap: length / 2.0,
            whole,
        },
        PatternKind::Herringbone => FillPattern::Herringbone {
            angle,
            spacing,
            length,
            whole,
        },
        PatternKind::Scales => FillPattern::Scales {
            angle,
            spacing,
            radius: 2.0 * spacing,
            whole,
        },
        PatternKind::Sunburst => FillPattern::Sunburst {
            spacing: length + width,
            length,
            pitch: length,
            whole,
        },
    };
    if out.pattern.strokes() && out.ramp <= 0.0 {
        out.ramp = match out.pattern {
            FillPattern::Scales { radius, .. } => radius,
            _ => length / 3.0,
        };
    }
    out
}

/// A new moulding's stepover with `tool`, mm: with a ball or bull nose, the relief's ridge
/// rule (ridges of 0.02 mm on flat ground); with a flat endmill, a quarter of it.
fn moulding_stepover(tool: Option<&Tool>) -> f64 {
    let diameter = tool.map_or(6.0, |t| t.diameter);
    match ridge_stepover(tool, RELIEF_RIDGE_MM) {
        Some(share) => share * diameter,
        None => MOULDING_ENDMILL_STEPOVER * diameter,
    }
}

/// A new pattern's (depth, spacing) with `tool`: `DEFAULT_PATTERN_DEPTH`, no deeper than
/// a V-bit's cone; the spacing twice the groove's width there, at least
/// `MIN_PATTERN_SPACING` (docs/specs/fill-pattern.md §8).
fn pattern_start(tool: &Tool) -> (f64, f64) {
    let depth = chipload_toolpath::plan::vbit_cone_depth(tool)
        .map_or(DEFAULT_PATTERN_DEPTH, |deepest| {
            DEFAULT_PATTERN_DEPTH.min(deepest)
        });
    let width = chipload_toolpath::flute::flute_width_mm(tool, depth).unwrap_or(tool.diameter);
    (depth, (2.0 * width).max(MIN_PATTERN_SPACING))
}

/// A chamfer's side for a profile's: inside for holes, outside otherwise.
fn edge_side(side: Side) -> EdgeSide {
    match side {
        Side::Inside => EdgeSide::Inside,
        Side::Outside | Side::On => EdgeSide::Outside,
    }
}

/// The note for two profiles split from a layer or a whole drawing: they name their shapes.
pub fn by_shapes(holes: &str, outline: &str) -> Message {
    Message::new("pipeline-new-by-shapes")
        .arg("holes", holes)
        .arg("outline", outline)
}

/// Whether a selector follows the drawing (a layer or a whole source) rather than naming
/// its shapes: splitting it into shapes loses that.
fn follows_drawing(select: &Selector) -> bool {
    select.shapes.is_none() && (select.layer.is_some() || select.source.is_some())
}

/// What a profile's selection tells about its side (docs/specs/styles.md §2.1).
enum Sides {
    /// Outlines only (outside) or holes only (inside).
    One(Side),
    /// Both: the holes and the outlines, each by its shapes.
    Split { holes: Selector, outlines: Selector },
    /// No closed contour, or a selection that can't be told apart.
    Unknown,
}

/// The side a profile of part `part` on `select` defaults to: inside for holes only,
/// outside for outlines only, None (the user chooses) for a mix or nothing
/// (docs/specs/styles.md §2.1). A shape inside an outline only other parts cut is an
/// outline (docs/specs/parts-and-setups.md §2a).
pub fn default_side(job: &Job, part: u64, select: &Selector, contours: &[Contour]) -> Option<Side> {
    match profile_sides(job, part, select, contours) {
        Sides::One(side) => Some(side),
        Sides::Split { .. } | Sides::Unknown => None,
    }
}

fn profile_sides(job: &Job, part: u64, select: &Selector, contours: &[Contour]) -> Sides {
    // Nothing selected yet: an empty selector would match every contour.
    if select.is_empty() {
        return Sides::Unknown;
    }
    let label = Message::new("toolpath-error-no-operation").arg("at", "new");
    let Ok(selection) = chipload_toolpath::select::select_contours(
        contours,
        &select.resolved(&job.drawing),
        &label,
    ) else {
        return Sides::Unknown;
    };
    // Open contours matched (centre lines on a cutting layer) are skipped by a profile and
    // don't change its side. An outline only other parts cut doesn't make a hole of what
    // lies in it: that is a part of its own, drawn there (docs/specs/parts-and-setups.md
    // §2a, the user, 2026-11-17).
    let picked: BTreeSet<usize> = selection.contours.iter().map(|c| c.id).collect();
    let passed = others_outlines(job, part, contours, &picked);
    let is_hole = |c: &Contour| depth_without(c, contours, &passed) % 2 == 1;
    let (holes, outlines): (Vec<&Contour>, Vec<&Contour>) =
        selection.contours.iter().partition(|c| is_hole(c));
    match (holes.is_empty(), outlines.is_empty()) {
        (true, true) => Sides::Unknown,
        (true, false) => Sides::One(Side::Outside),
        (false, true) => Sides::One(Side::Inside),
        (false, false) => match (shapes_selector(&holes), shapes_selector(&outlines)) {
            (Some(holes), Some(outlines)) => Sides::Split { holes, outlines },
            _ => Sides::Unknown,
        },
    }
}

/// The closed contours (ids) that only the operations of parts other than `part` cut: not
/// cut by `part`, nor in `picked`, the selection.
fn others_outlines(
    job: &Job,
    part: u64,
    contours: &[Contour],
    picked: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let label = Message::new("toolpath-error-no-operation").arg("at", "new");
    let mut mine = BTreeSet::new();
    let mut theirs = BTreeSet::new();
    for op in &job.operations {
        if !op.selects_shapes() {
            continue;
        }
        let Ok(selection) = chipload_toolpath::select::select_contours(
            contours,
            &op.select().resolved(&job.drawing),
            &label,
        ) else {
            continue;
        };
        let into = if op.part() == part {
            &mut mine
        } else {
            &mut theirs
        };
        into.extend(selection.contours.iter().filter(|c| c.closed).map(|c| c.id));
    }
    theirs
        .into_iter()
        .filter(|id| !mine.contains(id) && !picked.contains(id))
        .collect()
}

/// Contour `inner`'s nesting depth among the drawing's closed contours, those in `passed`
/// left out; its depth from the drawing when none are.
fn depth_without(inner: &Contour, contours: &[Contour], passed: &BTreeSet<usize>) -> usize {
    let depth = inner.nesting_depth.unwrap_or(0);
    if depth == 0 || passed.is_empty() {
        return depth;
    }
    let ring = |c: &Contour| chipload_core::nesting::Ring::from_points(c.points());
    let me = ring(inner);
    contours
        .iter()
        .filter(|c| c.id != inner.id && c.nesting_depth.is_some() && !passed.contains(&c.id))
        .filter(|c| chipload_core::nesting::contains(&ring(c), &me))
        .count()
}

/// A selector of these contours' shapes; None when one has a part without a shape id (a
/// traced picture's or a text's), which can't be named.
fn shapes_selector(contours: &[&Contour]) -> Option<Selector> {
    let mut shapes: Vec<u64> = Vec::new();
    let mut seen = BTreeSet::new();
    for contour in contours {
        for part in &contour.parts {
            let id = part.segment.shape?;
            if seen.insert(id) {
                shapes.push(id);
            }
        }
    }
    Some(Selector {
        shapes: Some(shapes),
        ..Selector::default()
    })
}

/// What every new operation in a request starts from: its part, tool and values.
struct Start {
    part: u64,
    tool: i64,
    diameter: f64,
    center_cutting: Option<bool>,
    tool_kind: Option<ToolType>,
    step_down: f64,
    feed: f64,
    plunge_feed: f64,
    stepover: f64,
    /// The tool's cutting data for the material, applied over the values above.
    cutting: Option<Cutting>,
    thickness: f64,
    /// How far a through cut goes below the stock: the setup's max overcut.
    below: f64,
    /// The depth of the last operation of each kind, with its `through`.
    last: Vec<(OpKind, f64, Option<f64>)>,
    /// The project's models and their depths: what a relief carves.
    models: Vec<(u64, f64)>,
    /// A new relief's stepover for the tool (docs/specs/stl-relief.md §15).
    relief_stepover: f64,
    /// A new flute's depth with the tool (docs/specs/fluting.md §8).
    flute_depth: f64,
    /// A new pattern's depth and spacing with the tool (docs/specs/fill-pattern.md §8).
    pattern: (f64, f64),
    /// A new moulding's stepover with the tool, mm (docs/specs/moulding.md §6).
    moulding_stepover: f64,
}

impl Start {
    fn new(job: &Job, index: usize, tool: i64, library: Option<&ToolLibrary>) -> Start {
        let chosen = job.tool(tool);
        let diameter = chosen.map_or(0.0, |t| t.diameter);
        // Without cutting data: the nearest operation above with the same tool (docs/specs/
        // styles.md §2.3), else values from the diameter.
        let above = job.operations[..index.min(job.operations.len())]
            .iter()
            .rev()
            .find(|e| e.tool() == tool);
        let (step_down, feed, plunge_feed) = match above {
            Some(op) => (op_step_down(op), op_feed(op), op_plunge(op)),
            None => (
                crate::DEFAULT_STEP_DOWN,
                default_feed(diameter),
                default_plunge_feed(diameter),
            ),
        };
        let stepover = job.operations[..index.min(job.operations.len())]
            .iter()
            .rev()
            .find_map(|e| match e {
                Operation::Pocket(p) if p.tool == tool => Some(p.stepover),
                _ => None,
            })
            .unwrap_or(DEFAULT_STEPOVER);
        let last = [OpKind::Pocket, OpKind::Vcarve, OpKind::Path]
            .into_iter()
            .filter_map(|kind| {
                let op = job.operations.iter().rev().find(|e| kind.is(e))?;
                Some((kind, op.depths(&job.tools).1, op.through()))
            })
            .collect();
        Start {
            part: job.active_part_id().unwrap_or(1),
            tool,
            diameter,
            center_cutting: chosen.and_then(|t| t.center_cutting),
            tool_kind: chosen.map(|t| t.kind),
            step_down,
            feed,
            plunge_feed,
            stepover,
            relief_stepover: relief_stepover(chosen),
            flute_depth: chosen.map_or(DEFAULT_FLUTE_DEPTH, flute_depth),
            pattern: chosen.map_or((DEFAULT_PATTERN_DEPTH, 2.0), pattern_start),
            moulding_stepover: moulding_stepover(chosen),
            cutting: tool_cutting(job, library, tool),
            thickness: job.stock().thickness,
            below: job.stock().max_overcut,
            last,
            models: job.drawing.models.iter().map(|m| (m.id, m.depth)).collect(),
        }
    }

    /// The depth a new pocket, V-carve or path takes: the last of its kind's.
    fn depth(&self, kind: OpKind) -> (f64, Option<f64>) {
        self.last
            .iter()
            .find(|(k, _, _)| *k == kind)
            .map_or((DEFAULT_DEPTH, None), |(_, end, through)| (*end, *through))
    }

    /// A profile on `side`; `told` when the selection told the side (outlines or holes):
    /// it then cuts through, and an outline keeps its part with tabs.
    fn profile(&self, name: String, select: &Selector, side: Side, told: bool) -> Operation {
        let through = told.then_some(self.below);
        let tabs = (side == Side::Outside && through.is_some())
            .then(|| Tabs::starting(DEFAULT_RAMP_ANGLE_RAD, self.center_cutting, self.thickness));
        Operation::Profile(ProfileOp {
            material: None,
            name,
            enabled: true,
            part: self.part,
            select: select.clone(),
            side,
            direction: Direction::Climb,
            tool: self.tool,
            start_depth: 0.0,
            end_depth: DEFAULT_DEPTH,
            through,
            clearance: 0.0,
            step_down: self.step_down,
            feed: self.feed,
            plunge_feed: self.plunge_feed,
            tabs,
            ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
            starts: Vec::new(),
            finish: None,
            skin: None,
            corners: None,
            corner_choices: Vec::new(),
        })
    }

    /// A moulding with the profile on `side`: a 6 mm cove, square ends, in levels.
    fn moulding(&self, name: String, select: &Selector, side: MouldSide) -> Operation {
        Operation::Moulding(MouldingOp {
            material: None,
            name,
            enabled: true,
            part: self.part,
            select: select.clone(),
            tool: self.tool,
            profile: MouldProfile::Cove {
                radius: DEFAULT_MOULDING_RADIUS,
            },
            side,
            ends: MouldEnds::Square,
            start_depth: 0.0,
            stepover: self.moulding_stepover,
            step_down: Some(self.step_down),
            direction: Direction::Climb,
            feed: self.feed,
            plunge_feed: self.plunge_feed,
        })
    }

    /// A chamfer on `side`, 1 mm wide in one pass; a V-bit goes down on its point.
    fn chamfer(&self, name: String, select: &Selector, side: EdgeSide) -> Operation {
        Operation::Chamfer(ChamferOp {
            material: None,
            name,
            enabled: true,
            part: self.part,
            select: select.clone(),
            tool: self.tool,
            side,
            width: DEFAULT_CHAMFER_WIDTH,
            start_depth: 0.0,
            step_down: None,
            direction: Direction::Climb,
            feed: self.feed,
            plunge_feed: self.plunge_feed,
            // A pointed V-bit goes straight down by its shape; any other tool ramps in,
            // centre-cutting or not (docs/specs/styles.md §2.4, changed 2026-11-20).
            entry: if self.tool_kind == Some(ToolType::Vbit) {
                Entry::Plunge {}
            } else {
                Entry::Ramp {
                    ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                }
            },
        })
    }

    fn other(&self, kind: OpKind, name: String, select: &Selector) -> Operation {
        let select = select.clone();
        let (end_depth, through) = self.depth(kind);
        match kind {
            OpKind::Profile => self.profile(name, &select, Side::Outside, false),
            OpKind::Chamfer => self.chamfer(name, &select, EdgeSide::Outside),
            OpKind::Moulding => self.moulding(name, &select, MouldSide::Outside),
            OpKind::Prism => Operation::Prism(PrismOp {
                material: None,
                name,
                enabled: true,
                part: self.part,
                select,
                tool: self.tool,
                start_depth: 0.0,
                max_depth: None,
                step_down: Some(self.step_down),
                direction: Direction::Climb,
                feed: self.feed,
                plunge_feed: self.plunge_feed,
            }),
            OpKind::Pattern => Operation::Pattern(PatternOp {
                material: None,
                name,
                enabled: true,
                part: self.part,
                select,
                tool: self.tool,
                pattern: FillPattern::Lines {
                    angle: DEFAULT_PATTERN_ANGLE_DEG.to_radians(),
                    spacing: self.pattern.1,
                },
                inside: true,
                start_depth: 0.0,
                end_depth: self.pattern.0,
                step_down: Some(self.step_down),
                ramp: 0.0,
                feed: self.feed,
                plunge_feed: self.plunge_feed,
            }),
            OpKind::Flute => Operation::Flute(FluteOp {
                material: None,
                name,
                enabled: true,
                part: self.part,
                select,
                tool: self.tool,
                start_depth: 0.0,
                end_depth: self.flute_depth,
                step_down: Some(self.step_down),
                ramp_start: DEFAULT_FLUTE_RAMP,
                ramp_end: DEFAULT_FLUTE_RAMP,
                ramp_shape: RampShape::Straight,
                feed: self.feed,
                plunge_feed: self.plunge_feed,
            }),
            OpKind::Pocket => Operation::Pocket(PocketOp {
                material: None,
                name,
                enabled: true,
                part: self.part,
                select,
                tool: self.tool,
                start_depth: 0.0,
                end_depth,
                through,
                clearance: 0.0,
                step_down: self.step_down,
                stepover: self.stepover,
                slot_feed: None,
                slot_step_down: None,
                direction: Direction::Climb,
                feed: self.feed,
                plunge_feed: self.plunge_feed,
                // Down round a helix, centre-cutting or not: gentler on the tool
                // (docs/specs/helix-and-leads.md decision 6, styles.md §2.4, changed
                // 2026-11-20). A plunge is chosen in the pocket's form.
                entry: Entry::default_helix(self.diameter),
                starts: Vec::new(),
                lead: None,
                corners: None,
                corner_choices: Vec::new(),
                rest: None,
                rest_passes: Vec::new(),
                finish: None,
            }),
            OpKind::Vcarve => Operation::Vcarve(VcarveOp {
                material: None,
                name,
                enabled: true,
                part: self.part,
                select,
                tool: self.tool,
                start_depth: 0.0,
                end_depth,
                passes: 1,
                step_down: None,
                ignore_kinks_deg: chipload_job::DEFAULT_IGNORE_KINKS_DEG,
                plug: None,
                feed: self.feed,
                plunge_feed: self.plunge_feed,
                floor: None,
            }),
            OpKind::Path => Operation::Path(PathOp {
                material: None,
                name,
                enabled: true,
                part: self.part,
                select,
                tool: self.tool,
                start_depth: 0.0,
                end_depth,
                through,
                step_down: self.step_down,
                feed: self.feed,
                plunge_feed: self.plunge_feed,
                // A pointed V-bit is centre-cutting by its shape (docs/specs/follow-path.md
                // §5); any other tool ramps in (ramp-entry.md decision 6, styles.md §2.4).
                entry: if self.tool_kind == Some(ToolType::Vbit) {
                    Entry::Plunge {}
                } else {
                    Entry::Ramp {
                        ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                    }
                },
            }),
            OpKind::Holes => {
                // A drill drills; anything else gets a bore, which the planner refuses for
                // a V-bit with a message.
                let method = self
                    .tool_kind
                    .and_then(|kind| HoleMethod::default_for(kind, self.diameter))
                    .unwrap_or(HoleMethod::Bore {
                        step_down: 0.5 * self.diameter,
                        ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                        direction: Direction::Climb,
                    });
                Operation::Holes(HolesOp {
                    material: None,
                    name,
                    enabled: true,
                    part: self.part,
                    select,
                    tool: self.tool,
                    start_depth: 0.0,
                    end_depth: DEFAULT_DEPTH,
                    // Holes in sheet goods mostly go through (docs/specs/styles.md §2.2).
                    through: Some(self.below),
                    feed: self.feed,
                    plunge_feed: self.plunge_feed,
                    method,
                    dowels: false,
                })
            }
            // The model whose outline is selected, else the project's first, within the
            // selection (docs/specs/stl-relief.md §4).
            OpKind::Relief => {
                let model = match select.source {
                    Some(chipload_job::Source::Model(id)) => Some(id),
                    _ => self.models.first().map(|(id, _)| *id),
                };
                let depth = model
                    .and_then(|id| self.models.iter().find(|(m, _)| *m == id))
                    .map_or(0.0, |(_, d)| *d);
                Operation::Relief(ReliefOp {
                    material: None,
                    name,
                    enabled: true,
                    part: self.part,
                    model: model.unwrap_or(0),
                    select,
                    tool: self.tool,
                    start_depth: 0.0,
                    end_depth: depth,
                    stepover: self.relief_stepover,
                    angle: 0.0,
                    zigzag: true,
                    feed: self.feed,
                    plunge_feed: self.plunge_feed,
                    roughing: None,
                    rim: Some(chipload_job::DEFAULT_RIM),
                    smoothing: None,
                })
            }
            OpKind::Surface => {
                let (lo, hi) = SURFACE_STEPOVER;
                // A part surface: the selected shapes, grown by the tool's radius
                // (docs/specs/surface-levels.md §3).
                Operation::Surface(SurfaceOp {
                    material: None,
                    name,
                    enabled: true,
                    part: self.part,
                    select,
                    margin: self.diameter / 2.0,
                    tool: self.tool,
                    start_depth: 0.0,
                    end_depth: DEFAULT_SURFACE_DEPTH,
                    step_down: self.step_down.max(DEFAULT_SURFACE_DEPTH),
                    stepover: DEFAULT_PART_SURFACE_STEPOVER.clamp(lo, hi),
                    feed: self.feed,
                    plunge_feed: self.plunge_feed,
                })
            }
        }
    }

    /// The setup's material and the tool's values for it, and a through depth worked out.
    fn finish(&self, job: &Job, op: &mut Operation) {
        op.set_material(job.stock().material.clone());
        if let Some(values) = &self.cutting {
            op.apply_cutting(values);
        }
        let point = job.drill_point(op);
        op.follow_thickness(self.thickness, point);
    }
}

fn op_step_down(op: &Operation) -> f64 {
    match op {
        Operation::Profile(p) => p.step_down,
        Operation::Pocket(p) => p.step_down,
        Operation::Path(p) => p.step_down,
        Operation::Surface(f) => f.step_down,
        Operation::Chamfer(c) => c.step_down.unwrap_or(crate::DEFAULT_STEP_DOWN),
        Operation::Flute(f) => f.step_down.unwrap_or(crate::DEFAULT_STEP_DOWN),
        Operation::Pattern(f) => f.step_down.unwrap_or(crate::DEFAULT_STEP_DOWN),
        Operation::Prism(f) => f.step_down.unwrap_or(crate::DEFAULT_STEP_DOWN),
        Operation::Moulding(m) => m.step_down.unwrap_or(crate::DEFAULT_STEP_DOWN),
        Operation::Vcarve(_) | Operation::Holes(_) | Operation::Relief(_) => {
            crate::DEFAULT_STEP_DOWN
        }
    }
}

fn op_feed(op: &Operation) -> f64 {
    match op {
        Operation::Profile(p) => p.feed,
        Operation::Pocket(p) => p.feed,
        Operation::Vcarve(v) => v.feed,
        Operation::Path(p) => p.feed,
        Operation::Chamfer(c) => c.feed,
        Operation::Flute(f) => f.feed,
        Operation::Pattern(f) => f.feed,
        Operation::Prism(f) => f.feed,
        Operation::Moulding(m) => m.feed,
        Operation::Holes(h) => h.feed,
        Operation::Surface(f) => f.feed,
        Operation::Relief(r) => r.feed,
    }
}

fn op_plunge(op: &Operation) -> f64 {
    match op {
        Operation::Profile(p) => p.plunge_feed,
        Operation::Pocket(p) => p.plunge_feed,
        Operation::Vcarve(v) => v.plunge_feed,
        Operation::Path(p) => p.plunge_feed,
        Operation::Chamfer(c) => c.plunge_feed,
        Operation::Flute(f) => f.plunge_feed,
        Operation::Pattern(f) => f.plunge_feed,
        Operation::Prism(f) => f.plunge_feed,
        Operation::Moulding(m) => m.plunge_feed,
        Operation::Holes(h) => h.plunge_feed,
        Operation::Surface(f) => f.plunge_feed,
        Operation::Relief(r) => r.plunge_feed,
    }
}

/// Insert `made` into `job` at its index, in order; the first one's index.
pub fn insert(job: &mut Job, made: NewOps) -> usize {
    for (offset, new) in made.ops.into_iter().enumerate() {
        job.operations.insert(made.index + offset, new.op);
    }
    made.index
}

#[cfg(test)]
mod tests;
