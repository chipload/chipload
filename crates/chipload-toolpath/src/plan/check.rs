// SPDX-License-Identifier: GPL-3.0-or-later
//! Generation-time rules: every operation against its own tool and the stock.

use std::collections::BTreeMap;

use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    Entry, FillPattern, Job, MAX_LEAD_DIAMETERS, Note, Operation, PocketOp, Problem, ProfileOp,
    Tool, ToolType, ZZero,
};

use super::check_passes::{
    floor_pass_checks, plug_checks, rest_check, rest_pass_checks, surface_order_note,
    surface_tool_rule,
};
use super::check_tools::{
    chamfer_tool_rule, flank_notes, flute_tool_rule, holes_tool_notes, moulding_tool_rule,
    path_tool_notes, plunge_unknown, pocket_tool_notes, prism_tool_rule, tool_group_notes,
    vcarve_tool_notes,
};
use super::job_error;
use super::op_relief::relief_pass_checks;
use super::op_vcarve::vbit_cone_depth;
use crate::ToolpathError;
use crate::profile::labelled_note;

/// Generation-time rules from the specs, each operation against its own tool. Returns
/// warnings.
pub fn validate_job(job: &Job) -> Result<Vec<Note>, ToolpathError> {
    let (problems, notes) = check_job(job);
    match problems.into_iter().next() {
        Some(first) => Err(match first.operation() {
            Some(index) => job_error(first.text_message).at(index),
            None => job_error(first.text_message),
        }),
        None => Ok(notes),
    }
}

/// Every rule checked before planning that the job breaks, each with the field it is
/// about (docs/specs/live-problems.md); [`validate_job`] refuses the job for the first.
pub fn job_problems(job: &Job) -> Vec<Problem> {
    check_job(job).0
}

/// A problem of an operation: the field shows `short`, the full text starts with the
/// operation's `label` and ": ".
pub(super) fn labelled(path: String, label: &Message, short: Message) -> Problem {
    let text = Message::new("job-operation-problem")
        .arg("label", label.clone())
        .arg("problem", short.clone());
    Problem::new(path, short, text)
}

/// A length as Rust writes it with `{}`, with its unit.
pub(super) fn length(mm: f64) -> Value {
    Value::Length(mm, Digits::Shortest)
}

/// A bull nose's warnings for an operation cut with an end mill's path
/// (docs/specs/corner-radius.md §4.2): a through cut less than the corner radius below the
/// board leaves a fillet on the cut's bottom edge, and a pocket's or surface's stepover
/// wider than the flat leaves ridges on the floor. `below`: how far the cut goes below the
/// board.
fn bull_nose_notes(op: &Operation, tool: &Tool, below: f64) -> Vec<Message> {
    let Some(corner) = tool.corner_radius.filter(|_| tool.is_bull_nose()) else {
        return Vec::new();
    };
    let mut notes = Vec::new();
    if op.through().is_some() && below < corner - DEPTH_TOL_MM {
        notes.push(
            Message::new("toolpath-through-corner")
                .arg("radius", length(corner))
                .arg("below", Value::Length(below.max(0.0), Digits::Trim(3))),
        );
    }
    let stepover = match op {
        Operation::Pocket(p) => Some(p.stepover),
        Operation::Surface(f) => Some(f.stepover),
        _ => None,
    };
    let flat = tool.diameter - 2.0 * corner;
    if let Some(step) = stepover.map(|s| s * tool.diameter)
        && step > flat + DEPTH_TOL_MM
    {
        // Halfway across the band the flat misses, the corner stands this high.
        let half = ((step - flat) / 2.0).min(corner);
        let ridge = corner - (corner * corner - half * half).max(0.0).sqrt();
        notes.push(
            Message::new("toolpath-corner-ridges")
                .arg("ridge", Value::Length(ridge, Digits::Trim(3)))
                .arg("flat", Value::Length(flat, Digits::Trim(3))),
        );
    }
    notes
}

/// How far a through cut goes into the spoilboard, as a plain note (docs/specs/styles.md
/// §2.1); None for a cut that isn't through or stays in the stock.
pub(super) fn through_overcut(job: &Job, op: &Operation) -> Option<Message> {
    op.through()?;
    let stock = job.setup_of(op).map_or(job.stock(), |s| &s.stock);
    let below = op.depths(&job.tools).1 - job.drill_point(op) - stock.thickness;
    (below > DEPTH_TOL_MM).then(|| {
        Message::new("toolpath-through-spoilboard")
            .arg("below", Value::Length(below, Digits::Fixed(3)))
    })
}

