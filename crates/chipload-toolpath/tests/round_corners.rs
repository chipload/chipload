// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounded corners on the clearing rings of a pocket (docs/specs/pocket-round-corners.md).

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, OFFSET_CHECK_SLACK_MM};
use chipload_job::{Direction, PocketStrategy, Spindle};
use chipload_toolpath::ToolpathError;
use chipload_toolpath::geom::segment_distance;
use chipload_toolpath::offset::{contour_pline, pline_segments};
use chipload_toolpath::pocket::{Choices, PocketPlan, plan_pocket_starting};
use common::{polygon_id, rect};
use proptest::prelude::*;

const RADIUS: f64 = 3.0;

fn plan(contours: &[Contour], stepover: f64, round: bool) -> Result<PocketPlan, ToolpathError> {
    let refs: Vec<&Contour> = contours.iter().collect();
    plan_pocket_starting(
        &refs,
        RADIUS,
        0.0,
        stepover,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
        PocketStrategy::Rings {},
        Choices {
            round_corners: round,
            ..Choices::default()
        },
    )
}

/// The sharpest turn between consecutive chords of a closed ring, radians.
fn sharpest_turn(ring: &[Point]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b, c) = (ring[i], ring[(i + 1) % n], ring[(i + 2) % n]);
            let u = (b.0 - a.0, b.1 - a.1);
            let v = (c.0 - b.0, c.1 - b.1);
            let cross = u.0 * v.1 - u.1 * v.0;
            let dot = u.0 * v.0 + u.1 * v.1;
            cross.atan2(dot).abs()
        })
        .fold(0.0, f64::max)
}

fn no_uncut(plan: &PocketPlan) -> bool {
    !plan.notes.iter().any(|n| n.contains("uncut material"))
}

#[test]
fn inner_rings_of_a_rectangle_turn_round_and_the_wall_ring_stays_sharp() {
    let stepover = 2.4;
    let contours = [rect(1, 0.0, 0.0, 100.0, 60.0)];
    let plan = plan(&contours, stepover, true).unwrap();
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
    let piece = &plan.pieces[0];
    let mut inner = 0;
    for (ring, &wall) in piece.rings.iter().zip(&piece.walls) {
        let turn = sharpest_turn(ring);
        // The ring's distance from the walls, and its corner nearest (0, 0).
        let d = ring.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let corner = ring
            .iter()
            .map(|&p| distance(p, (0.0, 0.0)))
            .fold(f64::INFINITY, f64::min);
        if wall {
            assert!(turn > 1.5, "the wall ring keeps its 90° corners: {turn}");
            continue;
        }
        inner += 1;
        // Chords of an arc: small radii are drawn with few chords (22.5° at 0.4 mm).
        assert!(turn < 0.5, "ring {d} mm in turns by {turn} rad");
        // Corner radius min(k · stepover, D), k = rings in from the wall ring, where the
        // ring is wide enough for it (the innermost ones, 1.2 and 6 mm wide, are not).
        let rho = (d - RADIUS).min(2.0 * RADIUS);
        let expected = (d + rho) * std::f64::consts::SQRT_2 - rho;
        let sharp = d * std::f64::consts::SQRT_2;
        if d < 30.0 - 2.0 * stepover - 2.0 * RADIUS {
            assert!(
                (corner - expected).abs() < 0.05,
                "ring {d} mm in: corner {corner} from (0, 0), expected {expected}"
            );
        } else {
            assert!(corner > sharp + 0.1, "ring {d} mm in is rounded a little");
        }
    }
    assert!(inner > 5);
}

#[test]
fn rounding_off_keeps_every_ring_sharp() {
    let contours = [rect(1, 0.0, 0.0, 100.0, 60.0)];
    let plan = plan(&contours, 2.4, false).unwrap();
    for ring in &plan.pieces[0].rings {
        assert!(sharpest_turn(ring) > 1.5);
    }
}

#[test]
fn a_narrow_neck_keeps_every_ring() {
    // Two rooms joined by a neck: rounding must not drop the rings through it.
    let dumbbell = polygon_id(
        1,
        &[
            (0.0, 0.0),
            (40.0, 0.0),
            (40.0, 17.0),
            (60.0, 17.0),
            (60.0, 0.0),
            (100.0, 0.0),
            (100.0, 40.0),
            (60.0, 40.0),
            (60.0, 23.0),
            (40.0, 23.0),
            (40.0, 40.0),
            (0.0, 40.0),
        ],
    );
    let contours = [dumbbell];
    let sharp = plan(&contours, 2.4, false).unwrap();
    let round = plan(&contours, 2.4, true).unwrap();
    assert_eq!(round.pieces[0].rings.len(), sharp.pieces[0].rings.len());
    assert!(no_uncut(&round), "{:?}", round.notes);
}

