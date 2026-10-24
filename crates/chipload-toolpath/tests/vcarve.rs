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
    plan_vcarve(&refs, bit, max_depth, (0.0, 0.0)).unwrap()
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
            if inside(contours, g) {
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
fn a_stroke_wider_than_the_cap_is_carved_to_it_and_reported() {
    let shape = [common::rect(7, 0.0, 0.0, 30.0, 10.0)];
    let bit = bit(90.0, 0.0);
    let plan = carve(&shape, &bit, 2.0);
    assert!((deepest(&plan) - 2.0).abs() < 1e-9);
    assert_no_gouge(&shape, &plan, &bit);
    assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
    assert!(
        plan.notes[0].english().contains("contours #7"),
        "{:?}",
        plan.notes[0]
    );
    assert!(
        plan.notes[0].english().contains("need 5.000 mm"),
        "{:?}",
        plan.notes[0]
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
            .contains("carves at most 3.000 mm deep"),
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
}

// -- the operation in a job (plan_job)

fn vcarve_job(tool: serde_json::Value, step_down: serde_json::Value) -> chipload_job::Job {
    use chipload_job::migrations::migrate;
    chipload_job::parse_job(
        migrate(serde_json::json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [tool],
            "operations": [{"type": "vcarve", "select": {"layer": "L"}, "tool": 1,
                            "start_depth": 0.5, "end_depth": 10, "step_down": step_down,
                            "feed": 1000, "plunge_feed": 300}],
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn vbit_tool(tip: Option<f64>) -> serde_json::Value {
    serde_json::json!({"id": 1, "type": "vbit", "diameter": 20, "rpm": 18000,
                       "angle": std::f64::consts::FRAC_PI_2, "tip_diameter": tip})
}

#[test]
fn a_vcarve_needs_a_vbit_with_an_angle() {
    let shape = [common::rect(1, 0.0, 0.0, 10.0, 4.0)];
    let endmill = serde_json::json!({"id": 1, "diameter": 6, "rpm": 18000});
    let err = chipload_toolpath::plan_job(&vcarve_job(endmill, serde_json::Value::Null), &shape)
        .unwrap_err();
    assert!(err.to_string().contains("needs a V-bit"), "{err}");
    let no_angle = serde_json::json!({"id": 1, "type": "vbit", "diameter": 6, "rpm": 18000});
    let err = chipload_toolpath::plan_job(&vcarve_job(no_angle, serde_json::Value::Null), &shape)
        .unwrap_err();
    assert!(err.to_string().contains("no angle"), "{err}");
    let plan = chipload_toolpath::plan_job(
        &vcarve_job(vbit_tool(None), serde_json::Value::Null),
        &shape,
    )
    .unwrap();
    assert!(
        plan.notes.iter().any(|n| n.contains("takes it as pointed")),
        "{:?}",
        plan.notes
    );
    let plan = chipload_toolpath::plan_job(
        &vcarve_job(vbit_tool(Some(0.0)), serde_json::Value::Null),
        &shape,
    )
    .unwrap();
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
    let one = chipload_toolpath::plan_job(
        &vcarve_job(vbit_tool(Some(0.0)), serde_json::Value::Null),
        &shape,
    )
    .unwrap();
    let stepped = chipload_toolpath::plan_job(
        &vcarve_job(vbit_tool(Some(0.0)), serde_json::json!(0.8)),
        &shape,
    )
    .unwrap();
    let lowest = |moves: &[([f64; 3], [f64; 3])]| {
        moves
            .iter()
            .map(|(_, b)| b[2])
            .fold(f64::INFINITY, f64::min)
    };
    let (single, passes) = (feed_moves(&one.program), feed_moves(&stepped.program));
    assert!((lowest(&single) + 2.5).abs() < 1e-9, "{}", lowest(&single));
    assert!((lowest(&passes) + 2.5).abs() < 1e-9);
    // Three passes: to 0.5 + 0.667, 0.5 + 1.333 and 0.5 + 2 below the top.
    let levels: Vec<f64> = passes
        .iter()
        .filter(|(a, b)| a[2] == b[2] && (a[0] != b[0] || a[1] != b[1]))
        .map(|(_, b)| b[2])
        .collect();
    for z in [-0.5 - 2.0 / 3.0, -0.5 - 4.0 / 3.0, -2.5] {
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
