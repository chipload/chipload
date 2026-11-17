// SPDX-License-Identifier: GPL-3.0-or-later
//! `get`, `set` and `check-project`: a project's values by path, and its problems.

use super::*;

/// `get` (docs/specs/scripting.md §4): the value as JSON.
pub(crate) fn get(out: &mut Out, project: &Path, path: &str) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let value = match serde_json::to_value(&job) {
        Ok(value) => value,
        Err(err) => return out.fail(err),
    };
    // A part's placement is in its setup since schema 41: the old path is refused, the new
    // one named.
    if let Some(refused) = chipload_job::paths::moved(&job, path) {
        return out.fail(refused);
    }
    // `stock…` and `machine.work_zero…` are the active setup's since schema 34.
    let aliased = chipload_job::paths::alias(&job, path);
    let path = aliased.as_deref().unwrap_or(path);
    match chipload_job::paths::get(&value, path) {
        Ok(found) => {
            if out.json() {
                out.set("value", found.clone());
            } else {
                match serde_json::to_string_pretty(found) {
                    Ok(text) => out.line(text),
                    Err(err) => return out.fail(err),
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

/// `set` (§4): every value set, the result loaded again, then written; its problems are
/// listed but don't stop the write.
pub(crate) fn set(
    out: &mut Out,
    project: &Path,
    values: &[String],
    output: Option<&Path>,
) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let assignments = match values
        .iter()
        .map(|text| chipload_job::paths::assignment(text))
        .collect::<Result<Vec<_>, String>>()
    {
        Ok(assignments) => assignments,
        Err(err) => return out.fail(err),
    };
    let changed = match chipload_job::paths::set_all(&job, &assignments) {
        Ok(changed) => changed,
        Err(err) => return out.fail(err),
    };
    // An image whose settings changed is traced again (docs/specs/image-trace.md §5), a
    // changed text written again (text.md §6).
    let (changed, reports) = match chipload_pipeline::images::retrace_changed(&job, &changed) {
        Ok(traced) => traced,
        Err(err) => return out.fail(err),
    };
    let fonts = texts::computer_fonts();
    let (changed, text_reports) =
        match chipload_pipeline::texts::remake_changed(&job, &changed, &fonts) {
            Ok(written) => written,
            Err(err) => return out.fail(err),
        };
    let changed = match models::remake_changed(out, &job, changed) {
        Ok(changed) => changed,
        Err(code) => return code,
    };
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &changed, target) {
        return out.fail(err);
    }
    for (path, value) in &assignments {
        out.line(format!("Set {path} = {value}"));
    }
    for report in &reports {
        images::reported(out, report);
    }
    for report in &text_reports {
        texts::reported(out, report);
    }
    let problems = chipload_pipeline::job_problems(&changed);
    problem_lines(out, &problems);
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// `check-project` (§4): the problems the GUI shows, with their paths.
pub(crate) fn check_project(out: &mut Out, project: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let problems = chipload_pipeline::job_problems(&job);
    problem_lines(out, &problems);
    if problems.is_empty() {
        out.line("No problems.");
        ExitCode::SUCCESS
    } else {
        let count = problems.len();
        let first = problems[0].text.clone();
        out.problems(problems);
        out.err(match count {
            1 => format!("error: 1 problem: {first}"),
            _ => format!("error: {count} problems, the first: {first}"),
        });
        ExitCode::from(EXIT_FAILED)
    }
}

/// One line per problem ("problem: operations.0.rest: …"), and the list as JSON.
pub(crate) fn problem_lines(out: &mut Out, problems: &[chipload_job::Problem]) {
    for p in problems {
        out.line(format!("problem: {}: {}", p.path, p.text));
    }
    out.set(
        "problems",
        json!(
            problems
                .iter()
                .map(|p| json!({"path": p.path, "short": p.short, "text": p.text}))
                .collect::<Vec<_>>()
        ),
    );
}
