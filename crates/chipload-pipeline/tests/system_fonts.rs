// SPDX-License-Identifier: GPL-3.0-or-later
//! Texts in fonts that aren't built in (docs/specs/system-fonts.md §8): a font from the
//! settings folder's `fonts`, and the same project on a computer without it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use chipload_job::{Job, PathSegment};
use chipload_pipeline::empty_project;
use chipload_pipeline::fonts::{BuiltinOnly, ComputerFonts};
use chipload_pipeline::texts::{NewText, add_text, remake, text_size};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-fonts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `data` with every `from` turned into `to`, as one-byte and as UTF-16 (big-endian)
/// text: a font's names renamed, the same length, so nothing else moves.
fn renamed(data: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let wide = |s: &str| s.bytes().flat_map(|b| [0, b]).collect::<Vec<u8>>();
    let mut out = data.to_vec();
    for (a, b) in [
        (from.as_bytes().to_vec(), to.as_bytes().to_vec()),
        (wide(from), wide(to)),
    ] {
        let mut i = 0;
        while i + a.len() <= out.len() {
            if out[i..i + a.len()] == a[..] {
                out[i..i + a.len()].copy_from_slice(&b);
                i += a.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// A folder of fonts holding FreeSans renamed "TestSans": a font that isn't built in,
/// whose letters are FreeSans's.
fn test_fonts(name: &str) -> ComputerFonts {
    let dir = temp_dir(name);
    let free_sans = chipload_text::builtin("FreeSans").unwrap().data();
    std::fs::write(
        dir.join("TestSans.ttf"),
        renamed(free_sans, "FreeSans", "TestSans"),
    )
    .unwrap();
    ComputerFonts::folder_only(dir)
}

/// Every point of the letters of text `id`, in order.
fn letter_points(job: &Job, id: u64) -> Vec<(f64, f64)> {
    job.drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(id))
        .flat_map(|s| s.segments().to_vec())
        .flat_map(|s: PathSegment| s.to_primitive().points())
        .collect()
}

fn assert_near(a: &[(f64, f64)], b: &[(f64, f64)]) {
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(b) {
        assert!(
            (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6,
            "{p:?} {q:?}"
        );
    }
}

fn hello(font: &str, fonts: &dyn chipload_pipeline::fonts::FontSource) -> Job {
    let mut new = NewText::new("HELLO", 20.0);
    new.font = font;
    (new.x, new.y) = (30.0, 40.0);
    add_text(&empty_project(), new, fonts).unwrap().0
}

#[test]
fn the_folders_fonts_are_found_by_name_and_built_in_names_stay_built_in() {
    let fonts = test_fonts("names");
    assert_eq!(fonts.names(), ["TestSans"]);
    // A file of a built-in font's name: the built-in one wins, so it isn't listed.
    let dir = temp_dir("builtin-name");
    std::fs::write(
        dir.join("FreeSans.ttf"),
        chipload_text::builtin("FreeSans").unwrap().data(),
    )
    .unwrap();
    assert!(ComputerFonts::folder_only(dir).names().is_empty());
}

#[test]
fn a_text_in_a_computer_font_is_written_in_it() {
    let fonts = test_fonts("written");
    let job = hello("TestSans", &fonts);
    assert_eq!(job.drawing.texts[0].font, "TestSans");
    // TestSans is FreeSans renamed: the same letters.
    let builtin = hello("FreeSans", &BuiltinOnly);
    assert_near(&letter_points(&job, 1), &letter_points(&builtin, 1));
    // Not on this computer: a new text in it is refused, with the note.
    let mut new = NewText::new("HI", 10.0);
    new.font = "TestSans";
    let err = add_text(&empty_project(), new, &BuiltinOnly).unwrap_err();
    assert!(
        err.to_string()
            .contains("There is no font \"TestSans\" on this computer."),
        "{err}"
    );
}

/// The project opened where the font isn't: it keeps its letters; its words and size are
/// refused with the note; moving and turning move the stored letters; another font is
/// taken (decision 3).
#[test]
fn without_its_font_a_text_keeps_its_letters_and_only_moves() {
    let fonts = test_fonts("missing");
    let job = hello("TestSans", &fonts);
    let id = 1;
    let elsewhere = BuiltinOnly;

    // The size from the stored letters, as with the font.
    let text = &job.drawing.texts[0];
    let with = text_size(&job, text, &fonts).unwrap();
    let without = text_size(&job, text, &elsewhere).unwrap();
    assert!((with.0 - without.0).abs() < 1e-9 && (with.1 - without.1).abs() < 1e-9);

    // New words: refused.
    let mut changed = job.clone();
    changed.drawing.texts[0].text = "HELLO WORLD".into();
    let err = remake(&job, &changed, id, &elsewhere).unwrap_err();
    assert!(
        err.to_string()
            .contains("to change its words or size, choose another font"),
        "{err}"
    );
    changed.drawing.texts[0].text = "HELLO".into();
    changed.drawing.texts[0].height = 25.0;
    assert!(remake(&job, &changed, id, &elsewhere).is_err());

    // Moved, turned and on another layer: the stored letters, moved, are where the font
    // would put them.
    let mut moved = job.clone();
    let t = &mut moved.drawing.texts[0];
    (t.x, t.y, t.rotation) = (55.0, 12.5, 30.0);
    t.layer = "engrave".into();
    let (without_font, report) = remake(&job, &moved, id, &elsewhere).unwrap();
    let (with_font, _) = remake(&job, &moved, id, &fonts).unwrap();
    assert_near(
        &letter_points(&without_font, id),
        &letter_points(&with_font, id),
    );
    assert!(
        without_font
            .drawing
            .shapes
            .iter()
            .filter(|s| s.text() == Some(id))
            .all(|s| s.layer == "engrave")
    );
    assert!(
        report.lines[0].starts_with("Moved \"HELLO\""),
        "{:?}",
        report.lines
    );
    // The same shapes: their ids stay.
    let ids = |j: &Job| {
        j.drawing
            .shapes
            .iter()
            .filter(|s| s.text() == Some(id))
            .map(|s| s.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&without_font), ids(&job));

    // Turned again from there: still where the font puts it.
    let mut again = without_font.clone();
    again.drawing.texts[0].rotation = 300.0;
    let (turned_without, _) = remake(&without_font, &again, id, &elsewhere).unwrap();
    let (turned_with, _) = remake(&without_font, &again, id, &fonts).unwrap();
    assert_near(
        &letter_points(&turned_without, id),
        &letter_points(&turned_with, id),
    );

    // Another font: written in it.
    let mut other = job.clone();
    other.drawing.texts[0].font = "FreeSerif".into();
    other.drawing.texts[0].text = "HELLO WORLD".into();
    let (job, _) = remake(&job, &other, id, &elsewhere).unwrap();
    assert_eq!(job.drawing.texts[0].font, "FreeSerif");
}
