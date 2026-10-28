// SPDX-License-Identifier: GPL-3.0-or-later
//! The job being edited: operations in cut order, defaults for new ones, and conversion to a
//! valid `Job`. Plain data, no GUI, so every rule here is unit tested.
//! See docs/specs/job-editor.md.

use std::collections::BTreeSet;

use chipload_core::chain::{Contour, ShapeFrom};
use chipload_i18n::Message;
use chipload_job::library::Cutting;
use chipload_job::{
    DEFAULT_RAMP_ANGLE_RAD, Direction, Entry, HoleMethod, HolesOp, Job, Operation, PathOp,
    PocketOp, Problem, ProfileOp, SURFACE_STEPOVER, Selector, Side, SurfaceOp, ToolType, VcarveOp,
    find_tool,
};

use crate::i18n::{Lang, t};
use crate::view::Kind;

/// Defaults for a new operation when there's none above it to copy from (decision
/// 2026-10-09): a shallow, slow test cut.
pub use chipload_pipeline::chip_load::{default_feed, default_plunge_feed};
pub const DEFAULT_DEPTH: f64 = 1.0;
pub const DEFAULT_STEP_DOWN: f64 = chipload_pipeline::DEFAULT_STEP_DOWN;
pub const DEFAULT_STEPOVER: f64 = 0.4;

/// What is selected on the canvas. Clicked geometry is kept as shape ids, which never
/// change, so a selection survives anything that renumbers contours (undo, updating or
/// drawing); shapes that no longer exist simply drop out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CanvasSelection {
    #[default]
    Nothing,
    /// The shapes of the clicked contours.
    Shapes(BTreeSet<u64>),
    /// Every contour on one layer; with a source, only that DXF's or image's
    /// (docs/specs/drawing-sources.md §4).
    Layer(String, Option<ShapeFrom>),
    /// Every contour of one DXF or image, whatever its layers: an operation made from it
    /// follows the DXF's updates and the image's retracing.
    Source(ShapeFrom),
    /// A whole DXF or image but the contours with these shapes, left out with Ctrl+click
    /// (docs/specs/leave-out-shapes.md §5). Never empty.
    Except(ShapeFrom, BTreeSet<u64>),
}

/// The shape ids a contour is made of.
pub fn shapes_of(contour: &Contour) -> BTreeSet<u64> {
    contour
        .parts
        .iter()
        .filter_map(|p| p.segment.shape)
        .collect()
}

impl CanvasSelection {
    /// The selection made of these contours (their shapes).
    pub fn of_contours(ids: &BTreeSet<usize>, contours: &[Contour]) -> Self {
        let shapes: BTreeSet<u64> = contours
            .iter()
            .filter(|c| ids.contains(&c.id))
            .flat_map(shapes_of)
            .collect();
        if shapes.is_empty() {
            CanvasSelection::Nothing
        } else {
            CanvasSelection::Shapes(shapes)
        }
    }

    /// The contour ids this selection covers: those with a selected shape, or on the layer.
    pub fn ids(&self, contours: &[Contour]) -> BTreeSet<usize> {
        match self {
            CanvasSelection::Nothing => BTreeSet::new(),
            CanvasSelection::Shapes(shapes) => contours
                .iter()
                .filter(|c| !shapes_of(c).is_disjoint(shapes))
                .map(|c| c.id)
                .collect(),
            CanvasSelection::Source(from) => contours
                .iter()
                .filter(|c| c.parts.iter().all(|p| p.segment.from == Some(*from)))
                .map(|c| c.id)
                .collect(),
            CanvasSelection::Except(from, left_out) => contours
                .iter()
                .filter(|c| c.parts.iter().all(|p| p.segment.from == Some(*from)))
                .filter(|c| shapes_of(c).is_disjoint(left_out))
                .map(|c| c.id)
                .collect(),
            CanvasSelection::Layer(layer, from) => contours
                .iter()
                .filter(|c| c.layers().iter().any(|l| l.eq_ignore_ascii_case(layer)))
                .filter(|c| from.is_none() || c.parts.iter().all(|p| p.segment.from == *from))
                .map(|c| c.id)
                .collect(),
        }
    }

