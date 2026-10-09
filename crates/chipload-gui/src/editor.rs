// SPDX-License-Identifier: GPL-3.0-or-later
//! The job being edited: operations in cut order, defaults for new ones, and conversion to a
//! valid `Job`. Plain data, no GUI, so every rule here is unit tested.
//! See docs/specs/job-editor.md.

use std::collections::BTreeSet;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_job::{
    CURRENT_VERSION, Direction, Job, Machine, Operation, PocketOp, ProfileOp, Selector, Side,
    Source, Spindle, Stock, StockOrigin, Tool, ToolType, Units, ZZero,
};

use crate::view::Kind;

/// Defaults for a new operation when there's none above it to copy from (decision
/// 2026-10-09): a shallow, slow test cut.
pub const DEFAULT_FEED: f64 = 500.0;
pub const DEFAULT_PLUNGE_FEED: f64 = 150.0;
pub const DEFAULT_DEPTH: f64 = 1.0;
pub const DEFAULT_STEP_DOWN: f64 = 1.0;
pub const DEFAULT_STEPOVER: f64 = 0.4;

/// Defaults for a new job (docs/specs/job-editor.md §6).
pub const DEFAULT_THICKNESS: f64 = 10.0;
pub const DEFAULT_SAFE_Z: f64 = 5.0;
pub const DEFAULT_TOOL_DIAMETER: f64 = 6.0;
pub const DEFAULT_RPM: f64 = 18000.0;

/// What is selected on the canvas.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CanvasSelection {
    #[default]
    Nothing,
    /// Clicked contours (ids).
    Contours(BTreeSet<usize>),
    /// Every contour on one layer.
    Layer(String),
}

impl CanvasSelection {
    /// The contour ids this selection covers.
    pub fn ids(&self, contours: &[Contour]) -> BTreeSet<usize> {
        match self {
            CanvasSelection::Nothing => BTreeSet::new(),
            CanvasSelection::Contours(ids) => ids.clone(),
            CanvasSelection::Layer(layer) => contours
                .iter()
                .filter(|c| c.layers().iter().any(|l| l.eq_ignore_ascii_case(layer)))
                .map(|c| c.id)
                .collect(),
        }
    }

