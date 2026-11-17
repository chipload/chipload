// SPDX-License-Identifier: GPL-3.0-or-later
//! The area a part's tools sweep (docs/specs/nesting.md §3): every feed move widened by
//! the radius of the tool that made it, a capsule each.

use chipload_core::Point;
use chipload_core::math;
use chipload_core::primitives::{rotate_point, rotation};
use chipload_core::tolerances::ARC_APPROX_TOL_MM;

use crate::model::{Move, MoveKind};
use crate::plan::OpSpan;

/// One feed move swept by its tool: the points within `radius` of the segment `a`–`b`.
/// A plunge, which moves only in Z, has `a == b`: a disc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capsule {
    pub a: Point,
    pub b: Point,
    pub radius: f64,
}

/// What a part cuts in XY: its capsules.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sweep {
    pub capsules: Vec<Capsule>,
}

impl Sweep {
    /// The feed moves of copy `copy` among `spans` (those `keep` takes), each with its
    /// tool's radius. A G2/G3 arc is split into chords within the arc tolerance, each
    /// capsule widened by that tolerance, so the sweep never comes out smaller than the
    /// arc's. Positions carry over from span to span, as the program runs.
    pub fn from_moves(
        moves: &[Move],
        spans: &[OpSpan],
        copy: u32,
        keep: impl Fn(&OpSpan) -> bool,
    ) -> Sweep {
        let mut capsules = Vec::new();
        let mut at: (Option<f64>, Option<f64>) = (None, None);
        for span in spans {
            let wanted = span.copy == copy && keep(span);
            for m in moves.get(span.moves.clone()).unwrap_or(&[]) {
                let from = at;
                at = (m.x.or(at.0), m.y.or(at.1));
                if !wanted || m.kind == MoveKind::Rapid {
                    continue;
                }
                let ((Some(x0), Some(y0)), (Some(x1), Some(y1))) = (from, at) else {
                    continue;
                };
                let (a, b) = ((x0, y0), (x1, y1));
                match (m.kind, m.circle) {
                    (MoveKind::ArcCw | MoveKind::ArcCcw, Some(circle)) => {
                        let ccw = m.kind == MoveKind::ArcCcw;
                        let radius = span.tool_radius + ARC_APPROX_TOL_MM;
                        let points = arc_points(a, b, circle.center, circle.radius, ccw);
                        for pair in points.windows(2) {
                            if let [p, q] = pair {
                                capsules.push(Capsule {
                                    a: *p,
                                    b: *q,
                                    radius,
                                });
                            }
                        }
                    }
                    _ => capsules.push(Capsule {
                        a,
                        b,
                        radius: span.tool_radius,
                    }),
                }
            }
        }
        Sweep { capsules }
    }

    /// The sweep turned by `angle` (radians, counter-clockwise) about `centre`, exact on
    /// quarter turns, as an instance's cut is turned (docs/specs/instances.md §3).
    pub fn turned(&self, angle: f64, centre: Point) -> Sweep {
        let turn = rotation(angle);
        Sweep {
            capsules: self
                .capsules
                .iter()
                .map(|c| Capsule {
                    a: rotate_point(c.a, centre, turn),
                    b: rotate_point(c.b, centre, turn),
                    radius: c.radius,
                })
                .collect(),
        }
    }

    /// The box round everything swept, radii included; None when nothing is.
    pub fn extent(&self) -> Option<(Point, Point)> {
        self.capsules.iter().fold(None, |extent, c| {
            let r = c.radius;
            let low = (c.a.0.min(c.b.0) - r, c.a.1.min(c.b.1) - r);
            let high = (c.a.0.max(c.b.0) + r, c.a.1.max(c.b.1) + r);
            Some(match extent {
                None => (low, high),
                Some(((x0, y0), (x1, y1))) => (
                    (x0.min(low.0), y0.min(low.1)),
                    (x1.max(high.0), y1.max(high.1)),
                ),
            })
        })
    }
}

/// Points along the arc from `a` to `b` round `centre`, chords within the arc tolerance.
fn arc_points(a: Point, b: Point, centre: Point, radius: f64, ccw: bool) -> Vec<Point> {
    let start = math::atan2(a.1 - centre.1, a.0 - centre.0);
    let end = math::atan2(b.1 - centre.1, b.0 - centre.0);
    let tau = std::f64::consts::TAU;
    let mut sweep = if ccw { end - start } else { start - end };
    if sweep <= 0.0 {
        sweep += tau;
    }
    // A chord of angle θ strays r (1 − cos θ/2) from its arc.
    let step = if radius > ARC_APPROX_TOL_MM {
        2.0 * math::acos(1.0 - ARC_APPROX_TOL_MM / radius)
    } else {
        sweep
    };
    let count = (sweep / step.max(f64::MIN_POSITIVE))
        .ceil()
        .clamp(1.0, 4096.0) as usize;
    let mut points = Vec::with_capacity(count + 1);
    points.push(a);
    for k in 1..count {
        let t = sweep * k as f64 / count as f64;
        let angle = if ccw { start + t } else { start - t };
        let (sin, cos) = math::sin_cos(angle);
        points.push((centre.0 + radius * cos, centre.1 + radius * sin));
    }
    points.push(b);
    points
}
