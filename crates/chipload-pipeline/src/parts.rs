// SPDX-License-Identifier: GPL-3.0-or-later
//! Parts (docs/specs/parts.md): add, change, remove and activate them, and move operations
//! between them. Every change keeps the operations grouped by part, in the parts' order,
//! so the list's order stays the cutting order.

use chipload_job::{Job, Part};

use crate::PipelineError;

fn refused(text: String) -> PipelineError {
    PipelineError::Job(text)
}

fn index_of(job: &Job, id: u64) -> Result<usize, PipelineError> {
    job.parts
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| refused(format!("No part {id} in the project")))
}

/// A name no part has: `wanted`, or "Part N" for the lowest free N.
fn free_name(job: &Job, wanted: Option<&str>) -> Result<String, PipelineError> {
    let taken = |name: &str| {
        job.parts
            .iter()
            .any(|p| p.name.trim().eq_ignore_ascii_case(name.trim()))
    };
    if let Some(name) = wanted {
        if name.trim().is_empty() {
            return Err(refused("A part's name can't be empty".into()));
        }
        if taken(name) {
            return Err(refused(format!(
                "Another part is named \"{}\"",
                name.trim()
            )));
        }
        return Ok(name.trim().to_string());
    }
    Ok((1..)
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
        Some(problem) => Err(refused(problem)),
        None => Ok(job),
    }
}

/// A new, empty part at the end, which becomes the active part. Returns its id.
pub fn add_part(job: &Job, name: Option<&str>) -> Result<(Job, u64), PipelineError> {
    let mut job = job.clone();
    let id = job.parts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
    let name = free_name(&job, name)?;
    job.parts.push(Part {
        id,
        name,
        enabled: true,
        copies: None,
    });
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
        return Err(refused(
            "A project needs at least one part: this is the last one".into(),
        ));
    }
    let name = job.parts[at].name.clone();
    let operations: Vec<usize> = (0..job.operations.len())
        .filter(|i| job.operations[*i].part() == id)
        .collect();
    if !operations.is_empty() && !with_operations {
        return Err(refused(format!(
            "Part {name} has {} operation{}: move them to another part first, or remove \
             them with it",
            operations.len(),
            if operations.len() == 1 { "" } else { "s" }
        )));
    }
    let mut job = job.clone();
    job.parts.remove(at);
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

/// Move operation `index` (0-based) into part `id`, at the end of that part's operations.
/// Returns its new index.
pub fn move_operation_to_part(
    job: &Job,
    index: usize,
    id: u64,
) -> Result<(Job, usize), PipelineError> {
    index_of(job, id)?;
    let mut job = job.clone();
    if index >= job.operations.len() {
        return Err(refused(format!(
            "No operation {} in the project",
            index + 1
        )));
    }
    let mut op = job.operations.remove(index);
    op.set_part(id);
    let at = job.insert_index(id);
    job.operations.insert(at, op);
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
