// SPDX-License-Identifier: GPL-3.0-or-later
//! Human-readable rendering of results, for the CLI and the GUI report panel.

use chipload_core::chain::Contour;
use chipload_dxf::{ImportReport, Severity, UnitsFrom, units_name};
use chipload_job::Job;
use chipload_toolpath::left::{LeftKind, MaterialLeft};

pub fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

pub fn format_import_report(report: &ImportReport) -> Vec<String> {
    let from = match report.units_from {
        UnitsFrom::Dxf => "dxf",
        UnitsFrom::User => "user",
    };
    let mut lines = vec![format!("Units: {} (from {from})", units_name(report.units))];
    if let Some(((x0, y0), (x1, y1))) = report.bbox {
        lines.push(format!(
            "Extents: X {x0:.3} .. {x1:.3}, Y {y0:.3} .. {y1:.3} ({:.3} x {:.3} mm)",
            x1 - x0,
            y1 - y0
        ));
    }
    for (title, counts) in [
        ("Imported", &report.imported),
        ("Flattened", &report.flattened),
        ("Ignored", &report.ignored),
    ] {
        if !counts.is_empty() {
            let items: Vec<String> = counts.iter().map(|(k, n)| format!("{k} {n}")).collect();
            lines.push(format!("{title}: {}", items.join(", ")));
        }
    }
    if report.issues.is_empty() {
        lines.push("Issues: none".into());
    } else {
        lines.push(format!("Issues ({}):", report.issues.len()));
        lines.extend(
            report
                .issues
                .iter()
                .map(|i| format!("  [{}] {}", severity_name(i.severity), i.message)),
        );
    }
    lines
}

pub fn format_contours(contours: &[Contour]) -> Vec<String> {
    let mut lines = vec![format!("Contours ({}):", contours.len())];
    for c in contours {
        let state = if !c.closed {
            "open"
        } else if c.invalid_reason.is_some() {
            "INVALID"
        } else if c.nesting_depth.unwrap_or(0) % 2 == 0 {
            "outer"
        } else {
            "hole"
        };
        let pts = c.points();
        let fold = |f: fn(f64, f64) -> f64, start: f64, axis: fn(&(f64, f64)) -> f64| {
            pts.iter().map(axis).fold(start, f)
        };
        let (x0, x1) = (
            fold(f64::min, f64::INFINITY, |p| p.0),
            fold(f64::max, f64::NEG_INFINITY, |p| p.0),
        );
        let (y0, y1) = (
            fold(f64::min, f64::INFINITY, |p| p.1),
            fold(f64::max, f64::NEG_INFINITY, |p| p.1),
        );
        lines.push(format!(
            "  #{:<4} {state:<8} layer {:<16} X {x0:.3} .. {x1:.3}  Y {y0:.3} .. {y1:.3}",
            c.id,
            c.layers().join(", ")
        ));
    }
    lines
}

/// One info line per operation that leaves material the tool can't reach (spec
/// material-left.md section 4). Missed material already has its own warning.
pub fn format_material_left(job: &Job, left: &[MaterialLeft]) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, op) in job.operations.iter().enumerate() {
        let pieces: Vec<&MaterialLeft> = left
            .iter()
            .filter(|m| m.operation == index && m.kind == LeftKind::Unreachable)
            .collect();
        let size = |m: &MaterialLeft| {
            let ((x0, y0), (x1, y1)) = m.bounds();
            (x1 - x0, y1 - y0)
        };
        // Largest by bounding box area; ties keep the first (lowest, then leftmost).
        let Some(largest) = pieces.iter().copied().reduce(|best, m| {
            let (a, b) = (size(best), size(m));
            if b.0 * b.1 > a.0 * a.1 { m } else { best }
        }) else {
            continue;
        };
        let label = match op.name() {
            "" => format!("Operation {}", index + 1),
            name => format!("Operation {} ({name})", index + 1),
        };
        let count = pieces.len();
        let (w, h) = size(largest);
        lines.push(format!(
            "{label}: the tool can't reach material in {count} place{}, largest about \
             {w:.2} x {h:.2} mm",
            if count > 1 { "s" } else { "" }
        ));
    }
    lines
}
