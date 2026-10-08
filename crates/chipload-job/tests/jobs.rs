// SPDX-License-Identifier: GPL-3.0-or-later
//! Job loading against the shared corpus and the Python implementation's rules.

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Direction, JobError, Operation, Spindle, load_job, parse_job};
use serde_json::{Value, json};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs")
}

fn all_corpus_jobs() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in std::fs::read_dir(corpus()).unwrap() {
        for file in std::fs::read_dir(dir.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|e| e == "json") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn every_frozen_and_current_corpus_job_loads() {
    let files = all_corpus_jobs();
    assert!(
        files.len() >= 5,
        "expected the shared corpus, found {files:?}"
    );
    for path in files {
        let job = load_job(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(job.schema_version, CURRENT_VERSION, "{}", path.display());
    }
}

#[test]
fn v1_sample_migrates_like_python() {
    let job = load_job(&corpus().join("v1/example_job.json")).unwrap();
    assert_eq!(job.machine.spindle, Spindle::Cw);
    let Operation::Profile(op) = &job.operations[0] else {
        panic!("expected a profile")
    };
    assert!((op.plunge_feed - 2000.0 / 3.0).abs() < 1e-9);
}

#[test]
fn missing_newer_and_malformed_versions_are_refused() {
    for data in [
        json!({}),
        json!({"schema_version": CURRENT_VERSION + 1}),
        json!({"schema_version": 0}),
        json!({"schema_version": true}),
        json!({"schema_version": "1"}),
        json!({"schema_version": 1.0}),
    ] {
        let result = migrate(data.clone());
        assert!(
            matches!(result, Err(JobError::UnsupportedVersion { .. })),
            "{data} should be refused"
        );
    }
}

#[test]
fn version_error_message_matches_python() {
    let err = migrate(json!({"schema_version": 99})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Job schema_version 99 is not supported (this app supports up to 3). \
         Update the application or check the file."
    );
    let err = migrate(json!({})).unwrap_err();
    assert!(err.to_string().starts_with("Job schema_version None "));
}

#[test]
fn v1_to_current_sets_new_fields() {
    let v1 = json!({
        "schema_version": 1,
        "machine": {"post": "grbl", "safe_z": 5},
        "operations": [
            {"type": "profile", "feed": 1500},
            {"type": "pocket", "feed": 900},
            {"type": "drill"},
            "junk",
        ],
    });
    let current = migrate(v1).unwrap();
    assert_eq!(current["schema_version"], json!(CURRENT_VERSION));
    assert_eq!(current["machine"]["spindle"], json!("cw"));
    assert_eq!(current["operations"][0]["plunge_feed"], json!(500.0));
    assert_eq!(current["operations"][1]["plunge_feed"], json!(300.0));
    assert_eq!(current["operations"][1]["direction"], json!("climb"));
    assert!(current["operations"][2].get("plunge_feed").is_none());
    assert_eq!(current["operations"][3], json!("junk"));
}

fn lower_job() -> Value {
    let text = std::fs::read_to_string(corpus().join("v3/messy_2_lower.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn pockets_cannot_have_tabs() {
    let mut data = lower_job();
    data["operations"][0]["tabs"] = json!({"count": 4, "width": 8, "height": 3});
    let err = parse_job(data, "job.json").unwrap_err();
    assert!(err.to_string().contains("tabs"), "{err}");
}

#[test]
fn unknown_fields_are_refused() {
    let mut data = lower_job();
    data["stock"]["colour"] = json!("oak");
    assert!(parse_job(data, "job.json").is_err());
}

#[test]
fn value_constraints_match_python() {
    let cases: [(&str, Value); 5] = [
        ("/stock/thickness", json!(0)),
        ("/operations/0/stepover", json!(1.5)),
        ("/operations/0/depth", json!(-1)),
        ("/tools/0/diameter", json!(0)),
        ("/operations/1/select", json!({})),
    ];
    for (pointer, bad) in cases {
        let mut data = lower_job();
        *data.pointer_mut(pointer).unwrap() = bad;
        assert!(
            parse_job(data, "job.json").is_err(),
            "{pointer} should be refused"
        );
    }
}

#[test]
fn pocket_fields_parse() {
    let job = parse_job(lower_job(), "job.json").unwrap();
    let Operation::Pocket(pocket) = &job.operations[0] else {
        panic!("expected a pocket")
    };
    assert_eq!(pocket.direction, Direction::Climb);
    assert!(pocket.keep_tool_down);
    assert_eq!(job.tools[0].center_cutting, Some(true));
}
