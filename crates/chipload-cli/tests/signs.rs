// SPDX-License-Identifier: GPL-3.0-or-later
//! `sign add --size` (docs/specs/sign-size-and-start.md §1).
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
        "chipload-signs-{name}-{}-{now}",
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

fn ok(dir: &Path, args: &[&str]) -> Value {
    let out = run(dir, args);
    let value: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&out.stderr)));
    assert_eq!(value["ok"], Value::Bool(true), "{args:?}: {value}");
    value
}

const TOOLS: &str = r#"tools=[{"id":1,"type":"endmill","diameter":6.0,"rpm":18000.0},{"id":2,"type":"vbit","diameter":12.7,"angle":1.0471975511965976,"tip_diameter":0.2,"rpm":18000.0}]"#;

#[test]
fn a_sign_of_a_fixed_size_fits_its_letters() {
    let dir = temp_dir("size");
    ok(&dir, &["new", "-o", "p.chipload"]);
    ok(&dir, &["set", "p.chipload", TOOLS]);
    let tools = ["--vbit", "2", "--end-mill", "1"];
    let mut args = vec![
        "sign",
        "add",
        "p.chipload",
        "SMITH\\n12",
        "--size",
        "300x100",
    ];
    args.extend(tools);
    let made = ok(&dir, &args);
    let height = made["height"].as_f64().unwrap();
    assert!(height > 10.0 && height < 50.0, "{made}");
    assert!(
        made["notes"]
            .to_string()
            .contains("the most that fit 300 × 100 mm"),
        "{made}"
    );
    // Round words need a round size; --size and --height don't go together.
    let mut round = vec![
        "sign",
        "add",
        "p.chipload",
        "TOP\\nBOTTOM",
        "--frame",
        "oval",
        "--round",
        "--size",
        "200x150",
    ];
    round.extend(tools);
    assert_eq!(run(&dir, &round).status.code(), Some(1));
    let mut both = vec![
        "sign",
        "add",
        "p.chipload",
        "X",
        "--size",
        "200x100",
        "--height",
        "20",
    ];
    both.extend(tools);
    assert_eq!(run(&dir, &both).status.code(), Some(2));
}

#[test]
fn a_single_line_sign_is_engraved_at_the_depth_asked() {
    let dir = temp_dir("single-line");
    ok(&dir, &["new", "-o", "p.chipload"]);
    ok(&dir, &["set", "p.chipload", TOOLS]);
    ok(
        &dir,
        &[
            "sign",
            "add",
            "p.chipload",
            "SHED",
            "--font",
            "Relief SingleLine",
            "--engrave-depth",
            "0.6",
            "--vbit",
            "2",
            "--end-mill",
            "1",
        ],
    );
    let op = ok(&dir, &["get", "p.chipload", "operations.0"])["value"].clone();
    assert_eq!(op["type"], "path", "{op}");
    assert_eq!(op["end_depth"], 0.6);
}