    /// A job selector for it: handles for clicked contours, the layer for a layer selection.
    /// Open contours are left out, as the pipeline would skip them anyway.
    pub fn selector(&self, contours: &[Contour]) -> Selector {
        match self {
            CanvasSelection::Layer(layer) => Selector {
                layer: Some(layer.clone()),
                handles: None,
            },
            _ => {
                let ids = self.ids(contours);
                let mut handles: Vec<String> = Vec::new();
                for c in contours.iter().filter(|c| ids.contains(&c.id) && c.closed) {
                    for part in &c.parts {
                        if !handles.contains(&part.segment.handle) {
                            handles.push(part.segment.handle.clone());
                        }
                    }
                }
                Selector {
                    layer: None,
                    handles: Some(handles),
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

    /// A new job for a drawing (docs/specs/job-editor.md §6).
    pub fn for_drawing(file_name: &str, units: Units, bbox: Option<(Point, Point)>) -> Self {
        let ((x0, y0), (x1, y1)) = bbox.unwrap_or(((0.0, 0.0), (100.0, 100.0)));
        let first_quadrant = x0 >= 0.0 && y0 >= 0.0;
        let (origin, x, y) = if first_quadrant {
            (StockOrigin::BottomLeft, x1, y1)
        } else {
            (
                StockOrigin::Center,
                2.0 * x0.abs().max(x1.abs()),
                2.0 * y0.abs().max(y1.abs()),
            )
        };
        let settings = Job {
            schema_version: CURRENT_VERSION,
            written_by: None,
            source: Source {
                file: file_name.to_string(),
                sha256: None,
                units,
            },
            stock: Stock {
                x: x.max(1.0),
                y: y.max(1.0),
                thickness: DEFAULT_THICKNESS,
                origin,
                z_zero: ZZero::Top,
            },
            machine: Machine {
                post: "grbl".into(),
                safe_z: DEFAULT_SAFE_Z,
                spindle: Spindle::Cw,
                max_feed: None,
                travel: None,
            },
            tools: vec![Tool {
                id: 1,
                kind: ToolType::Endmill,
                diameter: DEFAULT_TOOL_DIAMETER,
                flutes: None,
                center_cutting: None,
                rpm: Some(DEFAULT_RPM),
            }],
            operations: Vec::new(),
        };
        EditJob {
            settings,
            ops: Vec::new(),
        }
    }

    fn tool_id(&self) -> i64 {
        self.settings.tools.first().map_or(1, |t| t.id)
    }

    /// Feeds, step-down and stepover from the nearest operation above `index`, or defaults.
    fn inherited(&self, index: usize) -> (f64, f64, f64, f64) {
        let above = self.ops[..index].iter().rev().find_map(|e| match &e.op {
            Operation::Profile(p) => Some((p.step_down, p.feed, p.plunge_feed, None)),
            Operation::Pocket(p) => Some((p.step_down, p.feed, p.plunge_feed, Some(p.stepover))),
            Operation::Drill(_) => None,
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
                stepover_above.unwrap_or(DEFAULT_STEPOVER),
            ),
            None => (
                DEFAULT_STEP_DOWN,
                DEFAULT_FEED,
                DEFAULT_PLUNGE_FEED,
                DEFAULT_STEPOVER,
            ),
        }
    }

    /// Add an operation for the canvas selection at the end of the list; returns its index.
    pub fn add(
        &mut self,
        kind: OpKind,
        selection: &CanvasSelection,
        contours: &[Contour],
    ) -> usize {
        let index = self.ops.len();
        let (step_down, feed, plunge_feed, stepover) = self.inherited(index);
        let number = self
            .ops
            .iter()
            .filter(|e| {
                matches!(
                    (&e.op, kind),
                    (Operation::Profile(_), OpKind::Profile)
                        | (Operation::Pocket(_), OpKind::Pocket)
                )
            })
            .count()
            + 1;
        let select = selection.selector(contours);
        let tool = self.tool_id();
        let (op, side_chosen) = match kind {
            OpKind::Profile => {
                let side = default_side(&selection.ids(contours), contours);
                (
                    Operation::Profile(ProfileOp {
                        name: format!("Profile {number}"),
                        enabled: true,
                        select,
                        side: side.unwrap_or(Side::Outside),
                        direction: Direction::Climb,
                        tool,
                        depth: DEFAULT_DEPTH,
                        step_down,
                        feed,
                        plunge_feed,
                        tabs: None,
                    }),
                    side.is_some(),
                )
            }
            OpKind::Pocket => (
                Operation::Pocket(PocketOp {
                    name: format!("Pocket {number}"),
                    enabled: true,
                    select,
                    tool,
                    depth: DEFAULT_DEPTH,
                    step_down,
                    stepover,
                    direction: Direction::Climb,
                    feed,
                    plunge_feed,
                    keep_tool_down: false,
                }),
                true,
            ),
        };
        self.ops.push(EditOp { op, side_chosen });
        index
    }

    /// Replace an operation's geometry with the canvas selection.
    pub fn set_geometry(
        &mut self,
        index: usize,
        selection: &CanvasSelection,
        contours: &[Contour],
    ) {
        let select = selection.selector(contours);
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
            Operation::Drill(d) => d.select = select,
        }
    }

    /// Move the operation at `from` so it ends up at position `to` (cut order).
    pub fn move_op(&mut self, from: usize, to: usize) {
        if from < self.ops.len() && to < self.ops.len() && from != to {
            let op = self.ops.remove(from);
            self.ops.insert(to, op);
        }
    }

    pub fn duplicate(&mut self, index: usize) -> Option<usize> {
        let mut copy = self.ops.get(index)?.clone();
        if let Operation::Profile(p) = &mut copy.op {
            p.name = format!("{} (copy)", p.name);
        } else if let Operation::Pocket(p) = &mut copy.op {
            p.name = format!("{} (copy)", p.name);
        }
        self.ops.insert(index + 1, copy);
        Some(index + 1)
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.ops.len() {
            self.ops.remove(index);
        }
    }

    /// Missing fields that keep the job from running or being saved, per operation index.
    pub fn missing(&self) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        for (i, edit) in self.ops.iter().enumerate() {
            if !edit.side_chosen {
                out.push((
                    i,
                    "choose the profile side (inside, outside or on)".to_string(),
                ));
            }
            let select = match &edit.op {
                Operation::Profile(p) => &p.select,
                Operation::Pocket(p) => &p.select,
                Operation::Drill(d) => &d.select,
            };
            let has_geometry =
                select.layer.is_some() || select.handles.as_ref().is_some_and(|h| !h.is_empty());
            if !has_geometry {
                out.push((
                    i,
                    "no geometry: select contours, then Use selection".to_string(),
                ));
            }
        }
        out
    }

    /// The job to run or save, or every reason it isn't complete and valid.
    pub fn to_job(&self) -> Result<Job, Vec<String>> {
        let mut problems: Vec<String> = self
            .missing()
            .into_iter()
            .map(|(i, what)| format!("Operation {}: {what}", i + 1))
            .collect();
        let mut job = self.settings.clone();
        job.operations = self.ops.iter().map(|e| e.op.clone()).collect();
        if problems.is_empty() {
            problems.extend(job.validate());
        }
        if problems.is_empty() {
            Ok(job)
        } else {
            Err(problems)
        }
    }

    /// Whether any operation selects by handle (and so depends on this exact drawing).
    pub fn uses_handles(&self) -> bool {
        self.ops.iter().any(|e| {
            let select = match &e.op {
                Operation::Profile(p) => &p.select,
                Operation::Pocket(p) => &p.select,
                Operation::Drill(d) => &d.select,
            };
            select.handles.as_ref().is_some_and(|h| !h.is_empty())
        })
    }
}

/// A short description of what an operation selects, e.g. "#4, #6" or "layer upper".
pub fn geometry_summary(op: &Operation, contours: &[Contour]) -> String {
    let select = match op {
        Operation::Profile(p) => &p.select,
        Operation::Pocket(p) => &p.select,
        Operation::Drill(d) => &d.select,
    };
    if let Some(layer) = &select.layer {
        return format!("layer {layer}");
    }
    let handles = select.handles.clone().unwrap_or_default();
    if handles.is_empty() {
        return "no geometry".into();
    }
    let ids: Vec<String> = contours
        .iter()
        .filter(|c| {
            c.parts.iter().any(|p| {
                handles
                    .iter()
                    .any(|h| h.eq_ignore_ascii_case(&p.segment.handle))
            })
        })
        .map(|c| format!("#{}", c.id))
        .collect();
    if ids.is_empty() {
        format!("handles {} (not in this drawing)", handles.join(", "))
    } else {
        ids.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_dxf::import_dxf;
    use std::path::PathBuf;

    fn messy_2() -> Vec<Contour> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/dxf/messy_2.dxf");
        import_dxf(&path, None).unwrap().contours
    }

    fn ids(list: &[usize]) -> CanvasSelection {
        CanvasSelection::Contours(list.iter().copied().collect())
    }

    fn new_job() -> EditJob {
        EditJob::for_drawing(
            "messy_2.dxf",
            Units::Mm,
            Some(((-90.5, -50.0), (90.5, 55.0))),
        )
    }

    #[test]
    fn new_job_covers_the_drawing_with_visible_defaults() {
        let job = new_job();
        assert_eq!(job.settings.stock.origin, StockOrigin::Center);
        assert_eq!((job.settings.stock.x, job.settings.stock.y), (181.0, 110.0));
        assert_eq!(job.settings.stock.thickness, DEFAULT_THICKNESS);
        let first = EditJob::for_drawing("a.dxf", Units::Mm, Some(((5.0, 5.0), (50.0, 30.0))));
        assert_eq!(first.settings.stock.origin, StockOrigin::BottomLeft);
        assert_eq!(
            (first.settings.stock.x, first.settings.stock.y),
            (50.0, 30.0)
        );
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
        assert!(err[0].contains("choose the profile side"), "{err:?}");
    }

    #[test]
    fn click_selection_makes_handles_and_layer_selection_a_layer() {
        let contours = messy_2();
        let mut job = new_job();
        let a = job.add(OpKind::Pocket, &ids(&[4, 6, 7]), &contours); // #7 is open: left out
        let b = job.add(
            OpKind::Profile,
            &CanvasSelection::Layer("upper".into()),
            &contours,
        );
        let Operation::Pocket(pocket) = &job.ops[a].op else {
            panic!()
        };
        assert_eq!(pocket.select.handles.as_ref().unwrap().len(), 2);
        assert_eq!(geometry_summary(&job.ops[a].op, &contours), "#4, #6");
        let Operation::Profile(profile) = &job.ops[b].op else {
            panic!()
        };
        assert_eq!(profile.select.layer.as_deref(), Some("upper"));
        assert_eq!(geometry_summary(&job.ops[b].op, &contours), "layer upper");
        assert!(job.uses_handles());
    }

    #[test]
    fn new_operations_copy_from_above_or_use_small_defaults() {
        let contours = messy_2();
        let mut job = new_job();
        let first = job.add(OpKind::Profile, &ids(&[2]), &contours);
        let Operation::Profile(p) = &mut job.ops[first].op else {
            panic!()
        };
        assert_eq!(
            (p.depth, p.step_down, p.feed, p.plunge_feed),
            (1.0, 1.0, 500.0, 150.0)
        );
        assert_eq!(p.name, "Profile 1");
        p.feed = 1200.0;
        p.step_down = 3.0;
        let second = job.add(OpKind::Pocket, &ids(&[4]), &contours);
        let Operation::Pocket(q) = &job.ops[second].op else {
            panic!()
        };
        assert_eq!(
            (q.depth, q.step_down, q.feed, q.stepover),
            (1.0, 3.0, 1200.0, 0.4)
        );
        assert_eq!(q.name, "Pocket 1");
    }

    #[test]
    fn empty_geometry_is_reported_until_assigned() {
        let contours = messy_2();
        let mut job = new_job();
        let i = job.add(OpKind::Pocket, &CanvasSelection::Nothing, &contours);
        assert!(job.to_job().unwrap_err()[0].contains("no geometry"));
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
        assert_eq!(job.duplicate(1), Some(2));
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
