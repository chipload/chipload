// SPDX-License-Identifier: GPL-3.0-or-later
//! Running a project: planned, posted and checked, setup by setup; the G-code files.

use std::path::{Path, PathBuf};

use chipload_check::{CheckIssue, CheckLimits, check_gcode, limits_for_tool, limits_from_job};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_dxf::ImportReport;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Note, ToolChange, Turn};
use chipload_post::{
    FileHeader, PostConfig, ToolPart, render_program_with, render_tool_file,
    render_with_tool_changes,
};
use chipload_toolpath::model::{MoveKind, Program};
use chipload_toolpath::plan::cut_extent;
use chipload_toolpath::pocket::PocketPiece;
use chipload_toolpath::{OpSpan, RingStart, ToolGroup, plan_job_cancellable, validate_job};

use crate::{Cancel, PipelineError, load_drawing, write_atomically};

#[derive(Debug, Clone)]
pub struct RunResult {
    /// How long the job takes (docs/specs/cycle-time.md).
    pub estimate: crate::estimate::Estimate,
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
    /// Plain notes, each with the operation it is about: a V-groove's width.
    pub facts: Vec<Note>,
    pub check_warnings: Vec<CheckIssue>,
    /// The checker's plain lines: moves below the stock bottom within the allowed overcut.
    pub check_notes: Vec<CheckIssue>,
    /// A setup cut in pieces: each piece's own program, from which this result's (the
    /// pieces put together, every piece's files) is made (docs/specs/pieces.md §7). Empty
    /// for a setup cut as one board.
    pub pieces: Vec<crate::pieces::PieceRun>,
}

/// What the checker said about a run's programs besides errors, each line naming its file
/// when there are several.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CheckFindings {
    pub warnings: Vec<CheckIssue>,
    pub notes: Vec<CheckIssue>,
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
    Ok(run_setups_pieces(job, only, None)?
        .into_iter()
        .map(|run| (run.setup, run.result))
        .collect())
}

/// One program of a run: a setup's, or a piece's of a setup in pieces.
#[derive(Debug, Clone)]
pub struct SetupRun {
    pub setup: u64,
    /// The piece's name ("A1"); None for a setup cut as one board.
    pub piece: Option<String>,
    /// The project as this program cuts it: the setup's, or for a piece, its board and
    /// work zero.
    pub job: Job,
    pub result: RunResult,
}

/// [`run_setups`], a setup in pieces as a program per piece (docs/specs/pieces.md §7), each
/// file named with its piece after the setup (`-A1`, `-S2-A1-1-T1`); with `piece`, only
/// that piece of each setup in pieces.
pub fn run_setups_pieces(
    job: &Job,
    only: Option<u64>,
    piece: Option<&str>,
) -> Result<Vec<SetupRun>, PipelineError> {
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
        // One run of the setup; a setup in pieces gives each piece's program in it.
        let mut result = run_job(&one)?;
        let results: Vec<(Option<String>, Job, RunResult)> = if result.pieces.is_empty() {
            vec![(None, one.clone(), result)]
        } else {
            if let Some(name) = piece
                && !result.pieces.iter().any(|p| p.name == name)
            {
                return Err(PipelineError::Job(
                    Message::new("pipeline-pieces-no-such").arg("piece", name.to_string()),
                ));
            }
            std::mem::take(&mut result.pieces)
                .into_iter()
                .filter(|run| piece.is_none_or(|n| n == run.name))
                .map(|run| (Some(run.name), run.job, run.result))
                .collect()
        };
        for (name, cut, mut result) in results {
            let tail_name = name.as_ref().map_or(String::new(), |n| format!("-{n}"));
            let lead = if several {
                format!("-S{}{tail_name}", setup.id)
            } else {
                tail_name
            };
            if !lead.is_empty() {
                for file in &mut result.files {
                    let tail = file.suffix.take().unwrap_or_default();
                    file.suffix = Some(format!("{lead}{tail}"));
                }
            }
            runs.push(SetupRun {
                setup: setup.id,
                piece: name,
                job: cut,
                result,
            });
        }
    }
    if runs.is_empty() {
        return Err(PipelineError::Job(match only {
            Some(id) => Message::new("pipeline-no-setup").arg("id", id),
            None => Message::new("pipeline-no-setup-cut"),
        }));
    }
    Ok(runs)
}

/// Planning's share of a run's progress.
const PLANNING_SHARE: f64 = 0.95;

