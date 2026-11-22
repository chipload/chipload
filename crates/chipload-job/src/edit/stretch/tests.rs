// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::models::Shape;
use chipload_core::primitives::Line;
use proptest::prelude::*;

fn shape(geometry: Geometry) -> Shape {
    Shape {
        id: 1,
        layer: "0".into(),
        name: None,
        geometry,
        construction: false,
    }
}

fn pieces(geometry: &Geometry) -> Vec<Primitive> {
    shape(geometry.clone())
        .segments()
        .iter()
        .map(|s| s.to_primitive())
        .collect()
}

fn line_distance(line: &Line, p: (f64, f64)) -> f64 {
    let (dx, dy) = (line.end.0 - line.start.0, line.end.1 - line.start.1);
    let length2 = dx * dx + dy * dy;
    let t = if length2 > 0.0 {
        (((p.0 - line.start.0) * dx + (p.1 - line.start.1) * dy) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    math::hypot(p.0 - line.start.0 - t * dx, p.1 - line.start.1 - t * dy)
}

fn arc_distance(arc: &Arc, p: (f64, f64)) -> f64 {
    let angle = math::atan2(p.1 - arc.center.1, p.0 - arc.center.0);
    let along = (angle - arc.start_angle) * arc.sweep.signum();
    if along.rem_euclid(TAU) <= arc.sweep.abs() {
        (math::hypot(p.0 - arc.center.0, p.1 - arc.center.1) - arc.radius).abs()
    } else {
        let (a, b) = (
            arc.point_at(arc.start_angle),
            arc.point_at(arc.start_angle + arc.sweep),
        );
        math::hypot(p.0 - a.0, p.1 - a.1).min(math::hypot(p.0 - b.0, p.1 - b.1))
    }
}

fn distance_to(pieces: &[Primitive], p: (f64, f64)) -> f64 {
    pieces
        .iter()
        .map(|piece| match piece {
            Primitive::Line(line) => line_distance(line, p),
            Primitive::Arc(arc) => arc_distance(arc, p),
        })
        .fold(f64::INFINITY, f64::min)
}

/// Points along `geometry`'s own segments, `per` on each.
fn along(pieces: &[Primitive], per: usize) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    for piece in pieces {
        for k in 0..=per {
            let t = k as f64 / per as f64;
            points.push(match piece {
                Primitive::Line(l) => (
                    l.start.0 + t * (l.end.0 - l.start.0),
                    l.start.1 + t * (l.end.1 - l.start.1),
                ),
                Primitive::Arc(a) => a.point_at(a.start_angle + t * a.sweep),
            });
        }
    }
    points
}

/// The stretched shape and the true stretched curve lie within the arc tolerance of each
/// other, both ways (spec §4).
fn assert_follows(before: &Geometry, about: [f64; 2], fx: f64, fy: f64) {
    let after = stretched(before, about, fx, fy).unwrap().geometry;
    let map = |p: (f64, f64)| {
        (
            about[0] + fx * (p.0 - about[0]),
            about[1] + fy * (p.1 - about[1]),
        )
    };
    let truth: Vec<(f64, f64)> = along(&pieces(before), 200).into_iter().map(map).collect();
    let fitted = pieces(&after);
    let slack = ARC_APPROX_TOL_MM + 1e-9;
    for p in &truth {
        let d = distance_to(&fitted, *p);
        assert!(
            d <= slack,
            "the stretched curve at {p:?} is {d} from the shape"
        );
    }
    // Back the other way: the shape against the true curve, measured on a dense sample
    // of the true curve's pieces stretched (exact enough at 2000 a piece).
    let dense: Vec<(f64, f64)> = along(&pieces(before), 2000).into_iter().map(map).collect();
    for p in along(&fitted, 50) {
        let d = dense
            .windows(2)
            .map(|w| {
                line_distance(
                    &Line {
                        start: w[0],
                        end: w[1],
                    },
                    p,
                )
            })
            .fold(f64::INFINITY, f64::min);
        assert!(
            d <= slack,
            "the shape at {p:?} is {d} from the stretched curve"
        );
    }
}

#[test]
fn straight_sides_move_their_points_exactly() {
    let before = Geometry::Polyline {
        closed: false,
        points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]],
        bulges: vec![0.0, 0.0],
        radii: vec![0.0, 2.0, 0.0],
    };
    let after = stretched(&before, [0.0, 0.0], 2.0, 3.0).unwrap();
    assert!(!after.changed);
    assert_eq!(
        after.geometry,
        Geometry::Polyline {
            closed: false,
            points: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 15.0]],
            bulges: vec![0.0, 0.0],
            radii: vec![0.0, 2.0, 0.0],
        }
    );
}

#[test]
fn a_square_rectangle_stays_a_rectangle_with_its_radii() {
    let flat = Geometry::Rectangle {
        center: [10.0, 10.0],
        length: 40.0,
        width: 20.0,
        angle: 0.0,
        corner_radius: 3.0,
        radii: Vec::new(),
    };
    let after = stretched(&flat, [0.0, 0.0], 1.5, 0.5).unwrap();
    assert!(!after.changed);
    assert_eq!(
        after.geometry,
        Geometry::Rectangle {
            center: [15.0, 5.0],
            length: 60.0,
            width: 10.0,
            angle: 0.0,
            corner_radius: 3.0,
            radii: Vec::new(),
        }
    );
    // Turned a quarter: its length lies along Y and takes the Y factor.
    let upright = Geometry::Rectangle {
        center: [10.0, 10.0],
        length: 40.0,
        width: 20.0,
        angle: FRAC_PI_2,
        corner_radius: 3.0,
        radii: Vec::new(),
    };
    let Geometry::Rectangle { length, width, .. } =
        stretched(&upright, [0.0, 0.0], 1.5, 0.5).unwrap().geometry
    else {
        panic!("still a rectangle");
    };
    assert_eq!((length, width), (20.0, 30.0));
}