    /// A job selector for it: the shapes of the selected contours, or the layer for a layer
    /// selection. Open contours are left out unless `open` (a follow path cuts them;
    /// profiles and pockets would skip them anyway).
    pub fn selector(&self, contours: &[Contour], open: bool) -> Selector {
        match self {
            CanvasSelection::Source(from) => Selector {
                dxf: match from {
                    ShapeFrom::Dxf(id) => Some(*id),
                    _ => None,
                },
                image: match from {
                    ShapeFrom::Image(id) => Some(*id),
                    _ => None,
                },
                text: match from {
                    ShapeFrom::Text(id) => Some(*id),
                    _ => None,
                },
                ..Selector::default()
            },
            CanvasSelection::Except(from, left_out) => Selector {
                exclude: Some(left_out.iter().copied().collect()),
                ..CanvasSelection::Source(*from).selector(contours, open)
            },
            CanvasSelection::Layer(layer, from) => Selector {
                layer: Some(layer.clone()),
                dxf: match from {
                    Some(ShapeFrom::Dxf(id)) => Some(*id),
                    _ => None,
                },
                image: match from {
                    Some(ShapeFrom::Image(id)) => Some(*id),
                    _ => None,
                },
                text: match from {
                    Some(ShapeFrom::Text(id)) => Some(*id),
                    _ => None,
                },
                ..Selector::default()
            },
            _ => {
                let ids = self.ids(contours);
                let mut shapes: Vec<u64> = Vec::new();
                for c in contours
                    .iter()
                    .filter(|c| ids.contains(&c.id) && (c.closed || open))
                {
                    for id in c.parts.iter().filter_map(|p| p.segment.shape) {
                        if !shapes.contains(&id) {
                            shapes.push(id);
                        }
                    }
                }
                Selector {
                    shapes: Some(shapes),
                    ..Selector::default()
                }
            }
        }
    }
}

/// Profile side for a selection: inside for holes only, outside for outers only, None (the
/// user must choose) for a mix or nothing.
pub fn default_side(ids: &BTreeSet<usize>, contours: &[Contour]) -> Option<Side> {
    let kinds: BTreeSet<&str> = contours
        .iter()
        .filter(|c| ids.contains(&c.id) && c.closed)
        .map(|c| Kind::of(c).name())
        .collect();
    match kinds.iter().copied().collect::<Vec<_>>().as_slice() {
        ["hole"] => Some(Side::Inside),
        ["outer"] => Some(Side::Outside),
        _ => None,
    }
}

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

use chipload_pipeline::surface::{DEFAULT_SURFACE_DEPTH, DEFAULT_SURFACE_STEPOVER};

impl OpKind {
    /// Whether it cuts selected shapes; a surface faces the whole stock.
    pub fn needs_selection(self) -> bool {
        self != OpKind::Surface
    }
}

/// A Holes operation's method follows its tool (docs/specs/holes.md §7): after the tool
/// changes between a drill and an endmill, the method becomes that tool's, with its
/// defaults. A V-bit keeps the method (the planner refuses the tool).
pub fn fit_hole_method(op: &mut Operation, tools: &[chipload_job::Tool]) {
    let Operation::Holes(h) = op else {
        return;
    };
    let Some(tool) = find_tool(tools, h.tool) else {
        return;
    };
    if h.method.tool_type() != tool.kind
        && let Some(method) = HoleMethod::default_for(tool.kind, tool.diameter)
    {
        h.method = method;
    }
}

/// One operation as edited. `side_chosen` is false while a profile's side still has to be
/// picked (the stored side is then only a placeholder and is never written).
#[derive(Debug, Clone, PartialEq)]
pub struct EditOp {
    pub op: Operation,
    pub side_chosen: bool,
}

/// The job as edited: settings plus operations in cut order.
#[derive(Debug, Clone, PartialEq)]
pub struct EditJob {
    /// Everything but the operations (`settings.operations` is unused).
    pub settings: Job,
    pub ops: Vec<EditOp>,
}

impl EditJob {
    pub fn from_job(job: &Job) -> Self {
        let mut settings = job.clone();
        let ops = std::mem::take(&mut settings.operations)
            .into_iter()
            .map(|op| EditOp {
                op,
                side_chosen: true,
            })
            .collect();
        EditJob { settings, ops }
    }

    /// Where an operation added to part `id` goes: after the part's last operation, or,
    /// for a part without operations, after the operations of the parts before it
    /// (as `Job::insert_index`).
    pub fn insert_index(&self, id: u64) -> usize {
        let order: Vec<u64> = self.settings.parts.iter().map(|p| p.id).collect();
        let rank = |part: u64| order.iter().position(|p| *p == part).unwrap_or(usize::MAX);
        let mine = rank(id);
        self.ops
            .iter()
            .rposition(|e| rank(e.op.part()) <= mine)
            .map_or(0, |i| i + 1)
    }

