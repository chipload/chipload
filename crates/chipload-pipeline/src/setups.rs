// SPDX-License-Identifier: GPL-3.0-or-later
//! Setups (docs/specs/setups.md): add, change, remove and activate them, and move parts
//! between them. A setup is one board clamped once: its stock, its work zero and the parts
//! cut from it, each run as a program of its own.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{BackOf, Geometry, Job, Note, Origin, Part, Register, Setup, Turn};

use crate::PipelineError;
use crate::parts::free_name;

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn index_of(job: &Job, id: u64) -> Result<usize, PipelineError> {
    job.setups
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| refused(Message::new("pipeline-no-setup").arg("id", id)))
}

/// Refuse a result the project's rules don't allow.
fn checked(job: Job) -> Result<Job, PipelineError> {
    match job.validate().into_iter().next() {
        Some(problem) => Err(refused(problem.text_message)),
        None => Ok(job),
    }
}

/// A new setup at the end, which becomes the active one, with a first part of its own, the
/// active part: a board like the active setup's (its stock and zero), `thickness` thick
/// when given. Returns its id and its part's.
pub fn add_setup(
    job: &Job,
    name: Option<&str>,
    thickness: Option<f64>,
) -> Result<(Job, u64, u64), PipelineError> {
    let mut job = job.clone();
    let id = job.setups.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let name = match name {
        Some(name) if name.trim().is_empty() => {
            return Err(refused(Message::new("pipeline-setup-name-empty")));
        }
        Some(name) => name.trim().to_string(),
        None => format!("Setup {}", job.setups.len() + 1),
    };
    let mut stock = job.stock().clone();
    if let Some(thickness) = thickness {
        stock.thickness = thickness;
    }
    let work_zero = job.work_zero();
    job.setups.push(Setup {
        id,
        name,
        enabled: true,
        stock,
        work_zero,
        back_of: None,
    });
    let part = job.parts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
    let part_name = free_name(&job, None)?;
    job.parts.push(Part {
        id: part,
        name: part_name,
        enabled: true,
        copies: None,
        setup: id,
        mirror: None,
    });
    job.active_setup = Some(id);
    job.active_part = Some(part);
    Ok((checked(job)?, id, part))
}

/// A new setup that is the back of setup `front`, the same board turned over `turn`
/// (docs/specs/two-sided.md §3), right after it: named "<front> back" unless `name` is
/// given, located by pins, with a first part of its own; both become active. Refused for a front that is
/// itself a back or already has one. Returns its id and its part's.
pub fn add_back_side(
    job: &Job,
    front: u64,
    turn: Turn,
    name: Option<&str>,
) -> Result<(Job, u64, u64), PipelineError> {
    let at = index_of(job, front)?;
    let name = match name {
        Some(name) => name.to_string(),
        None => format!("{} back", job.setups[at].name),
    };
    let (mut job, id, part) = add_setup(job, Some(&name), None)?;
    // Right after its front: cut, listed and numbered in the files after it.
    if let Some(new) = job.setups.pop() {
        job.setups.insert(at + 1, new);
    }
    set_back_of(
        &mut job,
        id,
        Some(BackOf {
            setup: front,
            turn,
            register: Register::Pins,
        }),
    )?;
    Ok((checked(job)?, id, part))
}

/// What to change of a setup's board (docs/specs/two-sided.md §9): make it the back of
/// another setup (`Some(Some(front))`) or give it a board of its own (`Some(None)`), and
/// how a back is turned over and located.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BoardChange {
    pub back_of: Option<Option<u64>>,
    pub turn: Option<Turn>,
    pub register: Option<Register>,
}

