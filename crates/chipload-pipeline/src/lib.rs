// SPDX-License-Identifier: GPL-3.0-or-later
//! End to end: job + DXF -> checked G-code. The CLI and the GUI both call this.

pub mod box_handles;
pub mod chip_load;
pub mod copies;
pub mod corners;
pub mod drawing_edit;
pub mod dxfs;
pub mod estimate;
pub mod explode;
pub mod images;
pub mod library;
pub mod materials;
pub mod notices;
pub mod parts;
pub mod placement;
pub mod posts;
pub mod recovery;
pub mod report;
pub mod setups;
pub mod shapes;
pub mod sheet;
pub mod simulate;
pub mod surface;
pub mod tabs;
pub mod texts;
pub mod units;

use std::path::{Path, PathBuf};

use chipload_check::{CheckIssue, CheckLimits, check_gcode, limits_for_tool, limits_from_job};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_dxf::{
    ImportError, ImportReport, ImportResult, ImportedShapes, contours_from_shapes, sha256_hex,
};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::library::{Cutting, LibraryTool};
use chipload_job::{
    CURRENT_VERSION, Drawing, Entry, Job, Machine, Note, Operation, PocketOp, RestRefusal, Spindle,
    Stock, StockCorner, StockOrigin, Tool, ToolChange, ToolType, Units, VcarveOp, WorkZero, ZZero,
    find_tool, rest_refusal,
};
use chipload_post::{
    FileHeader, PostConfig, PostError, ToolPart, render_program_with, render_tool_file,
    render_with_tool_changes,
};
pub use chipload_toolpath::Cancel;
use chipload_toolpath::model::Program;
use chipload_toolpath::plan::cut_extent;
use chipload_toolpath::pocket::PocketPiece;
use chipload_toolpath::{
    OpSpan, RingStart, ToolGroup, ToolpathError, plan_job_cancellable, validate_job,
};

/// Why a project couldn't be changed, planned or written. Its text is a message, shown in
/// the app's language; `Display` writes it in English for the command line.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineError {
    /// A change or run the project refuses (`pipeline.ftl`, or the problems of `job.ftl`).
    Job(Message),
    Import(ImportError),
    Toolpath(ToolpathError),
    Image(chipload_image::ImageError),
    Text(chipload_text::TextError),
    Post(PostError),
    /// The independent checker rejected the program. No G-code is returned; `program` is
    /// the rejected toolpath, for display only, and must never be posted or written out.
    /// `messages` are `errors` as messages, for the app (docs/specs/translations.md §7).
    CheckFailed {
        errors: Vec<String>,
        messages: Vec<Message>,
        program: Box<Program>,
    },
}

impl PipelineError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            PipelineError::Job(message) => message.clone(),
            PipelineError::Import(e) => e.message(),
            PipelineError::Toolpath(e) => e.message(),
            PipelineError::Image(e) => e.message(),
            PipelineError::Text(e) => e.message(),
            PipelineError::Post(e) => e.message(),
            PipelineError::CheckFailed { messages, .. } => messages.iter().fold(
                Message::new("pipeline-error-check-failed"),
                |list, finding| {
                    Message::new("pipeline-error-check-finding")
                        .arg("list", list)
                        .arg("finding", finding.clone())
                },
            ),
        }
    }
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for PipelineError {}

impl From<ImportError> for PipelineError {
    fn from(e: ImportError) -> Self {
        PipelineError::Import(e)
    }
}

impl From<ToolpathError> for PipelineError {
    fn from(e: ToolpathError) -> Self {
        PipelineError::Toolpath(e)
    }
}

impl From<chipload_image::ImageError> for PipelineError {
    fn from(e: chipload_image::ImageError) -> Self {
        PipelineError::Image(e)
    }
}

impl From<chipload_text::TextError> for PipelineError {
    fn from(e: chipload_text::TextError) -> Self {
        PipelineError::Text(e)
    }
}

impl From<PostError> for PipelineError {
    fn from(e: PostError) -> Self {
        PipelineError::Post(e)
    }
}

#[derive(Debug, Clone)]
pub struct RunResult {
    /// How long the job takes (docs/specs/cycle-time.md).
    pub estimate: estimate::Estimate,
    /// The checked G-code: one file, or one per tool group (docs/specs/tool-per-operation.md
    /// §3.1), in cut order.
    pub files: Vec<GcodeFile>,
    pub post: PostConfig,
    pub import_report: ImportReport,
    /// Everything imported, for display.
    pub contours: Vec<Contour>,
    /// The toolpath the G-code was rendered from.
    pub program: Program,
    /// Tool-path stretch of every tab, for display.
    pub tabs: Vec<Vec<Point>>,
    /// Every pocket piece (rings and links), for display.
    pub pockets: Vec<PocketPiece>,
    /// Which moves of `program` each operation made, for display.
    pub op_spans: Vec<OpSpan>,
    /// The tool groups, in cut order: one file each, or one file with M6 between them.
    pub groups: Vec<ToolGroup>,
    /// Rings whose start the user can move, for the start markers.
    pub ring_starts: Vec<RingStart>,
    /// Corners a bone can't reach, by operation, for the corner markers.
    pub unreached_corners: Vec<(usize, chipload_core::Point)>,
    /// Every profile tab's centre, for the tab markers.
    pub tab_centres: Vec<chipload_toolpath::plan::TabCentre>,
    /// V-carves whose floor pass leaves part of the floor: the operation and the largest
    /// end mill diameter that would clear it all, mm (docs/specs/vcarve-wide-strokes.md §4.1).
    pub floor_hints: Vec<(usize, f64)>,
    /// Toolpath warnings, each with the operation it is about.
    pub notes: Vec<Note>,
    pub check_warnings: Vec<CheckIssue>,
}

