// SPDX-License-Identifier: GPL-3.0-or-later
//! Parts (docs/specs/parts.md): add, change, remove and activate them, and move operations
//! between them. Every change keeps the operations grouped by part, in the parts' order,
//! so the list's order stays the cutting order.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_i18n::Message;
use chipload_job::{Job, Mirror, Operation, Part};
use chipload_toolpath::select::{select_contours, select_paths};

use crate::PipelineError;

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn index_of(job: &Job, id: u64) -> Result<usize, PipelineError> {
    job.parts
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| refused(Message::new("pipeline-no-part").arg("id", id)))
}

/// A name no part has: `wanted`, or "Part N", N one more than the parts there are (the
/// next free one above that).
pub fn free_name(job: &Job, wanted: Option<&str>) -> Result<String, PipelineError> {
    let taken = |name: &str| {
        job.parts
            .iter()
            .any(|p| p.name.trim().eq_ignore_ascii_case(name.trim()))
    };
    if let Some(name) = wanted {
        if name.trim().is_empty() {
            return Err(refused(Message::new("pipeline-part-name-empty")));
        }
        if taken(name) {
            return Err(refused(
                Message::new("pipeline-part-name-taken").arg("name", name.trim()),
            ));
        }
        return Ok(name.trim().to_string());
    }
    Ok((job.parts.len() + 1..)
        .map(|n| format!("Part {n}"))
        .find(|name| !taken(name))
        .unwrap_or_default())
}

/// Stable-sort the operations by their part's place in `parts`: after reordering parts.
fn group_operations(job: &mut Job) {
    let order: Vec<u64> = job.parts.iter().map(|p| p.id).collect();
    job.operations.sort_by_key(|op| {
        order
            .iter()
            .position(|id| *id == op.part())
            .unwrap_or(usize::MAX)
    });
}

/// Refuse a result the project's rules don't allow.
fn checked(job: Job) -> Result<Job, PipelineError> {
    match job.validate().into_iter().next() {
        Some(problem) => Err(refused(problem.text_message)),
        None => Ok(job),
    }
}

/// A new, empty part at the end, which becomes the active part. Returns its id.
pub fn add_part(job: &Job, name: Option<&str>) -> Result<(Job, u64), PipelineError> {
    let mut job = job.clone();
    let id = job.parts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
    let name = free_name(&job, name)?;
    // In the active setup (docs/specs/setups.md §7).
    let setup = job.active_setup_id().unwrap_or(1);
    job.parts.push(Part {
        id,
        name,
        enabled: true,
        order: chipload_job::InstanceOrder::ByOperation,
        mirror: None,
        turns: None,
        stock: None,
    });
    // Once, where it is drawn (docs/specs/parts-and-setups.md §2).
    job.place_part(id, setup);
    job.active_part = Some(id);
    Ok((checked(job)?, id))
}

/// Rename a part, or turn it on or off.
pub fn set_part(
    job: &Job,
    id: u64,
    name: Option<&str>,
    enabled: Option<bool>,
) -> Result<Job, PipelineError> {
    let mut job = job.clone();
    let at = index_of(&job, id)?;
    if let Some(name) = name {
        let mut others = job.clone();
        others.parts.remove(at);
        job.parts[at].name = free_name(&others, Some(name))?;
    }
    if let Some(enabled) = enabled {
        job.parts[at].enabled = enabled;
    }
    checked(job)
}

/// The project with part `id` turned as `turns` allows when nested
/// (docs/specs/nesting.md §3).
pub fn set_part_turns(
    job: &Job,
    id: u64,
    turns: chipload_job::Turns,
) -> Result<Job, PipelineError> {
    let mut job = job.clone();
    let at = index_of(&job, id)?;
    job.parts[at].turns = Some(turns);
    checked(job)
}

/// The project with part `id` cut mirrored, or as drawn with None
/// (docs/specs/mirror-part.md §6).
pub fn set_part_mirror(
    job: &Job,
    id: u64,
    mirror: Option<chipload_job::Mirror>,
) -> Result<Job, PipelineError> {
    let mut job = job.clone();
    let at = index_of(&job, id)?;
    job.parts[at].mirror = mirror;
    checked(job)
}

