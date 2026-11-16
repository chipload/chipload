// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::empty_project;
use crate::texts::{NewText, add_text};

fn project_with_text() -> (Job, Vec<u64>) {
    let mut new = NewText::new("SIGN", 30.0);
    (new.x, new.y) = (150.0, 100.0);
    let job = add_text(&empty_project(), new, &crate::fonts::BuiltinOnly)
        .unwrap()
        .0;
    let ids = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(1))
        .map(|s| s.id)
        .collect();
    (job, ids)
}

#[test]
fn a_frame_round_a_text_is_centred_with_the_margin_inside_its_border() {
    let (job, letters) = project_with_text();
    let ((x0, y0), (x1, y1)) = shapes_bounds(&job, &letters).unwrap();
    let mut frame = NewFrame::new(
        FrameKind::Rounded,
        FrameSize::Around {
            ids: letters.clone(),
            margin: 10.0,
        },
    );
    frame.borders = 1;
    frame.r = Some(4.0);
    let edited = add_frame(&job, &frame).unwrap();
    assert_eq!(edited.new_ids.len(), 2);
    let shape = |id| {
        edited
            .job
            .drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .clone()
    };
    let outline = shape(edited.new_ids[0]);
    let border = shape(edited.new_ids[1]);
    assert_eq!(outline.layer, "frame");
    assert_eq!(border.layer, "border");
    let Geometry::Rectangle {
        center,
        length,
        width,
        ..
    } = border.geometry
    else {
        panic!("{:?}", border.geometry);
    };
    assert!((center[0] - (x0 + x1) / 2.0).abs() < 1e-9);
    assert!((center[1] - (y0 + y1) / 2.0).abs() < 1e-9);
    // The border is the text's box plus the margin, its corners not cutting in.
    assert!((length - (x1 - x0 + 20.0)).abs() < 1e-9, "{length}");
    assert!((width - (y1 - y0 + 20.0)).abs() < 1e-9, "{width}");
    let Geometry::Rectangle { length: outer, .. } = outline.geometry else {
        panic!();
    };
    assert!((outer - length - 10.0).abs() < 1e-9);
}

#[test]
fn a_frame_to_a_size_sits_on_the_stock_and_borders_must_fit() {
    let job = empty_project();
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let size = FrameSize::Fixed {
        width: 200.0,
        height: 100.0,
        center: None,
    };
    let mut frame = NewFrame::new(FrameKind::Oval, size.clone());
    frame.borders = 2;
    let edited = add_frame(&job, &frame).unwrap();
    let ((a0, b0), (a1, b1)) = shapes_bounds(&edited.job, &edited.new_ids[..1]).unwrap();
    assert!(((a0 + a1) - (x0 + x1)).abs() < 0.02 && ((b0 + b1) - (y0 + y1)).abs() < 0.02);
    assert!((a1 - a0 - 200.0).abs() < 0.02);

    frame.gap = 30.0;
    let err = add_frame(&job, &frame).unwrap_err();
    assert!(err.to_string().contains("2 borders 30 mm apart"), "{err}");
    frame.borders = 4;
    assert!(add_frame(&job, &frame).is_err());

    let mut round = NewFrame::new(
        FrameKind::Oval,
        FrameSize::Fixed {
            width: 80.0,
            height: 80.0,
            center: Some((10.0, 10.0)),
        },
    );
    round.borders = 1;
    let edited = add_frame(&job, &round).unwrap();
    assert!(
        edited
            .job
            .drawing
            .shapes
            .iter()
            .all(|s| matches!(s.geometry, Geometry::Circle { .. }))
    );

    let mut big = NewFrame::new(FrameKind::Notched, size);
    big.r = Some(80.0);
    let err = add_frame(&job, &big).unwrap_err();
    assert!(err.to_string().contains("at most 50 mm"), "{err}");
}

#[test]
fn nothing_to_frame_is_refused() {
    let job = empty_project();
    let frame = NewFrame::new(
        FrameKind::Rectangle,
        FrameSize::Around {
            ids: vec![99],
            margin: 5.0,
        },
    );
    assert!(add_frame(&job, &frame).is_err());
}

// -- frames that follow (docs/specs/sign-maker.md §4)

/// A rounded frame with one border round text 1, and its outline's id.
fn framed_text() -> (Job, u64) {
    let (job, letters) = project_with_text();
    let mut frame = NewFrame::new(
        FrameKind::Rounded,
        FrameSize::Around {
            ids: letters,
            margin: 10.0,
        },
    );
    frame.borders = 1;
    frame.r = Some(6.0);
    let edited = add_frame(&job, &frame).unwrap();
    (edited.job, edited.new_ids[0])
}

