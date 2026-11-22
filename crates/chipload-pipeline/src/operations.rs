// SPDX-License-Identifier: GPL-3.0-or-later
//! Operations removed, duplicated and moved in the cutting order: the same rules for the
//! window and the command line (`op remove`, `op duplicate`, `op move`).

use chipload_i18n::{Language, Message, Units};
use chipload_job::{Job, Operation};

use crate::PipelineError;
use crate::passes::no_operation;

/// Operation `index` (0-based), or the refusal naming it 1-based.
fn at(job: &Job, index: usize) -> Result<&Operation, PipelineError> {
    job.operations
        .get(index)
        .ok_or_else(|| no_operation(index + 1, job.operations.len()))
}

/// Operation `index` taken out; the project and the operation.
pub fn remove_operation(job: &Job, index: usize) -> Result<(Job, Operation), PipelineError> {
    at(job, index)?;
    let mut out = job.clone();
    let removed = out.operations.remove(index);
    Ok((out, removed))
}

/// A copy's name: "Profile 1 (copy)".
pub fn copy_name(name: &str, lang: Language) -> String {
    Message::new("pipeline-name-copy")
        .arg("name", name)
        .render(lang, Units::Mm)
}

/// Operation `index` copied right after itself, in the same part; the project and the
/// copy's index.
pub fn duplicate_operation(
    job: &Job,
    index: usize,
    lang: Language,
) -> Result<(Job, usize), PipelineError> {
    let mut copy = at(job, index)?.clone();
    let name = copy_name(copy.name(), lang);
    *copy.name_mut() = name;
    let mut out = job.clone();
    out.operations.insert(index + 1, copy);
    Ok((out, index + 1))
}

/// Whether operation `from` can go to `to` in the cutting order: both exist, they differ,
/// and neither is a surface of the whole board, which stays first in its setup.
pub fn can_move(job: &Job, from: usize, to: usize) -> Result<(), PipelineError> {
    let moved = at(job, from)?;
    let target = at(job, to)?;
    if from == to {
        return Err(PipelineError::Job(
            Message::new("pipeline-op-move-same").arg("number", from + 1),
        ));
    }
    if moved.is_board_surface() || target.is_board_surface() {
        return Err(PipelineError::Job(Message::new(
            "pipeline-op-board-surface",
        )));
    }
    Ok(())
}

/// Operation `from` moved to place `to` in the cutting order, joining the part of the
/// operation that was there.
pub fn move_operation(job: &Job, from: usize, to: usize) -> Result<Job, PipelineError> {
    can_move(job, from, to)?;
    let part = job.operations[to].part();
    let mut out = job.clone();
    let mut moved = out.operations.remove(from);
    moved.set_part(part);
    out.operations.insert(to, moved);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> Job {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/corpus/jobs/v5/drawn_part.chipload"
        );
        chipload_job::load_job(std::path::Path::new(path)).unwrap()
    }

    fn names(job: &Job) -> Vec<String> {
        job.operations
            .iter()
            .map(|o| o.name().to_string())
            .collect()
    }

    #[test]
    fn operations_are_removed_duplicated_and_moved() {
        let job = job();
        let before = names(&job);
        assert!(before.len() >= 2, "{before:?}");
        let (copied, at) = duplicate_operation(&job, 0, Language::En).unwrap();
        assert_eq!(at, 1);
        assert_eq!(names(&copied)[1], format!("{} (copy)", before[0]));
        let moved = move_operation(&copied, 1, 0).unwrap();
        assert_eq!(names(&moved)[0], format!("{} (copy)", before[0]));
        let (removed, op) = remove_operation(&moved, 0).unwrap();
        assert_eq!(op.name(), format!("{} (copy)", before[0]));
        assert_eq!(names(&removed), before);
        let count = job.operations.len();
        assert!(remove_operation(&job, count).is_err());
        assert!(move_operation(&job, 0, 0).is_err());
        assert!(duplicate_operation(&job, count, Language::En).is_err());
    }
}
