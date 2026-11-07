// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload run`, `plan`, `sheet`, `simulate`, `render` and `check`: a project planned, cut,
//! simulated and checked, and the JSON they answer with.

use super::*;

/// What the sheet says it was made from: the project's file name and this version; its
/// lengths in `units`, its words in `language`.
pub(crate) fn sheet_info(
    project: &Path,
    (units, language): (Units, Language),
    parts: Vec<String>,
) -> SheetInfo {
    SheetInfo {
        project: project.file_name().map_or_else(
            || project.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        written_by: format!("chipload {}", env!("CARGO_PKG_VERSION")),
        units,
        language,
        parts,
    }
}

pub(crate) fn sheet(
    out: &mut Out,
    project: &Path,
    output: &Path,
    gcode: Option<&Path>,
    tool_change: Option<library::ToolChangeArg>,
    words: (Units, Language),
    (sets, parts): (&[String], &[u64]),
) -> ExitCode {
    let (job, result, names) = match load_and_run(out, project, tool_change, sets, parts) {
        Ok(run) => run,
        Err(code) => return code,
    };
    let gcode = gcode.map_or_else(
        || project.with_extension(result.post.file_extension.trim_start_matches('.')),
        Path::to_path_buf,
    );
    let html = setup_sheet(&job, &result, &gcode, &sheet_info(project, words, names));
    if let Err(err) = write_atomically(output, &html) {
        out.err(format!("error: cannot write {}: {err}", output.display()));
        return ExitCode::from(EXIT_FAILED);
    }
    out.wrote(output, "");
    ExitCode::SUCCESS
}

/// `simulate`'s options.
pub(crate) struct SimulateArgs {
    pub(crate) cell: Option<f64>,
    pub(crate) after: Option<usize>,
    pub(crate) image: bool,
    pub(crate) hash: bool,
}

pub(crate) fn simulate(
    out: &mut Out,
    project: &Path,
    output: Option<&Path>,
    args: SimulateArgs,
    sets: &[String],
) -> ExitCode {
    let SimulateArgs {
        cell,
        after,
        image,
        hash,
    } = args;
    let (job, result, _) = match load_and_run(out, project, None, sets, &[]) {
        Ok(run) => run,
        Err(code) => return code,
    };
    let mut program = result.program.clone();
    if let Some(done) = after {
        let total = program.moves.len();
        if done > total {
            out.err(format!(
                "error: --after {done}: the program has {total} moves"
            ));
            return ExitCode::from(EXIT_FAILED);
        }
        program.moves.truncate(done);
        out.line(format!("After {done} of {total} moves"));
    }
    let sim = match chipload_pipeline::simulate::simulate(
        &job,
        &program,
        &result.groups,
        cell,
        &chipload_pipeline::Cancel::new(),
    ) {
        Ok(sim) => sim,
        Err(err) => {
            out.err(format!("error: {err}"));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    for line in sim.lines() {
        out.line(line.to_string());
    }
    if hash {
        out.line(format!("Heights hash: {:016x}", sim.heights_hash()));
    }
    out.set(
        "numbers",
        json!({
            "removed_cm3": sim.numbers.removed_cm3,
            "deepest_z": sim.numbers.deepest,
            "below_bottom_mm": sim.numbers.below_bottom,
            "drill_point_assumed": sim.numbers.drill_point_assumed,
            "vbit_angle_assumed": sim.numbers.vbit_angle_assumed,
            "cell_mm": sim.cell,
            "columns": sim.columns,
            "rows": sim.rows,
        }),
    );
    out.set("image", Value::Null);
    if image {
        let path = output.map_or_else(
            || {
                let stem = project
                    .file_stem()
                    .map_or_else(|| "part".into(), |s| s.to_string_lossy().into_owned());
                project.with_file_name(format!("{stem}-sim.png"))
            },
            Path::to_path_buf,
        );
        let bytes = match sim.png(chipload_pipeline::simulate::PRINT_PALETTE) {
            Ok(bytes) => bytes,
            Err(err) => {
                out.err(format!("error: {err}"));
                return ExitCode::from(EXIT_FAILED);
            }
        };
        if let Err(err) = std::fs::write(&path, bytes) {
            out.err(format!("error: cannot write {}: {err}", path.display()));
            return ExitCode::from(EXIT_FAILED);
        }
        out.wrote(&path, "");
        let shown = std::path::absolute(&path).unwrap_or(path);
        out.set("image", json!(shown.display().to_string()));
    }
    ExitCode::SUCCESS
}

pub(crate) fn run(
    out: &mut Out,
    project: &Path,
    output: &Path,
    tool_change: Option<library::ToolChangeArg>,
    sheet: Option<(Units, Language)>,
    sets: &[String],
    (parts, setup): (&[u64], Option<u64>),
) -> ExitCode {
    let (mut job, names) = match load_parts(out, project, sets, parts) {
        Ok(loaded) => loaded,
        Err(code) => return code,
    };
    if let Some(change) = tool_change {
        job.machine.tool_change = change.value();
    }
    // Every setup is a program of its own (docs/specs/setups.md §4).
    let runs = match chipload_pipeline::run_setups(&job, setup) {
        Ok(runs) => runs,
        Err(err @ PipelineError::CheckFailed { .. }) => {
            out.err(format!("error: {err}"));
            out.err("No output written.");
            return ExitCode::from(EXIT_FAILED);
        }
        Err(err) => {
            out.err(format!("error: {err}"));
            out.problems(chipload_pipeline::job_problems(&job));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let several = job.setups.len() > 1;
    let mut files = Vec::new();
    let mut sheets = Vec::new();
    for (index, (id, result)) in runs.iter().enumerate() {
        let one = job.for_setup(*id);
        if several {
            let name = one.setup(*id).map_or("", |s| s.name.as_str());
            out.line(format!("Setup {id} ({name}):"));
        }
        // The drawing's facts are the same for every setup: said once.
        if index == 0 {
            for line in format_import_report(&result.import_report) {
                out.line(line.to_string());
            }
        }
        for note in &result.notes {
            out.line(format!("warning: {note}"));
        }
        for fact in &result.facts {
            out.line(format!("note: {fact}"));
        }
        for line in floor_hint_lines(&result.floor_hints) {
            out.line(line);
        }
        for warning in &result.check_warnings {
            out.line(format!("check warning: {}", warning.message));
        }
        for note in &result.check_notes {
            out.line(format!("check note: {}", note.message));
        }
        out.line(format!("Work zero: {}", one.work_zero_text()));
        if !names.is_empty() {
            out.line(format!("Only: {}", names.join(", ")));
        }
        if let Err(err) = write_gcode_files(output, &result.files) {
            out.err(format!("error: cannot write {}: {err}", output.display()));
            return ExitCode::from(EXIT_FAILED);
        }
        for file in &result.files {
            let lines = file.gcode.matches('\n').count();
            let path = file.path(output);
            if file.suffix.is_some() {
                let detail = format!(
                    " ({lines} lines, tool {}, operations {})",
                    file.tool,
                    chipload_post::operations_text(&file.operations)
                );
                out.wrote(&path, &detail);
            } else {
                out.wrote(
                    &path,
                    &format!(" ({lines} lines, post {})", result.post.name),
                );
            }
            files.push(json!({
                "path": std::path::absolute(&path).unwrap_or(path).display().to_string(),
                "setup": id,
                "tool": file.tool,
                "operations": operations_json(&file.operations),
                "rest_passes": rest_passes_json(&file.operations),
            }));
        }
        for line in estimate_lines(
            &one,
            &result.estimate,
            result.files.len() > 1,
            Lengths::Totals,
        ) {
            out.line(line.to_string());
        }
        if result.files.len() > 1 {
            out.line(format!(
                "{} files, post {}: change the tool and set Z zero between them, keeping X \
                 and Y zero.",
                result.files.len(),
                result.post.name
            ));
        }
        if let Some(words) = sheet {
            // Each setup's sheet beside its files: out-S2-sheet.html.
            let info = sheet_info(project, words, names.clone());
            match write_setup_sheet(&one, result, output, &info) {
                Ok(path) => {
                    out.wrote(&path, " (setup sheet)");
                    let shown = std::path::absolute(&path).unwrap_or(path);
                    sheets.push(json!(shown.display().to_string()));
                }
                Err(err) => {
                    out.err(format!(
                        "error: cannot write the setup sheet {}: {err}",
                        chipload_pipeline::sheet::setup_sheet_path(&one, output).display()
                    ));
                    return ExitCode::from(EXIT_FAILED);
                }
            }
        }
    }
    out.set("files", json!(files));
    if let [(_, result)] = runs.as_slice() {
        out.set("estimate", estimate_json(&job, &result.estimate));
        out.set("sheet", sheets.first().cloned().unwrap_or(Value::Null));
    } else {
        out.set(
            "setups",
            json!(
                runs.iter()
                    .map(|(id, r)| json!({
                        "setup": id,
                        "estimate": estimate_json(&job, &r.estimate),
                    }))
                    .collect::<Vec<_>>()
            ),
        );
        out.set("estimate", Value::Null);
        out.set("sheet", json!(sheets));
    }
    ExitCode::SUCCESS
}

pub(crate) fn render(out: &mut Out, program_path: &Path, post_name: &str) -> ExitCode {
    let parsed = std::fs::read_to_string(program_path)
        .map_err(|e| format!("cannot read {}: {e}", program_path.display()))
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()))
        .and_then(|value| program_from_json(&value));
    let program = match parsed {
        Ok(program) => program,
        Err(err) => {
            out.err(format!("error: {err}"));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    match load_builtin_post(post_name).and_then(|post| render_program(&program, &post)) {
        Ok(text) => {
            out.text("gcode", &text);
            ExitCode::SUCCESS
        }
        Err(err) => {
            out.err(format!("error: {err}"));
            ExitCode::from(EXIT_FAILED)
        }
    }
}

pub(crate) fn program_from_json(value: &Value) -> Result<Program, String> {
    // A plan with tool changes is posted by `run`, which writes each tool's part.
    if let Some(groups) = value.get("tool_groups").and_then(Value::as_u64)
        && groups > 1
    {
        return Err(format!("this plan uses {groups} tool groups; use run"));
    }
    let number = |v: &Value, key: &str| -> Result<f64, String> {
        v.get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("program.{key} must be a number"))
    };
    let optional = |v: &Value, key: &str| -> Result<Option<f64>, String> {
        match v.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(x) => x
                .as_f64()
                .map(Some)
                .ok_or_else(|| format!("move.{key} must be a number or null")),
        }
    };
    let spindle = match value.get("spindle").and_then(Value::as_str) {
        Some("cw") => Spindle::Cw,
        Some("ccw") => Spindle::Ccw,
        _ => return Err("program.spindle must be \"cw\" or \"ccw\"".into()),
    };
    let moves = value
        .get("moves")
        .and_then(Value::as_array)
        .ok_or("program.moves must be a list")?
        .iter()
        .map(|m| {
            let kind = match m.get("kind").and_then(Value::as_str) {
                Some("rapid") => MoveKind::Rapid,
                Some("feed") => MoveKind::Feed,
                Some("arc_cw") => MoveKind::ArcCw,
                Some("arc_ccw") => MoveKind::ArcCcw,
                _ => {
                    return Err(
                        "move.kind must be \"rapid\", \"feed\", \"arc_cw\" or \"arc_ccw\""
                            .to_string(),
                    );
                }
            };
            let circle = match m.get("circle") {
                None | Some(Value::Null) => None,
                Some(c) => {
                    let center = c
                        .get("center")
                        .and_then(Value::as_array)
                        .and_then(|xy| Some((xy.first()?.as_f64()?, xy.get(1)?.as_f64()?)))
                        .ok_or("move.circle.center must be [x, y]")?;
                    Some(Circle {
                        center,
                        radius: number(c, "radius")?,
                    })
                }
            };
            Ok(Move {
                kind,
                x: optional(m, "x")?,
                y: optional(m, "y")?,
                z: optional(m, "z")?,
                feed: optional(m, "feed")?,
                circle,
                comment: None,
            })
        })
        .collect::<Result<Vec<Move>, String>>()?;
    Ok(Program {
        tool_id: value
            .get("tool_id")
            .and_then(Value::as_i64)
            .ok_or("program.tool_id must be an integer")?,
        tool_diameter: number(value, "tool_diameter")?,
        rpm: number(value, "rpm")?,
        spindle,
        safe_z: number(value, "safe_z")?,
        moves,
        comments: Vec::new(),
    })
}

pub(crate) fn plan(out: &mut Out, project: &Path, sets: &[String], parts: &[u64]) -> ExitCode {
    let job = match load_parts(out, project, sets, parts) {
        Ok((job, _)) => job,
        Err(code) => return code,
    };
    let imported = match load_drawing(&job) {
        Ok(result) => result,
        Err(err) => return out.fail(err),
    };
    let plan = match plan_job(&job, &imported.contours) {
        Ok(plan) => plan,
        Err(err) => {
            out.problems(chipload_pipeline::job_problems(&job));
            return out.fail(err);
        }
    };
    // What was planned: the active setup, its parts and their operations
    // (docs/specs/setups.md §8).
    let active = job.active_setup_id();
    let planned = active.map_or_else(|| job.clone(), |id| job.for_setup(id));
    // The post's spindle wait; a post that can't be read only leaves it out here (`run`
    // refuses it).
    let dwell = chipload_pipeline::posts::job_post(&job).map_or(0.0, |p| p.spindle_dwell_s);
    let estimate = estimate(&job, &plan.program, &plan.op_spans, &plan.groups, dwell);
    if out.json() {
        for note in &plan.notes {
            out.line(note.text.clone());
        }
        out.set_all(json!({
            "program": program_with_groups(&plan.program, plan.groups.len()),
            "groups": plan.groups.iter().map(|g| json!({
                "tool": g.tool.id,
                "operations": operations_json(&g.operations),
                "rest_passes": rest_passes_json(&g.operations),
                "moves": [g.moves.start, g.moves.end],
            })).collect::<Vec<_>>(),
            "tabs": plan.tabs,
            "pockets": plan.pockets.iter().map(|p| json!({
                "contour_ids": p.contour_ids,
                "rings": p.rings.iter().map(|r| &r.points).collect::<Vec<_>>(),
                "open": p.rings.iter().map(|r| r.open).collect::<Vec<_>>(),
                "feed_links": p.feed_links,
            })).collect::<Vec<_>>(),
            "ring_starts": plan.ring_starts.iter().map(|s| json!({
                "operation": s.operation,
                "start": s.ring.first(),
                "stored": s.stored,
            })).collect::<Vec<_>>(),
            "estimate": estimate_json(&job, &estimate),
            "facts": plan.facts.iter().map(|n| json!({
                "operation": n.operation,
                "text": n.text,
            })).collect::<Vec<_>>(),
            "floor_hints": plan.floor_hints.iter().map(|&(operation, largest)| json!({
                "operation": operation,
                "largest_mm": largest,
            })).collect::<Vec<_>>(),
            "chip_load": planned.operations.iter().enumerate()
                .filter(|(i, _)| planned.cuts(*i))
                .map(|(i, op)| json!({
                    "operation": i,
                    "chip_load_mm": chip_load(&planned, op).mm(),
                })).collect::<Vec<_>>(),
        }));
    } else {
        for note in &plan.notes {
            out.line(format!("warning: {note}"));
        }
        for fact in &plan.facts {
            out.line(format!("note: {fact}"));
        }
        for line in floor_hint_lines(&plan.floor_hints) {
            out.line(line);
        }
        for start in &plan.ring_starts {
            let Some((x, y)) = start.ring.first() else {
                continue;
            };
            let from = match start.stored {
                Some(k) => format!("start point {}", k + 1),
                None => "default".into(),
            };
            out.line(format!(
                "Operation {}: a ring starts at ({x:.3}, {y:.3}) ({from})",
                start.operation + 1
            ));
        }
        if job.setups.len() > 1
            && let Some(setup) = active.and_then(|id| job.setup(id))
        {
            out.line(format!(
                "Setup {} ({}) of {}: the other setups are planned on their own (--setup N)",
                setup.id,
                setup.name,
                job.setups.len()
            ));
        }
        if let Some(text) = copies_text(&job) {
            out.line(format!("Copies: {text}"));
        }
        // Numbered as in the project.
        for (line, part) in parts_lines(&job).into_iter().zip(&job.parts) {
            if job.setups.len() < 2 || Some(part.setup) == active {
                out.line(line);
            }
        }
        for line in estimate_lines(
            &job,
            &estimate,
            plan.groups.len() > 1,
            Lengths::PerOperation,
        ) {
            out.line(line.to_string());
        }
        for line in chip_load_lines(&planned) {
            out.line(line.to_string());
        }
        if plan.groups.len() > 1 {
            for (i, g) in plan.groups.iter().enumerate() {
                out.line(format!(
                    "Tool group {}: tool {}, operations {}",
                    i + 1,
                    g.tool.id,
                    chipload_post::operations_text(&g.operations)
                ));
            }
        }
        out.line(format!("Planned {} moves.", plan.program.moves.len()));
    }
    ExitCode::SUCCESS
}

/// The cycle time estimate as `plan` and `run` print it with `--json`.
pub(crate) fn estimate_json(
    job: &chipload_job::Job,
    estimate: &chipload_pipeline::estimate::Estimate,
) -> Value {
    let mut value = json!({
        "total_s": estimate.total,
        "cutting_s": estimate.cutting,
        "rapid_s": estimate.rapid,
        "spindle_s": estimate.spindle,
        "operations": estimate.operation_counts.iter().map(|(at, c)| {
            let mut op = json!({
                "operation": at.operation,
                "pass": at.pass,
                "seconds": c.seconds(),
                "cutting_s": c.cutting_s,
                "rapid_s": c.rapid_s,
            });
            add_counts(&mut op, c);
            op
        }).collect::<Vec<_>>(),
        // Each part the run planned (docs/specs/part-statistics.md §3.2).
        "parts": job.parts.iter().filter_map(|part| {
            let counted = chipload_pipeline::estimate::part_counts(job, estimate, part.id)?;
            let c = &counted.counts;
            let mut value = json!({
                "id": part.id,
                "name": part.name,
                "operations": counted.operations,
                "seconds": c.seconds(),
                "cutting_s": c.cutting_s,
                "rapid_s": c.rapid_s,
            });
            add_counts(&mut value, c);
            Some(value)
        }).collect::<Vec<_>>(),
        "groups_s": estimate.groups,
        "tool_changes": estimate.tool_changes,
        "rapid_mm_min": estimate.rapid_rate,
        "rapid_assumed": estimate.rapid_assumed,
    });
    add_counts(&mut value, &estimate.counts);
    value
}

/// An estimate's lengths (mm) and lifts beside its seconds
/// (docs/specs/operation-statistics.md §3.3).
fn add_counts(value: &mut Value, counts: &chipload_pipeline::estimate::Counts) {
    value["cutting_mm"] = json!(counts.cutting_mm);
    value["plunge_mm"] = json!(counts.plunge_mm);
    value["plunge_s"] = json!(counts.plunge_s);
    value["rapid_mm"] = json!(counts.rapid_mm);
    value["lifts"] = json!(counts.lifts);
}

/// The program as `render` reads it, with how many tool groups it has: `render` refuses a
/// plan with tool changes.
pub(crate) fn program_with_groups(program: &Program, groups: usize) -> Value {
    let mut value = program_json(program);
    value["tool_groups"] = json!(groups);
    value
}

pub(crate) fn program_json(program: &Program) -> Value {
    let moves: Vec<Value> = program
        .moves
        .iter()
        .map(|m| {
            let kind = match m.kind {
                MoveKind::Rapid => "rapid",
                MoveKind::Feed => "feed",
                MoveKind::ArcCw => "arc_cw",
                MoveKind::ArcCcw => "arc_ccw",
            };
            // On a feed: the circle the chord lies on (merged into an arc by `render` when
            // the post writes arcs). On an arc: its circle.
            let circle = m
                .circle
                .map(|c| json!({"center": [c.center.0, c.center.1], "radius": c.radius}));
            json!({"kind": kind, "x": m.x, "y": m.y, "z": m.z, "feed": m.feed, "circle": circle})
        })
        .collect();
    json!({
        "tool_id": program.tool_id,
        "tool_diameter": program.tool_diameter,
        "rpm": program.rpm,
        "spindle": match program.spindle { chipload_job::Spindle::Cw => "cw", chipload_job::Spindle::Ccw => "ccw" },
        "safe_z": program.safe_z,
        "moves": moves,
    })
}

pub(crate) fn check(out: &mut Out, gcode: &Path, job_path: &Path, tool: Option<i64>) -> ExitCode {
    let job = match load_job(job_path) {
        Ok(job) => job,
        Err(err) => {
            out.err(format!("error: {err}"));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let text = match std::fs::read_to_string(gcode) {
        Ok(text) => text,
        Err(err) => {
            out.err(format!("error: cannot read {}: {err}", gcode.display()));
            return ExitCode::from(EXIT_FAILED);
        }
    };
    let limits = match tool {
        None => limits_from_job(&job),
        Some(id) => match limits_for_tool(&job, id) {
            Some(limits) => limits,
            None => return out.fail(format!("the project has no tool {id}")),
        },
    };
    let report = check_gcode(&text, &limits);
    if out.json() {
        let issues = |list: &[CheckIssue]| -> Vec<serde_json::Value> {
            list.iter()
                .map(|i| json!({"line": i.line, "message": i.message}))
                .collect()
        };
        out.set_all(json!({
            "ok": report.ok(),
            "errors": issues(&report.errors),
            "warnings": issues(&report.warnings),
            "facts": issues(&report.notes),
        }));
    } else {
        for e in &report.errors {
            out.line(format!("error (line {}): {}", e.line, e.message));
        }
        for w in &report.warnings {
            out.line(format!("warning: {}", w.message));
        }
        for n in &report.notes {
            out.line(format!("note: {}", n.message));
        }
        out.line(
            (if report.ok() {
                "Check passed."
            } else {
                "Check FAILED."
            })
            .to_string(),
        );
    }
    if report.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_FAILED)
    }
}

/// For each V-carve whose floor pass leaves part of the floor, the library's end mills that
/// would clear it all (docs/specs/vcarve-wide-strokes.md §4.1, decision 8). Nothing when
/// none fits or there is no library.
pub(crate) fn floor_hint_lines(hints: &[(usize, f64)]) -> Vec<String> {
    if hints.is_empty() {
        return Vec::new();
    }
    let library = settings_dir()
        .map(|dir| Library::load(&dir))
        .unwrap_or_default();
    hints
        .iter()
        .filter_map(|&(operation, largest)| {
            let fitting = chipload_pipeline::fitting_end_mills(&library.tools.tools, largest);
            (!fitting.is_empty()).then(|| {
                let names: Vec<String> = fitting
                    .iter()
                    .map(|t| format!("{} ({} mm)", t.name, chipload_core::text::mm(t.diameter)))
                    .collect();
                format!(
                    "Operation {}: end mills in your library that clear the floor: {}",
                    operation + 1,
                    names.join(", ")
                )
            })
        })
        .collect()
}
