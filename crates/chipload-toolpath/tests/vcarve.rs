// SPDX-License-Identifier: GPL-3.0-or-later
//! V-carving (docs/specs/v-carve.md). The checks here are written separately from the
//! planner: they measure the cone's circle at the surface against the drawn outline.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::Primitive;
use chipload_toolpath::vcarve::{CarvePath, VBit, VcarvePlan, plan_vcarve};
use proptest::prelude::*;

/// How far the checks allow the cut to stray: the spec's tolerance, plus the gap between
/// the samples these checks take.
const TOL: f64 = 0.01;

fn bit(degrees: f64, tip: f64) -> VBit {
    VBit {
        angle: degrees.to_radians(),
        tip_diameter: tip,
        diameter: 50.0,
    }
}

fn carve(contours: &[Contour], bit: &VBit, max_depth: f64) -> VcarvePlan {
    let refs: Vec<&Contour> = contours.iter().collect();
    plan_vcarve(&refs, bit, max_depth, (0.0, 0.0), None).unwrap()
}

// -- an independent geometry, for the checks

fn dist_to_prim(prim: &Primitive, p: Point) -> f64 {
    match prim {
        Primitive::Line(l) => {
            let (ax, ay) = l.start;
            let (bx, by) = l.end;
            let (dx, dy) = (bx - ax, by - ay);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((p.0 - ax) * dx + (p.1 - ay) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            ((ax + t * dx - p.0).powi(2) + (ay + t * dy - p.1).powi(2)).sqrt()
        }
        Primitive::Arc(a) => {
            let angle = (p.1 - a.center.1).atan2(p.0 - a.center.0);
            let from_start = if a.sweep >= 0.0 {
                (angle - a.start_angle).rem_euclid(std::f64::consts::TAU)
            } else {
                (a.start_angle - angle).rem_euclid(std::f64::consts::TAU)
            };
            let to_circle =
                (((p.0 - a.center.0).powi(2) + (p.1 - a.center.1).powi(2)).sqrt() - a.radius).abs();
            if from_start <= a.sweep.abs() {
                to_circle
            } else {
                let end = a.start_angle + a.sweep;
                let ends = [
                    (
                        a.center.0 + a.radius * a.start_angle.cos(),
                        a.center.1 + a.radius * a.start_angle.sin(),
                    ),
                    (
                        a.center.0 + a.radius * end.cos(),
                        a.center.1 + a.radius * end.sin(),
                    ),
                ];
                ends.iter()
                    .map(|e| ((e.0 - p.0).powi(2) + (e.1 - p.1).powi(2)).sqrt())
                    .fold(f64::INFINITY, f64::min)
            }
        }
    }
}

fn dist_to_outline(contours: &[Contour], p: Point) -> f64 {
    contours
        .iter()
        .flat_map(|c| c.parts.iter().map(|part| part.segment.prim))
        .map(|prim| dist_to_prim(&prim, p))
        .fold(f64::INFINITY, f64::min)
}

/// The contour as a ring with arcs split far finer than the import's chords (sag at most
/// a thousandth of TOL): a point between a chord and its arc is outside the shape, and
/// counting it in would turn the chords' sag into a miss.
fn fine_ring(contour: &Contour) -> Vec<Point> {
    let sag = TOL / 1000.0;
    let mut ring = Vec::new();
    for part in &contour.parts {
        let mut points = match part.segment.prim {
            Primitive::Line(l) => vec![l.start, l.end],
            Primitive::Arc(a) => {
                let step = 2.0 * (1.0 - sag / a.radius).clamp(-1.0, 1.0).acos();
                let n = (a.sweep.abs() / step).ceil().max(1.0) as usize;
                (0..=n)
                    .map(|i| {
                        let t = a.start_angle + a.sweep * i as f64 / n as f64;
                        (
                            a.center.0 + a.radius * t.cos(),
                            a.center.1 + a.radius * t.sin(),
                        )
                    })
                    .collect()
            }
        };
        if part.reversed {
            points.reverse();
        }
        ring.extend(points.into_iter().skip(usize::from(!ring.is_empty())));
    }
    ring
}

/// Even-odd: inside the outer contour and outside its holes, with the real arcs.
fn inside(contours: &[Contour], p: Point) -> bool {
    let crossings: usize = contours
        .iter()
        .map(|c| {
            let ring = fine_ring(c);
            let n = ring.len();
            (0..n)
                .filter(|&i| {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    (a.1 > p.1) != (b.1 > p.1)
                        && p.0 < a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0)
                })
                .count()
        })
        .sum();
    crossings % 2 == 1
}

