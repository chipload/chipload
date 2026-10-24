// SPDX-License-Identifier: GPL-3.0-or-later
//! The English messages, frozen (docs/specs/translations.md decision 14): what the command
//! line says about the corpus must stay byte for byte while the library crates' messages
//! move into `chipload-i18n`. Covers planning every corpus project, the problems of the
//! deliberately broken projects in tests/corpus/messages/, the DXF import report and the
//! checker's findings on deliberately wrong G-code.
//!
//! When a change is intended, regenerate with `CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p
//! chipload-cli --test messages`, review the diff, and explain it in the commit message.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Files under `dir` (relative to the repository) ending in `ext`, sorted, as relative
/// paths, so the output names them the same on every computer.
fn files(dir: &str, ext: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![root().join(dir)];
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else if path.to_string_lossy().ends_with(ext) {
                let rel = path.strip_prefix(root()).unwrap();
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

/// `text` with every number of 7 or more decimals rounded to 6: full-precision numbers
/// (`--json`'s coordinates) differ in their last digits between platforms' maths
/// libraries, and the snapshot is about the messages, not the last bits of a float.
fn steady(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > start && i + 1 < bytes.len() && bytes[i] == b'.' {
            let mut end = i + 1;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end - i > 7 {
                let value: f64 = text[start..end].parse().unwrap();
                out.push_str(&format!("{value:.6}"));
                i = end;
                continue;
            }
        }
        if i == start {
            let c = text[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        } else {
            out.push_str(&text[start..i]);
        }
    }
    out
}

/// The first lines where `got` differs from `want`, to show in a failure.
fn first_differences(want: &str, got: &str) -> String {
    let mut out = String::new();
    let (mut w, mut g) = (want.lines(), got.lines());
    let mut shown = 0;
    for n in 1.. {
        match (w.next(), g.next()) {
            (None, None) => break,
            (a, b) if a == b => {}
            (a, b) => {
                writeln!(out, "  line {n}:\n    want {a:?}\n    got  {b:?}").unwrap();
                shown += 1;
                if shown == 5 {
                    break;
                }
            }
        }
    }
    out
}

/// One command's exit code, standard output and standard error, as frozen.
fn run(args: &[&str], settings: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .args(args)
        .current_dir(root())
        .env("CHIPLOAD_CONFIG_DIR", settings)
        .output()
        .unwrap();
    let mut text = format!("$ chipload {}\n", args.join(" "));
    writeln!(text, "exit {}", out.status.code().unwrap_or(-1)).unwrap();
    text.push_str("--- stdout\n");
    text.push_str(&String::from_utf8_lossy(&out.stdout));
    text.push_str("--- stderr\n");
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text.push('\n');
    steady(&text)
}

/// Every command of the snapshot, grouped by the file it is frozen in.
fn cases() -> Vec<(&'static str, Vec<Vec<String>>)> {
    let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let projects = files("tests/corpus/jobs", ".chipload");
    let broken = files("tests/corpus/messages", ".chipload");
    let dxfs: Vec<String> = files("tests/corpus/dxf", ".dxf")
        .into_iter()
        .filter(|p| !p.contains("/fuzz/"))
        .collect();
    let gcode = files("tests/corpus/messages", ".nc");
    vec![
        (
            "plan",
            projects.iter().map(|p| args(&["plan", p])).collect(),
        ),
        (
            "problems",
            broken
                .iter()
                .flat_map(|p| {
                    [
                        args(&["check-project", p]),
                        args(&["check-project", p, "--json"]),
                        args(&["plan", p]),
                    ]
                })
                .collect(),
        ),
        (
            "import",
            dxfs.iter()
                .flat_map(|d| [args(&["inspect", d]), args(&["inspect", d, "--json"])])
                .collect(),
        ),
        (
            "checker",
            gcode
                .iter()
                .flat_map(|g| {
                    let job = "tests/corpus/messages/holes_big_tool.chipload";
                    [
                        args(&["check", g, "--job", job]),
                        args(&["check", g, "--job", job, "--json"]),
                    ]
                })
                .collect(),
        ),
    ]
}

#[test]
fn messages_are_unchanged() {
    let settings = std::env::temp_dir().join(format!("chipload-messages-{}", std::process::id()));
    std::fs::create_dir_all(&settings).unwrap();
    let expected = root().join("tests/corpus/expected/messages");
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut changed = Vec::new();
    for (name, commands) in cases() {
        let mut text = String::new();
        for command in &commands {
            let args: Vec<&str> = command.iter().map(String::as_str).collect();
            text.push_str(&run(&args, &settings));
        }
        let path = expected.join(format!("{name}.txt"));
        if update {
            std::fs::create_dir_all(&expected).unwrap();
            std::fs::write(&path, &text).unwrap();
        } else {
            let want = std::fs::read_to_string(&path).unwrap_or_default();
            if want != text {
                changed.push(format!("{name}.txt\n{}", first_differences(&want, &text)));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&settings);
    assert!(
        changed.is_empty(),
        "messages changed (if intended, regenerate with CHIPLOAD_UPDATE_GOLDEN=1, review \
         the diff and explain it in the commit message):\n{}",
        changed.join("\n")
    );
}