#[test]
fn a_turned_rectangle_becomes_a_polyline_of_its_stretched_corners() {
    let before = Geometry::Rectangle {
        center: [0.0, 0.0],
        length: 20.0,
        width: 10.0,
        angle: 0.5,
        corner_radius: 1.0,
        radii: Vec::new(),
    };
    let after = stretched(&before, [0.0, 0.0], 2.0, 1.0).unwrap();
    assert!(after.changed);
    let Geometry::Polyline {
        closed,
        points,
        bulges,
        radii,
    } = after.geometry
    else {
        panic!("a polyline");
    };
    assert!(closed);
    let corners = rectangle_corners([0.0, 0.0], 20.0, 10.0, 0.5);
    for (p, c) in points.iter().zip(corners) {
        assert_eq!(*p, [2.0 * c[0], c[1]]);
    }
    assert_eq!(bulges, vec![0.0; 4]);
    assert_eq!(radii, vec![1.0; 4]);
}

#[test]
fn a_polygon_becomes_a_polyline_keeping_its_radii() {
    let before = Geometry::Polygon {
        center: [0.0, 0.0],
        radius: 10.0,
        sides: 6,
        angle: 0.0,
        radii: vec![1.0; 6],
    };
    let after = stretched(&before, [0.0, 0.0], 1.0, 2.0).unwrap();
    assert!(after.changed);
    let Geometry::Polyline { points, radii, .. } = after.geometry else {
        panic!("a polyline");
    };
    assert_eq!(points.len(), 6);
    assert_eq!(points[0], [10.0, 0.0]);
    assert_eq!(radii, vec![1.0; 6]);
}

#[test]
fn a_circle_becomes_the_fitted_ellipse() {
    let circle = Geometry::Circle {
        center: [5.0, 5.0],
        radius: 20.0,
    };
    let after = stretched(&circle, [0.0, 0.0], 2.0, 1.0).unwrap();
    assert!(after.changed);
    assert!(matches!(
        after.geometry,
        Geometry::Polyline { closed: true, .. }
    ));
    assert_follows(&circle, [0.0, 0.0], 2.0, 1.0);
}

#[test]
fn an_arc_side_is_fitted_and_its_neighbours_keep_their_radii() {
    // A slot: two half circles joined by straight sides, a radius on none.
    let slot = Geometry::Polyline {
        closed: true,
        points: vec![[0.0, 0.0], [30.0, 0.0], [30.0, 10.0], [0.0, 10.0]],
        bulges: vec![0.0, 1.0, 0.0, 1.0],
        radii: vec![],
    };
    let after = stretched(&slot, [15.0, 5.0], 1.0, 1.6).unwrap();
    assert!(after.changed);
    assert_follows(&slot, [15.0, 5.0], 1.0, 1.6);
    // A radius at a straight corner stays on that corner.
    let corner = Geometry::Polyline {
        closed: false,
        points: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [40.0, 20.0]],
        bulges: vec![0.0, 0.0, 0.5],
        radii: vec![0.0, 4.0, 0.0, 0.0],
    };
    let Geometry::Polyline { points, radii, .. } =
        stretched(&corner, [0.0, 0.0], 1.3, 0.7).unwrap().geometry
    else {
        panic!("a polyline");
    };
    assert_eq!(radii[1], 4.0);
    assert_eq!(points[1], [26.0, 0.0]);
    assert!(radii[3..].iter().all(|r| *r == 0.0));
}

#[test]
fn a_spline_keeps_its_points_stretched() {
    let before = Geometry::Spline {
        closed: false,
        points: vec![[0.0, 0.0], [10.0, 5.0], [20.0, 0.0]],
    };
    let after = stretched(&before, [0.0, 0.0], 2.0, 1.0).unwrap();
    assert!(!after.changed);
    assert_eq!(
        after.geometry,
        Geometry::Spline {
            closed: false,
            points: vec![[0.0, 0.0], [20.0, 5.0], [40.0, 0.0]],
        }
    );
}

#[test]
fn factors_that_arent_positive_numbers_stretch_nothing() {
    let circle = Geometry::Circle {
        center: [0.0, 0.0],
        radius: 1.0,
    };
    for (fx, fy) in [
        (0.0, 1.0),
        (1.0, -1.0),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
    ] {
        assert!(stretched(&circle, [0.0, 0.0], fx, fy).is_none());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Arcs of every size and sweep, stretched by factors from 0.2 to 5, stay within the
    /// arc tolerance of the true stretched curve (spec §4).
    #[test]
    fn stretched_arcs_follow_the_true_curve(
        radius in 0.5f64..200.0,
        bulge in -2.0f64..2.0,
        fx in 0.2f64..5.0,
        fy in 0.2f64..5.0,
        turn in 0.0f64..TAU,
    ) {
        prop_assume!(bulge.abs() > 0.05);
        let (sin, cos) = math::sin_cos(turn);
        let a = [radius * cos, radius * sin];
        let b = [-radius * sin, radius * cos];
        let side = Geometry::Polyline {
            closed: false,
            points: vec![a, b],
            bulges: vec![bulge],
            radii: Vec::new(),
        };
        assert_follows(&side, [3.0, -2.0], fx, fy);
    }
}
