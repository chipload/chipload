// SPDX-License-Identifier: GPL-3.0-or-later
//! JSON Schemas of the persisted formats (docs/specs/scripting.md §2): the committed copies
//! are the generated ones, and every file Chipload writes is valid against them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use chipload_job::schema::{Format, schema};
use chipload_job::{dump_job, load_job};
use serde_json::{Value, json};

fn root(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn committed(format: Format) -> PathBuf {
    root(&format!(
        "docs/schemas/chipload-{}.schema.json",
        format.name()
    ))
}

const FORMATS: [Format; 3] = [Format::Project, Format::Tools, Format::Machines];

#[test]
fn the_committed_schemas_are_the_generated_ones() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    for format in FORMATS {
        let text = format!(
            "{}\n",
            serde_json::to_string_pretty(&schema(format)).unwrap()
        );
        let path = committed(format);
        if update {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &text).unwrap();
            continue;
        }
        let saved = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            saved == text,
            "{} is out of date: run with CHIPLOAD_UPDATE_GOLDEN=1 and review the diff",
            path.display()
        );
    }
}

/// Validate `value` against the schema of `format`; the error text if it isn't valid.
fn validate(format: Format, value: &Value) -> Result<(), String> {
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let url = format!("https://chipload.invalid/{}.json", format.name());
    compiler.add_resource(&url, schema(format)).unwrap();
    let index = compiler.compile(&url, &mut schemas).unwrap();
    schemas.validate(value, index).map_err(|e| format!("{e:#}"))
}

fn corpus_projects() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for version in std::fs::read_dir(root("tests/corpus/jobs")).unwrap() {
        for file in std::fs::read_dir(version.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|e| e == "chipload") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn every_corpus_project_as_saved_today_is_valid() {
    // Each frozen sample migrated to the current version and written as `save_job` writes.
    let projects = corpus_projects();
    assert!(projects.len() > 20, "{} projects", projects.len());
    for path in projects {
        let job = load_job(&path).unwrap();
        let value: Value = serde_json::from_str(&dump_job(&job).unwrap()).unwrap();
        if let Err(err) = validate(Format::Project, &value) {
            panic!("{}: {err}", path.display());
        }
    }
}

#[test]
fn broken_projects_are_refused() {
    let path = root("tests/corpus/jobs/v18/rest.chipload");
    let good: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    validate(Format::Project, &good).unwrap();
    let broken = |change: &dyn Fn(&mut Value)| {
        let mut value = good.clone();
        change(&mut value);
        validate(Format::Project, &value).is_err()
    };
    // A misspelt field, an unknown operation type, a missing required field, a wrong type,
    // an older version, a value outside an enum.
    assert!(broken(&|v| v["operations"][0]["end_dpth"] = json!(6)));
    assert!(broken(&|v| v["operations"][0]["type"] = json!("surfacing")));
    assert!(broken(&|v| {
        v["operations"][0].as_object_mut().unwrap().remove("tool");
    }));
    assert!(broken(&|v| v["stock"]["thickness"] = json!("12")));
    assert!(broken(&|v| v["schema_version"] = json!(17)));
    assert!(broken(
        &|v| v["operations"][0]["direction"] = json!("sideways")
    ));
    // Optional fields may be left out.
    assert!(!broken(&|v| {
        v["operations"][1].as_object_mut().unwrap().remove("rest");
    }));
}

#[test]
fn library_files_chipload_writes_are_valid() {
    let dir = std::env::temp_dir().join(format!("chipload-schema-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let chipload = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
            .env("CHIPLOAD_CONFIG_DIR", &dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    chipload(&[
        "tool",
        "add",
        "6 mm",
        "--diameter",
        "6",
        "--flutes",
        "2",
        "--rpm",
        "18000",
        "--feed",
        "1500",
        "--plunge-feed",
        "400",
        "--step-down",
        "2",
        "--stepover",
        "0.4",
    ]);
    chipload(&[
        "tool",
        "add",
        "V 60",
        "--type",
        "vbit",
        "--diameter",
        "12.7",
        "--angle",
        "60",
        "--tip",
        "0.2",
    ]);
    chipload(&["machine", "add", "Router", "--safe-z", "10"]);
    let export = dir.join("export.json");
    chipload(&["tool", "export", export.to_str().unwrap()]);
    let read = |path: PathBuf| -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    };
    for (format, path) in [
        (Format::Tools, dir.join("tools.json")),
        (Format::Tools, export),
        (Format::Machines, dir.join("machines.json")),
    ] {
        if let Err(err) = validate(format, &read(path.clone())) {
            panic!("{}: {err}", path.display());
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
