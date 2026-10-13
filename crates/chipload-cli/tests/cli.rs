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
    assert_eq!(job["drawing"]["dxfs"], serde_json::json!([]));
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
    assert!(out.contains("as DXF 1 \"simple_part\": 7 shapes"), "{out}");
    assert!(!out.contains("beyond the stock"), "{out}");
    let job: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
    let shapes = job["drawing"]["shapes"].as_array().unwrap();
    assert_eq!(shapes.len(), 8);
    assert!(shapes.iter().any(|s| s["id"] == 1 && s["kind"] == "circle"));
    assert_eq!(job["drawing"]["dxfs"][0]["file"], "simple_part.dxf");
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

#[test]
fn start_points_are_shown_by_plan_and_pass_the_checker() {
    // docs/specs/helix-and-leads.md part A
    let dir = temp_dir("starts");
    let project = dir.join("p.chipload");
    let p = path(&project);
    let ok = |args: &[&str]| {
        let out = chipload(args);
        assert!(out.status.success(), "{args:?}: {}", text(&out.stderr));
        text(&out.stdout)
    };
    ok(&["new", "-o", p]);
    ok(&[
        "shape", "add", p, "circle", "--center", "50,50", "--radius", "20",
    ]);
    ok(&[
        "shape",
        "add",
        p,
        "rectangle",
        "--center",
        "150,50",
        "--length",
        "60",
        "--width",
        "40",
    ]);
    let mut job: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
    job["operations"] = serde_json::json!([
        {"type": "pocket", "select": {"shapes": [2]}, "tool": 1, "start_depth": 0.0,
         "end_depth": 4.0, "clearance": 0.0, "step_down": 2.0, "stepover": 0.4,
         "direction": "climb", "feed": 1500.0, "plunge_feed": 400.0,
         "starts": [[150.0, 75.0]], "round_corners": false},
        {"type": "profile", "select": {"shapes": [1]}, "side": "outside",
         "direction": "climb", "tool": 1, "start_depth": 0.0, "end_depth": 10.0,
         "clearance": 0.0, "step_down": 5.0, "feed": 1500.0, "plunge_feed": 400.0,
         "tabs": {"count": 3, "width": 6.0, "height": 2.0, "flank_angle": std::f64::consts::FRAC_PI_2}, "starts": [[90.0, 90.0]]}
    ]);
    std::fs::write(&project, serde_json::to_string_pretty(&job).unwrap()).unwrap();
    let out = ok(&["plan", p]);
    // The pocket's wall ring runs 3 mm inside the 60 x 40 rectangle: its top is y = 67.
    assert!(
        out.contains("Operation 1: a ring starts at (150.000, 67.000) (start point 1)"),
        "{out}"
    );
    // On the circle 3 mm outside the disc, towards (90, 90): 50 + 23 / √2.
    assert!(
        out.contains("Operation 2: a ring starts at (66.263, 66.263) (start point 1)"),
        "{out}"
    );
    // Arcs split at the start still pass the independent checker.
    let nc = dir.join("p.nc");
    ok(&["run", p, "-o", path(&nc)]);
    let gcode = std::fs::read_to_string(&nc).unwrap();
    assert!(gcode.contains("G2") || gcode.contains("G3"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_imported_drawing_is_placed_from_the_command_line() {
    // docs/specs/drawing-placement.md §4
    let dir = temp_dir("place");
    let project = dir.join("p.chipload");
    std::fs::copy(corpus("jobs/v7/simple_part_full.chipload"), &project).unwrap();
    let p = path(&project);
    let run = |args: &[&str]| {
        let out = chipload(args);
        (out.status.code(), text(&out.stdout), text(&out.stderr))
    };
    let placement = || {
        let job: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
        job["drawing"]["dxfs"][0]["placement"].clone()
    };
    let extents = || {
        ok_inspect(p)
            .lines()
            .find(|l| l.starts_with("Extents"))
            .unwrap()
            .to_string()
    };

    let (code, out, _) = run(&["place", p]);
    assert_eq!(code, Some(0));
    assert!(out.contains("Drawing: as in the DXF"), "{out}");
    assert!(!ok_inspect(p).contains("Drawing:"), "only shown when moved");

    let (code, _, err) = run(&["place", p, "--margin", "5"]);
    assert_eq!(code, Some(1));
    assert!(
        err.contains("--margin goes with --align bottom-left"),
        "{err}"
    );
    let (code, _, err) = run(&["place", p, "--align", "center", "--margin", "5"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--margin"), "{err}");
    let (code, _, err) = run(&["place", p, "--by", "1"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--by: expected a point"), "{err}");

    let (code, out, _) = run(&["place", p, "--by", "-20,30"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("Moved the imported drawing by X-20.000 Y30.000"),
        "{out}"
    );
    assert_eq!(
        placement(),
        serde_json::json!({"scale": 1.0, "quarter_turns": 0, "dx": -20.0, "dy": 30.0})
    );
    assert!(extents().contains("X 30.000 .. 230.000, Y 80.000 .. 200.000"));
    assert!(ok_inspect(p).contains("Drawing: moved X-20.000 Y30.000 from the DXF"));

    // Stock 300 × 220, drawing 200 × 120.
    run(&["place", p, "--align", "bottom-left", "--margin", "10"]);
    assert!(extents().contains("X 10.000 .. 210.000, Y 10.000 .. 130.000"));
    run(&["place", p, "--align", "center"]);
    assert!(extents().contains("X 50.000 .. 250.000, Y 50.000 .. 170.000"));
    assert_eq!(
        placement(),
        serde_json::json!({"scale": 1.0, "quarter_turns": 0, "dx": 0.0, "dy": 0.0})
    );
    run(&["place", p, "--to", "5,-5"]);
    assert_eq!(
        placement(),
        serde_json::json!({"scale": 1.0, "quarter_turns": 0, "dx": 5.0, "dy": -5.0})
    );

    // An update keeps it where it was put.
    let (code, out, _) = run(&["update", p, "--from", path(&corpus("dxf/simple_part.dxf"))]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("7 unchanged"), "{out}");
    assert!(out.contains("Placed as before"), "{out}");
    assert!(extents().contains("X 55.000 .. 255.000, Y 45.000 .. 165.000"));

    // And it still runs through the checker.
    let (code, _, err) = run(&["run", p, "-o", path(&dir.join("x.nc"))]);
    assert_eq!(code, Some(0), "{err}");

    // Nothing imported, nothing to move.
    let empty = dir.join("empty.chipload");
    run(&["new", "-o", path(&empty)]);
    let (code, _, err) = run(&["place", path(&empty), "--by", "1,1"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("no imported drawing"), "{err}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_imported_drawing_is_turned_from_the_command_line() {
    // docs/specs/drawing-rotation.md §4
    let dir = temp_dir("rotate");
    let project = dir.join("p.chipload");
    std::fs::copy(corpus("jobs/v7/simple_part_full.chipload"), &project).unwrap();
    let p = path(&project);
    let run = |args: &[&str]| {
        let out = chipload(args);
        (out.status.code(), text(&out.stdout), text(&out.stderr))
    };
    let extents = || {
        ok_inspect(p)
            .lines()
            .find(|l| l.starts_with("Extents"))
            .unwrap()
            .to_string()
    };

    let (code, _, err) = run(&["place", p, "--rotate", "45"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("steps of 90 degrees"), "{err}");
    // A usage error (clap): the offset would be unclear after a turn.
    let (code, _, err) = run(&["place", p, "--rotate", "90", "--by", "1,1"]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("cannot be used with"), "{err}");

    // 200 × 120 centred on (150, 110): turned in place it is 120 × 200.
    let (code, out, _) = run(&["place", p, "--rotate", "90"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("Turned the imported drawing 90° counter-clockwise"),
        "{out}"
    );
    assert!(extents().contains("X 90.000 .. 210.000, Y 10.000 .. 210.000"));
    let (_, out, _) = run(&["place", p]);
    assert!(
        out.contains("Drawing: turned 90°, moved X260.000 Y-40.000 from the DXF"),
        "{out}"
    );
    // Stock 300 × 220: turned and put in the corner, it fits.
    let (code, out, _) = run(&[
        "place",
        p,
        "--rotate",
        "-90",
        "--align",
        "bottom-left",
        "--margin",
        "10",
    ]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("90° clockwise"), "{out}");
    assert!(extents().contains("X 10.000 .. 210.000, Y 10.000 .. 130.000"));
    let (code, out, _) = run(&["place", p, "--rotate", "270"]);
    assert_eq!(code, Some(0), "{out}");
    let (code, out, _) = run(&["place", p, "--align", "bottom-left", "--margin", "10"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(extents().contains("X 10.000 .. 130.000, Y 10.000 .. 210.000"));
    let (code, _, err) = run(&["run", p, "-o", path(&dir.join("x.nc"))]);
    assert_eq!(code, Some(0), "{err}");
    let (code, out, _) = run(&["place", p, "--reset"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(extents().contains("X 50.000 .. 250.000, Y 50.000 .. 170.000"));
    let (_, out, _) = run(&["place", p]);
    assert!(out.contains("Drawing: as in the DXF"), "{out}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_imported_drawing_is_scaled_from_the_command_line() {
    // docs/specs/drawing-scale.md §4
    let dir = temp_dir("scale");
    let project = dir.join("p.chipload");
    std::fs::copy(corpus("jobs/v7/simple_part_full.chipload"), &project).unwrap();
    let p = path(&project);
    let run = |args: &[&str]| {
        let out = chipload(args);
        (out.status.code(), text(&out.stdout), text(&out.stderr))
    };
    let extents = || {
        ok_inspect(p)
            .lines()
            .find(|l| l.starts_with("Extents"))
            .unwrap()
            .to_string()
    };

    for bad in ["0", "-1"] {
        let (code, _, err) = run(&["place", p, "--scale", bad]);
        assert_eq!(code, Some(1), "{bad}: {err}");
        assert!(err.contains("greater than 0"), "{err}");
    }
    let (code, _, err) = run(&["place", p, "--scale", "5000"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("between ×0.001 and ×1000"), "{err}");
    // Usage errors (clap): one size at a time, and no offsets with it.
    for args in [
        ["--scale", "2", "--width", "100"],
        ["--width", "100", "--height", "50"],
        ["--scale", "2", "--by", "1,1"],
    ] {
        let mut all = vec!["place", p];
        all.extend(args);
        let (code, _, err) = run(&all);
        assert_eq!(code, Some(2), "{args:?}: {err}");
        assert!(err.contains("cannot be used with"), "{err}");
    }

    // 200 × 120 centred on (150, 110).
    let (code, out, _) = run(&["place", p, "--scale", "0.5"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("Scaled the imported drawing ×0.5 (now 100.000 × 60.000 mm)"),
        "{out}"
    );
    assert!(extents().contains("X 100.000 .. 200.000, Y 80.000 .. 140.000"));
    let (_, out, _) = run(&["place", p]);
    assert!(
        out.contains("Drawing: scaled ×0.5, moved X75.000 Y55.000 from the DXF"),
        "{out}"
    );
    assert!(out.contains("Size: 100.000 × 60.000 mm"), "{out}");
    assert!(ok_inspect(p).contains("Drawing: scaled ×0.5,"));
    // Turn, then scale to a width, then align: 60 wide, 100 high, in the corner.
    let (code, out, _) = run(&[
        "place",
        p,
        "--rotate",
        "90",
        "--width",
        "60",
        "--align",
        "bottom-left",
        "--margin",
        "10",
    ]);
    assert_eq!(code, Some(0), "{out}");
    assert!(extents().contains("X 10.000 .. 70.000, Y 10.000 .. 110.000"));
    let (code, out, _) = run(&["place", p, "--height", "150"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("(now 90.000 × 150.000 mm)"), "{out}");
    let (code, _, err) = run(&["run", p, "-o", path(&dir.join("x.nc"))]);
    assert_eq!(code, Some(0), "{err}");
    let (code, out, _) = run(&["place", p, "--reset"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(extents().contains("X 50.000 .. 250.000, Y 50.000 .. 170.000"));
    let (_, out, _) = run(&["place", p]);
    assert!(out.contains("Drawing: as in the DXF"), "{out}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_project_with_two_tools_writes_one_checked_file_per_tool() {
    // docs/specs/tool-per-operation.md §3.1, §6, §7.
    let dir = temp_dir("two-tools");
    let project = dir.join("two.chipload");
    std::fs::copy(corpus("jobs/v11/two_tools.chipload"), &project).unwrap();
    let out_path = dir.join("part.nc");
    let out = chipload(&["run", path(&project), "-o", path(&out_path)]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    let (first, second) = (dir.join("part-1-T2.nc"), dir.join("part-2-T1.nc"));
    for (file, tool, ops) in [(&first, 2, "1"), (&second, 1, "2")] {
        assert!(
            stdout.contains(&format!("Wrote {} (", file.display()))
                && stdout.contains(&format!("lines, tool {tool}, operations {ops})")),
            "{stdout}"
        );
        // Each file passes the checker with its own tool, and with the largest.
        let tool = tool.to_string();
        for extra in [&["--tool", tool.as_str()][..], &[][..]] {
            let mut args = vec!["check", path(file), "--job", path(&project)];
            args.extend_from_slice(extra);
            let out = chipload(&args);
            assert!(out.status.success(), "{}", text(&out.stdout));
        }
    }
    assert!(stdout.contains("2 files, post GRBL: change the tool and set Z zero"));
    assert!(
        !out_path.exists(),
        "no single file for a project with two tools"
    );
    let out = chipload(&[
        "check",
        path(&first),
        "--job",
        path(&project),
        "--tool",
        "9",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("the project has no tool 9"));

    // plan: the groups, and render refuses a plan with tool changes.
    let out = chipload(&["plan", path(&project), "--json"]);
    let plan: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(plan["groups"][0]["tool"], 2);
    assert_eq!(plan["groups"][0]["operations"], serde_json::json!([0]));
    assert_eq!(plan["groups"][1]["tool"], 1);
    assert_eq!(plan["program"]["tool_groups"], 2);
    let program = dir.join("program.json");
    std::fs::write(&program, plan["program"].to_string()).unwrap();
    let out = chipload(&["render", path(&program)]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("this plan uses 2 tool groups; use run"));
    let out = chipload(&["plan", path(&project)]);
    assert!(text(&out.stdout).contains("Tool group 1: tool 2, operations 1"));

    // op tool: both operations with tool 1 give one file again.
    let out = chipload(&["op", "tool", path(&project), "--op", "1", "--tool", "1"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("Operation 1 now cuts with tool 1."));
    let out = chipload(&["run", path(&project), "-o", path(&out_path)]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(out_path.exists());
    for (args, message) in [
        (
            ["--op", "1", "--tool", "7"],
            "the project has no tool 7; copy one in",
        ),
        (
            ["--op", "3", "--tool", "2"],
            "there is no operation 3; the project has 2",
        ),
        (["--op", "0", "--tool", "2"], "there is no operation 0"),
    ] {
        let mut all = vec!["op", "tool", path(&project)];
        all.extend_from_slice(&args);
        let out = chipload(&all);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(text(&out.stderr).contains(message), "{}", text(&out.stderr));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn tool_changes_in_one_file_from_the_machine_or_for_one_run() {
    // docs/specs/tool-per-operation.md §3.2, §3.3.
    let dir = temp_dir("tool-change");
    let out_path = dir.join("part.nc");
    // The v12 sample's machine says "program": one file.
    let project = corpus("jobs/v12/two_tools_program.chipload");
    let out = chipload(&["run", path(&project), "-o", path(&out_path)]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let gcode = std::fs::read_to_string(&out_path).unwrap();
    assert!(gcode.contains("\nT2 M6\n") && gcode.contains("\nT1 M6\n"));
    let out = chipload(&["check", path(&out_path), "--job", path(&project)]);
    assert!(out.status.success(), "{}", text(&out.stdout));
    std::fs::remove_file(&out_path).unwrap();
    // The same for one run of the v11 sample, whose machine says "files".
    let project = corpus("jobs/v11/two_tools.chipload");
    let out = chipload(&[
        "run",
        path(&project),
        "-o",
        path(&out_path),
        "--tool-change",
        "program",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(std::fs::read_to_string(&out_path).unwrap(), gcode);
    assert!(!dir.join("part-1-T2.nc").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn plan_and_run_say_how_long_the_job_takes() {
    // docs/specs/cycle-time.md §4
    let project = corpus("jobs/v11/two_tools.chipload");
    let out = chipload(&["plan", path(&project), "--json"]);
    let plan: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let estimate = &plan["estimate"];
    assert!(estimate["total_s"].as_f64().unwrap() > estimate["cutting_s"].as_f64().unwrap());
    assert_eq!(estimate["tool_changes"], 1);
    assert_eq!(estimate["operations"].as_array().unwrap().len(), 2);
    assert_eq!(estimate["groups_s"].as_array().unwrap().len(), 2);
    assert_eq!(estimate["rapid_assumed"], false);
    let out = chipload(&["plan", path(&project)]);
    let said = text(&out.stdout);
    assert!(said.contains("Estimated time: "), "{said}");
    assert!(said.contains("plus 1 tool change"), "{said}");
    assert!(said.contains("  File 2: "), "{said}");
}

#[test]
fn run_writes_a_setup_sheet_next_to_the_gcode() {
    // docs/specs/setup-sheet.md §4
    let dir = temp_dir("sheet");
    let project = corpus("jobs/v11/two_tools.chipload");
    let out = chipload(&["run", path(&project), "-o", path(&dir.join("part.nc"))]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("part-sheet.html (setup sheet)"));
    let sheet = std::fs::read_to_string(dir.join("part-sheet.html")).unwrap();
    assert!(sheet.contains("1. part-1-T2.nc") && sheet.contains("2. part-2-T1.nc"));
    assert!(sheet.contains("Setup sheet: two_tools.chipload"));
    assert!(sheet.contains(concat!("written by chipload ", env!("CARGO_PKG_VERSION"))));

    // --no-sheet: only the G-code.
    let other = temp_dir("no-sheet");
    let out = chipload(&[
        "run",
        path(&project),
        "-o",
        path(&other.join("part.nc")),
        "--no-sheet",
    ]);
    assert!(out.status.success());
    let mut names: Vec<String> = std::fs::read_dir(&other)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["part-1-T2.nc", "part-2-T1.nc"]);

    // `sheet`: only the sheet, named for the G-code `run` would write.
    let alone = temp_dir("sheet-alone");
    let out = chipload(&[
        "sheet",
        path(&project),
        "-o",
        path(&alone.join("setup.html")),
        "--tool-change",
        "program",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let names: Vec<_> = std::fs::read_dir(&alone).unwrap().collect();
    let sheet = std::fs::read_to_string(alone.join("setup.html")).unwrap();
    for d in [dir, other, alone] {
        std::fs::remove_dir_all(d).unwrap();
    }
    assert_eq!(names.len(), 1);
    assert!(sheet.contains("1. two_tools.nc"), "one file with M6");
    assert!(sheet.contains("Tool change (M6) to T1"));
}

#[test]
fn a_refused_job_gets_no_sheet() {
    let dir = temp_dir("sheet-refused");
    let job = dir.join("job.chipload");
    let mut data: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(corpus("jobs/v4/simple_part_full.chipload")).unwrap(),
    )
    .unwrap();
    // The part is 206 mm wide: the checker refuses the program.
    data["machine"]["travel"] = serde_json::json!({"x": 100, "y": 600, "z": 100});
    std::fs::write(&job, data.to_string()).unwrap();
    let out = chipload(&["sheet", path(&job), "-o", path(&dir.join("s.html"))]);
    let run = chipload(&["run", path(&job), "-o", path(&dir.join("out.nc"))]);
    let count = std::fs::read_dir(&dir).unwrap().count();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(run.status.code(), Some(1));
    assert_eq!(count, 1, "only the project");
}

#[test]
fn shapes_are_copied_and_mirrored_from_the_command_line() {
    // docs/specs/small-items.md §4
    let dir = temp_dir("shape-copy");
    let project = dir.join("part.chipload");
    std::fs::copy(corpus("jobs/v5/drawn_part.chipload"), &project).unwrap();
    let p = path(&project);
    let out = chipload(&["shape", "copy", p, "2", "3", "--dx", "-20", "--dy", "5"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("Added shapes S4, S5."));
    let out = chipload(&["shape", "mirror", p, "1", "4", "--axis", "y"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("Mirrored shapes S1, S4."));
    let job = chipload_job::load_job(&project).unwrap();
    assert_eq!(job.drawing.shapes.len(), 5);
    // The project still runs: the operations follow their shapes.
    let run = chipload(&["run", p, "-o", path(&dir.join("x.nc")), "--no-sheet"]);
    let refused = chipload(&["shape", "copy", p, "9", "--dx", "1", "--dy", "1"]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(!refused.status.success());
    assert!(text(&refused.stderr).contains("there is no shape 9"));
}

#[test]
fn plan_shows_the_chip_load() {
    // docs/specs/small-items.md §5: two_tools has 2-flute tools at 20000 and 18000 rpm.
    let project = corpus("jobs/v11/two_tools.chipload");
    let said = text(&chipload(&["plan", path(&project)]).stdout);
    assert!(said.contains("Chip load (at the cutting feed):"), "{said}");
    assert!(
        said.contains("  Operation 1 (Holes): 0.030 mm/tooth"),
        "{said}"
    );
    assert!(
        said.contains("  Operation 2 (Outline): 0.050 mm/tooth"),
        "{said}"
    );
    let out = chipload(&["plan", path(&project), "--json"]);
    let plan: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(plan["chip_load"][1]["operation"], 1);
    let mm = plan["chip_load"][1]["chip_load_mm"].as_f64().unwrap();
    assert!((mm - 0.05).abs() < 1e-12, "{mm}");
}

#[test]
fn removed_pocket_strategies_load_but_are_refused() {
    // Trochoidal and adaptive were removed (docs/decisions/0017-scope.md): projects using
    // them still load (inspect works, nothing is changed), and generating is refused with
    // what to do.
    for (job, name) in [
        ("jobs/v12/adaptive.chipload", "adaptive"),
        ("jobs/v7/trochoidal.chipload", "trochoidal"),
    ] {
        let out = chipload(&["inspect", path(&corpus(job))]);
        assert!(out.status.success(), "{job}: {}", text(&out.stderr));
        let out = chipload(&["plan", path(&corpus(job))]);
        let said = text(&out.stdout) + &text(&out.stderr);
        assert!(!out.status.success(), "{job}: {said}");
        assert!(
            said.contains(&format!(
                "{name} clearing was removed in Chipload 0.14; switch the pocket to rings"
            )),
            "{job}: {said}"
        );
    }
}

#[test]
fn op_rest_sets_and_removes_rest_machining() {
    // docs/specs/rest-machining.md §6.
    let dir = temp_dir("op-rest");
    let project = dir.join("rest.chipload");
    std::fs::copy(corpus("jobs/v18/rest.chipload"), &project).unwrap();
    let out_path = dir.join("rest.nc");
    let plan = |project: &Path| text(&chipload(&["plan", path(project)]).stdout);
    let rested = plan(&project);
    assert!(rested.contains("Operation 2 (Rest): 49 s"), "{rested}");

    let out = chipload(&["op", "rest", path(&project), "--op", "2", "--none"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("Operation 2 now clears the whole pocket."));
    let whole = plan(&project);
    assert!(!whole.contains("Operation 2 (Rest): 49 s"), "{whole}");

    let out = chipload(&[
        "op",
        "rest",
        path(&project),
        "--op",
        "2",
        "--after-tool",
        "1",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("Operation 2 now rest machines after tool 1."));
    assert_eq!(plan(&project), rested);
    let out = chipload(&["run", path(&project), "-o", path(&out_path)]);
    assert!(out.status.success(), "{}", text(&out.stderr));

    // The larger tool is checked with the job's rules.
    let out = chipload(&[
        "op",
        "rest",
        path(&project),
        "--op",
        "1",
        "--after-tool",
        "2",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let out = chipload(&["plan", path(&project)]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains(
            "rest after tool 2 (4 mm) needs a tool larger than this \
             pocket's (12 mm)"
        ),
        "{}",
        text(&out.stderr)
    );
    for (args, message) in [
        (
            vec!["--op", "2", "--after-tool", "7"],
            "the project has no tool 7",
        ),
        (
            vec!["--op", "3", "--after-tool", "1"],
            "there is no operation 3; the project has 2",
        ),
    ] {
        let mut all = vec!["op", "rest", path(&project)];
        all.extend_from_slice(&args);
        let out = chipload(&all);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(text(&out.stderr).contains(message), "{}", text(&out.stderr));
    }
    // One of --after-tool and --none.
    let out = chipload(&["op", "rest", path(&project), "--op", "2"]);
    assert_eq!(out.status.code(), Some(2));

    // op add-rest: a copy of pocket 1 with tool 2, after it (§10).
    let fresh = dir.join("fresh.chipload");
    let out = chipload(&[
        "set",
        path(&project),
        "operations.0.rest=null",
        "-o",
        path(&fresh),
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let out = chipload(&["op", "add-rest", path(&fresh), "--op", "1", "--tool", "2"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout)
            .contains("Added operation 2 (Rough (rest)), rest after operation 1's tool"),
        "{}",
        text(&out.stdout)
    );
    let out = chipload(&["get", path(&fresh), "operations.1.rest"]);
    assert_eq!(text(&out.stdout), "{\n  \"after_tool\": 1\n}\n");
    let out = chipload(&["op", "add-rest", path(&fresh), "--op", "2", "--tool", "2"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("it is a rest pocket itself"));
    std::fs::remove_dir_all(&dir).unwrap();
}
