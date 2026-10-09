// SPDX-License-Identifier: GPL-3.0-or-later
//! End-to-end rules of run_job.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Job, Operation, Spindle, load_job, parse_job};
use chipload_pipeline::{
    PipelineError, RunResult, embed_drawing, load_drawing, new_project, run_job, update_drawing,
};
use chipload_toolpath::ToolpathError;
use chipload_toolpath::model::MoveKind;
use serde_json::{Value, json};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn simple() -> PathBuf {
    corpus("dxf/simple_part.dxf")
}

fn messy_2() -> PathBuf {
    corpus("dxf/messy_2.dxf")
}

fn read(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(corpus(rel)).unwrap()).unwrap()
}

/// `base` with `changes` merged in: objects one level deep are updated, the rest replaced.
fn with(mut base: Value, changes: Value) -> Job {
    for (key, value) in changes.as_object().unwrap() {
        match (base.get_mut(key), value) {
            (Some(Value::Object(target)), Value::Object(update)) => {
                for (k, v) in update {
                    target.insert(k.clone(), v.clone());
                }
            }
            _ => {
                base[key] = value.clone();
            }
        }
    }
    parse_job(migrate(base).unwrap(), "test").unwrap()
}

/// The v2 sample job (simple_part, inside + outside profile) with changes.
fn job(changes: Value) -> Job {
    with(read("jobs/v2/simple_part_full.json"), changes)
}

/// The sample's outside profile with changes.
fn op(changes: Value) -> Value {
    let mut base = read("jobs/v2/simple_part_full.json")["operations"][1].clone();
    for (k, v) in changes.as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}

fn z_values(gcode: &str) -> Vec<f64> {
    gcode
        .lines()
        .flat_map(str::split_whitespace)
        .filter_map(|w| w.strip_prefix('Z'))
        .map(|z| z.parse().unwrap())
        .collect()
}

