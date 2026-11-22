// SPDX-License-Identifier: GPL-3.0-or-later
//! `op remove`, `op duplicate` and `op move`: what the app's Delete, Duplicate and
//! reordering do, from the command line.
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
    let dir =
        std::env::temp_dir().join(format!("chipload-ops-{name}-{}-{now}", std::process::id()));
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
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["ok"], Value::Bool(true), "{args:?}: {value}");
    value
}

fn names(dir: &Path) -> Vec<String> {
    ok(dir, &["get", "p.chipload", "operations"])["value"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["name"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn operations_are_removed_duplicated_and_moved() {
    let dir = temp_dir("edit");
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/jobs/v5/drawn_part.chipload");
    std::fs::copy(corpus, dir.join("p.chipload")).unwrap();
    let before = names(&dir);
    let copied = ok(&dir, &["op", "duplicate", "p.chipload", "--op", "1"]);
    assert_eq!(copied["op"], 2);
    let after = names(&dir);
    assert_eq!(after[1], format!("{} (copy)", before[0]));
    ok(
        &dir,
        &["op", "move", "p.chipload", "--op", "2", "--to", "1"],
    );
    assert_eq!(names(&dir)[0], format!("{} (copy)", before[0]));
    ok(&dir, &["op", "remove", "p.chipload", "--op", "1"]);
    assert_eq!(names(&dir), before);
    // Refused: no such operation, numbers from 1, moving to where it is.
    let count = before.len().to_string();
    let past = (before.len() + 1).to_string();
    for args in [
        vec!["op", "remove", "p.chipload", "--op", past.as_str()],
        vec!["op", "remove", "p.chipload", "--op", "0"],
        vec![
            "op",
            "move",
            "p.chipload",
            "--op",
            count.as_str(),
            "--to",
            count.as_str(),
        ],
    ] {
        assert_eq!(run(&dir, &args).status.code(), Some(1), "{args:?}");
    }
}
