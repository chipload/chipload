// SPDX-License-Identifier: GPL-3.0-or-later
//! Combining closed shapes: union, difference, intersection.

use chipload_draw::offset::Chain;
use chipload_i18n::Message;
use chipload_job::{Geometry, Job, Shape};

use crate::PipelineError;
use crate::shapes::{delete_shape, reshape};

use super::trim::join_group_of;
use super::{Edited, add_all, chain_of, drawn, polyline_of, refused};
pub use chipload_draw::boolean::Combine;
use chipload_draw::boolean::{self, BooleanError};

/// Combine closed drawn shapes `ids` (two or more; for a subtraction the first is cut by the
/// others). A result that is one of the shapes unchanged keeps it as drawn, with its id;
/// the first other result takes the first shape's id (when that shape isn't kept as it
/// was), the rest are new shapes on its layer. The shapes not kept are removed, and the
/// operations that used them are flagged.
pub fn combine_shapes(job: &Job, op: Combine, ids: &[u64]) -> Result<Edited, PipelineError> {
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            return Err(refused(Message::new("pipeline-shape-twice").arg("id", *id)));
        }
    }
    let shapes: Vec<Shape> = ids
        .iter()
        .map(|&id| drawn(job, id).cloned())
        .collect::<Result<_, _>>()?;
    let chains: Vec<Chain> = shapes.iter().map(chain_of).collect();
    let first = shapes.first().map(|s| s.id).unwrap_or_default();
    let pieces = boolean::combine(op, &chains).map_err(|e| match e {
        BooleanError::TooFew => refused(Message::new("pipeline-combine-two")),
        BooleanError::Open(i) => refused(Message::new("pipeline-combine-open").arg("id", ids[i])),
        BooleanError::Empty => refused(
            Message::new("pipeline-combine-empty")
                .arg("id", first)
                .arg("op", combine_word(op)),
        ),
        BooleanError::Library => refused(
            Message::new("pipeline-combine-failed").arg("why", "the boolean library failed"),
        ),
        BooleanError::Unverified(why) => {
            refused(Message::new("pipeline-combine-failed").arg("why", why))
        }
    })?;
    let kept: Vec<u64> = pieces
        .iter()
        .filter_map(|p| p.from)
        .map(|i| ids[i])
        .collect();
    let mut changed = pieces.iter().filter(|p| p.from.is_none()).map(|p| &p.chain);
    let mut out = job.clone();
    let mut survivors = kept.clone();
    if !kept.contains(&first)
        && let Some(chain) = changed.next()
    {
        out = reshape(&out, first, polyline_of(chain))?;
        survivors.push(first);
    }
    let mut new_ids = Vec::new();
    let mut notes = Vec::new();
    let rest: Vec<Geometry> = changed.map(polyline_of).collect();
    if !rest.is_empty() {
        out = add_all(out, &shapes[0], rest, &mut new_ids, &mut notes)?;
    }
    join_group_of(&mut out, first, &new_ids);
    for &id in ids.iter().filter(|id| !survivors.contains(id)) {
        let (next, _) = delete_shape(&out, id)?;
        out = next;
        notes.extend(gone_note(job, id));
    }
    Ok(Edited {
        job: out,
        new_ids,
        notes,
        joined: None,
    })
}

fn combine_word(op: Combine) -> &'static str {
    match op {
        Combine::Union => "union",
        Combine::Subtract => "subtract",
        Combine::Intersect => "intersect",
    }
}

/// A note naming the operations that selected shape `id`, which is gone: they are flagged
/// until their geometry is chosen again.
fn gone_note(job: &Job, id: u64) -> Option<Message> {
    let ops: Vec<String> = job
        .operations
        .iter()
        .enumerate()
        .filter(|(_, op)| op.select().shapes.as_ref().is_some_and(|s| s.contains(&id)))
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    (!ops.is_empty()).then(|| {
        Message::new("pipeline-shape-gone")
            .arg("id", id)
            .arg("operations", ops.join(", "))
            .arg("ops", ops.len())
    })
}

// -- arrays (§6)