fn min(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn max(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// Embed `dxf` into a job made from the old-style samples (as `chipload import --into`).
fn embed(job: &Job, dxf: &Path) -> Result<Job, PipelineError> {
    Ok(embed_drawing(job, dxf)?.0)
}

fn run(job: &Job, dxf: &Path) -> RunResult {
    run_job(&embed(job, dxf).unwrap()).unwrap()
}

fn error(job: &Job, dxf: &Path) -> PipelineError {
    match embed(job, dxf).and_then(|j| run_job(&j)) {
        Ok(_) => panic!("expected an error"),
        Err(err) => err,
    }
}

fn job_error(job: &Job, dxf: &Path, text: &str) {
    let err = error(job, dxf);
    assert!(
        matches!(&err, PipelineError::Toolpath(ToolpathError::Job(m)) if m.contains(text)),
        "{err}"
    );
}

#[test]
fn sample_runs_and_passes_check() {
    let result = run(&job(json!({})), &simple());
    assert!(result.check_warnings.is_empty());
    assert_eq!(min(&z_values(&result.gcode)), -12.0);
}

#[test]
fn depth_equal_to_thickness_is_allowed() {
    let result = run(
        &job(json!({"operations": [op(json!({"depth": 12}))]})),
        &simple(),
    );
    assert_eq!(min(&z_values(&result.gcode)), -12.0);
}

#[test]
fn depth_beyond_thickness_is_refused() {
    job_error(
        &job(json!({"operations": [op(json!({"depth": 12.5}))]})),
        &simple(),
        "deeper than the stock",
    );
}

#[test]
fn z_zero_bottom_shifts_heights() {
    let result = run(
        &job(json!({"stock": {"z_zero": "bottom"}, "machine": {"safe_z": 17}})),
        &simple(),
    );
    let zs = z_values(&result.gcode);
    assert_eq!((min(&zs), max(&zs)), (0.0, 17.0));
}

#[test]
fn safe_z_must_clear_the_stock() {
    // safe_z 5 is below the top (12) when Z zero is the bottom.
    job_error(
        &job(json!({"stock": {"z_zero": "bottom"}})),
        &simple(),
        "safe_z",
    );
}

#[test]
fn tool_without_rpm_is_refused() {
    job_error(
        &job(json!({"tools": [{"id": 1, "diameter": 6}]})),
        &simple(),
        "rpm",
    );
}

#[test]
fn multiple_tools_are_refused() {
    let tools =
        json!([{"id": 1, "diameter": 6, "rpm": 18000}, {"id": 2, "diameter": 3, "rpm": 18000}]);
    let ops = json!([op(json!({})), op(json!({"tool": 2}))]);
    job_error(
        &job(json!({"tools": tools, "operations": ops})),
        &simple(),
        "one tool",
    );
}

#[test]
fn feed_above_machine_max_is_refused_not_clamped() {
    job_error(
        &job(json!({"operations": [op(json!({"feed": 6000}))]})),
        &simple(),
        "max_feed",
    );
}

#[test]
fn drill_is_not_implemented_yet() {
    let drill = json!({"type": "drill", "select": {"layer": "HOLES"}, "tool": 1});
    job_error(
        &job(json!({"operations": [drill]})),
        &simple(),
        "not implemented",
    );
}

fn selector_error(job: &Job, dxf: &Path, text: &str) {
    let err = error(job, dxf);
    assert!(
        matches!(&err, PipelineError::Toolpath(ToolpathError::Selector(m)) if m.contains(text)),
        "{err}"
    );
}

#[test]
fn selector_matching_nothing_is_an_error() {
    let ops = json!([op(json!({"select": {"layer": "NOPE"}}))]);
    selector_error(
        &job(json!({"operations": ops})),
        &simple(),
        "matches nothing",
    );
}

#[test]
fn layer_match_is_case_insensitive() {
    let ops = json!([op(json!({"select": {"layer": "cut_outside"}}))]);
    run(&job(json!({"operations": ops})), &simple());
}

#[test]
fn selection_of_only_open_contours_is_an_error() {
    let ops = json!([op(json!({"select": {"layer": "OPEN"}}))]);
    selector_error(
        &job(json!({"source": {"units": "mm"}, "operations": ops})),
        &corpus("dxf/messy.dxf"),
        "only open contours",
    );
}

#[test]
fn open_centreline_on_a_cutting_layer_is_skipped_and_reported() {
    // Real drawing: the `upper` layer holds the lid's shapes plus an open centreline (#22).
    let data = job(json!({
        "stock": {"x": 200, "y": 120, "origin": "center"},
        "operations": [op(json!({"select": {"layer": "upper"}, "side": "on"}))],
    }));
    let result = run(&data, &messy_2());
    let skipped: Vec<&String> = result
        .notes
        .iter()
        .filter(|n| n.contains("skipped open contour"))
        .collect();
    assert_eq!(
        skipped,
        [
            "Operation 1 (Outline): skipped open contour #22 on upper (64.750, 50.000) to (64.750, 20.000)"
        ]
    );
}

#[test]
fn tool_larger_than_hole_names_the_contour() {
    let tools = json!([{"id": 1, "diameter": 32, "rpm": 18000}]);
    let holes = op(json!({"select": {"layer": "HOLES"}, "side": "inside"}));
    let err = error(
        &job(json!({"tools": tools, "operations": [holes]})),
        &simple(),
    );
    assert!(
        matches!(&err, PipelineError::Toolpath(ToolpathError::Geometry(m)) if m.contains("contour #2")),
        "{err}"
    );
}

#[test]
fn holes_are_cut_before_the_outline_even_if_listed_together() {
    // One op selecting by handles across the outline and a hole: still hole first.
    // Written as text: the dxf crate's writer has the same local-time date bug as its reader.
    let (outline, hole) = ("A1", "A2");
    let mut text = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    text += "0\nSECTION\n2\nENTITIES\n";
    text += &format!("0\nLWPOLYLINE\n5\n{outline}\n8\n0\n90\n4\n70\n1\n");
    for (x, y) in [(10, 10), (110, 10), (110, 90), (10, 90)] {
        text += &format!("10\n{x}\n20\n{y}\n");
    }
    text += &format!("0\nCIRCLE\n5\n{hole}\n8\n0\n10\n60\n20\n50\n30\n0\n40\n10\n");
    text += "0\nENDSEC\n0\nEOF\n";
    let dir = std::env::temp_dir().join(format!("chipload-pipeline-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("part.dxf");
    std::fs::write(&path, text).unwrap();

    let select = json!({"handles": [outline, hole]});
    let result = run(
        &job(json!({"operations": [op(json!({"select": select, "side": "on"}))]})),
        &path,
    );
    std::fs::remove_dir_all(&dir).unwrap();
    let first = result
        .program
        .moves
        .iter()
        .find(|m| m.kind == MoveKind::Rapid && m.x.is_some())
        .unwrap();
    let x = first.x.unwrap();
    assert!(
        (50.0..=70.0).contains(&x),
        "starts at x={x}, not at the hole"
    );
}

#[test]
fn sha256_mismatch_is_refused() {
    let err = error(
        &job(json!({"source": {"sha256": "0".repeat(64)}})),
        &simple(),
    );
    assert!(
        matches!(&err, PipelineError::Job(m) if m.contains("sha256")),
        "{err}"
    );
}

#[test]
fn sha256_match_is_accepted() {
    // sha256 of tests/corpus/dxf/simple_part.dxf; a frozen corpus file never changes.
    let digest = sha256_hex(&std::fs::read(simple()).unwrap());
    run(&job(json!({"source": {"sha256": digest}})), &simple());
    run(
        &job(json!({"source": {"sha256": digest.to_uppercase()}})),
        &simple(),
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn inch_job_on_mm_drawing_is_refused() {
    let err = error(&job(json!({"source": {"units": "inch"}})), &simple());
    assert!(
        matches!(&err, PipelineError::Import(e) if e.to_string().contains("inch")),
        "{err}"
    );
}

#[test]
fn v1_sample_migrates_to_current() {
    let loaded = load_job(&corpus("jobs/v1/example_job.json")).unwrap();
    assert_eq!(loaded.schema_version, CURRENT_VERSION);
    assert_eq!(loaded.machine.spindle, Spindle::Cw);
    let Operation::Profile(profile) = &loaded.operations[0] else {
        panic!("expected a profile");
    };
    assert!((profile.plunge_feed - 2000.0 / 3.0).abs() < 1e-9);
    run(&loaded, &simple());
}

#[test]
fn checker_rejection_returns_no_gcode_but_the_toolpath() {
    // The part is 206 mm wide.
    let small = job(json!({"machine": {"travel": {"x": 100, "y": 600, "z": 100}}}));
    match error(&small, &simple()) {
        PipelineError::CheckFailed { errors, program } => {
            assert!(
                errors.iter().any(|e| e.contains("exceeds machine travel")),
                "{errors:?}"
            );
            assert!(!program.moves.is_empty());
        }
        other => panic!("expected a check failure, got {other}"),
    }
}

#[test]
fn result_exposes_toolpath_and_contours() {
    let result = run(&job(json!({})), &simple());
    assert_eq!(result.contours.len(), 4);
    assert_eq!(result.program.moves[0].kind, MoveKind::Rapid);
    assert_eq!(result.program.spindle, Spindle::Cw);
}

#[test]
fn drawing_outside_the_stock_is_noted() {
    let result = run(&job(json!({"stock": {"x": 100}})), &simple());
    assert!(
        result
            .notes
            .iter()
            .any(|n| n == "The drawing extends outside the stock")
    );
}

// -- pocket

fn lower_job(tool_changes: Value, pocket_changes: Value) -> Job {
    let mut data = read("jobs/v3/messy_2_lower.json");
    for (k, v) in tool_changes.as_object().unwrap() {
        data["tools"][0][k] = v.clone();
    }
    for (k, v) in pocket_changes.as_object().unwrap() {
        data["operations"][0][k] = v.clone();
    }
    parse_job(migrate(data).unwrap(), "test").unwrap()
}

#[test]
fn pocket_job_runs_on_the_real_drawing() {
    let result = run(&lower_job(json!({}), json!({})), &messy_2());
    assert!(result.check_warnings.is_empty());
    assert_eq!(min(&z_values(&result.gcode)), -18.0); // the outline goes through
    assert_eq!(result.pockets.len(), 3);
}

#[test]
fn pocket_refuses_a_tool_that_is_not_centre_cutting() {
    job_error(
        &lower_job(json!({"center_cutting": false}), json!({})),
        &messy_2(),
        "not centre-cutting",
    );
}

#[test]
fn pocket_warns_when_centre_cutting_is_unknown() {
    let result = run(
        &lower_job(json!({"center_cutting": null}), json!({})),
        &messy_2(),
    );
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.contains("isn't marked center_cutting"))
    );
}

#[test]
fn pocket_refuses_a_vbit() {
    job_error(
        &lower_job(json!({"type": "vbit"}), json!({})),
        &messy_2(),
        "need an endmill",
    );
}

#[test]
fn pocket_retracts_between_rings_by_default() {
    let down = run(
        &lower_job(json!({}), json!({"keep_tool_down": true})),
        &messy_2(),
    )
    .gcode;
    let retract = run(
        &lower_job(json!({}), json!({"keep_tool_down": false})),
        &messy_2(),
    )
    .gcode;
    assert!(retract.matches("G0 Z5.000").count() > down.matches("G0 Z5.000").count());
}

#[test]
fn pocket_errors_name_the_operation() {
    let err = error(&lower_job(json!({"diameter": 16}), json!({})), &messy_2());
    let text = err.to_string();
    assert!(
        text.starts_with("Operation 1 (Openings):") && text.contains("does not fit"),
        "{text}"
    );
}

// -- tabs

fn tabs_job(tab_changes: Value, op_changes: Value) -> Job {
    let mut data = read("jobs/v3/simple_part_tabs.json");
    for (k, v) in tab_changes.as_object().unwrap() {
        data["operations"][1]["tabs"][k] = v.clone();
    }
    for (k, v) in op_changes.as_object().unwrap() {
        data["operations"][1][k] = v.clone();
    }
    parse_job(migrate(data).unwrap(), "test").unwrap()
}

#[test]
fn tabs_lift_only_the_last_pass_and_are_reported_for_display() {
    let result = run(&tabs_job(json!({}), json!({})), &simple());
    assert_eq!(result.tabs.len(), 4);
    let zs = z_values(&result.gcode);
    assert!(zs.contains(&-9.0)); // tab top: cut bottom -12 + height 3
    assert_eq!(min(&zs), -12.0);
}

#[test]
fn tab_height_must_be_below_the_cut_depth() {
    job_error(
        &tabs_job(json!({"height": 12}), json!({})),
        &simple(),
        "tab height",
    );
}

#[test]
fn too_many_tabs_names_the_contour() {
    let err = error(&tabs_job(json!({"count": 60}), json!({})), &simple());
    let text = err.to_string();
    assert!(
        text.contains("contour #1") && text.contains("don't fit"),
        "{text}"
    );
}

#[test]
fn tabs_on_an_inside_profile_warn() {
    let holes = json!({"select": {"layer": "HOLES"}, "side": "inside"});
    let result = run(&tabs_job(json!({"count": 1, "width": 3}), holes), &simple());
    assert!(result.notes.iter().any(|n| n.contains("inside profile")));
    // The round hole has no straight stretch for its tab.
    assert!(result.notes.iter().any(|n| n.contains("corner or curve")));
}

// -- enabled / disabled operations (docs/specs/job-editor.md §2)

#[test]
fn disabled_operations_are_skipped_and_keep_their_numbers() {
    let all = run(&job(json!({})), &simple());
    let hole_only = run(
        &job(json!({"operations": [
            read("jobs/v2/simple_part_full.json")["operations"][0].clone(),
            op(json!({"enabled": false})),
        ]})),
        &simple(),
    );
    assert!(hole_only.program.moves.len() < all.program.moves.len());
    // The second (outline) operation is gone: nothing cuts at the outline's corner x = 47.
    let tool_r = 3.0;
    assert!(
        hole_only
            .program
            .moves
            .iter()
            .all(|m| m.x.is_none_or(|x| x > 50.0 - tool_r))
    );
    let err = error(
        &job(json!({"operations": [
            op(json!({"enabled": false})),
            op(json!({"select": {"layer": "NOPE"}})),
        ]})),
        &simple(),
    );
    assert!(err.to_string().starts_with("Operation 2"), "{err}");
}

#[test]
fn every_operation_disabled_is_an_error() {
    job_error(
        &job(json!({"operations": [op(json!({"enabled": false}))]})),
        &simple(),
        "nothing to cut",
    );
}

#[test]
fn a_disabled_drill_does_not_block_the_job() {
    let drill = json!({"type": "drill", "select": {"layer": "HOLES"}, "tool": 1, "enabled": false});
    run(
        &job(json!({"operations": [drill, op(json!({}))]})),
        &simple(),
    );
}

// -- projects (docs/specs/project-file.md)

/// A DXF (text) with closed LWPOLYLINE squares: (handle, layer, x, y, size).
fn square_dxf(path: &Path, squares: &[(&str, &str, f64, f64, f64)]) {
    let mut text = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    text += "0\nSECTION\n2\nENTITIES\n";
    for (handle, layer, x, y, size) in squares {
        text += &format!("0\nLWPOLYLINE\n5\n{handle}\n8\n{layer}\n90\n4\n70\n1\n");
        for (px, py) in [
            (x, y),
            (&(x + size), y),
            (&(x + size), &(y + size)),
            (x, &(y + size)),
        ] {
            text += &format!("10\n{px}\n20\n{py}\n");
        }
    }
    text += "0\nENDSEC\n0\nEOF\n";
    std::fs::write(path, text).unwrap();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-project-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A project of `dxf` with one outside profile on shape 1.
fn project_with_profile(dxf: &Path) -> Job {
    let mut job = new_project(chipload_dxf::shapes_from_dxf(dxf, None).unwrap());
    let mut data = serde_json::to_value(&job).unwrap();
    data["stock"]["x"] = json!(300);
    data["stock"]["y"] = json!(300);
    data["operations"] = json!([op(
        json!({"select": {"shapes": [1]}, "depth": 3, "step_down": 3})
    )]);
    job = parse_job(data, "p").unwrap();
    job
}

#[test]
fn new_projects_embed_the_drawing_and_run_without_it() {
    let dir = temp_dir("new");
    let dxf = dir.join("part.dxf");
    square_dxf(&dxf, &[("A1", "CUT", 10.0, 10.0, 50.0)]);
    let job = project_with_profile(&dxf);
    assert_eq!(job.drawing.next_shape_id, 2);
    assert_eq!(job.drawing.source.as_ref().unwrap().file, "part.dxf");
    std::fs::remove_file(&dxf).unwrap(); // the DXF is no longer needed
    let result = run_job(&job).unwrap();
    assert_eq!(result.contours.len(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_unembedded_job_is_refused_with_how_to_fix_it() {
    let old = job(json!({}));
    let err = run_job(&old).unwrap_err().to_string();
    assert!(
        err.contains("not embedded") && err.contains("chipload import --into"),
        "{err}"
    );
    assert!(load_drawing(&old).is_err());
}

#[test]
fn embedding_turns_handles_into_shapes_and_keeps_the_golden_output() {
    let lower = parse_job(
        migrate(read("jobs/v3/messy_2_lower.json")).unwrap(),
        "lower",
    )
    .unwrap();
    let (project, report) = embed_drawing(&lower, &messy_2()).unwrap();
    let Operation::Profile(outline) = &project.operations[1] else {
        panic!("expected the outline profile");
    };
    assert!(outline.select.handles.is_none());
    assert_eq!(outline.select.shapes.as_ref().unwrap().len(), 1);
    assert!(
        report.iter().any(|l| l.contains("handles 50 -> shapes S")),
        "{report:?}"
    );
    let golden = std::fs::read_to_string(corpus("expected/v3_messy_2_lower.nc")).unwrap();
    assert_eq!(run_job(&project).unwrap().gcode, golden);
}

#[test]
fn update_keeps_ids_follows_moved_geometry_and_flags_lost_shapes() {
    let dir = temp_dir("update");
    let v1 = dir.join("v1.dxf");
    square_dxf(
        &v1,
        &[
            ("A1", "CUT", 10.0, 10.0, 50.0),
            ("A2", "CUT", 100.0, 10.0, 20.0),
        ],
    );
    let mut job = project_with_profile(&v1);
    // A shape drawn in Chipload (no DXF handle) must survive updates.
    let mut drawn = job.drawing.shapes[1].clone();
    drawn.id = job.drawing.next_shape_id;
    drawn.dxf_handle = None;
    for seg in &mut drawn.segments {
        if let chipload_job::PathSegment::Line(l) = seg {
            l.start[1] += 100.0;
            l.end[1] += 100.0;
        }
    }
    job.drawing.next_shape_id += 1;
    job.drawing.shapes.push(drawn);

    // Same drawing: nothing changes.
    let (same, report) = update_drawing(&job, &v1, None).unwrap();
    assert_eq!(report.unchanged, [1, 2]);
    assert!(report.changed.is_empty() && report.added.is_empty() && report.removed.is_empty());
    assert_eq!(same.drawing.shapes, job.drawing.shapes);

    // Square A1 moved: same id, the operation follows it.
    let v2 = dir.join("v2.dxf");
    square_dxf(
        &v2,
        &[
            ("A1", "CUT", 30.0, 10.0, 50.0),
            ("A2", "CUT", 100.0, 10.0, 20.0),
            ("A3", "CUT", 150.0, 10.0, 5.0),
        ],
    );
    let (moved, report) = update_drawing(&job, &v2, None).unwrap();
    assert_eq!(
        (report.changed.clone(), report.added.clone()),
        (vec![1], vec![4])
    );
    assert!(
        report.lines.iter().any(|l| l.contains("geometry changed")),
        "{:?}",
        report.lines
    );
    assert!(
        moved
            .drawing
            .shapes
            .iter()
            .any(|s| s.id == 3 && s.dxf_handle.is_none())
    );
    let before = run_job(&job).unwrap().program.moves;
    let after = run_job(&moved).unwrap().program.moves;
    let first_x =
        |moves: &[chipload_toolpath::model::Move]| moves.iter().find_map(|m| m.x).unwrap();
    assert!((first_x(&after) - first_x(&before) - 20.0).abs() < 1e-9);

    // Square A1 deleted: the operation is flagged and can't run.
    let v3 = dir.join("v3.dxf");
    square_dxf(&v3, &[("A2", "CUT", 100.0, 10.0, 20.0)]);
    let (lost, report) = update_drawing(&job, &v3, None).unwrap();
    assert_eq!(report.removed, [1]);
    assert_eq!(report.lost, [0]);
    let err = run_job(&lost).unwrap_err().to_string();
    assert!(err.contains("S1 is not part of any contour"), "{err}");

    // Different units are never guessed.
    let inch = dir.join("inch.dxf");
    square_dxf(&inch, &[("A1", "CUT", 1.0, 1.0, 2.0)]);
    let text = std::fs::read_to_string(&inch)
        .unwrap()
        .replace("70\n4\n0\nENDSEC", "70\n1\n0\nENDSEC");
    std::fs::write(&inch, text).unwrap();
    assert!(update_drawing(&job, &inch, None).is_err());
    assert!(update_drawing(&job, &inch, Some(chipload_job::Units::Inch)).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}