    /// The tool a new operation gets: the last operation's, else the project's first
    /// (docs/specs/tool-per-operation.md §8).
    pub fn next_tool_id(&self) -> i64 {
        let known = |id: &i64| self.settings.tools.iter().any(|t| t.id == *id);
        self.ops
            .last()
            .map(|e| e.op.tool())
            .filter(known)
            .or(self.settings.tools.first().map(|t| t.id))
            .unwrap_or(1)
    }

    /// The tool a new operation of `kind` starts with: the last operation's tool when it
    /// suits the kind, else the project's first that does. A V-carve takes a V-bit; other
    /// kinds don't (user report 2026-10-14: a V-carve made in a project with a V-bit got
    /// the endmill and was refused).
    pub fn tool_for(&self, kind: OpKind) -> i64 {
        // A surface takes the widest endmill (docs/specs/surfacing.md §4).
        if kind == OpKind::Surface
            && let Some(widest) = chipload_pipeline::surface::surface_tool(&self.settings)
        {
            return widest;
        }
        let suits = |id: i64| {
            self.settings
                .tools
                .iter()
                .find(|t| t.id == id)
                .is_some_and(|t| (t.kind == ToolType::Vbit) == (kind == OpKind::Vcarve))
        };
        let next = self.next_tool_id();
        if suits(next) {
            return next;
        }
        self.settings
            .tools
            .iter()
            .map(|t| t.id)
            .find(|id| suits(*id))
            .unwrap_or(next)
    }

    /// Feeds, step-down and stepover from the nearest operation above `index`, else the
    /// tool's cutting values from the library, else defaults.
    fn inherited(
        &self,
        index: usize,
        cutting: Option<&Cutting>,
        diameter: f64,
    ) -> (f64, f64, f64, f64) {
        let above = self.ops[..index].iter().next_back().map(|e| match &e.op {
            Operation::Profile(p) => (p.step_down, p.feed, p.plunge_feed, None),
            Operation::Pocket(p) => (p.step_down, p.feed, p.plunge_feed, Some(p.stepover)),
            Operation::Vcarve(v) => (
                v.step_down.unwrap_or(DEFAULT_STEP_DOWN),
                v.feed,
                v.plunge_feed,
                None,
            ),
            Operation::Path(p) => (p.step_down, p.feed, p.plunge_feed, None),
            Operation::Holes(h) => (DEFAULT_STEP_DOWN, h.feed, h.plunge_feed, None),
            Operation::Surface(f) => (f.step_down, f.feed, f.plunge_feed, None),
        });
        let stepover_above = self.ops[..index].iter().rev().find_map(|e| match &e.op {
            Operation::Pocket(p) => Some(p.stepover),
            _ => None,
        });
        match above {
            Some((step_down, feed, plunge, _)) => (
                step_down,
                feed,
                plunge,
                stepover_above
                    .or(cutting.map(|c| c.stepover))
                    .unwrap_or(DEFAULT_STEPOVER),
            ),
            None => match cutting {
                Some(c) => (c.step_down, c.feed, c.plunge_feed, c.stepover),
                None => (
                    DEFAULT_STEP_DOWN,
                    default_feed(diameter),
                    default_plunge_feed(diameter),
                    DEFAULT_STEPOVER,
                ),
            },
        }
    }

    /// Add an operation for the canvas selection at the end of the list; returns its index.
    #[cfg(test)]
    pub fn add(
        &mut self,
        kind: OpKind,
        selection: &CanvasSelection,
        contours: &[Contour],
    ) -> usize {
        self.add_with(kind, selection, contours, None, &crate::i18n::Language::En)
    }

