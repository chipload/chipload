// SPDX-License-Identifier: GPL-3.0-or-later
//! The shape of a tool's cutting end, turned about its axis (docs/specs/stl-relief.md §6,
//! §8): how high its surface stands above its tip at each distance from the axis. The drop
//! cutter, the simulation and the check of a relief's path all read a tool through this.
//!
//! Every shape here is convex: its height rises ever more steeply (or stays) as the
//! distance grows. The drop cutter relies on that: along any line, the height the tool's
//! tip can reach is a concave function, whose highest point is found among a few
//! candidates.

/// A tool's cutting end. All lengths in mm, angles in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cutter {
    /// A flat end mill: a flat bottom out to its radius.
    Flat { radius: f64 },
    /// A ball nose: a half sphere of its radius.
    Ball { radius: f64 },
    /// A V-bit (or a drill's point): a flat tip of radius `tip`, then a cone rising
    /// `1 / tan(half_angle)` per mm out to `radius`.
    Cone {
        radius: f64,
        tip: f64,
        half_angle: f64,
    },
    /// A tapered ball nose: a ball of radius `ball` at the tip, joined smoothly to a cone
    /// of `half_angle` out to `radius`.
    TaperedBall {
        radius: f64,
        ball: f64,
        half_angle: f64,
    },
    /// An end mill with a corner radius, a bull nose (docs/specs/corner-radius.md §3): a
    /// flat bottom out to `radius − corner`, then a quarter round of radius `corner` up to
    /// the side. `corner` is more than 0 and less than `radius`.
    Bull { radius: f64, corner: f64 },
}

/// Steps of the golden-section search for a bull nose's level point along a line
/// (docs/specs/corner-radius.md §3): each keeps 0.618 of the stretch, so 60 take a tool's
/// width far below a nanometre. A fixed count gives the same answer on every run.
const LEVEL_SEARCH_STEPS: u32 = 60;

impl Cutter {
    /// The tool's outer radius.
    pub fn radius(self) -> f64 {
        match self {
            Cutter::Flat { radius }
            | Cutter::Ball { radius }
            | Cutter::Cone { radius, .. }
            | Cutter::TaperedBall { radius, .. }
            | Cutter::Bull { radius, .. } => radius,
        }
    }

    /// A cutter that holds this one at least `allowance` off whatever it stays clear of,
    /// when its tip is raised by `allowance` (a relief's roughing, docs/specs/stl-relief.md
    /// §14): the flat end mill or ball nose wider by the allowance. A flat end mill grown by
    /// a ball of the allowance fits inside the wider cylinder raised by it, and a ball grown
    /// so is the wider ball; a bull nose grown so is the bull nose wider by the allowance
    /// with a corner larger by it (docs/specs/corner-radius.md §3). None for a V-bit or a
    /// tapered ball nose.
    pub fn grown(self, allowance: f64) -> Option<Cutter> {
        match self {
            Cutter::Flat { radius } => Some(Cutter::Flat {
                radius: radius + allowance,
            }),
            Cutter::Ball { radius } => Some(Cutter::Ball {
                radius: radius + allowance,
            }),
            Cutter::Bull { radius, corner } => Some(Cutter::Bull {
                radius: radius + allowance,
                corner: corner + allowance,
            }),
            Cutter::Cone { .. } | Cutter::TaperedBall { .. } => None,
        }
    }

    /// Where a tapered ball's sphere meets its cone: the radius and height there.
    fn tangent(ball: f64, half_angle: f64) -> (f64, f64) {
        let (sin, cos) = crate::math::sin_cos(half_angle);
        (ball * cos, ball * (1.0 - sin))
    }

    /// The height of the cutting surface above the tip at distance `d` from the axis; None
    /// beyond the tool's radius.
    pub fn height_at(self, d: f64) -> Option<f64> {
        let d = d.abs();
        if d > self.radius() {
            return None;
        }
        Some(match self {
            Cutter::Flat { .. } => 0.0,
            Cutter::Ball { radius } => radius - (radius * radius - d * d).max(0.0).sqrt(),
            Cutter::Cone {
                tip, half_angle, ..
            } => (d - tip).max(0.0) / crate::math::tan(half_angle),
            Cutter::TaperedBall {
                ball, half_angle, ..
            } => {
                let (rt, ht) = Cutter::tangent(ball, half_angle);
                if d <= rt {
                    ball - (ball * ball - d * d).max(0.0).sqrt()
                } else {
                    ht + (d - rt) / crate::math::tan(half_angle)
                }
            }
            Cutter::Bull { radius, corner } => {
                let out = (d - (radius - corner)).max(0.0);
                corner - (corner * corner - out * out).max(0.0).sqrt()
            }
        })
    }

