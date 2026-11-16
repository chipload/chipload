// SPDX-License-Identifier: GPL-3.0-or-later
//! Arrays of shapes: in a grid, along a line, round a centre.

use chipload_core::math;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::Job;

use super::{Edited, layer_cutters, refused};
use crate::PipelineError;

/// The most copies one array makes: more is a typing slip, not a drawing.
pub const ARRAY_MOST: u32 = 10_000;

/// How a polar array turns its copies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Polar {
    /// The copies including the original.
    pub count: u32,
    /// The centre they go round, mm.
    pub center: (f64, f64),
    /// The sweep, degrees: 360 spreads them evenly round, less puts the first and last at
    /// its ends; negative goes clockwise.
    pub angle_deg: f64,
    /// Each copy turned with its position (false: moved only).
    pub rotate: bool,
}

/// A grid of copies of drawn shapes `ids`: `columns` × `rows` including the originals,
/// `dx`, `dy` apart (mm; negative goes left or down). The copies are new shapes on the
/// originals' layers.
pub fn array_grid(
    job: &Job,
    ids: &[u64],
    columns: u32,
    rows: u32,
    (dx, dy): (f64, f64),
) -> Result<Edited, PipelineError> {
    let total = u64::from(columns) * u64::from(rows);
    if columns == 0 || rows == 0 || total < 2 || total > u64::from(ARRAY_MOST) {
        return Err(refused(
            Message::new("pipeline-array-count")
                .arg("got", total)
                .arg("most", ARRAY_MOST),
        ));
    }
    if !(dx.is_finite() && dy.is_finite())
        || (columns > 1 && dx.abs() < CHAIN_TOL_MM)
        || (rows > 1 && dy.abs() < CHAIN_TOL_MM)
    {
        return Err(refused(Message::new("pipeline-array-spacing")));
    }
    let mut moves = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            if (row, column) != (0, 0) {
                moves.push((f64::from(column) * dx, f64::from(row) * dy, 0.0));
            }
        }
    }
    copies(job, ids, &moves, (0.0, 0.0))
}

/// A line of copies of drawn shapes `ids` (docs/specs/compass.md §8): `count` including the
/// originals, each `step` (mm, any direction) further than the one before. The copies are
/// new shapes on the originals' layers.
pub fn array_linear(
    job: &Job,
    ids: &[u64],
    count: u32,
    (dx, dy): (f64, f64),
) -> Result<Edited, PipelineError> {
    if !(2..=ARRAY_MOST).contains(&count) {
        return Err(refused(
            Message::new("pipeline-array-count")
                .arg("got", count)
                .arg("most", ARRAY_MOST),
        ));
    }
    if !(dx.is_finite() && dy.is_finite()) || math::hypot(dx, dy) < CHAIN_TOL_MM {
        return Err(refused(Message::new("pipeline-array-spacing")));
    }
    let moves: Vec<(f64, f64, f64)> = (1..count)
        .map(|k| (f64::from(k) * dx, f64::from(k) * dy, 0.0))
        .collect();
    copies(job, ids, &moves, (0.0, 0.0))
}

/// Copies of drawn shapes `ids` round a centre (`polar`): the copies are new shapes on the
/// originals' layers.
pub fn array_polar(job: &Job, ids: &[u64], polar: Polar) -> Result<Edited, PipelineError> {
    let Polar {
        count,
        center,
        angle_deg,
        rotate,
    } = polar;
    if !(2..=ARRAY_MOST).contains(&count) {
        return Err(refused(
            Message::new("pipeline-array-count")
                .arg("got", count)
                .arg("most", ARRAY_MOST),
        ));
    }
    if !(angle_deg.is_finite() && angle_deg.abs() > 0.0 && angle_deg.abs() <= 360.0)
        || !(center.0.is_finite() && center.1.is_finite())
    {
        return Err(refused(
            Message::new("pipeline-array-angle")
                .arg("got", Value::Number(angle_deg, Digits::Shortest)),
        ));
    }
    // A whole turn: evenly round, the last copy one step short of the original.
    let step = if angle_deg.abs() == 360.0 {
        angle_deg / f64::from(count)
    } else {
        angle_deg / f64::from(count - 1)
    };
    let middle = crate::shapes::drawn_middle(job, ids)?;
    let moves: Vec<(f64, f64, f64)> = (1..count)
        .map(|k| {
            let turn = step * f64::from(k);
            if rotate {
                (0.0, 0.0, turn)
            } else {
                let (sin, cos) = math::sin_cos(turn.to_radians());
                let (x, y) = (middle.0 - center.0, middle.1 - center.1);
                let to = (center.0 + x * cos - y * sin, center.1 + x * sin + y * cos);
                (to.0 - middle.0, to.1 - middle.1, 0.0)
            }
        })
        .collect();
    copies(job, ids, &moves, center)
}

/// A copy of shapes `ids` for each of `moves`: moved by (dx, dy), then turned by its
/// degrees about `about`. The notes name the operations that now cut the copies.
fn copies(
    job: &Job,
    ids: &[u64],
    moves: &[(f64, f64, f64)],
    about: (f64, f64),
) -> Result<Edited, PipelineError> {
    let mut out = job.clone();
    let mut new_ids = Vec::new();
    for &(dx, dy, degrees) in moves {
        let (next, added) = crate::shapes::copy_shapes(&out, ids, (dx, dy))?;
        out = if degrees == 0.0 {
            next
        } else {
            crate::shapes::transform_shapes(&next, &added, about, about, 1.0, degrees)?
        };
        new_ids.extend(added);
    }
    let mut layers: Vec<(String, usize)> = Vec::new();
    for shape in out
        .drawing
        .shapes
        .iter()
        .filter(|s| new_ids.contains(&s.id))
    {
        match layers.iter_mut().find(|(l, _)| *l == shape.layer) {
            Some((_, n)) => *n += 1,
            None => layers.push((shape.layer.clone(), 1)),
        }
    }
    let notes = layers
        .iter()
        .filter_map(|(layer, n)| layer_cutters(&out, layer, *n))
        .collect();
    Ok(Edited {
        job: out,
        new_ids,
        notes,
        joined: None,
    })
}

// -- align and distribute (§7)
