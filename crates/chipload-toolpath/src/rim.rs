// SPDX-License-Identifier: GPL-3.0-or-later
//! A relief's area and its rim (docs/specs/stl-relief.md §16): where the tool's centre goes,
//! either closed loops as selected or filled loops grown by a distance, and the ramp from
//! the rim's flat up to the stock top, as a surface the tool is lowered onto.
//!
//! Everything is measured by the distance to the loops' inside (0 within them), which
//! changes by at most the distance moved: so a raster line meets the grown area in the
//! union of the loops' insides and their edges' capsules, and the ramp under a tool is
//! bounded by its height at the axis plus the tool's radius times its slope.

use chipload_core::Point;
use chipload_core::cutter::Cutter;
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::relief::{encloses, inside_loops_link, loop_spans};

/// A link between neighbouring lines is tested at this many points inside it.
const LINK_SAMPLES: usize = 8;

/// Where a relief's tool centre goes.
#[derive(Debug, Clone, PartialEq)]
pub enum Area {
    /// Closed loops, holes by the even-odd rule.
    Loops(Vec<Vec<Point>>),
    /// Within `reach` of the inside of filled `loops`.
    Grown { loops: Vec<Vec<Point>>, reach: f64 },
}

impl Area {
    /// The area with every point mapped by `f`, a turn or a move (distances kept).
    pub fn mapped(&self, f: impl Fn(Point) -> Point) -> Area {
        let map = |loops: &[Vec<Point>]| -> Vec<Vec<Point>> {
            loops
                .iter()
                .map(|l| l.iter().map(|p| f(*p)).collect())
                .collect()
        };
        match self {
            Area::Loops(loops) => Area::Loops(map(loops)),
            Area::Grown { loops, reach } => Area::Grown {
                loops: map(loops),
                reach: *reach,
            },
        }
    }

    /// The lowest and highest Y the area reaches.
    pub fn y_range(&self) -> Option<(f64, f64)> {
        let (loops, grow) = match self {
            Area::Loops(loops) => (loops, 0.0),
            Area::Grown { loops, reach } => (loops, *reach),
        };
        let (lo, hi) = loops
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.1), hi.max(p.1))
            });
        (lo <= hi).then_some((lo - grow, hi + grow))
    }

    /// Its box in plan: lower-left and upper-right corners.
    pub fn bounds(&self) -> Option<(Point, Point)> {
        let (loops, grow) = match self {
            Area::Loops(loops) => (loops, 0.0),
            Area::Grown { loops, reach } => (loops, *reach),
        };
        let mut points = loops.iter().flatten();
        let first = *points.next()?;
        let (lo, hi) = points.fold((first, first), |(lo, hi), p| {
            (
                (lo.0.min(p.0), lo.1.min(p.1)),
                (hi.0.max(p.0), hi.1.max(p.1)),
            )
        });
        Some(((lo.0 - grow, lo.1 - grow), (hi.0 + grow, hi.1 + grow)))
    }

    /// Where the line at Y `v` lies inside the area, as (x0, x1) from low to high X,
    /// stretches shorter than the chaining tolerance left out.
    pub fn spans(&self, v: f64) -> Vec<(f64, f64)> {
        match self {
            Area::Loops(loops) => loop_spans(loops, v),
            Area::Grown { loops, reach } => grown_spans(loops, *reach, v),
        }
    }

    /// Whether the straight move from `a` to `b` stays inside the area.
    pub fn link_inside(&self, a: Point, b: Point) -> bool {
        match self {
            Area::Loops(loops) => inside_loops_link(loops, a, b),
            Area::Grown { loops, reach } => (1..LINK_SAMPLES + 1).all(|k| {
                let t = k as f64 / (LINK_SAMPLES + 1) as f64;
                let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                distance(loops, p) <= reach + CHAIN_TOL_MM
            }),
        }
    }

    /// The area shrunk by `inset` (a roughing tool's): None when nothing is left of a grown
    /// one. Only for a grown area; loops are offset by the planner.
    pub fn shrunk(&self, inset: f64) -> Option<Area> {
        match self {
            Area::Loops(_) => None,
            Area::Grown { loops, reach } => (reach - inset > CHAIN_TOL_MM).then(|| Area::Grown {
                loops: loops.clone(),
                reach: reach - inset,
            }),
        }
    }
}

