// SPDX-License-Identifier: GPL-3.0-or-later
//! End-to-end rules of run_job.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::migrations::migrate;
use chipload_job::{Job, Note, OpRef, Spindle, load_job, parse_job};
use chipload_pipeline::{
    Cancel, PipelineError, RunResult, new_project, run_job, run_job_cancellable, update_drawing,
};
use chipload_toolpath::ToolpathError;
use chipload_toolpath::model::MoveKind;
use serde_json::{Value, json};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
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
        // The copies belong to the one part (docs/specs/parts.md §4).
        if key == "copies" {
            base["parts"][0]["copies"] = value.clone();
            continue;
        }
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

/// The simple_part sample project (inside + outside profile) with changes.
fn job(changes: Value) -> Job {
    with(current("jobs/v4/simple_part_full.chipload"), changes)
}

/// A frozen project migrated to the current version, as raw data.
fn current(rel: &str) -> Value {
    migrate(read(rel)).unwrap()
}

/// The sample's outside profile with changes.
fn op(changes: Value) -> Value {
    let mut base = current("jobs/v4/simple_part_full.chipload")["operations"][1].clone();
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

/// `job` with its drawing replaced by `dxf`, imported in mm (for drawings without units).
fn on_drawing(mut job: Job, dxf: &Path) -> Job {
    let imported = chipload_dxf::shapes_from_dxf(dxf, Some(chipload_job::Units::Mm)).unwrap();
    job.drawing = new_project(imported).drawing;
    job
}

fn run(job: &Job) -> RunResult {
    run_job(job).unwrap()
}

fn error(job: &Job) -> PipelineError {
    match run_job(job) {
        Ok(_) => panic!("expected an error"),
        Err(err) => err,
    }
}

fn job_error(job: &Job, text: &str) {
    let err = error(job);
    assert!(
        matches!(&err, PipelineError::Toolpath(e)
            if matches!(e.cause(), ToolpathError::Job(m) if m.contains(text))),
        "{err}"
    );
}

#[test]
fn sample_runs_and_passes_check() {
    let result = run(&job(json!({})));
    assert!(result.check_warnings.is_empty());
    assert_eq!(min(&z_values(&result.files[0].gcode)), -12.0);
}

#[test]
fn depth_equal_to_thickness_is_allowed() {
    let result = run(&job(json!({"operations": [op(json!({"end_depth": 12}))]})));
    assert_eq!(min(&z_values(&result.files[0].gcode)), -12.0);
}

#[test]
fn depth_beyond_thickness_is_refused() {
    job_error(
        &job(json!({"operations": [op(json!({"end_depth": 12.5}))]})),
        "0.500 mm below the stock (12 mm thick), more than the allowed 0 mm",
    );
}

#[test]
fn z_zero_bottom_shifts_heights() {
    let result = run(&job(
        json!({"stock": {"z_zero": "bottom"}, "machine": {"safe_z": 17}}),
    ));
    let zs = z_values(&result.files[0].gcode);
    assert_eq!((min(&zs), max(&zs)), (0.0, 17.0));
}

#[test]
fn safe_z_must_clear_the_stock() {
    // safe_z 5 is below the top (12) when Z zero is the bottom.
    job_error(&job(json!({"stock": {"z_zero": "bottom"}})), "safe_z");
}

#[test]
fn tool_without_rpm_is_refused() {
    job_error(&job(json!({"tools": [{"id": 1, "diameter": 6}]})), "rpm");
}

// -- a tool per operation (docs/specs/tool-per-operation.md)

/// Tool 1: 6 mm at 18000 rpm (the sample's); tool 2: 3 mm at 20000 rpm.
fn two_tools() -> Value {
    json!([
        {"id": 1, "diameter": 6, "rpm": 18000},
        {"id": 2, "diameter": 3, "rpm": 20000, "name": "3 mm (downcut)"}
    ])
}

/// The sample's inside profile with tool 2, then its outside profile with tool 1.
fn two_tool_job(changes: Value) -> Job {
    let mut ops = current("jobs/v4/simple_part_full.chipload")["operations"].clone();
    ops[0]["tool"] = json!(2);
    let mut base = json!({"tools": two_tools(), "operations": ops});
    for (k, v) in changes.as_object().unwrap() {
        base[k] = v.clone();
    }
    job(base)
}

/// The program after its header comments: what the machine does.
fn body(gcode: &str) -> Vec<&str> {
    gcode.lines().skip_while(|l| l.starts_with('(')).collect()
}

#[test]
fn each_tool_group_is_a_checked_file_of_its_own() {
    let job = two_tool_job(json!({}));
    let result = run(&job);
    assert_eq!(result.files.len(), 2);
    let (first, second) = (&result.files[0], &result.files[1]);
    assert_eq!(first.suffix.as_deref(), Some("-1-T2"));
    assert_eq!(second.suffix.as_deref(), Some("-2-T1"));
    assert_eq!(
        (first.tool, first.operations.clone()),
        (2, vec![OpRef::op(0)])
    );
    assert_eq!(
        (second.tool, second.operations.clone()),
        (1, vec![OpRef::op(1)])
    );
    assert!(
        first.gcode.starts_with(
            "(Chipload - GRBL)\n(File 1 of 2: tool 2, 3.000 mm endmill \"3 mm [downcut]\")\n\
             (Operations 1)\n(Set Z zero with this tool. Keep X and Y zero.)\n(Tool 2: 3.000 mm)\n"
        ),
        "{}",
        first.gcode
    );
    assert!(
        second
            .gcode
            .contains("(File 2 of 2: tool 1, 6.000 mm endmill)\n")
    );
    for (file, rpm, tool) in [(first, "S20000", 2), (second, "S18000", 1)] {
        let lines = body(&file.gcode);
        assert_eq!(
            &lines[..4],
            [
                "G21 G90 G17 G94",
                &format!("M3 {rpm}"),
                "G4 P3.000",
                "G0 Z5.000"
            ]
        );
        assert_eq!(&lines[lines.len() - 2..], ["M5", "M30"]);
        let limits = chipload_check::limits_for_tool(&job, tool).unwrap();
        let report = chipload_check::check_gcode(&file.gcode, &limits);
        assert!(report.ok(), "{:?}", report.errors);
    }
}

#[test]
fn each_operation_is_cut_as_with_its_tool_alone() {
    let both = run(&two_tool_job(json!({})));
    // The inside profile alone, with tool 2; the outside profile alone, with tool 1.
    let ops = current("jobs/v4/simple_part_full.chipload")["operations"].clone();
    let mut inside = ops[0].clone();
    inside["tool"] = json!(2);
    let alone = |op: Value| run(&job(json!({"tools": two_tools(), "operations": [op]})));
    let inside_alone = alone(inside);
    let outside_alone = alone(ops[1].clone());
    assert_eq!(inside_alone.files.len(), 1);
    assert_eq!(
        body(&both.files[0].gcode),
        body(&inside_alone.files[0].gcode)
    );
    assert_eq!(
        body(&both.files[1].gcode),
        body(&outside_alone.files[0].gcode)
    );
}

#[test]
fn one_tool_writes_one_file_without_a_suffix() {
    let result = run(&job(json!({"tools": two_tools()})));
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].suffix, None);
    assert!(!result.files[0].gcode.contains("(File "));
}

