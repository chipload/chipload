// SPDX-License-Identifier: GPL-3.0-or-later
//! Tab placement: unit tests and the no-overlap / no-lost-material property.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_job::Side;
use chipload_toolpath::ToolpathError;
use chipload_toolpath::arcs::ArcTable;
use chipload_toolpath::lap::{RingCut, TabSurface, ring_moves};
use chipload_toolpath::model::Move;
use chipload_toolpath::offset::offset_rings;
use chipload_toolpath::profile::orient;
use chipload_toolpath::tabs::{
    TAB_MAX_TURN_RAD, TabLayout, place_tabs, ring_length, split_ring, tab_polylines, tabs_at,
};
use common::polygon;
use proptest::prelude::*;

const D: f64 = 6.0;

/// A plunged profile with straight tabs, as before ramped tabs.
fn straight_tabbed(
    ring: &[Point],
    layout: &TabLayout,
    pass_z: &[f64],
    z_tab: f64,
) -> Result<Vec<Move>, ToolpathError> {
    let cut = RingCut {
        ring,
        tabs: Some(TabSurface {
            layout,
            z_tab,
            flank_angle: std::f64::consts::FRAC_PI_2,
        }),
        pass_z,
        top_z: 0.0,
        safe_z: 5.0,
        feed: 1800.0,
        plunge_feed: 600.0,
        ramp_angle: None,
        tool_diameter: D,
    };
    ring_moves(&cut, &ArcTable::default()).map(|r| r.moves)
}

fn rect_path() -> Vec<Point> {
    let rect = polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 60.0), (0.0, 60.0)]);
    let rings = offset_rings(&rect, Side::Outside, D / 2.0).unwrap();
    orient(&rings[0], true)
}