/// The distance from `p` to the inside of the filled `loops`: 0 within any of them.
pub fn distance(loops: &[Vec<Point>], p: Point) -> f64 {
    if loops.iter().any(|l| encloses(l, p)) {
        return 0.0;
    }
    loops
        .iter()
        .flat_map(|l| (0..l.len()).map(move |k| (l[k], l[(k + 1) % l.len()])))
        .map(|(a, b)| segment_distance(a, b, p))
        .fold(f64::INFINITY, f64::min)
}

/// [`distance`], negative inside the loops: there minus the distance to the nearest edge of
/// any loop. Changes by at most the distance moved, and is never below the true signed
/// distance to the loops' union (an edge inside another loop only brings it nearer 0), so
/// the ramp bounded by it is never too low.
pub fn signed_distance(loops: &[Vec<Point>], p: Point) -> f64 {
    let nearest = loops
        .iter()
        .flat_map(|l| (0..l.len()).map(move |k| (l[k], l[(k + 1) % l.len()])))
        .map(|(a, b)| segment_distance(a, b, p))
        .fold(f64::INFINITY, f64::min);
    if loops.iter().any(|l| encloses(l, p)) {
        -nearest
    } else {
        nearest
    }
}

fn segment_distance(a: Point, b: Point, p: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length2 = dx * dx + dy * dy;
    let t = if length2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a.0 + t * dx - p.0).hypot(a.1 + t * dy - p.1)
}

