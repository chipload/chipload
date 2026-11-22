// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use chipload_job::{Geometry, Shape};

fn with(shapes: Vec<Geometry>) -> Job {
    let mut job = crate::empty_project();
    for (i, geometry) in shapes.into_iter().enumerate() {
        job.drawing.shapes.push(Shape {
            id: i as u64 + 1,
            layer: "0".into(),
            name: None,
            geometry,
            construction: false,
        });
    }
    job.drawing.next_shape_id = job.drawing.shapes.len() as u64 + 1;
    job
}

fn line(a: [f64; 2], b: [f64; 2]) -> Geometry {
    Geometry::Polyline {
        closed: false,
        points: vec![a, b],
        bulges: vec![0.0],
        radii: Vec::new(),
    }
}

fn points(job: &Job, id: u64) -> Vec<[f64; 2]> {
    match &job
        .drawing
        .shapes
        .iter()
        .find(|s| s.id == id)
        .unwrap()
        .geometry
    {
        Geometry::Polyline { points, .. } => points.clone(),
        other => panic!("a polyline, got {other:?}"),
    }
}

#[test]
fn a_reference_scales_so_its_length_is_the_new_one_and_its_first_point_stays() {
    let job = with(vec![line([10.0, 10.0], [40.0, 50.0])]);
    let scaled = scale_by_reference(&job, &[1], [10.0, 10.0], [40.0, 50.0], 100.0).unwrap();
    let p = points(&scaled, 1);
    assert_eq!(p[0], [10.0, 10.0]);
    let length = chipload_core::math::hypot(p[1][0] - p[0][0], p[1][1] - p[0][1]);
    assert!((length - 100.0).abs() < CHAIN_TOL_MM, "{length}");
}

#[test]
fn a_reference_off_the_shapes_measures_elsewhere() {
    // Two points on another shape set the factor: 20 → 30, ×1.5 about the first point.
    let job = with(vec![
        line([0.0, 0.0], [10.0, 0.0]),
        line([0.0, 5.0], [20.0, 5.0]),
    ]);
    let scaled = scale_by_reference(&job, &[1], [0.0, 5.0], [20.0, 5.0], 30.0).unwrap();
    let p = points(&scaled, 1);
    assert!(
        (p[1][0] - 15.0).abs() < 1e-9 && (p[1][1] + 2.5).abs() < 1e-9,
        "{p:?}"
    );
    assert_eq!(points(&scaled, 2), points(&job, 2));
}

#[test]
fn a_reference_is_refused_without_a_length() {
    let job = with(vec![line([0.0, 0.0], [10.0, 0.0])]);
    for (b, length) in [
        ([0.0, 0.0], 5.0),
        ([10.0, 0.0], 0.0),
        ([10.0, 0.0], -1.0),
        ([10.0, 0.0], f64::NAN),
    ] {
        assert!(scale_by_reference(&job, &[1], [0.0, 0.0], b, length).is_err());
    }
}

#[test]
fn a_stretch_to_a_width_keeps_the_middle_and_the_height() {
    let job = with(vec![line([0.0, 0.0], [10.0, 4.0])]);
    let done = stretch_shapes(&job, &[1], None, Along::Size(20.0), Along::Factor(1.0)).unwrap();
    assert_eq!(points(&done.job, 1), vec![[-5.0, 0.0], [15.0, 4.0]]);
    assert_eq!(done.factors, (2.0, 1.0));
    assert!(done.fitted.is_empty());
}

#[test]
fn a_stretched_circle_is_reported_as_fitted_and_equal_factors_keep_it_round() {
    let circle = Geometry::Circle {
        center: [0.0, 0.0],
        radius: 10.0,
    };
    let job = with(vec![circle]);
    let done = stretch_shapes(
        &job,
        &[1],
        Some([0.0, 0.0]),
        Along::Factor(2.0),
        Along::Factor(1.0),
    )
    .unwrap();
    assert_eq!(done.fitted, vec![1]);
    let even = stretch_shapes(
        &job,
        &[1],
        Some([0.0, 0.0]),
        Along::Factor(2.0),
        Along::Size(40.0),
    )
    .unwrap();
    assert!(even.fitted.is_empty());
    assert_eq!(
        even.job.drawing.shapes[0].geometry,
        Geometry::Circle {
            center: [0.0, 0.0],
            radius: 20.0
        }
    );
}

#[test]
fn a_stretch_is_refused_when_a_radius_no_longer_fits() {
    let rounded = Geometry::Rectangle {
        center: [0.0, 0.0],
        length: 40.0,
        width: 20.0,
        angle: 0.0,
        corner_radius: 8.0,
        radii: Vec::new(),
    };
    let job = with(vec![rounded]);
    assert!(stretch_shapes(&job, &[1], None, Along::Factor(1.0), Along::Factor(0.5)).is_err());
    assert!(stretch_shapes(&job, &[1], None, Along::Factor(1.0), Along::Factor(0.9)).is_ok());
}

#[test]
fn a_stretch_is_refused_for_bad_sizes_and_flat_shapes() {
    let job = with(vec![line([0.0, 0.0], [10.0, 0.0])]);
    assert!(stretch_shapes(&job, &[1], None, Along::Factor(1.0), Along::Size(5.0)).is_err());
    assert!(stretch_shapes(&job, &[1], None, Along::Size(0.0), Along::Factor(1.0)).is_err());
    assert!(stretch_shapes(&job, &[1], None, Along::Factor(-1.0), Along::Factor(1.0)).is_err());
    assert!(stretch_shapes(&job, &[2], None, Along::Factor(2.0), Along::Factor(1.0)).is_err());
}