#[test]
fn a_tool_that_comes_back_is_another_group_and_a_note() {
    let ops = current("jobs/v4/simple_part_full.chipload")["operations"].clone();
    let with_tool = |i: usize, tool: i64| {
        let mut op = ops[i].clone();
        op["tool"] = json!(tool);
        op
    };
    // 1, 2, 1: three files.
    let job3 = job(
        json!({"tools": two_tools(), "operations": [with_tool(0, 1), with_tool(1, 2), with_tool(0, 1)]}),
    );
    let result = run(&job3);
    let tools: Vec<i64> = result.files.iter().map(|f| f.tool).collect();
    assert_eq!(tools, [1, 2, 1]);
    assert!(
        result.notes.iter().any(|n| n
            == "Tool 1 is used in 2 groups (operations 1, 3); moving them next to each other \
                saves a tool change"),
        "{:?}",
        result.notes
    );
    // 1, 1, 2: two files, no note.
    let job2 = job(
        json!({"tools": two_tools(), "operations": [with_tool(0, 1), with_tool(0, 1), with_tool(1, 2)]}),
    );
    let result = run(&job2);
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.files[0].operations, [OpRef::op(0), OpRef::op(1)]);
    assert!(!result.notes.iter().any(|n| n.contains("tool change")));
    // A disabled operation between two with the same tool doesn't split them.
    let mut off = with_tool(1, 2);
    off["enabled"] = json!(false);
    let job1 =
        job(json!({"tools": two_tools(), "operations": [with_tool(0, 1), off, with_tool(1, 1)]}));
    let result = run(&job1);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].operations, [OpRef::op(0), OpRef::op(2)]);
}

#[test]
fn copies_are_cut_tool_group_by_tool_group() {
    // Inside and outside profile with tool 1, then the inside again with tool 2.
    let ops = current("jobs/v4/simple_part_full.chipload")["operations"].clone();
    let mut again = ops[0].clone();
    again["tool"] = json!(2);
    let operations = json!([ops[0], ops[1], again]);
    let expected = [
        (
            "by_operation",
            [(0, 1), (0, 2), (1, 1), (1, 2), (2, 1), (2, 2)],
        ),
        ("by_copy", [(0, 1), (1, 1), (0, 2), (1, 2), (2, 1), (2, 2)]),
    ];
    for (order, cut) in expected {
        let copies = json!({"columns": 2, "rows": 1, "dx": 250.0, "dy": 0.0, "order": order});
        let job = job(json!({"tools": two_tools(), "operations": operations, "copies": copies}));
        let result = run(&job);
        assert_eq!(result.files.len(), 2, "{order}");
        let spans: Vec<(usize, u32)> = result
            .op_spans
            .iter()
            .map(|s| (s.operation, s.copy))
            .collect();
        assert_eq!(spans, cut, "{order}");
    }
}