    /// As `add`, with the tool's cutting values from the library (used when there is no
    /// operation above to copy from; docs/specs/tool-library-and-machines.md §3). The new
    /// operation is named in `lang`'s language.
    pub fn add_with(
        &mut self,
        kind: OpKind,
        selection: &CanvasSelection,
        contours: &[Contour],
        cutting: Option<&Cutting>,
        lang: &impl Lang,
    ) -> usize {
        // Into the active part, after its operations (docs/specs/parts.md §8).
        let part = self.settings.active_part_id().unwrap_or(1);
        // A surface goes first in its part: it faces the stock before anything is cut
        // (docs/specs/surfacing.md §6).
        let index = match kind {
            OpKind::Surface => self
                .ops
                .iter()
                .position(|e| e.op.part() == part)
                .unwrap_or_else(|| self.insert_index(part)),
            _ => self.insert_index(part),
        };
        let tool = self.tool_for(kind);
        let chosen = self.settings.tool(tool);
        let diameter = chosen.map_or(0.0, |t| t.diameter);
        let (step_down, feed, plunge_feed, stepover) = self.inherited(index, cutting, diameter);
        let number = self
            .ops
            .iter()
            .filter(|e| {
                matches!(
                    (&e.op, kind),
                    (Operation::Profile(_), OpKind::Profile)
                        | (Operation::Pocket(_), OpKind::Pocket)
                        | (Operation::Vcarve(_), OpKind::Vcarve)
                        | (Operation::Path(_), OpKind::Path)
                        | (Operation::Holes(_), OpKind::Holes)
                        | (Operation::Surface(_), OpKind::Surface)
                )
            })
            .count()
            + 1;
        let select = selection.selector(contours, kind == OpKind::Path);
        // A tool marked not centre-cutting can't plunge: new pockets go down round a helix
        // (docs/specs/helix-and-leads.md, decision 6), new follow paths ramp in
        // (docs/specs/ramp-entry.md, decision 6). Profiles always spiral.
        let plunges = chosen.and_then(|t| t.center_cutting) != Some(false);
        let entry = match (plunges, kind) {
            (true, _) | (false, OpKind::Profile) => Entry::Plunge {},
            (false, OpKind::Pocket) => Entry::default_helix(diameter),
            // A V-bit's point goes in along the path; the entry isn't used.
            (false, OpKind::Vcarve) => Entry::Plunge {},
            // A pointed V-bit is centre-cutting by its shape (docs/specs/follow-path.md §5).
            (false, OpKind::Path) if chosen.is_some_and(|t| t.kind == ToolType::Vbit) => {
                Entry::Plunge {}
            }
            (false, OpKind::Path) => Entry::Ramp {
                ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
            },
            // Holes go down their own way, a surface beside the stock; the entry isn't used.
            (false, OpKind::Holes | OpKind::Surface) => Entry::Plunge {},
        };
        let (op, side_chosen) = match kind {
            OpKind::Profile => {
                let side = default_side(&selection.ids(contours), contours);
                (
                    Operation::Profile(ProfileOp {
                        material: None,
                        name: t!(lang, "editor-name-profile", number = number),
                        enabled: true,
                        part,
                        select,
                        side: side.unwrap_or(Side::Outside),
                        direction: Direction::Climb,
                        tool,
                        start_depth: 0.0,
                        end_depth: DEFAULT_DEPTH,
                        clearance: 0.0,
                        step_down,
                        feed,
                        plunge_feed,
                        tabs: None,
                        ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                        starts: Vec::new(),
                        finish: None,
                        skin: None,
                        corners: None,
                        corner_choices: Vec::new(),
                    }),
                    side.is_some(),
                )
            }
            OpKind::Pocket => (
                Operation::Pocket(PocketOp {
                    material: None,
                    name: t!(lang, "editor-name-pocket", number = number),
                    enabled: true,
                    part,
                    select,
                    tool,
                    start_depth: 0.0,
                    end_depth: DEFAULT_DEPTH,
                    clearance: 0.0,
                    step_down,
                    stepover,
                    direction: Direction::Climb,
                    feed,
                    plunge_feed,
                    entry,
                    starts: Vec::new(),
                    lead: None,
                    corners: None,
                    corner_choices: Vec::new(),
                    rest: None,
                    rest_passes: Vec::new(),
                }),
                true,
            ),
            OpKind::Vcarve => (
                Operation::Vcarve(VcarveOp {
                    material: None,
                    name: t!(lang, "editor-name-vcarve", number = number),
                    enabled: true,
                    part,
                    select,
                    tool,
                    start_depth: 0.0,
                    end_depth: DEFAULT_DEPTH,
                    step_down: None,
                    feed,
                    plunge_feed,
                }),
                true,
            ),
            OpKind::Path => (
                Operation::Path(PathOp {
                    material: None,
                    name: t!(lang, "editor-name-path", number = number),
                    enabled: true,
                    part,
                    select,
                    tool,
                    start_depth: 0.0,
                    end_depth: DEFAULT_DEPTH,
                    step_down,
                    feed,
                    plunge_feed,
                    entry,
                }),
                true,
            ),
            OpKind::Holes => {
                // A drill drills; anything else gets a bore, which the planner refuses for
                // a V-bit with a message.
                let method = chosen
                    .and_then(|t| HoleMethod::default_for(t.kind, t.diameter))
                    .unwrap_or(HoleMethod::Bore {
                        step_down: 0.5 * diameter,
                        ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                        direction: Direction::Climb,
                    });
                (
                    Operation::Holes(HolesOp {
                        material: None,
                        name: t!(lang, "editor-name-holes", number = number),
                        enabled: true,
                        part,
                        select,
                        tool,
                        start_depth: 0.0,
                        end_depth: DEFAULT_DEPTH,
                        feed,
                        plunge_feed,
                        method,
                    }),
                    true,
                )
            }
            OpKind::Surface => {
                let (lo, hi) = SURFACE_STEPOVER;
                (
                    Operation::Surface(SurfaceOp {
                        material: None,
                        name: t!(lang, "editor-name-surface", number = number),
                        enabled: true,
                        part,
                        tool,
                        start_depth: 0.0,
                        end_depth: DEFAULT_SURFACE_DEPTH,
                        step_down: step_down.max(DEFAULT_SURFACE_DEPTH),
                        stepover: DEFAULT_SURFACE_STEPOVER.clamp(lo, hi),
                        feed,
                        plunge_feed,
                    }),
                    true,
                )
            }
        };
        self.ops.insert(index, EditOp { op, side_chosen });
        index
    }

