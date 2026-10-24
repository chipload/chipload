// SPDX-License-Identifier: GPL-3.0-or-later
//! Every language has every English text, with the same arguments, and every text formats
//! (docs/specs/translations.md §4).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::Path;

use chipload_i18n::check::texts;
use chipload_i18n::{Arg, Language, SOURCES, tr};

#[test]
fn every_file_parses_in_every_language() {
    let mut bad = Vec::new();
    for (file, sources) in SOURCES {
        for lang in Language::ALL {
            for error in texts(sources[lang as usize]).errors {
                bad.push(format!("{}/{file}: {error}", lang.code()));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn every_language_has_every_text_with_the_same_arguments() {
    let mut bad = Vec::new();
    for (file, sources) in SOURCES {
        let english = texts(sources[0]).texts;
        for lang in &Language::ALL[1..] {
            let other = texts(sources[*lang as usize]).texts;
            for (id, args) in &english {
                match other.get(id) {
                    None => bad.push(format!("{}/{file}: {id} missing", lang.code())),
                    Some(a) if a != args => bad.push(format!(
                        "{}/{file}: {id} takes {a:?}, English {args:?}",
                        lang.code()
                    )),
                    Some(_) => {}
                }
            }
            for id in other.keys().filter(|id| !english.contains_key(*id)) {
                bad.push(format!("{}/{file}: {id} is not in English", lang.code()));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} problems:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn no_text_is_in_two_files() {
    let mut seen = BTreeSet::new();
    for (file, sources) in SOURCES {
        for id in texts(sources[0]).texts.keys() {
            assert!(
                seen.insert(id.clone()),
                "{id} is in {file} and another file"
            );
        }
    }
}

#[test]
fn every_text_formats_with_any_count() {
    for (file, sources) in SOURCES {
        for (id, args) in texts(sources[0]).texts {
            for lang in Language::ALL {
                for n in [0, 1, 2, 5, 21, 1000] {
                    let given: Vec<(&str, Arg)> =
                        args.iter().map(|a| (a.as_str(), Arg::Count(n))).collect();
                    let text = tr(lang, &id, &given);
                    assert!(
                        !text.is_empty() && text != id,
                        "{}/{file}: {id} gives {text:?}",
                        lang.code()
                    );
                    assert!(
                        !text.contains(['\u{2068}', '\u{2069}']),
                        "{id}: isolation marks"
                    );
                }
            }
        }
    }
}

#[test]
fn every_folder_holds_exactly_the_listed_files() {
    let listed: BTreeSet<String> = SOURCES.iter().map(|(f, _)| f.to_string()).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    for lang in Language::ALL {
        let found: BTreeSet<String> = std::fs::read_dir(root.join(lang.code()))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".ftl"))
            .collect();
        assert_eq!(found, listed, "locales/{}", lang.code());
    }
}

#[test]
fn languages_from_system_tags() {
    assert_eq!(Language::from_tag("de-AT"), Some(Language::De));
    assert_eq!(Language::from_tag("sv_SE.UTF-8"), Some(Language::Sv));
    assert_eq!(Language::from_tag("fr"), Some(Language::Fr));
    assert_eq!(Language::from_tag("ES-mx"), Some(Language::Es));
    assert_eq!(Language::from_tag("pl-PL"), None);
    assert_eq!(Language::from_tag("C"), None);
    assert_eq!(Language::from_tag(""), None);
}

#[test]
fn a_missing_text_falls_back_to_english_then_to_its_id() {
    assert_eq!(tr(Language::Sv, "no-such-text", &[]), "no-such-text");
    assert_eq!(tr(Language::En, "common-ok", &[]), "OK");
}
