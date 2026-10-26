// SPDX-License-Identifier: GPL-3.0-or-later
//! Moving, turning and scaling the imported drawing (docs/specs/drawing-placement.md,
//! docs/specs/drawing-rotation.md, docs/specs/drawing-scale.md). The stored shapes are the placed ones and
//! each DXF's `placement` records its placement, so an update from the DXF can put the new
//! shapes in the same place. Each DXF of a project is placed on its own
//! (docs/specs/drawing-sources.md §2). The CLI and the GUI both go through here.

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{ANGLE_TOL_RAD, ARC_APPROX_TOL_MM, PLACEMENT_ROUNDING_MM};
use chipload_dxf::contours_from_shapes;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, PathSegment, Placement, Selector, Shape, WorkZero, rotated};

use crate::{PipelineError, stock_fit_message};

/// How to move the imported drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Move {
    /// Move by (dx, dy) mm.
    By(f64, f64),
    /// Set the placement to (dx, dy) mm from where the DXF has it.
    To(f64, f64),
    /// The imported shapes' bounding box at the stock's bottom-left corner, `margin` mm in.
    BottomLeft { margin: f64 },
    /// The bounding box's centre on the stock's centre.
    Center,
    /// Turn by this many degrees counter-clockwise (negative: clockwise) about the centre of
    /// the bounding box (docs/specs/direct-placement.md).
    Rotate(f64),
    /// Put the bounding box's lower-left corner at (x, y) mm.
    Corner(f64, f64),
    /// Scale by this factor (> 0) about the centre of the bounding box.
    Scale(f64),
    /// Scale about the centre of the bounding box until it is this wide, mm.
    Width(f64),
    /// Scale about the centre of the bounding box until it is this high, mm.
    Height(f64),
    /// Back to where the DXF has it: no turn, no scale, no move.
    Reset,
}

/// One change of placement: turn about the origin by `degrees` counter-clockwise (exact
/// for multiples of 90), scale about the origin, then move by (dx, dy).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Step {
    scale: f64,
    degrees: f64,
    dx: f64,
    dy: f64,
}

impl Step {
    fn apply(&self, p: [f64; 2]) -> [f64; 2] {
        let q = rotated(p, self.degrees);
        [self.scale * q[0] + self.dx, self.scale * q[1] + self.dy]
    }
}

/// An angle in degrees as the file keeps it, in [0, 360).
fn normal_degrees(degrees: f64) -> f64 {
    let d = degrees.rem_euclid(360.0);
    // rem_euclid of a tiny negative angle can round up to 360 itself.
    if d >= 360.0 { 0.0 } else { d }
}