fn circle_path(r: f64, n: usize) -> Vec<Point> {
    let ring: Vec<Point> = (0..n)
        .map(|i| {
            let a = TAU * i as f64 / n as f64;
            (r * a.cos(), r * a.sin())
        })
        .collect();
    orient(&ring, true)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn gap_on_the_path_is_width_plus_tool_diameter() {
    let layout = place_tabs(&rect_path(), 4, 8.0, D).unwrap();
    assert!(layout.spans.iter().all(|(a, b)| near(b - a, 8.0 + D)));
}

#[test]
fn tabs_on_straight_edges_stay_at_nominal_positions() {
    let ring = rect_path();
    let layout = place_tabs(&ring, 4, 8.0, D).unwrap();
    let spacing = ring_length(&ring) / 4.0;
    for (i, (a, b)) in layout.spans.iter().enumerate() {
        assert!(near((a + b) / 2.0, (i as f64 + 0.5) * spacing));
    }
    assert_eq!(layout.on_curves, 0);
}

#[test]
fn tabs_slide_off_corners_within_their_slot() {
    let ring = rect_path();
    let layout = place_tabs(&ring, 8, 8.0, D).unwrap();
    let spacing = ring_length(&ring) / 8.0;
    let mut moved = 0;
    for (i, &(s0, s1)) in layout.spans.iter().enumerate() {
        let i = i as f64;
        let nominal = (i + 0.5) * spacing;
        if ((s0 + s1) / 2.0 - nominal).abs() > 1e-9 {
            moved += 1;
        }
        assert!(i * spacing + D / 2.0 <= s0 + 1e-9);
        assert!(s1 <= (i + 1.0) * spacing - D / 2.0 + 1e-9);
        assert!(((s0 + s1) / 2.0 - nominal).abs() <= spacing / 4.0 + 1e-9);
    }
    assert!(moved > 0, "two nominal centres sit on the rounded corners");
    assert_eq!(layout.on_curves, 0);
}

#[test]
fn slid_tabs_avoid_turning() {
    let ring = rect_path();
    let layout = place_tabs(&ring, 8, 8.0, D).unwrap();
    for line in tab_polylines(&ring, &layout) {
        let headings: Vec<f64> = line
            .windows(2)
            .map(|w| (w[1].1 - w[0].1).atan2(w[1].0 - w[0].0))
            .collect();
        let turning: f64 = headings
            .windows(2)
            .map(|h| {
                let d = h[1] - h[0];
                (d - TAU * (d / TAU).round_ties_even()).abs()
            })
            .sum();
        assert!(turning < TAB_MAX_TURN_RAD + 1e-9);
    }
}

#[test]
fn round_part_keeps_nominal_tabs_and_reports_them() {
    let layout = place_tabs(&circle_path(15.0, 90), 3, 5.0, D).unwrap();
    assert_eq!(layout.on_curves, 3);
}

fn message(err: ToolpathError) -> String {
    err.to_string()
}

#[test]
fn too_many_tabs_is_an_error_naming_what_fits() {
    let err = place_tabs(&rect_path(), 20, 8.0, D).unwrap_err();
    assert!(message(err).contains("at most 16 fit"));
}

#[test]
fn no_tab_fits_on_a_tiny_path() {
    let err = place_tabs(&circle_path(2.0, 90), 1, 8.0, D).unwrap_err();
    assert!(message(err).contains("not even one fits"));
}

#[test]
fn one_tab_that_just_fits() {
    let ring = circle_path(10.0, 90);
    let width = ring_length(&ring) - 2.0 * D; // spacing - gap == D exactly
    assert_eq!(
        place_tabs(&ring, 1, width - 1e-6, D).unwrap().spans.len(),
        1
    );
    assert!(place_tabs(&ring, 1, width + 1e-6, D).is_err());
}

#[test]
fn tool_wider_than_the_tab_is_fine() {
    let layout = place_tabs(&rect_path(), 4, 2.0, 12.0).unwrap();
    assert!(layout.spans.iter().all(|(a, b)| near(b - a, 14.0)));
}

#[test]
fn start_point_is_never_inside_a_tab() {
    let layout = place_tabs(&rect_path(), 8, 8.0, D).unwrap();
    assert!(
        layout
            .spans
            .iter()
            .all(|&(a, b)| a > 0.0 && b < layout.length)
    );
}

#[test]
fn split_inserts_exact_boundaries() {
    let ring = rect_path();
    let layout = place_tabs(&ring, 4, 8.0, D).unwrap();
    let path = split_ring(&ring, &layout.spans);
    let mut in_tab = 0.0;
    let mut previous = ring[0];
    for &(point, flag) in &path {
        if flag {
            in_tab += (point.0 - previous.0).hypot(point.1 - previous.1);
        }
        previous = point;
    }
    assert!((in_tab - 4.0 * (8.0 + D)).abs() < 1e-6);
    assert_eq!(path.last().unwrap().0, ring[0]);
}

fn count_at(moves: &[Move], z: f64) -> usize {
    moves.iter().filter(|m| m.z == Some(z)).count()
}

#[test]
fn only_passes_below_the_tab_top_lift() {
    let ring = rect_path();
    let layout = place_tabs(&ring, 4, 8.0, D).unwrap();
    let moves = straight_tabbed(&ring, &layout, &[-6.0, -12.0, -18.0], -15.0).unwrap();
    let mut levels: Vec<f64> = moves.iter().filter_map(|m| m.z).collect();
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    assert_eq!(levels, [-18.0, -15.0, -12.0, -6.0, 5.0]);
    assert_eq!(count_at(&moves, -15.0), 4); // once per tab, on the last pass only
}

#[test]
fn taller_tabs_lift_on_more_passes() {
    let ring = rect_path();
    let layout = place_tabs(&ring, 4, 8.0, D).unwrap();
    let moves = straight_tabbed(&ring, &layout, &[-6.0, -12.0, -18.0], -10.0).unwrap();
    assert_eq!(count_at(&moves, -10.0), 8); // passes -12 and -18
}

fn convex_ring() -> impl Strategy<Value = Vec<Point>> {
    (3usize..40).prop_flat_map(|n| {
        (
            prop::collection::btree_set(0u32..36_000, n),
            prop::collection::vec(20.0f64..80.0, n),
        )
            .prop_filter_map("needs n distinct angles", move |(angles, radii)| {
                (angles.len() == n).then(|| {
                    angles
                        .iter()
                        .zip(&radii)
                        .map(|(&a, &r)| {
                            let a = f64::from(a) / 36_000.0 * TAU;
                            let round = |v: f64| (v * 1e6).round() / 1e6;
                            (round(r * a.cos()), round(r * a.sin()))
                        })
                        .collect()
                })
            })
    })
}

// -- placed by hand (docs/specs/tabs-by-hand.md §3)

/// A 200 × 100 mm square path from (0, 0), counter-clockwise: 600 mm round. With D = 6 and
/// width 8 a tab's gap is 14 mm: centres at least 20 mm apart, and from 10 to 590 along.
fn square() -> Vec<Point> {
    vec![(0.0, 0.0), (200.0, 0.0), (200.0, 100.0), (0.0, 100.0)]
}

#[test]
fn tabs_by_hand_stay_where_they_are_put_even_on_a_corner() {
    let layout = tabs_at(
        &square(),
        &[(100.0, 0.0), (200.0, 0.0), (50.0, 100.0)],
        8.0,
        D,
    )
    .unwrap();
    let centres: Vec<f64> = layout.spans.iter().map(|(a, b)| (a + b) / 2.0).collect();
    assert_eq!(centres, vec![100.0, 200.0, 450.0]);
    assert_eq!(layout.on_curves, 1, "the one on the corner is noted");
    assert!(straight_tabbed(&square(), &layout, &[-3.0, -6.0], -4.0).is_ok());
}

#[test]
fn tabs_by_hand_are_moved_onto_the_ring() {
    let layout = tabs_at(&square(), &[(100.0, -2.5)], 8.0, D).unwrap();
    assert!(near(layout.spans[0].0, 93.0) && near(layout.spans[0].1, 107.0));
}

#[test]
fn tabs_by_hand_closer_than_their_width_and_two_tools_are_refused() {
    assert!(tabs_at(&square(), &[(100.0, 0.0), (120.0, 0.0)], 8.0, D).is_ok());
    let err = message(tabs_at(&square(), &[(100.0, 0.0), (119.99, 0.0)], 8.0, D).unwrap_err());
    assert!(
        err.contains("the tabs at (100.000, 0.000) mm and (119.990, 0.000) mm are too close"),
        "{err}"
    );
}

#[test]
fn a_tab_by_hand_may_not_cover_the_rings_start() {
    assert!(tabs_at(&square(), &[(10.0, 0.0)], 8.0, D).is_ok());
    assert!(
        tabs_at(&square(), &[(0.0, 10.0)], 8.0, D).is_ok(),
        "590 along, coming back"
    );
    for p in [(9.99, 0.0), (0.0, 9.99), (0.0, 0.0)] {
        let err = message(tabs_at(&square(), &[p], 8.0, D).unwrap_err());
        assert!(err.contains("covers the ring's start"), "{p:?}: {err}");
    }
}

proptest! {
    #![proptest_config(common::proptest_config(150))]

    #[test]
    fn tabs_never_overlap_cover_the_start_or_lose_material(
        ring in convex_ring(),
        count in 1usize..=12,
        width in 1.0f64..20.0,
        tool in 2.0f64..12.0,
    ) {
        let Ok(layout) = place_tabs(&ring, count, width, tool) else {
            // Refused only when they really don't fit.
            prop_assert!(ring_length(&ring) / count as f64 - (width + tool) < tool);
            return Ok(());
        };
        let spans = &layout.spans;
        prop_assert_eq!(spans.len(), count);
        prop_assert!(spans[0].0 >= tool / 2.0 - 1e-9);
        prop_assert!(spans[count - 1].1 <= layout.length - tool / 2.0 + 1e-9);
        for w in spans.windows(2) {
            prop_assert!(w[1].0 - w[0].1 >= tool - 1e-9);
        }
        for (s0, s1) in spans {
            prop_assert!((s1 - s0 - (width + tool)).abs() < 1e-9);
        }
        // The move builder verifies its own result (never below a tab).
        prop_assert!(straight_tabbed(&ring, &layout, &[-3.0, -6.0], -4.0).is_ok());
    }

    /// Tabs placed by hand anywhere on the square, kept apart and off the start: the gaps are
    /// exact and the moves never cut below a tab.
    #[test]
    fn tabs_by_hand_cut_as_placed(
        along in proptest::collection::vec(10.0f64..590.0, 1..12),
    ) {
        let mut along = along;
        along.sort_by(f64::total_cmp);
        along.dedup_by(|b, a| *b - *a < 20.0);
        let ring = square();
        let at = |s: f64| match s {
            s if s <= 200.0 => (s, 0.0),
            s if s <= 300.0 => (200.0, s - 200.0),
            s if s <= 500.0 => (500.0 - s, 100.0),
            s => (0.0, 600.0 - s),
        };
        let points: Vec<Point> = along.iter().map(|&s| at(s)).collect();
        let layout = tabs_at(&ring, &points, 8.0, D).unwrap();
        prop_assert_eq!(layout.spans.len(), points.len());
        for (s0, s1) in &layout.spans {
            prop_assert!((s1 - s0 - 14.0).abs() < 1e-9);
        }
        prop_assert!(straight_tabbed(&ring, &layout, &[-3.0, -6.0], -4.0).is_ok());
    }
}
