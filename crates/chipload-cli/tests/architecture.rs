// SPDX-License-Identifier: GPL-3.0-or-later
//! Architecture rules from CLAUDE.md, checked on the resolved dependency graph
//! (`cargo metadata`), so indirect dependencies count too. Only normal dependencies matter;
//! tests may use anything.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

/// Package name -> names of everything it depends on at run time, directly or not.
fn runtime_closure() -> BTreeMap<String, BTreeSet<String>> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: Value = serde_json::from_slice(&out.stdout).unwrap();
    let name_of: BTreeMap<&str, &str> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    let mut direct: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in meta["resolve"]["nodes"].as_array().unwrap() {
        let deps = node["deps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| {
                d["dep_kinds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|k| k["kind"].is_null()) // null = normal, not dev or build
            })
            .map(|d| d["pkg"].as_str().unwrap())
            .collect();
        direct.insert(node["id"].as_str().unwrap(), deps);
    }
    let members: Vec<&str> = meta["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    members
        .iter()
        .map(|&member| {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut todo = direct[member].clone();
            while let Some(id) = todo.pop() {
                if seen.insert(id) {
                    todo.extend(direct[id].iter().copied());
                }
            }
            let names = seen.iter().map(|id| name_of[id].to_string()).collect();
            (name_of[member].to_string(), names)
        })
        .collect()
}

const GUI_CRATES: [&str; 4] = ["egui", "eframe", "winit", "rfd"];

#[test]
fn checker_is_independent_of_the_generator() {
    // The checker must not reuse generator code paths, or it can't catch their bugs.
    let deps = &runtime_closure()["chipload-check"];
    for generator in [
        "chipload-toolpath",
        "chipload-post",
        "chipload-dxf",
        "chipload-pipeline",
    ] {
        assert!(
            !deps.contains(generator),
            "chipload-check depends on {generator}"
        );
    }
}

#[test]
fn core_is_headless_and_at_the_bottom() {
    let deps = &runtime_closure()["chipload-core"];
    let bad: Vec<&String> = deps
        .iter()
        .filter(|d| d.starts_with("chipload-") || GUI_CRATES.contains(&d.as_str()))
        .collect();
    assert!(bad.is_empty(), "chipload-core depends on {bad:?}");
}

#[test]
fn only_the_gui_crate_uses_gui_libraries() {
    for (crate_name, deps) in runtime_closure() {
        if crate_name == "chipload-gui" {
            continue;
        }
        let bad: Vec<&String> = deps
            .iter()
            .filter(|d| GUI_CRATES.contains(&d.as_str()))
            .collect();
        assert!(bad.is_empty(), "{crate_name} depends on {bad:?}");
    }
}

#[test]
fn nothing_depends_on_the_front_ends() {
    // ui -> cli/app services -> core, never the reverse.
    for (crate_name, deps) in runtime_closure() {
        for front_end in ["chipload-gui", "chipload-cli"] {
            assert!(
                !deps.contains(front_end),
                "{crate_name} depends on {front_end}"
            );
        }
    }
}