#[test]
fn operations_are_checked_against_their_own_tool() {
    let mut tools = two_tools();
    tools[1]["rpm"] = Value::Null;
    job_error(
        &two_tool_job(json!({"tools": tools})),
        "Tool 2 has no spindle speed",
    );
    // A clearance that fits tool 1's radius but not tool 2's.
    let mut ops = current("jobs/v4/simple_part_full.chipload")["operations"].clone();
    ops[0]["tool"] = json!(2);
    ops[0]["clearance"] = json!(2.0);
    job_error(
        &job(json!({"tools": two_tools(), "operations": ops})),
        "clearance 2 mm must be less than the tool radius 1.5 mm",
    );
    let mut ops = current("jobs/v4/simple_part_full.chipload")["operations"].clone();
    ops[1]["tool"] = json!(3);
    job_error(
        &job(json!({"tools": two_tools(), "operations": ops})),
        "Tool 3 is not defined",
    );
}

#[test]
fn with_tool_changes_in_the_program_it_is_one_checked_file_with_m6() {
    // docs/specs/tool-per-operation.md §3.2.
    let job = two_tool_job(json!({"machine": {"tool_change": "program"}}));
    let result = run(&job);
    assert_eq!(result.files.len(), 1);
    let file = &result.files[0];
    assert_eq!(
        (file.suffix.clone(), file.operations.clone()),
        (None, vec![OpRef::op(0), OpRef::op(1)])
    );
    let lines: Vec<&str> = file.gcode.lines().collect();
    assert!(
        lines.contains(&"(T2 3.000 mm endmill \"3 mm [downcut]\")"),
        "{}",
        file.gcode
    );
    assert!(lines.contains(&"(T1 6.000 mm endmill)"));
    let start = lines.iter().position(|l| *l == "G21 G90 G17 G94").unwrap();
    assert_eq!(
        &lines[start..start + 5],
        [
            "G21 G90 G17 G94",
            "G0 Z5.000",
            "T2 M6",
            "M3 S20000",
            "G4 P3.000"
        ]
    );
    let change = lines.iter().position(|l| *l == "T1 M6").unwrap();
    assert_eq!(
        &lines[change - 3..change + 3],
        [
            "(Tool change 1 of 1: tool 1, 6.000 mm endmill, operations 2)",
            "G0 Z5.000",
            "M5",
            "T1 M6",
            "M3 S18000",
            "G4 P3.000"
        ]
    );
    // The machine does what the two files do, in one go.
    let files = run(&two_tool_job(json!({})));
    let moves = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|l| ["G1 ", "G2 ", "G3 "].iter().any(|g| l.starts_with(g)))
            .map(String::from)
            .collect()
    };
    let both: Vec<String> = files.files.iter().flat_map(|f| moves(&f.gcode)).collect();
    assert_eq!(moves(&file.gcode), both);
}

#[test]
fn one_tool_gives_the_same_file_in_both_modes() {
    let files = run(&job(json!({})));
    let program = run(&job(json!({"machine": {"tool_change": "program"}})));
    assert_eq!(files.files, program.files);
    assert!(!program.files[0].gcode.contains("M6"));
}

#[test]
fn gcode_file_names_add_the_group_and_tool_to_the_stem() {
    let file = |suffix: Option<&str>| chipload_pipeline::GcodeFile {
        suffix: suffix.map(String::from),
        tool: 3,
        operations: vec![OpRef::op(1), OpRef::op(3)],
        gcode: String::new(),
    };
    let out = Path::new("dir/part.nc");
    assert_eq!(file(None).path(out), Path::new("dir/part.nc"));
    assert_eq!(file(Some("-2-T3")).path(out), Path::new("dir/part-2-T3.nc"));
    assert_eq!(
        file(Some("-2-T3")).path(Path::new("part")),
        Path::new("part-2-T3")
    );
    assert_eq!(
        file(Some("-2-T3")).describe(out),
        "part-2-T3.nc (tool 3, operations 2, 4)"
    );
}

#[test]
fn feed_above_machine_max_is_refused_not_clamped() {
    job_error(
        &job(json!({"operations": [op(json!({"feed": 6000}))]})),
        "max_feed",
    );
}

fn selector_error(job: &Job, text: &str) {
    let err = error(job);
    assert!(
        matches!(&err, PipelineError::Toolpath(e)
            if matches!(e.cause(), ToolpathError::Selector(m) if m.contains(text))),
        "{err}"
    );
}

#[test]
fn selector_matching_nothing_is_an_error() {
    let ops = json!([op(json!({"select": {"layer": "NOPE"}}))]);
    selector_error(&job(json!({"operations": ops})), "matches nothing");
}