    /// Replace an operation's geometry with the canvas selection.
    pub fn set_geometry(
        &mut self,
        index: usize,
        selection: &CanvasSelection,
        contours: &[Contour],
    ) {
        let open = matches!(self.ops.get(index).map(|e| &e.op), Some(Operation::Path(_)));
        let select = selection.selector(contours, open);
        let Some(edit) = self.ops.get_mut(index) else {
            return;
        };
        match &mut edit.op {
            Operation::Profile(p) => {
                p.select = select;
                if !edit.side_chosen
                    && let Some(side) = default_side(&selection.ids(contours), contours)
                {
                    p.side = side;
                    edit.side_chosen = true;
                }
            }
            Operation::Pocket(p) => p.select = select,
            Operation::Vcarve(v) => v.select = select,
            Operation::Path(p) => p.select = select,
            Operation::Holes(h) => h.select = select,
            // The whole stock: no shapes to set.
            Operation::Surface(_) => {}
        }
    }

    /// Move the operation at `from` so it ends up at position `to` (cut order).
    /// Move operation `from` to `to`. It takes the part of the operation it lands on, so
    /// moving past the first or last operation of a part moves it into the next part and
    /// the list stays grouped by part (docs/specs/parts.md §8).
    pub fn move_op(&mut self, from: usize, to: usize) {
        if from < self.ops.len() && to < self.ops.len() && from != to {
            let part = self.ops[to].op.part();
            let mut op = self.ops.remove(from);
            op.op.set_part(part);
            self.ops.insert(to, op);
        }
    }

    /// Move operation `from` to the end of part `id`'s operations. Returns its new index.
    pub fn move_op_to_part(&mut self, from: usize, id: u64) -> Option<usize> {
        if from >= self.ops.len() || self.settings.part(id).is_none() {
            return None;
        }
        let mut op = self.ops.remove(from);
        op.op.set_part(id);
        let at = self.insert_index(id);
        self.ops.insert(at, op);
        Some(at)
    }

    /// A new, empty part at the end, which becomes the active part. Returns its id.
    pub fn add_part(&mut self) -> u64 {
        let id = self.settings.parts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let name = chipload_pipeline::parts::free_name(&self.settings, None).unwrap_or_default();
        self.settings.parts.push(chipload_job::Part {
            id,
            name,
            enabled: true,
            copies: None,
        });
        self.settings.active_part = Some(id);
        id
    }

    /// Remove part `id` and its operations; the last part stays. Returns how many
    /// operations went with it.
    pub fn delete_part(&mut self, id: u64) -> Option<usize> {
        if self.settings.parts.len() < 2 {
            return None;
        }
        let at = self.settings.parts.iter().position(|p| p.id == id)?;
        self.settings.parts.remove(at);
        let before = self.ops.len();
        self.ops.retain(|e| e.op.part() != id);
        if self.settings.active_part == Some(id) {
            self.settings.active_part = self.settings.parts.first().map(|p| p.id);
        }
        Some(before - self.ops.len())
    }

