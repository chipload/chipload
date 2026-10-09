// SPDX-License-Identifier: GPL-3.0-or-later
//! End to end: job + DXF -> checked G-code. The CLI and the GUI both call this.

pub mod notices;
pub mod report;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chipload_check::{CheckIssue, check_gcode, limits_from_job};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_dxf::{
    ImportError, ImportReport, ImportResult, ImportedShapes, contours_from_shapes, sha256_hex,
    shapes_from_dxf,
};
use chipload_job::{
    CURRENT_VERSION, Drawing, Job, Machine, Operation, Selector, Shape, Spindle, Stock,
    StockOrigin, Tool, ToolType, Units, ZZero,
};
use chipload_post::{PostConfig, PostError, load_builtin_post, render_program};
use chipload_toolpath::model::Program;
use chipload_toolpath::pocket::PocketPiece;
use chipload_toolpath::{ToolpathError, plan_job, validate_job};

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
    let gcode = render_program(&plan.program, &post)?;
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
    Ok(RunResult {
        gcode,
        post,
        import_report: imported.report,
        contours: imported.contours,
        program: plan.program,
        tabs: plan.tabs,
        pockets: plan.pockets,
        notes,
        check_warnings: report.warnings,
    })
}

/// The project's contours and import report (from its embedded shapes).
pub fn load_drawing(job: &Job) -> Result<ImportResult, PipelineError> {
    if let Some(external) = &job.drawing.external {
        return Err(PipelineError::Job(format!(
            "The drawing ({}) is not embedded in this job yet; embed it first (GUI: Embed \
             drawing, or `chipload import --into <job> <dxf> -o <project>.chipload`)",
            external.file
        )));
    }
    Ok(contours_from_shapes(
        &job.drawing.shapes,
        job.drawing.source.as_ref(),
    ))
}

/// Defaults for a new project (docs/specs/job-editor.md §6): all visible and editable.
pub const DEFAULT_THICKNESS: f64 = 10.0;
pub const DEFAULT_SAFE_Z: f64 = 5.0;
pub const DEFAULT_TOOL_DIAMETER: f64 = 6.0;
pub const DEFAULT_RPM: f64 = 18000.0;

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
    Job {
        schema_version: CURRENT_VERSION,
        written_by: None,
        drawing: Drawing {
            shapes: imported.shapes,
            next_shape_id,
            source: Some(imported.source),
            external: None,
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
    }
}

fn selector_mut(op: &mut Operation) -> &mut Selector {
    match op {
        Operation::Profile(p) => &mut p.select,
        Operation::Pocket(p) => &mut p.select,
        Operation::Drill(d) => &mut d.select,
    }
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

/// Embed the DXF of a job from before projects (docs/specs/project-file.md §3): import it,
/// turn each `handles` selector into the shapes with those handles, record the source.
/// Returns the project and one report line per operation.
pub fn embed_drawing(job: &Job, dxf: &Path) -> Result<(Job, Vec<String>), PipelineError> {
    let Some(external) = &job.drawing.external else {
        return Err(PipelineError::Job(
            "The drawing is already embedded in this project".into(),
        ));
    };
    if let Some(expected) = &external.sha256 {
        let digest = drawing_sha256(dxf)
            .map_err(|e| PipelineError::Job(format!("Cannot read {}: {e}", dxf.display())))?;
        if !digest.eq_ignore_ascii_case(expected) {
            return Err(PipelineError::Job(format!(
                "{} does not match the job's sha256; it is not the drawing the job was made \
                 for",
                dxf.display()
            )));
        }
    }
    let imported = shapes_from_dxf(dxf, Some(external.units))?;
    let mut out = job.clone();
    let mut report = vec![format!(
        "Embedded {} ({} shapes).",
        imported.source.file,
        imported.shapes.len()
    )];
    for (i, op) in out.operations.iter_mut().enumerate() {
        let label = op_label(i, op);
        let select = selector_mut(op);
        if let Some(handles) = select.handles.take().filter(|h| !h.is_empty()) {
            let ids: Vec<u64> = imported
                .shapes
                .iter()
                .filter(|s| {
                    s.dxf_handle
                        .as_ref()
                        .is_some_and(|h| handles.iter().any(|w| w.eq_ignore_ascii_case(h)))
                })
                .map(|s| s.id)
                .collect();
            if ids.is_empty() {
                return Err(PipelineError::Job(format!(
                    "{label}: no shape in {} has handle {}; fix the job's selector first",
                    imported.source.file,
                    handles.join(", ")
                )));
            }
            report.push(format!(
                "{label}: handles {} -> shapes {}",
                handles.join(", "),
                ids_text(&ids)
            ));
            select.shapes = Some(ids);
        } else if let Some(layer) = &select.layer {
            report.push(format!("{label}: layer {layer} (unchanged)"));
        }
    }
    let next_shape_id = imported.shapes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    out.drawing = Drawing {
        shapes: imported.shapes,
        next_shape_id,
        source: Some(imported.source),
        external: None,
    };
    out.schema_version = CURRENT_VERSION;
    let problems = out.validate();
    if !problems.is_empty() {
        return Err(PipelineError::Job(problems.join("; ")));
    }
    Ok((out, report))
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
    if job.drawing.external.is_some() {
        return Err(PipelineError::Job(
            "Embed the drawing before updating it".into(),
        ));
    }
    let expected = units.or(job.drawing.source.as_ref().map(|s| s.units));
    let imported = shapes_from_dxf(dxf, expected)?;
    let old = &job.drawing;
    // Old imported shapes by handle, in order; each can be matched once.
    let mut by_handle: BTreeMap<&str, Vec<&Shape>> = BTreeMap::new();
    for shape in &old.shapes {
        if let Some(handle) = &shape.dxf_handle {
            by_handle.entry(handle.as_str()).or_default().push(shape);
        }
    }
    // How many shapes with each handle the new drawing has used so far.
    let mut taken: BTreeMap<String, usize> = BTreeMap::new();
    let mut next = old.next_shape_id;
    let mut report = UpdateReport::default();
    let mut shapes: Vec<Shape> = Vec::new();
    for mut shape in imported.shapes {
        let handle = shape.dxf_handle.clone().unwrap_or_default();
        let k = taken.entry(handle.clone()).or_default();
        let matched = by_handle
            .get(handle.as_str())
            .and_then(|list| list.get(*k))
            .copied();
        *k += 1;
        match matched {
            Some(previous) => {
                shape.id = previous.id;
                let same = previous.layer == shape.layer
                    && previous.segments == shape.segments
                    && previous.dxf_type == shape.dxf_type;
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
        .filter(|s| s.dxf_handle.is_some() && !kept.contains(&s.id))
        .map(|s| s.id)
        .collect();
    // Shapes drawn in Chipload stay as they are.
    shapes.extend(
        old.shapes
            .iter()
            .filter(|s| s.dxf_handle.is_none())
            .cloned(),
    );

    let mut out = job.clone();
    out.drawing = Drawing {
        shapes,
        next_shape_id: next,
        source: Some(imported.source),
        external: None,
    };
    report.lines.push(format!(
        "Updated from {}: {} unchanged, {} changed, {} added, {} removed.",
        dxf.display(),
        report.unchanged.len(),
        report.changed.len(),
        report.added.len(),
        report.removed.len()
    ));
    for (i, op) in out.operations.iter().enumerate() {
        let label = op_label(i, op);
        let select = match op {
            Operation::Profile(p) => &p.select,
            Operation::Pocket(p) => &p.select,
            Operation::Drill(d) => &d.select,
        };
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
