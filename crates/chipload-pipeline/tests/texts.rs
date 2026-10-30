// SPDX-License-Identifier: GPL-3.0-or-later
//! Texts in a project (docs/specs/text.md §3, §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_job::{Job, Operation, PathSegment, Selector, TextAnchor, Tool, ToolType, VcarveOp};
use chipload_pipeline::images::turned_box;
use chipload_pipeline::texts::{
    NewText, add_text, remake_changed, remove_text, text_size, with_anchor,
};
use chipload_pipeline::{empty_project, run_job};

/// The box round every letter of text `id`.
fn extent(job: &Job, id: u64) -> ((f64, f64), (f64, f64)) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for shape in job.drawing.shapes.iter().filter(|s| s.text() == Some(id)) {
        for segment in shape.segments() {
            let piece = match segment {
                PathSegment::Line(l) => Primitive::Line(Line {
                    start: (l.start[0], l.start[1]),
                    end: (l.end[0], l.end[1]),
                }),
                PathSegment::Arc(a) => Primitive::Arc(Arc {
                    center: (a.center[0], a.center[1]),
                    radius: a.radius,
                    start_angle: a.start_angle,
                    sweep: a.sweep,
                }),
            };
            let (low, high) = piece.bounds();
            lo = (lo.0.min(low.0), lo.1.min(low.1));
            hi = (hi.0.max(high.0), hi.1.max(high.1));
        }
    }
    (lo, hi)
}

fn hello(job: &Job) -> (Job, chipload_pipeline::texts::TextReport) {
    let mut new = NewText::new("HELLO", 20.0);
    new.x = 30.0;
    new.y = 40.0;
    add_text(job, new).unwrap()
}

#[test]
fn a_text_is_added_written_and_placed() {
    let (job, report) = hello(&empty_project());
    let text = &job.drawing.texts[0];
    assert_eq!(
        (text.id, text.layer.as_str(), text.font.as_str()),
        (1, "text", "FreeSans")
    );
    // H, E, L, L and the O's two outlines.
    assert_eq!(report.shapes.len(), 6, "{:?}", report.lines);
    assert!(
        report.lines[0].starts_with("Wrote \"HELLO\" in FreeSans, 20 mm high ("),
        "{:?}",
        report.lines
    );
    let ((x0, y0), (_, y1)) = extent(&job, 1);
    assert!(
        (x0 - 30.0).abs() < 1e-6 && (y0 - 40.0).abs() < 1e-6,
        "{x0}, {y0}"
    );
    // Capitals only: the box is a capital tall (an O reaches a little above and below).
    assert!((y1 - y0 - 20.0).abs() < 1.0, "{}", y1 - y0);
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    // The shapes are letters of the text, on its layer, and closed.
    for shape in job.drawing.shapes.iter().filter(|s| s.text() == Some(1)) {
        assert_eq!(shape.layer, "text");
        assert!(shape.closed());
    }
}

#[test]
fn a_turned_text_turns_about_its_centre() {
    let (job, _) = hello(&empty_project());
    let mut turned = job.clone();
    turned.drawing.texts[0].rotation = 90.0;
    let (turned, reports) = remake_changed(&job, &turned).unwrap();
    assert_eq!(reports.len(), 1);
    let (w, h) = text_size(&turned.drawing.texts[0]).unwrap();
    let (lo, hi) = extent(&turned, 1);
    let (want_lo, want_hi) = turned_box(30.0, 40.0, w, h, 90.0);
    for (a, b) in [
        (lo.0, want_lo.0),
        (lo.1, want_lo.1),
        (hi.0, want_hi.0),
        (hi.1, want_hi.1),
    ] {
        assert!(
            (a - b).abs() < 1e-6,
            "{lo:?} {hi:?} vs {want_lo:?} {want_hi:?}"
        );
    }
}

#[test]
fn a_left_out_letter_is_found_again_after_the_text_grows() {
    let (mut job, report) = hello(&empty_project());
    // Leave the E out (the second letter from the left).
    let e = report.shapes[1];
    job.operations.push(vcarve(Selector {
        text: Some(1),
        exclude: Some(vec![e]),
        ..Selector::default()
    }));
    let mut bigger = job.clone();
    bigger.drawing.texts[0].height = 30.0;
    let (bigger, reports) = remake_changed(&job, &bigger).unwrap();
    assert!(
        reports[0].lines.iter().any(|l| l.contains("found again")),
        "{:?}",
        reports[0].lines
    );
    let left_out = bigger.operations[0].select().exclude.clone().unwrap();
    assert_eq!(left_out.len(), 1);
    assert_ne!(left_out[0], e, "the new E");
    assert_eq!(
        bigger
            .drawing
            .shapes
            .iter()
            .find(|s| s.id == left_out[0])
            .and_then(|s| s.text()),
        Some(1)
    );
    assert!(bigger.validate().is_empty(), "{:?}", bigger.validate());
}

