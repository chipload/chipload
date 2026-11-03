// SPDX-License-Identifier: GPL-3.0-or-later
//! The job being edited: operations in cut order, defaults for new ones, and conversion to a
//! valid `Job`. Plain data, no GUI, so every rule here is unit tested.
//! See docs/specs/job-editor.md.

use std::collections::BTreeSet;

use chipload_core::chain::{Contour, ShapeFrom};
use chipload_i18n::Message;
use chipload_job::library::ToolLibrary;
use chipload_job::{
    Drawing, HoleMethod, Job, Operation, Problem, Selector, Side, Source, WorkZero, find_tool,
};
use chipload_pipeline::new_ops::{NewRequest, new_operations};

use crate::i18n::{Lang, t};
use crate::view::Kind;

/// Defaults for a new operation with nothing to take them from (decision 2026-10-09): a
/// shallow, slow test cut. New operations are made in the pipeline
/// (docs/specs/styles.md §2).
pub use chipload_pipeline::chip_load::{default_feed, default_plunge_feed};
pub use chipload_pipeline::new_ops::{DEFAULT_STEPOVER, OpKind};
pub const DEFAULT_STEP_DOWN: f64 = chipload_pipeline::DEFAULT_STEP_DOWN;

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
    /// All of a DXF, picture, text or group, whatever its layers, but the contours with the
    /// shapes left out with Ctrl+click (docs/specs/leave-out-shapes.md §5, shape-groups.md
    /// §5, source-model.md §2.3). An operation made from it follows the source: a DXF's
    /// updates, a picture's retracing, a text made again, a group's members.
    Whole { of: Source, left_out: BTreeSet<u64> },
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

    /// All of `of`.
    pub fn whole(of: impl Into<Source>) -> Self {
        CanvasSelection::Whole {
            of: of.into(),
            left_out: BTreeSet::new(),
        }
    }

    /// The source it is all of, if it is.
    pub fn source(&self) -> Option<Source> {
        match self {
            CanvasSelection::Whole { of, .. } => Some(*of),
            _ => None,
        }
    }

    /// The contour ids this selection covers: those with a selected shape, on the layer, or
    /// of the source. A group's shapes are read from `drawing` (none without it).
    pub fn ids(&self, contours: &[Contour], drawing: Option<&Drawing>) -> BTreeSet<usize> {
        match self {
            CanvasSelection::Nothing => BTreeSet::new(),
            CanvasSelection::Shapes(shapes) => contours
                .iter()
                .filter(|c| !shapes_of(c).is_disjoint(shapes))
                .map(|c| c.id)
                .collect(),
            CanvasSelection::Whole { of, left_out } => {
                let members: BTreeSet<u64> = match (of, drawing) {
                    (Source::Group(id), Some(drawing)) => {
                        drawing.group_shapes(*id).into_iter().collect()
                    }
                    _ => BTreeSet::new(),
                };
                let of_source = |c: &Contour| match of.shape_from() {
                    Some(from) => c.parts.iter().all(|p| p.segment.from == Some(from)),
                    None => !shapes_of(c).is_disjoint(&members),
                };
                contours
                    .iter()
                    .filter(|c| of_source(c))
                    .filter(|c| shapes_of(c).is_disjoint(left_out))
                    .map(|c| c.id)
                    .collect()
            }
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
            // An operation made from a group follows it too (docs/specs/shape-groups.md,
            // decision 8).
            CanvasSelection::Whole { of, left_out } => Selector {
                exclude: left_out.iter().copied().collect(),
                ..Selector::of(*of)
            },
            CanvasSelection::Layer(layer, from) => Selector {
                source: from.map(Source::from),
                ..Selector::on_layer(layer)
            },
            CanvasSelection::Nothing | CanvasSelection::Shapes(_) => {
                let ids = self.ids(contours, None);
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

/// The project as edited: the whole job, and which profiles still wait for their side to be
/// picked (the stored side of those is only a placeholder and is never written).
#[derive(Debug, Clone, PartialEq)]
pub struct EditJob {
    pub job: Job,
    /// Indices into `job.operations` whose side isn't chosen yet. Kept right by the methods
    /// here that add, remove or reorder operations.
    unsided: BTreeSet<usize>,
}

impl EditJob {
    /// Where work zero `zero` would be, in drawing coordinates.
    pub fn work_zero_point(&self, zero: WorkZero) -> (f64, f64) {
        Job::work_zero_on(zero, self.job.stock_bounds())
    }

    /// Where the active setup's work zero is, in drawing coordinates.
    pub fn work_zero_xy(&self) -> (f64, f64) {
        self.work_zero_point(self.job.work_zero())
    }

    /// The work zero a point dropped at `at` means ([`Job::work_zero_at`]).
    pub fn work_zero_at(&self, at: (f64, f64), reach: f64) -> WorkZero {
        Job::work_zero_at_on(self.job.stock_bounds(), at, reach)
    }

    pub fn from_job(job: &Job) -> Self {
        EditJob {
            job: job.clone(),
            unsided: BTreeSet::new(),
        }
    }

    /// Take `job`, changed by the pipeline, as the project. The marks of profiles whose side
    /// isn't chosen yet stay when the operations are as many as before: the pipeline's
    /// edits don't add, remove or reorder operations.
    pub fn adopt(&mut self, job: Job) {
        if job.operations.len() != self.job.operations.len() {
            self.unsided.clear();
        }
        self.job = job;
    }

    /// Whether operation `index`'s side is chosen (true for anything but a new profile).
    pub fn side_chosen(&self, index: usize) -> bool {
        !self.unsided.contains(&index)
    }

    /// Mark operation `index`'s side as chosen.
    pub fn choose_side(&mut self, index: usize) {
        self.unsided.remove(&index);
    }

    /// Change the list of operations with each one's side mark carried along, so adding,
    /// removing and reordering keep the marks on their operations.
    fn reorder<T>(&mut self, change: impl FnOnce(&mut Vec<(Operation, bool)>) -> T) -> T {
        let mut marked: Vec<(Operation, bool)> = std::mem::take(&mut self.job.operations)
            .into_iter()
            .enumerate()
            .map(|(i, op)| (op, self.unsided.contains(&i)))
            .collect();
        let out = change(&mut marked);
        self.unsided = marked
            .iter()
            .enumerate()
            .filter(|(_, (_, unsided))| *unsided)
            .map(|(i, _)| i)
            .collect();
        self.job.operations = marked.into_iter().map(|(op, _)| op).collect();
        out
    }

    /// Where an operation added to part `id` goes: after the part's last operation, or,
    /// for a part without operations, after the operations of the parts before it
    /// (as `Job::insert_index`).
    pub fn insert_index(&self, id: u64) -> usize {
        let order: Vec<u64> = self.job.parts.iter().map(|p| p.id).collect();
        let rank = |part: u64| order.iter().position(|p| *p == part).unwrap_or(usize::MAX);
        let mine = rank(id);
        self.job
            .operations
            .iter()
            .rposition(|e| rank(e.part()) <= mine)
            .map_or(0, |i| i + 1)
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
            .0
    }

    /// Add the operations of `kind` for the canvas selection, as the pipeline makes them
    /// (docs/specs/styles.md §2), with the tool's cutting data from `library`, named in
    /// `lang`'s language. Returns the first one's index and what the report should say.
    pub fn add_with(
        &mut self,
        kind: OpKind,
        selection: &CanvasSelection,
        contours: &[Contour],
        library: Option<&ToolLibrary>,
        lang: &impl Lang,
    ) -> (usize, Vec<Message>) {
        let request = NewRequest {
            kind,
            select: selection.selector(contours, kind == OpKind::Path),
            tool: None,
        };
        let made = new_operations(&self.job, &request, contours, library, lang.lang());
        let index = made.index;
        self.reorder(|ops| {
            for (offset, new) in made.ops.into_iter().enumerate() {
                ops.insert(index + offset, (new.op, !new.side_chosen));
            }
        });
        (index, made.messages)
    }

    /// Replace an operation's geometry with the canvas selection.
    pub fn set_geometry(
        &mut self,
        index: usize,
        selection: &CanvasSelection,
        contours: &[Contour],
    ) {
        let open = matches!(self.job.operations.get(index), Some(Operation::Path(_)));
        let select = selection.selector(contours, open);
        let unsided = !self.side_chosen(index);
        let Some(op) = self.job.operations.get_mut(index) else {
            return;
        };
        match op {
            Operation::Profile(p) => {
                p.select = select;
                if unsided
                    && let Some(side) =
                        default_side(&selection.ids(contours, Some(&self.job.drawing)), contours)
                {
                    p.side = side;
                    self.unsided.remove(&index);
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
        let len = self.job.operations.len();
        if from < len && to < len && from != to {
            let part = self.job.operations[to].part();
            self.reorder(|ops| {
                let mut moved = ops.remove(from);
                moved.0.set_part(part);
                ops.insert(to, moved);
            });
        }
    }

    /// Move operation `from` to the end of part `id`'s operations. Returns its new index.
    pub fn move_op_to_part(&mut self, from: usize, id: u64) -> Option<usize> {
        if from >= self.job.operations.len() || self.job.part(id).is_none() {
            return None;
        }
        let mut moved = self.reorder(|ops| ops.remove(from));
        moved.0.set_part(id);
        let at = self.insert_index(id);
        self.reorder(|ops| ops.insert(at, moved));
        Some(at)
    }

    /// A new, empty part at the end, which becomes the active part. Returns its id.
    pub fn add_part(&mut self) -> u64 {
        let id = self.job.parts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let name = chipload_pipeline::parts::free_name(&self.job, None).unwrap_or_default();
        self.job.parts.push(chipload_job::Part {
            id,
            name,
            enabled: true,
            copies: None,
            setup: self.job.active_setup_id().unwrap_or(1),
        });
        self.job.active_part = Some(id);
        id
    }

    /// Remove part `id` and its operations; the last part stays. Returns how many
    /// operations went with it.
    pub fn delete_part(&mut self, id: u64) -> Option<usize> {
        if self.job.parts.len() < 2 {
            return None;
        }
        let at = self.job.parts.iter().position(|p| p.id == id)?;
        self.job.parts.remove(at);
        let before = self.job.operations.len();
        self.reorder(|ops| ops.retain(|(op, _)| op.part() != id));
        if self.job.active_part == Some(id) {
            self.job.active_part = self.job.parts.first().map(|p| p.id);
        }
        Some(before - self.job.operations.len())
    }

    /// Move part `id` one place earlier (`up`) or later, with its operations.
    pub fn move_part(&mut self, id: u64, up: bool) {
        let Some(at) = self.job.parts.iter().position(|p| p.id == id) else {
            return;
        };
        let to = if up { at.checked_sub(1) } else { Some(at + 1) };
        let Some(to) = to.filter(|t| *t < self.job.parts.len()) else {
            return;
        };
        self.job.parts.swap(at, to);
        let order: Vec<u64> = self.job.parts.iter().map(|p| p.id).collect();
        self.reorder(|ops| {
            ops.sort_by_key(|(op, _)| {
                order
                    .iter()
                    .position(|p| *p == op.part())
                    .unwrap_or(usize::MAX)
            });
        });
    }

    /// Copy operation `index` to just after it, its name marked as a copy in `lang`'s
    /// language.
    pub fn duplicate(&mut self, index: usize, lang: &impl Lang) -> Option<usize> {
        let mut copy = self.job.operations.get(index)?.clone();
        let name = copy.name_mut();
        *name = t!(lang, "editor-name-copy", name = name.as_str());
        let unsided = !self.side_chosen(index);
        self.reorder(|ops| ops.insert(index + 1, (copy, unsided)));
        Some(index + 1)
    }

    /// Remove operation `index`; returns it.
    pub fn remove(&mut self, index: usize) -> Option<Operation> {
        if index < self.job.operations.len() {
            Some(self.reorder(|ops| ops.remove(index)).0)
        } else {
            None
        }
    }

    /// What an operation still lacks before it can be cut or saved, each at its field.
    pub fn missing(&self) -> Vec<Problem> {
        let mut out = Vec::new();
        for (i, op) in self.job.operations.iter().enumerate() {
            let mut lack = |field: &str, what: Message| {
                let label = Message::new("job-operation-label").arg("number", i + 1);
                let text = Message::new("job-operation-problem")
                    .arg("label", label)
                    .arg("problem", what.clone());
                out.push(Problem::new(format!("operations.{i}.{field}"), what, text));
            };
            if !self.side_chosen(i) {
                lack("side", Message::new("editor-choose-side"));
            }
            // A whole DXF or picture is geometry too (user report 2026-10-14: a V-carve of
            // a traced picture was refused as "no geometry").
            if op.selects_shapes() && op.select().is_empty() {
                lack("select", Message::new("editor-no-geometry"));
            }
        }
        out
    }

    /// The project as it stands, complete or not, its cuts through the stock following the
    /// thickness as set now.
    pub(crate) fn as_cut(&self) -> Job {
        let mut job = self.job.clone();
        job.follow_thickness();
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
            chipload_pipeline::job_problems(&self.as_cut())
                .into_iter()
                .filter(|p| !lacking.contains(&p.path)),
        );
        problems
    }

    /// The job to run or save, or every reason it isn't complete and valid.
    pub fn to_job(&self) -> Result<Job, Vec<Problem>> {
        let mut problems = self.missing();
        let job = self.as_cut();
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
    let source = select.source.map(|source| match source {
        Source::Dxf(id) => t!(lang, "editor-summary-dxf", id = id),
        Source::Image(id) => t!(lang, "editor-summary-image", id = id),
        Source::Text(id) => t!(lang, "editor-summary-text", id = id),
        Source::Group(id) => t!(lang, "editor-summary-group", id = id),
    });
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
        return match select.exclude.len() {
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
    use chipload_job::{DEFAULT_RAMP_ANGLE_RAD, Entry};
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
    fn a_side_not_chosen_stays_with_its_operation() {
        let contours = messy_2();
        let mut job = new_job();
        assert!(job.job.operations.is_empty());
        let unsided = |job: &EditJob| -> Vec<usize> {
            (0..job.job.operations.len())
                .filter(|&k| !job.side_chosen(k))
                .collect()
        };
        // Nothing selected yet: no side to tell.
        job.add(OpKind::Profile, &CanvasSelection::Nothing, &contours);
        job.add(OpKind::Pocket, &ids(&[2]), &contours);
        assert_eq!(unsided(&job), [0]);
        // [pocket, profile*]
        job.move_op(0, 1);
        assert_eq!(unsided(&job), [1]);
        // [pocket, profile*, copy*]
        assert_eq!(job.duplicate(1, &crate::i18n::Language::En), Some(2));
        assert_eq!(unsided(&job), [1, 2]);
        // [profile*, copy*]
        assert!(matches!(job.remove(0), Some(Operation::Pocket(_))));
        assert_eq!(unsided(&job), [0, 1]);
        // A new part moved ahead takes its operation, mark and all: [copy*, profile*].
        let part = job.add_part();
        assert_eq!(job.move_op_to_part(1, part), Some(1));
        job.move_part(part, true);
        assert_eq!(job.job.operations[0].part(), part);
        assert_eq!(unsided(&job), [0, 1]);
        // [profile*]
        assert_eq!(job.delete_part(part), Some(1));
        assert_eq!(unsided(&job), [0]);
        // Picking geometry that has a side chooses it.
        job.set_geometry(0, &ids(&[2]), &contours);
        assert!(unsided(&job).is_empty());
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
        let i = job.add(OpKind::Profile, &CanvasSelection::Nothing, &contours);
        assert!(!job.side_chosen(i));
        let err = job.to_job().unwrap_err();
        assert!(err[0].text.contains("choose the profile side"), "{err:?}");
    }

    #[test]
    fn an_outline_with_its_hole_gets_two_profiles_the_hole_first() {
        let contours = messy_2();
        let mut job = new_job();
        let i = job.add(OpKind::Profile, &ids(&[1, 2]), &contours);
        assert_eq!((i, job.job.operations.len()), (0, 2));
        let sides: Vec<(Side, bool, bool)> = job
            .job
            .operations
            .iter()
            .enumerate()
            .map(|(k, op)| {
                let Operation::Profile(p) = op else { panic!() };
                (p.side, p.tabs.is_some(), job.side_chosen(k))
            })
            .collect();
        assert_eq!(
            sides,
            [(Side::Inside, false, true), (Side::Outside, true, true)]
        );
        assert_eq!(job.job.operations[0].name(), "Profile 1 (holes)");
        assert_eq!(job.job.operations[1].name(), "Profile 2 (outline)");
        assert!(job.to_job().is_ok());
    }

    #[test]
    fn click_selection_makes_shapes_and_layer_selection_a_layer() {
        let contours = messy_2();
        let mut job = new_job();
        let a = job.add(OpKind::Pocket, &ids(&[4, 6, 7]), &contours); // #7 is open: left out
        let b = job.add(
            OpKind::Pocket,
            &CanvasSelection::Layer("upper".into(), None),
            &contours,
        );
        let Operation::Pocket(pocket) = &job.job.operations[a] else {
            panic!()
        };
        assert_eq!(pocket.select.shapes.as_ref().unwrap().len(), 2);
        assert_eq!(
            geometry_summary(&job.job.operations[a], &contours, &En),
            "#4, #6"
        );
        let Operation::Pocket(layered) = &job.job.operations[b] else {
            panic!()
        };
        assert_eq!(layered.select.layer.as_deref(), Some("upper"));
        assert_eq!(
            geometry_summary(&job.job.operations[b], &contours, &En),
            "layer upper"
        );
    }

    #[test]
    fn new_operations_copy_from_above_or_use_small_defaults() {
        let contours = messy_2();
        let mut job = new_job();
        let first = job.add(OpKind::Profile, &ids(&[2]), &contours);
        let stock = job.job.stock().clone();
        let Operation::Profile(p) = &mut job.job.operations[first] else {
            panic!()
        };
        // An outline goes through; the 6 mm tool: 250 mm/min per mm, a third to plunge.
        assert_eq!(
            (p.end_depth, p.through, p.step_down, p.feed, p.plunge_feed),
            (
                stock.thickness + stock.max_overcut,
                Some(stock.max_overcut),
                1.0,
                1500.0,
                500.0
            )
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
        let Operation::Pocket(q) = &job.job.operations[second] else {
            panic!()
        };
        assert_eq!(
            (q.end_depth, q.step_down, q.feed, q.stepover),
            (1.0, 3.0, 1200.0, 0.4)
        );
        assert_eq!(q.name, "Pocket 1");
    }

    #[test]
    fn new_pockets_go_down_round_a_helix_when_the_tool_cant_plunge() {
        let contours = messy_2();
        let mut job = new_job();
        job.job.tools[0].center_cutting = Some(false);
        let i = job.add(OpKind::Pocket, &ids(&[4]), &contours);
        let diameter = job.job.tools[0].diameter;
        assert_eq!(
            job.job.operations[i].entry(),
            Entry::default_helix(diameter)
        );
    }

    #[test]
    fn new_profiles_spiral_and_pockets_plunge_when_the_tool_can() {
        let contours = messy_2();
        let mut job = new_job();
        // Not known to be centre-cutting: round a helix (docs/specs/styles.md §2.4).
        let i = job.add(OpKind::Pocket, &ids(&[4]), &contours);
        let diameter = job.job.tools[0].diameter;
        assert_eq!(
            job.job.operations[i].entry(),
            Entry::default_helix(diameter)
        );
        job.job.tools[0].center_cutting = Some(true);
        let i = job.add(OpKind::Pocket, &ids(&[4]), &contours);
        assert_eq!(job.job.operations[i].entry(), Entry::Plunge {});
        let j = job.add(OpKind::Profile, &ids(&[2]), &contours);
        let Operation::Profile(p) = &job.job.operations[j] else {
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
            j.job
                .operations
                .iter()
                .map(|e| e.name().to_string())
                .collect()
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
