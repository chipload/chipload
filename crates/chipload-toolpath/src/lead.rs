// SPDX-License-Identifier: GPL-3.0-or-later
//! Lead-in and lead-out (docs/specs/helix-and-leads.md part B): a quarter circle from the
//! waste onto the ring's start, tangent to its first chord, and one off it again, tangent
//! to its last chord. The tool goes down at the lead-in's start, in the waste, so the part
//! keeps no plunge mark.

use std::f64::consts::FRAC_PI_2;

use chipload_core::Point;
use chipload_core::primitives::{distance, segment_count};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, OFFSET_CHECK_SLACK_MM};

use crate::arcs::{ArcTable, Circle, arc_midpoint};
use crate::geom::PrimIndex;

/// The smallest lead radius tried before going without one, in tool diameters.
pub const MIN_LEAD_DIAMETERS: f64 = 0.1;

/// Lead-in (from its start in the waste to the ring's start) and lead-out (from the ring's
/// start to its end in the waste), as points, with their circles.
#[derive(Debug, Clone, PartialEq)]
pub struct Leads {
    pub lead_in: Vec<Point>,
    pub lead_out: Vec<Point>,
    pub in_circle: Circle,
    pub out_circle: Circle,
}

impl Leads {
    /// Record the chords of both arcs, so they are written as G2/G3.
    pub fn tag(&self, arcs: &mut ArcTable) {
        for w in self.lead_in.windows(2) {
            arcs.insert(w[0], w[1], self.in_circle);
        }
        for w in self.lead_out.windows(2) {
            arcs.insert(w[0], w[1], self.out_circle);
        }
    }

    /// Every chord of both arcs, with its circle.
    fn chords(&self) -> impl Iterator<Item = (Point, Point, Circle)> + '_ {
        let lead_in = self
            .lead_in
            .windows(2)
            .map(|w| (w[0], w[1], self.in_circle));
        let lead_out = self
            .lead_out
            .windows(2)
            .map(|w| (w[0], w[1], self.out_circle));
        lead_in.chain(lead_out)
    }

    /// Every chord, and the arc's middle above it, keeps at least `clearance` from every
    /// primitive in `index`.
    pub fn clear_of(&self, index: &PrimIndex, clearance: f64) -> bool {
        let limit = clearance - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM;
        self.chords().all(|(a, b, circle)| {
            let mid = arc_midpoint(a, b, circle);
            index
                .segment_distance_within(a, b, limit)
                .is_none_or(|d| d >= limit)
                && index.distance_within(mid, limit).is_none_or(|d| d >= limit)
        })
    }

    /// Every chord's ends and arc middles satisfy `inside`.
    pub fn all_points(&self, inside: impl Fn(Point) -> bool) -> bool {
        self.chords()
            .all(|(a, b, circle)| inside(a) && inside(b) && inside(arc_midpoint(a, b, circle)))
    }
}

fn unit(a: Point, b: Point) -> Option<Point> {
    let d = distance(a, b);
    (d > 0.0).then(|| ((b.0 - a.0) / d, (b.1 - a.1) / d))
}

/// Points on the circle from angle `from`, sweeping `sweep` (CCW positive), ends included.
fn arc_points(center: Point, radius: f64, from: f64, sweep: f64) -> Vec<Point> {
    let n = segment_count(radius, sweep.abs()).max(2);
    (0..=n)
        .map(|k| {
            let a = from + sweep * k as f64 / n as f64;
            (center.0 + radius * a.cos(), center.1 + radius * a.sin())
        })
        .collect()
}

/// Quarter-circle leads of `radius` at the closed ring's start; the waste is on the left
/// of the cutting direction when `waste_left`. None for a ring of fewer than 2 points.
pub fn quarter_leads(ring: &[Point], radius: f64, waste_left: bool) -> Option<Leads> {
    let n = ring.len();
    if n < 2 {
        return None;
    }
    let p = ring[0];
    let t_out = unit(p, ring[1])?;
    let t_in = unit(ring[n - 1], p)?;
    // The normal towards the waste, and the turning sense that keeps the arcs there.
    let normal = |t: Point| if waste_left { (-t.1, t.0) } else { (t.1, -t.0) };
    let sweep = if waste_left { FRAC_PI_2 } else { -FRAC_PI_2 };
    let (n_out, n_in) = (normal(t_out), normal(t_in));
    let in_center = (p.0 + radius * n_out.0, p.1 + radius * n_out.1);
    let out_center = (p.0 + radius * n_in.0, p.1 + radius * n_in.1);
    // Lead-in: from the point behind the centre (against the cut) round to the start.
    let mut lead_in = arc_points(in_center, radius, (-t_out.1).atan2(-t_out.0), sweep);
    // Lead-out: from the start round to the point ahead of the centre.
    let mut lead_out = arc_points(out_center, radius, (-n_in.1).atan2(-n_in.0), sweep);
    // The ring's start exactly, so the arcs join it without a gap.
    if let Some(last) = lead_in.last_mut() {
        *last = p;
    }
    if let Some(first) = lead_out.first_mut() {
        *first = p;
    }
    Some(Leads {
        lead_in,
        lead_out,
        in_circle: Circle {
            center: in_center,
            radius,
        },
        out_circle: Circle {
            center: out_center,
            radius,
        },
    })
}

