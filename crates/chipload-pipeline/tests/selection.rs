// SPDX-License-Identifier: GPL-3.0-or-later
//! What a selection is good for, and selecting by kind (docs/specs/fewer-clicks.md §2, §3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chipload_core::chain::Contour;
use chipload_job::{Geometry, Job, load_job};
use chipload_pipeline::load_drawing;
use chipload_pipeline::new_ops::OpKind;
use chipload_pipeline::selection::{
    Unfit, contour_of_shape, inside, shapes_of, similar, unfit_for,
};
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

fn rect(x: f64, y: f64, length: f64, width: f64) -> Geometry {
    Geometry::Rectangle {
        center: [x, y],
        length,
        width,
        angle: 0.0,
        corner_radius: 0.0,
    }
}

fn circle(x: f64, y: f64, d: f64) -> Geometry {
    Geometry::Circle {
        center: [x, y],
        radius: d / 2.0,
    }
}

/// Two parts side by side, each an outline with pockets and holes; some on other layers.
/// Returns the job, its contours and the shape ids by name.
fn parts() -> (Vec<Contour>, std::collections::BTreeMap<&'static str, u64>) {
    let mut job = chipload_pipeline::empty_project();
    let mut ids = std::collections::BTreeMap::new();
    let shapes: [(&'static str, &str, Geometry); 12] = [
        ("outline a", "cut", rect(50.0, 50.0, 90.0, 90.0)),
        ("slot a", "pocket", rect(30.0, 30.0, 20.0, 10.0)),
        ("slot a turned", "pocket", rect(60.0, 60.0, 10.0, 20.0)),
        ("hole a", "drill", circle(30.0, 70.0, 5.0)),
        ("island", "pocket", rect(70.0, 25.0, 20.0, 20.0)),
        ("island hole", "drill", circle(70.0, 25.0, 5.0)),
        ("outline b", "cut", rect(200.0, 50.0, 90.0, 90.0)),
        ("slot b", "pocket", rect(180.0, 30.0, 20.0, 10.0)),
        ("slot b other layer", "other", rect(210.0, 30.0, 20.0, 10.0)),
        ("hole b other layer", "other", circle(180.0, 70.0, 5.0)),
        ("hole b bigger", "drill", circle(210.0, 70.0, 6.0)),
        (
            "line",
            "pocket",
            Geometry::straight_polyline(false, vec![[150.0, 30.0], [150.0, 70.0]]),
        ),
    ];
    for (name, layer, geometry) in shapes {
        let (next, id) = add_shape(&job, layer, geometry).unwrap();
        job = next;
        ids.insert(name, id);
    }
    let contours = load_drawing(&job).unwrap().contours;
    (contours, ids)
}

fn named(
    contours: &[Contour],
    ids: &std::collections::BTreeMap<&'static str, u64>,
    found: &BTreeSet<usize>,
) -> Vec<&'static str> {
    let shapes = shapes_of(contours, found);
    ids.iter()
        .filter(|(_, id)| shapes.contains(id))
        .map(|(name, _)| *name)
        .collect()
}

#[test]
fn similar_circles_by_diameter_on_any_layer() {
    let (all, ids) = parts();
    let hole = contour_of_shape(&all, ids["hole a"]).unwrap();
    assert_eq!(
        named(&all, &ids, &similar(&all, hole)),
        ["hole a", "hole b other layer", "island hole"]
    );
}

#[test]
fn similar_shapes_by_box_either_way_round_on_the_same_layer() {
    let (all, ids) = parts();
    let slot = contour_of_shape(&all, ids["slot a"]).unwrap();
    assert_eq!(
        named(&all, &ids, &similar(&all, slot)),
        ["slot a", "slot a turned", "slot b"]
    );
    // An open line: only open ones, of its size.
    let line = contour_of_shape(&all, ids["line"]).unwrap();
    assert_eq!(named(&all, &ids, &similar(&all, line)), ["line"]);
    // The island (20 × 20) is not a slot.
    let island = contour_of_shape(&all, ids["island"]).unwrap();
    assert_eq!(named(&all, &ids, &similar(&all, island)), ["island"]);
}

#[test]
fn inside_takes_the_closed_shapes_within_nested_too() {
    let (all, ids) = parts();
    let outline = contour_of_shape(&all, ids["outline a"]).unwrap();
    assert_eq!(
        named(&all, &ids, &inside(&all, outline).unwrap()),
        ["hole a", "island", "island hole", "slot a", "slot a turned"]
    );
    // The other part's, without the line (open, and outside both anyway).
    let other = contour_of_shape(&all, ids["outline b"]).unwrap();
    assert_eq!(
        named(&all, &ids, &inside(&all, other).unwrap()),
        [
            "hole b bigger",
            "hole b other layer",
            "slot b",
            "slot b other layer"
        ]
    );
    // An open contour has no inside.
    let line = contour_of_shape(&all, ids["line"]).unwrap();
    assert_eq!(inside(&all, line), None);
    // Nothing inside a hole.
    let hole = contour_of_shape(&all, ids["hole a"]).unwrap();
    assert!(inside(&all, hole).unwrap().is_empty());
}