/// One G-code file of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct GcodeFile {
    /// Added to the output's file stem: None for a single file, `-2-T3` for the second
    /// file of a set, cut with tool 3.
    pub suffix: Option<String>,
    pub tool: i64,
    /// The operations and rest passes, in cut order.
    pub operations: Vec<chipload_job::OpRef>,
    pub gcode: String,
}

impl GcodeFile {
    /// Where this file goes when the user asked for `output`: `part.nc` gives `part.nc` or
    /// `part-2-T3.nc`.
    pub fn path(&self, output: &Path) -> PathBuf {
        let Some(suffix) = &self.suffix else {
            return output.to_path_buf();
        };
        let stem = output
            .file_stem()
            .map_or(String::new(), |s| s.to_string_lossy().into_owned());
        let name = match output.extension() {
            Some(ext) => format!("{stem}{suffix}.{}", ext.to_string_lossy()),
            None => format!("{stem}{suffix}"),
        };
        output.with_file_name(name)
    }

    /// `part-1-T1.nc (tool 1, operations 1, 3)`, for the CLI and the report.
    pub fn describe(&self, output: &Path) -> String {
        self.describe_message(output).english()
    }

    /// [`GcodeFile::describe`] as a message.
    pub fn describe_message(&self, output: &Path) -> Message {
        let path = self.path(output);
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        Message::new("pipeline-gcode-file")
            .arg("name", name)
            .arg("tool", self.tool)
            .arg(
                "operations",
                chipload_post::operations_text(&self.operations),
            )
    }
}

/// "there is no operation 7; the project has 6".
pub(crate) fn no_operation(number: usize, count: usize) -> PipelineError {
    PipelineError::Job(
        Message::new("pipeline-no-operation")
            .arg("number", number)
            .arg("count", count),
    )
}

/// Set the tool operation `number` (1-based, as listed) cuts with. The tool must be in the
/// project; feeds stay as they are (docs/specs/tool-per-operation.md §7, decision 6).
pub fn set_op_tool(job: &mut Job, number: usize, tool: i64) -> Result<(), PipelineError> {
    if job.tool(tool).is_none() {
        return Err(PipelineError::Job(
            Message::new("pipeline-no-tool-use").arg("tool", tool),
        ));
    }
    let count = job.operations.len();
    let op = number
        .checked_sub(1)
        .and_then(|i| job.operations.get_mut(i))
        .ok_or_else(|| no_operation(number, count))?;
    op.set_tool(tool);
    Ok(())
}

/// Make pocket `number` (1-based, as listed) rest machine after tool `after` (a project
/// tool), or a whole pocket again with None (docs/specs/rest-machining.md §6). Whether the
/// tool is larger is checked with the job's other rules, so the problem shows in the GUI too.
pub fn set_op_rest(job: &mut Job, number: usize, after: Option<i64>) -> Result<(), PipelineError> {
    if let Some(tool) = after
        && job.tool(tool).is_none()
    {
        return Err(PipelineError::Job(
            Message::new("pipeline-no-tool-use").arg("tool", tool),
        ));
    }
    let count = job.operations.len();
    let op = number
        .checked_sub(1)
        .and_then(|i| job.operations.get_mut(i))
        .ok_or_else(|| no_operation(number, count))?;
    let Operation::Pocket(pocket) = op else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-pocket").arg("number", number),
        ));
    };
    pocket.rest = after.map(|after_tool| chipload_job::Rest { after_tool });
    Ok(())
}

