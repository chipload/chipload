// SPDX-License-Identifier: GPL-3.0-or-later
//! End to end: job + DXF -> checked G-code. The CLI and the GUI both call this.

pub mod chip_load;
pub mod copies;
pub mod dxfs;
pub mod estimate;
pub mod images;
pub mod library;
pub mod notices;
pub mod parts;
pub mod placement;
pub mod posts;
pub mod recovery;
pub mod report;
pub mod shapes;
pub mod sheet;
pub mod simulate;

use std::path::{Path, PathBuf};

use chipload_check::{CheckIssue, CheckLimits, check_gcode, limits_for_tool, limits_from_job};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_dxf::{
    ImportError, ImportReport, ImportResult, ImportedShapes, contours_from_shapes, sha256_hex,
};
use chipload_job::library::Cutting;
use chipload_job::{
    CURRENT_VERSION, Drawing, Entry, Job, Machine, Operation, PocketOp, Spindle, Stock,
    StockOrigin, Tool, ToolChange, ToolType, Units, WorkZero, ZZero,
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

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PipelineError {
    #[error("{0}")]
    Job(String),
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error(transparent)]
    Toolpath(#[from] ToolpathError),
    #[error(transparent)]
    Image(#[from] chipload_image::ImageError),
    #[error(transparent)]
    Post(#[from] PostError),
    /// The independent checker rejected the program. No G-code is returned; `program` is
    /// the rejected toolpath, for display only, and must never be posted or written out.
    #[error("G-code check failed:\n{}", errors.iter().map(|e| format!("  {e}")).collect::<Vec<_>>().join("\n"))]
    CheckFailed {
        errors: Vec<String>,
        program: Box<Program>,
    },
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
    /// Toolpath warnings.
    pub notes: Vec<String>,
    pub check_warnings: Vec<CheckIssue>,
}

/// One G-code file of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct GcodeFile {
    /// Added to the output's file stem: None for a single file, `-2-T3` for the second
    /// file of a set, cut with tool 3.
    pub suffix: Option<String>,
    pub tool: i64,
    /// Indexes in `job.operations`, in cut order.
    pub operations: Vec<usize>,
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
        let path = self.path(output);
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        format!(
            "{name} (tool {}, operations {})",
            self.tool,
            chipload_post::operations_text(&self.operations)
        )
    }
}

/// Set the tool operation `number` (1-based, as listed) cuts with. The tool must be in the
/// project; feeds stay as they are (docs/specs/tool-per-operation.md §7, decision 6).
pub fn set_op_tool(job: &mut Job, number: usize, tool: i64) -> Result<(), PipelineError> {
    if !job.tools.iter().any(|t| t.id == tool) {
        return Err(PipelineError::Job(format!(
            "the project has no tool {tool}; copy one in from your library with `tool use`"
        )));
    }
    let count = job.operations.len();
    let op = number
        .checked_sub(1)
        .and_then(|i| job.operations.get_mut(i))
        .ok_or_else(|| {
            PipelineError::Job(format!(
                "there is no operation {number}; the project has {count}"
            ))
        })?;
    op.set_tool(tool);
    Ok(())
}

/// Make pocket `number` (1-based, as listed) rest machine after tool `after` (a project
/// tool), or a whole pocket again with None (docs/specs/rest-machining.md §6). Whether the
/// tool is larger is checked with the job's other rules, so the problem shows in the GUI too.
pub fn set_op_rest(job: &mut Job, number: usize, after: Option<i64>) -> Result<(), PipelineError> {
    if let Some(tool) = after
        && !job.tools.iter().any(|t| t.id == tool)
    {
        return Err(PipelineError::Job(format!(
            "the project has no tool {tool}; copy one in from your library with `tool use`"
        )));
    }
    let count = job.operations.len();
    let op = number
        .checked_sub(1)
        .and_then(|i| job.operations.get_mut(i))
        .ok_or_else(|| {
            PipelineError::Job(format!(
                "there is no operation {number}; the project has {count}"
            ))
        })?;
    let Operation::Pocket(pocket) = op else {
        return Err(PipelineError::Job(format!(
            "operation {number} is not a pocket; only pockets rest machine"
        )));
    };
    pocket.rest = after.map(|after_tool| chipload_job::Rest { after_tool });
    Ok(())
}

/// A rest pocket for `pocket`, cut with project tool `tool` (docs/specs/rest-machining.md
/// §10): its shapes, depths and settings, rest after its tool. The feeds and step down are
/// `cutting`'s (the library's values for the tool) when given, else the pocket's. Refused,
/// with the reason, for a rest pocket and for a tool that isn't a smaller endmill.
pub fn rest_pocket_of(
    pocket: &PocketOp,
    tools: &[Tool],
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<PocketOp, String> {
    if pocket.rest.is_some() {
        return Err("it is a rest pocket itself; add the rest pocket to the first one".into());
    }
    let small = tools
        .iter()
        .find(|t| t.id == tool)
        .ok_or_else(|| format!("the project has no tool {tool}"))?;
    let large = tools
        .iter()
        .find(|t| t.id == pocket.tool)
        .ok_or_else(|| format!("the project has no tool {}", pocket.tool))?;
    if small.kind != ToolType::Endmill {
        return Err(format!("tool {tool} is not an endmill"));
    }
    if small.diameter >= large.diameter {
        return Err(format!(
            "tool {tool} ({} mm) is not smaller than the pocket's tool {} ({} mm)",
            small.diameter, large.id, large.diameter
        ));
    }
    let entry = match pocket.entry {
        Entry::Helix { .. } => Entry::default_helix(small.diameter),
        Entry::Plunge {} if small.center_cutting == Some(false) => {
            Entry::default_helix(small.diameter)
        }
        other => other,
    };
    let name = match pocket.name.as_str() {
        "" => "Rest".to_string(),
        name => format!("{name} (rest)"),
    };
    let mut rest = PocketOp {
        name,
        enabled: true,
        tool,
        entry,
        starts: Vec::new(),
        lead: None,
        corners: None,
        rest: Some(chipload_job::Rest {
            after_tool: pocket.tool,
        }),
        ..pocket.clone()
    };
    if let Some(c) = cutting {
        rest.feed = c.feed;
        rest.plunge_feed = c.plunge_feed;
        rest.step_down = c.step_down;
        rest.stepover = c.stepover;
    }
    Ok(rest)
}

/// Add a rest pocket for pocket `number` (1-based) with tool `tool`, right after it
/// (docs/specs/rest-machining.md §10). Returns the new operation's index.
pub fn add_rest_pocket(
    job: &mut Job,
    number: usize,
    tool: i64,
    cutting: Option<&Cutting>,
) -> Result<usize, PipelineError> {
    let count = job.operations.len();
    let index = number
        .checked_sub(1)
        .filter(|&i| i < count)
        .ok_or_else(|| {
            PipelineError::Job(format!(
                "there is no operation {number}; the project has {count}"
            ))
        })?;
    let Operation::Pocket(pocket) = &job.operations[index] else {
        return Err(PipelineError::Job(format!(
            "operation {number} is not a pocket; only pockets rest machine"
        )));
    };
    let rest = rest_pocket_of(pocket, &job.tools, tool, cutting)
        .map_err(|e| PipelineError::Job(format!("operation {number}: {e}")))?;
    job.operations.insert(index + 1, Operation::Pocket(rest));
    Ok(index + 1)
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

/// Every problem that keeps a project from being planned and needs no geometry to find:
/// the file's value rules, then the job rules checked before planning, each with its field
/// (docs/specs/live-problems.md).
pub fn job_problems(job: &Job) -> Vec<chipload_job::Problem> {
    let mut problems = job.problems();
    problems.extend(chipload_toolpath::job_problems(job));
    problems
}

/// Generate G-code from a project. Fails for anything that is not safe to cut, including a
/// failed independent check, so unchecked G-code never leaves this function.
pub fn run_job(job: &Job) -> Result<RunResult, PipelineError> {
    run_job_cancellable(job, &Cancel::new())
}

/// [`run_job`] that stops with `ToolpathError::Cancelled` once `cancel` is set, from
/// another thread (docs/specs/background-generation.md). Not cancelled, it gives the same
/// result.
pub fn run_job_cancellable(job: &Job, cancel: &Cancel) -> Result<RunResult, PipelineError> {
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
    notes.extend(extent_notes(job, &imported.report));
    notes.extend(dxfs::layer_clash_notes(job));
    notes.extend(copies_notes(job, &plan.program, &plan.op_spans));
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
        notes,
        check_warnings,
    })
}

/// Post and check a plan: one program, or one per tool group. Every program is checked
/// with its own tool's radius before any is returned (docs/specs/tool-per-operation.md §6).
pub fn post_and_check(
    job: &Job,
    post: &PostConfig,
    program: &Program,
    groups: &[ToolGroup],
) -> Result<(Vec<GcodeFile>, Vec<CheckIssue>), PipelineError> {
    let work_zero = job.work_zero_text();
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
            render_tool_file(&part, post, Some(&work_zero), &header)?
        } else {
            render_program_with(&part, post, Some(&work_zero))?
        };
        let limits = tool_limits(job, group.tool.id)?;
        let report = check_gcode(&gcode, &limits);
        let label = |issue: &CheckIssue| {
            if several {
                format!("file {}, line {}: {}", i + 1, issue.line, issue.message)
            } else {
                format!("line {}: {}", issue.line, issue.message)
            }
        };
        if !report.ok() {
            return Err(PipelineError::CheckFailed {
                errors: report.errors.iter().map(label).collect(),
                program: Box::new(program.clone()),
            });
        }
        warnings.extend(report.warnings.iter().map(|w| CheckIssue {
            line: w.line,
            message: if several {
                format!("file {}: {}", i + 1, w.message)
            } else {
                w.message.clone()
            },
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
    let gcode = render_with_tool_changes(&parts, post, Some(&job.work_zero_text()))?;
    let report = check_gcode(&gcode, &limits_from_job(job));
    if !report.ok() {
        return Err(PipelineError::CheckFailed {
            errors: report
                .errors
                .iter()
                .map(|e| format!("line {}: {}", e.line, e.message))
                .collect(),
            program: Box::new(program.clone()),
        });
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

fn tool_limits(job: &Job, tool: i64) -> Result<CheckLimits, PipelineError> {
    limits_for_tool(job, tool)
        .ok_or_else(|| PipelineError::Job(format!("Tool {tool} is not defined in the job")))
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
/// How far a new project may cut below the stock (spoilboard), mm (decision 2026-10-10).
pub const DEFAULT_MAX_OVERCUT: f64 = 0.5;
pub const DEFAULT_SAFE_Z: f64 = 5.0;
pub const DEFAULT_TOOL_DIAMETER: f64 = 6.0;
pub const DEFAULT_RPM: f64 = 18000.0;

/// Stock of a project started without a drawing, mm (docs/specs/new-project.md decision 1).
pub const DEFAULT_EMPTY_STOCK: (f64, f64) = (300.0, 200.0);

/// A new project with no drawing yet (docs/specs/new-project.md): 300 × 200 mm stock, origin
/// bottom left, GRBL, one 6 mm endmill, no operations.
pub fn empty_project() -> Job {
    Job {
        schema_version: CURRENT_VERSION,
        written_by: None,
        drawing: Drawing {
            shapes: Vec::new(),
            next_shape_id: 1,
            dxfs: Vec::new(),
            images: Vec::new(),
            hidden: Vec::new(),
        },
        stock: Stock {
            x: DEFAULT_EMPTY_STOCK.0,
            y: DEFAULT_EMPTY_STOCK.1,
            thickness: DEFAULT_THICKNESS,
            max_overcut: DEFAULT_MAX_OVERCUT,
            origin: StockOrigin::BottomLeft,
            z_zero: ZZero::Top,
        },
        machine: Machine {
            name: None,
            post: "grbl".into(),
            safe_z: DEFAULT_SAFE_Z,
            spindle: Spindle::Cw,
            max_feed: None,
            travel: None,
            work_zero: WorkZero::DrawingOrigin {},
            tool_change: ToolChange::Files,
            custom_post: None,
        },
        tools: vec![Tool {
            id: 1,
            name: None,
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
        }],
        active_part: Some(1),
        operations: Vec::new(),
    }
}

/// A new project holding an imported drawing: stock covering it (origin bottom left if it
/// lies in the first quadrant, else centred), GRBL, one 6 mm endmill, no operations.
pub fn new_project(imported: ImportedShapes) -> Job {
    let bbox = contours_from_shapes(&imported.shapes, std::slice::from_ref(&imported.source))
        .report
        .bbox;
    let ((x0, y0), (x1, y1)) = bbox.unwrap_or(((0.0, 0.0), (100.0, 100.0)));
    let (origin, x, y) = if x0 >= 0.0 && y0 >= 0.0 {
        (StockOrigin::BottomLeft, x1, y1)
    } else {
        (
            StockOrigin::Center,
            2.0 * x0.abs().max(x1.abs()),
            2.0 * y0.abs().max(y1.abs()),
        )
    };
    let next_shape_id = imported.shapes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let mut job = empty_project();
    job.drawing = Drawing {
        shapes: imported.shapes,
        next_shape_id,
        dxfs: vec![imported.source],
        images: Vec::new(),
        hidden: Vec::new(),
    };
    job.stock.x = x.max(1.0);
    job.stock.y = y.max(1.0);
    job.stock.origin = origin;
    job
}

/// A note when the drawing reaches beyond the stock (docs/specs/new-project.md §3); None
/// when it fits or is empty.
pub fn stock_fit_note(job: &Job) -> Option<String> {
    let bbox = contours_from_shapes(&job.drawing.shapes, &job.drawing.dxfs)
        .report
        .bbox?;
    let ((x0, y0), (x1, y1)) = bbox;
    let ((sx0, sy0), (sx1, sy1)) = job.stock.xy_bounds();
    let fits = x0 >= sx0 - CHAIN_TOL_MM
        && y0 >= sy0 - CHAIN_TOL_MM
        && x1 <= sx1 + CHAIN_TOL_MM
        && y1 <= sy1 + CHAIN_TOL_MM;
    (!fits).then(|| {
        format!(
            "The drawing (X{x0:.3} to {x1:.3}, Y{y0:.3} to {y1:.3}) extends beyond the stock \
             ({:.3} × {:.3} mm); check the stock under Job settings.",
            job.stock.x, job.stock.y
        )
    })
}

pub(crate) fn op_label(index: usize, op: &Operation) -> String {
    match op.name() {
        "" => format!("Operation {}", index + 1),
        name => format!("Operation {} ({name})", index + 1),
    }
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
    pub lines: Vec<String>,
}

/// Update the project's one DXF from `dxf` ([`dxfs::update_dxf`]); with none, `dxf` becomes
/// its first ([`dxfs::add_dxf`]). Refused when it has several: name the DXF.
pub fn update_drawing(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
) -> Result<(Job, UpdateReport), PipelineError> {
    match job.drawing.dxfs.as_slice() {
        [] => dxfs::add_dxf(job, dxf, units),
        [only] => dxfs::update_dxf(job, only.id, dxf, units),
        several => Err(PipelineError::Job(format!(
            "the project has {} DXFs ({}): say which to update",
            several.len(),
            placement::dxf_list(several)
        ))),
    }
}

/// The sha256 of a drawing file, lowercase hex, as stored in a project.
pub fn drawing_sha256(path: &Path) -> std::io::Result<String> {
    Ok(sha256_hex(&std::fs::read(path)?))
}

/// A note for each part whose copies reach beyond the stock (docs/specs/copies-grid.md §4,
/// parts.md §5).
fn copies_notes(job: &Job, program: &Program, spans: &[OpSpan]) -> Vec<String> {
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
        let (lo, hi) = job.stock.xy_bounds();
        let tol = CHAIN_TOL_MM;
        let fits = x0 >= lo.0 - tol && y0 >= lo.1 - tol && x1 <= hi.0 + tol && y1 <= hi.1 + tol;
        if fits {
            continue;
        }
        let whose = if job.parts.len() > 1 {
            format!("Part {}: the", part.name)
        } else {
            "The".to_string()
        };
        notes.push(format!(
            "{whose} {} copies cut X{x0:.3} to {x1:.3}, Y{y0:.3} to {y1:.3}, beyond the stock \
             ({:.3} × {:.3} mm); check the stock or the part's copies.",
            part.count(),
            job.stock.x,
            job.stock.y
        ));
    }
    notes
}

fn extent_notes(job: &Job, report: &ImportReport) -> Vec<String> {
    let Some(((x0, y0), (x1, y1))) = report.bbox else {
        return Vec::new();
    };
    let (lo, hi) = job.stock.xy_bounds();
    if x0 < lo.0 || y0 < lo.1 || x1 > hi.0 || y1 > hi.1 {
        vec!["The drawing extends outside the stock".to_string()]
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