#[test]
fn layer_match_is_case_insensitive() {
    let ops = json!([op(json!({"select": {"layer": "cut_outside"}}))]);
    run(&job(json!({"operations": ops})));
}

#[test]
fn selection_of_only_open_contours_is_an_error() {
    let ops = json!([op(json!({"select": {"layer": "OPEN"}}))]);
    selector_error(
        &on_drawing(job(json!({"operations": ops})), &corpus("dxf/messy.dxf")),
        "only open contours",
    );
}

#[test]
fn open_centreline_on_a_cutting_layer_is_skipped_and_reported() {
    // Real drawing: the `upper` layer holds the lid's shapes plus an open centreline (#22).
    let data = on_drawing(
        job(json!({
            "stock": {"x": 200, "y": 120, "origin": "center"},
            "operations": [op(json!({"select": {"layer": "upper"}, "side": "on"}))],
        })),
        &messy_2(),
    );
    let result = run(&data);
    let skipped: Vec<&Note> = result
        .notes
        .iter()
        .filter(|n| n.contains("skipped open contour"))
        .collect();
    assert_eq!(
        skipped,
        [
            "Operation 1 (Outline): skipped open contour #22 on upper (64.750, 50.000) mm to (64.750, 20.000) mm"
        ]
    );
}

#[test]
fn tool_larger_than_hole_names_the_contour() {
    let tools = json!([{"id": 1, "diameter": 32, "rpm": 18000}]);
    let holes = op(json!({"select": {"layer": "HOLES"}, "side": "inside"}));
    let err = error(&job(json!({"tools": tools, "operations": [holes]})));
    assert!(
        matches!(&err, PipelineError::Toolpath(e)
            if matches!(e.cause(), ToolpathError::Geometry(m) if m.contains("contour #2"))),
        "{err}"
    );
}