/// Every point of every path, and points between them, as (point, cut radius).
fn samples(paths: &[CarvePath], bit: &VBit, step: f64) -> Vec<(Point, f64)> {
    let mut out = Vec::new();
    for path in paths {
        for k in 0..path.points.len().saturating_sub(1) {
            let (a, b) = (path.points[k], path.points[k + 1]);
            let (da, db) = (path.depths[k], path.depths[k + 1]);
            let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let n = (length / step).ceil().max(1.0) as usize;
            for i in 0..=n {
                let t = i as f64 / n as f64;
                let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                out.push((p, bit.radius_at(da + (db - da) * t)));
            }
        }
    }
    out
}

/// Rule 1 of spec §1: no cut's circle at the surface leaves the shape.
fn assert_no_gouge(contours: &[Contour], plan: &VcarvePlan, bit: &VBit) {
    for (p, r) in samples(&plan.paths, bit, 0.05) {
        let room = dist_to_outline(contours, p);
        assert!(
            inside(contours, p) || room <= TOL,
            "({:.4}, {:.4}) is outside the shape",
            p.0,
            p.1
        );
        assert!(
            r <= room + TOL,
            "the cut at ({:.4}, {:.4}) is {r:.4} wide, the outline {room:.4} away",
            p.0,
            p.1
        );
    }
}

/// Rule 2 of spec §1: every point of the shape is within reach of some cut (grid points
/// `spacing` apart). Returns the worst miss.
fn worst_miss(contours: &[Contour], plan: &VcarvePlan, bit: &VBit, spacing: f64) -> f64 {
    worst_miss_within(contours, plan, bit, spacing, f64::INFINITY)
}

/// [`worst_miss`] over the points no farther than `band` from the outline: the strip walls
/// reach when the floor between them is left.
fn worst_miss_within(
    contours: &[Contour],
    plan: &VcarvePlan,
    bit: &VBit,
    spacing: f64,
    band: f64,
) -> f64 {
    let cuts = samples(&plan.paths, bit, TOL / 2.0);
    let all: Vec<Point> = contours.iter().flat_map(|c| c.points()).collect();
    let lo = all.iter().fold((f64::INFINITY, f64::INFINITY), |a, p| {
        (a.0.min(p.0), a.1.min(p.1))
    });
    let hi = all
        .iter()
        .fold((f64::NEG_INFINITY, f64::NEG_INFINITY), |a, p| {
            (a.0.max(p.0), a.1.max(p.1))
        });
    let mut worst = 0.0_f64;
    let mut y = lo.1 + spacing / 2.0;
    while y < hi.1 {
        let mut x = lo.0 + spacing / 2.0;
        while x < hi.0 {
            let g = (x, y);
            if inside(contours, g) && dist_to_outline(contours, g) <= band {
                let miss = cuts
                    .iter()
                    .map(|(p, r)| ((g.0 - p.0).powi(2) + (g.1 - p.1).powi(2)).sqrt() - r)
                    .fold(f64::INFINITY, f64::min);
                worst = worst.max(miss);
            }
            x += spacing;
        }
        y += spacing;
    }
    worst
}

fn deepest(plan: &VcarvePlan) -> f64 {
    plan.deepest
}

// -- exact cases (spec §10)

