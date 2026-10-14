// SPDX-License-Identifier: GPL-3.0-or-later
//! Pictures in a project (docs/specs/image-trace.md §4, §5, §9).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_job::{ImageThreshold, ImageTrace, Selector};
use chipload_pipeline::images::{
    NewImage, add_image, black_white_png, remove_image, retrace_changed,
};
use chipload_pipeline::{empty_project, placement};

fn logo() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/images/logo.png"))
        .unwrap()
}

/// Every traced point's (min, max) over all shapes of the image.
fn extent(job: &chipload_job::Job) -> ((f64, f64), (f64, f64)) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for shape in job.drawing.shapes.iter().filter(|s| s.image().is_some()) {
        for segment in shape.segments() {
            let piece = match segment {
                chipload_job::PathSegment::Line(l) => Primitive::Line(Line {
                    start: (l.start[0], l.start[1]),
                    end: (l.end[0], l.end[1]),
                }),
                chipload_job::PathSegment::Arc(a) => Primitive::Arc(Arc {
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

#[test]
fn a_picture_is_added_traced_and_placed() {
    let bytes = logo();
    let mut new = NewImage::new("logo.png", &bytes, 160.0);
    new.x = 10.0;
    new.y = 5.0;
    let (job, report) = add_image(&empty_project(), new).unwrap();
    let image = &job.drawing.images[0];
    assert_eq!((image.id, image.layer.as_str()), (1, "logo"));
    assert_eq!(image.threshold, ImageThreshold::AUTO);
    // The ring's two edges, the bar's outline and its hole.
    assert_eq!(report.shapes.len(), 4, "{:?}", report.lines);
    assert_eq!((report.width, report.height), (160.0, 80.0));
    assert!(
        report.lines[0].starts_with("Traced logo.png (160.0 × 80.0 mm"),
        "{:?}",
        report.lines
    );
    // 2 mm a pixel: the ring's outside reaches from x = 10 + 2·5 to the bar's right edge.
    let ((x0, y0), (x1, y1)) = extent(&job);
    assert!(
        (x0 - 20.0).abs() < 1.0 && (x1 - 160.0).abs() < 1.0,
        "{x0} {x1}"
    );
    assert!(
        (y0 - 15.0).abs() < 1.0 && (y1 - 75.0).abs() < 1.0,
        "{y0} {y1}"
    );
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    assert!(
        job.drawing
            .shapes
            .iter()
            .all(|s| s.layer == "logo" && s.closed())
    );
}

/// docs/specs/direct-placement.md §3: a turned picture is traced as it is, then its
/// shapes are turned about its centre.
#[test]
fn a_turned_picture_has_the_same_trace_turned_about_its_centre() {
    let bytes = logo();
    let mut new = NewImage::new("logo.png", &bytes, 160.0);
    new.x = 10.0;
    new.y = 5.0;
    let (job, report) = add_image(&empty_project(), new).unwrap();
    let mut turned = job.clone();
    turned.drawing.images[0].rotation = 45.0;
    let (turned, again) = chipload_pipeline::images::retrace(&turned, 1).unwrap();
    assert_eq!(again.shapes.len(), report.shapes.len());
    // The picture's centre: (10 + 80, 5 + 40).
    let c = [90.0, 45.0];
    let before: Vec<_> = job
        .drawing
        .shapes
        .iter()
        .flat_map(|s| s.segments())
        .collect();
    let after: Vec<_> = turned
        .drawing
        .shapes
        .iter()
        .flat_map(|s| s.segments())
        .collect();
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        let (chipload_job::PathSegment::Line(a), chipload_job::PathSegment::Line(b)) = (a, b)
        else {
            continue;
        };
        let p = chipload_job::rotated([a.start[0] - c[0], a.start[1] - c[1]], 45.0);
        assert!((p[0] + c[0] - b.start[0]).abs() < 1e-9, "{a:?} {b:?}");
        assert!((p[1] + c[1] - b.start[1]).abs() < 1e-9, "{a:?} {b:?}");
    }
    // Its box: an 160 × 80 rectangle at 45° is (160 + 80) / √2 wide and high.
    let ((x0, y0), (x1, y1)) = chipload_pipeline::images::turned_box(10.0, 5.0, 160.0, 80.0, 45.0);
    let side = 240.0 / 2.0_f64.sqrt();
    assert!((x1 - x0 - side).abs() < 1e-9 && (y1 - y0 - side).abs() < 1e-9);
    assert!(((x0 + x1) / 2.0 - 90.0).abs() < 1e-9 && ((y0 + y1) / 2.0 - 45.0).abs() < 1e-9);
}

#[test]
fn a_changed_setting_traces_again_and_operations_follow_the_layer() {
    let bytes = logo();
    let (job, first) =
        add_image(&empty_project(), NewImage::new("logo.png", &bytes, 80.0)).unwrap();
    let mut changed = job.clone();
    changed.drawing.images[0].invert = true;
    let (after, reports) = retrace_changed(&job, &changed).unwrap();
    assert_eq!(reports.len(), 1);
    // Inverted: the background is black now, closed along the picture's edge.
    assert!(reports[0].black_at_border);
    assert!(
        reports[0]
            .shapes
            .iter()
            .all(|id| !first.shapes.contains(id))
    );
    assert_eq!(
        after.drawing.shapes.len(),
        reports[0].shapes.len(),
        "the old shapes are gone"
    );
    // Nothing changed: nothing is traced again.
    let (same, none) = retrace_changed(&after, &after).unwrap();
    assert!(none.is_empty());
    assert_eq!(same, after);
}

#[test]
fn traced_shapes_are_selected_by_their_image_not_by_id() {
    let bytes = logo();
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v18/rest.chipload");
    let project = chipload_job::load_job(&corpus).unwrap();
    let (mut job, report) = add_image(&project, NewImage::new("logo.png", &bytes, 80.0)).unwrap();
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    *job.operations[0].select_mut() = Selector {
        shapes: Some(vec![report.shapes[0]]),
        ..Selector::default()
    };
    let problems = job.validate();
    assert!(
        problems
            .iter()
            .any(|p| p.text.contains("select the image instead ({\"image\": 1})")),
        "{problems:?}"
    );
}

#[test]
fn removing_an_image_removes_its_shapes() {
    let bytes = logo();
    let (job, _) = add_image(&empty_project(), NewImage::new("logo.png", &bytes, 80.0)).unwrap();
    let job = remove_image(&job, 1).unwrap();
    assert!(job.drawing.images.is_empty() && job.drawing.shapes.is_empty());
    assert!(remove_image(&job, 1).is_err());
}

#[test]
fn the_drawings_placement_leaves_traced_shapes_alone() {
    let bytes = logo();
    let (job, _) = add_image(&empty_project(), NewImage::new("logo.png", &bytes, 80.0)).unwrap();
    // No DXF: there is nothing to place, and the traced shapes don't count as one.
    assert!(placement::imported_size(&job.drawing.shapes).is_none());
}

#[test]
fn the_black_and_white_preview_is_a_png() {
    let bytes = logo();
    let (job, _) = add_image(&empty_project(), NewImage::new("logo.png", &bytes, 80.0)).unwrap();
    let png = black_white_png(&job, 1).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let decoded = chipload_image::decode(&png).unwrap();
    assert_eq!((decoded.width, decoded.height), (80, 40));
}

#[test]
fn bad_pictures_and_settings_are_refused() {
    let project = empty_project();
    assert!(add_image(&project, NewImage::new("x.png", b"nonsense", 80.0)).is_err());
    let bytes = logo();
    let mut bad = NewImage::new("logo.png", &bytes, 80.0);
    bad.smoothing = 0.0;
    let error = add_image(&project, bad).unwrap_err().to_string();
    assert!(error.contains("smoothing"), "{error}");
    let zero = NewImage::new("logo.png", &bytes, 0.0);
    assert!(add_image(&project, zero).is_err());
}

// -- Arcs and centre lines (docs/specs/image-arcs-centrelines.md §8)

fn strokes() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/images/strokes.png"),
    )
    .unwrap()
}

#[test]
fn new_images_get_arcs_and_far_fewer_segments() {
    let bytes = logo();
    let count = |arcs: bool| {
        let mut new = NewImage::new("logo.png", &bytes, 80.0);
        new.arcs = arcs;
        let (job, report) = add_image(&empty_project(), new).unwrap();
        assert_eq!(report.shapes.len(), 4);
        let segments: Vec<chipload_job::PathSegment> = job
            .drawing
            .shapes
            .iter()
            .flat_map(|s| s.segments())
            .collect();
        let arcs = segments
            .iter()
            .filter(|s| matches!(s, chipload_job::PathSegment::Arc(_)))
            .count();
        (segments.len(), arcs)
    };
    let (lines, none) = count(false);
    let (fitted, arcs) = count(true);
    assert_eq!(none, 0);
    assert!(
        arcs > 0 && fitted * 3 < lines,
        "{fitted} ({arcs} arcs) against {lines}"
    );
}

#[test]
fn a_picture_of_strokes_traces_into_centre_lines() {
    let bytes = strokes();
    let mut new = NewImage::new("strokes.png", &bytes, 60.0);
    new.trace = ImageTrace::CentreLine;
    let (job, report) = add_image(&empty_project(), new).unwrap();
    // The T's three strokes and the O; the dot is left out.
    assert_eq!(report.shapes.len(), 4, "{:?}", report.lines);
    assert!(report.centre_lines);
    assert_eq!(report.dots, 1);
    let (narrow, wide) = report.widths.unwrap();
    // 6 pixels of 0.5 mm.
    assert!(narrow > 2.5 && wide < 3.5, "{narrow} {wide}");
    let closed = job.drawing.shapes.iter().filter(|s| s.closed()).count();
    assert_eq!(closed, 1, "the O");
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    assert!(report.lines.iter().any(|l| l.contains("follow path")));
}

#[test]
fn only_a_follow_path_may_cut_open_centre_lines() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v18/rest.chipload");
    let project = chipload_job::load_job(&corpus).unwrap();
    // A T and an L: open strokes only.
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/images/open_strokes.png"),
    )
    .unwrap();
    let mut new = NewImage::new("open_strokes.png", &bytes, 60.0);
    new.trace = ImageTrace::CentreLine;
    let (mut job, _) = add_image(&project, new).unwrap();
    assert!(
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.image() == Some(1))
            .all(|s| !s.closed())
    );
    *job.operations[0].select_mut() = Selector {
        layer: Some("open_strokes".into()),
        ..Selector::default()
    };
    let problems = job.validate();
    assert!(
        problems.iter().any(
            |p| p.text.contains("image 1 is traced as open centre lines")
                && p.text.contains("follow path")
        ),
        "{problems:?}"
    );
    // Selecting the image itself is refused the same way.
    *job.operations[0].select_mut() = Selector {
        image: Some(1),
        ..Selector::default()
    };
    assert!(
        job.validate()
            .iter()
            .any(|p| p.text.contains("image 1 is traced as open centre lines")),
        "{:?}",
        job.validate()
    );
    // A picture with a closed line too (a T and an O): allowed; the O is the area.
    let strokes = strokes();
    let mut new = NewImage::new("strokes.png", &strokes, 60.0);
    new.trace = ImageTrace::CentreLine;
    let (mut job, _) = add_image(&project, new).unwrap();
    *job.operations[0].select_mut() = Selector {
        image: Some(1),
        ..Selector::default()
    };
    assert!(job.validate().is_empty(), "{:?}", job.validate());
}

