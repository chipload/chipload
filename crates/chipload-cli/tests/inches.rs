// SPDX-License-Identifier: GPL-3.0-or-later
//! Inches on the command line (docs/specs/inch-units.md §4, §5.3): lengths typed with a
//! unit, G-code in G20, and the setup sheet in inches.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-inches-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `chipload --json ARGS` in `dir`, and its JSON.
fn chipload(dir: &Path, args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(dir)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{args:?}: {}", String::from_utf8_lossy(&out.stderr)))
}

fn ok(dir: &Path, args: &[&str]) -> Value {
    let value = chipload(dir, args);
    assert_eq!(value["ok"], Value::Bool(true), "{args:?}: {value}");
    value
}

/// `value` is `mm` within rounding.
fn mm(value: &Value, mm: f64) {
    let v = value
        .as_f64()
        .unwrap_or_else(|| panic!("{value} is not a number"));
    assert!((v - mm).abs() < 1e-9, "{v} vs {mm}");
}

fn get(dir: &Path, path: &str) -> Value {
    ok(dir, &["get", "p.chipload", path])["value"].clone()
}

#[test]
fn lengths_take_a_unit_and_a_plain_number_is_mm() {
    let dir = temp_dir("lengths");
    ok(&dir, &["new", "-o", "p.chipload"]);
    ok(
        &dir,
        &[
            "shape",
            "add",
            "p.chipload",
            "rectangle",
            "--center",
            "2in,1in",
            "--length",
            "3in",
            "--width",
            "1 1/2in",
            "--corner-radius",
            "1/4in",
        ],
    );
    let shape = &get(&dir, "drawing.shapes.0");
    mm(&shape["center"][0], 50.8);
    mm(&shape["center"][1], 25.4);
    mm(&shape["length"], 76.2);
    mm(&shape["width"], 38.1);
    mm(&shape["corner_radius"], 6.35);
    ok(
        &dir,
        &[
            "text",
            "add",
            "p.chipload",
            "HI",
            "--height",
            "0.5in",
            "--at",
            "1in,12",
        ],
    );
    mm(&get(&dir, "drawing.texts.0.height"), 12.7);
    mm(&get(&dir, "drawing.texts.0.x"), 25.4);
    mm(&get(&dir, "drawing.texts.0.y"), 12.0);
    // A machine profile in inches, its feed in in/min.
    ok(
        &dir,
        &[
            "machine",
            "add",
            "Inchy",
            "--gcode-units",
            "inch",
            "--safe-z",
            "0.2in",
            "--max-feed",
            "200in/min",
            "--travel",
            "30in,30in,4in",
        ],
    );
    let machines = ok(&dir, &["machine", "list"]);
    let m = &machines["machines"][0];
    assert_eq!(m["gcode_units"], "inch");
    mm(&m["safe_z"], 5.08);
    mm(&m["max_feed"], 5080.0);
    mm(&m["travel"]["x"], 762.0);
    // Nonsense is refused, saying how to write it.
    let bad = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .current_dir(&dir)
        .args(["text", "add", "p.chipload", "X", "--height", "6cm"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    let message = String::from_utf8_lossy(&bad.stderr);
    assert!(message.contains("or with a unit (0.25in"), "{message}");
}

#[test]
fn an_inch_machine_writes_g20_and_the_sheet_can_be_in_inches() {
    let dir = temp_dir("run");
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v18/holes.chipload");
    std::fs::copy(corpus, dir.join("p.chipload")).unwrap();
    ok(&dir, &["set", "p.chipload", "machine.gcode_units=inch"]);
    ok(
        &dir,
        &["run", "p.chipload", "-o", "out.nc", "--sheet-units", "inch"],
    );
    let gcode = std::fs::read_to_string(dir.join("out.nc")).unwrap();
    assert!(gcode.contains("\nG20 G90"), "{gcode}");
    let sheet = std::fs::read_to_string(dir.join("out-sheet.html")).unwrap();
    assert!(sheet.contains("lengths in in, feeds in in/min"));
    assert!(sheet.contains("G-code in inches (G20)"));
    // `sheet` alone, in mm by default.
    ok(&dir, &["sheet", "p.chipload", "-o", "s.html"]);
    let sheet = std::fs::read_to_string(dir.join("s.html")).unwrap();
    assert!(sheet.contains("lengths in mm, feeds in mm/min"));
    ok(
        &dir,
        &["sheet", "p.chipload", "-o", "s.html", "--units", "inch"],
    );
    let sheet = std::fs::read_to_string(dir.join("s.html")).unwrap();
    assert!(sheet.contains("lengths in in, feeds in in/min"));
}
