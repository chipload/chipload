// SPDX-License-Identifier: GPL-3.0-or-later
//! Single-line text (docs/specs/single-line-text.md §5, §7): open strokes, not merged, on
//! an arc too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_job::{ArcSide, Geometry, Job, TextArc};
use chipload_pipeline::empty_project;
use chipload_pipeline::fonts::BuiltinOnly;
use chipload_pipeline::texts::{NewText, add_text};

const FONT: &str = "Relief SingleLine";

fn written(words: &str, arc: Option<TextArc>) -> Job {
    let mut new = NewText::new(words, 8.0);
    new.font = FONT;
    new.arc = arc;
    add_text(&empty_project(), new, &BuiltinOnly).unwrap().0
}

fn open_paths(job: &Job) -> usize {
    job.drawing
        .shapes
        .iter()
        .filter(|s| matches!(s.geometry, Geometry::Path { closed: false, .. }))
        .count()
}

#[test]
fn single_line_letters_are_open_paths_and_crossing_strokes_stay_apart() {
    // An X: two strokes that cross, each its own open path (no merging).
    let job = written("X", None);
    assert_eq!(job.drawing.shapes.len(), 2);
    assert_eq!(open_paths(&job), 2);
    let job = written("HELLO", None);
    // H three strokes, E two, each L one, the O one.
    assert_eq!(open_paths(&job), 8);
}

#[test]
fn single_line_letters_go_on_an_arc() {
    let arc = TextArc {
        radius: 40.0,
        side: ArcSide::Top,
        angle: ArcSide::Top.default_angle(),
    };
    let straight = written("NAME", None);
    let round = written("NAME", Some(arc));
    assert_eq!(round.drawing.shapes.len(), straight.drawing.shapes.len());
    assert_eq!(open_paths(&round), round.drawing.shapes.len());
}
