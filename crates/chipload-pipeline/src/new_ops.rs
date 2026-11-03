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
    DEFAULT_RAMP_ANGLE_RAD, Direction, Entry, HoleMethod, HolesOp, Job, Operation, PathOp,
    PocketOp, ProfileOp, SURFACE_STEPOVER, Selector, Side, SurfaceOp, Tabs, ToolType, VcarveOp,
};

use crate::chip_load::{default_feed, default_plunge_feed};
use crate::surface::{DEFAULT_SURFACE_DEPTH, DEFAULT_SURFACE_STEPOVER};

/// The depth of a new pocket, V-carve or path with nothing to take it from, and of a
/// profile of open shapes: a shallow test cut (decision 2026-10-09).
pub const DEFAULT_DEPTH: f64 = 1.0;
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
    /// Holes (docs/specs/holes.md).
    Holes,
    /// Surface: the whole stock (docs/specs/surfacing.md).
    Surface,
}

impl OpKind {
    /// Whether it cuts selected shapes; a surface faces the whole stock.
    pub fn needs_selection(self) -> bool {
        self != OpKind::Surface
    }

    /// Whether `op` is of this kind.
    pub fn is(self, op: &Operation) -> bool {
        matches!(
            (op, self),
            (Operation::Profile(_), OpKind::Profile)
                | (Operation::Pocket(_), OpKind::Pocket)
                | (Operation::Vcarve(_), OpKind::Vcarve)
                | (Operation::Path(_), OpKind::Path)
                | (Operation::Holes(_), OpKind::Holes)
                | (Operation::Surface(_), OpKind::Surface)
        )
    }

