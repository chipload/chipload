// SPDX-License-Identifier: GPL-3.0-or-later
//! `shape scale --reference`, `shape stretch` and `place --reference`
//! (docs/specs/scale-and-stretch.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// A fresh folder each run (process numbers repeat between runs in a sandbox).
fn temp_dir(name: &str) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "chipload-scaling-{name}-{}-{now}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(dir)
        .arg("--json")
        .args(args)
        .output()
        .unwrap()
}

/// `chipload --json ARGS` in `dir`: its JSON, which must say ok.
fn ok(dir: &Path, args: &[&str]) -> Value {
    let out = run(dir, args);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["ok"], Value::Bool(true), "{args:?}: {value}");
    value
}

fn get(dir: &Path, path: &str) -> Value {
    ok(dir, &["get", "p.chipload", path])["value"].clone()
}

fn point(value: &Value) -> [f64; 2] {
    [value[0].as_f64().unwrap(), value[1].as_f64().unwrap()]
}

#[test]
fn shapes_scale_by_a_reference_the_first_point_staying() {
    let dir = temp_dir("reference");
    ok(&dir, &["new", "-o", "p.chipload"]);
    ok(
        &dir,
        &[
            "shape",
            "add",
            "p.chipload",
            "polyline",
            "--open",
            "--points",
            "10,10 40,50",
        ],
    );
    // A reference on another shape: the second line, 20 long, to be 1 inch.
    ok(
        &dir,
        &[
            "shape",
            "add",
            "p.chipload",
            "polyline",
            "--open",
            "--points",
            "0,0 20,0",
        ],
    );
    let done = ok(
        &dir,
        &[
            "shape",
            "scale",
            "p.chipload",
            "1",
            "--reference",
            "10,10 40,50",
            "--length",
            "100",
        ],
    );
    assert!(done["notes"].to_string().contains("×2"), "{done}");
    let points = get(&dir, "drawing.shapes.0.points");
    assert_eq!(point(&points[0]), [10.0, 10.0]);
    assert_eq!(point(&points[1]), [70.0, 90.0]);
    ok(
        &dir,
        &[
            "shape",
            "scale",
            "p.chipload",
            "2",
            "--reference",
            "0,0 20,0",
            "--length",
            "1in",
        ],
    );
    assert_eq!(point(&get(&dir, "drawing.shapes.1.points")[1]), [25.4, 0.0]);

    // Refused: points on top of each other, a length of 0; usage: with --factor, or
    // --length alone.
    let same = run(
        &dir,
        &[
            "shape",
            "scale",
            "p.chipload",
            "1",
            "--reference",
            "5,5 5,5",
            "--length",
            "3",
        ],
    );
    assert_eq!(same.status.code(), Some(1));
    let zero = run(
        &dir,
        &[
            "shape",
            "scale",
            "p.chipload",
            "1",
            "--reference",
            "0,0 5,5",
            "--length",
            "0",
        ],
    );
    assert_eq!(zero.status.code(), Some(1));
    for args in [
        vec!["--factor", "2", "--reference", "0,0 1,1", "--length", "3"],
        vec!["--length", "3"],
        vec!["--reference", "0,0 1,1"],
    ] {
        let mut all = vec!["shape", "scale", "p.chipload", "1"];
        all.extend(args.iter());
        assert_eq!(run(&dir, &all).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn shapes_stretch_by_factors_and_sizes_about_a_point() {
    let dir = temp_dir("stretch");
    ok(&dir, &["new", "-o", "p.chipload"]);
    ok(
        &dir,
        &[
            "shape",
            "add",
            "p.chipload",
            "polyline",
            "--open",
            "--points",
            "0,0 10,4",
        ],
    );
    ok(
        &dir,
        &[
            "shape",
            "add",
            "p.chipload",
            "circle",
            "--center",
            "50,0",
            "--radius",
            "10",
        ],
    );
    // About the box's middle: (5, 2).
    ok(
        &dir,
        &["shape", "stretch", "p.chipload", "1", "--width", "20"],
    );
    let points = get(&dir, "drawing.shapes.0.points");
    assert_eq!(point(&points[0]), [-5.0, 0.0]);
    assert_eq!(point(&points[1]), [15.0, 4.0]);
    ok(
        &dir,
        &[
            "shape",
            "stretch",
            "p.chipload",
            "1",
            "--y",
            "2",
            "--about",
            "0,0",
        ],
    );
    assert_eq!(point(&get(&dir, "drawing.shapes.0.points")[1]), [15.0, 8.0]);

    // The circle becomes a polyline, and says so.
    let done = ok(&dir, &["shape", "stretch", "p.chipload", "2", "--x", "1.5"]);
    assert!(
        done["notes"].to_string().contains("S2: now a polyline"),
        "{done}"
    );
    assert_eq!(get(&dir, "drawing.shapes.1.kind"), "polyline");
    // Equal factors scale evenly: a circle stays a circle.
    ok(
        &dir,
        &[
            "shape",
            "add",
            "p.chipload",
            "circle",
            "--center",
            "0,50",
            "--radius",
            "5",
        ],
    );
    ok(
        &dir,
        &[
            "shape",
            "stretch",
            "p.chipload",
            "3",
            "--x",
            "2",
            "--y",
            "2",
        ],
    );
    assert_eq!(get(&dir, "drawing.shapes.2.kind"), "circle");
    assert_eq!(get(&dir, "drawing.shapes.2.radius"), 10.0);

    // Refused: a factor of 0; usage: --x with --width.
    let zero = run(&dir, &["shape", "stretch", "p.chipload", "1", "--x", "0"]);
    assert_eq!(zero.status.code(), Some(1));
    let both = run(
        &dir,
        &[
            "shape",
            "stretch",
            "p.chipload",
            "1",
            "--x",
            "2",
            "--width",
            "5",
        ],
    );
    assert_eq!(both.status.code(), Some(2));
}

#[test]
fn a_dxf_scales_by_a_reference() {
    let dir = temp_dir("dxf");
    let dxf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/dxf/simple_part.dxf");
    ok(&dir, &["import", dxf.to_str().unwrap(), "-o", "p.chipload"]);
    ok(
        &dir,
        &[
            "place",
            "p.chipload",
            "--reference",
            "0,0 10,0",
            "--length",
            "20",
        ],
    );
    assert_eq!(get(&dir, "drawing.dxfs.0.placement.scale"), 2.0);
    // Pictures and texts take their own sizes.
    ok(
        &dir,
        &[
            "text",
            "add",
            "p.chipload",
            "A",
            "--height",
            "10",
            "--at",
            "0,0",
        ],
    );
    let text = run(
        &dir,
        &[
            "place",
            "p.chipload",
            "--text",
            "1",
            "--reference",
            "0,0 1,0",
            "--length",
            "2",
        ],
    );
    assert_eq!(text.status.code(), Some(1));
}