/// A rest pass for `pocket` with project tool `tool` (docs/specs/rest-passes.md §2): it
/// rests after the last tool of the pocket's chain (its own, or its last pass's). The feeds,
/// step down and stepover are `cutting`'s (the library's values for the tool) when given,
/// else the pocket's; the entry is the pocket's, made to fit the smaller tool. Refused, with
/// the reason, for dogbones and for a tool that isn't a smaller endmill.
pub fn rest_pass_of(
    pocket: &PocketOp,
    tools: &[Tool],
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::RestPass, Message> {
    if chipload_job::has_bones(pocket.corners, &pocket.corner_choices) {
        return Err(Message::new("pipeline-rest-dogbones"));
    }
    let before = pocket.rest_passes.last().map_or(pocket.tool, |p| p.tool);
    let no_tool = |id: i64| Message::new("pipeline-no-tool").arg("tool", id);
    let small = find_tool(tools, tool).ok_or_else(|| no_tool(tool))?;
    let large = find_tool(tools, before).ok_or_else(|| no_tool(before))?;
    match rest_refusal(small, large) {
        Some(RestRefusal::SmallNotEndmill) => {
            return Err(Message::new("pipeline-rest-not-endmill").arg("tool", tool));
        }
        Some(RestRefusal::LargeNotEndmill) => {
            return Err(Message::new("pipeline-rest-before-not-endmill").arg("tool", large.id));
        }
        Some(RestRefusal::NotLarger) => {
            return Err(Message::new("pipeline-rest-not-smaller")
                .arg("tool", tool)
                .arg("diameter", Value::Length(small.diameter, Digits::Shortest))
                .arg("before", large.id)
                .arg(
                    "before_diameter",
                    Value::Length(large.diameter, Digits::Shortest),
                ));
        }
        None => {}
    }
    let entry = match pocket.entry {
        Entry::Helix { .. } => Entry::default_helix(small.diameter),
        Entry::Plunge {} if small.center_cutting == Some(false) => {
            Entry::default_helix(small.diameter)
        }
        other => other,
    };
    let (feed, plunge_feed, step_down, stepover) = match cutting {
        Some(c) => (c.feed, c.plunge_feed, c.step_down, c.stepover),
        None => (
            pocket.feed,
            pocket.plunge_feed,
            pocket.step_down,
            pocket.stepover,
        ),
    };
    Ok(chipload_job::RestPass {
        tool,
        feed,
        plunge_feed,
        step_down,
        stepover,
        entry,
        enabled: true,
    })
}

/// Add a rest pass to pocket `number` (1-based) with tool `tool` (docs/specs/rest-passes.md
/// §6). Returns which pass it is.
pub fn add_rest_pass(
    job: &mut Job,
    number: usize,
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::OpRef, PipelineError> {
    let count = job.operations.len();
    let index = number
        .checked_sub(1)
        .filter(|&i| i < count)
        .ok_or_else(|| no_operation(number, count))?;
    let tools = job.tools.clone();
    let Operation::Pocket(pocket) = &mut job.operations[index] else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-pocket").arg("number", number),
        ));
    };
    let pass = rest_pass_of(pocket, &tools, tool, cutting).map_err(|e| {
        PipelineError::Job(
            Message::new("pipeline-operation-refused")
                .arg("number", number)
                .arg("reason", e),
        )
    })?;
    pocket.rest_passes.push(pass);
    Ok(chipload_job::OpRef::pass(
        index,
        pocket.rest_passes.len() - 1,
    ))
}

