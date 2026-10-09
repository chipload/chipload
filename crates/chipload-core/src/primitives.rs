// SPDX-License-Identifier: GPL-3.0-or-later
//! 2D primitives in millimetres. Angles in radians, counter-clockwise positive.
//! Port of `src/chipload/core/geometry/primitives.py`.

use std::f64::consts::{PI, TAU};

use crate::Point;
use crate::tolerances::ARC_APPROX_TOL_MM;

/// Never approximate a full circle with fewer segments than this, however small.
pub const MIN_SEGMENTS_PER_CIRCLE: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub start: Point,
    pub end: Point,
}

/// Circular arc from `start_angle`, sweeping `sweep` radians (positive = CCW, negative = CW).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    pub center: Point,
    pub radius: f64,
    pub start_angle: f64,
    pub sweep: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Line(Line),
    Arc(Arc),
}

impl Arc {
    pub fn point_at(&self, angle: f64) -> Point {
        (
            self.center.0 + self.radius * angle.cos(),
            self.center.1 + self.radius * angle.sin(),
        )
    }
}

impl Primitive {
    pub fn start(&self) -> Point {
        match self {
            Primitive::Line(l) => l.start,
            Primitive::Arc(a) => a.point_at(a.start_angle),
        }
    }

    pub fn end(&self) -> Point {
        match self {
            Primitive::Line(l) => l.end,
            Primitive::Arc(a) => a.point_at(a.start_angle + a.sweep),
        }
    }

    pub fn length(&self) -> f64 {
        match self {
            Primitive::Line(l) => distance(l.start, l.end),
            Primitive::Arc(a) => a.sweep.abs() * a.radius,
        }
    }

    pub fn midpoint(&self) -> Point {
        match self {
            Primitive::Line(l) => ((l.start.0 + l.end.0) / 2.0, (l.start.1 + l.end.1) / 2.0),
            Primitive::Arc(a) => a.point_at(a.start_angle + a.sweep / 2.0),
        }
    }

    /// Start to end. Arcs become chords within ARC_APPROX_TOL_MM of the arc.
    pub fn points(&self) -> Vec<Point> {
        match self {
            Primitive::Line(l) => vec![l.start, l.end],
            Primitive::Arc(a) => {
                let n = segment_count(a.radius, a.sweep.abs());
                let step = a.sweep / n as f64;
                let mut pts: Vec<Point> = (0..n)
                    .map(|i| a.point_at(a.start_angle + i as f64 * step))
                    .collect();
                pts.push(self.end());
                pts
            }
        }
    }

    /// The same primitive with every coordinate multiplied by `factor` (unit conversion).
    pub fn scaled(&self, factor: f64) -> Primitive {
        let s = |p: Point| (p.0 * factor, p.1 * factor);
        match self {
            Primitive::Line(l) => Primitive::Line(Line {
                start: s(l.start),
                end: s(l.end),
            }),
            Primitive::Arc(a) => Primitive::Arc(Arc {
                center: s(a.center),
                radius: a.radius * factor,
                ..*a
            }),
        }
    }
}

pub fn distance(a: Point, b: Point) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Largest chord angle whose sagitta r*(1 - cos(theta/2)) stays within `tol`.
pub fn max_segment_angle(radius: f64, tol: f64) -> f64 {
    if radius <= tol {
        return PI;
    }
    2.0 * (1.0 - tol / radius).acos()
}

pub fn segment_count(radius: f64, sweep: f64) -> usize {
    let by_tolerance = (sweep / max_segment_angle(radius, ARC_APPROX_TOL_MM)).ceil();
    let by_minimum = (sweep / (TAU / MIN_SEGMENTS_PER_CIRCLE)).ceil();
    // Sweeps are bounded by 2*pi, so these are small non-negative integers.
    by_tolerance.max(by_minimum).max(1.0) as usize
}

/// Shoelace area of an implicitly closed ring. Positive = counter-clockwise.
pub fn signed_area(ring: &[Point]) -> f64 {
    let n = ring.len();
    let twice: f64 = (0..n)
        .map(|i| {
            let (x1, y1) = ring[i];
            let (x2, y2) = ring[(i + 1) % n];
            x1 * y2 - x2 * y1
        })
        .sum();
    twice / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_chords_stay_within_tolerance() {
        let arc = Primitive::Arc(Arc {
            center: (0.0, 0.0),
            radius: 50.0,
            start_angle: 0.0,
            sweep: TAU,
        });
        let pts = arc.points();
        for w in pts.windows(2) {
            let mid = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
            assert!(50.0 - distance(mid, (0.0, 0.0)) <= ARC_APPROX_TOL_MM + 1e-12);
        }
        assert!(distance(pts[0], *pts.last().unwrap()) < 1e-9);
    }

    #[test]
    fn signed_area_orientation() {
        let ccw = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((signed_area(&ccw) - 1.0).abs() < 1e-12);
    }
}