    /// The radii where the surface's slope jumps: a flat tip's edge, a ball's joint with its
    /// cone. The drop cutter tries them as candidates; a bull nose's flat edge too, where
    /// the slope doesn't jump but the shape changes.
    pub fn kinks(self) -> Vec<f64> {
        match self {
            Cutter::Bull { radius, corner } => vec![radius - corner],
            Cutter::Flat { .. } | Cutter::Ball { .. } => Vec::new(),
            Cutter::Cone { tip, .. } => vec![tip],
            Cutter::TaperedBall {
                ball, half_angle, ..
            } => vec![Cutter::tangent(ball, half_angle).0],
        }
    }

    /// Distances from the axis where the surface's slope is `slope` (dh/dd ≥ 0), one per
    /// smooth piece of the shape that has that slope inside it: where a plane of that slope
    /// touches the tool.
    pub fn where_slope(self, slope: f64) -> Vec<f64> {
        let on_sphere = |r: f64| r * slope / (1.0 + slope * slope).sqrt();
        match self {
            Cutter::Flat { .. } | Cutter::Cone { .. } => Vec::new(),
            Cutter::Ball { radius } => vec![on_sphere(radius)],
            // On the corner's quarter round, a circle of radius `corner` in section.
            Cutter::Bull { radius, corner } => vec![radius - corner + on_sphere(corner)],
            Cutter::TaperedBall {
                ball, half_angle, ..
            } => {
                let d = on_sphere(ball);
                if d <= Cutter::tangent(ball, half_angle).0 {
                    vec![d]
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Positions `t` along a line in the tool's plan, measured from the foot of the
    /// perpendicular from the axis (`p` away), where the tip's reach `m·t − h(√(p² + t²))`
    /// is level: the stationary points of each smooth piece, for a line rising `m` per mm.
    pub fn level_along(self, m: f64, p: f64) -> Vec<f64> {
        // On a sphere of radius r: m·t + √(r² − p² − t²) is level at t = m·ρ/√(1 + m²).
        let sphere = |r: f64| -> Option<f64> {
            let rho2 = r * r - p * p;
            (rho2 > 0.0).then(|| m * rho2.sqrt() / (1.0 + m * m).sqrt())
        };
        // On a cone rising k per mm: m·t − k·√(p² + t²) is level at t = m·p/√(k² − m²).
        let cone = |k: f64| -> Option<f64> {
            let q = k * k - m * m;
            (q > 0.0).then(|| m * p / q.sqrt())
        };
        match self {
            Cutter::Flat { .. } => Vec::new(),
            Cutter::Ball { radius } => sphere(radius).into_iter().collect(),
            Cutter::Cone { half_angle, .. } => cone(1.0 / crate::math::tan(half_angle))
                .into_iter()
                .collect(),
            Cutter::TaperedBall {
                ball, half_angle, ..
            } => sphere(ball)
                .into_iter()
                .chain(cone(1.0 / crate::math::tan(half_angle)))
                .collect(),
            Cutter::Bull { radius, .. } => self.highest_along(m, p, radius).into_iter().collect(),
        }
    }

    /// Where the reach `m·t − h(√(p² + t²))` is highest along a line `p` from the axis, for
    /// `t` within the tool (docs/specs/corner-radius.md §3): no closed form on a bull nose's
    /// corner, so a golden-section search, which the reach's concavity makes exact (the
    /// shape is convex). None when the line misses the tool.
    fn highest_along(self, m: f64, p: f64, radius: f64) -> Option<f64> {
        let half = (radius * radius - p * p).max(0.0).sqrt();
        if half <= 0.0 {
            return None;
        }
        // A level line: the reach is even about the foot, so highest there.
        if m == 0.0 {
            return Some(0.0);
        }
        let reach = |t: f64| {
            let d = (p * p + t * t).sqrt().min(radius);
            m * t - self.height_at(d).unwrap_or(0.0)
        };
        let ratio = (5f64.sqrt() - 1.0) / 2.0;
        let (mut lo, mut hi) = (-half, half);
        let mut a = hi - ratio * (hi - lo);
        let mut b = lo + ratio * (hi - lo);
        let (mut fa, mut fb) = (reach(a), reach(b));
        for _ in 0..LEVEL_SEARCH_STEPS {
            if fa < fb {
                lo = a;
                a = b;
                fa = fb;
                b = lo + ratio * (hi - lo);
                fb = reach(b);
            } else {
                hi = b;
                b = a;
                fb = fa;
                a = hi - ratio * (hi - lo);
                fa = reach(a);
            }
        }
        Some((lo + hi) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tapered() -> Cutter {
        Cutter::TaperedBall {
            radius: 3.175,
            ball: 0.4,
            half_angle: 5f64.to_radians(),
        }
    }

    fn bull() -> Cutter {
        Cutter::Bull {
            radius: 3.0,
            corner: 1.0,
        }
    }

    #[test]
    fn a_bull_nose_is_flat_then_round() {
        let b = bull();
        assert_eq!(b.height_at(0.0), Some(0.0));
        assert_eq!(b.height_at(2.0), Some(0.0));
        assert!((b.height_at(3.0).unwrap() - 1.0).abs() < 1e-12);
        // Halfway round the corner: 1 − √(1 − 0.5²).
        let h = b.height_at(2.5).unwrap();
        assert!((h - (1.0 - 0.75f64.sqrt())).abs() < 1e-12, "{h}");
        assert_eq!(b.height_at(3.01), None);
        // Grown by 0.5: wider and rounder, the same flat.
        assert_eq!(
            b.grown(0.5),
            Some(Cutter::Bull {
                radius: 3.5,
                corner: 1.5
            })
        );
    }

    #[test]
    fn a_bull_nose_near_half_its_width_is_a_ball() {
        let ball = Cutter::Ball { radius: 3.0 };
        let almost = Cutter::Bull {
            radius: 3.0,
            corner: 3.0 - 1e-9,
        };
        for k in 0..=30 {
            let d = 0.1 * f64::from(k);
            let (a, b) = (almost.height_at(d).unwrap(), ball.height_at(d).unwrap());
            assert!((a - b).abs() < 1e-4, "{d}: {a} {b}");
        }
    }

    #[test]
    fn a_plane_touches_a_bull_nose_where_its_slope_matches() {
        let b = bull();
        for slope in [0.2, 1.0, 4.0] {
            let d = b.where_slope(slope)[0];
            let e = 1e-7;
            let found = (b.height_at(d + e).unwrap() - b.height_at(d - e).unwrap()) / (2.0 * e);
            assert!((found - slope).abs() < 1e-4, "{slope}: {found}");
        }
    }

    #[test]
    fn a_bull_noses_level_point_is_the_highest_reach() {
        let b = bull();
        for (m, p) in [(0.3, 0.0), (0.8, 1.5), (-0.5, 2.2), (2.0, 2.9), (0.0, 1.0)] {
            let t = b.level_along(m, p)[0];
            let reach = |t: f64| m * t - b.height_at((p * p + t * t).sqrt().min(3.0)).unwrap();
            let half = (9.0f64 - p * p).sqrt();
            let best = (0..=20_000)
                .map(|k| -half + 2.0 * half * f64::from(k) / 20_000.0)
                .map(reach)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(reach(t) >= best - 1e-6, "{m} {p}: {} < {best}", reach(t));
        }
        assert!(b.level_along(1.0, 3.5).is_empty());
    }

    #[test]
    fn heights_follow_each_shape() {
        assert_eq!(Cutter::Flat { radius: 3.0 }.height_at(2.9), Some(0.0));
        assert_eq!(Cutter::Flat { radius: 3.0 }.height_at(3.1), None);
        let ball = Cutter::Ball { radius: 2.0 };
        assert_eq!(ball.height_at(0.0), Some(0.0));
        assert!((ball.height_at(2.0).unwrap() - 2.0).abs() < 1e-12);
        let v90 = Cutter::Cone {
            radius: 6.0,
            tip: 0.0,
            half_angle: 45f64.to_radians(),
        };
        assert!((v90.height_at(3.0).unwrap() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn a_tapered_ball_is_smooth_where_ball_meets_cone() {
        let t = tapered();
        let rt = t.kinks()[0];
        let e = 1e-7;
        let below = (t.height_at(rt).unwrap() - t.height_at(rt - e).unwrap()) / e;
        let above = (t.height_at(rt + e).unwrap() - t.height_at(rt).unwrap()) / e;
        assert!((below - above).abs() < 1e-3, "{below} {above}");
        // The cone's slope is 1/tan(5°).
        assert!((above - 1.0 / crate::math::tan(5f64.to_radians())).abs() < 1e-3);
    }

    #[test]
    fn each_shape_is_convex() {
        for c in [
            Cutter::Flat { radius: 3.0 },
            Cutter::Ball { radius: 3.0 },
            Cutter::Cone {
                radius: 3.0,
                tip: 0.5,
                half_angle: 30f64.to_radians(),
            },
            tapered(),
            bull(),
        ] {
            let r = c.radius();
            let n = 300;
            let h = |k: usize| c.height_at(r * k as f64 / n as f64).unwrap();
            for k in 1..n {
                assert!(h(k - 1) + h(k + 1) >= 2.0 * h(k) - 1e-9, "{c:?} at {k}");
            }
        }
    }

    #[test]
    fn a_plane_touches_a_ball_where_its_slope_matches() {
        let ball = Cutter::Ball { radius: 2.0 };
        let d = ball.where_slope(1.0)[0];
        let e = 1e-7;
        let slope = (ball.height_at(d + e).unwrap() - ball.height_at(d - e).unwrap()) / (2.0 * e);
        assert!((slope - 1.0).abs() < 1e-5, "{slope}");
    }
}