/// Change setup `id`'s board. A back keeps its turn and its way of locating unless they
/// are given (new: left to right, pins); a setup unlinked keeps the board and zero it had
/// as a back. A turn or a way of locating for a setup that isn't a back is refused.
pub fn set_board(job: &Job, id: u64, change: BoardChange) -> Result<Job, PipelineError> {
    let mut job = job.clone();
    let at = index_of(&job, id)?;
    let old = job.setups[at].back_of;
    let back = match change.back_of {
        Some(None) => None,
        Some(Some(front)) => Some(BackOf {
            setup: front,
            turn: change
                .turn
                .or(old.map(|b| b.turn))
                .unwrap_or(Turn::LeftRight),
            register: change
                .register
                .or(old.map(|b| b.register))
                .unwrap_or(Register::Pins),
        }),
        None => match old {
            Some(old) => Some(BackOf {
                turn: change.turn.unwrap_or(old.turn),
                register: change.register.unwrap_or(old.register),
                ..old
            }),
            None if change.turn.is_some() || change.register.is_some() => {
                return Err(refused(
                    Message::new("pipeline-setup-not-a-back").arg("id", id),
                ));
            }
            None => None,
        },
    };
    set_back_of(&mut job, id, back)?;
    checked(job)
}

/// Set setup `id`'s link, then let every back follow its front's board and zero.
fn set_back_of(job: &mut Job, id: u64, back: Option<BackOf>) -> Result<(), PipelineError> {
    let at = index_of(job, id)?;
    job.setups[at].back_of = back;
    job.follow_thickness();
    Ok(())
}

/// Rename a setup, or turn it on or off.
pub fn set_setup(
    job: &Job,
    id: u64,
    name: Option<&str>,
    enabled: Option<bool>,
) -> Result<Job, PipelineError> {
    let mut job = job.clone();
    let at = index_of(&job, id)?;
    if let Some(name) = name {
        if name.trim().is_empty() {
            return Err(refused(Message::new("pipeline-setup-name-empty")));
        }
        job.setups[at].name = name.trim().to_string();
    }
    if let Some(enabled) = enabled {
        job.setups[at].enabled = enabled;
    }
    checked(job)
}

/// Make setup `id` the active one: shown, simulated and previewed; new parts go into it,
/// and its first part becomes the active part.
pub fn activate_setup(job: &Job, id: u64) -> Result<Job, PipelineError> {
    index_of(job, id)?;
    let mut job = job.clone();
    job.active_setup = Some(id);
    let in_it = |part: Option<u64>| {
        part.and_then(|p| job.part(p))
            .is_some_and(|p| p.setup == id)
    };
    if !in_it(job.active_part) {
        job.active_part = job.parts.iter().find(|p| p.setup == id).map(|p| p.id);
    }
    Ok(job)
}

/// Remove a setup. Refused for the last one, and while it has parts unless `with_parts`,
/// which removes them and their operations too. Returns what was done, for the report.
pub fn remove_setup(
    job: &Job,
    id: u64,
    with_parts: bool,
) -> Result<(Job, Vec<String>), PipelineError> {
    let at = index_of(job, id)?;
    if job.setups.len() == 1 {
        return Err(refused(Message::new("pipeline-last-setup")));
    }
    let name = job.setups[at].name.clone();
    let parts: Vec<u64> = job
        .parts
        .iter()
        .filter(|p| p.setup == id)
        .map(|p| p.id)
        .collect();
    if !parts.is_empty() && !with_parts {
        return Err(refused(
            Message::new("pipeline-setup-has-parts")
                .arg("name", name)
                .arg("count", parts.len()),
        ));
    }
    let mut job = job.clone();
    let mut lines = vec![format!("Removed setup {name}")];
    // Its back keeps the board and zero it had: a copy, now its own.
    if let Some(back) = job
        .setups
        .iter_mut()
        .find(|s| s.back_of.is_some_and(|b| b.setup == id))
    {
        back.back_of = None;
        lines.push(format!("{} has a board of its own now", back.name));
    }
    job.setups.remove(at);
    job.parts.retain(|p| p.setup != id);
    job.operations.retain(|op| !parts.contains(&op.part()));
    if job.active_setup == Some(id) {
        job.active_setup = job.setups.first().map(|s| s.id);
    }
    if job.active_part.is_some_and(|p| parts.contains(&p)) {
        job.active_part = job.parts.first().map(|p| p.id);
    }
    if !parts.is_empty() {
        lines.push(format!(
            "Removed its {} part(s) and their operations",
            parts.len()
        ));
    }
    if job.parts.is_empty() {
        return Err(refused(Message::new("pipeline-last-part")));
    }
    Ok((checked(job)?, lines))
}