/// "45", "90", "12.5": degrees for people, up to 3 decimals.
pub fn degrees_text(degrees: f64) -> String {
    let text = format!("{degrees:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// What placing did: lines to show.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceReport {
    /// The report as sentences, in English, for the command line: `messages` in English.
    pub lines: Vec<String>,
    /// The report as messages, for the app (docs/specs/translations.md §7).
    pub messages: Vec<Message>,
}

impl PlaceReport {
    fn of(messages: Vec<Message>) -> Self {
        PlaceReport {
            lines: messages.iter().map(Message::english).collect(),
            messages,
        }
    }
}

/// A scale factor as [`factor_text`] writes it.
fn factor_value(factor: f64) -> Value {
    Value::Number(factor, Digits::Trim(6))
}

/// Degrees as [`degrees_text`] writes them.
fn degrees_value(degrees: f64) -> Value {
    Value::Number(degrees, Digits::Trim(3))
}

/// Move, turn or scale the project's one DXF ([`place_dxf`]); refused when it has several.
pub fn place_drawing(job: &Job, how: Move) -> Result<(Job, PlaceReport), PipelineError> {
    match job.drawing.dxfs.as_slice() {
        [] => Err(nothing_imported()),
        [only] => place_dxf(job, only.id, how),
        several => Err(PipelineError::Job(
            Message::new("pipeline-place-which-dxf")
                .arg("count", several.len())
                .arg("dxfs", dxf_list(several)),
        )),
    }
}

/// "1 part-a, 2 part-b".
pub fn dxf_list(dxfs: &[chipload_job::DrawingSource]) -> String {
    dxfs.iter()
        .map(|d| format!("{} {}", d.id, d.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Move, turn or scale DXF `id`, with the start points that belong to it. Its shapes alone
/// move: other DXFs, drawn and traced shapes and a work zero at a point stay where they are.
pub fn place_dxf(job: &Job, id: u64, how: Move) -> Result<(Job, PlaceReport), PipelineError> {
    let Some(source) = job.drawing.dxfs.iter().find(|d| d.id == id) else {
        return Err(no_dxf(id));
    };
    let what: Value = if job.drawing.dxfs.len() > 1 {
        source.name.clone().into()
    } else {
        Message::new("pipeline-place-the-drawing").into()
    };
    let imported: Vec<Shape> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.dxf() == Some(id))
        .cloned()
        .collect();
    if imported.is_empty() {
        return Err(nothing_imported());
    }
    let step = step_for(job, &imported, source.placement, how)?;
    let Step {
        scale,
        degrees,
        dx,
        dy,
    } = step;
    if !(dx.is_finite() && dy.is_finite()) {
        return Err(PipelineError::Job(
            Message::new("pipeline-place-cannot-move")
                .arg("dx", Value::Number(dx, Digits::Shortest))
                .arg("dy", Value::Number(dy, Digits::Shortest)),
        ));
    }
    if (scale, normal_degrees(degrees), dx, dy) == (1.0, 0.0, 0.0, 0.0) {
        let report = PlaceReport::of(vec![Message::new("pipeline-place-already-there")]);
        return Ok((job.clone(), report));
    }

    let mut out = job.clone();
    let mut moved_starts = 0;
    let mut moved_corners = 0;
    for op in &mut out.operations {
        let select = op.select().clone();
        for start in op.starts_mut().into_iter().flatten() {
            if nearest_is_from(&job.drawing.shapes, &select, (start[0], start[1]), id) {
                *start = step.apply(*start);
                moved_starts += 1;
            }
        }
        // Corner choices are points on the drawing too (docs/specs/dogbone-per-corner.md §4).
        for choice in op.corner_choices_mut().into_iter().flatten() {
            if nearest_is_from(
                &job.drawing.shapes,
                &select,
                (choice.at[0], choice.at[1]),
                id,
            ) {
                choice.at = step.apply(choice.at);
                moved_corners += 1;
            }
        }
    }
    apply_to_imported(&mut out.drawing.shapes, step, Some(id));
    // The old placement followed by the step: turns add up, scales multiply, and the old
    // move turns, scales and moves too. Reset is exactly the DXF's placement.
    let old = source.placement;
    let [pdx, pdy] = step.apply([old.dx, old.dy]);
    let placement = if how == Move::Reset {
        Placement::default()
    } else {
        Placement {
            scale: old.scale * scale,
            rotation: normal_degrees(old.rotation + degrees),
            dx: pdx,
            dy: pdy,
        }
    };
    if let Some(source) = out.drawing.dxfs.iter_mut().find(|d| d.id == id) {
        source.placement = placement;
    }

    let done = match how {
        Move::Rotate(d) => Message::new("pipeline-place-turned")
            .arg("what", what)
            .arg("degrees", degrees_value(d.abs()))
            .arg("direction", if d > 0.0 { "ccw" } else { "cw" }),
        Move::Reset => Message::new("pipeline-place-reset").arg("what", what),
        Move::Scale(_) | Move::Width(_) | Move::Height(_) => {
            let scaled = Message::new("pipeline-place-scaled")
                .arg("what", what)
                .arg("factor", factor_value(scale));
            match dxf_size(&out.drawing.shapes, id) {
                Some((w, h)) => Message::new("pipeline-place-scaled-size")
                    .arg("scaled", scaled)
                    .arg("width", Value::LengthNumber(w, Digits::Fixed(3)))
                    .arg("height", Value::Length(h, Digits::Fixed(3))),
                None => scaled,
            }
        }
        _ => Message::new("pipeline-place-moved")
            .arg("what", what)
            .arg("dx", Value::LengthNumber(dx, Digits::Fixed(3)))
            .arg("dy", Value::Length(dy, Digits::Fixed(3))),
    };
    let mut lines = vec![
        Message::new("pipeline-place-done")
            .arg("done", done)
            .arg("placement", placement_message(placement)),
    ];
    if scale != 1.0 {
        lines.extend(scale_notes(&imported, &out.drawing.shapes, id, scale));
    }
    if moved_starts > 0 {
        lines.push(Message::new("pipeline-place-moved-starts").arg("count", moved_starts));
    }
    if moved_corners > 0 {
        lines.push(Message::new("pipeline-place-moved-corners").arg("count", moved_corners));
    }
    if job.drawing.shapes.iter().any(Shape::is_drawn) {
        lines.push(Message::new("pipeline-place-drawn-stay"));
    }
    if job.drawing.dxfs.len() > 1 {
        lines.push(Message::new("pipeline-place-other-dxfs-stay"));
    }
    if let WorkZero::Point { x, y } = job.machine.work_zero {
        lines.push(
            Message::new("pipeline-place-work-zero-stays")
                .arg("x", Value::Length(x, Digits::Fixed(3)))
                .arg("y", Value::Length(y, Digits::Fixed(3))),
        );
    }
    lines.extend(stock_fit_message(&out));
    crate::refuse_problems(&out.validate())?;
    Ok((out, PlaceReport::of(lines)))
}

/// "as in the DXF", or e.g. "scaled ×2, turned 90°, moved X12.000 Y-3.500 from the DXF".
pub fn placement_text(placement: Placement) -> String {
    placement_message(placement).english()
}

/// [`placement_text`] as a message.
pub fn placement_message(placement: Placement) -> Message {
    let mut parts = Vec::new();
    if placement.scale != 1.0 {
        parts.push(
            Message::new("pipeline-placement-scaled").arg("factor", factor_value(placement.scale)),
        );
    }
    if placement.rotation != 0.0 {
        parts.push(
            Message::new("pipeline-placement-turned")
                .arg("degrees", degrees_value(placement.rotation)),
        );
    }
    if (placement.dx, placement.dy) != (0.0, 0.0) {
        parts.push(
            Message::new("pipeline-placement-moved")
                .arg("dx", Value::Number(placement.dx, Digits::Fixed(3)))
                .arg("dy", Value::Number(placement.dy, Digits::Fixed(3))),
        );
    }
    let joined = parts.into_iter().reduce(|list, last| {
        Message::new("pipeline-placement-and")
            .arg("list", list)
            .arg("last", last)
    });
    match joined {
        None => Message::new("pipeline-placement-as-dxf"),
        Some(parts) => Message::new("pipeline-placement-from-dxf").arg("parts", parts),
    }
}

/// The refusal for a DXF the project doesn't have.
pub(crate) fn no_dxf(id: u64) -> PipelineError {
    PipelineError::Job(Message::new("pipeline-no-dxf").arg("id", id))
}

/// A scale factor for people: up to 6 decimals, without trailing zeros ("2", "0.5",
/// "25.4").
pub fn factor_text(factor: f64) -> String {
    let text = format!("{factor:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Width and height of the bounding box of every DXF's shapes, mm.
pub fn imported_size(shapes: &[Shape]) -> Option<(f64, f64)> {
    let imported: Vec<Shape> = shapes
        .iter()
        .filter(|s| s.dxf().is_some())
        .cloned()
        .collect();
    size_of(&imported)
}

/// Width and height of DXF `id`'s shapes, mm.
pub fn dxf_size(shapes: &[Shape], id: u64) -> Option<(f64, f64)> {
    let imported: Vec<Shape> = shapes
        .iter()
        .filter(|s| s.dxf() == Some(id))
        .cloned()
        .collect();
    size_of(&imported)
}

/// The box round DXF `id`'s shapes: lower-left and upper-right corners, mm.
pub fn dxf_box(shapes: &[Shape], id: u64) -> Option<(Point, Point)> {
    let imported: Vec<Shape> = shapes
        .iter()
        .filter(|s| s.dxf() == Some(id))
        .cloned()
        .collect();
    contours_from_shapes(&imported, &[]).report.bbox
}

fn size_of(shapes: &[Shape]) -> Option<(f64, f64)> {
    let ((x0, y0), (x1, y1)) = contours_from_shapes(shapes, &[]).report.bbox?;
    Some((x1 - x0, y1 - y0))
}

/// What scaling DXF `id` by `factor` may have changed beyond the size
/// (docs/specs/drawing-scale.md §3): curves approximated at the old size, contours that join
/// differently, units. `before` is that DXF's shapes before.
fn scale_notes(before: &[Shape], after: &[Shape], id: u64, by: f64) -> Vec<Message> {
    let mut notes = Vec::new();
    let after: Vec<Shape> = after
        .iter()
        .filter(|s| s.dxf() == Some(id))
        .cloned()
        .collect();
    let curves = after
        .iter()
        .filter(|s| matches!(s.dxf_type(), Some("SPLINE" | "ELLIPSE")))
        .count();
    if curves > 0 && by > 1.0 {
        notes.push(
            Message::new("pipeline-scale-curves")
                .arg("count", curves)
                .arg("within", Value::Length(ARC_APPROX_TOL_MM, Digits::Fixed(3)))
                .arg("factor", factor_value(by))
                .arg(
                    "off",
                    Value::Length(ARC_APPROX_TOL_MM * by, Digits::Fixed(3)),
                ),
        );
    }
    let closed = |shapes: &[Shape]| {
        contours_from_shapes(shapes, &[])
            .contours
            .iter()
            .filter(|c| c.closed)
            .count()
    };
    let (was, now) = (closed(before), closed(&after));
    if was != now {
        notes.push(
            Message::new("pipeline-scale-contours")
                .arg("was", was)
                .arg("now", now)
                .arg(
                    "within",
                    Value::Length(chipload_core::tolerances::CHAIN_TOL_MM, Digits::Fixed(3)),
                ),
        );
    }
    let units = |f: f64| (f - 25.4).abs() <= 1e-9 * 25.4 || (f * 25.4 - 1.0).abs() <= 1e-9;
    if units(by) {
        notes.push(Message::new("pipeline-scale-units"));
    }
    notes
}

fn nothing_imported() -> PipelineError {
    PipelineError::Job(Message::new("pipeline-place-nothing-imported"))
}

/// The step `how` asks for, from the current placement and the imported shapes' extent.
fn step_for(
    job: &Job,
    imported: &[Shape],
    current: Placement,
    how: Move,
) -> Result<Step, PipelineError> {
    let moved = |dx: f64, dy: f64| Step {
        scale: 1.0,
        degrees: 0.0,
        dx,
        dy,
    };
    let bbox = || {
        contours_from_shapes(imported, &[])
            .report
            .bbox
            .ok_or_else(nothing_imported)
    };
    let ((sx0, sy0), (sx1, sy1)) = job.stock.xy_bounds();
    Ok(match how {
        Move::By(dx, dy) => moved(dx, dy),
        Move::To(x, y) => moved(x - current.dx, y - current.dy),
        Move::Corner(x, y) => {
            let ((x0, y0), _) = bbox()?;
            moved(x - x0, y - y0)
        }
        Move::BottomLeft { margin } => {
            if !(margin.is_finite() && margin >= 0.0) {
                return Err(PipelineError::Job(
                    Message::new("pipeline-place-margin")
                        .arg("got", Value::Length(margin, Digits::Shortest)),
                ));
            }
            let ((x0, y0), _) = bbox()?;
            moved(sx0 + margin - x0, sy0 + margin - y0)
        }
        Move::Center => {
            let ((x0, y0), (x1, y1)) = bbox()?;
            moved(
                (sx0 + sx1) / 2.0 - (x0 + x1) / 2.0,
                (sy0 + sy1) / 2.0 - (y0 + y1) / 2.0,
            )
        }
        Move::Reset => {
            // Undo s·turn(p) + d: turn back and divide, so the move turns back and shrinks
            // too.
            let back = normal_degrees(-current.rotation);
            let d = rotated([current.dx, current.dy], back);
            Step {
                scale: 1.0 / current.scale,
                degrees: back,
                dx: -d[0] / current.scale,
                dy: -d[1] / current.scale,
            }
        }
        Move::Scale(factor) => scaled_about_centre(bbox()?, current, factor)?,
        Move::Width(width) => {
            let ((x0, y0), (x1, y1)) = bbox()?;
            let factor = size_factor("width", width, x1 - x0)?;
            scaled_about_centre(((x0, y0), (x1, y1)), current, factor)?
        }
        Move::Height(height) => {
            let ((x0, y0), (x1, y1)) = bbox()?;
            let factor = size_factor("height", height, y1 - y0)?;
            scaled_about_centre(((x0, y0), (x1, y1)), current, factor)?
        }
        Move::Rotate(d) => {
            if !d.is_finite() {
                return Err(PipelineError::Job(
                    Message::new("pipeline-place-rotation")
                        .arg("got", Value::Number(d, Digits::Shortest)),
                ));
            }
            // About the bounding box's centre c: p -> turn(p - c) + c = turn(p) + c - turn(c).
            let ((x0, y0), (x1, y1)) = bbox()?;
            let degrees = normal_degrees(d);
            let c = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
            let tc = rotated(c, degrees);
            Step {
                scale: 1.0,
                degrees,
                dx: c[0] - tc[0],
                dy: c[1] - tc[1],
            }
        }
    })
}

/// The step that scales by `factor` about the centre of `bbox`: p -> f(p - c) + c.
fn scaled_about_centre(
    bbox: (Point, Point),
    current: Placement,
    factor: f64,
) -> Result<Step, PipelineError> {
    if !(factor.is_finite() && factor > 0.0) {
        return Err(PipelineError::Job(
            Message::new("pipeline-place-scale")
                .arg("got", Value::Number(factor, Digits::Shortest)),
        ));
    }
    let total = current.scale * factor;
    if !Placement::scale_in_range(total) {
        return Err(PipelineError::Job(
            Message::new("pipeline-place-scale-range")
                .arg("factor", factor_value(total))
                .arg("min", factor_value(Placement::MIN_SCALE))
                .arg("max", factor_value(Placement::MAX_SCALE)),
        ));
    }
    let ((x0, y0), (x1, y1)) = bbox;
    let c = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    Ok(Step {
        scale: factor,
        degrees: 0.0,
        dx: c[0] - factor * c[0],
        dy: c[1] - factor * c[1],
    })
}

/// The factor that makes `current` mm into `wanted` mm.
fn size_factor(what: &str, wanted: f64, current: f64) -> Result<f64, PipelineError> {
    if !(wanted.is_finite() && wanted > 0.0) {
        return Err(PipelineError::Job(
            Message::new("pipeline-place-size")
                .arg("what", what)
                .arg("got", Value::LengthNumber(wanted, Digits::Shortest)),
        ));
    }
    if current <= PLACEMENT_ROUNDING_MM {
        return Err(PipelineError::Job(
            Message::new("pipeline-place-no-size").arg("what", what),
        ));
    }
    Ok(wanted / current)
}

/// Put shapes read from a DXF at the placement's scale (`shapes_from_dxf_scaled`) where
/// the placement says: turn and move only.
pub(crate) fn place_scaled_import(shapes: &mut [Shape], placement: Placement) {
    apply_to_imported(
        shapes,
        Step {
            scale: 1.0,
            degrees: placement.rotation,
            dx: placement.dx,
            dy: placement.dy,
        },
        None,
    );
}

/// Turn, scale, then move the shapes from DXF `id` (None: from any DXF); other shapes stay.
fn apply_to_imported(shapes: &mut [Shape], step: Step, id: Option<u64>) {
    for shape in shapes {
        let from = shape.dxf();
        if from.is_none() || (id.is_some() && from != id) {
            continue;
        }
        if let Geometry::Path { segments, .. } = &mut shape.geometry {
            for segment in segments.iter_mut() {
                let turned = segment.rotated(step.degrees);
                let scaled = if step.scale == 1.0 {
                    turned
                } else {
                    turned.scaled(step.scale)
                };
                *segment = scaled.translated(step.dx, step.dy);
            }
        }
    }
}

/// The same shape up to the last bits repeated moves and turns may leave
/// (`PLACEMENT_ROUNDING_MM`, `ANGLE_TOL_RAD`).
pub(crate) fn same_shape(a: &Shape, b: &Shape) -> bool {
    match (&a.geometry, &b.geometry) {
        (
            Geometry::Path {
                dxf_handle: ha,
                dxf_type: ta,
                closed: ca,
                segments: sa,
                image: ia,
                dxf: da,
                text: xa,
            },
            Geometry::Path {
                dxf_handle: hb,
                dxf_type: tb,
                closed: cb,
                segments: sb,
                image: ib,
                dxf: db,
                text: xb,
            },
        ) => {
            a.layer == b.layer
                && ha == hb
                && ta == tb
                && ia == ib
                && da == db
                && xa == xb
                && ca == cb
                && sa.len() == sb.len()
                && sa.iter().zip(sb).all(|(x, y)| same_segment(x, y))
        }
        _ => a == b,
    }
}

fn same_segment(a: &PathSegment, b: &PathSegment) -> bool {
    let close = |p: [f64; 2], q: [f64; 2]| {
        (p[0] - q[0]).abs() <= PLACEMENT_ROUNDING_MM && (p[1] - q[1]).abs() <= PLACEMENT_ROUNDING_MM
    };
    match (a, b) {
        (PathSegment::Line(x), PathSegment::Line(y)) => {
            close(x.start, y.start) && close(x.end, y.end)
        }
        // Radius and sweep come from trigonometry for a bulged polyline, which may round
        // differently in the last bit on another platform (a project saved on Linux,
        // updated on Windows): within the tolerances too.
        (PathSegment::Arc(x), PathSegment::Arc(y)) => {
            let turn = (x.start_angle - y.start_angle).rem_euclid(TAU);
            close(x.center, y.center)
                && (x.radius - y.radius).abs() <= PLACEMENT_ROUNDING_MM
                && (x.sweep - y.sweep).abs() <= ANGLE_TOL_RAD
                && (turn <= ANGLE_TOL_RAD || TAU - turn <= ANGLE_TOL_RAD)
        }
        _ => false,
    }
}

/// Whether the shape nearest to `p`, among those the selector picks (all shapes when it
/// picks none), comes from DXF `id`. Ties go to the earlier shape.
fn nearest_is_from(shapes: &[Shape], select: &Selector, p: Point, id: u64) -> bool {
    let picked: Vec<&Shape> = shapes.iter().filter(|s| selects(select, s)).collect();
    let candidates: Vec<&Shape> = if picked.is_empty() {
        shapes.iter().collect()
    } else {
        picked
    };
    let mut best: Option<(f64, bool)> = None;
    for shape in candidates {
        for segment in shape.segments() {
            let d = distance_to_segment(&segment, p);
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, shape.dxf() == Some(id)));
            }
        }
    }
    best.is_some_and(|(_, imported)| imported)
}

/// As planning matches segments (chipload_toolpath::select): layer names ignore case, and an
/// empty id list selects by layer only.
fn selects(select: &Selector, shape: &Shape) -> bool {
    if select.dxf.is_some_and(|d| shape.dxf() != Some(d))
        || select.image.is_some_and(|i| shape.image() != Some(i))
        || select.text.is_some_and(|t| shape.text() != Some(t))
    {
        return false;
    }
    if let Some(layer) = &select.layer
        && shape.layer.to_lowercase() != layer.to_lowercase()
    {
        return false;
    }
    if select
        .exclude
        .as_ref()
        .is_some_and(|left_out| left_out.contains(&shape.id))
    {
        return false;
    }
    match &select.shapes {
        Some(ids) if !ids.is_empty() => ids.contains(&shape.id),
        // A layer, or a whole DXF or picture.
        _ => !select.is_empty(),
    }
}

fn distance_to_segment(segment: &PathSegment, p: Point) -> f64 {
    match segment {
        PathSegment::Line(l) => {
            let (a, b) = ((l.start[0], l.start[1]), (l.end[0], l.end[1]));
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len2 = dx * dx + dy * dy;
            if len2 == 0.0 {
                return distance(a, p);
            }
            let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
            distance((a.0 + dx * t, a.1 + dy * t), p)
        }
        PathSegment::Arc(arc) => {
            let c = (arc.center[0], arc.center[1]);
            let angle = (p.1 - c.1).atan2(p.0 - c.0);
            // How far round from the start, in the arc's own direction.
            let along = ((angle - arc.start_angle) * arc.sweep.signum()).rem_euclid(TAU);
            if along <= arc.sweep.abs() {
                return (distance(c, p) - arc.radius).abs();
            }
            let at = |a: f64| (c.0 + arc.radius * a.cos(), c.1 + arc.radius * a.sin());
            let end = arc.start_angle + arc.sweep;
            distance(at(arc.start_angle), p).min(distance(at(end), p))
        }
    }
}