fn geometry(job: &Job, id: u64) -> Geometry {
    job.drawing
        .shapes
        .iter()
        .find(|s| s.id == id)
        .unwrap()
        .geometry
        .clone()
}

fn rectangle_size(job: &Job, id: u64) -> (f64, f64, [f64; 2]) {
    match geometry(job, id) {
        Geometry::Rectangle {
            length,
            width,
            center,
            ..
        } => (length, width, center),
        other => panic!("{other:?}"),
    }
}

/// Text 1's words changed, its letters made again.
fn retext(job: &Job, words: &str) -> Job {
    let mut changed = job.clone();
    changed.drawing.texts[0].text = words.to_string();
    crate::texts::remake(job, &changed, 1, &crate::fonts::BuiltinOnly)
        .unwrap()
        .0
}

#[test]
fn a_frame_round_a_text_is_recorded_by_the_text() {
    let (job, outline) = framed_text();
    let frame = &job.drawing.frames[0];
    assert_eq!(frame.outline, outline);
    assert_eq!(frame.borders.len(), 1);
    assert_eq!(frame.around.as_ref().unwrap().texts, vec![1]);
    assert!(frame.around.as_ref().unwrap().shapes.is_empty());
    assert_eq!(frame.kind, FrameShape::Rounded);
    assert!(job.drawing.validate().is_empty());
    // Nothing changed: following changes nothing.
    let (same, notes) = follow_frames(job.clone());
    assert_eq!(same, job);
    assert!(notes.is_empty());
}

#[test]
fn new_words_draw_the_frame_again_round_them_keeping_its_shapes() {
    let (job, outline) = framed_text();
    let border = job.drawing.frames[0].borders[0];
    let (w0, h0, _) = rectangle_size(&job, outline);
    let (followed, notes) = follow_frames(retext(&job, "A LONGER SIGN"));
    assert!(notes.is_empty(), "{notes:?}");
    let (w1, h1, _) = rectangle_size(&followed, outline);
    assert!(w1 > w0 + 50.0, "{w0} → {w1}");
    // Other letters, a little taller or shorter.
    assert!((h1 - h0).abs() < 2.0, "{h0} → {h1}");
    // As a new frame round the new words would be.
    let letters: Vec<u64> = followed
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(1))
        .map(|s| s.id)
        .collect();
    let mut again = NewFrame::new(
        FrameKind::Rounded,
        FrameSize::Around {
            ids: letters,
            margin: 10.0,
        },
    );
    again.borders = 1;
    again.r = Some(6.0);
    let place = place_frame(&followed, &again).unwrap();
    assert!((w1 - place.width).abs() < 1e-9 && (h1 - place.height).abs() < 1e-9);
    let (bw, _, _) = rectangle_size(&followed, border);
    assert!((w1 - bw - 10.0).abs() < 1e-6);
    assert_eq!(followed.drawing.frames[0].width, w1);
    assert!(followed.drawing.validate().is_empty());
}

#[test]
fn a_frame_moved_alone_is_let_go_of_but_moved_with_its_text_it_follows() {
    let (job, outline) = framed_text();
    let border = job.drawing.frames[0].borders[0];
    let moved = crate::shapes::transform_shapes(&job, &[outline], (0.0, 0.0), (5.0, 0.0), 1.0, 0.0)
        .unwrap();
    let (alone, notes) = follow_frames(moved.clone());
    assert!(alone.drawing.frames.is_empty());
    assert_eq!(notes.len(), 1);
    assert!(notes[0].english().contains("changed by hand"), "{notes:?}");
    // Its shapes stay as the hand left them.
    assert_eq!(geometry(&alone, outline), geometry(&moved, outline));

    let mut together: Vec<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(1))
        .map(|s| s.id)
        .collect();
    together.extend([outline, border]);
    let mut both = job.clone();
    let text = &mut both.drawing.texts[0];
    text.x += 7.0;
    text.y -= 3.0;
    let both = crate::texts::remake(&job, &both, 1, &crate::fonts::BuiltinOnly)
        .unwrap()
        .0;
    let both = crate::shapes::transform_shapes(
        &both,
        &[outline, border],
        (0.0, 0.0),
        (7.0, -3.0),
        1.0,
        0.0,
    )
    .unwrap();
    let (followed, notes) = follow_frames(both);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(followed.drawing.frames.len(), 1);
    let (_, _, c0) = rectangle_size(&job, outline);
    let (_, _, c1) = rectangle_size(&followed, outline);
    assert!((c1[0] - c0[0] - 7.0).abs() < 1e-6 && (c1[1] - c0[1] + 3.0).abs() < 1e-6);
}