#[test]
fn a_closed_centre_line_can_be_profiled_like_any_outline() {
    // User report 2026-10-13: an outline drawn by hand and scanned traces as one closed
    // centre line, and a profile round it was refused.
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/images/hand_outline.png"),
    )
    .unwrap();
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v18/rest.chipload");
    let project = chipload_job::load_job(&corpus).unwrap();
    let mut new = NewImage::new("hand_outline.png", &bytes, 80.0);
    new.trace = ImageTrace::CentreLine;
    let (mut job, _) = add_image(&project, new).unwrap();
    let traced: Vec<_> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.image() == Some(1))
        .collect();
    assert_eq!(traced.len(), 1);
    assert!(traced[0].closed());
    // A profile outside it, alone in the project.
    let op: chipload_job::Operation = serde_json::from_value(serde_json::json!({
        "type": "profile", "part": 1, "select": {"image": 1}, "side": "outside",
        "direction": "climb", "tool": job.tools[0].id, "start_depth": 0, "end_depth": 3,
        "clearance": 0, "step_down": 3, "feed": 1000, "plunge_feed": 300}))
    .unwrap();
    job.operations = vec![op];
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    chipload_pipeline::run_job(&job).unwrap();
}

// -- leaving shapes out of a whole picture (docs/specs/leave-out-shapes.md)

