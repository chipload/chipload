// SPDX-License-Identifier: GPL-3.0-or-later
//! Scale by reference and stretch in X and Y (docs/specs/scale-and-stretch.md).

use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::Job;
use chipload_job::edit::stretch::stretched;

use crate::PipelineError;
use crate::shapes::{checked, drawn_box, drawn_shapes, follow_points, transform_shapes};

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// The factor that makes the distance from `a` to `b` `length` (spec §2).
pub fn reference_factor(a: [f64; 2], b: [f64; 2], length: f64) -> Result<f64, PipelineError> {
    let now = chipload_core::math::hypot(b[0] - a[0], b[1] - a[1]);
    if !(now.is_finite() && now > CHAIN_TOL_MM) {
        return Err(refused(Message::new("pipeline-reference-points")));
    }
    if !(length.is_finite() && length > 0.0) {
        return Err(refused(
            Message::new("pipeline-reference-length")
                .arg("got", Value::Length(length, Digits::Shortest)),
        ));
    }
    Ok(length / now)
}

/// Drawn shapes `ids` scaled so the distance from `a` to `b` becomes `length`, `a` staying.
pub fn scale_by_reference(
    job: &Job,
    ids: &[u64],
    a: [f64; 2],
    b: [f64; 2],
    length: f64,
) -> Result<Job, PipelineError> {
    let factor = reference_factor(a, b, length)?;
    let at = (a[0], a[1]);
    transform_shapes(job, ids, at, at, factor, 0.0)
}

/// One axis of a stretch: a factor, or the size the box round the shapes should have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Along {
    Factor(f64),
    Size(f64),
}

/// A stretched project and the shapes that became polylines or had their arcs fitted
/// again.
#[derive(Debug, Clone)]
pub struct StretchDone {
    pub job: Job,
    pub fitted: Vec<u64>,
    /// The factors used, along X and Y.
    pub factors: (f64, f64),
}

/// Drawn shapes `ids` stretched along X and Y about `about` (None: the middle of the box
/// round them), spec §3. Equal factors scale evenly, so circles stay circles.
pub fn stretch_shapes(
    job: &Job,
    ids: &[u64],
    about: Option<[f64; 2]>,
    x: Along,
    y: Along,
) -> Result<StretchDone, PipelineError> {
    drawn_shapes(job, ids)?;
    let ((x0, y0), (x1, y1)) =
        drawn_box(job, ids).ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?;
    let fx = factor(x, x1 - x0)?;
    let fy = factor(y, y1 - y0)?;
    let about = about.unwrap_or([(x0 + x1) / 2.0, (y0 + y1) / 2.0]);
    if !(about[0].is_finite() && about[1].is_finite()) {
        return Err(refused(Message::new("pipeline-transform-not-numbers")));
    }
    if fx == fy {
        let at = (about[0], about[1]);
        let job = transform_shapes(job, ids, at, at, fx, 0.0)?;
        return Ok(StretchDone {
            job,
            fitted: Vec::new(),
            factors: (fx, fy),
        });
    }
    let mut out = job.clone();
    let mut fitted = Vec::new();
    for shape in out
        .drawing
        .shapes
        .iter_mut()
        .filter(|s| ids.contains(&s.id))
    {
        if let Some(done) = stretched(&shape.geometry, about, fx, fy) {
            if done.changed {
                fitted.push(shape.id);
            }
            shape.geometry = done.geometry;
        }
    }
    let map = |p: [f64; 2]| {
        [
            about[0] + fx * (p[0] - about[0]),
            about[1] + fy * (p[1] - about[1]),
        ]
    };
    follow_points(&mut out, job, ids, |p| Some(map(p)));
    Ok(StretchDone {
        job: checked(out)?,
        fitted,
        factors: (fx, fy),
    })
}

/// An axis's factor: given, or the size wanted over the size now.
fn factor(along: Along, now: f64) -> Result<f64, PipelineError> {
    let factor = match along {
        Along::Factor(f) => f,
        Along::Size(size) => {
            if !(size.is_finite() && size > 0.0) {
                return Err(refused(
                    Message::new("pipeline-stretch-size")
                        .arg("got", Value::Length(size, Digits::Shortest)),
                ));
            }
            if now <= CHAIN_TOL_MM {
                return Err(refused(Message::new("pipeline-stretch-flat")));
            }
            size / now
        }
    };
    if !(factor.is_finite() && factor > 0.0) {
        return Err(refused(
            Message::new("pipeline-place-scale")
                .arg("got", Value::Number(factor, Digits::Shortest)),
        ));
    }
    Ok(factor)
}

#[cfg(test)]
mod tests;
