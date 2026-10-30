// SPDX-License-Identifier: GPL-3.0-or-later
//! What is cut, in the order it is cut: the operations, their pockets' rest passes after
//! the run of pockets they belong to (docs/specs/rest-passes.md §3), and V-carves' floor
//! passes before the run of V-carves they belong to (docs/specs/vcarve-wide-strokes.md §4).
//! An operation keeps its number in the list; a rest pass is "N.k", k counting from 1, and
//! a floor pass "N.1".

use std::fmt;

use chipload_i18n::Message;

use crate::models::{
    Direction, Job, Operation, PocketOp, Rest, VcarveOp, operation_label, operation_label_message,
};

/// An operation, or one of its pocket's rest passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpRef {
    /// Index in `job.operations`.
    pub operation: usize,
    /// Index in the pocket's `rest_passes`, or 0 for a V-carve's floor pass; None: the
    /// operation itself.
    pub pass: Option<usize>,
}

impl OpRef {
    pub fn op(operation: usize) -> Self {
        OpRef {
            operation,
            pass: None,
        }
    }

    pub fn pass(operation: usize, pass: usize) -> Self {
        OpRef {
            operation,
            pass: Some(pass),
        }
    }
}

/// As people number them: "3", or "3.1" for its first rest pass.
impl fmt::Display for OpRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pass {
            None => write!(f, "{}", self.operation + 1),
            Some(k) => write!(f, "{}.{}", self.operation + 1, k + 1),
        }
    }
}

impl Job {
    /// What is cut, in cut order within the list (docs/specs/rest-passes.md §3): every
    /// operation that is cut, and after each run of pockets (next to each other, in one
    /// part, with one tool) the rest passes of that run, larger tools first. Part by part
    /// and copy by copy ordering is the planner's, on top of this.
    pub fn cut_list(&self) -> Vec<OpRef> {
        let mut list = Vec::new();
        let mut run: Vec<usize> = Vec::new();
        let mut carves: Vec<usize> = Vec::new();
        let cut: Vec<usize> = (0..self.operations.len())
            .filter(|&i| self.cuts(i))
            .collect();
        for (n, &i) in cut.iter().enumerate() {
            let joins = |kind: fn(&Operation) -> bool| {
                cut.get(n + 1).is_some_and(|&j| {
                    let (a, b) = (&self.operations[i], &self.operations[j]);
                    kind(a) && kind(b) && a.part() == b.part() && a.tool() == b.tool()
                })
            };
            // V-carves wait for the end of their run: its floor passes go first.
            if matches!(self.operations[i], Operation::Vcarve(_)) {
                carves.push(i);
                if !joins(|op| matches!(op, Operation::Vcarve(_))) {
                    list.extend(self.floor_passes_of(&carves));
                    list.extend(carves.drain(..).map(OpRef::op));
                }
                continue;
            }
            list.push(OpRef::op(i));
            if matches!(self.operations[i], Operation::Pocket(_)) {
                run.push(i);
            }
            if !joins(|op| matches!(op, Operation::Pocket(_))) {
                list.extend(self.rest_passes_of(&run));
                run.clear();
            }
        }
        list
    }

    /// The enabled floor passes of the V-carves in `run`, larger tools first (stable).
    fn floor_passes_of(&self, run: &[usize]) -> Vec<OpRef> {
        let mut passes: Vec<(f64, OpRef)> = Vec::new();
        for &i in run {
            if let Operation::Vcarve(VcarveOp {
                floor: Some(floor), ..
            }) = &self.operations[i]
                && floor.enabled
            {
                let diameter = self.tool(floor.tool).map_or(0.0, |t| t.diameter);
                passes.push((diameter, OpRef::pass(i, 0)));
            }
        }
        passes.sort_by(|a, b| b.0.total_cmp(&a.0));
        passes.into_iter().map(|(_, r)| r).collect()
    }

    /// The enabled rest passes of the pockets in `run`, larger tools first; a pocket's own
    /// passes keep their order (each has a smaller tool than the one before).
    fn rest_passes_of(&self, run: &[usize]) -> Vec<OpRef> {
        let mut passes: Vec<(f64, OpRef)> = Vec::new();
        for &i in run {
            let Operation::Pocket(pocket) = &self.operations[i] else {
                continue;
            };
            for (k, pass) in pocket.rest_passes.iter().enumerate() {
                if pass.enabled {
                    let diameter = self.tool(pass.tool).map_or(0.0, |t| t.diameter);
                    passes.push((diameter, OpRef::pass(i, k)));
                }
            }
        }
        // Stable: equal diameters keep the pockets' order.
        passes.sort_by(|a, b| b.0.total_cmp(&a.0));
        passes.into_iter().map(|(_, r)| r).collect()
    }

