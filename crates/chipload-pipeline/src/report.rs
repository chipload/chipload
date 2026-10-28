// SPDX-License-Identifier: GPL-3.0-or-later
//! Human-readable rendering of results, for the CLI and the GUI report panel.

use chipload_core::chain::Contour;
use chipload_dxf::{ImportReport, Severity, UnitsFrom, units_name};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{DrawingFormat, Geometry, Shape};

pub fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

pub fn format_import_report(report: &ImportReport) -> Vec<String> {
    import_report_messages(report)
        .iter()
        .map(Message::english)
        .collect()
}

/// A number of the drawing's extents or contour table: 3 decimals, no unit.
fn coordinate(mm: f64) -> Value {
    Value::LengthNumber(mm, Digits::Fixed(3))
}

/// [`format_import_report`] as messages, for the app (docs/specs/translations.md §7).
pub fn import_report_messages(report: &ImportReport) -> Vec<Message> {
    let from = match (report.units_from, report.format) {
        (UnitsFrom::Dxf, DrawingFormat::Svg) => "svg",
        (UnitsFrom::Dxf, DrawingFormat::Dxf) => "dxf",
        (UnitsFrom::User, _) => "user",
    };
    let mut lines = vec![
        Message::new("pipeline-report-units")
            .arg("units", units_name(report.units))
            .arg("from", from),
    ];
    if let Some(((x0, y0), (x1, y1))) = report.bbox {
        lines.push(
            Message::new("pipeline-report-extents")
                .arg("x0", coordinate(x0))
                .arg("x1", coordinate(x1))
                .arg("y0", coordinate(y0))
                .arg("y1", coordinate(y1))
                .arg("width", coordinate(x1 - x0))
                .arg("height", Value::Length(y1 - y0, Digits::Fixed(3))),
        );
    }
    for (title, counts) in [
        (Message::new("pipeline-report-imported"), &report.imported),
        (Message::new("pipeline-report-flattened"), &report.flattened),
        (Message::new("pipeline-report-ignored"), &report.ignored),
    ] {
        if !counts.is_empty() {
            // DXF entity names and counts: never translated.
            let items: Vec<String> = counts.iter().map(|(k, n)| format!("{k} {n}")).collect();
            lines.push(title.arg("items", items.join(", ")));
        }
    }
    if report.issues.is_empty() {
        lines.push(Message::new("pipeline-report-no-issues"));
    } else {
        lines.push(Message::new("pipeline-report-issues").arg("count", report.issues.len()));
        lines.extend(report.issues.iter().map(|i| {
            Message::new("pipeline-report-issue")
                .arg("severity", severity_name(i.severity))
                .arg("issue", i.text_message.clone())
        }));
    }
    lines
}

pub fn format_contours(contours: &[Contour]) -> Vec<String> {
    contour_messages(contours)
        .iter()
        .map(Message::english)
        .collect()
}

/// [`format_contours`] as messages: a title and a table, one contour a line.
pub fn contour_messages(contours: &[Contour]) -> Vec<Message> {
    let mut lines = vec![Message::new("pipeline-report-contours").arg("count", contours.len())];
    for c in contours {
        let state = if !c.closed {
            "open"
        } else if c.invalid_reason.is_some() {
            "invalid"
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
        let state = Message::new("pipeline-report-contour-state").arg("state", state);
        lines.push(
            Message::new("pipeline-report-contour")
                .arg("id", Value::padded(c.id.to_string(), 4))
                .arg("state", Value::padded(state, 8))
                .arg("layers", Value::padded(c.layers().join(", "), 16))
                .arg("x0", coordinate(x0))
                .arg("x1", coordinate(x1))
                .arg("y0", coordinate(y0))
                .arg("y1", coordinate(y1)),
        );
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
        // The name last, so lines without one read as they always did.
        let name = shape
            .name
            .as_ref()
            .map(|n| format!(" name {n:?}"))
            .unwrap_or_default();
        lines.push(format!(
            "  S{:<4} {:<9} layer {:<12} {}{name}",
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
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => format!(
            "centre ({:.3}, {:.3}) radius {radius:.3} sides {sides} angle {:.3}°",
            center[0],
            center[1],
            angle.to_degrees()
        ),
    }
}
