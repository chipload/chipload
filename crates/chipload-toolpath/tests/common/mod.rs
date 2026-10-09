// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(dead_code)]

use chipload_core::Point;
use chipload_core::chain::{Contour, OrientedSegment, Segment};
use chipload_core::primitives::{Arc, Line, Primitive};

pub fn segment(prim: Primitive) -> OrientedSegment {
    OrientedSegment {
        segment: Segment {
            prim,
            layer: "L".into(),
            handle: "1".into(),
            source_type: "LWPOLYLINE".into(),
            shape: None,
        },
        reversed: false,
    }
}

/// A closed contour through `(x, y, bulge)` vertices, like an LWPOLYLINE.
pub fn bulged(id: usize, vertices: &[(f64, f64, f64)]) -> Contour {
    let n = vertices.len();
    let parts = (0..n)
        .map(|i| {
            let (x1, y1, b) = vertices[i];
            let (x2, y2, _) = vertices[(i + 1) % n];
            segment(match Arc::from_bulge((x1, y1), (x2, y2), b) {
                Some(arc) => Primitive::Arc(arc),
                None => Primitive::Line(Line {
                    start: (x1, y1),
                    end: (x2, y2),
                }),
            })
        })
        .collect();
    Contour {
        id,
        parts,
        closed: true,
        invalid_reason: None,
        nesting_depth: Some(0),
    }
}

pub fn polygon(points: &[Point]) -> Contour {
    polygon_id(1, points)
}

pub fn polygon_id(id: usize, points: &[Point]) -> Contour {
    let vertices: Vec<(f64, f64, f64)> = points.iter().map(|p| (p.0, p.1, 0.0)).collect();
    bulged(id, &vertices)
}

pub fn rect(id: usize, x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    polygon_id(id, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

pub fn circle(center: Point, radius: f64) -> Contour {
    circle_id(1, center, radius)
}

pub fn circle_id(id: usize, center: Point, radius: f64) -> Contour {
    let mut contour = polygon_id(id, &[]);
    contour.parts = vec![segment(Primitive::Arc(Arc {
        center,
        radius,
        start_angle: 0.0,
        sweep: std::f64::consts::TAU,
    }))];
    contour
}

pub fn bbox(ring: &[Point]) -> (f64, f64, f64, f64) {
    ring.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(a, b, c, d), p| (a.min(p.0), b.min(p.1), c.max(p.0), d.max(p.1)),
    )
}