    /// Move part `id` one place earlier (`up`) or later, with its operations.
    pub fn move_part(&mut self, id: u64, up: bool) {
        let Some(at) = self.settings.parts.iter().position(|p| p.id == id) else {
            return;
        };
        let to = if up { at.checked_sub(1) } else { Some(at + 1) };
        let Some(to) = to.filter(|t| *t < self.settings.parts.len()) else {
            return;
        };
        self.settings.parts.swap(at, to);
        let order: Vec<u64> = self.settings.parts.iter().map(|p| p.id).collect();
        self.ops.sort_by_key(|e| {
            order
                .iter()
                .position(|p| *p == e.op.part())
                .unwrap_or(usize::MAX)
        });
    }

    /// Copy operation `index` to just after it, its name marked as a copy in `lang`'s
    /// language.
    pub fn duplicate(&mut self, index: usize, lang: &impl Lang) -> Option<usize> {
        let mut copy = self.ops.get(index)?.clone();
        let name = copy.op.name_mut();
        *name = t!(lang, "editor-name-copy", name = name.as_str());
        self.ops.insert(index + 1, copy);
        Some(index + 1)
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.ops.len() {
            self.ops.remove(index);
        }
    }

    /// What an operation still lacks before it can be cut or saved, each at its field.
    pub fn missing(&self) -> Vec<Problem> {
        let mut out = Vec::new();
        for (i, edit) in self.ops.iter().enumerate() {
            let mut lack = |field: &str, what: Message| {
                let label = Message::new("job-operation-label").arg("number", i + 1);
                let text = Message::new("job-operation-problem")
                    .arg("label", label)
                    .arg("problem", what.clone());
                out.push(Problem::new(format!("operations.{i}.{field}"), what, text));
            };
            if !edit.side_chosen {
                lack("side", Message::new("editor-choose-side"));
            }
            // A whole DXF or picture is geometry too (user report 2026-10-14: a V-carve of
            // a traced picture was refused as "no geometry").
            if edit.op.selects_shapes() && edit.op.select().is_empty() {
                lack("select", Message::new("editor-no-geometry"));
            }
        }
        out
    }

    /// The project as it stands, complete or not.
    fn job(&self) -> Job {
        let mut job = self.settings.clone();
        job.operations = self.ops.iter().map(|e| e.op.clone()).collect();
        job
    }

    /// Every problem that needs no geometry to find, each with its field
    /// (docs/specs/live-problems.md): what an operation still lacks, then the project's
    /// rules.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = self.missing();
        // The project's own rule for an empty selection says the same less helpfully.
        let lacking: Vec<String> = problems.iter().map(|p| p.path.clone()).collect();
        problems.extend(
            chipload_pipeline::job_problems(&self.job())
                .into_iter()
                .filter(|p| !lacking.contains(&p.path)),
        );
        problems
    }

    /// The job to run or save, or every reason it isn't complete and valid.
    pub fn to_job(&self) -> Result<Job, Vec<Problem>> {
        let mut problems = self.missing();
        let job = self.job();
        if problems.is_empty() {
            problems.extend(job.validate());
        }
        if problems.is_empty() {
            Ok(job)
        } else {
            Err(problems)
        }
    }
}