/// Problems and notes, in the order the checks find them.
#[derive(Default)]
struct Found {
    problems: Vec<Problem>,
    notes: Vec<Note>,
}

impl Found {
    fn add(&mut self, (problems, notes): (Vec<Problem>, Vec<Note>)) {
        self.problems.extend(problems);
        self.notes.extend(notes);
    }
}

/// The problems, in the order the checks were always made (so the first is the error
/// `validate_job` always gave), and the notes of a job without problems.
fn check_job(job: &Job) -> (Vec<Problem>, Vec<Note>) {
    if let Some(problem) = nothing_to_cut(job) {
        return (vec![problem], Vec::new());
    }
    let mut found = Found {
        problems: tool_problems(job),
        notes: Vec::new(),
    };
    found.problems.extend(safe_z_problem(job));
    found.notes = tool_group_notes(job);
    // Operations that plunge straight down with a tool not marked centre-cutting, by
    // tool: one note per tool, after the loop.
    let mut plunging: BTreeMap<i64, Vec<(usize, &str)>> = BTreeMap::new();
    for (index, op) in enabled(job) {
        let Some(k) = tool_index(job, op.tool()) else {
            continue; // reported by `tool_problems`
        };
        let check = OpCheck::new(job, index, op, k);
        check.depths(&mut found);
        check.radii(&mut found);
        check.passes(&mut found);
        check.feeds(&mut found);
        check.entry(&mut found);
        if check.tool_rules(&mut found) {
            plunging
                .entry(check.tool.id)
                .or_default()
                .push((index, op_kind_name(op)));
        }
    }
    found.notes.extend(plunge_notes(plunging));
    (found.problems, found.notes)
}

/// The operations that are cut, with their indexes.
fn enabled(job: &Job) -> impl Iterator<Item = (usize, &Operation)> {
    job.operations
        .iter()
        .enumerate()
        .filter(|(i, _)| job.cuts(*i))
}

fn tool_index(job: &Job, id: i64) -> Option<usize> {
    job.tools.iter().position(|t| t.id == id)
}

/// No operations, or none switched on: the only problem worth showing.
fn nothing_to_cut(job: &Job) -> Option<Problem> {
    if job.operations.is_empty() {
        return Some(Problem::whole(
            "operations",
            Message::new("toolpath-no-operations"),
        ));
    }
    if !(0..job.operations.len()).any(|i| job.cuts(i)) {
        return Some(Problem::whole(
            "operations",
            Message::new("toolpath-all-disabled"),
        ));
    }
    None
}

/// Operations naming a tool the project doesn't have, and tools used without a speed (each
/// tool once).
fn tool_problems(job: &Job) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut no_rpm: Vec<i64> = Vec::new();
    for (index, op) in enabled(job) {
        let Some(k) = tool_index(job, op.tool()) else {
            problems.push(Problem::whole(
                format!("operations.{index}.tool"),
                Message::new("toolpath-tool-undefined").arg("tool", op.tool()),
            ));
            continue;
        };
        let tool = &job.tools[k];
        if tool.rpm.is_none_or(|rpm| rpm <= 0.0) && !no_rpm.contains(&tool.id) {
            no_rpm.push(tool.id);
            problems.push(Problem::new(
                format!("tools.{k}.rpm"),
                Message::new("toolpath-no-rpm-short"),
                Message::new("toolpath-no-rpm").arg("tool", tool.id),
            ));
        }
    }
    problems
}

/// Safe Z clears every setup's board (docs/specs/setups.md §5): the highest top.
fn safe_z_problem(job: &Job) -> Option<Problem> {
    let highest = job
        .setups
        .iter()
        .map(|s| &s.stock)
        .max_by(|a, b| a.top_z().total_cmp(&b.top_z()))
        .unwrap_or(job.stock());
    let top = highest.top_z();
    let z_zero = match highest.z_zero {
        ZZero::Top => "top",
        ZZero::Bottom => "bottom",
    };
    (job.machine.safe_z <= top).then(|| {
        Problem::new(
            "machine.safe_z",
            Message::new("toolpath-safe-z-short").arg("top", length(top)),
            Message::new("toolpath-safe-z")
                .arg("safe", length(job.machine.safe_z))
                .arg("top", length(top))
                .arg("zero", z_zero),
        )
    })
}