/// Where mirrored parts are cut (docs/specs/mirror-part.md §5): the contours each mirrored
/// part's operations cut, mirrored about its box's centre line. For drawing them faded
/// where the cut is; the drawing stays as drawn. Its instances are [`instance_outlines`].
pub fn mirrored_outlines(job: &Job, contours: &[Contour]) -> Vec<Vec<Point>> {
    let mut out = Vec::new();
    for part in job.parts.iter().filter(|p| p.mirror.is_some()) {
        let mut ids: Vec<usize> = Vec::new();
        let mut line = None;
        for op in job.operations.iter().filter(|op| op.part() == part.id) {
            let Some(mirror) = job.mirror_of(op) else {
                continue;
            };
            line = Some(mirror);
            let select = op.select().resolved(&job.drawing);
            let label = op.label_message(0);
            let picked = match op {
                Operation::Path(_) | Operation::Holes(_) => select_paths(contours, &select, &label),
                _ => select_contours(contours, &select, &label),
            };
            if let Ok(selection) = picked {
                ids.extend(selection.contours.iter().map(|c| c.id));
            }
        }
        let Some((mirror, at)) = line else {
            continue;
        };
        let flip = |p: Point| match mirror {
            Mirror::LeftRight => (2.0 * at - p.0, p.1),
            Mirror::TopBottom => (p.0, 2.0 * at - p.1),
        };
        for contour in contours.iter().filter(|c| ids.contains(&c.id)) {
            let points: Vec<Point> = contour.points().into_iter().map(flip).collect();
            out.push(points);
        }
    }
    out
}

/// One instance of a part where it is cut (docs/specs/instances.md §4): what the window
/// draws as its ghost and picks it by.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceOutline {
    pub part: u64,
    pub id: u32,
    pub enabled: bool,
    /// The point it is mirrored and turned about, where it lies on the instance: its turn
    /// handle goes round it.
    pub centre: Point,
    /// The contours the part's operations cut, mirrored with the part if it is, then put
    /// where the instance lies; each with whether it is closed.
    pub outlines: Vec<(Vec<Point>, bool)>,
}

/// Every instance of every part (on or off), where it is cut, the part itself (number 1)
/// first when it is placed elsewhere than drawn (docs/specs/part-placement.md §4). An
/// instance only moved is its outlines moved by its shift exactly; a turned or mirrored one
/// is placed about the part's centre.
pub fn instance_outlines(job: &Job, contours: &[Contour]) -> Vec<InstanceOutline> {
    instance_outlines_showing(job, contours, &[])
}

/// [`instance_outlines`], the part itself also for parts `shown` when it is cut where drawn
/// (the window shows it on request, to place it: docs/specs/part-placement.md §6).
pub fn instance_outlines_showing(
    job: &Job,
    contours: &[Contour],
    shown: &[u64],
) -> Vec<InstanceOutline> {
    let mut out = Vec::new();
    let own_shown = |p: &chipload_job::Part| job.is_placed(p.id) || shown.contains(&p.id);
    let listed = |p: &chipload_job::Part| {
        job.instances_of(p.id)
            .into_iter()
            .filter(|(_, i)| i.id != 1 || own_shown(p))
            .map(|(_, i)| (i.id, i.enabled, i.spot()))
            .collect::<Vec<_>>()
    };
    for part in job.parts.iter().filter(|p| !listed(p).is_empty()) {
        let centre = job.part_centre(part.id).unwrap_or((0.0, 0.0));
        let mut line = None;
        let mut ids: Vec<usize> = Vec::new();
        for op in job
            .operations
            .iter()
            .filter(|op| op.part() == part.id && op.selects_shapes())
        {
            line = line.or(job.mirror_of(op));
            let select = op.select().resolved(&job.drawing);
            let label = op.label_message(0);
            let picked = match op {
                Operation::Path(_) | Operation::Holes(_) => select_paths(contours, &select, &label),
                _ => select_contours(contours, &select, &label),
            };
            if let Ok(selection) = picked {
                ids.extend(selection.contours.iter().map(|c| c.id));
            }
        }
        let flip = |p: Point| match line {
            Some((Mirror::LeftRight, at)) => (2.0 * at - p.0, p.1),
            Some((Mirror::TopBottom, at)) => (p.0, 2.0 * at - p.1),
            None => p,
        };
        let drawn: Vec<(Vec<Point>, bool)> = contours
            .iter()
            .filter(|c| ids.contains(&c.id))
            .map(|c| (c.points().into_iter().map(flip).collect(), c.closed))
            .collect();
        // The part itself (number 1) is on with its part: it can't be off alone
        // (docs/specs/part-placement.md decision 3).
        for (id, enabled, spot) in listed(part) {
            let enabled = if id == 1 { part.enabled } else { enabled };
            let place = chipload_toolpath::place::Placement::new(&spot, centre);
            let (dx, dy) = (spot.at[0], spot.at[1]);
            let put = |p: Point| {
                if spot.is_move() {
                    (p.0 + dx, p.1 + dy)
                } else {
                    place.apply(p)
                }
            };
            out.push(InstanceOutline {
                part: part.id,
                id,
                enabled,
                centre: put(centre),
                outlines: drawn
                    .iter()
                    .map(|(points, closed)| (points.iter().map(|&p| put(p)).collect(), *closed))
                    .collect(),
            });
        }
    }
    out
}