    fn name_id(self) -> &'static str {
        match self {
            OpKind::Profile => "pipeline-name-profile",
            OpKind::Pocket => "pipeline-name-pocket",
            OpKind::Vcarve => "pipeline-name-vcarve",
            OpKind::Path => "pipeline-name-path",
            OpKind::Holes => "pipeline-name-holes",
            OpKind::Surface => "pipeline-name-surface",
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
/// suits the kind, else the project's first that does. A V-carve takes a V-bit; other
/// kinds don't (user report 2026-10-14). A surface takes the widest endmill
/// (docs/specs/surfacing.md §4).
pub fn tool_for(job: &Job, kind: OpKind) -> i64 {
    if kind == OpKind::Surface
        && let Some(widest) = crate::surface::surface_tool(job)
    {
        return widest;
    }
    let suits = |id: i64| {
        job.tools
            .iter()
            .find(|t| t.id == id)
            .is_some_and(|t| (t.kind == ToolType::Vbit) == (kind == OpKind::Vcarve))
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
/// operations; a surface first in it (it faces the stock before anything is cut,
/// docs/specs/surfacing.md §6).
pub fn insert_index(job: &Job, kind: OpKind) -> usize {
    let part = job.active_part_id().unwrap_or(1);
    match kind {
        OpKind::Surface => job
            .operations
            .iter()
            .position(|e| e.part() == part)
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
            let sides = profile_sides(job, &request.select, contours);
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

fn profile_sides(job: &Job, select: &Selector, contours: &[Contour]) -> Sides {
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
    // don't change its side.
    let is_hole = |c: &Contour| c.nesting_depth.unwrap_or(0) % 2 == 1;
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
                Some((kind, op.depths().1, op.through()))
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
            cutting: tool_cutting(job, library, tool),
            thickness: job.stock().thickness,
            below: job.stock().max_overcut,
            last,
        }
    }

    /// The depth a new pocket, V-carve or path takes: the last of its kind's.
    fn depth(&self, kind: OpKind) -> (f64, Option<f64>) {
        self.last
            .iter()
            .find(|(k, _, _)| *k == kind)
            .map_or((DEFAULT_DEPTH, None), |(_, end, through)| (*end, *through))
    }

    /// Whether the tool goes straight down: only one marked centre-cutting
    /// (docs/specs/styles.md §2.4).
    fn plunges(&self) -> bool {
        self.center_cutting == Some(true)
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

    fn other(&self, kind: OpKind, name: String, select: &Selector) -> Operation {
        let select = select.clone();
        let (end_depth, through) = self.depth(kind);
        match kind {
            OpKind::Profile => self.profile(name, &select, Side::Outside, false),
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
                // A tool not known to be centre-cutting goes down round a helix
                // (docs/specs/helix-and-leads.md decision 6, styles.md §2.4).
                entry: if self.plunges() {
                    Entry::Plunge {}
                } else {
                    Entry::default_helix(self.diameter)
                },
                starts: Vec::new(),
                lead: None,
                corners: None,
                corner_choices: Vec::new(),
                rest: None,
                rest_passes: Vec::new(),
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
                // §5); another tool not known to be ramps in (ramp-entry.md decision 6).
                entry: if self.plunges() || self.tool_kind == Some(ToolType::Vbit) {
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
                })
            }
            OpKind::Surface => {
                let (lo, hi) = SURFACE_STEPOVER;
                Operation::Surface(SurfaceOp {
                    material: None,
                    name,
                    enabled: true,
                    part: self.part,
                    tool: self.tool,
                    start_depth: 0.0,
                    end_depth: DEFAULT_SURFACE_DEPTH,
                    step_down: self.step_down.max(DEFAULT_SURFACE_DEPTH),
                    stepover: DEFAULT_SURFACE_STEPOVER.clamp(lo, hi),
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
        Operation::Vcarve(_) | Operation::Holes(_) => crate::DEFAULT_STEP_DOWN,
    }
}

fn op_feed(op: &Operation) -> f64 {
    match op {
        Operation::Profile(p) => p.feed,
        Operation::Pocket(p) => p.feed,
        Operation::Vcarve(v) => v.feed,
        Operation::Path(p) => p.feed,
        Operation::Holes(h) => h.feed,
        Operation::Surface(f) => f.feed,
    }
}

fn op_plunge(op: &Operation) -> f64 {
    match op {
        Operation::Profile(p) => p.plunge_feed,
        Operation::Pocket(p) => p.plunge_feed,
        Operation::Vcarve(v) => v.plunge_feed,
        Operation::Path(p) => p.plunge_feed,
        Operation::Holes(h) => h.plunge_feed,
        Operation::Surface(f) => f.plunge_feed,
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
mod tests {
    use super::*;
    use chipload_job::library::LibraryTool;
    use chipload_job::{MaterialRef, Source};
    use std::path::PathBuf;

    /// simple_part: contour 1 the outline (layer CUT_OUTSIDE), 2-4 its holes (HOLES).
    fn part() -> (Job, Vec<Contour>) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/dxf/simple_part.dxf");
        let job = crate::new_project(chipload_dxf::shapes_from_dxf(&path, None).unwrap());
        let contours = crate::load_drawing(&job).unwrap().contours;
        (job, contours)
    }

    fn layer(name: &str) -> Selector {
        Selector::on_layer(name)
    }

    fn add(job: &Job, contours: &[Contour], kind: OpKind, select: Selector) -> NewOps {
        let request = NewRequest {
            kind,
            select,
            tool: None,
        };
        new_operations(job, &request, contours, None, Language::En)
    }

    fn profile(op: &Operation) -> &ProfileOp {
        match op {
            Operation::Profile(p) => p,
            _ => panic!("a profile"),
        }
    }

    #[test]
    fn an_outline_goes_outside_and_through_with_tabs_and_holes_inside() {
        let (job, contours) = part();
        let stock = job.stock().clone();
        let made = add(&job, &contours, OpKind::Profile, layer("CUT_OUTSIDE"));
        assert_eq!(made.ops.len(), 1);
        let p = profile(&made.ops[0].op);
        assert_eq!((p.side, made.ops[0].side_chosen), (Side::Outside, true));
        assert_eq!(p.through, Some(stock.max_overcut));
        assert_eq!(p.end_depth, stock.thickness + stock.max_overcut);
        assert_eq!(p.tabs.as_ref().map(|t| t.height), Some(3.0));
        assert_eq!(p.select, layer("CUT_OUTSIDE"), "a layer stays a layer");
        let made = add(&job, &contours, OpKind::Profile, layer("HOLES"));
        let p = profile(&made.ops[0].op);
        assert_eq!((p.side, p.tabs.is_none()), (Side::Inside, true));
        assert_eq!(p.through, Some(stock.max_overcut));
        assert!(made.messages.is_empty());
    }

    #[test]
    fn an_outline_with_its_holes_is_two_profiles_holes_first_by_shapes() {
        let (job, contours) = part();
        let made = add(
            &job,
            &contours,
            OpKind::Profile,
            Selector::of(Source::Dxf(1)),
        );
        let ops: Vec<&ProfileOp> = made.ops.iter().map(|n| profile(&n.op)).collect();
        assert_eq!(
            ops.iter().map(|p| p.side).collect::<Vec<_>>(),
            [Side::Inside, Side::Outside]
        );
        assert_eq!(
            ops.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Profile 1 (holes)", "Profile 2 (outline)"]
        );
        let shapes = |p: &ProfileOp| p.select.shapes.as_ref().map_or(0, Vec::len);
        assert!(shapes(ops[0]) > 0 && shapes(ops[1]) > 0);
        assert!(ops[0].tabs.is_none() && ops[1].tabs.is_some());
        // A whole DXF split into shapes: the report says new shapes won't be cut.
        assert_eq!(made.messages.len(), 1);
        assert!(
            made.messages[0]
                .english()
                .contains("added to the drawing later")
        );
        // Picked by shapes: nothing to say.
        let mut all = Vec::new();
        for p in &ops {
            all.extend(p.select.shapes.clone().unwrap());
        }
        let by_shapes = Selector {
            shapes: Some(all),
            ..Selector::default()
        };
        let made = add(&job, &contours, OpKind::Profile, by_shapes);
        assert_eq!(made.ops.len(), 2);
        assert!(made.messages.is_empty());
    }

    #[test]
    fn nothing_selected_leaves_the_side_to_choose() {
        let (job, contours) = part();
        let made = add(&job, &contours, OpKind::Profile, Selector::default());
        assert_eq!(made.ops.len(), 1);
        assert!(!made.ops[0].side_chosen);
        let p = profile(&made.ops[0].op);
        assert_eq!((p.through, p.end_depth), (None, DEFAULT_DEPTH));
    }

    #[test]
    fn tabs_are_at_most_half_the_thickness() {
        let (mut job, contours) = part();
        job.stock_mut().thickness = 4.0;
        let made = add(&job, &contours, OpKind::Profile, layer("CUT_OUTSIDE"));
        let p = profile(&made.ops[0].op);
        assert_eq!(p.tabs.as_ref().map(|t| t.height), Some(2.0));
    }

    #[test]
    fn depths_come_from_the_last_of_the_kind_and_holes_go_through() {
        let (mut job, contours) = part();
        let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
        assert_eq!(made.ops[0].op.depths().1, DEFAULT_DEPTH);
        insert(&mut job, made);
        job.operations[0].set_end_depth(6.0);
        let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
        assert_eq!(made.ops[0].op.depths().1, 6.0);
        assert_eq!(made.ops[0].op.name(), "Pocket 2");
        let made = add(&job, &contours, OpKind::Holes, layer("HOLES"));
        let stock = job.stock();
        assert_eq!(made.ops[0].op.through(), Some(stock.max_overcut));
        assert_eq!(
            made.ops[0].op.depths().1,
            stock.thickness + stock.max_overcut
        );
    }

    #[test]
    fn a_tool_not_known_to_be_centre_cutting_helixes_or_ramps() {
        let (mut job, contours) = part();
        let pocket = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
        let diameter = job.tools[0].diameter;
        assert_eq!(pocket.ops[0].op.entry(), Entry::default_helix(diameter));
        let path = add(&job, &contours, OpKind::Path, layer("HOLES"));
        assert!(matches!(path.ops[0].op.entry(), Entry::Ramp { .. }));
        job.tools[0].center_cutting = Some(true);
        let pocket = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
        assert_eq!(pocket.ops[0].op.entry(), Entry::Plunge {});
    }

    #[test]
    fn cutting_values_come_from_the_tool_not_from_another_tools_operation() {
        let (mut job, contours) = part();
        // An operation above with tool 1 and its own feed.
        let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
        insert(&mut job, made);
        if let Operation::Pocket(p) = &mut job.operations[0] {
            p.feed = 900.0;
        }
        // Without cutting data, the same tool takes the feed above.
        let made = add(&job, &contours, OpKind::Profile, layer("CUT_OUTSIDE"));
        assert!(matches!(&made.ops[0].op, Operation::Profile(p) if p.feed == 900.0));
        // A second tool from the library with a row for the setup's material.
        let mut tool = job.tools[0].clone();
        tool.id = 2;
        tool.library_uid = Some("c000000000000002".into());
        job.tools.push(tool.clone());
        let mut entry = LibraryTool::from_tool("6 mm", "c000000000000002", &tool, Vec::new());
        let ply = Cutting::uniform(2000.0, 600.0, 2.5, 0.45);
        entry.set_row(Some("m1"), ply);
        entry.set_row(None, Cutting::uniform(1000.0, 300.0, 1.0, 0.3));
        let mut library = ToolLibrary::default();
        library.tools.push(entry);
        job.stock_mut().material = Some(MaterialRef {
            name: "Plywood".into(),
            library_uid: "m1".into(),
        });
        let request = NewRequest {
            kind: OpKind::Pocket,
            select: layer("HOLES"),
            tool: Some(2),
        };
        let made = new_operations(&job, &request, &contours, Some(&library), Language::En);
        let op = &made.ops[0].op;
        assert!(op.has_cutting(&ply), "{op:?}");
        assert_eq!(op.material().map(|m| m.name.as_str()), Some("Plywood"));
    }

    #[test]
    fn a_helix_made_for_a_larger_tool_fits_the_smaller_one() {
        let (mut job, contours) = part();
        let made = add(&job, &contours, OpKind::Pocket, layer("HOLES"));
        insert(&mut job, made);
        assert_eq!(job.operations[0].entry(), Entry::default_helix(6.0));
        job.tools[0].diameter = 3.0;
        job.fit_helixes();
        assert_eq!(job.operations[0].entry(), Entry::default_helix(3.0));
        // One that fits stays.
        job.tools[0].diameter = 10.0;
        job.fit_helixes();
        assert_eq!(job.operations[0].entry(), Entry::default_helix(3.0));
    }
}
