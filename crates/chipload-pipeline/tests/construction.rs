// SPDX-License-Identifier: GPL-3.0-or-later
//! Construction geometry (docs/specs/dxf-export.md, part B): shown and snapped to, never
//! cut.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::{Geometry, Job, load_job};
use chipload_pipeline::drawing_edit::offset_shapes;
use chipload_pipeline::dxfs::{ExportOptions, export_drawing};
use chipload_pipeline::shapes::{add_shape, copy_shapes, set_construction};
use chipload_pipeline::{PipelineError, load_drawing, run_job};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// The drawn part (S1 rectangle, S2 circle, S3 triangle on layer "drawn"; operation 1
/// profiles S2 and S3, operation 2 S1) with a guide line S4 across it on the same layer,
/// touching the rectangle's edge.
fn with_guide() -> (Job, Job) {
    let job = load_job(&corpus("jobs/v5/drawn_part.chipload")).unwrap();
    let (guided, id) = add_shape(
        &job,
        "drawn",
        Geometry::straight_polyline(false, vec![[0.0, 80.0], [200.0, 80.0]]),
    )
    .unwrap();
    let (guided, notes) = set_construction(&guided, &[id], true).unwrap();
    assert!(notes.is_empty());
    (job, guided)
}

#[test]
fn a_guide_changes_nothing_that_is_cut() {
    let (job, guided) = with_guide();
    let gcode = |job: &Job| -> Vec<String> {
        run_job(job)
            .unwrap()
            .files
            .into_iter()
            .map(|f| f.gcode)
            .collect()
    };
    assert_eq!(gcode(&guided), gcode(&job));
    // It is a contour of its own, with no warning about being open.
    let drawing = load_drawing(&guided).unwrap();
    let guides: Vec<_> = drawing
        .contours
        .iter()
        .filter(|c| c.is_construction())
        .collect();
    assert_eq!(guides.len(), 1);
    assert!(
        !drawing
            .report
            .issues
            .iter()
            .any(|i| i.code == "open_contour")
    );
}

#[test]
fn an_operation_that_names_a_guide_is_refused() {
    let (_, guided) = with_guide();
    let (guided, notes) = set_construction(&guided, &[2], true).unwrap();
    assert_eq!(notes.len(), 1);
    let Err(PipelineError::Toolpath(err)) = run_job(&guided) else {
        panic!("the run should be refused");
    };
    assert!(
        err.to_string().contains("S2 is construction geometry"),
        "{err}"
    );
    // Imported shapes can't be guides.
    let imported = load_job(&corpus("jobs/v4/simple_part_full.chipload")).unwrap();
    assert!(set_construction(&imported, &[1], true).is_err());
}

#[test]
fn copies_and_offsets_of_a_guide_are_guides() {
    let (_, guided) = with_guide();
    let (copied, ids) = copy_shapes(&guided, &[4], (0.0, 10.0)).unwrap();
    assert!(
        copied
            .drawing
            .shapes
            .iter()
            .any(|s| s.id == ids[0] && s.construction)
    );
    let edited = offset_shapes(
        &guided,
        &[4],
        5.0,
        None,
        chipload_pipeline::drawing_edit::Corners::Round,
        1,
    )
    .unwrap();
    let new = edited.new_ids[0];
    assert!(
        edited
            .job
            .drawing
            .shapes
            .iter()
            .any(|s| s.id == new && s.construction)
    );
    // No note that operations now cut it: they don't.
    assert!(edited.notes.is_empty());
}

#[test]
fn guides_are_exported_only_when_asked_on_a_layer_of_their_own() {
    let (_, guided) = with_guide();
    let (plain, _) = export_drawing(&guided, &ExportOptions::default()).unwrap();
    assert_eq!(plain.layers, ["drawn"]);
    let with = ExportOptions {
        construction: true,
        ..ExportOptions::default()
    };
    let (out, _) = export_drawing(&guided, &with).unwrap();
    assert_eq!(out.layers, ["drawn", "drawn_construction"]);
}
