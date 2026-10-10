// SPDX-License-Identifier: GPL-3.0-or-later
//! End to end: job + DXF -> checked G-code. The CLI and the GUI both call this.

pub mod copies;
pub mod library;
pub mod notices;
pub mod report;
pub mod shapes;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chipload_check::{CheckIssue, check_gcode, limits_from_job};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_dxf::{
    ImportError, ImportReport, ImportResult, ImportedShapes, contours_from_shapes, sha256_hex,
    shapes_from_dxf,
};
use chipload_job::{
    CURRENT_VERSION, Drawing, Job, Machine, Operation, Shape, Spindle, Stock, StockOrigin, Tool,
    ToolType, Units, WorkZero, ZZero,
};
use chipload_post::{PostConfig, PostError, load_builtin_post, render_program_with};
use chipload_toolpath::left::MaterialLeft;
use chipload_toolpath::model::Program;
use chipload_toolpath::plan::cut_extent;
use chipload_toolpath::pocket::PocketPiece;
use chipload_toolpath::{OpSpan, RingStart, ToolpathError, plan_job, validate_job};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PipelineError {
    #[error("{0}")]
    Job(String),
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error(transparent)]
    Toolpath(#[from] ToolpathError),
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
    pub gcode: String,
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
    /// Material each operation leaves (unreachable corners, missed spots), for display.
    pub material_left: Vec<MaterialLeft>,
    /// Which moves of `program` each operation made, for display.
    pub op_spans: Vec<OpSpan>,
    /// Rings whose start the user can move, for the start markers.
    pub ring_starts: Vec<RingStart>,
    /// Toolpath warnings.
    pub notes: Vec<String>,
    pub check_warnings: Vec<CheckIssue>,
}

/// Generate G-code. Fails for anything that is not safe to cut, including a failed
/// independent check, so unchecked G-code never leaves this function.
/// Generate G-code from a project. Fails for anything that is not safe to cut, including a
/// failed independent check, so unchecked G-code never leaves this function.
pub fn run_job(job: &Job) -> Result<RunResult, PipelineError> {
    validate_job(job)?;
    let post = load_builtin_post(&job.machine.post)?;
    let imported = load_drawing(job)?;
    let plan = plan_job(job, &imported.contours)?;
    let output = in_work_coordinates(&plan.program, job.work_zero_xy());
    let gcode = render_program_with(&output, &post, Some(&job.work_zero_text()))?;
    let report = check_gcode(&gcode, &limits_from_job(job));
    if !report.ok() {
        return Err(PipelineError::CheckFailed {
            errors: report
                .errors
                .iter()
                .map(|e| format!("line {}: {}", e.line, e.message))
                .collect(),
            program: Box::new(plan.program),
        });
    }
    let mut notes = plan.notes;
    notes.extend(extent_notes(job, &imported.report));
    notes.extend(copies_note(job, &plan.program, &plan.op_spans));
    Ok(RunResult {
        gcode,
        post,
        import_report: imported.report,
        contours: imported.contours,
        program: plan.program,
        tabs: plan.tabs,
        pockets: plan.pockets,
        material_left: plan.material_left,
        op_spans: plan.op_spans,
        ring_starts: plan.ring_starts,
        notes,
        check_warnings: report.warnings,
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
    Ok(contours_from_shapes(
        &job.drawing.shapes,
        job.drawing.source.as_ref(),
    ))
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
            source: None,
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
        },
        tools: vec![Tool {
            id: 1,
            name: None,
            kind: ToolType::Endmill,
            diameter: DEFAULT_TOOL_DIAMETER,
            flutes: None,
            center_cutting: None,
            rpm: Some(DEFAULT_RPM),
        }],
        operations: Vec::new(),
        copies: None,
    }
}

/// A new project holding an imported drawing: stock covering it (origin bottom left if it
/// lies in the first quadrant, else centred), GRBL, one 6 mm endmill, no operations.
pub fn new_project(imported: ImportedShapes) -> Job {
    let bbox = contours_from_shapes(&imported.shapes, Some(&imported.source))
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
        source: Some(imported.source),
    };
    job.stock.x = x.max(1.0);
    job.stock.y = y.max(1.0);
    job.stock.origin = origin;
    job
}