#[test]
fn removing_a_text_removes_its_letters() {
    let (job, _) = hello(&empty_project());
    let job = remove_text(&job, 1).unwrap();
    assert!(job.drawing.texts.is_empty());
    assert!(job.drawing.shapes.iter().all(|s| s.text().is_none()));
    assert!(remove_text(&job, 1).is_err());
}

#[test]
fn a_v_carved_text_passes_the_checker() {
    let (mut job, _) = hello(&empty_project());
    job.tools.push(Tool {
        id: 2,
        name: None,
        library_uid: None,
        kind: ToolType::Vbit,
        diameter: 12.7,
        flutes: Some(2),
        center_cutting: Some(true),
        rpm: Some(18000.0),
        angle: Some(60.0_f64.to_radians()),
        tip_diameter: Some(0.2),
    });
    job.operations.push(vcarve(Selector {
        text: Some(1),
        ..Selector::default()
    }));
    let result = run_job(&job).unwrap();
    assert!(result.files[0].gcode.contains("G1"));
}

fn vcarve(select: Selector) -> Operation {
    Operation::Vcarve(VcarveOp {
        material: None,
        name: "Text".into(),
        enabled: true,
        part: 1,
        select,
        tool: 2,
        start_depth: 0.0,
        end_depth: 3.0,
        step_down: None,
        feed: 1200.0,
        plunge_feed: 400.0,
        floor: None,
    })
}

/// The middle of the box round text `id`'s letters.
fn middle(job: &Job, id: u64) -> (f64, f64) {
    let ((x0, y0), (x1, y1)) = extent(job, id);
    ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
}

#[test]
fn a_centred_text_stays_centred_when_it_turns_and_changes() {
    let mut new = NewText::new("HELLO", 20.0);
    (new.x, new.y) = (80.0, 50.0);
    new.anchor = TextAnchor::Center;
    let (job, _) = add_text(&empty_project(), new).unwrap();
    let close = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6;
    assert!(
        close(middle(&job, 1), (80.0, 50.0)),
        "{:?}",
        middle(&job, 1)
    );
    // Turned: every point of the letters is turned about the centre, which stays.
    let mut turned = job.clone();
    turned.drawing.texts[0].rotation = 33.0;
    let (turned, _) = remake_changed(&job, &turned).unwrap();
    let starts = |job: &Job| -> Vec<(f64, f64)> {
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.text() == Some(1))
            .flat_map(|s| s.segments())
            .map(|segment| match segment {
                PathSegment::Line(l) => (l.start[0], l.start[1]),
                PathSegment::Arc(a) => {
                    let t = a.start_angle;
                    (
                        a.center[0] + a.radius * t.cos(),
                        a.center[1] + a.radius * t.sin(),
                    )
                }
            })
            .collect()
    };
    let (sin, cos) = 33.0_f64.to_radians().sin_cos();
    for (p, q) in starts(&job).iter().zip(starts(&turned)) {
        let (dx, dy) = (p.0 - 80.0, p.1 - 50.0);
        let want = (80.0 + dx * cos - dy * sin, 50.0 + dx * sin + dy * cos);
        assert!(close(q, want), "{q:?} vs {want:?}");
    }
    // Other words: still centred there.
    let mut longer = job.clone();
    longer.drawing.texts[0].text = "HELLO WORLD".into();
    let (longer, _) = remake_changed(&job, &longer).unwrap();
    assert!(
        close(middle(&longer, 1), (80.0, 50.0)),
        "{:?}",
        middle(&longer, 1)
    );
}

#[test]
fn changing_the_anchor_leaves_the_letters_where_they_are() {
    let (job, _) = hello(&empty_project());
    let text = &job.drawing.texts[0];
    let size = text_size(text).unwrap();
    let centred = with_anchor(text, TextAnchor::Center, size);
    assert_eq!(centred.anchor, TextAnchor::Center);
    let mut after = job.clone();
    after.drawing.texts[0] = centred;
    let (after, _) = remake_changed(&job, &after).unwrap();
    assert_eq!(extent(&after, 1), extent(&job, 1));
    let back = with_anchor(&after.drawing.texts[0], TextAnchor::Corner, size);
    assert!((back.x - 30.0).abs() < 1e-9 && (back.y - 40.0).abs() < 1e-9);
}