/// One note per tool of unknown centre-cutting that plunges, listing its operations.
fn plunge_notes(plunging: BTreeMap<i64, Vec<(usize, &str)>>) -> Vec<Note> {
    let mut notes = Vec::new();
    for (tool, ops) in plunging {
        let Some(&(first, _)) = ops.first() else {
            continue;
        };
        let listed: Vec<Message> = ops
            .iter()
            .map(|(index, kind)| {
                Message::new("toolpath-operation-kind")
                    .arg("number", index + 1)
                    .arg("kind", *kind)
            })
            .collect();
        let count = listed.len();
        let Some(listed) = and_list(listed) else {
            continue;
        };
        notes.push(Note::at(
            first,
            Message::new("toolpath-plunge-unknown")
                .arg("tool", tool)
                .arg("count", count)
                .arg("operations", listed),
        ));
    }
    notes
}

/// One operation checked against its tool and its board.
struct OpCheck<'a> {
    job: &'a Job,
    index: usize,
    op: &'a Operation,
    tool: &'a Tool,
    /// The tool's index in `job.tools`.
    k: usize,
    label: Message,
    /// "Operation N", for the rules that name the operation by number only.
    numbered: Message,
}

impl<'a> OpCheck<'a> {
    fn new(job: &'a Job, index: usize, op: &'a Operation, k: usize) -> Self {
        OpCheck {
            job,
            index,
            op,
            tool: &job.tools[k],
            k,
            label: op.label_message(index),
            numbered: Message::new("job-operation-label").arg("number", index + 1),
        }
    }

    fn at(&self, field: &str) -> String {
        format!("operations.{}.{field}", self.index)
    }

    /// The tool's angle for a V-bit, else the operation's tool: where a V-bit rule's
    /// problem shows.
    fn vbit_field(&self) -> String {
        if self.tool.kind == ToolType::Vbit {
            format!("tools.{}.angle", self.k)
        } else {
            self.at("tool")
        }
    }

    fn tool_radius(&self) -> f64 {
        self.tool.diameter / 2.0
    }

    fn problem(&self, found: &mut Found, field: &str, short: Message) {
        found
            .problems
            .push(labelled(self.at(field), &self.label, short));
    }

    fn note(&self, found: &mut Found, message: Message) {
        found
            .notes
            .push(Note::at(self.index, labelled_note(&self.label, message)));
    }

    /// Depths: below the board, past a V-bit's cone, a chamfer wider than the cone, and a
    /// bull nose's marks.
    fn depths(&self, found: &mut Found) {
        let below = self.below_board();
        self.board_depth(found, below);
        self.cone_depth(found);
        self.chamfer_width(found);
        for note in bull_nose_notes(self.op, self.tool, below) {
            self.note(found, note);
        }
    }

    /// How far the cut goes below its own setup's board (docs/specs/setups.md §5). A drill's
    /// point goes deeper than its full diameter: the allowance is for the diameter
    /// (docs/specs/holes.md decision 10).
    fn below_board(&self) -> f64 {
        let (_, end_depth) = self.op.depths(&self.job.tools);
        end_depth - self.job.drill_point(self.op) - self.stock().thickness
    }

