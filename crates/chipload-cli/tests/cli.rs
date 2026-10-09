// SPDX-License-Identifier: GPL-3.0-or-later
//! The `chipload` command line (port of tests/unit/test_cli_*.py).
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
        path(&corpus("dxf/simple_part.dxf")),
        "--job",
        path(&corpus("jobs/v2/simple_part_full.json")),
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
        &std::fs::read_to_string(corpus("jobs/v2/simple_part_full.json")).unwrap(),
    )
    .unwrap();
    // The part is 206 mm wide: the checker refuses the program.
    data["machine"]["travel"] = serde_json::json!({"x": 100, "y": 600, "z": 100});
    std::fs::write(&job, data.to_string()).unwrap();
    let out_file = dir.join("out.nc");
    std::fs::write(&out_file, "old program\n").unwrap();
    let out = chipload(&[
        "run",
        path(&corpus("dxf/simple_part.dxf")),
        "--job",
        path(&job),
        "-o",
        path(&out_file),
    ]);
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
    let out = chipload(&[
        "run",
        path(&corpus("dxf/simple_part.dxf")),
        "--job",
        path(&job),
        "-o",
        path(&out_file),
    ]);
    let written = out_file.exists();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("schema_version 99 is not supported"));
    assert!(!written);
}
