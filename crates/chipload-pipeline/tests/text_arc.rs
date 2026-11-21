// SPDX-License-Identifier: GPL-3.0-or-later
//! Texts on an arc (docs/specs/text-on-arc.md): letters moved and turned whole onto a
//! circle round the text's position.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::primitives::{Primitive, distance};
use chipload_job::{ArcSide, Geometry, Job, TextAlign, TextArc};
use chipload_pipeline::empty_project;
use chipload_pipeline::fonts::BuiltinOnly;
use chipload_pipeline::texts::{NewText, add_text};

fn on_arc(words: &str, side: ArcSide, align: TextAlign) -> Result<Job, String> {
    let mut new = NewText::new(words, 10.0);
    new.align = align;
    new.arc = Some(TextArc {
        radius: 50.0,
        side,
        angle: side.default_angle(),
    });
    add_text(&empty_project(), new, &BuiltinOnly)
        .map(|(job, _)| job)
        .map_err(|e| e.message().english())
}

/// Each letter's box: (low, high) corners, in the order of its shapes.
fn boxes(job: &Job) -> Vec<((f64, f64), (f64, f64))> {
    job.drawing
        .shapes
        .iter()
        .map(|s| {
            let Geometry::Path { segments, .. } = &s.geometry else {
                panic!("letters are paths");
            };
            let mut lo = (f64::INFINITY, f64::INFINITY);
            let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for (a, b) in segments.iter().map(|g| g.to_primitive().bounds()) {
                lo = (lo.0.min(a.0), lo.1.min(a.1));
                hi = (hi.0.max(b.0), hi.1.max(b.1));
            }
            (lo, hi)
        })
        .collect()
}

#[test]
fn one_letter_on_top_stands_straight_up_its_middle_on_the_circle() {
    let job = on_arc("I", ArcSide::Top, TextAlign::Center).unwrap();
    let [(lo, hi)] = boxes(&job)[..] else {
        panic!("one letter");
    };
    // Upright, on the Y axis (placed by the middle of its advance, which its ink sits
    // near), its middle 50 up.
    assert!((lo.0 + hi.0).abs() < 1.0, "{lo:?} {hi:?}");
    assert!(hi.1 - lo.1 > hi.0 - lo.0, "standing, taller than wide");
    assert!(((lo.1 + hi.1) / 2.0 - 50.0).abs() < 0.5, "{lo:?} {hi:?}");
}

#[test]
fn top_reads_clockwise_and_bottom_reads_left_to_right_upright() {
    let top = boxes(&on_arc("AB", ArcSide::Top, TextAlign::Center).unwrap());
    assert!(top[0].0.0 < top[1].0.0, "A left of B along the top");
    let bottom = boxes(&on_arc("AB", ArcSide::Bottom, TextAlign::Center).unwrap());
    assert!(
        bottom[0].0.0 < bottom[1].0.0,
        "A left of B along the bottom"
    );
    for (lo, hi) in bottom {
        assert!(
            hi.1 < -40.0 && lo.1 > -60.0,
            "near the bottom: {lo:?} {hi:?}"
        );
    }
    // Left alignment starts at the angle: everything to its right on the top.
    let left = boxes(&on_arc("AB", ArcSide::Top, TextAlign::Left).unwrap());
    assert!(left[0].0.0 > -1.0, "{left:?}");
}

#[test]
fn letters_stay_exact_arcs_and_the_same_text_gives_the_same_shapes() {
    let a = on_arc("OSCAR", ArcSide::Top, TextAlign::Center).unwrap();
    let b = on_arc("OSCAR", ArcSide::Top, TextAlign::Center).unwrap();
    assert_eq!(a.drawing.shapes, b.drawing.shapes);
    let arcs = a
        .drawing
        .shapes
        .iter()
        .flat_map(|s| s.segments())
        .filter(|g| matches!(g.to_primitive(), Primitive::Arc(_)))
        .count();
    assert!(arcs > 10, "the O, S and C keep their arcs: {arcs}");
    // The O's points all lie about 50 from the centre, give or take its height.
    for seg in a.drawing.shapes[0].segments() {
        let d = distance(seg.to_primitive().start(), (0.0, 0.0));
        assert!((d - 50.0).abs() < 6.0, "{d}");
    }
}

#[test]
fn too_long_or_too_near_the_centre_is_refused() {
    let long = "A".repeat(80);
    let err = on_arc(&long, ArcSide::Top, TextAlign::Center).unwrap_err();
    assert!(err.contains("longer than its circle"), "{err}");
    let mut new = NewText::new("TWO\nLINES", 10.0);
    new.line_spacing = 6.0;
    new.arc = Some(TextArc {
        radius: 50.0,
        side: ArcSide::Top,
        angle: 90.0,
    });
    let err = add_text(&empty_project(), new, &BuiltinOnly)
        .unwrap_err()
        .message()
        .english();
    assert!(err.contains("line 2") && err.contains("centre"), "{err}");
}

#[test]
fn a_text_on_an_arc_is_saved_and_read_back() {
    let job = on_arc("HI", ArcSide::Bottom, TextAlign::Center).unwrap();
    let saved = serde_json::to_value(&job).unwrap();
    assert_eq!(
        saved["drawing"]["texts"][0]["arc"],
        serde_json::json!({"radius": 50.0, "side": "bottom", "angle": 270.0})
    );
    let straight = add_text(&empty_project(), NewText::new("HI", 10.0), &BuiltinOnly)
        .unwrap()
        .0;
    let saved = serde_json::to_value(&straight).unwrap();
    assert!(saved["drawing"]["texts"][0].get("arc").is_none());
}