    fn stock(&self) -> &'a chipload_job::Stock {
        self.job
            .setup_of(self.op)
            .map_or(self.job.stock(), |s| &s.stock)
    }

    /// Below the stock: a warning within the overcut allowance, an error beyond it
    /// (docs/specs/depths-and-clearance.md section 2).
    fn board_depth(&self, found: &mut Found, below: f64) {
        let stock = self.stock();
        // Dowel holes go deeper, for the pins (docs/specs/two-sided.md §6).
        let allowed = self.op.allowed_below(stock.max_overcut);
        // A prism's end depth is only how deep its bit could go; how deep it does go follows
        // from its shapes and is checked when it is planned (docs/specs/prism.md §4).
        if below > allowed + DEPTH_TOL_MM && !matches!(self.op, Operation::Prism(_)) {
            let (_, end_depth) = self.op.depths(&self.job.tools);
            self.problem(
                found,
                "end_depth",
                Message::new("toolpath-too-deep")
                    .arg("end", length(end_depth))
                    .arg("below", Value::Length(below, Digits::Fixed(3)))
                    .arg("thickness", length(stock.thickness))
                    .arg("allowed", length(allowed)),
            );
        } else if below > DEPTH_TOL_MM && self.op.through().is_none() {
            // A through cut goes below on purpose: a plain note when planned (`facts`).
            self.note(
                found,
                Message::new("toolpath-into-spoilboard")
                    .arg("below", Value::Length(below, Digits::Fixed(3))),
            );
        }
    }

    /// A V-bit following a path, or cutting a flute or pattern, no deeper than its cone: the
    /// shank would rub below it (docs/specs/follow-path.md §5, fluting.md §5,
    /// fill-pattern.md §6).
    fn cone_depth(&self, found: &mut Found) {
        let cut = match self.op {
            Operation::Path(p) => Some(p.end_depth - p.start_depth),
            Operation::Flute(f) => Some(f.end_depth - f.start_depth),
            Operation::Pattern(f) => Some(f.end_depth - f.start_depth),
            _ => None,
        };
        if let Some(cut) = cut
            && let Some(deepest) = vbit_cone_depth(self.tool)
            && cut > deepest + DEPTH_TOL_MM
        {
            self.problem(
                found,
                "end_depth",
                Message::new("toolpath-path-past-cone")
                    .arg("deepest", Value::Length(deepest, Digits::Trim(3)))
                    .arg("depth", length(cut)),
            );
        }
    }

    /// A chamfer no wider than the bit's cone: the shank would rub beyond it
    /// (docs/specs/chamfer.md §4).
    fn chamfer_width(&self, found: &mut Found) {
        if let Operation::Chamfer(c) = self.op
            && let Some(widest) = self.tool.widest_chamfer()
            && c.width > widest + DEPTH_TOL_MM
        {
            self.problem(
                found,
                "width",
                Message::new("toolpath-chamfer-past-cone")
                    .arg("widest", Value::Length(widest, Digits::Trim(3)))
                    .arg("tool", self.tool.id)
                    .arg("width", length(c.width)),
            );
        }
    }

    /// Lengths that must stay under the tool's radius or the cut's depth: clearance, finish
    /// stock and walls, tab height.
    fn radii(&self, found: &mut Found) {
        let radius = self.tool_radius();
        if self.op.clearance() >= radius {
            self.problem(
                found,
                "clearance",
                Message::new("toolpath-clearance-radius")
                    .arg("clearance", length(self.op.clearance()))
                    .arg("radius", length(radius)),
            );
        }
        let finish = match self.op {
            Operation::Profile(ProfileOp {
                finish: Some(f), ..
            }) => Some(("finish.stock", f.stock)),
            Operation::Pocket(PocketOp {
                finish: Some(f), ..
            }) => Some(("finish.wall", f.wall)),
            _ => None,
        };
        if let Some((field, stock)) = finish
            && stock >= radius
        {
            self.problem(
                found,
                field,
                Message::new("toolpath-finish-radius")
                    .arg("stock", length(stock))
                    .arg("radius", length(radius)),
            );
        }
        if let Operation::Profile(ProfileOp {
            tabs: Some(tabs),
            start_depth,
            end_depth,
            ..
        }) = self.op
            && tabs.height >= end_depth - start_depth
        {
            self.problem(
                found,
                "tabs.height",
                Message::new("toolpath-tab-height")
                    .arg("height", length(tabs.height))
                    .arg("cut", length(end_depth - start_depth)),
            );
        }
    }

    /// The extra passes: a rest pocket and its rest passes, a V-carve's floor pass, a
    /// relief's roughing and smoothing, a V-carve plug.
    fn passes(&self, found: &mut Found) {
        let (job, index) = (self.job, self.index);
        match self.op {
            Operation::Pocket(pocket) => {
                if let Some(rest) = &pocket.rest {
                    match rest_check(job, index, rest.after_tool, self.tool, &self.label) {
                        Ok(more) => found
                            .notes
                            .extend(more.into_iter().map(|n| Note::at(index, n))),
                        Err(short) => self.problem(found, "rest", short),
                    }
                }
                found.add(rest_pass_checks(job, index, pocket));
            }
            Operation::Vcarve(vcarve) => {
                if let Some(floor) = &vcarve.floor {
                    found.add(floor_pass_checks(job, index, floor));
                }
                if let Some(plug) = vcarve.plug {
                    found.add(plug_checks(
                        job,
                        index,
                        vcarve,
                        plug,
                        self.tool,
                        &self.label,
                    ));
                }
            }
            Operation::Relief(relief) => self.relief_passes(found, relief),
            _ => {}
        }
    }

    /// A relief's roughing and smoothing passes against their tools; without roughing, a
    /// relief deeper than the tool is wide gets a note (docs/specs/stl-relief.md §14 step 3).
    fn relief_passes(&self, found: &mut Found, relief: &chipload_job::ReliefOp) {
        let (job, index) = (self.job, self.index);
        match (&relief.roughing, &relief.smoothing) {
            (Some(rough), smooth) if rough.enabled => {
                found.add(relief_pass_checks(
                    job,
                    index,
                    (0, "roughing"),
                    (rough.tool, rough.allowance),
                ));
                if let Some(smooth) = smooth.as_ref().filter(|s| s.enabled) {
                    found.add(relief_pass_checks(
                        job,
                        index,
                        (1, "smoothing"),
                        (smooth.tool, smooth.allowance),
                    ));
                }
            }
            _ if relief.end_depth > self.tool.diameter + DEPTH_TOL_MM => self.note(
                found,
                Message::new("toolpath-relief-deep")
                    .arg("depth", length(relief.end_depth))
                    .arg("tool", self.tool.id),
            ),
            _ => {}
        }
    }

    /// Feeds no faster than the machine's fastest.
    fn feeds(&self, found: &mut Found) {
        let Some(max_feed) = self.job.machine.max_feed else {
            return;
        };
        for (name, value) in [
            ("feed", self.op.feed()),
            ("plunge_feed", self.op.plunge_feed()),
        ] {
            if value > max_feed {
                found.problems.push(labelled(
                    self.at(name),
                    &self.numbered,
                    Message::new("toolpath-max-feed")
                        .arg("field", name)
                        .arg("feed", Value::Feed(value, Digits::Shortest))
                        .arg("max", Value::Feed(max_feed, Digits::Shortest)),
                ));
            }
        }
    }

    /// The way in: a helix only where it fits and the operation can take one, a lead no
    /// longer than allowed.
    fn entry(&self, found: &mut Found) {
        if let Entry::Helix { radius, .. } = self.op.entry() {
            let short = match self.op {
                Operation::Chamfer(_) => Some(Message::new("toolpath-helix-for-chamfer")),
                Operation::Path(_) => Some(Message::new("toolpath-helix-for-path")),
                _ if radius >= self.tool_radius() => Some(
                    Message::new("toolpath-helix-radius")
                        .arg("radius", length(radius))
                        .arg("tool_radius", length(self.tool_radius())),
                ),
                _ => None,
            };
            if let Some(short) = short {
                self.problem(found, "entry", short);
            }
        }
        if let Some(lead) = self.op.lead() {
            let longest = MAX_LEAD_DIAMETERS * self.tool.diameter;
            if lead.radius > longest {
                self.problem(
                    found,
                    "lead",
                    Message::new("toolpath-lead-radius")
                        .arg("radius", length(lead.radius))
                        .arg("most", Value::Number(MAX_LEAD_DIAMETERS, Digits::Shortest))
                        .arg("longest", Value::Length(longest, Digits::Fixed(3))),
                );
            }
        }
    }

    /// The tool type's rules for this operation: a problem at its field, or any notes.
    /// Returns whether the operation plunges straight down with a tool of unknown
    /// centre-cutting.
    fn tool_rules(&self, found: &mut Found) -> bool {
        let (rule, more) = self.tool_rule();
        found
            .notes
            .extend(more.into_iter().map(|n| Note::at(self.index, n)));
        match rule {
            Ok(plunges) => plunges,
            Err((path, prefix, short)) => {
                found.problems.push(labelled(path, &prefix, short));
                false
            }
        }
    }

    /// The tool type's rule for this operation's kind, and any notes it gives.
    fn tool_rule(&self) -> (ToolRule, Vec<Message>) {
        let tool = self.tool;
        let at_tool = |e: Message| (self.at("tool"), self.numbered.clone(), e);
        let at_vbit = |e: Message| (self.vbit_field(), self.numbered.clone(), e);
        match self.op {
            Operation::Pocket(p) => {
                let field = if tool.kind == ToolType::Endmill {
                    "entry"
                } else {
                    "tool"
                };
                let rule = pocket_tool_notes(tool, p.entry)
                    .map_err(|e| (self.at(field), self.numbered.clone(), e));
                (rule, Vec::new())
            }
            // A profile spirals down; only its tabs' flanks can be a plunge.
            Operation::Profile(p) => match flank_notes(self.index + 1, tool, p) {
                Ok(found) => (Ok(false), found),
                Err(e) => (
                    Err((self.at("tabs.flank_angle"), self.numbered.clone(), e)),
                    Vec::new(),
                ),
            },
            Operation::Path(p) => (path_tool_notes(tool, p.entry).map_err(at_tool), Vec::new()),
            Operation::Chamfer(_) => (chamfer_tool_rule(tool).map_err(at_vbit), Vec::new()),
            Operation::Flute(f) => (flute_tool_rule(tool, f).map_err(at_tool), Vec::new()),
            // A ball nose or an endmill of known shape (docs/specs/moulding.md §8).
            Operation::Moulding(_) => (moulding_tool_rule(tool).map_err(at_tool), Vec::new()),
            // A V-bit with its angle (docs/specs/prism.md §4).
            Operation::Prism(_) => (prism_tool_rule(tool).map_err(at_vbit), Vec::new()),
            // Square ends and rings go straight down (docs/specs/fill-pattern.md §6).
            Operation::Pattern(f) => {
                let plunges = f.ramp <= 0.0 || matches!(f.pattern, FillPattern::Rings { .. });
                let entry = if plunges {
                    Entry::Plunge {}
                } else {
                    Entry::Ramp {
                        ramp_angle: chipload_job::DEFAULT_RAMP_ANGLE_RAD,
                    }
                };
                (path_tool_notes(tool, entry).map_err(at_tool), Vec::new())
            }
            Operation::Holes(h) => (
                holes_tool_notes(tool, h.method)
                    .map(|()| false)
                    .map_err(at_tool),
                Vec::new(),
            ),
            // A part surface clears as a pocket, down by a helix
            // (docs/specs/surface-levels.md §3).
            Operation::Surface(_) if !self.op.is_board_surface() => (
                pocket_tool_notes(tool, Entry::default_helix(tool.diameter)).map_err(at_tool),
                Vec::new(),
            ),
            // It goes down beside the stock: no plunge into material.
            Operation::Surface(_) => (
                surface_tool_rule(tool).map_err(at_tool),
                surface_order_note(self.job, self.index, &self.label)
                    .into_iter()
                    .collect(),
            ),
            Operation::Relief(_) => (relief_tool_rule(tool).map_err(at_tool), Vec::new()),
            Operation::Vcarve(_) => match vcarve_tool_notes(&self.label, tool) {
                Ok(found) => (Ok(false), found),
                Err(e) => (Err((self.vbit_field(), self.label.clone(), e)), Vec::new()),
            },
        }
    }
}