    /// The operation `r` names: the operation itself, or for a rest pass a pocket made from
    /// its pocket (shapes, depths, clearance, direction, part) and the pass (tool, feeds,
    /// step down, stepover, entry), resting after the tool before it. None if `r` names
    /// nothing.
    pub fn resolve(&self, r: OpRef) -> Option<Operation> {
        let op = self.operations.get(r.operation)?;
        let Some(k) = r.pass else {
            return Some(op.clone());
        };
        if let Operation::Vcarve(vcarve) = op {
            return self.floor_pocket(vcarve, k).map(Operation::Pocket);
        }
        let Operation::Pocket(pocket) = op else {
            return None;
        };
        let pass = pocket.rest_passes.get(k)?;
        let after_tool = match k {
            0 => pocket.tool,
            _ => pocket.rest_passes.get(k - 1)?.tool,
        };
        Some(Operation::Pocket(PocketOp {
            name: rest_pass_name(&pocket.name),
            enabled: pass.enabled,
            tool: pass.tool,
            feed: pass.feed,
            plunge_feed: pass.plunge_feed,
            step_down: pass.step_down,
            stepover: pass.stepover,
            entry: pass.entry,
            // The pocket's rings, not the pass's: starts and leads stay with it, and so do
            // dogbones (docs/specs/rest-passes.md §2).
            starts: Vec::new(),
            lead: None,
            corners: None,
            corner_choices: Vec::new(),
            rest: Some(Rest { after_tool }),
            rest_passes: Vec::new(),
            ..pocket.clone()
        }))
    }

    /// A V-carve's floor pass as a pocket (docs/specs/vcarve-wide-strokes.md §4): its
    /// shapes and depths, kept the floor inset away from the outline (a negative
    /// clearance), with the pass's tool, feeds, step down, stepover and entry. None for
    /// another pass than 0, no floor pass, or a V-bit without an angle.
    fn floor_pocket(&self, vcarve: &VcarveOp, k: usize) -> Option<PocketOp> {
        let floor = vcarve.floor.filter(|_| k == 0)?;
        let inset = vcarve.floor_inset(self.tool(vcarve.tool)?)?;
        Some(PocketOp {
            name: floor_pass_name(&vcarve.name),
            enabled: floor.enabled,
            part: vcarve.part,
            material: None,
            select: vcarve.select.clone(),
            tool: floor.tool,
            start_depth: vcarve.start_depth,
            end_depth: vcarve.end_depth,
            through: None,
            clearance: -inset,
            step_down: floor.step_down,
            stepover: floor.stepover,
            direction: Direction::Climb,
            feed: floor.feed,
            plunge_feed: floor.plunge_feed,
            entry: floor.entry,
            starts: Vec::new(),
            lead: None,
            corners: None,
            corner_choices: Vec::new(),
            rest: None,
            rest_passes: Vec::new(),
        })
    }

    /// Whether `r` is a V-carve's floor pass.
    pub fn is_floor_pass(&self, r: OpRef) -> bool {
        r.pass.is_some() && matches!(self.operations.get(r.operation), Some(Operation::Vcarve(_)))
    }

    /// "Operation 3 (Mortise)", "Operation 3.1 (Mortise, rest)" for a rest pass, or
    /// "Operation 2.1 (Sign, floor)" for a floor pass.
    pub fn label(&self, r: OpRef) -> String {
        let name = self.operations.get(r.operation).map_or("", |op| op.name());
        let kind = if self.is_floor_pass(r) {
            "floor"
        } else {
            "rest"
        };
        match r.pass {
            None => operation_label(r.operation, name),
            Some(_) => match name {
                "" => format!("Operation {r} ({kind})"),
                name => format!("Operation {r} ({name}, {kind})"),
            },
        }
    }

    /// [`Job::label`] as a message, for problems shown in the reader's language.
    pub fn label_message(&self, r: OpRef) -> Message {
        let name = self.operations.get(r.operation).map_or("", |op| op.name());
        let (bare, named) = if self.is_floor_pass(r) {
            (
                "job-operation-label-floor",
                "job-operation-label-named-floor",
            )
        } else {
            ("job-operation-label-rest", "job-operation-label-named-rest")
        };
        match (r.pass, name) {
            (None, name) => operation_label_message(r.operation, name),
            (Some(_), "") => Message::new(bare).arg("number", r.to_string()),
            (Some(_), name) => Message::new(named)
                .arg("number", r.to_string())
                .arg("name", name),
        }
    }
}

/// A rest pass's name, as the planner's resolved pocket carries it.
fn rest_pass_name(pocket: &str) -> String {
    match pocket {
        "" => "rest".into(),
        name => format!("{name}, rest"),
    }
}

/// A floor pass's name, as the planner's resolved pocket carries it.
fn floor_pass_name(vcarve: &str) -> String {
    match vcarve {
        "" => "floor".into(),
        name => format!("{name}, floor"),
    }
}