#[test]
fn holes_are_cut_before_the_outline_even_if_listed_together() {
    // One op selecting the outline and a hole together: still hole first.
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

    // Imported in order: the outline is S1, the hole S2.
    let select = json!({"shapes": [1, 2]});
    let result = run(&on_drawing(
        job(json!({"operations": [op(json!({"select": select, "side": "on"}))]})),
        &path,
    ));
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
fn checker_rejection_returns_no_gcode_but_the_toolpath() {
    // The part is 206 mm wide.
    let small = job(json!({"machine": {"travel": {"x": 100, "y": 600, "z": 100}}}));
    match error(&small) {
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
    let result = run(&job(json!({})));
    assert_eq!(result.contours.len(), 4);
    assert_eq!(result.program.moves[0].kind, MoveKind::Rapid);
    assert_eq!(result.program.spindle, Spindle::Cw);
}

#[test]
fn drawing_outside_the_stock_is_noted() {
    let result = run(&job(json!({"stock": {"x": 100}})));
    assert!(
        result
            .notes
            .iter()
            .any(|n| n == "The drawing extends outside the stock")
    );
}

// -- pocket

fn lower_job(tool_changes: Value, pocket_changes: Value) -> Job {
    let mut data = read("jobs/v4/messy_2_lower.chipload");
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
    let result = run(&lower_job(json!({}), json!({})));
    assert!(result.check_warnings.is_empty());
    assert_eq!(min(&z_values(&result.files[0].gcode)), -18.0); // the outline goes through
    assert_eq!(result.pockets.len(), 3);
}

#[test]
fn pocket_refuses_a_tool_that_is_not_centre_cutting() {
    job_error(
        &lower_job(json!({"center_cutting": false}), json!({})),
        "not centre-cutting",
    );
}

#[test]
fn pocket_warns_when_centre_cutting_is_unknown() {
    let result = run(&lower_job(json!({"center_cutting": null}), json!({})));
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
        "need an endmill",
    );
}

#[test]
fn pocket_rings_are_linked_at_depth_where_safe() {
    // docs/specs/pocket.md as built 2026-10-15: no setting; every verified link is fed.
    let result = run(&lower_job(json!({}), json!({})));
    // The two narrow bars are one ring each; the lower pocket's rings are all linked.
    let links: Vec<bool> = result
        .pockets
        .iter()
        .flat_map(|piece| piece.feed_links.iter().copied())
        .collect();
    assert!(!links.is_empty() && links.iter().all(|&l| l), "{links:?}");
}

#[test]
fn pocket_errors_name_the_operation() {
    let err = error(&lower_job(json!({"diameter": 16}), json!({})));
    let text = err.to_string();
    assert!(
        text.starts_with("Operation 1 (Openings):") && text.contains("does not fit"),
        "{text}"
    );
}

// -- tabs

fn tabs_job(tab_changes: Value, op_changes: Value) -> Job {
    let mut data = read("jobs/v4/simple_part_tabs.chipload");
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
    let result = run(&tabs_job(json!({}), json!({})));
    assert_eq!(result.tabs.len(), 4);
    let zs = z_values(&result.files[0].gcode);
    assert!(zs.contains(&-9.0)); // tab top: cut bottom -12 + height 3
    assert_eq!(min(&zs), -12.0);
}

#[test]
fn tab_height_must_be_below_the_cut_depth() {
    job_error(&tabs_job(json!({"height": 12}), json!({})), "tab height");
}

#[test]
fn too_many_tabs_names_the_contour() {
    let err = error(&tabs_job(json!({"count": 60}), json!({})));
    let text = err.to_string();
    assert!(
        text.contains("contour #1") && text.contains("don't fit"),
        "{text}"
    );
}

#[test]
fn tabs_on_an_inside_profile_warn() {
    let holes = json!({"select": {"layer": "HOLES"}, "side": "inside"});
    let result = run(&tabs_job(json!({"count": 1, "width": 3}), holes));
    assert!(result.notes.iter().any(|n| n.contains("inside profile")));
    // The round hole has no straight stretch for its tab.
    assert!(result.notes.iter().any(|n| n.contains("corner or curve")));
}

// -- enabled / disabled operations (docs/specs/job-editor.md §2)

#[test]
fn disabled_operations_are_skipped_and_keep_their_numbers() {
    let all = run(&job(json!({})));
    let hole_only = run(&job(json!({"operations": [
        current("jobs/v4/simple_part_full.chipload")["operations"][0].clone(),
        op(json!({"enabled": false})),
    ]})));
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
    let err = error(&job(json!({"operations": [
        op(json!({"enabled": false})),
        op(json!({"select": {"layer": "NOPE"}})),
    ]})));
    assert!(err.to_string().starts_with("Operation 2"), "{err}");
}

#[test]
fn every_operation_disabled_is_an_error() {
    job_error(
        &job(json!({"operations": [op(json!({"enabled": false}))]})),
        "nothing to cut",
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
        json!({"select": {"shapes": [1]}, "end_depth": 3, "step_down": 3})
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
    assert_eq!(job.drawing.dxfs[0].file, "part.dxf");
    assert_eq!(job.drawing.dxfs[0].name, "part");
    std::fs::remove_file(&dxf).unwrap(); // the DXF is no longer needed
    let result = run_job(&job).unwrap();
    assert_eq!(result.contours.len(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
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
    // A shape drawn in Chipload must survive updates.
    let drawn = chipload_job::Shape {
        id: job.drawing.next_shape_id,
        layer: "CUT".into(),
        geometry: chipload_job::Geometry::Rectangle {
            center: [110.0, 120.0],
            length: 20.0,
            width: 20.0,
            angle: 0.0,
            corner_radius: 0.0,
        },
    };
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
            .any(|s| s.id == 3 && s.is_drawn())
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

// -- work zero (docs/specs/work-zero.md)

fn drawn_part_with(work_zero: chipload_job::WorkZero) -> Job {
    let mut job = load_job(&corpus("jobs/v5/drawn_part.chipload")).unwrap();
    job.machine.work_zero = work_zero;
    job
}

/// (X, Y) words of every motion line, None where a line has no such word.
fn xy_words(gcode: &str) -> Vec<(Option<f64>, Option<f64>)> {
    let word = |line: &str, axis: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(axis).and_then(|v| v.parse::<f64>().ok()))
    };
    gcode
        .lines()
        .filter(|l| l.starts_with("G0") || l.starts_with("G1"))
        .map(|l| (word(l, 'X'), word(l, 'Y')))
        .collect()
}

#[test]
fn the_work_zero_only_translates_the_output() {
    use chipload_job::{StockCorner, WorkZero};
    let base = run_job(&drawn_part_with(WorkZero::DrawingOrigin {})).unwrap();
    // Stock 300 x 220 with its bottom-left corner at the drawing origin.
    for (zero, (dx, dy), text) in [
        (
            WorkZero::Stock {
                corner: StockCorner::Center,
            },
            (150.0, 110.0),
            "stock center = drawing X150.000 Y110.000, Z0 = stock top",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopRight,
            },
            (300.0, 220.0),
            "stock top right corner = drawing X300.000 Y220.000",
        ),
        (
            WorkZero::Point { x: 25.0, y: -10.0 },
            (25.0, -10.0),
            "point = drawing X25.000 Y-10.000",
        ),
    ] {
        let moved = run_job(&drawn_part_with(zero)).unwrap();
        assert!(
            moved.files[0]
                .gcode
                .contains(&format!("(Work zero: {text}")),
            "{text}"
        );
        // Same toolpath for display, every output X/Y shifted by the same vector.
        assert_eq!(moved.program, base.program);
        let (a, b) = (
            xy_words(&base.files[0].gcode),
            xy_words(&moved.files[0].gcode),
        );
        assert_eq!(a.len(), b.len());
        for ((x0, y0), (x1, y1)) in a.into_iter().zip(b) {
            assert_eq!(x0.is_some(), x1.is_some());
            assert_eq!(y0.is_some(), y1.is_some());
            if let (Some(x0), Some(x1)) = (x0, x1) {
                assert!((x0 - dx - x1).abs() < 1e-9, "{x0} - {dx} != {x1}");
            }
            if let (Some(y0), Some(y1)) = (y0, y1) {
                assert!((y0 - dy - y1).abs() < 1e-9, "{y0} - {dy} != {y1}");
            }
        }
    }
}

#[test]
fn the_checker_places_the_stock_by_the_work_zero_itself() {
    use chipload_check::{check_gcode, limits_from_job};
    use chipload_job::{StockCorner, WorkZero};
    let centred = drawn_part_with(WorkZero::Stock {
        corner: StockCorner::Center,
    });
    let gcode = run_job(&centred).unwrap().files[0].gcode.clone();
    let outside = |job: &Job| {
        check_gcode(&gcode, &limits_from_job(job))
            .warnings
            .iter()
            .any(|w| w.message.contains("outside the stock"))
    };
    // Judged with its own project: the stock is where the program expects it.
    assert!(!outside(&centred));
    // Judged as if zero were the drawing origin: the cuts land outside the stock.
    assert!(outside(&drawn_part_with(WorkZero::DrawingOrigin {})));
}

// -- polyline bulges (docs/specs/drawing-follow-ups.md section 3)

#[test]
fn a_pocket_of_a_bulged_polyline_passes_the_checker_with_arcs() {
    let mut job = load_job(&corpus("jobs/v7/adaptive.chipload")).unwrap();
    // The triangle's bottom side bows out by 0.3 × half its 30 mm chord.
    let triangle = job.drawing.shapes.iter_mut().find(|s| s.id == 3).unwrap();
    let chipload_job::Geometry::Polyline { bulges, .. } = &mut triangle.geometry else {
        panic!("polyline");
    };
    *bulges = vec![0.3, 0.0, 0.0];
    let chipload_job::Operation::Pocket(pocket) = &mut job.operations[0] else {
        panic!("pocket");
    };
    pocket.select.shapes = Some(vec![3]);
    // run_job refuses G-code the checker rejects.
    let result = run(&job);
    let arcs = result.files[0]
        .gcode
        .lines()
        .filter(|l| l.starts_with("G2") || l.starts_with("G3"))
        .count();
    assert!(arcs > 0, "{}", result.files[0].gcode);
    // The bow is cut: the pocket reaches below the triangle's straight bottom (y = 65).
    let lowest = result
        .pockets
        .iter()
        .flat_map(|piece| piece.rings.iter().flat_map(|r| &r.points))
        .map(|p| p.1)
        .fold(f64::INFINITY, f64::min);
    assert!(lowest < 65.0, "{lowest}");
}

// -- cancelling (docs/specs/background-generation.md)

fn round_corners_job() -> Job {
    load_job(&corpus("jobs/v14/round_corners.chipload")).unwrap()
}

#[test]
fn a_cancelled_run_stops_and_an_uncancelled_one_is_unchanged() {
    let job = round_corners_job();
    let cancel = Cancel::new();
    let same = run_job_cancellable(&job, &cancel).unwrap();
    assert_eq!(same.files, run(&job).files);
    cancel.cancel();
    assert!(matches!(
        run_job_cancellable(&job, &cancel),
        Err(PipelineError::Toolpath(ToolpathError::Cancelled))
    ));
}

#[test]
fn a_run_cancelled_from_another_thread_stops_early() {
    let job = round_corners_job();
    let started = std::time::Instant::now();
    run(&job);
    let full = started.elapsed();
    if full < std::time::Duration::from_millis(40) {
        return; // too quick to cancel halfway on this machine
    }
    let cancel = std::sync::Arc::new(Cancel::new());
    let flag = cancel.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(full / 8);
        flag.cancel();
    });
    let started = std::time::Instant::now();
    let result = run_job_cancellable(&job, &cancel);
    let took = started.elapsed();
    canceller.join().unwrap();
    assert!(matches!(
        result,
        Err(PipelineError::Toolpath(ToolpathError::Cancelled))
    ));
    assert!(took < full * 3 / 4, "{took:?} of {full:?}");
}

