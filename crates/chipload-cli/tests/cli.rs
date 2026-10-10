// SPDX-License-Identifier: GPL-3.0-or-later
//! The `chipload` command line.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// Runs with an empty settings folder: tests never read the user's tool library.
fn chipload(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", temp_dir("no-settings"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-cli-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn inspect_simple_part() {
    let out = chipload(&["inspect", path(&corpus("dxf/simple_part.dxf"))]);
    assert!(out.status.success());
    let stdout = text(&out.stdout);
    assert!(stdout.contains("Units: mm (from dxf)"));
    assert!(stdout.contains("Contours (4):"));
    assert!(stdout.contains("Issues: none"));
}

#[test]
fn inspect_unitless_file_needs_units() {
    let file = corpus("dxf/messy.dxf");
    let out = chipload(&["inspect", path(&file)]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("no units"),
        "{}",
        text(&out.stderr)
    );
    let out = chipload(&["inspect", path(&file), "--units", "mm"]);
    assert!(out.status.success());
}

#[test]
fn inspect_messy_file_reports_problems() {
    let out = chipload(&["inspect", path(&corpus("dxf/messy.dxf")), "--units", "mm"]);
    let stdout = text(&out.stdout);
    for expected in ["duplicate", "zero-length", "is open", "nonzero Z", "TEXT 1"] {
        assert!(
            stdout.contains(expected),
            "missing {expected:?} in\n{stdout}"
        );
    }
}

#[test]
fn inspect_missing_file() {
    let out = chipload(&["inspect", "/nonexistent/part.dxf", "--units", "mm"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).starts_with("error: Cannot read DXF"));
}

