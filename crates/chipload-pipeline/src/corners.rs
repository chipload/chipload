// SPDX-License-Identifier: GPL-3.0-or-later
//! Dogbones chosen per corner (docs/specs/dogbone-per-corner.md): an operation's corners
//! with what each gets, and a corner's own choice set or removed.

use chipload_core::primitives::distance;
use chipload_core::tolerances::ARC_APPROX_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Bone, CornerChoice, Job, Operation, Wall};
use chipload_toolpath::corners::{Gets, choice_at, gets, operation_corners};
use chipload_toolpath::select::select_contours;

use crate::{PipelineError, load_drawing, no_operation};

/// One corner of an operation on the tool's side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRow {
    /// In drawing coordinates, mm.
    pub at: [f64; 2],
    /// The corner's angle on the tool's side, radians.
    pub angle: f64,
    /// None: it stays round.
    pub gets: Option<Gets>,
    /// Whether that is the corner's own choice rather than the operation's.
    pub chosen: bool,
}

/// Every corner of operation `index` (0-based) on the tool's side, in the order planning
/// finds them, with what each gets.
pub fn corners_of(job: &Job, index: usize) -> Result<Vec<CornerRow>, PipelineError> {
    let op = with_corners(job, index)?;
    let drawing = load_drawing(job)?;
    let label = job.label_message(chipload_job::OpRef::op(index));
    let selection = select_contours(&drawing.contours, op.select(), &label)
        .map_err(|err| PipelineError::Toolpath(err.at(index)))?;
    let choices = op.corner_choices();
    Ok(operation_corners(op, &selection.contours)
        .iter()
        .map(|corner| CornerRow {
            at: [corner.at.0, corner.at.1],
            angle: corner.angle,
            gets: gets(corner, op.corners(), choices),
            chosen: choice_at(choices, corner).is_some(),
        })
        .collect())
}

/// Give the corner of operation `index` (0-based) at `at` its own bone, or remove its
/// choice with None. `at` must lie within the arc tolerance of a corner; the choice is
/// stored at the corner's exact point, which is returned.
pub fn choose_corner(
    job: &mut Job,
    index: usize,
    at: [f64; 2],
    choice: Option<(Bone, Option<Wall>)>,
) -> Result<[f64; 2], PipelineError> {
    let corner = corners_of(job, index)?
        .into_iter()
        .find(|c| distance((c.at[0], c.at[1]), (at[0], at[1])) <= ARC_APPROX_TOL_MM)
        .ok_or_else(|| {
            PipelineError::Job(
                Message::new("pipeline-corner-none-there")
                    .arg("number", index + 1)
                    .arg("at", Value::Point(at[0], at[1], Digits::Fixed(3))),
            )
        })?;
    let count = job.operations.len();
    let choices = job
        .operations
        .get_mut(index)
        .and_then(Operation::corner_choices_mut)
        .ok_or_else(|| no_operation(index + 1, count))?;
    let same = |c: &CornerChoice| {
        distance((c.at[0], c.at[1]), (corner.at[0], corner.at[1])) <= ARC_APPROX_TOL_MM
    };
    choices.retain(|c| !same(c));
    if let Some((bone, along)) = choice {
        let along = match bone {
            Bone::Tbone => Some(along.unwrap_or(Wall::Longer)),
            Bone::None | Bone::Dogbone => None,
        };
        choices.push(CornerChoice {
            at: corner.at,
            bone,
            along,
        });
    }
    Ok(corner.at)
}

/// Remove every corner choice of operation `index` (0-based); returns how many there were.
pub fn reset_corners(job: &mut Job, index: usize) -> Result<usize, PipelineError> {
    with_corners(job, index)?;
    let count = job.operations.len();
    let choices = job
        .operations
        .get_mut(index)
        .and_then(Operation::corner_choices_mut)
        .ok_or_else(|| no_operation(index + 1, count))?;
    let count = choices.len();
    choices.clear();
    Ok(count)
}

/// Operation `index`, if it is a profile or a pocket.
fn with_corners(job: &Job, index: usize) -> Result<&Operation, PipelineError> {
    let op = job
        .operations
        .get(index)
        .ok_or_else(|| no_operation(index + 1, job.operations.len()))?;
    match op {
        Operation::Profile(_) | Operation::Pocket(_) => Ok(op),
        _ => Err(PipelineError::Job(
            Message::new("pipeline-corner-not-profile-or-pocket").arg("number", index + 1),
        )),
    }
}
