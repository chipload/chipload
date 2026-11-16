// SPDX-License-Identifier: GPL-3.0-or-later
//! A project's problems, refused before it is planned.

use chipload_i18n::Message;
use chipload_job::Job;

use crate::PipelineError;

/// Problems as one line: "path: message; path: message".
pub fn joined(problems: &[chipload_job::Problem]) -> String {
    problems
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Words of the command line or another library, as a message that shows them as they are.
pub fn foreign_text(text: impl Into<String>) -> Message {
    Message::new("pipeline-text").arg("text", text.into())
}

/// A change refused for its problems, as [`joined`] writes them; Ok without problems.
pub fn refuse_problems(problems: &[chipload_job::Problem]) -> Result<(), PipelineError> {
    let messages = problems.iter().map(|p| p.text_message.clone());
    match chipload_job::problem::semicolons(messages) {
        None => Ok(()),
        Some(message) => Err(PipelineError::Job(message)),
    }
}

/// Every problem that keeps a project from being planned and needs no geometry to find:
/// the file's value rules, then the job rules checked before planning, each with its field
/// (docs/specs/live-problems.md).
pub fn job_problems(job: &Job) -> Vec<chipload_job::Problem> {
    let mut problems = job.validate();
    problems.extend(chipload_toolpath::job_problems(job));
    problems.extend(units_problem(job));
    problems
}

/// The machine's G-code units, if its post can't write them (docs/specs/inch-units.md
/// §4.2). A post that can't be read is reported when the project runs.
fn units_problem(job: &Job) -> Option<chipload_job::Problem> {
    let post = crate::posts::job_post(job).ok()?;
    let units = job.machine.gcode_units;
    (!post.units.contains(&units)).then(|| {
        chipload_job::Problem::at(
            "machine.gcode_units",
            Message::new("pipeline-post-units")
                .arg("post", post.name.as_str())
                .arg("units", units.name()),
        )
    })
}