#[test]
fn run_writes_checked_gcode() {
    let dir = temp_dir("run");
    let out_file = dir.join("out.nc");
    let out = chipload(&[
        "run",
        path(&corpus("jobs/v4/simple_part_full.chipload")),
        "-o",
        path(&out_file),
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let gcode = std::fs::read_to_string(&out_file).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(gcode.starts_with("(Chipload - GRBL)\n") && gcode.ends_with("M30\n"));
    assert!(text(&out.stdout).contains("Wrote "));
}

#[test]
fn failed_run_writes_nothing_and_keeps_an_old_file() {
    let dir = temp_dir("fail");
    let job = dir.join("job.json");
    let mut data: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(corpus("jobs/v4/simple_part_full.chipload")).unwrap(),
    )
    .unwrap();
    // The part is 206 mm wide: the checker refuses the program.
    data["machine"]["travel"] = serde_json::json!({"x": 100, "y": 600, "z": 100});
    std::fs::write(&job, data.to_string()).unwrap();
    let out_file = dir.join("out.nc");
    std::fs::write(&out_file, "old program\n").unwrap();
    let out = chipload(&["run", path(&job), "-o", path(&out_file)]);
    let kept = std::fs::read_to_string(&out_file).unwrap();
    let leftovers = std::fs::read_dir(&dir).unwrap().count();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    assert!(stderr.contains("exceeds machine travel") && stderr.contains("No output written."));
    assert_eq!(kept, "old program\n");
    assert_eq!(leftovers, 2, "no temporary file left behind");
}

#[test]
fn newer_job_version_is_refused() {
    let dir = temp_dir("newer");
    let job = dir.join("job.json");
    std::fs::write(&job, r#"{"schema_version": 99}"#).unwrap();
    let out_file = dir.join("out.nc");
    let out = chipload(&["run", path(&job), "-o", path(&out_file)]);
    let written = out_file.exists();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("schema_version 99 is not supported"));
    assert!(!written);
}

#[test]
fn licenses_prints_the_gpl_and_every_third_party_licence() {
    let out = chipload(&["licenses"]);
    assert!(out.status.success());
    let stdout = text(&out.stdout);
    assert!(stdout.contains("GNU GENERAL PUBLIC LICENSE"));
    assert!(stdout.contains("Third-party software in Chipload"));
    assert!(
        stdout.contains("cavalier_contours"),
        "dependencies are listed"
    );
}

#[test]
fn import_inspect_update_and_run_a_project() {
    let dir = temp_dir("project");
    let project = dir.join("part.chipload");
    let out = chipload(&[
        "import",
        path(&corpus("dxf/messy_2.dxf")),
        "-o",
        path(&project),
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("Imported messy_2.dxf (22 shapes)"));
    // inspect gives the same report for the project as for the DXF.
    let from_dxf = chipload(&["inspect", path(&corpus("dxf/messy_2.dxf"))]);
    let from_project = chipload(&["inspect", path(&project)]);
    assert_eq!(text(&from_project.stdout), text(&from_dxf.stdout));
    // A project without operations doesn't run.
    let x = dir.join("x.nc");
    assert_eq!(
        chipload(&["run", path(&project), "-o", path(&x)])
            .status
            .code(),
        Some(1)
    );
    // Updating with the same drawing changes nothing.
    let before = std::fs::read_to_string(&project).unwrap();
    let out = chipload(&[
        "update",
        path(&project),
        "--from",
        path(&corpus("dxf/messy_2.dxf")),
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("22 unchanged, 0 changed, 0 added, 0 removed"));
    assert_eq!(std::fs::read_to_string(&project).unwrap(), before);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_job_from_before_projects_is_refused_with_advice() {
    let dir = temp_dir("old-job");
    let job = dir.join("old.json");
    std::fs::write(&job, r#"{"schema_version": 3}"#).unwrap();
    let out = chipload(&["run", path(&job), "-o", path(&dir.join("out.nc"))]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("Chipload 0.3.0"),
        "{}",
        text(&out.stderr)
    );
    assert!(!dir.join("out.nc").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn shapes_are_added_changed_and_deleted_from_the_command_line() {
    // docs/specs/drawing-tools.md §5
    let dir = temp_dir("shapes");
    let project = dir.join("drawn.chipload");
    std::fs::copy(corpus("jobs/v5/drawn_part.chipload"), &project).unwrap();
    let p = path(&project);
    let ok = |args: &[&str]| {
        let out = chipload(args);
        assert!(out.status.success(), "{args:?}: {}", text(&out.stderr));
        text(&out.stdout)
    };
    let refused = |args: &[&str], why: &str| {
        let out = chipload(args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(
            text(&out.stderr).contains(why),
            "{args:?}: {}",
            text(&out.stderr)
        );
    };

    // Ids continue after the last one (3), negative coordinates are fine.
    let added = ok(&[
        "shape", "add", p, "circle", "--center", "-5,-6.5", "--radius", "2",
    ]);
    assert!(added.contains("Added shape S4."), "{added}");
    ok(&[
        "shape",
        "add",
        p,
        "rectangle",
        "--center",
        "200,50",
        "--length",
        "30",
        "--width",
        "10",
        "--angle-deg",
        "45",
        "--layer",
        "extra",
    ]);
    ok(&[
        "shape",
        "add",
        p,
        "polyline",
        "--points",
        "0,0 10,0 10,5",
        "--open",
    ]);
    ok(&["shape", "set", p, "4", "--radius", "2.5"]);
    ok(&["shape", "set", p, "6", "--closed"]);
    let listed = ok(&["inspect", p]);
    for expected in [
        "Shapes: 6 drawn, 0 imported",
        "S4    circle    layer 0            centre (-5.000, -6.500) radius 2.500",
        "S5    rectangle layer extra        centre (200.000, 50.000) length 30.000 width 10.000 \
         angle 45.000°",
        "S6    polyline  layer 0            closed (0.000, 0.000) (10.000, 0.000) (10.000, 5.000)",
    ] {
        assert!(listed.contains(expected), "{expected}\n{listed}");
    }

    refused(
        &["shape", "add", p, "circle", "--center", "0,0"],
        "needs --radius",
    );
    refused(
        &[
            "shape", "add", p, "circle", "--center", "0,0", "--radius", "0",
        ],
        "(S7).radius",
    );
    refused(
        &["shape", "set", p, "4", "--length", "3"],
        "--length doesn't apply to a circle",
    );
    refused(
        &["shape", "set", p, "99", "--radius", "3"],
        "there is no shape 99",
    );
    refused(
        &["shape", "add", p, "polyline", "--points", "0,0 1"],
        "expected a point as X,Y",
    );

    // Bulges (docs/specs/drawing-follow-ups.md section 3): one per segment, |b| <= 1.
    let added = ok(&[
        "shape",
        "add",
        p,
        "polyline",
        "--points",
        "0,0 40,0 20,30",
        "--bulges",
        "0 0.4 -1",
    ]);
    assert!(added.contains("Added shape S7."), "{added}");
    let listed = ok(&["inspect", p]);
    assert!(
        listed.contains("(20.000, 30.000) bulges 0.000 0.400 -1.000"),
        "{listed}"
    );
    refused(
        &["shape", "set", p, "7", "--bulges", "0 1.5 0"],
        "bulges.1: must be between -1 and 1",
    );
    refused(
        &["shape", "set", p, "7", "--bulges", "0 0"],
        "bulges: one per segment, 3 for this polyline, got 2",
    );
    // Opening it drops a segment; which bends to keep is not guessed.
    refused(&["shape", "set", p, "7", "--open"], "give --bulges too");
    ok(&["shape", "set", p, "7", "--open", "--bulges", "0 0.4"]);
    // A straight polyline just follows its new points.
    ok(&["shape", "set", p, "6", "--points", "0,0 10,0 10,5 0,5"]);
    refused(
        &["shape", "set", p, "4", "--bulges", "0"],
        "--bulges doesn't apply to a circle",
    );
    ok(&["shape", "delete", p, "7"]);

    // Deleting a shape an operation uses: written, the operation is flagged, exit code 1.
    let out = chipload(&["shape", "delete", p, "2"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).contains("Operation 1 (Holes): shape S2 no longer exists"));
    let listed = ok(&["inspect", p]);
    assert!(listed.contains("Shapes: 5 drawn"), "{listed}");
    let run = chipload(&["run", p, "-o", path(&dir.join("x.nc"))]);
    assert_eq!(run.status.code(), Some(1));
    // A deleted id is never handed out again.
    let added = ok(&[
        "shape", "add", p, "circle", "--center", "75,80", "--radius", "12",
    ]);
    assert!(added.contains("Added shape S8."), "{added}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn imported_shapes_cannot_be_changed_from_the_command_line() {
    let dir = temp_dir("imported-shape");
    let project = dir.join("p.chipload");
    std::fs::copy(corpus("jobs/v4/simple_part_full.chipload"), &project).unwrap();
    let out = chipload(&["shape", "set", path(&project), "1", "--radius", "3"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("was imported from the DXF"));
    let out = chipload(&["shape", "delete", path(&project), "1"]);
    assert_eq!(out.status.code(), Some(1));
    // Refused before writing: the project is unchanged.
    assert_eq!(
        std::fs::read_to_string(&project).unwrap(),
        std::fs::read_to_string(corpus("jobs/v4/simple_part_full.chipload")).unwrap()
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn plan_json_renders_with_arcs_through_the_post() {
    // docs/specs/arc-output.md section 6: `plan --json` carries the arc tags, `render` merges
    // them when the post writes arcs.
    let dir = temp_dir("plan-render");
    let out = chipload(&[
        "plan",
        path(&corpus("jobs/v5/drawn_part.chipload")),
        "--json",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let plan: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let program = dir.join("program.json");
    std::fs::write(&program, plan["program"].to_string()).unwrap();
    let out = chipload(&["render", path(&program)]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let gcode = text(&out.stdout);
    assert!(
        gcode.contains("G3 X75.000 Y89.000 I0.000 J9.000"),
        "{gcode}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_project_started_empty_is_drawn_in_run_and_gets_its_dxf_later() {
    // docs/specs/new-project.md
    let dir = temp_dir("new-project");
    let project = dir.join("p.chipload");
    let p = path(&project);
    let ok = |args: &[&str]| {
        let out = chipload(args);
        assert!(out.status.success(), "{args:?}: {}", text(&out.stderr));
        text(&out.stdout)
    };
    let out = ok(&["new", "-o", p]);
    assert!(out.contains("New empty project."), "{out}");
    let job: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
    assert_eq!(job["drawing"]["shapes"].as_array().unwrap().len(), 0);
    assert!(job["drawing"]["source"].is_null());
    assert_eq!(job["stock"]["x"], 300.0);
    assert_eq!(job["stock"]["y"], 200.0);
    assert_eq!(job["stock"]["origin"], "bottom_left");
    assert!(ok(&["inspect", p]).contains("Contours (0)"));

    // Draw, cut it out, run: the program passes the checker.
    let added = ok(&[
        "shape", "add", p, "circle", "--center", "20,20", "--radius", "10",
    ]);
    assert!(added.contains("Added shape S1."), "{added}");
    let mut job: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
    job["operations"] = serde_json::json!([{
        "type": "profile", "name": "Disc", "enabled": true,
        "select": {"layer": null, "shapes": [1]},
        "side": "outside", "direction": "climb", "tool": 1,
        "start_depth": 0.0, "end_depth": 10.0, "clearance": -0.3, "step_down": 3.0,
        "feed": 1500.0, "plunge_feed": 400.0, "tabs": null
    }]);
    std::fs::write(&project, serde_json::to_string_pretty(&job).unwrap()).unwrap();
    ok(&["run", p, "-o", path(&dir.join("disc.nc"))]);

    // The DXF comes later: its shapes are added, the drawn circle keeps its id.
    let dxf = corpus("dxf/simple_part.dxf");
    let out = ok(&["update", p, "--from", path(&dxf)]);
    assert!(
        out.contains("0 unchanged, 0 changed, 7 added, 0 removed"),
        "{out}"
    );
    assert!(!out.contains("beyond the stock"), "{out}");
    let job: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
    let shapes = job["drawing"]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 8);
    assert!(shapes.iter().any(|s| s["id"] == 1 && s["kind"] == "circle"));
    assert_eq!(job["drawing"]["source"]["file"], "simple_part.dxf");
    assert_eq!(job["stock"]["x"], 300.0, "the stock is left as it is");
    ok(&["run", p, "-o", path(&dir.join("disc2.nc"))]);

    // A drawing that doesn't fit the stock is pointed out.
    let mut job = job;
    job["stock"]["x"] = 100.0.into();
    std::fs::write(&project, serde_json::to_string_pretty(&job).unwrap()).unwrap();
    let out = ok(&["update", p, "--from", path(&dxf)]);
    assert!(
        out.contains("to 250.000, Y")
            && out.contains("extends beyond the stock (100.000 × 200.000 mm)"),
        "{out}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn copies_are_set_from_the_command_line() {
    // docs/specs/copies-grid.md §5
    let dir = temp_dir("copies");
    let project = dir.join("p.chipload");
    std::fs::copy(corpus("jobs/v7/ramp_entry.chipload"), &project).unwrap();
    let p = path(&project);
    let run = |args: &[&str]| {
        let out = chipload(args);
        (out.status.code(), text(&out.stdout), text(&out.stderr))
    };
    let copies = || {
        let job: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
        job["copies"].clone()
    };

    let (code, _, err) = run(&["copies", p, "--pitch", "200,150"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("give --grid"), "{err}");
    let (code, _, err) = run(&["copies", p, "--grid", "3x2"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--pitch DX,DY or --gap"), "{err}");
    let (code, _, err) = run(&["copies", p, "--grid", "3by2", "--pitch", "1,1"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("columns x rows like 3x2"), "{err}");
    assert!(copies().is_null(), "refused: nothing written");

    let (code, out, _) = run(&["copies", p, "--grid", "3x2", "--pitch", "200,-150"]);
    assert_eq!(code, Some(0));
    assert!(
        out.contains("Copies: 3 × 2 copies, pitch 200.000 × -150.000 mm, operation by operation"),
        "{out}"
    );
    assert_eq!(
        copies(),
        serde_json::json!({"columns": 3, "rows": 2, "dx": 200.0, "dy": -150.0,
                           "order": "by_operation"})
    );
    // Only what is given changes.
    run(&["copies", p, "--order", "by-part"]);
    assert_eq!(copies()["order"], "by_part");
    assert_eq!(copies()["dx"], 200.0);
    // A gap: the cut's size plus the gap.
    let (_, out, _) = run(&["copies", p, "--gap", "10"]);
    // It keeps the direction: the rows still grow down.
    assert!(
        out.contains("pitch 154.665 × -122.221 mm, part by part"),
        "{out}"
    );
    assert!(ok_inspect(p).contains("Copies: 3 × 2 copies"));
    // Overlap is only a warning here; running refuses it.
    let (code, _, err) = run(&["copies", p, "--pitch", "100,150"]);
    assert_eq!(code, Some(0));
    assert!(err.contains("warning: Copies overlap"), "{err}");
    let (code, _, err) = run(&["run", p, "-o", path(&dir.join("x.nc"))]);
    assert_eq!(code, Some(1));
    assert!(err.contains("Copies overlap"), "{err}");
    let (_, out, _) = run(&["copies", p, "--off"]);
    assert!(out.contains("Copies: off (one part)"), "{out}");
    assert!(copies().is_null());
    std::fs::remove_dir_all(&dir).unwrap();
}

fn ok_inspect(project: &str) -> String {
    let out = chipload(&["inspect", project]);
    assert!(out.status.success());
    text(&out.stdout)
}
