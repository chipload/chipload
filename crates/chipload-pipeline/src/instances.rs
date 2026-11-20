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

fn instance_exists(job: &Job, at: usize, id: u32) -> Result<(), PipelineError> {
    let part = &job.parts[at];
    match job.instance(part.id, id) {
        Some(_) => Ok(()),
        None => Err(refused(
            Message::new("pipeline-no-instance")
                .arg("part", part.name.as_str())
                .arg("id", id),
        )),
    }
}

/// The setup a new instance of part `part` goes on: `setup` when given (it must exist),
/// else the board the part is cut from ([`Job::home_setup`]), else the active one.
fn target_setup(job: &Job, part: u64, setup: Option<u64>) -> Result<u64, PipelineError> {
    match setup {
        Some(id) if job.setup(id).is_some() => Ok(id),
        Some(id) => Err(refused(Message::new("pipeline-no-setup").arg("id", id))),
        None => job
            .home_setup(part)
            .or_else(|| job.active_setup_id())
            .ok_or_else(|| refused(Message::new("pipeline-no-setup").arg("id", 0))),
    }
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
    setup: Option<u64>,
    at: Option<[f64; 2]>,
    angle: f64,
    mirror: Option<Mirror>,
) -> Result<(Job, u32), PipelineError> {
    part_index(job, part)?;
    let setup = target_setup(job, part, setup)?;
    let at = match at {
        Some(at) => at,
        None => beside(job, part, mirror, angle)?,
    };
    let mut job = job.clone();
    let id = job.next_instance_id(part);
    if let Some(s) = job.setup_mut(setup) {
        s.instances.push(Instance {
            part,
            id,
            enabled: true,
            mirror,
            angle,
            at,
        });
    }
    Ok((checked(job)?, id))
}

/// A new instance of part `part` like instance `like`: on its setup, turned and mirrored
/// as it is, moved by `at` or, with None, put beside what the part cuts
/// (docs/specs/canvas-handling.md §3). Returns the project and the new number.
pub fn duplicate_instance(
    job: &Job,
    part: u64,
    like: u32,
    at: Option<[f64; 2]>,
) -> Result<(Job, u32), PipelineError> {
    let index = part_index(job, part)?;
    instance_exists(job, index, like)?;
    let Some((setup, original)) = job
        .instances_of(part)
        .into_iter()
        .find(|(_, i)| i.id == like)
        .map(|(setup, i)| (setup, *i))
    else {
        return Err(refused(
            Message::new("pipeline-no-instance")
                .arg("part", job.parts[index].name.as_str())
                .arg("id", like),
        ));
    };
    add_instance(job, part, Some(setup), at, original.angle, original.mirror)
}

/// Instance `id` of part `part` moved onto setup `setup` (docs/specs/parts-and-setups.md
/// §10), after the instances there, keeping its number and placement.
pub fn move_instance(job: &Job, part: u64, id: u32, setup: u64) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    instance_exists(job, index, id)?;
    if job.setup(setup).is_none() {
        return Err(refused(Message::new("pipeline-no-setup").arg("id", setup)));
    }
    let mut job = job.clone();
    let mut moved = None;
    for s in &mut job.setups {
        if let Some(k) = s
            .instances
            .iter()
            .position(|i| i.part == part && i.id == id)
        {
            moved = Some(s.instances.remove(k));
        }
    }
    if let (Some(instance), Some(s)) = (moved, job.setup_mut(setup)) {
        s.instances.push(instance);
    }
    checked(job)
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
    part_index(job, part)?;
    let centre = job.part_centre(part).unwrap_or((0.0, 0.0));
    let ((x0, y0), (x1, y1), all_x1) = match planned_boxes(job, part) {
        Some(boxes) => boxes,
        None => {
            let (low, high) = job
                .part_box(part)
                .ok_or_else(|| refused(Message::new("pipeline-part-cuts-nothing")))?;
            let right = job
                .instances_of(part)
                .into_iter()
                .filter(|(_, i)| i.enabled)
                .map(|(_, i)| (i.id, i.spot()))
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
    let mut alone = job.for_setup(job.home_setup(part)?);
    for p in &mut alone.parts {
        p.enabled = p.id == part;
    }
    let drawing = load_drawing(&alone).ok()?;
    let plan = plan_job(&alone, &drawing.contours).ok()?;
    let (_, (all_x1, _)) = cut_extent(&plan.program.moves, &plan.op_spans, None)?;
    // Instances are placed relative to the part as drawn (part-placement.md decision 1).
    if alone.is_placed(part) {
        alone.set_instances_here(part, vec![Instance::drawn(part, 1)]);
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
    instance_exists(job, index, 1)?;
    let mut job = job.clone();
    if let Some(own) = job.instance_mut(part, 1) {
        if reset {
            own.at = [0.0, 0.0];
            own.angle = 0.0;
        }
        own.at = at.unwrap_or(own.at);
        own.angle = angle.unwrap_or(own.angle);
    }
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

/// Instances `ids` of part `part` changed as asked; instance 1 like any other
/// (docs/specs/parts-and-setups.md §6).
pub fn set_instances(
    job: &Job,
    part: u64,
    ids: &[u32],
    change: InstanceChange,
) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    let mut job = job.clone();
    for &id in ids {
        instance_exists(&job, index, id)?;
        let Some(instance) = job.instance_mut(part, id) else {
            continue;
        };
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

/// Part `part` without instances `ids`, instance 1 too; the others keep their numbers. A
/// part left with none is placed nowhere: not cut, kept (docs/specs/parts-and-setups.md §2).
pub fn remove_instances(job: &Job, part: u64, ids: &[u32]) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    for &id in ids {
        instance_exists(job, index, id)?;
    }
    let mut job = job.clone();
    for s in &mut job.setups {
        s.instances
            .retain(|i| i.part != part || !ids.contains(&i.id));
    }
    checked(job)
}

/// Part `part`'s instances on setup `setup` (default: the board it is cut from) made a
/// grid (docs/specs/instances.md §5, [`Job::set_grid_here`]): the grid's cells but the
/// first, in place of the instances it had there, numbered row by row from 2 and listed in
/// the order they are cut.
pub fn array_instances(
    job: &Job,
    part: u64,
    setup: Option<u64>,
    grid: Copies,
) -> Result<Job, PipelineError> {
    part_index(job, part)?;
    if let Some(problem) = grid.problems("grid").into_iter().next() {
        return Err(refused(problem.text_message));
    }
    let setup = target_setup(job, part, setup)?;
    let mut job = job.clone();
    let active = job.active_setup;
    job.active_setup = Some(setup);
    job.set_grid_here(part, &grid);
    job.active_setup = active;
    checked(job)
}

/// Part `part` cut in `order`.
pub fn set_order(job: &Job, part: u64, order: InstanceOrder) -> Result<Job, PipelineError> {
    let index = part_index(job, part)?;
    let mut job = job.clone();
    job.parts[index].order = order;
    checked(job)
}
