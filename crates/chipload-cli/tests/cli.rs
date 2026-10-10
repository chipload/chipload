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

fn chipload(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chipload"))
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
    assert!(added.contains("Added shape S7."), "{added}");
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