/// Where a ring with a lead starts when the user hasn't said: the middle of its longest
/// chord, where a lead has the most room. A corner, the planner's usual start, leaves a
/// lead no room beside an inside corner.
///
/// Chords within CHAIN_TOL_MM of the longest are a tie, as are heights within it: the
/// lowest middle wins, then the leftmost. A flattened arc, or a symmetric drawing, has
/// chords of one length up to the last bits, and those differ between platforms' maths
/// libraries: compared exactly, the start and the G-code followed them (golden
/// v31_svg_badge on Windows, macOS and ARM, 2026-10-29).
pub fn roomiest_start(ring: &[Point]) -> Option<Point> {
    let n = ring.len();
    let middle = |i: usize| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        (distance(a, b), ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0))
    };
    let longest = (0..n)
        .map(|i| middle(i).0)
        .fold(f64::NEG_INFINITY, f64::max);
    let long: Vec<Point> = (0..n)
        .map(middle)
        .filter(|&(length, _)| length >= longest - CHAIN_TOL_MM)
        .map(|(_, p)| p)
        .collect();
    let lowest = long.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    long.into_iter()
        .filter(|p| p.1 <= lowest + CHAIN_TOL_MM)
        .min_by(|p, q| p.0.total_cmp(&q.0))
}

/// The largest leads, from `radius` halving down to `min_radius`, that `fits` accepts;
/// with the radius used. None if none fits.
pub fn fitted_leads(
    ring: &[Point],
    radius: f64,
    min_radius: f64,
    waste_left: bool,
    fits: impl Fn(&Leads) -> bool,
) -> Option<(Leads, f64)> {
    let mut r = radius;
    loop {
        if let Some(leads) = quarter_leads(ring, r, waste_left)
            && fits(&leads)
        {
            return Some((leads, r));
        }
        if r <= min_radius {
            return None;
        }
        r = (r / 2.0).max(min_radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-9
    }

    /// A square cut counter-clockwise from the middle of its bottom edge.
    fn square() -> Vec<Point> {
        vec![
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ]
    }

    /// A regular polygon, its chords one length up to the last bits: the start is the
    /// lowest middle, then the leftmost, whatever the noise (the platforms' maths libraries
    /// differ there), and wherever the ring starts.
    #[test]
    fn the_roomiest_start_ignores_float_noise_between_equal_chords() {
        let polygon = |noise: f64, from: usize| -> Vec<Point> {
            let mut ring: Vec<Point> = (0..8)
                .map(|k| {
                    let a = (k as f64 + 0.5) * std::f64::consts::TAU / 8.0;
                    (10.0 * a.cos(), 10.0 * a.sin())
                })
                .collect();
            ring[3].0 += noise;
            ring.rotate_left(from);
            ring
        };
        let want = roomiest_start(&polygon(0.0, 0)).unwrap();
        assert!(
            close(want, (0.0, -10.0 * (std::f64::consts::PI / 8.0).cos())),
            "{want:?}"
        );
        for noise in [1e-15, -1e-15, 1e-12] {
            for from in 0..8 {
                let got = roomiest_start(&polygon(noise, from)).unwrap();
                assert!(close(got, want), "{noise} {from}: {got:?}");
            }
        }
    }

    #[test]
    fn leads_on_the_left_are_quarter_circles_tangent_at_the_start() {
        let leads = quarter_leads(&square(), 2.0, true).unwrap();
        // Waste on the left of +X is +Y: the centre is above the start.
        assert!(close(leads.in_circle.center, (5.0, 2.0)));
        assert!(close(leads.lead_in[0], (3.0, 2.0)));
        assert!(close(*leads.lead_in.last().unwrap(), (5.0, 0.0)));
        assert!(close(leads.lead_out[0], (5.0, 0.0)));
        assert!(close(*leads.lead_out.last().unwrap(), (7.0, 2.0)));
        // Every point on its circle, all on the waste side.
        for &p in leads.lead_in.iter().chain(&leads.lead_out) {
            assert!((distance(p, (5.0, 2.0)) - 2.0).abs() < 1e-9);
            assert!(p.1 >= -1e-12);
        }
        // Tangent: the last lead-in chord and the first lead-out chord run along +X.
        let a = leads.lead_in[leads.lead_in.len() - 2];
        assert!(a.0 < 5.0 && (5.0 - a.0) > 10.0 * a.1);
        let b = leads.lead_out[1];
        assert!(b.0 > 5.0 && (b.0 - 5.0) > 10.0 * b.1);
    }

    #[test]
    fn leads_on_the_right_mirror_them() {
        let leads = quarter_leads(&square(), 2.0, false).unwrap();
        assert!(close(leads.lead_in[0], (3.0, -2.0)));
        assert!(close(*leads.lead_out.last().unwrap(), (7.0, -2.0)));
    }

    #[test]
    fn at_a_corner_each_lead_is_tangent_to_its_own_chord() {
        // Start at the corner (10, 0): in along +X, out along +Y.
        let ring = vec![(10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)];
        let leads = quarter_leads(&ring, 1.0, false).unwrap();
        // Right of +Y is +X: the lead-in centre; right of +X is -Y: the lead-out centre.
        assert!(close(leads.in_circle.center, (11.0, 0.0)));
        assert!(close(leads.out_circle.center, (10.0, -1.0)));
        assert!(close(leads.lead_in[0], (11.0, -1.0)));
        assert!(close(*leads.lead_out.last().unwrap(), (11.0, -1.0)));
    }

    #[test]
    fn fitting_halves_the_radius_down_to_the_minimum() {
        let fits_below = |limit: f64| move |l: &Leads| l.in_circle.radius <= limit;
        let (_, r) = fitted_leads(&square(), 4.0, 0.5, true, fits_below(1.5)).unwrap();
        assert_eq!(r, 1.0);
        let (_, r) = fitted_leads(&square(), 4.0, 0.6, true, fits_below(0.6)).unwrap();
        assert_eq!(r, 0.6, "the minimum itself is tried");
        assert!(fitted_leads(&square(), 4.0, 0.6, true, fits_below(0.5)).is_none());
    }
}