/// Where the line at Y `v` lies within `reach` of the filled `loops`: each loop's own inside
/// and each edge's capsule, merged.
fn grown_spans(loops: &[Vec<Point>], reach: f64, v: f64) -> Vec<(f64, f64)> {
    let mut pieces: Vec<(f64, f64)> = Vec::new();
    for l in loops {
        pieces.extend(loop_spans(std::slice::from_ref(l), v));
        for k in 0..l.len() {
            if let Some(piece) = capsule_span(l[k], l[(k + 1) % l.len()], reach, v) {
                pieces.push(piece);
            }
        }
    }
    pieces.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (lo, hi) in pieces {
        match merged.last_mut() {
            Some(last) if lo <= last.1 => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    merged.retain(|(a, b)| b - a > CHAIN_TOL_MM);
    merged
}

/// Where the line at Y `v` lies within `r` of the segment `ab`: one stretch (the capsule is
/// convex), from its two end discs and the band beside it.
fn capsule_span(a: Point, b: Point, r: f64, v: f64) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for c in [a, b] {
        let dy = v - c.1;
        if dy.abs() <= r {
            let w = (r * r - dy * dy).sqrt();
            (lo, hi) = (lo.min(c.0 - w), hi.max(c.0 + w));
        }
    }
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length2 = dx * dx + dy * dy;
    if length2 > 0.0 {
        let length = length2.sqrt();
        // Along the segment, 0 to 1: ((x − ax)·dx + (v − ay)·dy) / length².
        let along = linear_range(
            dx / length2,
            (-a.0 * dx + (v - a.1) * dy) / length2,
            0.0,
            1.0,
        );
        // Across it, −r to r: ((x − ax)·(−dy) + (v − ay)·dx) / length.
        let across = linear_range(-dy / length, (a.0 * dy + (v - a.1) * dx) / length, -r, r);
        if let (Some(p), Some(q)) = (along, across) {
            let (l, h) = (p.0.max(q.0), p.1.min(q.1));
            if l <= h {
                (lo, hi) = (lo.min(l), hi.max(h));
            }
        }
    }
    (lo <= hi).then_some((lo, hi))
}

/// The X where `lo ≤ k·x + c ≤ hi`: every X, none, or a stretch.
fn linear_range(k: f64, c: f64, lo: f64, hi: f64) -> Option<(f64, f64)> {
    if k == 0.0 {
        return (lo..=hi)
            .contains(&c)
            .then_some((f64::NEG_INFINITY, f64::INFINITY));
    }
    let (p, q) = ((lo - c) / k, (hi - c) / k);
    Some((p.min(q), p.max(q)))
}

/// A relief's ramp (docs/specs/stl-relief.md §16): flat at `floor` within `foot` of the
/// filled `loops`, rising at `tan` per mm beyond, up to `top`.
#[derive(Debug, Clone, PartialEq)]
pub struct Ramp {
    pub loops: Vec<Vec<Point>>,
    /// The flat's edge, mm from the loops' inside.
    pub foot: f64,
    /// The slope, mm up per mm out.
    pub tan: f64,
    /// The flat's Z and the stock top's (program Z).
    pub floor: f64,
    pub top: f64,
}

impl Ramp {
    /// How far out from the loops' inside the ramp reaches the top.
    pub fn reach(&self) -> f64 {
        self.foot + (self.top - self.floor).max(0.0) / self.tan
    }

    /// The ramp's surface over `p`, exactly.
    pub fn surface(&self, p: Point) -> f64 {
        let out = (signed_distance(&self.loops, p) - self.foot).max(0.0);
        (self.floor + out * self.tan).min(self.top)
    }

    /// The highest the tip of `cutter` must stay over `p` so that it cuts nowhere into the
    /// ramp: within `ρ` of `p` the surface lies at most `min(top, floor + (s + ρ)·tan)`, `s`
    /// the signed distance beyond the foot at `p` (negative inside the loops), and the tip at that minus the tool's height
    /// there, highest among `ρ` = 0, the radius, the tool's kinks, where its slope is the
    /// ramp's, and where the bound reaches the top (the function is concave); never below
    /// the flat.
    pub fn drop(&self, cutter: Cutter, p: Point) -> f64 {
        // Signed: inside the loops a point lies that far before the foot (§16).
        let s = signed_distance(&self.loops, p) - self.foot;
        let r = cutter.radius();
        let mut candidates = vec![0.0, r, (self.top - self.floor) / self.tan - s];
        candidates.extend(cutter.kinks());
        candidates.extend(cutter.where_slope(self.tan));
        candidates
            .into_iter()
            .filter(|rho| (0.0..=r).contains(rho))
            .filter_map(|rho| {
                let h = cutter.height_at(rho)?;
                Some((self.floor + (s + rho) * self.tan).min(self.top) - h)
            })
            .fold(self.floor, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(half: f64) -> Vec<Vec<Point>> {
        vec![vec![
            (-half, -half),
            (half, -half),
            (half, half),
            (-half, half),
        ]]
    }

    #[test]
    fn a_grown_square_spans_its_rounded_outline() {
        let area = Area::Grown {
            loops: square(10.0),
            reach: 5.0,
        };
        // Through the middle: the square and 5 mm each side.
        assert_eq!(area.spans(0.0), vec![(-15.0, 15.0)]);
        // Through a rounded corner: 4 mm above the square, √(25 − 16) = 3 beyond its sides.
        let s = area.spans(14.0);
        assert_eq!(s.len(), 1);
        assert!(
            (s[0].0 + 13.0).abs() < 1e-9 && (s[0].1 - 13.0).abs() < 1e-9,
            "{s:?}"
        );
        assert!(area.spans(15.5).is_empty());
        assert_eq!(area.bounds(), Some(((-15.0, -15.0), (15.0, 15.0))));
    }

    #[test]
    fn two_loops_grown_together_merge_and_holes_are_filled() {
        let loops = vec![
            vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
            vec![(6.0, 0.0), (10.0, 0.0), (10.0, 4.0), (6.0, 4.0)],
        ];
        let area = Area::Grown { loops, reach: 1.5 };
        assert_eq!(area.spans(2.0), vec![(-1.5, 11.5)]);
        assert!(area.link_inside((1.0, 2.0), (9.0, 2.0)));
        assert!(!area.link_inside((1.0, 2.0), (1.0, 8.0)));
        assert_eq!(area.shrunk(2.0), None);
    }

    #[test]
    fn the_distance_is_zero_inside_and_to_the_nearest_edge_outside() {
        let loops = square(10.0);
        assert_eq!(distance(&loops, (3.0, -2.0)), 0.0);
        assert!((distance(&loops, (13.0, 0.0)) - 3.0).abs() < 1e-12);
        assert!((distance(&loops, (13.0, 14.0)) - 5.0).abs() < 1e-12);
    }

    /// A 45° ramp from a flat 5 mm round a square, 10 mm deep.
    fn ramp() -> Ramp {
        Ramp {
            loops: square(10.0),
            foot: 5.0,
            tan: 1.0,
            floor: -10.0,
            top: 0.0,
        }
    }

    #[test]
    fn the_ramp_rises_from_its_foot_to_the_top() {
        let ramp = ramp();
        assert_eq!(ramp.reach(), 15.0);
        assert_eq!(ramp.surface((0.0, 0.0)), -10.0);
        assert_eq!(ramp.surface((15.0, 0.0)), -10.0);
        assert!((ramp.surface((19.0, 0.0)) + 6.0).abs() < 1e-12);
        assert_eq!(ramp.surface((40.0, 0.0)), 0.0);
    }

    #[test]
    fn a_ball_rests_on_the_ramp_without_cutting_into_it() {
        let ramp = ramp();
        let ball = Cutter::Ball { radius: 1.0 };
        // On the flat, far from the foot: the floor.
        assert!((ramp.drop(ball, (5.0, 0.0)) + 10.0).abs() < 1e-12);
        // Halfway up a straight stretch: on a 45° slope a ball's tip stands
        // r·(1/cos 45° − 1) above the surface at its axis.
        let rest = 2f64.sqrt() - 1.0;
        let z = ramp.drop(ball, (22.5, 0.0));
        assert!((z - (ramp.surface((22.5, 0.0)) + rest)).abs() < 1e-9, "{z}");
        // Beyond the top: on the stock top.
        assert!((ramp.drop(ball, (40.0, 0.0))).abs() < 1e-12);
        // Never into the ramp: every point under the tool, at every position on a line.
        for k in 0..400 {
            let p = (12.0 + k as f64 * 0.05, 11.0);
            let tip = ramp.drop(ball, p);
            for ring in 0..=8 {
                let rho = ring as f64 / 8.0;
                for j in 0..32 {
                    let a = std::f64::consts::TAU * j as f64 / 32.0;
                    let q = (p.0 + rho * a.cos(), p.1 + rho * a.sin());
                    let h = ball.height_at(rho).unwrap();
                    assert!(tip + h >= ramp.surface(q) - 1e-9, "{p:?} {q:?}");
                }
            }
        }
    }

    /// A box rim with sharp corners has its foot on the loop itself: inside, the tool
    /// reaches the floor once its radius is clear of the edge (found by the user: the ramp
    /// was taken to start everywhere inside, holding the tool up).
    #[test]
    fn inside_a_foot_on_the_loop_the_tool_reaches_the_floor() {
        let ramp = Ramp {
            foot: 0.0,
            ..ramp()
        };
        let ball = Cutter::Ball { radius: 1.0 };
        let flat = Cutter::Flat { radius: 3.1 };
        assert_eq!(ramp.drop(ball, (0.0, 0.0)), -10.0);
        assert_eq!(ramp.drop(flat, (6.0, 0.0)), -10.0);
        // Nearer the edge than its radius, it rides up the ramp, never into it.
        let z = ramp.drop(flat, (8.0, 0.0));
        assert!((z - (-10.0 + 1.1)).abs() < 1e-9, "{z}");
        assert_eq!(signed_distance(&ramp.loops, (6.0, 0.0)), -4.0);
    }

    #[test]
    fn a_straight_wall_keeps_the_tool_its_radius_inside() {
        let wall = Ramp {
            tan: std::f64::consts::FRAC_PI_2.tan(),
            ..ramp()
        };
        let flat = Cutter::Flat { radius: 1.0 };
        assert!((wall.drop(flat, (13.9, 0.0)) + 10.0).abs() < 1e-9);
        assert!(wall.drop(flat, (14.1, 0.0)) > -1e-6);
    }
}
