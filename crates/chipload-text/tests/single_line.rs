// SPDX-License-Identifier: GPL-3.0-or-later
//! Single-line fonts (docs/specs/single-line-text.md §4, §7).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_text::{Align, Layout, builtin, letters};

const FONT: &str = "Relief SingleLine";

fn layout(text: &str) -> Layout<'_> {
    Layout {
        text,
        font: FONT,
        height: 10.0,
        spacing: 0.0,
        line_spacing: 1.6,
        align: Align::Left,
    }
}

/// Each stroke's pieces follow on end to end.
fn joined(stroke: &[Primitive]) -> bool {
    stroke
        .windows(2)
        .all(|w| distance(w[0].end(), w[1].start()) <= CHAIN_TOL_MM)
}

#[test]
fn the_font_is_built_in_as_single_line_and_licensed() {
    let font = builtin(FONT).unwrap();
    assert!(font.single_line());
    let text = std::str::from_utf8(font.data()).unwrap();
    assert!(text.contains("The Relief SingleLine Project Authors"));
    let licence = include_str!("../fonts/LICENSE-OFL-ReliefSingleLine.txt");
    assert!(licence.contains("SIL Open Font License, Version 1.1"));
}

#[test]
fn an_h_is_three_open_strokes_a_capital_height_tall() {
    let l = letters(&layout("H")).unwrap();
    assert_eq!(l.outlines.len(), 3, "two uprights and the bar");
    assert!(l.closed.iter().all(|c| !c));
    assert!(l.outlines.iter().all(|s| joined(s)));
    assert!((l.height - 10.0).abs() < 0.2, "{}", l.height);
}

#[test]
fn an_o_draws_a_loop_and_curves_become_arcs() {
    let l = letters(&layout("O")).unwrap();
    assert!(!l.outlines.is_empty());
    let arcs = l
        .outlines
        .iter()
        .flatten()
        .filter(|p| matches!(p, Primitive::Arc(_)))
        .count();
    assert!(arcs > 0, "fitted with arcs");
    // An O's stroke ends where it starts, whether the font closes it or not.
    let stroke = &l.outlines[0];
    let (a, b) = (stroke[0].start(), stroke[stroke.len() - 1].end());
    assert!(l.closed[0] || distance(a, b) < 0.5, "{a:?} {b:?}");
}

#[test]
fn the_alphabet_and_the_seven_languages_letters_are_there_but_not_cyrillic() {
    let l = letters(&layout(
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789\nÅÄÖåäö ÉÈÊËéèêë ÑñßŒœÇç €",
    ))
    .unwrap();
    assert!(l.missing.is_empty(), "{:?}", l.missing);
    assert_eq!(l.dropped, 0);
    assert_eq!(l.baselines.len(), 2);
    let cyrillic = letters(&layout("AПРИВЕТ")).unwrap();
    assert_eq!(cyrillic.missing.len(), 6);
}

#[test]
fn kerning_and_alignment_place_letters_as_the_outline_fonts_do() {
    let av = letters(&layout("AV")).unwrap();
    let a_v = letters(&layout("A V")).unwrap();
    assert!(av.width < a_v.width);
    // Centred lines: the shorter line's middle is the box's middle.
    let l = letters(&Layout {
        align: Align::Center,
        ..layout("WIDE LINE\nI")
    })
    .unwrap();
    let (start, end) = l.spans[1];
    assert!(((start + end) / 2.0 - l.width / 2.0).abs() < 0.5);
    // The same text twice: the same strokes.
    assert_eq!(
        letters(&layout("Chipload")).unwrap(),
        letters(&layout("Chipload")).unwrap()
    );
}