// -- problems while editing (docs/specs/live-problems.md)

fn paths(job: &Job) -> Vec<String> {
    chipload_pipeline::job_problems(job)
        .into_iter()
        .map(|p| p.path)
        .collect()
}

#[test]
fn every_problem_is_found_at_once_with_its_field() {
    assert!(paths(&job(json!({}))).is_empty());
    // Built valid, then broken the way the editor can (parsing would refuse it).
    let mut bad = job(json!({
        "machine": {"max_feed": 1000.0},
        "operations": [op(json!({"clearance": 5.0})), op(json!({}))],
    }));
    bad.machine.safe_z = -1.0;
    if let chipload_job::Operation::Profile(p) = &mut bad.operations[0] {
        p.feed = 0.0;
    }
    if let chipload_job::Operation::Profile(p) = &mut bad.operations[1] {
        p.plunge_feed = 2000.0;
    }
    let found = paths(&bad);
    for path in [
        "operations.0.feed",
        "machine.safe_z",
        "operations.0.clearance",
        "operations.1.plunge_feed",
    ] {
        assert!(found.iter().any(|p| p == path), "{path} in {found:?}");
    }
    // The first job rule is still the error a run gives.
    let first = chipload_toolpath::job_problems(&bad).remove(0);
    let PipelineError::Toolpath(err) = error(&bad) else {
        panic!("a toolpath error");
    };
    let ToolpathError::Job(message) = err.cause() else {
        panic!("a job error");
    };
    assert_eq!(&first.text, message);
    assert_eq!(err.operation(), None, "a machine rule is nobody's");
    assert!(first.text.starts_with("safe_z -1"), "{}", first.text);
    assert_eq!(first.short, "must be above the stock top (Z 0 mm)");
}

