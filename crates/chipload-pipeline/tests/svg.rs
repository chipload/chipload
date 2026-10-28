// SPDX-License-Identifier: GPL-3.0-or-later
//! SVGs as drawing files (docs/specs/svg-import.md §2, §4, §7, §11).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::{DrawingFormat, Job, UnitsFrom};
use chipload_pipeline::dxfs::{add_drawing_file, update_drawing_file};
use chipload_pipeline::explode::explode_dxf;
use chipload_pipeline::{empty_project, load_drawing};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// A scratch file with `text`, in a folder of this test's own.
fn scratch(test: &str, name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-svg-{test}"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn badge() -> Job {
    let (job, _) = add_drawing_file(
        &empty_project(),
        &corpus("svg/badge_layers.svg"),
        None,
        None,
        None,
    )
    .unwrap();
    job
}

#[test]
fn an_svg_becomes_a_drawing_file_with_its_layers() {
    let (job, report) = add_drawing_file(
        &empty_project(),
        &corpus("svg/badge_layers.svg"),
        None,
        None,
        None,
    )
    .unwrap();
    assert!(report.lines[0].starts_with("Added "), "{:?}", report.lines);
    assert!(report.lines[0].contains("as SVG 1 \"badge_layers\": 3 shapes"));
    let file = &job.drawing.dxfs[0];
    assert_eq!(file.format, DrawingFormat::Svg);
    assert_eq!((file.units_from, file.px_per_inch), (UnitsFrom::Dxf, None));
    assert_eq!(file.ignored.get("text"), Some(&1));
    let layers: Vec<&str> = job
        .drawing
        .shapes
        .iter()
        .map(|s| s.layer.as_str())
        .collect();
    assert_eq!(layers, ["Outline", "Pocket", "Pocket"]);
    let handles: Vec<&str> = job
        .drawing
        .shapes
        .iter()
        .filter_map(|s| s.dxf_handle())
        .collect();
    assert_eq!(handles, ["outline#1", "leaf#1", "eye#1"]);
    // The leaf lies inside the outline and the eye inside the leaf.
    let drawing = load_drawing(&job).unwrap();
    assert_eq!(drawing.contours.len(), 3);
    assert!(
        drawing
            .report
            .issues
            .iter()
            .any(|i| i.code == "svg_text" && i.message.starts_with("1 text left out"))
    );
}

#[test]
fn an_update_matches_shapes_by_their_element_id() {
    let job = badge();
    let original = std::fs::read_to_string(corpus("svg/badge_layers.svg")).unwrap();
    // The leaf moves, the eye goes, a dot comes; the outline stays.
    let newer = original.replace("d=\"M -25 0 C", "d=\"M -24 0 C").replace(
        r#"<circle id="eye" cx="-12" cy="0" r="3" style="fill:#0000ff"/>"#,
        r#"<circle id="dot" cx="12" cy="0" r="2" style="fill:#0000ff"/>"#,
    );
    assert_ne!(newer, original);
    let path = scratch("update", "badge_layers.svg", &newer);
    let (updated, report) = update_drawing_file(&job, 1, &path, None, None).unwrap();
    assert_eq!(report.unchanged, [1]);
    assert_eq!(report.changed, [2]);
    assert_eq!(report.removed, [3]);
    assert_eq!(report.added, [4]);
    assert_eq!(updated.drawing.dxfs[0].format, DrawingFormat::Svg);
}

#[test]
fn an_svg_in_plain_pixels_needs_its_pixels_per_inch_once() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200">
                   <rect id="r" x="0" y="0" width="96" height="96"/></svg>"#;
    let path = scratch("pixels", "pixels.svg", svg);
    let err = add_drawing_file(&empty_project(), &path, None, None, None).unwrap_err();
    assert_eq!(err.message().id, "svg-error-needs-scale");
    let (job, _) = add_drawing_file(&empty_project(), &path, None, Some(72.0), None).unwrap();
    let file = &job.drawing.dxfs[0];
    assert_eq!(
        (file.units_from, file.px_per_inch),
        (UnitsFrom::User, Some(72.0))
    );
    let drawing = load_drawing(&job).unwrap();
    let ((x0, _), (x1, _)) = drawing.report.bbox.unwrap();
    assert!((x1 - x0 - 96.0 / 72.0 * 25.4).abs() < 1e-3);
    // An update reads it at the pixels per inch chosen before.
    let (again, report) = update_drawing_file(&job, 1, &path, None, None).unwrap();
    assert_eq!(report.unchanged, [1]);
    assert_eq!(again.drawing.dxfs[0].px_per_inch, Some(72.0));
}

#[test]
fn an_svg_explodes_into_drawn_shapes() {
    let (job, report) = explode_dxf(&badge(), 1, None).unwrap();
    assert!(job.drawing.dxfs.is_empty());
    assert!(job.drawing.shapes.iter().all(chipload_job::Shape::is_drawn));
    assert!(
        report.lines[0].starts_with("Exploded SVG 1 \"badge_layers\""),
        "{:?}",
        report.lines
    );
}

#[test]
fn the_golden_badge_is_the_svg_as_imported_now() {
    // The frozen project of golden v31_svg_badge keeps the shapes the import gave; reading
    // the SVG again must give the same, so the golden guards the reader too.
    let frozen = chipload_job::load_job(&corpus("jobs/v31/svg_badge.chipload")).unwrap();
    let now = badge();
    assert_eq!(now.drawing.shapes, frozen.drawing.shapes);
    let (mut a, mut b) = (now.drawing.dxfs[0].clone(), frozen.drawing.dxfs[0].clone());
    a.placement = b.placement;
    b.name.clone_from(&a.name);
    assert_eq!(a, b);
}
