// SPDX-License-Identifier: GPL-3.0-or-later
//! The user guide (docs/specs/user-guide.md §4, §7): the generated reference pages are what
//! the code generates, and every link between pages points at a page that exists.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn book() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/book/src")
}

/// (argument of `chipload reference`, page under src/reference/)
const PAGES: [(&str, &str); 6] = [
    ("cli", "cli.md"),
    ("project", "project-file.md"),
    ("tools", "tools-file.md"),
    ("machines", "machines-file.md"),
    ("materials", "materials-file.md"),
    ("posts", "posts.md"),
];

fn generated(page: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .args(["reference", page])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_reference_pages_are_the_generated_ones() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut stale = Vec::new();
    for (page, file) in PAGES {
        let path = book().join("reference").join(file);
        let text = generated(page);
        if update {
            std::fs::write(&path, &text).unwrap();
        } else if std::fs::read_to_string(&path).unwrap_or_default() != text {
            stale.push(file);
        }
    }
    assert!(
        stale.is_empty(),
        "out of date: {stale:?}; run CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p chipload-cli --test \
         guide and review the diff"
    );
}

fn pages(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            pages(&path, found);
        } else if path.extension().is_some_and(|e| e == "md") {
            found.push(path);
        }
    }
}

/// Every Markdown link target in `text` (the part in `](…)`).
fn links(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("](") {
        let after = &rest[i + 2..];
        let Some(end) = after.find(')') else { break };
        found.push(&after[..end]);
        rest = &after[end..];
    }
    found
}

#[test]
fn links_between_pages_point_at_pages_that_exist() {
    let mut all = Vec::new();
    pages(&book(), &mut all);
    all.sort();
    let mut broken = Vec::new();
    for page in &all {
        let text = std::fs::read_to_string(page).unwrap();
        for target in links(&text) {
            let file = target.split('#').next().unwrap_or("");
            if file.is_empty() || file.contains("://") || file.starts_with("mailto:") {
                continue;
            }
            if !page.parent().unwrap().join(file).exists() {
                broken.push(format!(
                    "{}: {target}",
                    page.strip_prefix(book()).unwrap().display()
                ));
            }
        }
    }
    assert!(broken.is_empty(), "broken links: {broken:#?}");
}

/// The text of every written page (not the generated reference).
fn written() -> String {
    let mut all = Vec::new();
    pages(&book(), &mut all);
    all.sort();
    all.iter()
        .filter(|p| !p.starts_with(book().join("reference")))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_command_operation_and_field_is_on_a_written_page() {
    // docs/specs/user-guide.md §4: a word search, so nothing new goes unmentioned.
    let text = written();
    let mut missing = Vec::new();
    // Every command, as `chipload …`, from the generated command-line page's headings.
    for line in generated("cli").lines() {
        let Some(command) = line
            .trim_start_matches('#')
            .trim()
            .strip_prefix("`chipload ")
            .and_then(|rest| rest.strip_suffix('`'))
        else {
            continue;
        };
        if !text.contains(&format!("chipload {command}")) {
            missing.push(format!("command: chipload {command}"));
        }
    }
    // Every operation type and its fields, from the project file's schema.
    let schema = chipload_job::schema::schema(chipload_job::schema::Format::Project);
    let variants = schema["$defs"]["Operation"]["oneOf"].as_array().unwrap();
    for variant in variants {
        let kind = variant["properties"]["type"]["const"].as_str().unwrap();
        if !text.contains(&format!("\"type\": \"{kind}\"")) {
            missing.push(format!("operation type: {kind}"));
        }
        for field in variant["properties"].as_object().unwrap().keys() {
            if field != "type" && !text.contains(&format!("`{field}`")) {
                missing.push(format!("{kind} field: {field}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "not on any written page of docs/book/src: {missing:#?}"
    );
}