#[test]
fn left_out_shapes_are_found_again_when_the_picture_is_traced_again() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v18/rest.chipload");
    let project = chipload_job::load_job(&corpus).unwrap();
    // A T and an O, as centre lines: the O (closed) is left out of a follow path.
    let strokes = strokes();
    let mut new = NewImage::new("strokes.png", &strokes, 60.0);
    new.trace = ImageTrace::CentreLine;
    let (mut job, _) = add_image(&project, new).unwrap();
    let ring = job
        .drawing
        .shapes
        .iter()
        .find(|s| s.image() == Some(1) && s.closed())
        .unwrap()
        .id;
    let op: chipload_job::Operation = serde_json::from_value(serde_json::json!({
        "type": "path", "part": 1, "select": {"image": 1, "exclude": [ring]},
        "tool": job.tools[0].id, "start_depth": 0, "end_depth": 1, "step_down": 1,
        "feed": 1000, "plunge_feed": 300, "entry": {"type": "plunge"}}))
    .unwrap();
    job.operations = vec![op];
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    // Moved and made larger: the ring is found again under its new id.
    job.drawing.images[0].x += 25.0;
    job.drawing.images[0].width = 80.0;
    let (moved, report) = chipload_pipeline::images::retrace(&job, 1).unwrap();
    let new_ring = moved
        .drawing
        .shapes
        .iter()
        .find(|s| s.image() == Some(1) && s.closed())
        .unwrap()
        .id;
    assert_ne!(new_ring, ring);
    assert_eq!(moved.operations[0].select().exclude, Some(vec![new_ring]));
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("1 left-out shape found again")),
        "{:?}",
        report.lines
    );
    assert!(moved.validate().is_empty(), "{:?}", moved.validate());
    // Cut: the T only, not the ring.
    let result = chipload_pipeline::run_job(&moved).unwrap();
    assert!(!result.program.moves.is_empty());
    // Traced as outlines instead, there is no line like it: not found, and it blocks.
    let mut outlines = moved.clone();
    outlines.drawing.images[0].trace = ImageTrace::Outline;
    let (outlines, report) = chipload_pipeline::images::retrace(&outlines, 1).unwrap();
    assert!(
        report.lines.iter().any(|l| l.contains("not found again")),
        "{:?}",
        report.lines
    );
    assert!(
        outlines
            .validate()
            .iter()
            .any(|p| p.text.contains("select.exclude")
                && p.text.contains("pick the shapes to leave out again")),
        "{:?}",
        outlines.validate()
    );
}
