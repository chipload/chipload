// SPDX-License-Identifier: GPL-3.0-or-later
//! End to end: job + DXF -> checked G-code. The CLI and the GUI both call this.

pub mod report;

use std::path::Path;

use chipload_check::{CheckIssue, check_gcode, limits_from_job};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_dxf::{ImportError, ImportReport, import_dxf};
use chipload_job::Job;
use chipload_post::{PostConfig, PostError, load_builtin_post, render_program};
use chipload_toolpath::model::Program;
use chipload_toolpath::pocket::PocketPiece;
use chipload_toolpath::{ToolpathError, plan_job, validate_job};
use sha2::{Digest, Sha256};

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
pub fn run_job(job: &Job, dxf_path: &Path) -> Result<RunResult, PipelineError> {
    verify_source(job, dxf_path)?;
    validate_job(job)?;
    let post = load_builtin_post(&job.machine.post)?;
    let imported = import_dxf(dxf_path, Some(job.source.units))?;
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

fn verify_source(job: &Job, dxf_path: &Path) -> Result<(), PipelineError> {
    let Some(expected) = &job.source.sha256 else {
        return Ok(());
    };
    let bytes = std::fs::read(dxf_path)
        .map_err(|e| PipelineError::Job(format!("Cannot read {}: {e}", dxf_path.display())))?;
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(PipelineError::Job(format!(
            "{} does not match the job's source.sha256; the drawing has changed",
            dxf_path.display()
        )))
    }
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
