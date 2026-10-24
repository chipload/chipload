// SPDX-License-Identifier: GPL-3.0-or-later
//! Messages from the library crates (docs/specs/translations.md §7): numbers in either
//! units, and every id the crates name exists in English and is used.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chipload_i18n::check::texts;
use chipload_i18n::message::{feed_number, length_number, trimmed};
use chipload_i18n::{Digits, Language, Message, SOURCES, Units, Value};

#[test]
fn lengths_keep_their_digits_in_mm_and_take_four_decimals_in_inches() {
    assert_eq!(length_number(0.3, Digits::Fixed(3), Units::Mm), "0.300");
    assert_eq!(length_number(12.5, Digits::Trim(3), Units::Mm), "12.5");
    assert_eq!(length_number(12.5, Digits::Fixed(3), Units::Inch), "0.4921");
    assert_eq!(length_number(6.35, Digits::Trim(3), Units::Inch), "0.25");
    assert_eq!(feed_number(1200.0, Digits::Trim(0), Units::Mm), "1200");
    assert_eq!(feed_number(1200.0, Digits::Trim(0), Units::Inch), "47.24");
    assert_eq!(trimmed(-0.0001, 3), "0");
}

#[test]
fn a_message_renders_its_arguments_in_the_language_and_units() {
    let m = Message::new("menu-language-review").arg("language", Value::mm(12.5));
    assert_eq!(m.english(), "12.5 mm (review wanted)");
    assert_eq!(
        m.render(Language::En, Units::Inch),
        "0.4921 in (review wanted)"
    );
    let feed =
        Message::new("menu-language-review").arg("language", Value::Feed(1200.0, Digits::Trim(0)));
    assert_eq!(
        feed.render(Language::En, Units::Inch),
        "47.24 in/min (review wanted)"
    );
    let point = Message::new("menu-language-review")
        .arg("language", Value::Point(10.0, 20.5, Digits::Trim(3)));
    assert_eq!(point.english(), "(10, 20.5) mm (review wanted)");
    let load = Message::new("menu-language-review")
        .arg("language", Value::ChipLoad(0.05, Digits::Fixed(3)));
    assert_eq!(load.english(), "0.050 mm/tooth (review wanted)");
    assert_eq!(
        load.render(Language::En, Units::Inch),
        "0.0020 in/tooth (review wanted)"
    );
    let inner = Message::new("common-ok");
    let outer = Message::new("menu-language-review").arg("language", inner);
    assert_eq!(outer.english(), "OK (review wanted)");
}

#[test]
fn messages_compare_by_their_numbers_bits() {
    let a = Message::new("common-ok").arg("x", Value::mm(f64::NAN));
    assert_eq!(a, a.clone());
    assert_ne!(a, Message::new("common-ok").arg("x", Value::mm(1.0)));
}

/// The library crates' sources: every crate but the GUI and this one, tests left out.
fn library_code() -> String {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "tests") {
                    walk(&path, out);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&crates).unwrap() {
        let dir = entry.unwrap().path();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        if name != "chipload-gui" && name != "chipload-i18n" {
            walk(&dir.join("src"), &mut files);
        }
    }
    files.sort();
    files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap())
        .collect()
}

/// The English texts of the library crates: every file that isn't the app's.
fn library_texts() -> BTreeSet<String> {
    SOURCES
        .iter()
        .filter(|(file, _)| !file.starts_with("gui-") && *file != "common.ftl")
        .flat_map(|(_, sources)| texts(sources[0]).texts.into_keys())
        .collect()
}

#[test]
fn every_message_the_library_crates_name_exists_and_every_one_is_used() {
    let code = library_code();
    let english = library_texts();
    let named: BTreeSet<String> = code
        .split("Message::new(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .collect();
    let missing: Vec<&String> = named.iter().filter(|id| !english.contains(*id)).collect();
    assert!(missing.is_empty(), "no English text: {missing:?}");
    let unused: Vec<&String> = english
        .iter()
        .filter(|id| !code.contains(&format!("\"{id}\"")))
        .collect();
    assert!(unused.is_empty(), "never used: {unused:?}");
}

#[test]
fn one_move_is_one_move() {
    let below = |n: i64| {
        Message::new("check-below-stock")
            .arg("count", n)
            .arg("deepest", Value::mm(-12.3))
            .arg("bottom", Value::mm(-12.0))
            .arg("allowed", Value::mm(0.5))
            .english()
    };
    assert!(below(1).starts_with("1 move goes below"), "{}", below(1));
    assert!(below(3).starts_with("3 moves go below"), "{}", below(3));
    let outside = Message::new("check-outside-stock").arg("count", 1);
    assert!(outside.english().starts_with("1 cutting move ends outside"));
}
