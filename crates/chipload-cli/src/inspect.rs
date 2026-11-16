// SPDX-License-Identifier: GPL-3.0-or-later
//! `inspect`: a drawing's or project's units, contours and issues.

use super::*;

pub(crate) fn inspect(
    out: &mut Out,
    path: &Path,
    units: Option<Units>,
    px: Option<f64>,
) -> ExitCode {
    let mut shapes = None;
    let mut work_zero = None;
    let mut copies_line: Vec<String> = Vec::new();
    let mut placement_line = None;
    let loaded = if is_svg(path) {
        shapes_from_file(path, units, px, 1.0)
            .map(|read| contours_from_shapes(&read.shapes, std::slice::from_ref(&read.source)))
            .map_err(|e| e.to_string())
    } else if is_dxf(path) {
        import_dxf(path, units).map_err(|e| e.to_string())
    } else {
        load_job(path).map_err(|e| e.to_string()).and_then(|job| {
            shapes = Some(job.drawing.shapes.clone());
            // Only when set, so a fresh project still inspects exactly like its DXF.
            if job.work_zero() != (WorkZero::DrawingOrigin {}) {
                work_zero = Some(format!("Work zero: {}", job.work_zero_text()));
            }
            copies_line = copies_text(&job)
                .map(|text| format!("Instances: {text}"))
                .into_iter()
                .chain(parts_lines(&job))
                .collect();
            let several = job.drawing.dxfs.len() > 1;
            placement_line = Some(
                job.drawing
                    .dxfs
                    .iter()
                    .filter(|s| several || s.placement != Placement::default())
                    .map(|s| {
                        if several {
                            format!("DXF {} {}: {}", s.id, s.name, placement_text(s.placement))
                        } else {
                            format!("Drawing: {}", placement_text(s.placement))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
            .filter(|text| !text.is_empty());
            load_drawing(&job).map_err(|e| e.to_string())
        })
    };
    let result = match loaded {
        Ok(result) => result,
        Err(err) => return out.fail(err),
    };
    if out.json() {
        out.set_all(import_json(&result));
        return ExitCode::SUCCESS;
    }
    for line in format_import_report(&result.report)
        .into_iter()
        .chain(format_contours(&result.contours))
        .chain(shapes.as_deref().map(format_shapes).unwrap_or_default())
        .chain(placement_line)
        .chain(work_zero)
        .chain(copies_line)
    {
        out.line(line.to_string());
    }
    ExitCode::SUCCESS
}

pub(crate) fn contour_length(contour: &Contour) -> f64 {
    contour.parts.iter().map(|p| p.segment.prim.length()).sum()
}

pub(crate) fn import_json(result: &ImportResult) -> Value {
    let report = &result.report;
    let point = |p: (f64, f64)| json!([p.0, p.1]);
    let contours: Vec<Value> = result
        .contours
        .iter()
        .map(|c| {
            let mut handles: Vec<&str> =
                c.parts.iter().map(|p| p.segment.handle.as_str()).collect();
            handles.sort_unstable();
            handles.dedup();
            // The project's shape ids (`select.shapes`); none for a plain DXF.
            let mut shapes: Vec<u64> = c.parts.iter().filter_map(|p| p.segment.shape).collect();
            shapes.sort_unstable();
            shapes.dedup();
            json!({
                "id": c.id,
                "shapes": shapes,
                "closed": c.closed,
                "layers": c.layers(),
                "handles": handles,
                "nesting_depth": c.nesting_depth,
                "invalid_reason": c.invalid_reason,
                "length": contour_length(c),
                "points": c.points().into_iter().map(point).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "units": units_name(report.units),
        "units_from": match report.units_from { UnitsFrom::Dxf => "dxf", UnitsFrom::User => "user" },
        "imported": report.imported,
        "flattened": report.flattened,
        "ignored": report.ignored,
        "issues": report.issues.iter().map(|i| json!({
            "severity": severity_name(i.severity),
            "code": i.code,
            "message": i.message,
            "location": i.location.map(point),
        })).collect::<Vec<_>>(),
        "bbox": report.bbox.map(|(lo, hi)| json!([point(lo), point(hi)])),
        "contours": contours,
    })
}