#[test]
fn acute_corners_round_less_rather_than_leave_material() {
    // A triangle with 34° corners: the full radii would leave material in them, so the rings
    // near it get smaller radii, and the rest keep theirs.
    let triangle = [polygon_id(1, &[(0.0, 0.0), (120.0, 0.0), (60.0, 40.0)])];
    for stepover in [1.8, 2.4, 3.0] {
        let sharp = plan(&triangle, stepover, false).unwrap();
        assert!(no_uncut(&sharp));
        let round = plan(&triangle, stepover, true).unwrap();
        assert!(round.notes.is_empty(), "{stepover}: {:?}", round.notes);
        let piece = &round.pieces[0];
        let inner: Vec<f64> = piece
            .rings
            .iter()
            .zip(&piece.walls)
            .filter(|(_, wall)| !**wall)
            .map(|(ring, _)| sharpest_turn(ring))
            .collect();
        // Sharp, the 34° corners turn by 2.55 rad.
        let rounded = inner.iter().filter(|&&t| t < 1.0).count();
        assert!(2 * rounded >= inner.len(), "{stepover}: {inner:?}");
    }
    // Where the sharp rings already leave material, rounding adds none of its own.
    for stepover in [4.0, 4.8] {
        let sharp = plan(&triangle, stepover, false).unwrap();
        let round = plan(&triangle, stepover, true).unwrap();
        assert_eq!(sharp.notes.len(), 1);
        assert!(round.notes.contains(&sharp.notes[0]), "{:?}", round.notes);
    }
}

#[test]
fn a_large_pocket_with_islands_plans_quickly() {
    let big = [
        rect(1, 0.0, 0.0, 300.0, 200.0),
        polygon_id(2, &[(100.0, 80.0), (140.0, 80.0), (120.0, 120.0)]),
        rect(3, 200.0, 50.0, 230.0, 150.0),
    ];
    let start = std::time::Instant::now();
    let round = plan(&big, 2.4, true).unwrap();
    assert!(round.notes.is_empty(), "{:?}", round.notes);
    // 0.33 s in release; debug builds are much slower.
    if !cfg!(debug_assertions) {
        assert!(start.elapsed().as_secs_f64() < 2.0, "{:?}", start.elapsed());
    }
}

/// Rough pockets with islands, as in tests/pocket.rs.
fn star(n: usize, cx: f64, cy: f64, radii: &[f64]) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            (cx + radii[i] * a.cos(), cy + radii[i] * a.sin())
        })
        .collect()
}

fn pocket() -> impl Strategy<Value = Vec<Contour>> {
    let outer = prop::collection::vec(25.0f64..40.0, 5..12);
    let island = (
        -8.0f64..8.0,
        -8.0f64..8.0,
        prop::collection::vec(2.0f64..4.0, 3..7),
    );
    (outer, prop::option::of(island)).prop_map(|(radii, island)| {
        let mut contours = vec![polygon_id(1, &star(radii.len(), 0.0, 0.0, &radii))];
        if let Some((cx, cy, r)) = island {
            contours.push(polygon_id(2, &star(r.len(), cx, cy, &r)));
        }
        contours
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// Rounded rings never come closer than the tool radius to a wall and leave nothing
    /// uncut that the sharp rings cut.
    #[test]
    fn rounded_rings_never_gouge_and_clear_what_sharp_ones_clear(
        contours in pocket(),
        fraction in 0.2f64..0.5,
    ) {
        let stepover = fraction * 2.0 * RADIUS;
        let Ok(sharp) = plan(&contours, stepover, false) else {
            return Ok(());
        };
        let round = plan(&contours, stepover, true).unwrap();
        let walls: Vec<_> = contours
            .iter()
            .flat_map(|c| pline_segments(&contour_pline(c)))
            .collect();
        let clearance = RADIUS - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM;
        for piece in &round.pieces {
            for ring in &piece.rings {
                let n = ring.len();
                for i in 0..n {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    for wall in &walls {
                        let d = segment_distance(a, b, wall);
                        prop_assert!(d >= clearance, "chord {:?}-{:?} is {} from a wall", a, b, d);
                    }
                }
            }
        }
        if no_uncut(&sharp) {
            prop_assert!(no_uncut(&round), "{:?}", round.notes);
        }
    }
}
