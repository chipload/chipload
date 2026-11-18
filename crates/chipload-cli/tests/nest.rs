// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload nest` (docs/specs/nesting.md §8): the text and `--json`, a dry run, counts and
//! turns, the refusals, and `part set --turns`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn chipload(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    Run {
        code: out.status.code(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// A folder with the badge project (three instances) on a 400 × 300 board.
fn badge(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-nest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(
        corpus("jobs/v40/nest_badge.chipload"),
        dir.join("b.chipload"),
    )
    .unwrap();
    dir
}

fn json(run: &Run) -> Value {
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    serde_json::from_str(&run.stdout).unwrap()
}

#[test]
fn nest_places_every_instance_and_the_project_runs() {
    let dir = badge("places");
    let run = chipload(
        &dir,
        &["nest", "b.chipload", "--count", "1=5", "--turns", "1=none"],
    );
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout
            .starts_with("Nested setup 1 \"Setup 1\": 5 of 5 placed, in "),
        "{}",
        run.stdout
    );
    assert_eq!(run.stdout.matches("instance").count(), 5, "{}", run.stdout);
    assert!(
        !run.stdout.contains("turned"),
        "--turns none: {}",
        run.stdout
    );
    let list = chipload(&dir, &["get", "b.chipload", "parts.0.turns"]);
    assert_eq!(list.stdout.trim(), "\"none\"");
    let nest = chipload(&dir, &["get", "b.chipload", "setups.0.nest"]);
    assert!(nest.stdout.contains("\"gap\": 3.0"), "{}", nest.stdout);
    let out = chipload(&dir, &["run", "b.chipload", "-o", "b.nc", "--no-sheet"]);
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    let check = chipload(&dir, &["check", "b.nc", "--job", "b.chipload"]);
    assert!(check.stdout.contains("Check passed"), "{}", check.stdout);
}

#[test]
fn json_says_what_went_where_and_a_dry_run_writes_nothing() {
    let dir = badge("json");
    let before = std::fs::read_to_string(dir.join("b.chipload")).unwrap();
    let value = json(&chipload(
        &dir,
        &[
            "nest",
            "b.chipload",
            "--board",
            "200x150",
            "--count",
            "1=4",
            "--dry-run",
            "--json",
        ],
    ));
    assert_eq!(value["setup"], 1);
    let placed = value["placed"].as_array().unwrap();
    let unplaced = value["unplaced"].as_array().unwrap();
    assert_eq!(placed.len() + unplaced.len(), 4);
    assert!(!unplaced.is_empty(), "{value}");
    assert!(placed[0]["at"].is_array() && placed[0]["angle"].is_number());
    // The usable board (200 × 150 less the 10 mm margin) and the box round what is placed.
    assert_eq!(value["usable"], serde_json::json!([180.0, 130.0]));
    assert!(value["extent"].is_array() && value["used_pct"].is_number());
    assert_eq!(value["goal"], "compact");
    assert_eq!(value["unchanged"], false);
    assert_eq!(value["board_fixed"], false);
    assert!(value["written"].as_array().unwrap().is_empty());
    let notes = value["notes"].to_string();
    assert!(notes.contains("didn't fit: switched off"), "{notes}");
    assert_eq!(
        std::fs::read_to_string(dir.join("b.chipload")).unwrap(),
        before
    );
}

#[test]
fn a_board_that_follows_the_drawing_needs_a_size() {
    let dir = badge("fitted");
    let set = chipload(
        &dir,
        &[
            "set",
            "b.chipload",
            "setups.0.stock.origin=drawing",
            "setups.0.stock.fit=5",
        ],
    );
    assert_eq!(set.code, Some(0), "{}", set.stderr);
    let run = chipload(&dir, &["nest", "b.chipload"]);
    assert_ne!(run.code, Some(0));
    assert!(run.stderr.contains("--board"), "{}", run.stderr);
    let run = chipload(&dir, &["nest", "b.chipload", "--board", "600x400"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout.contains("it is now 600 × 400 mm"),
        "{}",
        run.stdout
    );
}

#[test]
fn bad_turns_and_counts_are_refused_before_anything_is_read() {
    let dir = badge("bad");
    for args in [
        &["nest", "b.chipload", "--turns", "sideways"][..],
        &["nest", "b.chipload", "--count", "5"],
        &["nest", "b.chipload", "--count", "1=0"],
        &["part", "set", "b.chipload", "1", "--turns", "eighth"],
    ] {
        let run = chipload(&dir, args);
        assert_ne!(run.code, Some(0), "{args:?}: {}", run.stdout);
        assert!(!run.stderr.is_empty(), "{args:?}");
    }
    let run = chipload(&dir, &["part", "set", "b.chipload", "1", "--turns", "half"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let turns = chipload(&dir, &["get", "b.chipload", "parts.0.turns"]);
    assert_eq!(turns.stdout.trim(), "\"half\"");
}

/// docs/specs/parts-and-setups.md §9: `--overflow new-boards` puts what doesn't fit onto
/// more boards; every board's program runs.
#[test]
fn overflow_puts_what_doesnt_fit_onto_new_boards() {
    let dir = badge("overflow");
    let run = chipload(
        &dir,
        &[
            "nest",
            "b.chipload",
            "--board",
            "200x150",
            "--count",
            "1=4",
            "--overflow",
            "new-boards",
        ],
    );
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("4 of 4 placed"), "{}", run.stdout);
    assert!(run.stdout.contains(", made:"), "{}", run.stdout);
    let out = json(&chipload(
        &dir,
        &[
            "nest",
            "b.chipload",
            "--overflow",
            "new-boards",
            "--dry-run",
            "--json",
        ],
    ));
    assert_eq!(out["unplaced"], serde_json::json!([]));
    let run = chipload(&dir, &["run", "b.chipload", "-o", "b.nc", "--no-sheet"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(dir.join("b-S2.nc").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_goal_is_chosen_and_kept() {
    let extent = |goal: &str| {
        let dir = badge(&format!("goal-{goal}"));
        let value = json(&chipload(
            &dir,
            &[
                "nest",
                "b.chipload",
                "--count",
                "1=2",
                "--goal",
                goal,
                "--json",
            ],
        ));
        assert_eq!(value["goal"], goal);
        // Kept for the next nest.
        let kept = chipload(&dir, &["get", "b.chipload", "setups.0.nest.goal"]);
        assert!(kept.stdout.contains(goal), "{}", kept.stdout);
        let e = &value["extent"];
        (e[0].as_f64().unwrap(), e[1].as_f64().unwrap())
    };
    // Low: side by side, lower and wider than left's, one above the other.
    let (low, left) = (extent("low"), extent("left"));
    assert!(low.1 < left.1 && low.0 > left.0, "{low:?} {left:?}");
}