/// A note when the drawing reaches beyond the stock (docs/specs/new-project.md §3); None
/// when it fits or is empty.
pub fn stock_fit_note(job: &Job) -> Option<String> {
    let bbox = contours_from_shapes(&job.drawing.shapes, job.drawing.source.as_ref())
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

fn op_label(index: usize, op: &Operation) -> String {
    match op.name() {
        "" => format!("Operation {}", index + 1),
        name => format!("Operation {} ({name})", index + 1),
    }
}

fn ids_text(ids: &[u64]) -> String {
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

/// Replace the imported shapes with those of `dxf`, keeping ids of shapes that match by
/// (DXF handle, n-th shape with that handle). Shapes drawn in Chipload are kept. `units`
/// overrides the units recorded at the last import (needed only if they differ).
pub fn update_drawing(
    job: &Job,
    dxf: &Path,
    units: Option<Units>,
) -> Result<(Job, UpdateReport), PipelineError> {
    let expected = units.or(job.drawing.source.as_ref().map(|s| s.units));
    let imported = shapes_from_dxf(dxf, expected)?;
    let old = &job.drawing;
    // Old imported shapes by handle, in order; each can be matched once.
    let mut by_handle: BTreeMap<&str, Vec<&Shape>> = BTreeMap::new();
    for shape in &old.shapes {
        if let Some(handle) = shape.dxf_handle() {
            by_handle.entry(handle).or_default().push(shape);
        }
    }
    // How many shapes with each handle the new drawing has used so far.
    let mut taken: BTreeMap<String, usize> = BTreeMap::new();
    let mut next = old.next_shape_id;
    let mut report = UpdateReport::default();
    let mut shapes: Vec<Shape> = Vec::new();
    for mut shape in imported.shapes {
        let handle = shape.dxf_handle().unwrap_or_default().to_string();
        let k = taken.entry(handle.clone()).or_default();
        let matched = by_handle
            .get(handle.as_str())
            .and_then(|list| list.get(*k))
            .copied();
        *k += 1;
        match matched {
            Some(previous) => {
                shape.id = previous.id;
                let same = previous.layer == shape.layer && previous.geometry == shape.geometry;
                if same {
                    report.unchanged.push(shape.id);
                } else {
                    report.changed.push(shape.id);
                }
            }
            None => {
                shape.id = next;
                next += 1;
                report.added.push(shape.id);
            }
        }
        shapes.push(shape);
    }
    let kept: BTreeSet<u64> = shapes.iter().map(|s| s.id).collect();
    report.removed = old
        .shapes
        .iter()
        .filter(|s| s.dxf_handle().is_some() && !kept.contains(&s.id))
        .map(|s| s.id)
        .collect();
    // Shapes drawn in Chipload stay as they are.
    shapes.extend(
        old.shapes
            .iter()
            .filter(|s| s.dxf_handle().is_none())
            .cloned(),
    );

    let mut out = job.clone();
    out.drawing = Drawing {
        shapes,
        next_shape_id: next,
        source: Some(imported.source),
    };
    report.lines.push(format!(
        "Updated from {}: {} unchanged, {} changed, {} added, {} removed.",
        dxf.display(),
        report.unchanged.len(),
        report.changed.len(),
        report.added.len(),
        report.removed.len()
    ));
    report.lines.extend(stock_fit_note(&out));
    for (i, op) in out.operations.iter().enumerate() {
        let label = op_label(i, op);
        let select = op.select();
        if let Some(ids) = &select.shapes {
            let lost: Vec<u64> = ids
                .iter()
                .copied()
                .filter(|id| report.removed.contains(id))
                .collect();
            let changed: Vec<u64> = ids
                .iter()
                .copied()
                .filter(|id| report.changed.contains(id))
                .collect();
            if !lost.is_empty() {
                report.lost.push(i);
                report.lines.push(format!(
                    "{label}: shapes {} no longer exist; choose its geometry again",
                    ids_text(&lost)
                ));
            } else if !changed.is_empty() {
                report.lines.push(format!(
                    "{label}: geometry changed (shapes {}); check its toolpath",
                    ids_text(&changed)
                ));
            }
        } else if let Some(layer) = &select.layer {
            report.lines.push(format!(
                "{label}: layer {layer} now uses the updated drawing; check its toolpath"
            ));
        }
    }
    Ok((out, report))
}

/// The sha256 of a drawing file, lowercase hex, as stored in a project.
pub fn drawing_sha256(path: &Path) -> std::io::Result<String> {
    Ok(sha256_hex(&std::fs::read(path)?))
}

/// A note when copies reach beyond the stock (docs/specs/copies-grid.md §4).
fn copies_note(job: &Job, program: &Program, spans: &[OpSpan]) -> Option<String> {
    let copies = job.copies.filter(|c| c.count() > 1)?;
    let radius = program.tool_diameter / 2.0;
    let ((x0, y0), (x1, y1)) = cut_extent(&program.moves, spans, radius, None)?;
    let (lo, hi) = job.stock.xy_bounds();
    let tol = CHAIN_TOL_MM;
    let fits = x0 >= lo.0 - tol && y0 >= lo.1 - tol && x1 <= hi.0 + tol && y1 <= hi.1 + tol;
    (!fits).then(|| {
        format!(
            "The {} copies cut X{x0:.3} to {x1:.3}, Y{y0:.3} to {y1:.3}, beyond the stock \
             ({:.3} × {:.3} mm); check the stock or the copies under Job settings.",
            copies.count(),
            job.stock.x,
            job.stock.y
        )
    })
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
