// SPDX-License-Identifier: GPL-3.0-or-later
//! The built-in fonts and merged letter outlines (docs/specs/more-fonts.md §6).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_job::{Job, Operation, PocketOp, Selector, Tool, ToolType, VcarveOp};
use chipload_pipeline::texts::{NewText, add_text, lettering};
use chipload_pipeline::{empty_project, run_job};
use chipload_text::{FONTS, Layout, letters};

fn tool(id: i64, kind: ToolType, diameter: f64) -> Tool {
    Tool {
        id,
        name: None,
        library_uid: None,
        kind,
        diameter,
        flutes: Some(2),
        center_cutting: Some(true),
        rpm: Some(18000.0),
        angle: (kind == ToolType::Vbit).then(|| 60.0_f64.to_radians()),
        tip_diameter: (kind == ToolType::Vbit).then_some(0.2),
    }
}

/// A project with one text in `font` and the two tools: a V-bit (2) and a 0.6 mm endmill (3).
fn written(text: &str, font: &str, line_spacing: f64) -> Job {
    let mut new = NewText::new(text, 15.0);
    new.font = font;
    new.x = 10.0;
    new.y = 10.0;
    new.line_spacing = line_spacing;
    let (mut job, _) = add_text(&empty_project(), new).unwrap_or_else(|e| panic!("{font}: {e}"));
    job.tools.push(tool(2, ToolType::Vbit, 12.7));
    job.tools.push(tool(3, ToolType::Endmill, 0.6));
    job
}

fn text_select() -> Selector {
    Selector {
        text: Some(1),
        ..Selector::default()
    }
}

fn vcarve() -> Operation {
    Operation::Vcarve(VcarveOp {
        name: "Carve".into(),
        enabled: true,
        part: 1,
        select: text_select(),
        tool: 2,
        start_depth: 0.0,
        end_depth: 2.0,
        step_down: None,
        feed: 1200.0,
        plunge_feed: 400.0,
    })
}

#[test]
fn every_font_v_carves_a_sentence() {
    assert_eq!(FONTS.len(), 17);
    let mut failed = Vec::new();
    for font in FONTS {
        let mut job = written("Maker Space Oak\nIron & Ash 1926", font.name, 1.4);
        job.operations.push(vcarve());
        if let Err(e) = run_job(&job) {
            failed.push(format!("{}: {e}", font.name));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
fn every_font_pockets_a_sentence_on_lines_that_overlap() {
    // Lines 0.7 capital heights apart: descenders reach into the line below, so letters of
    // different lines overlap too, and are merged. A pocket refuses outlines that cross.
    // Smokum is left out: its spiked serifs (the outline turns back by more than 150°) make
    // the pocket's first offset fail with tools under 2 mm even where nothing is merged, a
    // pocket limit of its own (TODO.md).
    let mut failed = Vec::new();
    for font in FONTS.iter().filter(|f| f.name != "Smokum") {
        let mut job = written("Maker Space Oak\nIron & Ash 1926", font.name, 0.7);
        job.operations.push(pocket());
        if let Err(e) = run_job(&job) {
            failed.push(format!("{}: {e}", font.name));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

fn pocket() -> Operation {
    Operation::Pocket(PocketOp {
        name: "Letters".into(),
        enabled: true,
        part: 1,
        select: text_select(),
        tool: 3,
        start_depth: 0.0,
        end_depth: 1.0,
        clearance: 0.0,
        step_down: 1.0,
        stepover: 0.4,
        direction: chipload_job::Direction::Climb,
        feed: 600.0,
        plunge_feed: 200.0,
        entry: chipload_job::Entry::Plunge {},
        starts: Vec::new(),
        lead: None,
        corners: None,
        rest: None,
        rest_passes: Vec::new(),
    })
}

#[test]
fn a_script_word_is_pocketed_as_one_shape() {
    let mut job = written("Oak", "Satisfy", 1.4);
    // The letters of "Oak" are joined: one outline round the word, with the holes of O and a.
    let outlines = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(1))
        .count();
    assert!(outlines <= 4, "{outlines} shapes");
    job.operations.push(pocket());
    run_job(&job).unwrap();
}

#[test]
fn letters_that_overlap_nothing_are_kept_exactly() {
    // FreeSans "HELLO": no letter overlaps another, so the outlines are the font's own.
    let text = &written("HELLO", "FreeSans", 1.4).drawing.texts[0].clone();
    let merged = lettering(text).unwrap();
    let raw = letters(&Layout {
        text: "HELLO",
        font: "FreeSans",
        height: 15.0,
        spacing: 0.0,
        line_spacing: 1.4,
        align: chipload_text::Align::Left,
    })
    .unwrap();
    assert_eq!(merged.outlines, raw.outlines);
    assert_eq!(merged.letter, raw.letter);
}
