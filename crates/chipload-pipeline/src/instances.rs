// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's instances (docs/specs/instances.md §5): add, change and remove them, lay out a
//! grid of them, and put a new one beside what the part already cuts. Each change returns
//! the project checked, or why it was refused.

use chipload_core::Point;
use chipload_core::tolerances::PLACEMENT_ROUNDING_MM;
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
/// bottom level with the part's. Plans the part alone; when that fails (a part still being
/// worked on), the boxes round its shapes as drawn and placed stand for the cuts.
pub fn beside(
    job: &Job,
    part: u64,
    mirror: Option<Mirror>,
    angle: f64,
) -> Result<[f64; 2], PipelineError> {
    let index = part_index(job, part)?;
    let centre = job.part_centre(part).unwrap_or((0.0, 0.0));
    let ((x0, y0), (x1, y1), all_x1) = match planned_boxes(job, part) {
        Some(boxes) => boxes,
        None => {
            let (low, high) = job
                .part_box(part)
                .ok_or_else(|| refused(Message::new("pipeline-part-cuts-nothing")))?;
            let right = job.parts[index]
                .spots()
                .into_iter()
                .map(|(_, spot)| placed_box(&spot, centre, (low, high)).1.0)
                .fold(high.0, f64::max);
            (low, high, right)
        }
    };
    let spot = Spot {
        mirror,
        angle,
        at: [0.0, 0.0],
    };
    let (low, _) = placed_box(&spot, centre, ((x0, y0), (x1, y1)));
    Ok([tidy(all_x1 + INSTANCE_GAP_MM - low.0), tidy(y0 - low.1)])
}

/// A placement without the last bits a turn leaves (`PLACEMENT_ROUNDING_MM`), and never a
/// negative zero: 0, not -1e-14.
fn tidy(mm: f64) -> f64 {
    let step = PLACEMENT_ROUNDING_MM;
    (mm / step).round() * step + 0.0
}

/// The part's own cut box, as drawn, and the right edge of everything it cuts, itself and
/// its instances where they are placed, planned with the part alone; None when it can't be
/// planned.
fn planned_boxes(job: &Job, part: u64) -> Option<(Point, Point, f64)> {
    let mut alone = job.clone();
    for p in &mut alone.parts {
        p.enabled = p.id == part;
    }
    let drawing = load_drawing(&alone).ok()?;
    let plan = plan_job(&alone, &drawing.contours).ok()?;
    let (_, (all_x1, _)) = cut_extent(&plan.program.moves, &plan.op_spans, None)?;
    // Instances are placed relative to the part as drawn (part-placement.md decision 1).
    let index = alone.parts.iter().position(|p| p.id == part)?;
    if alone.parts[index].is_placed() {
        alone.parts[index].angle = 0.0;
        alone.parts[index].at = [0.0, 0.0];
        alone.parts[index].instances.clear();
        let plan = plan_job(&alone, &drawing.contours).ok()?;
        let (low, high) = cut_extent(&plan.program.moves, &plan.op_spans, Some(1))?;
        return Some((low, high, all_x1));
    }
    let (low, high) = cut_extent(&plan.program.moves, &plan.op_spans, Some(1))?;
    Some((low, high, all_x1))
}

/// The box round `(low, high)` placed at `spot` about `centre`: the turned box holds the
/// turned cut. Only moved, the box is moved exactly (no rounding through a turn by nothing).
fn placed_box(spot: &Spot, centre: Point, (low, high): (Point, Point)) -> (Point, Point) {
    let corners = [low, (high.0, low.1), high, (low.0, high.1)];
    let place = Placement::new(spot, centre);
    let placed: Vec<Point> = corners
        .into_iter()
        .map(|p| {
            if spot.is_move() {
                (p.0 + spot.at[0], p.1 + spot.at[1])
            } else {
                place.apply(p)
            }
        })
        .collect();
    let fold = |f: fn(f64, f64) -> f64, start: f64, pick: fn(&Point) -> f64| {
        placed.iter().map(pick).fold(start, f)
    };
    (
        (
            fold(f64::min, f64::INFINITY, |p| p.0),
            fold(f64::min, f64::INFINITY, |p| p.1),
        ),
        (
            fold(f64::max, f64::NEG_INFINITY, |p| p.0),
            fold(f64::max, f64::NEG_INFINITY, |p| p.1),
        ),
    )
}

/// Part `part` itself placed (docs/specs/part-placement.md §7): moved to `at` and turned to
/// `angle` where given; `reset` puts it back where drawn.
pub fn place_part(
    job: &Job,
    part: u64,
    at: Option<[f64; 2]>,
    angle: Option<f64>,
    reset: bool,
) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    let mut job = job.clone();
    let own = &mut job.parts[index];
    if reset {
        own.at = [0.0, 0.0];
        own.angle = 0.0;
    }
    own.at = at.unwrap_or(own.at);
    own.angle = angle.unwrap_or(own.angle);
    checked(job)
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

/// Instances `ids` of part `part` changed as asked. Instance 1, the part itself
/// (docs/specs/part-placement.md §7), is moved and turned only: it has no mirror of its own
/// and is switched off with its part.
pub fn set_instances(
    job: &Job,
    part: u64,
    ids: &[u32],
    change: InstanceChange,
) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    let mut job = job.clone();
    for &id in ids {
        if id == 1 {
            let name = job.parts[index].name.clone();
            if change.mirror.is_some() {
                return Err(refused(
                    Message::new("pipeline-instance-one-mirror").arg("part", name),
                ));
            }
            if change.enabled.is_some() {
                return Err(refused(
                    Message::new("pipeline-instance-one-off").arg("part", name),
                ));
            }
            let own = &mut job.parts[index];
            own.at = change.at.unwrap_or(own.at);
            own.angle = change.angle.unwrap_or(own.angle);
            continue;
        }
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
    if ids.contains(&1) {
        return Err(refused(
            Message::new("pipeline-instance-one-remove")
                .arg("part", job.parts[index].name.as_str()),
        ));
    }
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