/// A floor pass for `vcarve` with project tool `tool` (docs/specs/vcarve-wide-strokes.md
/// §4). The feeds, step down and stepover are `cutting`'s (the library's values for the
/// tool) when given, else the V-carve's feeds, its depth in as many equal steps as it has
/// passes, and 0.45;
/// it plunges, or goes down in a helix when the end mill isn't centre-cutting. Refused, with
/// the reason, for a tool that isn't an end mill.
pub fn floor_pass_of(
    vcarve: &VcarveOp,
    tools: &[Tool],
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::FloorPass, Message> {
    let mill =
        find_tool(tools, tool).ok_or_else(|| Message::new("pipeline-no-tool").arg("tool", tool))?;
    if mill.kind != ToolType::Endmill {
        return Err(Message::new("pipeline-floor-not-endmill").arg("tool", tool));
    }
    let entry = if mill.center_cutting == Some(false) {
        Entry::default_helix(mill.diameter)
    } else {
        Entry::Plunge {}
    };
    let (feed, plunge_feed, step_down, stepover) = match cutting {
        Some(c) => (c.feed, c.plunge_feed, c.step_down, c.stepover),
        None => (
            vcarve.feed,
            vcarve.plunge_feed,
            // As many passes as the V-carve, equally deep.
            (vcarve.end_depth - vcarve.start_depth) / f64::from(vcarve.passes.max(1)),
            FLOOR_STEPOVER,
        ),
    };
    Ok(chipload_job::FloorPass {
        tool,
        feed,
        plunge_feed,
        step_down,
        stepover,
        entry,
        enabled: true,
    })
}

/// A floor pass's stepover when the library has no values for its tool.
const FLOOR_STEPOVER: f64 = 0.45;

/// Give V-carve `number` (1-based) a floor pass with tool `tool`, in place of any it had
/// (docs/specs/vcarve-wide-strokes.md §7). Returns the pass.
pub fn add_floor_pass(
    job: &mut Job,
    number: usize,
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<chipload_job::OpRef, PipelineError> {
    let count = job.operations.len();
    let index = number
        .checked_sub(1)
        .filter(|&i| i < count)
        .ok_or_else(|| no_operation(number, count))?;
    let tools = job.tools.clone();
    let Operation::Vcarve(vcarve) = &mut job.operations[index] else {
        return Err(PipelineError::Job(
            Message::new("pipeline-not-a-vcarve").arg("number", number),
        ));
    };
    let pass = floor_pass_of(vcarve, &tools, tool, cutting).map_err(|e| {
        PipelineError::Job(
            Message::new("pipeline-operation-refused")
                .arg("number", number)
                .arg("reason", e),
        )
    })?;
    vcarve.floor = Some(pass);
    Ok(chipload_job::OpRef::pass(index, 0))
}

/// The depths a V-carve's passes go to, mm down from the stock top, the last its end depth:
/// equal loads (docs/specs/v-carve.md decision 12). For the window to show under its
/// *Passes*.
pub fn vcarve_pass_depths(vcarve: &chipload_job::VcarveOp) -> Vec<f64> {
    chipload_toolpath::vcarve::carve_levels(vcarve.end_depth - vcarve.start_depth, vcarve.passes)
        .into_iter()
        .map(|level| vcarve.start_depth + level)
        .collect()
}

/// The library's end mills no wider than `largest` (a floor hint, mm), the nearest to it
/// first (docs/specs/vcarve-wide-strokes.md §4.1, decision 8).
pub fn fitting_end_mills(library: &[LibraryTool], largest: f64) -> Vec<&LibraryTool> {
    let mut fitting: Vec<&LibraryTool> = library
        .iter()
        .filter(|t| t.kind == ToolType::Endmill && t.diameter <= largest + DEPTH_TOL_MM)
        .collect();
    // Stable: equal diameters keep the library's order.
    fitting.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
    fitting
}

/// A standalone rest pocket that copies an earlier pocket of the same part: the same shapes,
/// depths, clearance and direction, resting after that pocket's tool (or its last pass's).
/// Its index, for *Make it a rest pass* (docs/specs/rest-passes.md §5).
pub fn rest_pocket_parent(job: &Job, index: usize) -> Option<usize> {
    let Some(Operation::Pocket(rest)) = job.operations.get(index) else {
        return None;
    };
    let after = rest.rest?.after_tool;
    (0..index).rev().find(|&i| {
        let Operation::Pocket(p) = &job.operations[i] else {
            return false;
        };
        let last = p.rest_passes.last().map_or(p.tool, |r| r.tool);
        p.rest.is_none()
            && p.part == rest.part
            && p.select == rest.select
            && p.start_depth == rest.start_depth
            && p.end_depth == rest.end_depth
            && p.clearance == rest.clearance
            && p.direction == rest.direction
            && !chipload_job::has_bones(p.corners, &p.corner_choices)
            && last == after
    })
}

/// *Make it a rest pass*: the rest pocket at `index` becomes the last rest pass of the pocket
/// it copies, keeping its tool, feeds, step down, stepover, entry and on/off. Returns the
/// pass. None when it copies no earlier pocket.
pub fn make_rest_pass(job: &mut Job, index: usize) -> Option<chipload_job::OpRef> {
    let parent = rest_pocket_parent(job, index)?;
    let Operation::Pocket(rest) = job.operations.remove(index) else {
        return None;
    };
    let Operation::Pocket(pocket) = job.operations.get_mut(parent)? else {
        return None;
    };
    pocket.rest_passes.push(chipload_job::RestPass {
        tool: rest.tool,
        feed: rest.feed,
        plunge_feed: rest.plunge_feed,
        step_down: rest.step_down,
        stepover: rest.stepover,
        entry: rest.entry,
        enabled: rest.enabled,
    });
    Some(chipload_job::OpRef::pass(
        parent,
        pocket.rest_passes.len() - 1,
    ))
}

/// Write every file of a run, each atomically. Returns the paths written.
pub fn write_gcode_files(output: &Path, files: &[GcodeFile]) -> std::io::Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for file in files {
        let path = file.path(output);
        write_atomically(&path, &file.gcode)?;
        written.push(path);
    }
    Ok(written)
}

/// Problems as one line: "path: message; path: message".
pub fn joined(problems: &[chipload_job::Problem]) -> String {
    problems
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Words of the command line or another library, as a message that shows them as they are.
pub fn foreign_text(text: impl Into<String>) -> Message {
    Message::new("pipeline-text").arg("text", text.into())
}

/// A change refused for its problems, as [`joined`] writes them; Ok without problems.
pub fn refuse_problems(problems: &[chipload_job::Problem]) -> Result<(), PipelineError> {
    let messages = problems.iter().map(|p| p.text_message.clone());
    match chipload_job::problem::semicolons(messages) {
        None => Ok(()),
        Some(message) => Err(PipelineError::Job(message)),
    }
}

/// Every problem that keeps a project from being planned and needs no geometry to find:
/// the file's value rules, then the job rules checked before planning, each with its field
/// (docs/specs/live-problems.md).
pub fn job_problems(job: &Job) -> Vec<chipload_job::Problem> {
    let mut problems = job.validate();
    problems.extend(chipload_toolpath::job_problems(job));
    problems.extend(units_problem(job));
    problems
}

/// The machine's G-code units, if its post can't write them (docs/specs/inch-units.md
/// §4.2). A post that can't be read is reported when the project runs.
fn units_problem(job: &Job) -> Option<chipload_job::Problem> {
    let post = posts::job_post(job).ok()?;
    let units = job.machine.gcode_units;
    (!post.units.contains(&units)).then(|| {
        chipload_job::Problem::at(
            "machine.gcode_units",
            Message::new("pipeline-post-units")
                .arg("post", post.name.as_str())
                .arg("units", units.name()),
        )
    })
}

/// Generate G-code from a project. Fails for anything that is not safe to cut, including a
/// failed independent check, so unchecked G-code never leaves this function.
pub fn run_job(job: &Job) -> Result<RunResult, PipelineError> {
    run_job_cancellable(job, &Cancel::new())
}

/// Every setup that is on and has something to cut, each run as its own program
/// (docs/specs/setups.md §4), or only setup `only`. With more than one setup in the project,
/// each file's name gets the setup's id first (`-S2`, `-S2-1-T1`); with one, the names are
/// as they always were.
pub fn run_setups(job: &Job, only: Option<u64>) -> Result<Vec<(u64, RunResult)>, PipelineError> {
    let several = job.setups.len() > 1;
    let mut runs = Vec::new();
    for setup in &job.setups {
        if only.is_some_and(|id| id != setup.id) || (only.is_none() && !setup.enabled) {
            continue;
        }
        let one = job.for_setup(setup.id);
        if several && only.is_none() && !(0..one.operations.len()).any(|i| one.cuts(i)) {
            continue;
        }
        let mut result = run_job(&one)?;
        if several {
            for file in &mut result.files {
                let tail = file.suffix.take().unwrap_or_default();
                file.suffix = Some(format!("-S{}{tail}", setup.id));
            }
        }
        runs.push((setup.id, result));
    }
    if runs.is_empty() {
        return Err(PipelineError::Job(match only {
            Some(id) => Message::new("pipeline-no-setup").arg("id", id),
            None => Message::new("pipeline-no-setup-cut"),
        }));
    }
    Ok(runs)
}

/// [`run_job`] that stops with `ToolpathError::Cancelled` once `cancel` is set, from
/// another thread (docs/specs/background-generation.md). Not cancelled, it gives the same
/// result.
pub fn run_job_cancellable(job: &Job, cancel: &Cancel) -> Result<RunResult, PipelineError> {
    // One program is one setup's (docs/specs/setups.md §4): the active one. Every setup in
    // turn: `run_setups`.
    let one = job.active_setup_id().map(|id| job.for_setup(id));
    let job = one.as_ref().unwrap_or(job);
    validate_job(job)?;
    let post = posts::job_post(job)?;
    let imported = load_drawing(job)?;
    let plan = plan_job_cancellable(job, &imported.contours, cancel)?;
    cancel.check()?;
    let (files, check_warnings) = post_and_check(job, &post, &plan.program, &plan.groups)?;
    let estimate = estimate::estimate(
        job,
        &plan.program,
        &plan.op_spans,
        &plan.groups,
        post.spindle_dwell_s,
    );
    let mut notes = plan.notes;
    notes.extend(
        extent_notes(job, &imported.report)
            .into_iter()
            .map(Note::job),
    );
    notes.extend(dxfs::layer_clash_notes(job));
    notes.extend(
        copies_notes(job, &plan.program, &plan.op_spans)
            .into_iter()
            .map(Note::job),
    );
    Ok(RunResult {
        estimate,
        files,
        post,
        import_report: imported.report,
        contours: imported.contours,
        program: plan.program,
        tabs: plan.tabs,
        pockets: plan.pockets,
        op_spans: plan.op_spans,
        groups: plan.groups,
        ring_starts: plan.ring_starts,
        unreached_corners: plan.unreached_corners,
        tab_centres: plan.tab_centres,
        floor_hints: plan.floor_hints,
        notes,
        check_warnings,
    })
}

/// Where the work zero is, for the program's header. The drawing's coordinates are mm; an
/// inch program says so (docs/specs/inch-units.md §4.2).
fn work_zero_note(job: &Job) -> String {
    match job.machine.gcode_units {
        chipload_job::Units::Mm => job.work_zero_text(),
        chipload_job::Units::Inch => format!("{}, drawing in mm", job.work_zero_text()),
    }
}

/// Post and check a plan: one program, or one per tool group. Every program is checked
/// with its own tool's radius before any is returned (docs/specs/tool-per-operation.md §6).
pub fn post_and_check(
    job: &Job,
    post: &PostConfig,
    program: &Program,
    groups: &[ToolGroup],
) -> Result<(Vec<GcodeFile>, Vec<CheckIssue>), PipelineError> {
    let work_zero = work_zero_note(job);
    let units = job.machine.gcode_units;
    let several = groups.len() > 1;
    if several && job.machine.tool_change == ToolChange::Program {
        return one_file_with_tool_changes(job, post, program, groups);
    }
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    for (i, group) in groups.iter().enumerate() {
        let part = in_work_coordinates(&group.program(program), job.work_zero_xy());
        let gcode = if several {
            let header = FileHeader {
                number: i + 1,
                count: groups.len(),
                tool: group.tool.clone(),
                operations: group.operations.clone(),
            };
            render_tool_file(&part, post, Some(&work_zero), &header, units)?
        } else {
            render_program_with(&part, post, Some(&work_zero), units)?
        };
        let limits = tool_limits(job, group.tool.id)?;
        let report = check_gcode(&gcode, &limits);
        let file = several.then_some(i + 1);
        if !report.ok() {
            return Err(check_failed(&report.errors, file, program));
        }
        warnings.extend(report.warnings.iter().map(|w| {
            match file {
                Some(file) => CheckIssue::new(
                    w.line,
                    Message::new("pipeline-check-file")
                        .arg("file", file)
                        .arg("message", w.text_message.clone()),
                ),
                None => w.clone(),
            }
        }));
        files.push(GcodeFile {
            suffix: several.then(|| format!("-{}-T{}", i + 1, group.tool.id)),
            tool: group.tool.id,
            operations: group.operations.clone(),
            gcode,
        });
    }
    Ok((files, warnings))
}

/// Every group in one program with `T M6` between them (docs/specs/tool-per-operation.md
/// §3.2), checked as a whole: each M6 sets the stock margin to its tool's radius.
fn one_file_with_tool_changes(
    job: &Job,
    post: &PostConfig,
    program: &Program,
    groups: &[ToolGroup],
) -> Result<(Vec<GcodeFile>, Vec<CheckIssue>), PipelineError> {
    let parts: Vec<ToolPart> = groups
        .iter()
        .map(|g| ToolPart {
            program: in_work_coordinates(&g.program(program), job.work_zero_xy()),
            tool: g.tool.clone(),
            operations: g.operations.clone(),
        })
        .collect();
    let gcode = render_with_tool_changes(
        &parts,
        post,
        Some(&work_zero_note(job)),
        job.machine.gcode_units,
    )?;
    let report = check_gcode(&gcode, &limits_from_job(job));
    if !report.ok() {
        return Err(check_failed(&report.errors, None, program));
    }
    let first = &groups[0];
    let file = GcodeFile {
        suffix: None,
        tool: first.tool.id,
        operations: groups.iter().flat_map(|g| g.operations.clone()).collect(),
        gcode,
    };
    Ok((vec![file], report.warnings))
}

/// The checker's `errors` as the refusal: each with its line, and its file's number when
/// the run makes several files.
fn check_failed(errors: &[CheckIssue], file: Option<usize>, program: &Program) -> PipelineError {
    let messages: Vec<Message> = errors
        .iter()
        .map(|e| match file {
            Some(file) => Message::new("pipeline-check-file-line")
                .arg("file", file)
                .arg("line", e.line)
                .arg("message", e.text_message.clone()),
            None => Message::new("pipeline-check-line")
                .arg("line", e.line)
                .arg("message", e.text_message.clone()),
        })
        .collect();
    PipelineError::CheckFailed {
        errors: messages.iter().map(Message::english).collect(),
        messages,
        program: Box::new(program.clone()),
    }
}

fn tool_limits(job: &Job, tool: i64) -> Result<CheckLimits, PipelineError> {
    limits_for_tool(job, tool).ok_or_else(|| {
        PipelineError::Job(Message::new("toolpath-tool-undefined").arg("tool", tool))
    })
}

/// The program moved so the work zero is at X0 Y0: every X and Y minus the work zero.
/// Only the output is translated; geometry and the displayed toolpath stay in drawing
/// coordinates (docs/specs/work-zero.md).
pub fn in_work_coordinates(program: &Program, (zx, zy): (f64, f64)) -> Program {
    let mut out = program.clone();
    if (zx, zy) == (0.0, 0.0) {
        return out; // drawing origin: bit-identical to the untranslated program
    }
    for m in &mut out.moves {
        m.x = m.x.map(|x| x - zx);
        m.y = m.y.map(|y| y - zy);
        if let Some(circle) = &mut m.circle {
            circle.center = (circle.center.0 - zx, circle.center.1 - zy);
        }
    }
    out
}

/// The project's contours and import report (from its embedded shapes).
pub fn load_drawing(job: &Job) -> Result<ImportResult, PipelineError> {
    Ok(contours_from_shapes(&job.drawing.shapes, &job.drawing.dxfs))
}

/// Defaults for a new project (docs/specs/job-editor.md §6): all visible and editable.
pub const DEFAULT_THICKNESS: f64 = 10.0;
/// How far a new project's stock sticks out round the drawing, each side, mm (the user,
/// 2026-10-28).
pub const DEFAULT_STOCK_MARGIN: f64 = 10.0;
/// How far a new project may cut below the stock (spoilboard), mm (decision 2026-10-10).
pub const DEFAULT_MAX_OVERCUT: f64 = 0.5;
pub const DEFAULT_SAFE_Z: f64 = 5.0;
pub const DEFAULT_TOOL_DIAMETER: f64 = 6.0;
pub const DEFAULT_RPM: f64 = 18000.0;

/// Step down of a new operation with nothing to copy from, mm.
pub const DEFAULT_STEP_DOWN: f64 = 1.0;

/// Stock of a project started without a drawing, mm (docs/specs/new-project.md decision 1).
pub const DEFAULT_EMPTY_STOCK: (f64, f64) = (300.0, 200.0);

/// A new project with no drawing yet (docs/specs/new-project.md): stock fitting the drawing
/// with a 10 mm margin once there is one (300 × 200 mm centred on the origin until then),
/// work zero at its bottom-left corner, GRBL, one 6 mm endmill, no operations.
pub fn empty_project() -> Job {
    Job {
        schema_version: CURRENT_VERSION,
        written_by: None,
        drawing: Drawing {
            shapes: Vec::new(),
            next_shape_id: 1,
            dxfs: Vec::new(),
            images: Vec::new(),
            texts: Vec::new(),
            hidden: Vec::new(),
        },
        setups: vec![chipload_job::Setup {
            id: 1,
            name: "Setup 1".into(),
            enabled: true,
            stock: Stock {
                x: DEFAULT_EMPTY_STOCK.0,
                y: DEFAULT_EMPTY_STOCK.1,
                thickness: DEFAULT_THICKNESS,
                max_overcut: DEFAULT_MAX_OVERCUT,
                // Fitting the drawing is the default (the user, 2026-10-28).
                origin: StockOrigin::Drawing,
                fit: Some(DEFAULT_STOCK_MARGIN),
                material: None,
                z_zero: ZZero::Top,
            },
            // The board's corner the operator touches off on (the user, 2026-10-29;
            // docs/specs/work-zero-marker.md §7).
            work_zero: WorkZero::Stock {
                corner: StockCorner::BottomLeft,
            },
        }],
        active_setup: Some(1),
        machine: Machine {
            name: None,
            library_uid: None,
            post: "grbl".into(),
            gcode_units: chipload_job::Units::Mm,
            safe_z: DEFAULT_SAFE_Z,
            spindle: Spindle::Cw,
            max_feed: None,
            travel: None,
            tool_change: ToolChange::Files,
            custom_post: None,
        },
        tools: vec![Tool {
            id: 1,
            name: None,
            library_uid: None,
            kind: ToolType::Endmill,
            diameter: DEFAULT_TOOL_DIAMETER,
            flutes: None,
            center_cutting: None,
            rpm: Some(DEFAULT_RPM),
            angle: None,
            tip_diameter: None,
        }],
        parts: vec![chipload_job::Part {
            id: 1,
            name: "Part 1".into(),
            enabled: true,
            copies: None,
            setup: 1,
        }],
        active_part: Some(1),
        operations: Vec::new(),
    }
}

/// A new project's starting values when lengths are shown in inches
/// (docs/specs/inch-units.md §5.2): tidy inch numbers, not converted mm ones. Stock 3/4"
/// thick (24" × 12" without a drawing) with a 1/2" margin round the drawing, 0.02" below it allowed, safe Z 0.2", a 1/4" endmill.
/// Only values that are still the mm defaults change; the project stays in mm.
pub fn with_inch_defaults(mut job: Job) -> Job {
    let inch = chipload_core::MM_PER_INCH;
    if job.drawing.shapes.is_empty() && (job.stock().x, job.stock().y) == DEFAULT_EMPTY_STOCK {
        job.stock_mut().x = 24.0 * inch;
        job.stock_mut().y = 12.0 * inch;
    }
    if job.stock().fit == Some(DEFAULT_STOCK_MARGIN) {
        job.stock_mut().fit = Some(0.5 * inch);
    }
    if job.stock().thickness == DEFAULT_THICKNESS {
        job.stock_mut().thickness = 0.75 * inch;
    }
    if job.stock().max_overcut == DEFAULT_MAX_OVERCUT {
        job.stock_mut().max_overcut = 0.02 * inch;
    }
    if job.machine.safe_z == DEFAULT_SAFE_Z {
        job.machine.safe_z = 0.2 * inch;
    }
    for tool in &mut job.tools {
        if tool.library_uid.is_none() && tool.diameter == DEFAULT_TOOL_DIAMETER {
            tool.diameter = 0.25 * inch;
        }
    }
    job
}

/// A new project holding an imported drawing: stock covering it (origin bottom left if it
/// lies in the first quadrant, else centred), GRBL, one 6 mm endmill, no operations.
pub fn new_project(imported: ImportedShapes) -> Job {
    let bbox = contours_from_shapes(&imported.shapes, std::slice::from_ref(&imported.source))
        .report
        .bbox;
    let ((x0, y0), (x1, y1)) = bbox.unwrap_or(((0.0, 0.0), (100.0, 100.0)));
    // Centred on the drawing, sticking out equally all round (the user, 2026-10-28;
    // docs/specs/stock-around-drawing.md).
    let (origin, x, y) = (
        StockOrigin::Drawing,
        x1 - x0 + 2.0 * DEFAULT_STOCK_MARGIN,
        y1 - y0 + 2.0 * DEFAULT_STOCK_MARGIN,
    );
    let next_shape_id = imported.shapes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let mut job = empty_project();
    job.drawing = Drawing {
        shapes: imported.shapes,
        next_shape_id,
        dxfs: vec![imported.source],
        images: Vec::new(),
        texts: Vec::new(),
        hidden: Vec::new(),
    };
    job.stock_mut().x = x.max(1.0);
    job.stock_mut().y = y.max(1.0);
    job.stock_mut().origin = origin;
    job.stock_mut().fit = Some(DEFAULT_STOCK_MARGIN);
    job
}

/// A note when the drawing reaches beyond the stock (docs/specs/new-project.md §3); None
/// when it fits or is empty.
pub fn stock_fit_note(job: &Job) -> Option<String> {
    stock_fit_message(job).map(|m| m.english())
}

/// [`stock_fit_note`] as a message.
pub fn stock_fit_message(job: &Job) -> Option<Message> {
    let bbox = contours_from_shapes(&job.drawing.shapes, &job.drawing.dxfs)
        .report
        .bbox?;
    let ((x0, y0), (x1, y1)) = bbox;
    let ((sx0, sy0), (sx1, sy1)) = job.stock_bounds();
    let fits = x0 >= sx0 - CHAIN_TOL_MM
        && y0 >= sy0 - CHAIN_TOL_MM
        && x1 <= sx1 + CHAIN_TOL_MM
        && y1 <= sy1 + CHAIN_TOL_MM;
    let number = |v: f64| Value::LengthNumber(v, Digits::Fixed(3));
    let length = |v: f64| Value::Length(v, Digits::Fixed(3));
    (!fits).then(|| {
        Message::new("pipeline-stock-fit")
            .arg("x0", number(x0))
            .arg("x1", length(x1))
            .arg("y0", number(y0))
            .arg("y1", length(y1))
            .arg("width", number(job.stock_size().0))
            .arg("length", length(job.stock_size().1))
    })
}

pub(crate) fn op_label(index: usize, op: &Operation) -> String {
    op.label(index)
}

pub(crate) fn ids_text(ids: &[u64]) -> String {
    ids.iter()
        .map(|id| format!("S{id}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What updating a drawing from a DXF did (docs/specs/project-file.md §4).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UpdateReport {
    pub unchanged: Vec<u64>,
    pub changed: Vec<u64>,
    pub added: Vec<u64>,
    pub removed: Vec<u64>,
    /// Operations (indices) that selected a removed shape: they can't run until fixed.
    pub lost: Vec<usize>,
    /// The report as sentences, in English, for the command line: `messages` in English.
    pub lines: Vec<String>,
    /// The report as messages, for the app (docs/specs/translations.md §7).
    pub messages: Vec<Message>,
}

/// Update the project's one DXF from `dxf` ([`dxfs::update_dxf`]); with none, `dxf` becomes
/// its first ([`dxfs::add_dxf`]). Refused when it has several: name the DXF.
pub fn update_drawing(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
) -> Result<(Job, UpdateReport), PipelineError> {
    update_drawing_with(job, dxf, units, None)
}

/// [`update_drawing`] from a drawing file, DXF or SVG; `px_per_inch` for an SVG without a
/// real size (docs/specs/svg-import.md §7).
pub fn update_drawing_with(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
    px_per_inch: Option<f64>,
) -> Result<(Job, UpdateReport), PipelineError> {
    match job.drawing.dxfs.as_slice() {
        [] => dxfs::add_drawing_file(job, dxf, units, px_per_inch, None),
        [only] => dxfs::update_drawing_file(job, only.id, dxf, units, px_per_inch),
        several => Err(PipelineError::Job(
            Message::new("pipeline-update-which-dxf")
                .arg("count", several.len())
                .arg("dxfs", placement::dxf_list(several)),
        )),
    }
}

/// The sha256 of a drawing file, lowercase hex, as stored in a project.
pub fn drawing_sha256(path: &Path) -> std::io::Result<String> {
    Ok(sha256_hex(&std::fs::read(path)?))
}

/// A note for each part whose copies reach beyond the stock (docs/specs/copies-grid.md §4,
/// parts.md §5).
fn copies_notes(job: &Job, program: &Program, spans: &[OpSpan]) -> Vec<Message> {
    let mut notes = Vec::new();
    for part in job.parts.iter().filter(|p| p.count() > 1) {
        let mine: Vec<OpSpan> = spans
            .iter()
            .filter(|s| {
                job.operations
                    .get(s.operation)
                    .is_some_and(|op| op.part() == part.id)
            })
            .cloned()
            .collect();
        let Some(((x0, y0), (x1, y1))) = cut_extent(&program.moves, &mine, None) else {
            continue;
        };
        let (lo, hi) = job.stock_bounds();
        let tol = CHAIN_TOL_MM;
        let fits = x0 >= lo.0 - tol && y0 >= lo.1 - tol && x1 <= hi.0 + tol && y1 <= hi.1 + tol;
        if fits {
            continue;
        }
        let note = if job.parts.len() > 1 {
            Message::new("pipeline-part-copies-beyond-stock").arg("part", part.name.as_str())
        } else {
            Message::new("pipeline-copies-beyond-stock")
        };
        let number = |mm: f64| Value::LengthNumber(mm, Digits::Fixed(3));
        let length = |mm: f64| Value::Length(mm, Digits::Fixed(3));
        notes.push(
            note.arg("count", part.count())
                .arg("x0", number(x0))
                .arg("x1", length(x1))
                .arg("y0", number(y0))
                .arg("y1", length(y1))
                .arg("stock_x", number(job.stock_size().0))
                .arg("stock_y", length(job.stock_size().1)),
        );
    }
    notes
}

fn extent_notes(job: &Job, report: &ImportReport) -> Vec<Message> {
    let Some(((x0, y0), (x1, y1))) = report.bbox else {
        return Vec::new();
    };
    let (lo, hi) = job.stock_bounds();
    if x0 < lo.0 || y0 < lo.1 || x1 > hi.0 || y1 > hi.1 {
        vec![Message::new("pipeline-drawing-outside-stock")]
    } else {
        Vec::new()
    }
}

/// Write via a temporary file and rename, so a machine never picks up a half-written
/// program. G-code is ASCII with LF line endings on every platform.
pub fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    if !text.is_ascii() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "G-code must be ASCII",
        ));
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}