#[test]
fn a_frame_whose_text_or_shape_is_gone_is_let_go_of() {
    let (job, outline) = framed_text();
    let gone = crate::texts::remove_text(&job, 1).unwrap();
    let (out, notes) = follow_frames(gone);
    assert!(out.drawing.frames.is_empty());
    assert!(notes[0].english().contains("is gone"), "{notes:?}");
    assert_eq!(geometry(&out, outline), geometry(&job, outline));

    let border = job.drawing.frames[0].borders[0];
    let (lost, _) = crate::shapes::delete_shape(&job, border).unwrap();
    let (out, notes) = follow_frames(lost);
    assert!(out.drawing.frames.is_empty());
    assert!(notes[0].english().contains("lost its shape"), "{notes:?}");
}

#[test]
fn a_frame_set_again_redraws_with_its_new_values() {
    let (job, outline) = framed_text();
    let first_border = job.drawing.frames[0].borders[0];
    let change = FrameChange {
        borders: Some(2),
        margin: Some(20.0),
        ..FrameChange::default()
    };
    let edited = set_frame(&job, first_border, &change).unwrap();
    let frame = &edited.job.drawing.frames[0];
    assert_eq!(frame.borders.len(), 2);
    assert_eq!(frame.borders[0], first_border);
    assert_eq!(edited.new_ids, vec![frame.borders[1]]);
    assert_eq!(frame.margin, 20.0);
    // The text's box, twice the margin and the two gaps; the 6 mm corners no longer
    // cut in, 10 mm in.
    let letters: Vec<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(1))
        .map(|s| s.id)
        .collect();
    let ((x0, _), (x1, _)) = shapes_bounds(&job, &letters).unwrap();
    let (w1, _, _) = rectangle_size(&edited.job, outline);
    assert!((w1 - (x1 - x0 + 40.0 + 20.0)).abs() < 1e-6, "{w1}");
    let layer = |j: &Job, id: u64| {
        j.drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .layer
            .clone()
    };
    assert_eq!(layer(&edited.job, frame.borders[1]), "border");

    let none = FrameChange {
        borders: Some(0),
        kind: Some(FrameKind::Notched),
        ..FrameChange::default()
    };
    let plain = set_frame(&edited.job, outline, &none).unwrap().job;
    assert!(plain.drawing.frames[0].borders.is_empty());
    assert_eq!(plain.drawing.shapes.len(), job.drawing.shapes.len() - 1);
    assert!(matches!(
        geometry(&plain, outline),
        Geometry::Polyline { .. }
    ));
    // Following it changes nothing.
    let (same, notes) = follow_frames(plain.clone());
    assert_eq!(same, plain);
    assert!(notes.is_empty());

    let sized = FrameChange {
        width: Some(400.0),
        ..FrameChange::default()
    };
    assert!(set_frame(&job, outline, &sized).is_err());
    let detached = detach_frame(&job, outline).unwrap();
    assert!(detached.drawing.frames.is_empty());
    assert!(detach_frame(&detached, outline).is_err());
}

#[test]
fn a_frame_to_a_size_follows_nothing_but_can_be_set() {
    let job = empty_project();
    let frame = NewFrame::new(
        FrameKind::Arched,
        FrameSize::Fixed {
            width: 200.0,
            height: 100.0,
            center: Some((50.0, 50.0)),
        },
    );
    let edited = add_frame(&job, &frame).unwrap();
    let record = &edited.job.drawing.frames[0];
    assert!(record.around.is_none());
    let change = FrameChange {
        width: Some(240.0),
        ..FrameChange::default()
    };
    let wider = set_frame(&edited.job, record.outline, &change).unwrap().job;
    assert_eq!(wider.drawing.frames[0].width, 240.0);
    assert_eq!(wider.drawing.frames[0].center, [50.0, 50.0]);
    let (same, notes) = follow_frames(wider.clone());
    assert_eq!(same, wider);
    assert!(notes.is_empty());
}

// -- borders (docs/specs/sign-maker.md §5)

