// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's instances (docs/specs/instances.md §5): add, change and remove them, lay out a
//! grid of them, and put a new one beside what the part already cuts. Each change returns
//! the project checked, or why it was refused.

use chipload_core::Point;
use chipload_i18n::Message;
use chipload_job::{Copies, Instance, InstanceOrder, Job, Mirror, Spot};
use chipload_toolpath::place::Placement;
use chipload_toolpath::plan::cut_extent;
use chipload_toolpath::plan_job;

use crate::{PipelineError, load_drawing};

/// How far a new instance lies from what its part cuts, mm (docs/specs/instances.md §4).
pub const INSTANCE_GAP_MM: f64 = 10.0;

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn part_index(job: &Job, part: u64) -> Result<usize, PipelineError> {
    job.parts
        .iter()
        .position(|p| p.id == part)
        .ok_or_else(|| refused(Message::new("pipeline-no-part").arg("id", part)))
}

fn instance_index(job: &Job, at: usize, id: u32) -> Result<usize, PipelineError> {
    let part = &job.parts[at];
    part.instances
        .iter()
        .position(|i| i.id == id)
        .ok_or_else(|| {
            refused(
                Message::new("pipeline-no-instance")
                    .arg("part", part.name.as_str())
                    .arg("id", id),
            )
        })
}

fn checked(job: Job) -> Result<Job, PipelineError> {
    match job.validate().into_iter().next() {
        Some(problem) => Err(refused(problem.text_message)),
        None => Ok(job),
    }
}

/// A new instance of part `part`, on, mirrored and turned as asked, moved by `at` or, with
/// None, put beside what the part cuts ([`beside`]). Returns the project and its number.
pub fn add_instance(
    job: &Job,
    part: u64,
    at: Option<[f64; 2]>,
    angle: f64,
    mirror: Option<Mirror>,
) -> Result<(Job, u32), PipelineError> {
    let index = part_index(job, part)?;
    let at = match at {
        Some(at) => at,
        None => beside(job, part, mirror, angle)?,
    };
    let mut job = job.clone();
    let id = job.parts[index].next_instance_id();
    job.parts[index].instances.push(Instance {
        id,
        enabled: true,
        mirror,
        angle,
        at,
    });
    Ok((checked(job)?, id))
}

/// Where a new instance mirrored and turned so goes (docs/specs/instances.md §4): its cut
/// [`INSTANCE_GAP_MM`] right of everything the part cuts now, its instances included, its
/// bottom level with the part's. Plans the part alone; refused when that fails.
pub fn beside(
    job: &Job,
    part: u64,
    mirror: Option<Mirror>,
    angle: f64,
) -> Result<[f64; 2], PipelineError> {
    part_index(job, part)?;
    let mut alone = job.clone();
    for p in &mut alone.parts {
        p.enabled = p.id == part;
    }
    let drawing = load_drawing(&alone)?;
    let plan = plan_job(&alone, &drawing.contours)?;
    let nothing = || refused(Message::new("pipeline-part-cuts-nothing"));
    let (_, (all_x1, _)) =
        cut_extent(&plan.program.moves, &plan.op_spans, None).ok_or_else(nothing)?;
    let ((x0, y0), (x1, y1)) =
        cut_extent(&plan.program.moves, &plan.op_spans, Some(1)).ok_or_else(nothing)?;
    // The part's cut box placed as the new instance would be before its move: the turned
    // box holds the turned cut.
    let centre = job.part_centre(part).unwrap_or((0.0, 0.0));
    let place = Placement::new(
        &Spot {
            mirror,
            angle,
            at: [0.0, 0.0],
        },
        centre,
    );
    // Only moved, the box is as it is (no rounding through a turn by nothing).
    let unturned = mirror.is_none() && angle == 0.0;
    let corners: Vec<Point> = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        .into_iter()
        .map(|p| if unturned { p } else { place.apply(p) })
        .collect();
    let low_x = corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let low_y = corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    // + 0.0: never a negative zero.
    Ok([all_x1 + INSTANCE_GAP_MM - low_x + 0.0, y0 - low_y + 0.0])
}

/// What to change on an instance; None leaves it.
#[derive(Debug, Clone, Copy, Default)]
pub struct InstanceChange {
    pub at: Option<[f64; 2]>,
    pub angle: Option<f64>,
    /// Some(None): no longer mirrored.
    pub mirror: Option<Option<Mirror>>,
    pub enabled: Option<bool>,
}

/// Instances `ids` of part `part` changed as asked.
pub fn set_instances(
    job: &Job,
    part: u64,
    ids: &[u32],
    change: InstanceChange,
) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    let mut job = job.clone();
    for &id in ids {
        let k = instance_index(&job, index, id)?;
        let instance = &mut job.parts[index].instances[k];
        if let Some(at) = change.at {
            instance.at = at;
        }
        if let Some(angle) = change.angle {
            instance.angle = angle;
        }
        if let Some(mirror) = change.mirror {
            instance.mirror = mirror;
        }
        if let Some(enabled) = change.enabled {
            instance.enabled = enabled;
        }
    }
    checked(job)
}

/// Part `part` without instances `ids`; the others keep their numbers.
pub fn remove_instances(job: &Job, part: u64, ids: &[u32]) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    for &id in ids {
        instance_index(job, index, id)?;
    }
    let mut job = job.clone();
    job.parts[index].instances.retain(|i| !ids.contains(&i.id));
    checked(job)
}

/// Part `part`'s instances made a grid (docs/specs/instances.md §5): the grid's cells but
/// the first, in place of the instances it had, numbered row by row from 2 and listed in
/// the order they are cut.
pub fn array_instances(job: &Job, part: u64, grid: Copies) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    if let Some(problem) = grid.problems("grid").into_iter().next() {
        return Err(refused(problem.text_message));
    }
    let mut job = job.clone();
    job.parts[index].instances = grid.instances();
    checked(job)
}

/// Part `part` cut in `order`.
pub fn set_order(job: &Job, part: u64, order: InstanceOrder) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    let mut job = job.clone();
    job.parts[index].order = order;
    checked(job)
}