/// A short description of what an operation selects, e.g. "#4, #6" or "layer upper".
pub fn geometry_summary(op: &Operation, contours: &[Contour], lang: &impl Lang) -> String {
    if !op.selects_shapes() {
        return t!(lang, "editor-summary-whole-stock");
    }
    let select = op.select();
    // A text is a source like a DXF or picture: its letters are written again when it
    // changes, so operations select the text, not its shapes (docs/specs/text.md §5).
    let source = match (select.dxf, select.image, select.text) {
        (Some(dxf), _, _) => Some(t!(lang, "editor-summary-dxf", id = dxf)),
        (_, Some(image), _) => Some(t!(lang, "editor-summary-image", id = image)),
        (_, _, Some(text)) => Some(t!(lang, "editor-summary-text", id = text)),
        _ => None,
    };
    if let Some(layer) = &select.layer {
        return match source {
            Some(source) => t!(
                lang,
                "editor-summary-layer-of",
                layer = layer,
                source = source
            ),
            None => t!(lang, "editor-summary-layer", layer = layer),
        };
    }
    if let Some(source) = source
        && select.shapes.as_ref().is_none_or(Vec::is_empty)
    {
        let left_out = select.exclude.as_ref().map_or(0, Vec::len);
        return match left_out {
            0 => t!(lang, "editor-summary-all-of", source = source),
            n => t!(lang, "editor-summary-left-out", source = source, count = n),
        };
    }
    if let Some(shapes) = select.shapes.as_ref().filter(|s| !s.is_empty()) {
        let ids: Vec<String> = contours
            .iter()
            .filter(|c| {
                c.parts
                    .iter()
                    .any(|p| p.segment.shape.is_some_and(|id| shapes.contains(&id)))
            })
            .map(|c| format!("#{}", c.id))
            .collect();
        return if ids.is_empty() {
            t!(lang, "editor-summary-not-in-drawing")
        } else {
            ids.join(", ")
        };
    }
    t!(lang, "editor-summary-no-geometry")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language::En;
    use chipload_dxf::shapes_from_dxf;
    use chipload_pipeline::{load_drawing, new_project};
    use std::path::PathBuf;

    fn project() -> chipload_job::Job {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/dxf/messy_2.dxf");
        new_project(shapes_from_dxf(&path, None).unwrap())
    }

    /// messy_2's contours as a project loads them (with shape ids).
    fn messy_2() -> Vec<Contour> {
        load_drawing(&project()).unwrap().contours
    }

    fn ids(list: &[usize]) -> CanvasSelection {
        CanvasSelection::of_contours(&list.iter().copied().collect(), &messy_2())
    }

    fn new_job() -> EditJob {
        EditJob::from_job(&project())
    }

    #[test]
    fn side_defaults_follow_the_selection() {
        let contours = messy_2();
        // #2 is an outer, #1 a hole (layer 0).
        assert_eq!(default_side(&[2].into(), &contours), Some(Side::Outside));
        assert_eq!(default_side(&[1].into(), &contours), Some(Side::Inside));
        assert_eq!(default_side(&[1, 2].into(), &contours), None);
        assert_eq!(default_side(&BTreeSet::new(), &contours), None);
        let mut job = new_job();
        let i = job.add(OpKind::Profile, &ids(&[1, 2]), &contours);
        assert!(!job.ops[i].side_chosen);
        let err = job.to_job().unwrap_err();
        assert!(err[0].text.contains("choose the profile side"), "{err:?}");
    }

    #[test]
    fn click_selection_makes_shapes_and_layer_selection_a_layer() {
        let contours = messy_2();
        let mut job = new_job();
        let a = job.add(OpKind::Pocket, &ids(&[4, 6, 7]), &contours); // #7 is open: left out
        let b = job.add(
            OpKind::Profile,
            &CanvasSelection::Layer("upper".into(), None),
            &contours,
        );
        let Operation::Pocket(pocket) = &job.ops[a].op else {
            panic!()
        };
        assert_eq!(pocket.select.shapes.as_ref().unwrap().len(), 2);
        assert_eq!(geometry_summary(&job.ops[a].op, &contours, &En), "#4, #6");
        let Operation::Profile(profile) = &job.ops[b].op else {
            panic!()
        };
        assert_eq!(profile.select.layer.as_deref(), Some("upper"));
        assert_eq!(
            geometry_summary(&job.ops[b].op, &contours, &En),
            "layer upper"
        );
    }

    #[test]
    fn new_operations_copy_from_above_or_use_small_defaults() {
        let contours = messy_2();
        let mut job = new_job();
        let first = job.add(OpKind::Profile, &ids(&[2]), &contours);
        let Operation::Profile(p) = &mut job.ops[first].op else {
            panic!()
        };
        // The 6 mm tool: 250 mm/min per mm, a third of it to plunge.
        assert_eq!(
            (p.end_depth, p.step_down, p.feed, p.plunge_feed),
            (1.0, 1.0, 1500.0, 500.0)
        );
        assert_eq!(
            (default_feed(3.0), default_plunge_feed(3.0)),
            (750.0, 250.0)
        );
        assert_eq!((default_feed(20.0), default_feed(0.5)), (3000.0, 300.0));
        assert_eq!(
            default_feed(0.0),
            chipload_pipeline::chip_load::DEFAULT_FEED
        );
        assert_eq!(p.name, "Profile 1");
        p.feed = 1200.0;
        p.step_down = 3.0;
        let second = job.add(OpKind::Pocket, &ids(&[4]), &contours);
        let Operation::Pocket(q) = &job.ops[second].op else {
            panic!()
        };
        assert_eq!(
            (q.end_depth, q.step_down, q.feed, q.stepover),
            (1.0, 3.0, 1200.0, 0.4)
        );
        assert_eq!(q.name, "Pocket 1");
    }

    #[test]
    fn the_tools_cutting_values_come_before_the_defaults_but_after_the_operation_above() {
        let contours = messy_2();
        let mut job = new_job();
        let cutting = Cutting {
            feed: 1500.0,
            plunge_feed: 400.0,
            step_down: 2.5,
            stepover: 0.3,
        };
        let first = job.add_with(OpKind::Pocket, &ids(&[4]), &contours, Some(&cutting), &En);
        let Operation::Pocket(p) = &mut job.ops[first].op else {
            panic!()
        };
        assert_eq!(
            (p.step_down, p.feed, p.plunge_feed, p.stepover),
            (2.5, 1500.0, 400.0, 0.3)
        );
        // The depth is not a tool value: still the shallow test cut.
        assert_eq!(p.end_depth, DEFAULT_DEPTH);
        p.feed = 900.0;
        let second = job.add_with(OpKind::Profile, &ids(&[2]), &contours, Some(&cutting), &En);
        let Operation::Profile(q) = &job.ops[second].op else {
            panic!()
        };
        assert_eq!(q.feed, 900.0, "copied from the operation above");
    }

    #[test]
    fn new_pockets_go_down_round_a_helix_when_the_tool_cant_plunge() {
        let contours = messy_2();
        let mut job = new_job();
        job.settings.tools[0].center_cutting = Some(false);
        let i = job.add(OpKind::Pocket, &ids(&[4]), &contours);
        let diameter = job.settings.tools[0].diameter;
        assert_eq!(job.ops[i].op.entry(), Entry::default_helix(diameter));
    }

    #[test]
    fn new_profiles_spiral_and_pockets_plunge_when_the_tool_can() {
        let contours = messy_2();
        let mut job = new_job();
        let i = job.add(OpKind::Pocket, &ids(&[4]), &contours);
        assert_eq!(job.ops[i].op.entry(), Entry::Plunge {});
        let j = job.add(OpKind::Profile, &ids(&[2]), &contours);
        let Operation::Profile(p) = &job.ops[j].op else {
            panic!()
        };
        assert_eq!(p.ramp_angle, DEFAULT_RAMP_ANGLE_RAD);
    }

    #[test]
    fn empty_geometry_is_reported_until_assigned() {
        let contours = messy_2();
        let mut job = new_job();
        let i = job.add(OpKind::Pocket, &CanvasSelection::Nothing, &contours);
        assert!(job.to_job().unwrap_err()[0].text.contains("no geometry"));
        job.set_geometry(i, &ids(&[4]), &contours);
        assert!(job.to_job().is_ok());
    }

    #[test]
    fn reorder_duplicate_remove() {
        let contours = messy_2();
        let mut job = new_job();
        job.add(OpKind::Pocket, &ids(&[4]), &contours);
        job.add(OpKind::Profile, &ids(&[2]), &contours);
        job.add(OpKind::Pocket, &ids(&[6]), &contours);
        let names = |j: &EditJob| -> Vec<String> {
            j.ops.iter().map(|e| e.op.name().to_string()).collect()
        };
        job.move_op(2, 0);
        assert_eq!(names(&job), ["Pocket 2", "Pocket 1", "Profile 1"]);
        job.move_op(0, 2);
        assert_eq!(names(&job), ["Pocket 1", "Profile 1", "Pocket 2"]);
        job.move_op(0, 9); // out of range: no change
        assert_eq!(job.duplicate(1, &En), Some(2));
        assert_eq!(
            names(&job),
            ["Pocket 1", "Profile 1", "Profile 1 (copy)", "Pocket 2"]
        );
        job.remove(0);
        assert_eq!(names(&job), ["Profile 1", "Profile 1 (copy)", "Pocket 2"]);
    }

    #[test]
    fn round_trip_through_job() {
        let contours = messy_2();
        let mut job = new_job();
        job.add(OpKind::Profile, &ids(&[2]), &contours);
        let built = job.to_job().unwrap();
        assert_eq!(EditJob::from_job(&built), job);
    }
}