/// The outlines of every instance on setup `setup` that is on, where it is cut, with
/// whether each is closed: the setup's sheet beside the active board
/// (docs/specs/parts-and-setups.md §7).
pub fn setup_outlines(job: &Job, contours: &[Contour], setup: u64) -> Vec<(Vec<Point>, bool)> {
    let Some(board) = job.setup(setup) else {
        return Vec::new();
    };
    let on: Vec<(u64, u32)> = board
        .instances
        .iter()
        .filter(|i| i.enabled && job.part(i.part).is_some_and(|p| p.enabled))
        .map(|i| (i.part, i.id))
        .collect();
    let parts: Vec<u64> = on.iter().map(|(p, _)| *p).collect();
    instance_outlines_showing(job, contours, &parts)
        .into_iter()
        .filter(|i| on.contains(&(i.part, i.id)))
        .flat_map(|i| i.outlines)
        .collect()
}

/// Where turned and mirrored instances that are on are cut, with whether each outline is
/// closed: what the setup sheet draws besides the shifts of [`crate::sheet::contour_copies`].
pub fn placed_outlines(job: &Job, contours: &[Contour]) -> Vec<(Vec<Point>, bool)> {
    // Only the active setup's: the sheet is one board's.
    let turned = |part: u64, id: u32| {
        job.spots(part)
            .into_iter()
            .find(|(n, _)| *n == id)
            .is_some_and(|(_, spot)| !spot.is_move())
    };
    instance_outlines(job, contours)
        .into_iter()
        .filter(|i| i.enabled && turned(i.part, i.id))
        .flat_map(|i| i.outlines)
        .collect()
}

/// The project with only the parts `ids` cut, every other one switched off: what exporting
/// those parts writes (docs/specs/part-export.md §2). A part that is off but named is cut.
/// The parts' names, in the project's order, for the status and the setup sheet.
pub fn only_parts(job: &Job, ids: &[u64]) -> Result<(Job, Vec<String>), PipelineError> {
    for &id in ids {
        index_of(job, id)?;
    }
    let mut job = job.clone();
    let mut names = Vec::new();
    for part in &mut job.parts {
        part.enabled = ids.contains(&part.id);
        if part.enabled {
            names.push(part.name.clone());
        }
    }
    Ok((job, names))
}

/// Make part `id` the one new operations go into.
pub fn activate_part(job: &Job, id: u64) -> Result<Job, PipelineError> {
    index_of(job, id)?;
    let mut job = job.clone();
    job.active_part = Some(id);
    Ok(job)
}

/// Remove a part. Refused for the last part, and while it has operations unless
/// `with_operations`, which removes them too. Returns what was done, for the report.
pub fn remove_part(
    job: &Job,
    id: u64,
    with_operations: bool,
) -> Result<(Job, Vec<String>), PipelineError> {
    let at = index_of(job, id)?;
    if job.parts.len() == 1 {
        return Err(refused(Message::new("pipeline-last-part")));
    }
    let name = job.parts[at].name.clone();
    let operations: Vec<usize> = (0..job.operations.len())
        .filter(|i| job.operations[*i].part() == id)
        .collect();
    if !operations.is_empty() && !with_operations {
        return Err(refused(
            Message::new("pipeline-part-has-operations")
                .arg("name", name)
                .arg("count", operations.len()),
        ));
    }
    let mut job = job.clone();
    job.parts.remove(at);
    job.remove_part_instances(id);
    job.operations.retain(|op| op.part() != id);
    if job.active_part == Some(id) {
        job.active_part = job.parts.first().map(|p| p.id);
    }
    let mut lines = vec![format!("Removed part {name}")];
    if !operations.is_empty() {
        let numbers: Vec<String> = operations.iter().map(|i| (i + 1).to_string()).collect();
        lines.push(format!(
            "Removed its operations {}; the operations after them are numbered again",
            numbers.join(", ")
        ));
    }
    Ok((checked(job)?, lines))
}

