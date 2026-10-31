// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload text …`, `place --text` and `set` on a text (docs/specs/text.md §6).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-text-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `chipload --json ARGS` in `dir`: its JSON, which must say ok.
fn ok(dir: &Path, args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(dir)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["ok"], Value::Bool(true), "{args:?}: {value}");
    value
}

#[test]
fn texts_are_added_changed_placed_listed_and_removed() {
    let dir = temp_dir("commands");
    ok(&dir, &["new", "-o", "p.chipload"]);
    let fonts = ok(&dir, &["text", "fonts"]);
    assert_eq!(fonts["fonts"][0], "FreeSans");

    // "\n" typed on the command line starts a new line.
    let added = ok(
        &dir,
        &[
            "text",
            "add",
            "p.chipload",
            "AB\\nC",
            "--height",
            "10",
            "--at",
            "5,6",
            "--align",
            "center",
        ],
    );
    assert_eq!(added["text"]["id"], 1);
    let shapes = added["text"]["shapes"].as_array().unwrap().len();
    assert_eq!(shapes, 6, "A and its hole, B and its two holes, C");
    let listed = ok(&dir, &["text", "list", "p.chipload"]);
    let text = &listed["texts"][0];
    assert_eq!(text["text"], "AB\nC");
    assert_eq!(
        (text["x"].as_f64(), text["y"].as_f64()),
        (Some(5.0), Some(6.0))
    );
    assert_eq!(text["align"], "center");

    ok(
        &dir,
        &[
            "text",
            "set",
            "p.chipload",
            "1",
            "--font",
            "FreeSerif",
            "--rotate",
            "-90",
        ],
    );
    ok(
        &dir,
        &["place", "p.chipload", "--text", "1", "--height", "20"],
    );
    let text = &ok(&dir, &["text", "list", "p.chipload"])["texts"][0];
    assert_eq!(text["font"], "FreeSerif");
    assert_eq!(text["rotation"].as_f64(), Some(270.0));
    assert_eq!(text["height"].as_f64(), Some(20.0));

    // A new anchor keeps the text where it is: the corner becomes the centre.
    let before = ok(&dir, &["text", "list", "p.chipload"])["texts"][0].clone();
    ok(
        &dir,
        &[
            "text",
            "set",
            "p.chipload",
            "1",
            "--anchor",
            "center",
            "--rotate",
            "0",
        ],
    );
    let centred = &ok(&dir, &["text", "list", "p.chipload"])["texts"][0];
    assert_eq!(centred["anchor"], "center");
    assert!(centred["x"].as_f64() > before["x"].as_f64(), "{centred}");

    // `set` by path writes the letters again.
    let set = ok(&dir, &["set", "p.chipload", "drawing.texts.0.text=\"ABC\""]);
    assert!(
        set["notes"].to_string().contains("Wrote \\\"ABC\\\""),
        "{set}"
    );

    ok(&dir, &["text", "remove", "p.chipload", "1"]);
    let after = ok(&dir, &["text", "list", "p.chipload"]);
    assert_eq!(after["texts"], Value::Array(Vec::new()));
}

#[test]
fn an_unknown_font_is_refused_with_the_fonts_there_are() {
    let dir = temp_dir("font");
    ok(&dir, &["new", "-o", "p.chipload"]);
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(&dir)
        .args([
            "text",
            "add",
            "p.chipload",
            "A",
            "--height",
            "10",
            "--font",
            "No Such Font 42",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("There is no font \"No Such Font 42\" on this computer")
            && err.contains("FreeSerif"),
        "{err}"
    );
}

/// A font from the settings folder's `fonts`, then the project where it isn't
/// (docs/specs/system-fonts.md §7, §8): listed, its text marked, words refused with the
/// note, a move accepted.
#[test]
fn a_font_in_the_settings_folder_writes_texts_kept_without_it() {
    let dir = temp_dir("folder-font");
    let fonts = dir.join("settings").join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    // FreeSans renamed "TestSans" in its name table: a font that isn't built in.
    let free_sans = chipload_text::builtin("FreeSans").unwrap().data();
    let mut data = free_sans.to_vec();
    for (from, to) in [
        (b"FreeSans".to_vec(), b"TestSans".to_vec()),
        (
            "FreeSans".bytes().flat_map(|b| [0, b]).collect(),
            "TestSans".bytes().flat_map(|b| [0, b]).collect::<Vec<u8>>(),
        ),
    ] {
        let mut i = 0;
        while i + from.len() <= data.len() {
            if data[i..i + from.len()] == from[..] {
                data[i..i + from.len()].copy_from_slice(&to);
            }
            i += 1;
        }
    }
    std::fs::write(fonts.join("TestSans.ttf"), data).unwrap();
    ok(&dir, &["new", "-o", "p.chipload"]);
    let listed = ok(&dir, &["text", "fonts"]);
    assert!(
        listed["computer_fonts"]
            .as_array()
            .unwrap()
            .contains(&Value::from("TestSans")),
        "{listed}"
    );
    ok(
        &dir,
        &[
            "text",
            "add",
            "p.chipload",
            "HI",
            "--height",
            "10",
            "--font",
            "TestSans",
        ],
    );
    let texts = ok(&dir, &["text", "list", "p.chipload"]);
    assert_eq!(texts["texts"][0]["font_available"], true);

    // Another computer: the font is gone.
    std::fs::remove_dir_all(&fonts).unwrap();
    let texts = ok(&dir, &["text", "list", "p.chipload"]);
    assert_eq!(texts["texts"][0]["font_available"], false);
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(&dir)
        .args(["text", "set", "p.chipload", "1", "--text", "HELLO"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Font \"TestSans\" is not on this computer")
            && err.contains("choose another font"),
        "{err}"
    );
    let moved = ok(&dir, &["text", "set", "p.chipload", "1", "--at", "50,20"]);
    assert_eq!(moved["text"]["shapes"].as_array().unwrap().len(), 2);
    ok(
        &dir,
        &[
            "text",
            "set",
            "p.chipload",
            "1",
            "--font",
            "FreeSans",
            "--text",
            "HELLO",
        ],
    );
}