/// Move part `part` into setup `id`: it is cut there, from that board, with its zero.
pub fn move_part_to_setup(job: &Job, part: u64, id: u64) -> Result<Job, PipelineError> {
    index_of(job, id)?;
    let mut job = job.clone();
    let Some(p) = job.parts.iter_mut().find(|p| p.id == part) else {
        return Err(refused(Message::new("pipeline-no-part").arg("id", part)));
    };
    p.setup = id;
    // Its cuts through the stock go through its new board.
    job.follow_thickness();
    checked(job)
}

/// For a back setup with a surface pass: a warning on its deepest that it goes over the
/// dowel pins, which must be set at least as deep below the back face
/// (docs/specs/two-sided.md §6). The checker allows that pass over them, and no deeper.
pub fn surface_over_pins_warning(job: &Job) -> Option<Note> {
    let (i, depth) = job.surface_over_pins()?;
    let front = job.active_setup_id().and_then(|id| job.front_of(id))?.id;
    let pins = job.operations.iter().any(|op| {
        op.is_dowels() && op.enabled() && job.setup_of(op).is_some_and(|s| s.id == front)
    });
    if !pins {
        return None;
    }
    let op = job.operations.get(i)?;
    Some(Note::at(
        i,
        Message::new("pipeline-surface-over-pins")
            .arg("label", op.label_message(i))
            .arg("depth", Value::Length(depth, Digits::Fixed(1))),
    ))
}

/// For the active setup, when it is a back: each operation that carves a text's letters,
/// once per text (docs/specs/two-sided.md §4, decision 9). The text reads right from the
/// front, where it is drawn, so on the back face it reads mirrored. That may be meant (a
/// stamp), so it is a plain note, and nothing is mirrored for the user. A mirrored part's
/// text reads right on the back: the two mirrors cancel (docs/specs/mirror-part.md §4).
pub fn back_text_notes(job: &Job) -> Vec<Note> {
    if job.back_turn().is_none() {
        return Vec::new();
    }
    let mut notes = Vec::new();
    for (i, op) in job.operations.iter().enumerate() {
        if !job.cuts(i) || !op.selects_shapes() || job.mirror_of(op).is_some() {
            continue;
        }
        for text in texts_carved(job, op) {
            notes.push(Note::at(
                i,
                Message::new("pipeline-back-text")
                    .arg("label", op.label_message(i))
                    .arg("text", crate::texts::text_label(text)),
            ));
        }
    }
    notes
}

/// For each mirrored part on a front, its first operation that carves a text's letters,
/// once per text (docs/specs/mirror-part.md §4): the text reads backwards on the board.
/// Often meant (a plug, a stamp), so a plain note.
pub fn mirrored_text_notes(job: &Job) -> Vec<Note> {
    if job.back_turn().is_some() {
        return Vec::new();
    }
    let mut notes = Vec::new();
    let mut said: Vec<(u64, u64)> = Vec::new();
    for (i, op) in job.operations.iter().enumerate() {
        if !job.cuts(i) || job.mirror_of(op).is_none() {
            continue;
        }
        let Some(part) = job.part_of(op) else {
            continue;
        };
        for text in texts_carved(job, op) {
            if said.contains(&(part.id, text.id)) {
                continue;
            }
            said.push((part.id, text.id));
            notes.push(Note::at(
                i,
                Message::new("pipeline-mirrored-text")
                    .arg("part", part.name.as_str())
                    .arg("text", crate::texts::text_label(text)),
            ));
        }
    }
    notes
}

/// The texts whose letters operation `op` may carve, by id.
fn texts_carved<'a>(job: &'a Job, op: &chipload_job::Operation) -> Vec<&'a chipload_job::Text> {
    let select = op.select().resolved(&job.drawing);
    let mut ids: Vec<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|shape| select.may_pick(shape))
        .filter_map(|shape| match &shape.geometry {
            Geometry::Path {
                origin: Some(Origin::Text(id)),
                ..
            } => Some(*id),
            _ => None,
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids.iter()
        .filter_map(|id| job.drawing.texts.iter().find(|t| t.id == *id))
        .collect()
}