/// [`run_job`] that stops with `ToolpathError::Cancelled` once `cancel` is set, from
/// another thread (docs/specs/background-generation.md). Not cancelled, it gives the same
/// result; `cancel` also carries how far it has got (§7).
pub fn run_job_cancellable(job: &Job, cancel: &Cancel) -> Result<RunResult, PipelineError> {
    // One program is one setup's (docs/specs/setups.md §4): the active one. Every setup in
    // turn: `run_setups`.
    let one = job.active_setup_id().map(|id| job.for_setup(id));
    let job = one.as_ref().unwrap_or(job);
    validate_job(job)?;
    // A setup in pieces is a program per piece (docs/specs/pieces.md §7), never one program
    // cut as one board, larger than the boards it is meant for: its result is the pieces'
    // programs put together, with every piece's files.
    if job
        .setups
        .get(job.active_setup_index())
        .is_some_and(|s| s.pieces.is_some())
    {
        return crate::pieces::run_assembled(job, cancel);
    }
    let post = crate::posts::job_post(job)?;
    let imported = load_drawing(job)?;
    // Planning is most of a run's progress, the post-processor and the check the rest
    // (docs/specs/background-generation.md §7).
    cancel.set_window(0.0, PLANNING_SHARE);
    let plan = plan_job_cancellable(job, &imported.contours, cancel)?;
    cancel.check()?;
    let (files, checked) = post_and_check(job, &post, &plan.program, &plan.groups)?;
    cancel.set_window(0.0, 1.0);
    cancel.stage_done(1.0);
    let estimate = crate::estimate::estimate(
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
    notes.extend(crate::dxfs::layer_clash_notes(job));
    notes.extend(crate::dowels::broken_hole_notes(
        job,
        &plan.program,
        &plan.op_spans,
    ));
    notes.extend(crate::setups::surface_over_pins_warning(job));
    notes.extend(crate::dowels::missing_dowels_warning(job));
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
        facts: plan
            .facts
            .into_iter()
            .chain(crate::setups::back_text_notes(job))
            .chain(crate::setups::mirrored_text_notes(job))
            .collect(),
        check_warnings: checked.warnings,
        check_notes: checked.notes,
        pieces: Vec::new(),
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
) -> Result<(Vec<GcodeFile>, CheckFindings), PipelineError> {
    let work_zero = work_zero_note(job);
    let units = job.machine.gcode_units;
    let several = groups.len() > 1;
    if several && job.machine.tool_change == ToolChange::Program {
        return one_file_with_tool_changes(job, post, program, groups);
    }
    let mut files = Vec::new();
    let mut checked = CheckFindings::default();
    for (i, group) in groups.iter().enumerate() {
        let part = in_work_coordinates(
            &on_the_board(job, &group.program(program)),
            job.work_zero_xy(),
        );
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
        let in_file = |w: &CheckIssue| match file {
            Some(file) => CheckIssue::new(
                w.line,
                Message::new("pipeline-check-file")
                    .arg("file", file)
                    .arg("message", w.text_message.clone()),
            ),
            None => w.clone(),
        };
        checked.warnings.extend(report.warnings.iter().map(in_file));
        checked.notes.extend(report.notes.iter().map(in_file));
        files.push(GcodeFile {
            suffix: several.then(|| format!("-{}-T{}", i + 1, group.tool.id)),
            tool: group.tool.id,
            operations: group.operations.clone(),
            gcode,
        });
    }
    Ok((files, checked))
}

/// Every group in one program with `T M6` between them (docs/specs/tool-per-operation.md
/// §3.2), checked as a whole: each M6 sets the stock margin to its tool's radius.
fn one_file_with_tool_changes(
    job: &Job,
    post: &PostConfig,
    program: &Program,
    groups: &[ToolGroup],
) -> Result<(Vec<GcodeFile>, CheckFindings), PipelineError> {
    let parts: Vec<ToolPart> = groups
        .iter()
        .map(|g| ToolPart {
            program: in_work_coordinates(
                &on_the_board(job, &g.program(program)),
                job.work_zero_xy(),
            ),
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
    let checked = CheckFindings {
        warnings: report.warnings,
        notes: report.notes,
    };
    Ok((vec![file], checked))
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

/// A program as it is cut on the board as it lies (docs/specs/two-sided.md §4): a back
/// setup's, planned in the front's coordinates with the cutting sense turned, mirrored about
/// the board's centre line, its arcs turning the other way; any other program as it is.
pub fn on_the_board(job: &Job, program: &Program) -> Program {
    let Some(turn) = job.back_turn() else {
        return program.clone();
    };
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let mirror = |p: (f64, f64)| match turn {
        Turn::LeftRight => (x0 + x1 - p.0, p.1),
        Turn::TopBottom => (p.0, y0 + y1 - p.1),
    };
    let mut out = program.clone();
    for m in &mut out.moves {
        match turn {
            Turn::LeftRight => m.x = m.x.map(|x| x0 + x1 - x),
            Turn::TopBottom => m.y = m.y.map(|y| y0 + y1 - y),
        }
        if let Some(circle) = &mut m.circle {
            circle.center = mirror(circle.center);
        }
        m.kind = match m.kind {
            MoveKind::ArcCw => MoveKind::ArcCcw,
            MoveKind::ArcCcw => MoveKind::ArcCw,
            kind => kind,
        };
    }
    out
}

/// The front's board heights (top, bottom) a back setup is shown on, in its program's Z
/// (docs/specs/two-sided.md §10.2); None for any other setup.
pub fn front_board_z(job: &Job) -> Option<(f64, f64)> {
    job.back_turn()?;
    let front = job.front_of(job.active_setup_id()?)?;
    let top = front.stock.top_z();
    Some((top, top - front.stock.thickness))
}

/// A back setup's planned program with its Z as heights of the front's board, for showing
/// it on that board in the front's view: a depth d below the back face is the front's bottom
/// plus d. Any other program as it is.
pub fn shown_on_board(job: &Job, program: &Program) -> Program {
    let Some((_, bottom)) = front_board_z(job) else {
        return program.clone();
    };
    let top = job.stock().top_z();
    let mut out = program.clone();
    for m in &mut out.moves {
        m.z = m.z.map(|z| bottom + top - z);
    }
    out
}

/// A note for each part whose copies reach beyond the stock (docs/specs/copies-grid.md §4,
/// parts.md §5).
fn copies_notes(job: &Job, program: &Program, spans: &[OpSpan]) -> Vec<Message> {
    let mut notes = Vec::new();
    for part in job.parts.iter().filter(|p| job.count(p.id) > 1) {
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
            note.arg("count", job.count(part.id))
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

/// What a run is made from (docs/specs/edit-and-simulate.md §5): the project without its
/// shapes, the contours each operation selects (numbers and geometry), each setup's board
/// and work zero, and whether the drawing reaches outside the stock (a note). Two projects
/// with the same inputs give the same run, so an edit that leaves them unchanged (a shape
/// no operation selects, on a board that doesn't fit the drawing) needs no new one.
#[derive(Debug, Clone, PartialEq)]
pub struct CutInputs {
    project: Job,
    selections: Vec<Vec<Contour>>,
    boards: Vec<((Point, Point), Point)>,
    outside: bool,
}

/// The [`CutInputs`] of a project; None when its drawing or a selection can't be read
/// (then a run says why).
pub fn cut_inputs(job: &Job) -> Option<CutInputs> {
    let imported = load_drawing(job).ok()?;
    let selections = chipload_toolpath::plan::selections(job, &imported.contours).ok()?;
    let boards = job
        .setups
        .iter()
        .map(|setup| {
            let one = job.for_setup(setup.id);
            (one.stock_bounds(), one.work_zero_xy())
        })
        .collect();
    let mut project = job.clone();
    project.drawing.shapes.clear();
    project.drawing.next_shape_id = 0;
    Some(CutInputs {
        project,
        selections,
        boards,
        outside: !extent_notes(job, &imported.report).is_empty(),
    })
}

fn extent_notes(job: &Job, report: &ImportReport) -> Vec<Message> {
    // With several setups, a board holds only its own setup's shapes
    // (docs/specs/setups.md §3): an inlay's plug board isn't the base's.
    let extents = if job.setups.len() > 1 {
        job.board_extents()
    } else {
        report.bbox
    };
    let Some(((x0, y0), (x1, y1))) = extents else {
        return Vec::new();
    };
    let (lo, hi) = job.stock_bounds();
    if x0 < lo.0 || y0 < lo.1 || x1 > hi.0 || y1 > hi.1 {
        vec![Message::new("pipeline-drawing-outside-stock")]
    } else {
        Vec::new()
    }
}
