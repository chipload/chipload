// SPDX-License-Identifier: GPL-3.0-or-later
//! Moving and turning the imported drawing (docs/specs/drawing-placement.md,
//! docs/specs/drawing-rotation.md). The stored shapes are the placed ones and
//! `drawing.source.placement` records the placement, so an update from the DXF can put the
//! new shapes in the same place. The CLI and the GUI both go through here.

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{ANGLE_TOL_RAD, PLACEMENT_ROUNDING_MM};
use chipload_dxf::contours_from_shapes;
use chipload_job::{
    Geometry, Job, PathSegment, Placement, Selector, Shape, WorkZero, quarter_turned,
};

use crate::{PipelineError, stock_fit_note};

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
    /// Turn by this many × 90° counter-clockwise (negative: clockwise) about the centre of
    /// the bounding box.
    Turn(i32),
    /// Back to where the DXF has it: no turn, no move.
    Reset,
}

/// One change of placement: turn about the origin by `turns` × 90° counter-clockwise,
/// then move by (dx, dy).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Step {
    turns: u8,
    dx: f64,
    dy: f64,
}

impl Step {
    fn apply(&self, p: [f64; 2]) -> [f64; 2] {
        let q = quarter_turned(p, self.turns);
        [q[0] + self.dx, q[1] + self.dy]
    }
}

/// What placing did: lines to show.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceReport {
    pub lines: Vec<String>,
}

/// Move or turn the imported drawing, with the start points that belong to it. Drawn shapes
/// and a work zero at a point stay where they are.
pub fn place_drawing(job: &Job, how: Move) -> Result<(Job, PlaceReport), PipelineError> {
    let Some(source) = &job.drawing.source else {
        return Err(nothing_imported());
    };
    let imported: Vec<Shape> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| !s.is_drawn())
        .cloned()
        .collect();
    if imported.is_empty() {
        return Err(nothing_imported());
    }
    let step = step_for(job, &imported, source.placement, how)?;
    let Step { turns, dx, dy } = step;
    if !(dx.is_finite() && dy.is_finite()) {
        return Err(PipelineError::Job(format!(
            "can't move the drawing by ({dx}, {dy})"
        )));
    }
    if (turns, dx, dy) == (0, 0.0, 0.0) {
        let report = PlaceReport {
            lines: vec!["The drawing is already there; nothing moved.".into()],
        };
        return Ok((job.clone(), report));
    }

    let mut out = job.clone();
    let mut moved_starts = 0;
    for op in &mut out.operations {
        let select = op.select().clone();
        for start in op.starts_mut() {
            if nearest_is_imported(&job.drawing.shapes, &select, (start[0], start[1])) {
                *start = step.apply(*start);
                moved_starts += 1;
            }
        }
    }
    apply_to_imported(&mut out.drawing.shapes, step);
    // turn(old placement) followed by the step: turns add up, the old move turns too.
    let old = source.placement;
    let [pdx, pdy] = step.apply([old.dx, old.dy]);
    let placement = Placement {
        quarter_turns: (old.quarter_turns + turns) % 4,
        dx: pdx,
        dy: pdy,
    };
    if let Some(source) = &mut out.drawing.source {
        source.placement = placement;
    }

    let done = match how {
        Move::Turn(k) => format!(
            "Turned the imported drawing {}° {}",
            90 * k.unsigned_abs(),
            if k > 0 {
                "counter-clockwise"
            } else {
                "clockwise"
            }
        ),
        Move::Reset => "Put the imported drawing back where the DXF has it".to_string(),
        _ => format!("Moved the imported drawing by X{dx:.3} Y{dy:.3} mm"),
    };
    let mut lines = vec![format!("{done}; it is now {}.", placement_text(placement))];
    if moved_starts > 0 {
        lines.push(format!("Moved {moved_starts} start point(s) with it."));
    }
    if job.drawing.shapes.iter().any(Shape::is_drawn) {
        lines.push("Shapes drawn in Chipload stay where they are.".into());
    }
    if let WorkZero::Point { x, y } = job.machine.work_zero {
        lines.push(format!(
            "The work zero stays at the point X{x:.3} Y{y:.3}; check that it's still where \
             you want it."
        ));
    }
    lines.extend(stock_fit_note(&out));
    let problems = out.validate();
    if !problems.is_empty() {
        return Err(PipelineError::Job(problems.join("; ")));
    }
    Ok((out, PlaceReport { lines }))
}