/// Move operation `index` (0-based) into part `id`, at the end of that part's operations;
/// a cut through the stock then goes through that part's board. Returns its new index.
pub fn move_operation_to_part(
    job: &Job,
    index: usize,
    id: u64,
) -> Result<(Job, usize), PipelineError> {
    index_of(job, id)?;
    let mut job = job.clone();
    if index >= job.operations.len() {
        return Err(refused(
            Message::new("pipeline-part-no-operation").arg("number", index + 1),
        ));
    }
    let mut op = job.operations.remove(index);
    op.set_part(id);
    let at = job.insert_index(id);
    job.operations.insert(at, op);
    // A cut through the stock ends at the thickness of its new part's board.
    job.follow_thickness();
    Ok((checked(job)?, at))
}

/// Move part `id` one place earlier (`up`) or later, with its operations.
pub fn move_part(job: &Job, id: u64, up: bool) -> Result<Job, PipelineError> {
    let at = index_of(job, id)?;
    let to = if up { at.checked_sub(1) } else { Some(at + 1) };
    let Some(to) = to.filter(|t| *t < job.parts.len()) else {
        return Ok(job.clone());
    };
    let mut job = job.clone();
    job.parts.swap(at, to);
    group_operations(&mut job);
    checked(job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::{migrations::migrate, parse_job};
    use serde_json::json;

    /// A v21 project with three profiles, migrated: one part holding them.
    fn job() -> Job {
        let op = |layer: &str| {
            json!({"type": "profile", "select": {"layer": layer}, "side": "outside",
                   "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 3,
                   "clearance": 0, "step_down": 3, "feed": 1000, "plunge_feed": 300})
        };
        parse_job(
            migrate(json!({
                "schema_version": 21,
                "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
                "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                          "max_overcut": 0.5},
                "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw",
                            "tool_change": "files"},
                "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
                "operations": [op("A"), op("B"), op("C")],
            }))
            .unwrap(),
            "test",
        )
        .unwrap()
    }

    #[test]
    fn a_new_part_is_active_and_operations_move_into_it_grouped() {
        let job = job();
        assert_eq!(job.parts.len(), 1);
        assert_eq!(job.parts[0].name, "Part 1");
        let (job, id) = add_part(&job, None).unwrap();
        assert_eq!((id, job.parts[1].name.as_str()), (2, "Part 2"));
        assert_eq!(job.active_part_id(), Some(2));
        // Operation 1 moves to the end, into part 2; the list stays grouped.
        let (job, at) = move_operation_to_part(&job, 0, 2).unwrap();
        assert_eq!(at, 2);
        let parts: Vec<u64> = job.operations.iter().map(|op| op.part()).collect();
        assert_eq!(parts, [1, 1, 2]);
        // Part 2 first: its operation comes first.
        let job = move_part(&job, 2, true).unwrap();
        let parts: Vec<u64> = job.operations.iter().map(|op| op.part()).collect();
        assert_eq!(parts, [2, 1, 1]);
        assert!(job.validate().is_empty(), "{:?}", job.validate());
    }

    #[test]
    fn names_are_unique_and_the_last_part_stays() {
        let job = job();
        let (job, _) = add_part(&job, Some("lid")).unwrap();
        assert!(add_part(&job, Some("LID ")).is_err());
        assert!(set_part(&job, 1, Some("lid"), None).is_err());
        let job = set_part(&job, 2, Some("Lid"), Some(false)).unwrap();
        assert!(!job.parts[1].enabled);
        // Part 1 has operations: refused, unless they go too.
        assert!(remove_part(&job, 1, false).is_err());
        let (job, lines) = remove_part(&job, 1, true).unwrap();
        assert!(job.operations.is_empty(), "{lines:?}");
        assert!(remove_part(&job, 2, true).is_err(), "the last part");
    }
}