#[test]
fn the_short_message_leaves_out_the_operation() {
    let bad = job(json!({"operations": [op(json!({"clearance": 5.0}))]}));
    let problem = chipload_toolpath::job_problems(&bad).remove(0);
    assert_eq!(problem.path, "operations.0.clearance");
    assert_eq!(
        problem.short,
        "clearance 5 mm must be less than the tool radius 3 mm"
    );
    assert!(problem.text.starts_with("Operation 1"), "{}", problem.text);
}

// -- cycle time (docs/specs/cycle-time.md)

mod cycle_time {
    use super::*;
    use chipload_pipeline::estimate::{ASSUMED_RAPID_MM_MIN, estimate};
    use chipload_toolpath::model::{Move, Program};
    use chipload_toolpath::{OpSpan, ToolGroup};

    fn rectangle() -> Program {
        Program {
            tool_id: 1,
            tool_diameter: 6.0,
            rpm: 18000.0,
            spindle: Spindle::Cw,
            safe_z: 5.0,
            moves: vec![
                Move::rapid_z(5.0),
                Move::rapid_xy((0.0, 0.0)),
                Move::feed_z(-3.0, 300.0),
                Move::feed_xy((100.0, 0.0), 1200.0),
                Move::feed_xy((100.0, 50.0), 1200.0),
                Move::feed_xy((0.0, 50.0), 1200.0),
                Move::feed_xy((0.0, 0.0), 1200.0),
                Move::rapid_z(5.0),
            ],
            comments: Vec::new(),
        }
    }

