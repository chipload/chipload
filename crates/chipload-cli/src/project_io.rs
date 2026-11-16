// SPDX-License-Identifier: GPL-3.0-or-later
//! Projects read and written by the commands: imported, created, updated, loaded with
//! `--set` values, saved.

use super::*;

pub(crate) fn is_dxf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dxf"))
}

/// A drawing file's size when it doesn't give one: a DXF's units, an SVG's pixels per inch.
pub(crate) type SizeGiven = (Option<Units>, Option<f64>);

pub(crate) fn import(
    out: &mut Out,
    dxf: &Path,
    (units, px_per_inch): SizeGiven,
    output: &Path,
    start: &StartArgs,
    rules: bool,
) -> ExitCode {
    let imported = match shapes_from_file(dxf, units, px_per_inch, 1.0) {
        Ok(imported) => imported,
        Err(err) => return out.fail(err),
    };
    let line = format!(
        "Imported {} ({} shapes).",
        imported.source.file,
        imported.shapes.len()
    );
    let code = create(out, new_project(imported), &line, output, start, rules);
    if out.json() && code == ExitCode::SUCCESS {
        // The import report as `inspect --json` gives it for the new project.
        if let Ok(result) = load_job(output)
            .map_err(|e| e.to_string())
            .and_then(|job| load_drawing(&job).map_err(|e| e.to_string()))
        {
            out.set("report", import_json(&result));
        }
    }
    code
}

/// Write a new project, started from the library and the options (docs/specs/
/// new-project-dialog.md §5).
/// A new project from `job`, started with the library's choices; with `rules`, the
/// styles' layer rules applied when any style has one (docs/specs/fewer-clicks.md §4).
pub(crate) fn create(
    out: &mut Out,
    mut job: Job,
    line: &str,
    output: &Path,
    start: &StartArgs,
    rules: bool,
) -> ExitCode {
    // The library only gives starting values; without a settings folder there is none.
    let library = settings_dir()
        .map(|dir| Library::load(&dir))
        .unwrap_or_default();
    let begin = start.start(&job);
    let notes = match chipload_pipeline::library::start_project(&mut job, &library, &begin) {
        Ok(notes) => notes,
        Err(err) => return out.fail(err.english()),
    };
    out.line(line.to_string());
    let has_rules = library.styles.styles.iter().any(|s| s.layer.is_some());
    if rules
        && has_rules
        && let Err(err) = styles::apply_rules(out, &mut job, &library.styles.styles, &library.tools)
    {
        return out.fail(err);
    }
    if let Err(err) = crate::save_project(out, &job, output) {
        return out.fail(err);
    }
    for problem in &library.problems {
        out.err(format!("warning: {}", problem.english()));
    }
    for note in &library.converted {
        out.line(note.english());
    }
    for note in notes {
        out.line(note.english());
    }
    out.wrote(output, "");
    ExitCode::SUCCESS
}

pub(crate) fn update(
    out: &mut Out,
    project: &Path,
    from: &Path,
    (units, px_per_inch): SizeGiven,
    dxf: Option<u64>,
    output: Option<&Path>,
) -> ExitCode {
    let job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let updated = match dxf {
        Some(id) => {
            chipload_pipeline::dxfs::update_drawing_file(&job, id, from, units, px_per_inch)
        }
        None => update_drawing_with(&job, from, units, px_per_inch),
    };
    let (updated, report) = match updated {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &updated, target) {
        return out.fail(err);
    }
    for line in &report.lines {
        out.line(line.to_string());
    }
    out.wrote(target, "");
    out.set("unchanged", json!(report.unchanged));
    out.set("changed", json!(report.changed));
    out.set("added", json!(report.added));
    out.set("removed", json!(report.removed));
    out.set("lost", json!(report.lost));
    if report.lost.is_empty() {
        ExitCode::SUCCESS
    } else {
        out.err(format!(
            "warning: {} operation(s) lost geometry and can't run until fixed",
            report.lost.len()
        ));
        ExitCode::from(EXIT_FAILED)
    }
}

