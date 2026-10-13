// SPDX-License-Identifier: GPL-3.0-or-later
//! Property: the tool-centre path keeps the cutter on the requested side and never gouges.
//! On random star polygons with bulged (arc) edges.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use cavalier_contours::core::math::Vector2;
use cavalier_contours::polyline::PlineSource;
use chipload_core::Point;
use chipload_core::primitives::Primitive;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, OFFSET_CHECK_SLACK_MM, OFFSET_FAR_SLACK_MM};
use chipload_job::Side;
use chipload_toolpath::offset::{contour_pline, offset_rings, pline_segments, point_distance};
use proptest::prelude::*;
use std::f64::consts::TAU;

/// Star-shaped polygons are simple for straight edges; bulges are kept small and the result
/// is checked for self-intersection anyway.
fn star() -> impl Strategy<Value = Vec<(f64, f64, f64)>> {
    (3usize..24).prop_flat_map(|n| {
        (
            prop::collection::btree_set(0u32..36_000, n),
            prop::collection::vec(10.0f64..100.0, n),
            prop::collection::vec(prop_oneof![Just(0.0), -0.3f64..0.3], n),
        )
            .prop_filter_map("needs n distinct angles", move |(angles, radii, bulges)| {
                (angles.len() == n).then(|| {
                    angles
                        .iter()
                        .zip(&radii)
                        .zip(&bulges)
                        .map(|((&a, &r), &b)| {
                            let a = f64::from(a) / 36_000.0 * TAU;
                            // Rounded to 1e-6 mm, realistic DXF precision.
                            let round = |v: f64| (v * 1e6).round() / 1e6;
                            (round(r * a.cos()), round(r * a.sin()), b)
                        })
                        .collect()
                })
            })
    })
}

/// Points along the closed ring every ~0.5 mm, including mid-chord points.
fn sample(ring: &[Point]) -> Vec<Point> {
    let n = ring.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let steps = ((len / 0.5).ceil() as usize).max(2);
        out.extend((0..steps).map(|k| {
            let t = k as f64 / steps as f64;
            (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
        }));
    }
    out
}

struct Part {
    segments: Vec<Primitive>,
    pline: cavalier_contours::polyline::Polyline<f64>,
}

impl Part {
    fn new(vertices: &[(f64, f64, f64)]) -> Option<Part> {
        let contour = common::bulged(1, vertices);
        let pline = contour_pline(&contour);
        // Contours that fold back on themselves are refused on import
        // (docs/specs/folded-contours.md), so they never reach an offset.
        if pline.scan_for_self_intersect()
            || pline.area().abs() < 1.0
            || chipload_core::fold::fold_point(&contour.points()).is_some()
        {
            return None;
        }
        Some(Part {
            segments: pline_segments(&pline),
            pline,
        })
    }

    fn distance(&self, p: Point) -> f64 {
        self.segments
            .iter()
            .map(|s| point_distance(s, p))
            .fold(f64::INFINITY, f64::min)
    }

    fn contains(&self, p: Point) -> bool {
        self.pline.winding_number(Vector2::new(p.0, p.1)) != 0
    }
}

proptest! {
    #![proptest_config(common::proptest_config(200))]

    #[test]
    fn outside_path_never_gouges(vertices in star(), radius in 0.5f64..10.0) {
        let Some(part) = Part::new(&vertices) else { return Ok(()) };
        let rings = offset_rings(&common::bulged(1, &vertices), Side::Outside, radius).unwrap();
        prop_assert_eq!(rings.len(), 1);
        for p in sample(&rings[0]) {
            prop_assert!(!part.contains(p));
            prop_assert!(part.distance(p) >= radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM);
        }
    }

    #[test]
    fn inside_path_never_gouges(vertices in star(), radius in 0.5f64..10.0) {
        let Some(part) = Part::new(&vertices) else { return Ok(()) };
        // Refusing (tool does not fit) is the safe outcome.
        let Ok(rings) = offset_rings(&common::bulged(1, &vertices), Side::Inside, radius) else {
            return Ok(());
        };
        for ring in &rings {
            for p in sample(ring) {
                prop_assert!(part.contains(p) || part.distance(p) <= OFFSET_CHECK_SLACK_MM);
                prop_assert!(
                    part.distance(p) >= radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM
                );
            }
        }
    }

    #[test]
    fn outside_path_stays_close(vertices in star(), radius in 0.5f64..10.0) {
        // The path hugs the part: no vertex is further than r from it (no wasted air cut).
        let Some(part) = Part::new(&vertices) else { return Ok(()) };
        let rings = offset_rings(&common::bulged(1, &vertices), Side::Outside, radius).unwrap();
        for &p in &rings[0] {
            prop_assert!(part.distance(p) <= radius + OFFSET_FAR_SLACK_MM);
        }
    }
}