    fn group(job: &Job, moves: std::ops::Range<usize>) -> ToolGroup {
        ToolGroup {
            tool: job.tools[0].clone(),
            operations: vec![OpRef::op(0)],
            moves,
        }
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_rectangle_takes_its_length_over_its_feeds() {
        let mut job = job(json!({}));
        job.machine.max_feed = None;
        let program = rectangle();
        let span = OpSpan {
            operation: 0,
            pass: None,
            copy: 1,
            moves: 0..8,
            tool_radius: 3.0,
        };
        let e = estimate(&job, &program, &[span], &[group(&job, 0..8)], 3.0);
        // Down 8 mm at 300, round 300 mm at 1200; up 8 mm at the assumed rapid rate.
        let cutting = 8.0 / 300.0 * 60.0 + 300.0 / 1200.0 * 60.0;
        let rapid = 8.0 / ASSUMED_RAPID_MM_MIN * 60.0;
        assert!(e.rapid_assumed && near(e.rapid_rate, ASSUMED_RAPID_MM_MIN));
        assert!(near(e.cutting, cutting), "{}", e.cutting);
        assert!(near(e.rapid, rapid), "{}", e.rapid);
        assert!(near(e.spindle, 3.0));
        assert!(near(e.total, cutting + rapid + 3.0));
        assert_eq!(e.operations.len(), 1);
        assert!(near(e.operations[0].1, cutting + rapid));
        assert_eq!(e.tool_changes, 0);

        // The machine's max feed is the rapid rate.
        job.machine.max_feed = Some(6000.0);
        let e = estimate(&job, &program, &[], &[group(&job, 0..8)], 3.0);
        assert!(!e.rapid_assumed && near(e.rapid, 8.0 / 6000.0 * 60.0));
    }

    #[test]
    fn copies_add_up_and_each_tool_group_waits_for_its_spindle() {
        let job = job(json!({}));
        let mut program = rectangle();
        let again = program.moves.clone();
        program.moves.extend(again);
        let spans: Vec<OpSpan> = [0..8, 8..16]
            .into_iter()
            .enumerate()
            .map(|(k, moves)| OpSpan {
                operation: 1,
                pass: None,
                copy: k as u32 + 1,
                moves,
                tool_radius: 3.0,
            })
            .collect();
        let one = estimate(&job, &rectangle(), &spans[..1], &[group(&job, 0..8)], 0.0);
        let two = estimate(
            &job,
            &program,
            &spans,
            &[group(&job, 0..8), group(&job, 8..16)],
            2.0,
        );
        assert_eq!(two.operations.len(), 1);
        // The second copy also travels from the first one's end: a little more than twice.
        assert!(two.operations[0].1 >= 2.0 * one.operations[0].1 - 1e-9);
        assert!(near(two.spindle, 4.0));
        assert_eq!(two.tool_changes, 1);
        assert_eq!(two.groups.len(), 2);
    }

    #[test]
    fn a_helix_counts_its_descent() {
        let job = job(json!({}));
        let mut program = rectangle();
        // Down 3 mm while going 4 mm across: 5 mm at 300.
        program.moves = vec![
            Move::rapid_z(0.0),
            Move::rapid_xy((0.0, 0.0)),
            Move::feed_xyz((4.0, 0.0), -3.0, 300.0),
        ];
        let e = estimate(&job, &program, &[], &[group(&job, 0..3)], 0.0);
        assert!(near(e.cutting, 5.0 / 300.0 * 60.0), "{}", e.cutting);
    }

    #[test]
    fn a_run_reports_its_estimate() {
        let job = load_job(&corpus("jobs/v11/two_tools.chipload")).unwrap();
        let result = run(&job);
        let e = &result.estimate;
        assert_eq!(e.operations.len(), 2);
        assert_eq!(e.groups.len(), 2);
        assert_eq!(e.tool_changes, 1);
        assert!(e.total > e.cutting && e.cutting > 0.0);
        let lines = chipload_pipeline::estimate::estimate_lines(&job, e, true);
        assert!(lines[0].starts_with("Estimated time: "), "{lines:?}");
        assert!(lines[0].ends_with("plus 1 tool change"), "{lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("  File 2: ")));
    }
}

// -- parts (docs/specs/parts.md §6)

#[test]
fn the_report_gives_each_parts_time_over_its_operations() {
    let job = load_job(&corpus_path("jobs/v22/two_parts.chipload")).unwrap();
    let result = run(&job);
    let lines =
        chipload_pipeline::estimate::estimate_lines(&job, &result.estimate, result.files.len() > 1);
    let part = |n: usize| {
        lines
            .iter()
            .position(|l| l.starts_with(&format!("  Part {n} ")))
    };
    let (one, two) = (part(1).unwrap(), part(2).unwrap());
    assert!(
        lines[one].starts_with("  Part 1 (simple part): "),
        "{lines:?}"
    );
    assert!(lines[two].starts_with("  Part 2 (part B): "), "{lines:?}");
    // Each part's operations under it.
    assert!(
        lines[one + 1].starts_with("    Operation 1 (Holes): "),
        "{lines:?}"
    );
    assert!(
        lines[two + 1].starts_with("    Operation 4 (Outline B): "),
        "{lines:?}"
    );
    // One part: as before parts.
    let one_part = run(&job_of_one_part());
    let lines =
        chipload_pipeline::estimate::estimate_lines(&job_of_one_part(), &one_part.estimate, false);
    assert!(lines.iter().all(|l| !l.contains("Part ")), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("  Operation 1")),
        "{lines:?}"
    );
}

fn job_of_one_part() -> Job {
    job(json!({}))
}

fn corpus_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

// -- inches (docs/specs/inch-units.md §4.2)

#[test]
fn an_inch_machine_with_a_post_that_writes_only_mm_is_a_problem_and_writes_nothing() {
    let mut job = load_job(&corpus("jobs/v4/simple_part_full.chipload")).unwrap();
    job.machine.gcode_units = chipload_job::Units::Inch;
    // The built-in posts write inches.
    assert!(chipload_pipeline::job_problems(&job).is_empty());
    let result = run_job(&job).unwrap();
    assert!(result.files[0].gcode.contains("G20 G90"));
    // A user post from before inches, copied into the project: its template writes G21.
    let builtin = include_str!("../../chipload-post/templates/grbl.nc.j2");
    job.machine.post = "old".into();
    job.machine.custom_post = Some(chipload_job::CustomPost {
        config: "schema_version = 1\nname = \"Old\"\ntemplate = \"old.nc.j2\"\n\
                 spindle_dwell_s = 3\narcs = true\n"
            .into(),
        template: builtin.replace("{{ units_word }}", "G21"),
    });
    let problems = chipload_pipeline::job_problems(&job);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].path, "machine.gcode_units");
    assert!(
        problems[0].short.contains("writes only mm"),
        "{}",
        problems[0].short
    );
    let err = run_job(&job).unwrap_err().to_string();
    assert!(err.contains("can't write G-code in inch"), "{err}");
    // In mm it runs as before.
    job.machine.gcode_units = chipload_job::Units::Mm;
    assert!(run_job(&job).is_ok());
}