/// Save `job` to `target`, its live frames drawn again round what they go round first
/// (docs/specs/sign-maker.md §4), with a line for each one let go of. Every command that
/// writes a project saves through here.
pub(crate) fn save_project(
    out: &mut Out,
    job: &Job,
    target: &Path,
) -> Result<(), chipload_job::JobError> {
    let (job, notes) = chipload_pipeline::frames::follow_frames(job.clone());
    for note in notes {
        out.line(note.english());
    }
    save_job(&job, target)
}

/// Save `job` to `target` and say so, or fail with why it couldn't be saved.
pub(crate) fn write_project(out: &mut Out, job: &Job, target: &Path) -> ExitCode {
    if let Err(err) = crate::save_project(out, job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

pub(crate) fn load(out: &mut Out, project: &Path) -> Result<Job, ExitCode> {
    load_job(project).map_err(|e| out.fail(e))
}

/// Load a project with `--set` values applied for this run (docs/specs/scripting.md §4).
pub(crate) fn load_set(out: &mut Out, project: &Path, sets: &[String]) -> Result<Job, ExitCode> {
    let job = load(out, project)?;
    if sets.is_empty() {
        return Ok(job);
    }
    let assignments = sets
        .iter()
        .map(|text| chipload_job::paths::assignment(text))
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| out.fail(e))?;
    let changed = chipload_job::paths::set_all(&job, &assignments).map_err(|e| out.fail(e))?;
    // An image whose settings changed is traced again (docs/specs/image-trace.md §5), a
    // changed text written again (text.md §6).
    let (traced, reports) =
        chipload_pipeline::images::retrace_changed(&job, &changed).map_err(|e| out.fail(e))?;
    let (written, text_reports) =
        chipload_pipeline::texts::remake_changed(&job, &traced, &texts::computer_fonts())
            .map_err(|e| out.fail(e))?;
    for line in reports
        .iter()
        .flat_map(|r| &r.lines)
        .chain(text_reports.iter().flat_map(|r| &r.lines))
    {
        out.line(line.clone());
    }
    // A changed model's outline drawn again (docs/specs/stl-relief.md §4).
    let written = models::remake_changed(out, &job, written)?;
    // Frames round a changed text drawn again (docs/specs/sign-maker.md §4).
    let (followed, notes) = chipload_pipeline::frames::follow_frames(written);
    for note in notes {
        out.line(note.english());
    }
    Ok(followed)
}

/// `load_set`, then only the parts `parts` cut when any are given
/// (docs/specs/part-export.md); with them their names.
pub(crate) fn load_parts(
    out: &mut Out,
    project: &Path,
    sets: &[String],
    parts: &[u64],
) -> Result<(Job, Vec<String>), ExitCode> {
    let job = load_set(out, project, sets)?;
    if parts.is_empty() {
        return Ok((job, Vec::new()));
    }
    chipload_pipeline::parts::only_parts(&job, parts).map_err(|e| out.fail(e))
}

/// Load a project and run it, as `run` and `sheet` do; errors are printed. With `parts`,
/// only those parts, and their names.
pub(crate) fn load_and_run(
    out: &mut Out,
    project: &Path,
    tool_change: Option<library::ToolChangeArg>,
    sets: &[String],
    parts: &[u64],
) -> Result<(Job, RunResult, Vec<String>), ExitCode> {
    let (mut job, names) = load_parts(out, project, sets, parts)?;
    if let Some(change) = tool_change {
        job.machine.tool_change = change.value();
    }
    match run_job(&job) {
        Ok(result) => Ok((job, result, names)),
        Err(err @ PipelineError::CheckFailed { .. }) => {
            out.err(format!("error: {err}"));
            out.err("No output written.");
            Err(ExitCode::from(EXIT_FAILED))
        }
        Err(err) => {
            out.err(format!("error: {err}"));
            out.problems(chipload_pipeline::job_problems(&job));
            Err(ExitCode::from(EXIT_FAILED))
        }
    }
}

/// `sets` with `active_setup=ID` when a setup is chosen (docs/specs/setups.md §8).
pub(crate) fn with_setup(mut sets: Vec<String>, setup: Option<u64>) -> Vec<String> {
    if let Some(id) = setup {
        sets.push(format!("active_setup={id}"));
    }
    sets
}
