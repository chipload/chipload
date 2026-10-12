// SPDX-License-Identifier: GPL-3.0-or-later
//! 2D primitives in millimetres. Angles in radians, counter-clockwise positive.

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
    /// The arc a bulged polyline segment describes: it sweeps 4·atan(bulge) from `a` to `b`
    /// (positive = counter-clockwise). None for a straight segment or a zero-length chord.
    pub fn from_bulge(a: Point, b: Point, bulge: f64) -> Option<Arc> {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let chord = dx.hypot(dy);
        if bulge == 0.0 || chord == 0.0 {
            return None;
        }
        let sweep = 4.0 * bulge.atan();
        let half = sweep / 2.0;
        // Centre: from the chord midpoint along the left normal, signed by the sweep.
        let offset = chord / 2.0 / half.tan();
        let center = (
            (a.0 + b.0) / 2.0 - dy / chord * offset,
            (a.1 + b.1) / 2.0 + dx / chord * offset,
        );
        Some(Arc {
            center,
            radius: chord / 2.0 / half.sin().abs(),
            start_angle: (a.1 - center.1).atan2(a.0 - center.0),
            sweep,
        })
    }

    /// Bulge of this arc as one polyline segment, tan(sweep / 4). Only meaningful for
    /// |sweep| < 2π; split longer arcs first.
    pub fn bulge(&self) -> f64 {
        (self.sweep / 4.0).tan()
    }

    pub fn point_at(&self, angle: f64) -> Point {
        (
            self.center.0 + self.radius * angle.cos(),
            self.center.1 + self.radius * angle.sin(),
        )
    }
}

impl Primitive {
    /// The same primitive moved by `(dx, dy)`.
    pub fn translated(&self, (dx, dy): Point) -> Primitive {
        match *self {
            Primitive::Line(l) => Primitive::Line(Line {
                start: (l.start.0 + dx, l.start.1 + dy),
                end: (l.end.0 + dx, l.end.1 + dy),
            }),
            Primitive::Arc(a) => Primitive::Arc(Arc {
                center: (a.center.0 + dx, a.center.1 + dy),
                ..a
            }),
        }
    }

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

    /// The exact box round the primitive: (min corner, max corner). An arc's box holds its
    /// ends and each of its circle's four extreme points that it passes.
    pub fn bounds(&self) -> (Point, Point) {
        let mut points = vec![self.start(), self.end()];
        if let Primitive::Arc(a) = self {
            let (cx, cy, r) = (a.center.0, a.center.1, a.radius);
            let extremes = [(cx + r, cy), (cx, cy + r), (cx - r, cy), (cx, cy - r)];
            for (k, extreme) in extremes.into_iter().enumerate() {
                let angle = k as f64 * PI / 2.0;
                let along = if a.sweep >= 0.0 {
                    (angle - a.start_angle).rem_euclid(TAU)
                } else {
                    (a.start_angle - angle).rem_euclid(TAU)
                };
                if along <= a.sweep.abs() {
                    points.push(extreme);
                }
            }
        }
        let first = points[0];
        points.iter().fold((first, first), |(lo, hi), p| {
            (
                (lo.0.min(p.0), lo.1.min(p.1)),
                (hi.0.max(p.0), hi.1.max(p.1)),
            )
        })
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
    #[test]
    fn bounds_are_exact_for_arcs() {
        let circle = Primitive::Arc(Arc {
            center: (10.0, 5.0),
            radius: 3.0,
            start_angle: 0.3,
            sweep: TAU,
        });
        assert_eq!(circle.bounds(), ((7.0, 2.0), (13.0, 8.0)));
        // A quarter from 45° to 135° passes only the top.
        let top = Primitive::Arc(Arc {
            center: (0.0, 0.0),
            radius: 2.0,
            start_angle: PI / 4.0,
            sweep: PI / 2.0,
        });
        let ((x0, y0), (x1, y1)) = top.bounds();
        assert!((x0 + 2f64.sqrt()).abs() < 1e-12 && (x1 - 2f64.sqrt()).abs() < 1e-12);
        assert!((y0 - 2f64.sqrt()).abs() < 1e-12 && y1 == 2.0);
        // The same clockwise from 135° to 45°.
        let cw = Primitive::Arc(Arc {
            center: (0.0, 0.0),
            radius: 2.0,
            start_angle: 3.0 * PI / 4.0,
            sweep: -PI / 2.0,
        });
        assert_eq!(cw.bounds().1.1, 2.0);
        // Clockwise the other way round passes the bottom, not the top.
        let under = Primitive::Arc(Arc {
            center: (0.0, 0.0),
            radius: 2.0,
            start_angle: PI / 4.0,
            sweep: -3.0 * PI / 2.0,
        });
        assert_eq!(under.bounds().0.1, -2.0);
        assert!(under.bounds().1.1 < 2.0);
        let line = Primitive::Line(Line {
            start: (3.0, -1.0),
            end: (1.0, 4.0),
        });
        assert_eq!(line.bounds(), ((1.0, -1.0), (3.0, 4.0)));
    }

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