/// "as in the DXF", or e.g. "turned 90°, moved X12.000 Y-3.500 from the DXF".
pub fn placement_text(placement: Placement) -> String {
    let mut parts = Vec::new();
    if placement.quarter_turns != 0 {
        parts.push(format!(
            "turned {}°",
            90 * u32::from(placement.quarter_turns)
        ));
    }
    if (placement.dx, placement.dy) != (0.0, 0.0) {
        parts.push(format!("moved X{:.3} Y{:.3}", placement.dx, placement.dy));
    }
    if parts.is_empty() {
        "as in the DXF".into()
    } else {
        format!("{} from the DXF", parts.join(", "))
    }
}

fn nothing_imported() -> PipelineError {
    PipelineError::Job("the project has no imported drawing to move".into())
}

/// The step `how` asks for, from the current placement and the imported shapes' extent.
fn step_for(
    job: &Job,
    imported: &[Shape],
    current: Placement,
    how: Move,
) -> Result<Step, PipelineError> {
    let moved = |dx: f64, dy: f64| Step { turns: 0, dx, dy };
    let bbox = || {
        contours_from_shapes(imported, None)
            .report
            .bbox
            .ok_or_else(nothing_imported)
    };
    let ((sx0, sy0), (sx1, sy1)) = job.stock.xy_bounds();
    Ok(match how {
        Move::By(dx, dy) => moved(dx, dy),
        Move::To(x, y) => moved(x - current.dx, y - current.dy),
        Move::BottomLeft { margin } => {
            if !(margin.is_finite() && margin >= 0.0) {
                return Err(PipelineError::Job(format!(
                    "the margin must be 0 or more, got {margin}"
                )));
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
            // Undo turn(p) + d: turn back by the same turns, so the move turns back too.
            let back = (4 - current.quarter_turns % 4) % 4;
            let d = quarter_turned([current.dx, current.dy], back);
            Step {
                turns: back,
                dx: -d[0],
                dy: -d[1],
            }
        }
        Move::Turn(k) => {
            // About the bounding box's centre c: p -> turn(p - c) + c = turn(p) + c - turn(c).
            let ((x0, y0), (x1, y1)) = bbox()?;
            let turns = u8::try_from(k.rem_euclid(4)).unwrap_or(0);
            let c = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
            let tc = quarter_turned(c, turns);
            Step {
                turns,
                dx: c[0] - tc[0],
                dy: c[1] - tc[1],
            }
        }
    })
}

/// Put shapes read from the DXF where `placement` says; drawn shapes stay.
pub(crate) fn place_imported(shapes: &mut [Shape], placement: Placement) {
    apply_to_imported(
        shapes,
        Step {
            turns: placement.quarter_turns,
            dx: placement.dx,
            dy: placement.dy,
        },
    );
}

/// Turn, then move, every imported shape; drawn shapes stay.
fn apply_to_imported(shapes: &mut [Shape], step: Step) {
    for shape in shapes {
        if let Geometry::Path { segments, .. } = &mut shape.geometry {
            for segment in segments.iter_mut() {
                *segment = segment
                    .quarter_turned(step.turns)
                    .translated(step.dx, step.dy);
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
            },
            Geometry::Path {
                dxf_handle: hb,
                dxf_type: tb,
                closed: cb,
                segments: sb,
            },
        ) => {
            a.layer == b.layer
                && ha == hb
                && ta == tb
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
/// picks none), is imported. Ties go to the earlier shape.
fn nearest_is_imported(shapes: &[Shape], select: &Selector, p: Point) -> bool {
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
                best = Some((d, !shape.is_drawn()));
            }
        }
    }
    best.is_some_and(|(_, imported)| imported)
}

/// As planning matches segments (chipload_toolpath::select): layer names ignore case, and an
/// empty id list selects by layer only.
fn selects(select: &Selector, shape: &Shape) -> bool {
    if let Some(layer) = &select.layer
        && shape.layer.to_lowercase() != layer.to_lowercase()
    {
        return false;
    }
    match &select.shapes {
        Some(ids) if !ids.is_empty() => ids.contains(&shape.id),
        _ => select.layer.is_some(),
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