#[test]
fn double_borders_and_groove_width_set_the_lines_and_the_margin() {
    let (job, letters) = project_with_text();
    let ((x0, _), (x1, _)) = shapes_bounds(&job, &letters).unwrap();
    let mut frame = NewFrame::new(
        FrameKind::Rectangle,
        FrameSize::Around {
            ids: letters,
            margin: 10.0,
        },
    );
    frame.borders = 2;
    frame.gap = 6.0;
    frame.double = true;
    frame.border_width = 2.0;
    let edited = add_frame(&job, &frame).unwrap();
    // Two borders of two lines each: 6, 10 (6 + 2·2), then 16, 20 in from the outline.
    assert_eq!(edited.new_ids.len(), 5);
    let widths: Vec<f64> = edited
        .new_ids
        .iter()
        .map(|&id| match geometry(&edited.job, id) {
            Geometry::Rectangle { length, .. } => length,
            other => panic!("{other:?}"),
        })
        .collect();
    let outer = widths[0];
    let ins: Vec<f64> = widths[1..].iter().map(|w| (outer - w) / 2.0).collect();
    for (got, want) in ins.iter().zip([6.0, 10.0, 16.0, 20.0]) {
        assert!((got - want).abs() < 1e-9, "{ins:?}");
    }
    // The margin to the innermost groove's inner edge: 10 + 20 + 1 from the text.
    assert!((outer - (x1 - x0 + 2.0 * 31.0)).abs() < 1e-9, "{outer}");
    let record = &edited.job.drawing.frames[0];
    assert!(record.double);
    assert_eq!(record.border_width, 2.0);
    // Followed: nothing changes, and the record counts two borders.
    let (same, notes) = follow_frames(edited.job.clone());
    assert_eq!(same, edited.job);
    assert!(notes.is_empty());

    frame.border_width = 0.0;
    let err = add_frame(&job, &frame).unwrap_err();
    assert!(err.to_string().contains("double border"), "{err}");
}

#[test]
fn borders_with_corners_of_their_own_inside_a_rectangular_frame() {
    let job = empty_project();
    let mut frame = NewFrame::new(
        FrameKind::Rounded,
        FrameSize::Fixed {
            width: 200.0,
            height: 100.0,
            center: Some((0.0, 0.0)),
        },
    );
    frame.r = Some(10.0);
    frame.borders = 1;
    frame.border_corners = Some(CornerStyle::Notch);
    let edited = add_frame(&job, &frame).unwrap();
    let Geometry::Polyline { points, .. } = geometry(&edited.job, edited.new_ids[1]) else {
        panic!();
    };
    // A notched rectangle 190 × 90 with 10 mm notches: its bottom side starts 10 in.
    assert!(
        points
            .iter()
            .any(|p| (p[0] + 85.0).abs() < 1e-9 && (p[1] + 45.0).abs() < 1e-9)
    );
    assert_eq!(
        edited.job.drawing.frames[0].border_corners,
        Some(FrameCorners::Notch)
    );
    // Another frame that isn't rectangular: its borders follow it again.
    let change = FrameChange {
        kind: Some(FrameKind::Oval),
        ..FrameChange::default()
    };
    let oval = set_frame(&edited.job, edited.new_ids[0], &change).unwrap();
    assert_eq!(oval.job.drawing.frames[0].border_corners, None);
    // Asked for on an oval: refused.
    let mut round = frame.clone();
    round.kind = FrameKind::Oval;
    let err = add_frame(&job, &round).unwrap_err();
    assert!(err.to_string().contains("rectangular"), "{err}");
}

#[test]
fn every_new_kind_is_framed_round_a_text_and_followed() {
    for kind in [
        FrameKind::Stepped,
        FrameKind::Gable,
        FrameKind::DoubleArch,
        FrameKind::Shield,
        FrameKind::Banner,
        FrameKind::Octagon,
    ] {
        let (job, letters) = project_with_text();
        let mut frame = NewFrame::new(
            kind,
            FrameSize::Around {
                ids: letters,
                margin: 8.0,
            },
        );
        frame.borders = 1;
        let edited = add_frame(&job, &frame).unwrap();
        assert_eq!(edited.new_ids.len(), 2, "{kind:?}");
        let (followed, notes) = follow_frames(retext(&edited.job, "SIGNS AGAIN"));
        assert!(notes.is_empty(), "{kind:?}: {notes:?}");
        assert_eq!(followed.drawing.frames[0].kind, shape_of(kind));
        assert!(followed.drawing.frames[0].width > edited.job.drawing.frames[0].width);
    }
}

#[test]
fn a_frame_round_some_of_a_texts_letters_keeps_them_by_id() {
    let (job, letters) = project_with_text();
    let some = letters[..2].to_vec();
    let frame = NewFrame::new(
        FrameKind::Rectangle,
        FrameSize::Around {
            ids: some.clone(),
            margin: 5.0,
        },
    );
    let edited = add_frame(&job, &frame).unwrap();
    let around = edited.job.drawing.frames[0].around.clone().unwrap();
    assert!(around.texts.is_empty());
    assert_eq!(around.shapes, some);
    // Followed: it stays round those two.
    let (same, notes) = follow_frames(edited.job.clone());
    assert_eq!(same, edited.job);
    assert!(notes.is_empty());
}
