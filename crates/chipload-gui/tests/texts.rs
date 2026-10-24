// SPDX-License-Identifier: GPL-3.0-or-later
//! The app's texts and its code agree (docs/specs/translations.md §4): every `t!` names a
//! text English has, and every English text of the app is named somewhere in the code.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chipload_i18n::SOURCES;
use chipload_i18n::check::texts;

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The GUI's code, tests and the `t!` macro itself left out.
fn code() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files.sort();
    files
        .into_iter()
        .filter(|p| !p.components().any(|c| c.as_os_str() == "tests"))
        .filter(|p| !p.ends_with("i18n.rs"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect()
}

/// The first string literal after each `t!(` (not `format!(` and the like).
fn named_ids(code: &str) -> Vec<String> {
    code.match_indices("t!(")
        .filter(|(at, _)| {
            !code[..*at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
        .filter_map(|(at, _)| {
            let rest = &code[at + 3..];
            let start = rest.find('"')? + 1;
            let len = rest[start..].find('"')?;
            Some(rest[start..start + len].to_string())
        })
        .collect()
}

/// The English texts of the app: `common.ftl` and the `gui-*.ftl` files.
fn english() -> BTreeSet<String> {
    SOURCES
        .iter()
        .filter(|(file, _)| file.starts_with("gui-") || *file == "common.ftl")
        .flat_map(|(_, sources)| texts(sources[0]).texts.into_keys())
        .collect()
}

#[test]
fn every_text_the_code_names_exists() {
    let english = english();
    let mut missing = Vec::new();
    for (path, code) in code() {
        for id in named_ids(&code) {
            if !english.contains(&id) {
                missing.push(format!("{}: {id}", path.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "no English text:\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_text_of_the_app_is_used() {
    let code: String = code().into_iter().map(|(_, c)| c).collect();
    let unused: Vec<String> = english()
        .into_iter()
        .filter(|id| !code.contains(&format!("\"{id}\"")))
        .collect();
    assert!(unused.is_empty(), "never used:\n{}", unused.join("\n"));
}
