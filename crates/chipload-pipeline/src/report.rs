// SPDX-License-Identifier: GPL-3.0-or-later
//! Human-readable rendering of results, for the CLI and the GUI report panel.

use chipload_core::chain::Contour;
use chipload_dxf::{ImportReport, Severity, UnitsFrom, units_name};
use chipload_job::{Geometry, Shape};

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

/// Drawn shapes with their defining values, after a count of both kinds (spec
/// drawing-tools.md section 5). Angles in degrees. Nothing if no shape is drawn, so a
/// project straight from a DXF inspects exactly like the DXF.
pub fn format_shapes(shapes: &[Shape]) -> Vec<String> {
    let drawn: Vec<&Shape> = shapes.iter().filter(|s| s.is_drawn()).collect();
    if drawn.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![format!(
        "Shapes: {} drawn, {} imported",
        drawn.len(),
        shapes.len() - drawn.len()
    )];
    for shape in drawn {
        lines.push(format!(
            "  S{:<4} {:<9} layer {:<12} {}",
            shape.id,
            shape.kind_name(),
            shape.layer,
            shape_values(&shape.geometry)
        ));
    }
    lines
}

/// The defining values of a drawn shape, for display.
pub fn shape_values(geometry: &Geometry) -> String {
    match geometry {
        Geometry::Path { segments, .. } => format!("{} segments", segments.len()),
        Geometry::Circle { center, radius } => format!(
            "centre ({:.3}, {:.3}) radius {radius:.3}",
            center[0], center[1]
        ),
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } => {
            let corners = if *corner_radius == 0.0 {
                String::new()
            } else {
                format!(" corner radius {corner_radius:.3}")
            };
            format!(
                "centre ({:.3}, {:.3}) length {length:.3} width {width:.3} angle {:.3}°{corners}",
                center[0],
                center[1],
                angle.to_degrees()
            )
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
        } => {
            let list: Vec<String> = points
                .iter()
                .map(|p| format!("({:.3}, {:.3})", p[0], p[1]))
                .collect();
            let mut text = format!(
                "{} {}",
                if *closed { "closed" } else { "open" },
                list.join(" ")
            );
            // Bulges only when a segment bends, so straight polylines read as before.
            if bulges.iter().any(|&b| b != 0.0) {
                let list: Vec<String> = bulges.iter().map(|b| format!("{b:.3}")).collect();
                text.push_str(&format!(" bulges {}", list.join(" ")));
            }
            text
        }
    }
}
