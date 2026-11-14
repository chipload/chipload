// SPDX-License-Identifier: GPL-3.0-or-later
//! What a selection is good for, and selecting by kind (docs/specs/fewer-clicks.md §2, §3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chipload_core::chain::Contour;
use chipload_job::{Geometry, Job, load_job};
use chipload_pipeline::load_drawing;
use chipload_pipeline::new_ops::OpKind;
use chipload_pipeline::selection::{Unfit, unfit_for};
use chipload_pipeline::shapes::add_shape;

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// The drawn part (S1 rectangle, S2 circle, S3 triangle on layer "drawn") with an open line
/// S4.
fn drawn() -> (Job, Vec<Contour>) {
    let job = load_job(&corpus("jobs/v5/drawn_part.chipload")).unwrap();
    let (job, _) = add_shape(
        &job,
        "drawn",
        Geometry::straight_polyline(false, vec![[0.0, 180.0], [200.0, 180.0]]),
    )
    .unwrap();
    let contours = load_drawing(&job).unwrap().contours;
    (job, contours)
}

/// The contour ids of shapes `shapes`.
fn of(contours: &[Contour], shapes: &[u64]) -> BTreeSet<usize> {
    contours
        .iter()
        .filter(|c| {
            c.parts
                .iter()
                .any(|p| p.segment.shape.is_some_and(|s| shapes.contains(&s)))
        })
        .map(|c| c.id)
        .collect()
}

#[test]
fn each_kind_says_what_the_selection_needs() {
    let (_, all) = drawn();
    let (rectangle, circle, line) = (of(&all, &[1]), of(&all, &[2]), of(&all, &[4]));
    assert_eq!((rectangle.len(), circle.len(), line.len()), (1, 1, 1));
    assert_eq!(
        unfit_for(OpKind::Profile, &BTreeSet::new(), &all),
        Some(Unfit::Nothing)
    );
    assert_eq!(unfit_for(OpKind::Profile, &line, &all), None);
    assert_eq!(unfit_for(OpKind::Path, &line, &all), None);
    assert_eq!(
        unfit_for(OpKind::Pocket, &line, &all),
        Some(Unfit::NoClosed)
    );
    assert_eq!(
        unfit_for(OpKind::Vcarve, &line, &all),
        Some(Unfit::NoClosed)
    );
    assert_eq!(
        unfit_for(OpKind::Holes, &rectangle, &all),
        Some(Unfit::NoCircles)
    );
    assert_eq!(unfit_for(OpKind::Holes, &circle, &all), None);
    assert_eq!(unfit_for(OpKind::Pocket, &rectangle, &all), None);
    // A mix takes what any of it can.
    let mixed: BTreeSet<usize> = line.union(&circle).copied().collect();
    assert_eq!(unfit_for(OpKind::Pocket, &mixed, &all), None);
    assert_eq!(unfit_for(OpKind::Holes, &mixed, &all), None);
}