/// A tool rule's outcome: whether the operation plunges with a tool of unknown
/// centre-cutting, or the problem's (path, label, short text).
type ToolRule = Result<bool, (String, Message, Message)>;

/// The kind of an operation in a note, as `toolpath-operation-kind` selects it.
fn op_kind_name(op: &Operation) -> &'static str {
    match op {
        Operation::Profile(_) => "profile",
        Operation::Pocket(_) => "pocket",
        Operation::Vcarve(_) => "vcarve",
        Operation::Path(_) => "path",
        Operation::Chamfer(_) => "chamfer",
        Operation::Flute(_) => "flute",
        Operation::Pattern(_) => "pattern",
        Operation::Prism(_) => "prism",
        Operation::Moulding(_) => "moulding",
        Operation::Holes(_) => "holes",
        Operation::Surface(_) => "surface",
        Operation::Relief(_) => "relief",
    }
}

/// Items joined with commas: "1, 2, 3".
pub(super) fn comma_list(items: Vec<Message>) -> Option<Message> {
    let mut items = items.into_iter();
    let first = items.next()?;
    Some(items.fold(first, |list, item| {
        Message::new("toolpath-comma")
            .arg("list", list)
            .arg("item", item)
    }))
}

/// Items joined with commas and "and" before the last: "1, 2 and 3".
pub(super) fn and_list(mut items: Vec<Message>) -> Option<Message> {
    let last = items.pop()?;
    Some(match comma_list(items) {
        Some(list) => Message::new("toolpath-and")
            .arg("list", list)
            .arg("last", last),
        None => last,
    })
}

/// A relief takes a ball nose, an endmill or a V-bit whose shape is known
/// (docs/specs/stl-relief.md §8); an endmill of unknown centre-cutting plunges to each line.
fn relief_tool_rule(tool: &Tool) -> Result<bool, Message> {
    match tool.kind {
        ToolType::Drill => Err(Message::new("toolpath-relief-drill").arg("tool", tool.id)),
        _ if tool.cutter().is_none() => {
            Err(Message::new("toolpath-relief-shape").arg("tool", tool.id))
        }
        ToolType::Endmill => plunge_unknown(tool, Entry::Plunge {}),
        ToolType::Vbit | ToolType::Ballnose => Ok(false),
    }
}