#[test]
fn a_rectangle_is_carved_along_its_centre_line_and_into_its_corners() {
    let shape = [common::rect(1, 0.0, 0.0, 10.0, 4.0)];
    let bit = bit(90.0, 0.0);
    let plan = carve(&shape, &bit, 10.0);
    // A 90° bit: depth = half the width.
    assert!((deepest(&plan) - 2.0).abs() < 1e-6, "{}", deepest(&plan));
    // The centre line from (2, 2) to (8, 2) at depth 2, and a branch into each corner.
    let points: Vec<(Point, f64)> = plan
        .paths
        .iter()
        .flat_map(|p| p.points.iter().copied().zip(p.depths.iter().copied()))
        .collect();
    for corner in [(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (0.0, 4.0)] {
        assert!(
            points.iter().any(|(p, d)| {
                (p.0 - corner.0).abs() < 1e-6 && (p.1 - corner.1).abs() < 1e-6 && d.abs() < 1e-6
            }),
            "no cut into the corner {corner:?}: {points:?}"
        );
    }
    for x in [2.0, 8.0] {
        assert!(points.iter().any(|(p, d)| (p.0 - x).abs() < 1e-6
            && (p.1 - 2.0).abs() < 1e-6
            && (d - 2.0).abs() < 1e-6));
    }
    assert_no_gouge(&shape, &plan, &bit);
    assert!(worst_miss(&shape, &plan, &bit, 0.1) <= TOL);
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
}

#[test]
fn nearly_straight_arcs_are_carved_without_running_out_of_memory() {
    // Found by the rounded-shapes property: sides bowed by a bulge of -0.0018 (radius about
    // 2 m). The distance index boxed each arc by its whole circle and ran out of memory.
    let (w, h, bulge) = (
        15.229564116995071,
        7.237798519283311,
        -0.0017924511987609248,
    );
    let shape = vec![common::bulged(
        1,
        &[
            (0.0, 0.0, bulge),
            (w, 0.0, 0.0),
            (w, h, bulge),
            (0.0, h, 0.0),
        ],
    )];
    let bit = bit(60.0, 0.0);
    let plan = carve(&shape, &bit, 100.0);
    assert_no_gouge(&shape, &plan, &bit);
    assert!(worst_miss(&shape, &plan, &bit, 0.4) <= TOL);
}

#[test]
fn a_ring_is_one_circle_at_one_depth_without_spokes() {
    // A letter O: between circles of radius 10 and 6, the centre line is the circle of
    // radius 8, 2 mm from both.
    let shape = [
        common::circle_id(1, (0.0, 0.0), 10.0),
        common::circle_id(2, (0.0, 0.0), 6.0),
    ];
    let bit = bit(60.0, 0.0);
    let plan = carve(&shape, &bit, 10.0);
    let expected = 2.0 / (30.0f64).to_radians().tan();
    assert_eq!(plan.paths.len(), 1, "one ring, no spokes");
    for (p, d) in plan.paths[0].points.iter().zip(&plan.paths[0].depths) {
        let r = (p.0 * p.0 + p.1 * p.1).sqrt();
        assert!((r - 8.0).abs() < 0.01, "off the centre circle: {r}");
        assert!(
            (d - expected).abs() < 0.02,
            "depth {d}, expected {expected}"
        );
    }
    assert_no_gouge(&shape, &plan, &bit);
    assert!(worst_miss(&shape, &plan, &bit, 0.25) <= TOL);
}

#[test]
fn a_region_with_a_hole_is_carved_round_it() {
    let shape = [
        common::rect(1, 0.0, 0.0, 20.0, 12.0),
        common::rect(2, 6.0, 4.0, 14.0, 8.0),
    ];
    let bit = bit(90.0, 0.0);
    let plan = carve(&shape, &bit, 10.0);
    // Widest between the outer corner and the hole's: at (a, a), a = 10 - √48.
    let widest = 10.0 - 48.0f64.sqrt();
    assert!((deepest(&plan) - widest).abs() < 1e-6, "{}", deepest(&plan));
    assert_no_gouge(&shape, &plan, &bit);
    assert!(worst_miss(&shape, &plan, &bit, 0.2) <= TOL);
}

#[test]
fn a_flat_tip_carves_shallower_and_leaves_the_corner_tips() {
    let shape = [common::rect(1, 0.0, 0.0, 10.0, 4.0)];
    let bit = bit(60.0, 0.2);
    let plan = carve(&shape, &bit, 10.0);
    let t = (30.0f64).to_radians().tan();
    assert!((deepest(&plan) - (2.0 - 0.1) / t).abs() < 1e-6);
    assert_no_gouge(&shape, &plan, &bit);
    // Only the tips of the corners (where the width is under 0.2) aren't reached.
    assert!(worst_miss(&shape, &plan, &bit, 0.1) <= 0.1 / (45.0f64).to_radians().sin());
    assert!(
        plan.notes.is_empty(),
        "corner tips aren't reported: {:?}",
        plan.notes
    );
}

#[test]
fn a_stroke_wider_than_the_cap_gets_walls_and_its_floor_is_reported() {
    // 30 x 10, a 90° bit at most 2 deep: 4 wide at the surface (vcarve-wide-strokes.md §8).
    let shape = [common::rect(7, 0.0, 0.0, 30.0, 10.0)];
    let bit = bit(90.0, 0.0);
    let plan = carve(&shape, &bit, 2.0);
    assert!((deepest(&plan) - 2.0).abs() < 1e-9);
    assert_no_gouge(&shape, &plan, &bit);
    // The walls reach the outline all round: everything within 4 of it is cut.
    assert!(worst_miss_within(&shape, &plan, &bit, 0.25, 4.0 - TOL) <= TOL);
    // A closed wall 2 in from the outline, at 2 deep.
    let wall = plan
        .paths
        .iter()
        .find(|p| p.points.first() == p.points.last() && p.points.len() > 4)
        .expect("a closed wall");
    for (&p, &d) in wall.points.iter().zip(&wall.depths) {
        assert!((dist_to_outline(&shape, p) - 2.0).abs() <= TOL, "{p:?}");
        assert!((d - 2.0).abs() < 1e-9);
    }
    // No centre line down the middle, where the floor is: only the corner branches.
    for path in &plan.paths {
        for &p in &path.points {
            assert!(dist_to_outline(&shape, p) <= 2.0 + TOL, "{p:?}");
        }
    }
    assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
    let note = plan.notes[0].english();
    assert!(note.contains("contours #7"), "{note}");
    assert!(note.contains("floor pass"), "{note}");
}

/// The share of the floor cleared with an end mill of `radius` (1 when nothing is left).
fn cleared(shape: &[Contour], bit: &VBit, cap: f64, radius: f64) -> f64 {
    let refs: Vec<&Contour> = shape.iter().collect();
    let plan = plan_vcarve(&refs, bit, cap, (0.0, 0.0), Some(radius)).unwrap();
    plan.floors.iter().map(|f| f.cleared).fold(1.0, f64::min)
}

#[test]
fn a_floor_pass_that_fits_leaves_nothing_and_one_too_wide_gets_a_hint() {
    // 40 x 10, a 90° bit at most 2 deep: the floor is 36 x 6 (vcarve-wide-strokes.md §8).
    let shape = [common::rect(7, 0.0, 0.0, 40.0, 10.0)];
    let bit = bit(90.0, 0.0);
    let refs: Vec<&Contour> = shape.iter().collect();
    // A 3 mm end mill fits: the V-bit cleans the corners, nothing is left.
    let fits = plan_vcarve(&refs, &bit, 2.0, (0.0, 0.0), Some(1.5)).unwrap();
    assert!(fits.floors.is_empty(), "{:?}", fits.floors);
    assert!(
        fits.notes.is_empty(),
        "no floor-left note: {:?}",
        fits.notes
    );
    assert_no_gouge(&shape, &fits, &bit);
    // A 12 mm one doesn't fit: the hint is about the floor's width.
    let wide = plan_vcarve(&refs, &bit, 2.0, (0.0, 0.0), Some(6.0)).unwrap();
    assert_eq!(wide.floors.len(), 1, "{:?}", wide.floors);
    let floor = &wide.floors[0];
    assert_eq!(floor.contours, "#7");
    assert!(floor.cleared < 1.0);
    assert!(
        (5.0..=6.0).contains(&floor.largest),
        "largest {}",
        floor.largest
    );
    // The hint holds: an end mill of that size leaves nothing that counts.
    assert_eq!(cleared(&shape, &bit, 2.0, floor.largest / 2.0), 1.0);
    // The V-bit cleans along the centre line where the end mill can't reach.
    assert_no_gouge(&shape, &wide, &bit);
    assert!(
        wide.paths
            .iter()
            .flat_map(|p| &p.points)
            .any(|&p| dist_to_outline(&shape, p) > 4.0),
        "a clean-up down the middle"
    );
}

#[test]
fn the_bit_limits_the_depth_too() {
    // A 90° bit 6 mm across carves at most 3 mm deep.
    let shape = [common::rect(1, 0.0, 0.0, 30.0, 10.0)];
    let small = VBit {
        diameter: 6.0,
        ..bit(90.0, 0.0)
    };
    let plan = carve(&shape, &small, 10.0);
    assert!((deepest(&plan) - 3.0).abs() < 1e-9);
    assert!(
        plan.notes[0]
            .english()
            .contains("reaches at most 3.000 mm deep"),
        "{:?}",
        plan.notes
    );
}

#[test]
fn a_neck_narrower_than_the_tip_is_reported() {
    // Two 6 × 6 squares joined by a 0.1 mm neck, 2 mm long; the tip is 0.4 mm.
    let shape = [common::polygon(&[
        (0.0, 0.0),
        (6.0, 0.0),
        (6.0, 2.95),
        (8.0, 2.95),
        (8.0, 0.0),
        (14.0, 0.0),
        (14.0, 6.0),
        (8.0, 6.0),
        (8.0, 3.05),
        (6.0, 3.05),
        (6.0, 6.0),
        (0.0, 6.0),
    ])];
    let bit = bit(90.0, 0.4);
    let plan = carve(&shape, &bit, 10.0);
    assert_no_gouge(&shape, &plan, &bit);
    assert!(
        plan.notes
            .iter()
            .any(|n| n.english().contains("narrower than the tip")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn arcs_and_lines_together() {
    // A rounded slot: a stadium 20 long, 6 wide.
    let shape = [common::bulged(
        1,
        &[
            (0.0, 0.0, 0.0),
            (20.0, 0.0, 1.0),
            (20.0, 6.0, 0.0),
            (0.0, 6.0, 1.0),
        ],
    )];
    let bit = bit(60.0, 0.0);
    let plan = carve(&shape, &bit, 20.0);
    let expected = 3.0 / (30.0f64).to_radians().tan();
    assert!(
        (deepest(&plan) - expected).abs() < 0.02,
        "{}",
        deepest(&plan)
    );
    assert_no_gouge(&shape, &plan, &bit);
    assert!(worst_miss(&shape, &plan, &bit, 0.2) <= TOL);
}

// -- property test (spec §10)

fn star(points: usize, outer: f64, inner: f64, turn: f64) -> Contour {
    let ring: Vec<Point> = (0..2 * points)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            let a = turn + std::f64::consts::PI * i as f64 / points as f64;
            (r * a.cos(), r * a.sin())
        })
        .collect();
    common::polygon(&ring)
}

proptest! {
    #![proptest_config(common::proptest_config(24))]

    #[test]
    fn stars_are_carved_without_gouging_and_completely(
        points in 3usize..8,
        outer in 6.0f64..14.0,
        ratio in 0.3f64..0.8,
        turn in 0.0f64..1.0,
        angle in 30.0f64..120.0,
    ) {
        let shape = [star(points, outer, outer * ratio, turn)];
        let bit = bit(angle, 0.0);
        let plan = carve(&shape, &bit, 100.0);
        assert_no_gouge(&shape, &plan, &bit);
        let miss = worst_miss(&shape, &plan, &bit, 0.4);
        prop_assert!(miss <= TOL, "missed by {miss}");
    }

    #[test]
    fn rounded_shapes_are_carved_without_gouging_and_completely(
        w in 4.0f64..20.0,
        h in 4.0f64..20.0,
        bulge in -0.6f64..0.6,
        hole in proptest::bool::ANY,
    ) {
        // A rectangle with bulged sides, maybe with a round hole in the middle. Sides bowed
        // inwards stay apart: their sag (bulge × w / 2) is under a third of the height.
        let bulge = bulge.max(-0.6 * h / w);
        let mut shape = vec![common::bulged(
            1,
            &[(0.0, 0.0, bulge), (w, 0.0, 0.0), (w, h, bulge), (0.0, h, 0.0)],
        )];
        let r = w.min(h) / 5.0;
        if hole {
            shape.push(common::circle_id(2, (w / 2.0, h / 2.0), r));
        }
        let bit = bit(60.0, 0.0);
        let plan = carve(&shape, &bit, 100.0);
        assert_no_gouge(&shape, &plan, &bit);
        let miss = worst_miss(&shape, &plan, &bit, 0.4);
        prop_assert!(miss <= TOL, "missed by {miss}");
    }

    /// A smaller end mill never clears less of a floor (vcarve-wide-strokes.md §8).
    #[test]
    fn a_smaller_end_mill_never_clears_less(
        points in 3usize..7,
        outer in 8.0f64..14.0,
        ratio in 0.4f64..0.7,
        angle in 60.0f64..100.0,
        small in 0.3f64..2.0,
        more in 0.1f64..3.0,
    ) {
        let shape = [star(points, outer, outer * ratio, 0.3)];
        let bit = bit(angle, 0.0);
        let (a, b) = (cleared(&shape, &bit, 0.8, small), cleared(&shape, &bit, 0.8, small + more));
        prop_assert!(a >= b - 0.02, "{} mm clears {}, {} mm clears {}", small, a, small + more, b);
    }

    /// Too wide for the depth: walls, no gouge, and everything within the walls' reach
    /// (twice the cut's radius at the surface) of the outline is cut
    /// (vcarve-wide-strokes.md §8).
    #[test]
    fn wide_shapes_get_walls_without_gouging(
        points in 3usize..8,
        outer in 8.0f64..16.0,
        ratio in 0.4f64..0.8,
        turn in 0.0f64..1.0,
        angle in 30.0f64..120.0,
        tip in 0.0f64..0.5,
        cap in 0.3f64..1.5,
    ) {
        let shape = [star(points, outer, outer * ratio, turn)];
        let bit = bit(angle, tip);
        let plan = carve(&shape, &bit, cap);
        assert_no_gouge(&shape, &plan, &bit);
        let band = 2.0 * bit.radius_at(cap) - tip;
        let miss = worst_miss_within(&shape, &plan, &bit, 0.4, band - TOL);
        // Corner tips narrower than a flat tip are never reached (v-carve.md §1).
        let corner_tip = tip / 2.0 / (std::f64::consts::PI / points as f64 / 2.0).sin();
        prop_assert!(miss <= TOL + corner_tip, "missed by {miss}");
    }
}

// -- the operation in a job (plan_job)

/// A V-carve 0.5 to `end_depth` mm deep in `passes` passes.
fn vcarve_job(tool: serde_json::Value, end_depth: f64, passes: u32) -> chipload_job::Job {
    use chipload_job::migrations::migrate;
    let mut job = chipload_job::parse_job(
        migrate(serde_json::json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [tool],
            "operations": [{"type": "vcarve", "select": {"layer": "L"}, "tool": 1,
                            "start_depth": 0.5, "end_depth": end_depth,
                            "feed": 1000, "plunge_feed": 300}],
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap();
    if let chipload_job::Operation::Vcarve(v) = &mut job.operations[0] {
        v.passes = passes;
    }
    job
}

fn vbit_tool(tip: Option<f64>) -> serde_json::Value {
    serde_json::json!({"id": 1, "type": "vbit", "diameter": 20, "rpm": 18000,
                       "angle": std::f64::consts::FRAC_PI_2, "tip_diameter": tip})
}

#[test]
fn a_vcarve_needs_a_vbit_with_an_angle() {
    let shape = [common::rect(1, 0.0, 0.0, 10.0, 4.0)];
    let endmill = serde_json::json!({"id": 1, "diameter": 6, "rpm": 18000});
    let err = chipload_toolpath::plan_job(&vcarve_job(endmill, 10.0, 1), &shape).unwrap_err();
    assert!(err.to_string().contains("needs a V-bit"), "{err}");
    let no_angle = serde_json::json!({"id": 1, "type": "vbit", "diameter": 6, "rpm": 18000});
    let err = chipload_toolpath::plan_job(&vcarve_job(no_angle, 10.0, 1), &shape).unwrap_err();
    assert!(err.to_string().contains("no angle"), "{err}");
    let plan = chipload_toolpath::plan_job(&vcarve_job(vbit_tool(None), 10.0, 1), &shape).unwrap();
    assert!(
        plan.notes.iter().any(|n| n.contains("takes it as pointed")),
        "{:?}",
        plan.notes
    );
    let plan =
        chipload_toolpath::plan_job(&vcarve_job(vbit_tool(Some(0.0)), 10.0, 1), &shape).unwrap();
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
}

/// Every feed move as (from, to) in X, Y, Z.
fn feed_moves(program: &chipload_toolpath::model::Program) -> Vec<([f64; 3], [f64; 3])> {
    use chipload_toolpath::model::MoveKind;
    let mut at = [0.0, 0.0, program.safe_z];
    let mut out = Vec::new();
    for m in &program.moves {
        let next = [
            m.x.unwrap_or(at[0]),
            m.y.unwrap_or(at[1]),
            m.z.unwrap_or(at[2]),
        ];
        if m.kind == MoveKind::Feed {
            out.push((at, next));
        }
        at = next;
    }
    out
}

#[test]
fn passes_go_down_in_steps_and_never_recut() {
    // 90° bit, 10 × 4 rectangle: 2 mm deep below the surface at 0.5 mm.
    let shape = [common::rect(1, 0.0, 0.0, 10.0, 4.0)];
    let one =
        chipload_toolpath::plan_job(&vcarve_job(vbit_tool(Some(0.0)), 3.0, 1), &shape).unwrap();
    let stepped =
        chipload_toolpath::plan_job(&vcarve_job(vbit_tool(Some(0.0)), 3.0, 4), &shape).unwrap();
    let lowest = |moves: &[([f64; 3], [f64; 3])]| {
        moves
            .iter()
            .map(|(_, b)| b[2])
            .fold(f64::INFINITY, f64::min)
    };
    let (single, passes) = (feed_moves(&one.program), feed_moves(&stepped.program));
    assert!((lowest(&single) + 2.5).abs() < 1e-9, "{}", lowest(&single));
    assert!((lowest(&passes) + 2.5).abs() < 1e-9);
    // Four passes of equal load to the end depth, 2.5 below the surface (v-carve.md
    // decision 12): pass k at 0.5 + 2.5 × √(k / 4) below the top, where the carve is that
    // deep; the rectangle is 2 deep, so the third pass reaches its bottom and the fourth
    // finds nothing left.
    let levels: Vec<f64> = passes
        .iter()
        .filter(|(a, b)| a[2] == b[2] && (a[0] != b[0] || a[1] != b[1]))
        .map(|(_, b)| b[2])
        .collect();
    for z in [-0.5 - 1.25, -0.5 - 2.5 * 0.5_f64.sqrt(), -2.5] {
        assert!(
            levels.iter().any(|l| (l - z).abs() < 1e-9),
            "{z} in {levels:?}"
        );
    }
    // A move cut at full depth in one pass is never cut again at that depth.
    let horizontal: Vec<&([f64; 3], [f64; 3])> = passes
        .iter()
        .filter(|(a, b)| a != b && a[2] == b[2])
        .collect();
    for (i, x) in horizontal.iter().enumerate() {
        for y in &horizontal[i + 1..] {
            assert!(x != y, "cut twice: {x:?}");
        }
    }
    // Nothing above the carved surface is cut; nothing goes below 2.5.
    for (_, b) in &passes {
        assert!(b[2] <= -0.5 + 1e-9 || b[2] >= 5.0 - 1e-9, "{b:?}");
    }
}

/// Each area gets all its passes before the tool moves to the next, the nearest first
/// (v-carve.md decision 13; the user, 2026-10-31: "a lot of back and forth").
#[test]
fn each_area_is_finished_before_the_next() {
    // Two rectangles, the far one first in the drawing: the near one is cut first.
    let shape = [
        common::rect(1, 50.0, 0.0, 60.0, 4.0),
        common::rect(2, 20.0, 0.0, 30.0, 4.0),
    ];
    let plan =
        chipload_toolpath::plan_job(&vcarve_job(vbit_tool(Some(0.0)), 3.0, 3), &shape).unwrap();
    // The area each feed move cuts in, in cut order: once into the far one, never back.
    let areas: Vec<bool> = feed_moves(&plan.program)
        .iter()
        .filter(|(a, b)| a[0] != b[0] || a[1] != b[1])
        .map(|(_, b)| b[0] > 40.0)
        .collect();
    assert!(!areas[0], "the nearer area first");
    let switches = areas.windows(2).filter(|w| w[0] != w[1]).count();
    assert_eq!(switches, 1, "one move from the first area to the second");
    // Both areas get the same passes: 1.94 below the top, then the bottom at 2.5 (the
    // third pass finds nothing left; a 4 wide rectangle is 2 deep below the surface).
    let levels = |far: bool| {
        let mut zs: Vec<i64> = feed_moves(&plan.program)
            .iter()
            .filter(|(a, b)| (b[0] > 40.0) == far && a[2] == b[2] && a[0] != b[0])
            .map(|(_, b)| (b[2] * 1000.0).round() as i64)
            .collect();
        zs.sort_unstable();
        zs.dedup();
        zs
    };
    assert_eq!(levels(false), levels(true));
    assert_eq!(levels(false), [-2500, -1943]);
}

/// Depth passes remove equal cross-sections (v-carve.md decision 12): the last exactly the
/// depth, and a V groove's area (depth²) grows by the same amount each pass.
#[test]
fn passes_remove_equal_areas() {
    use chipload_toolpath::vcarve::carve_levels;
    // The user's example: 5 mm in 6 passes.
    let levels = carve_levels(5.0, 6);
    assert_eq!(levels.len(), 6);
    let rounded: Vec<f64> = levels.iter().map(|l| (l * 100.0).round() / 100.0).collect();
    assert_eq!(rounded, [2.04, 2.89, 3.54, 4.08, 4.56, 5.0]);
    for (depth, passes) in [(3.0, 3), (3.5, 4), (6.0, 100), (0.5, 2)] {
        let levels = carve_levels(depth, passes);
        assert_eq!(levels.len(), passes as usize);
        assert_eq!(*levels.last().unwrap(), depth);
        let area = levels[0] * levels[0];
        let mut before = 0.0;
        for level in &levels {
            assert!(((level * level - before) - area).abs() < 1e-9, "{levels:?}");
            before = level * level;
        }
    }
    // One pass, and 0 taken as one.
    assert_eq!(carve_levels(3.0, 1), [3.0]);
    assert_eq!(carve_levels(3.0, 0), [3.0]);
}

/// How many cuts a program has: lifts to safe Z after cutting.
fn lifts(program: &chipload_toolpath::model::Program) -> usize {
    use chipload_toolpath::model::MoveKind;
    let mut cutting = false;
    let mut count = 0;
    for m in &program.moves {
        if m.kind == MoveKind::Feed {
            cutting = true;
        } else if m.z.is_some_and(|z| z >= program.safe_z - 1e-9) && cutting {
            count += 1;
            cutting = false;
        }
    }
    count
}

/// docs/specs/vcarve-fewer-lifts.md: a rectangle's four corner branches (4.2 mm each) are
/// cut out and back from the centre line, not as cuts of their own, and every later pass
/// is one run through the groove already cut. The carve is the same.
#[test]
fn corners_are_cut_out_and_back_and_passes_go_on_through_the_groove() {
    // 90° bit, 30 × 6: the centre line 3 mm deep, its corner branches 3√2 long.
    let shape = [common::rect(1, 0.0, 0.0, 30.0, 6.0)];
    let one =
        chipload_toolpath::plan_job(&vcarve_job(vbit_tool(Some(0.0)), 3.5, 1), &shape).unwrap();
    assert_eq!(lifts(&one.program), 1, "one cut: the corners out and back");
    let three =
        chipload_toolpath::plan_job(&vcarve_job(vbit_tool(Some(0.0)), 3.5, 3), &shape).unwrap();
    assert_eq!(lifts(&three.program), 3, "one run a pass");
    // Every point reached at the end is as deep as one pass cuts it: the deepest Z at each
    // XY point is the same (all four corners reached, the centre at 3 below the surface).
    let deepest = |plan: &chipload_toolpath::plan::Plan| {
        let mut at: std::collections::BTreeMap<(i64, i64), i64> = Default::default();
        for (_, b) in feed_moves(&plan.program) {
            let key = (
                (b[0] * 1000.0).round() as i64,
                (b[1] * 1000.0).round() as i64,
            );
            let z = (b[2] * 1000.0).round() as i64;
            let e = at.entry(key).or_insert(z);
            *e = (*e).min(z);
        }
        at
    };
    let (a, b) = (deepest(&one), deepest(&three));
    for (key, z) in &a {
        if let Some(w) = b.get(key) {
            assert_eq!(z, w, "at {key:?}");
        }
    }
    for corner in [(0, 0), (30_000, 0), (30_000, 6_000), (0, 6_000)] {
        assert!(
            a.contains_key(&corner) && b.contains_key(&corner),
            "{corner:?}"
        );
    }
    // A link never goes below what is cut: nothing below the surface outside the strokes'
    // own depth (the bottom at 3.5 below the top).
    for (_, b) in feed_moves(&three.program) {
        assert!(b[2] >= -3.5 - 1e-9, "{b:?}");
    }
}
