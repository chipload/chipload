// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::primitives::signed_area;
use chipload_job::{Direction, Entry, Side, Spindle};
use chipload_toolpath::ToolpathError;
use chipload_toolpath::arcs::ArcTable;
use chipload_toolpath::lap::{RingCut, ring_moves};
use chipload_toolpath::model::Move;
use chipload_toolpath::offset::offset_rings;
use chipload_toolpath::profile::{PathRing, cut_clockwise, order_rings, orient, pass_depths};
use common::{bbox, circle, polygon};

const SQUARE: [(f64, f64); 4] = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];

fn close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)]
        .iter()
        .all(|(x, y)| (x - y).abs() < 1e-9)
}

#[test]
fn outside_offset_grows_by_tool_radius() {
    let rings = offset_rings(&polygon(&SQUARE), Side::Outside, 3.0).unwrap();
    assert_eq!(rings.len(), 1);
    assert!(close(bbox(&rings[0]), (-3.0, -3.0, 103.0, 103.0)));
}

#[test]
fn inside_offset_shrinks_by_tool_radius() {
    let rings = offset_rings(&polygon(&SQUARE), Side::Inside, 3.0).unwrap();
    assert_eq!(rings.len(), 1);
    assert!(close(bbox(&rings[0]), (3.0, 3.0, 97.0, 97.0)));
}

#[test]
fn on_follows_the_contour() {
    let rings = offset_rings(&polygon(&SQUARE), Side::On, 3.0).unwrap();
    assert_eq!(rings, vec![SQUARE.to_vec()]);
}

#[test]
fn tool_larger_than_hole_is_an_error() {
    let hole = polygon(&[(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)]);
    let err = offset_rings(&hole, Side::Inside, 3.0).unwrap_err();
    assert!(matches!(&err, ToolpathError::Geometry(m) if m.contains("does not fit")));
}

#[test]
fn tool_exactly_as_wide_as_slot_is_an_error() {
    let slot = polygon(&[(0.0, 0.0), (50.0, 0.0), (50.0, 6.0), (0.0, 6.0)]);
    assert!(offset_rings(&slot, Side::Inside, 3.0).is_err());
}

#[test]
fn narrow_neck_splits_inside_offset() {
    // Two 40 mm squares joined by a 4 mm wide neck; a 6 mm tool cannot pass the neck.
    let dumbbell = polygon(&[
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 18.0),
        (60.0, 18.0),
        (60.0, 0.0),
        (100.0, 0.0),
        (100.0, 40.0),
        (60.0, 40.0),
        (60.0, 22.0),
        (40.0, 22.0),
        (40.0, 40.0),
        (0.0, 40.0),
    ]);
    let rings = offset_rings(&dumbbell, Side::Inside, 3.0).unwrap();
    assert_eq!(rings.len(), 2);
    // Sorted by lowest point, then x: the left square first.
    assert!(rings[0][0].0 < 50.0 && rings[1][0].0 > 50.0);
}

#[test]
fn direction_table() {
    use Direction::{Climb, Conventional};
    use Side::{Inside, On, Outside};
    use Spindle::{Ccw, Cw};
    let table = [
        (Outside, Climb, Cw, true),
        (Outside, Conventional, Cw, false),
        (Inside, Climb, Cw, false),
        (Inside, Conventional, Cw, true),
        (On, Climb, Cw, true),
        (On, Conventional, Cw, false),
        (Outside, Climb, Ccw, false),
        (Outside, Conventional, Ccw, true),
        (Inside, Climb, Ccw, true),
        (Inside, Conventional, Ccw, false),
        (On, Climb, Ccw, false),
        (On, Conventional, Ccw, true),
    ];
    for (side, direction, spindle, clockwise) in table {
        assert_eq!(cut_clockwise(side, direction, spindle), clockwise);
        let rings = offset_rings(&polygon(&SQUARE), side, 3.0).unwrap();
        let oriented = orient(&rings[0], clockwise);
        assert_eq!(signed_area(&oriented) < 0.0, clockwise);
    }
}

#[test]
fn orient_starts_at_lowest_then_leftmost_vertex() {
    let ring = [(100.0, 100.0), (0.0, 100.0), (0.0, 0.0), (100.0, 0.0)];
    assert_eq!(orient(&ring, true)[0], (0.0, 0.0));
    assert_eq!(orient(&ring, false)[0], (0.0, 0.0));
}

#[test]
fn start_ties_within_chaining_tolerance_go_left() {
    // Float noise must not move the start to the right end of a flat bottom.
    let ring = [(0.0, 0.0), (100.0, -1e-12), (100.0, 100.0), (0.0, 100.0)];
    assert_eq!(orient(&ring, true)[0], (0.0, 0.0));
}

#[test]
fn circle_starts_at_its_true_lowest_point() {
    let rings = offset_rings(&circle((10.0, 20.0), 50.0), Side::Outside, 3.0).unwrap();
    let start = orient(&rings[0], true)[0];
    assert!((start.0 - 10.0).abs() < 1e-9 && (start.1 - (20.0 - 53.0)).abs() < 1e-9);
}

#[test]
fn full_circle_offset_stays_round() {
    for side in [Side::Outside, Side::Inside] {
        let rings = offset_rings(&circle((0.0, 0.0), 50.0), side, 3.0).unwrap();
        let expected = if side == Side::Outside { 53.0 } else { 47.0 };
        for p in &rings[0] {
            // Vertices lie on the true arc.
            assert!((p.0.hypot(p.1) - expected).abs() < 1e-9);
        }
    }
}

#[test]
fn pass_depths_cases() {
    let cases: [(f64, f64, f64, &[f64]); 8] = [
        (0.0, 18.0, 6.0, &[6.0, 12.0, 18.0]), // exact multiple: no extra sliver pass
        (0.0, 6.0, 6.0, &[6.0]),              // depth equal to step down
        (0.0, 2.0, 6.0, &[2.0]),              // shallower than one step
        (0.0, 10.0, 6.0, &[5.0, 10.0]),       // not a multiple: equal passes
        (0.0, 18.0005, 6.0, &[6.000_166_7, 12.000_333_3, 18.0005]), // within DEPTH_TOL
        // With a start depth only the range is split (docs/specs/depths-and-clearance.md).
        (2.0, 6.0, 2.0, &[4.0, 6.0]),
        (5.0, 5.5, 6.0, &[5.5]), // step down larger than the range
        (3.0, 12.0, 4.0, &[6.0, 9.0, 12.0]), // 9 mm in passes of at most 4: three of 3
    ];
    for (start, end, step, expected) in cases {
        let got = pass_depths(start, end, step).unwrap();
        assert_eq!(got.len(), expected.len(), "{start}-{end}/{step}: {got:?}");
        for (g, e) in got.iter().zip(expected) {
            assert!((g - e).abs() < 1e-6, "{start}-{end}/{step}: {got:?}");
        }
    }
}

#[test]
fn pass_depths_never_exceed_step_down_and_end_exactly() {
    for start in [0.0, 0.5, 3.0] {
        for end in [3.1, 5.9, 6.0, 6.1, 17.99, 18.0, 18.01, 100.0] {
            let passes = pass_depths(start, end, 6.0).unwrap();
            let mut previous = start;
            for &p in &passes {
                assert!(p > previous && p - previous <= 6.0 + 1e-9, "{passes:?}");
                previous = p;
            }
            assert_eq!(*passes.last().unwrap(), end);
        }
    }
}

#[test]
fn pass_depths_rejects_bad_ranges() {
    assert!(pass_depths(0.0, 0.0, 6.0).is_err());
    assert!(pass_depths(0.0, 6.0, 0.0).is_err());
    assert!(pass_depths(0.0, f64::NAN, 6.0).is_err());
    assert!(pass_depths(4.0, 4.0, 6.0).is_err()); // start = end
    assert!(pass_depths(5.0, 4.0, 6.0).is_err()); // start below end
    assert!(pass_depths(-1.0, 4.0, 6.0).is_err()); // above the stock
}

#[test]
fn contour_moves_sequence() {
    let ring = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
    let cut = RingCut {
        ring: &ring,
        tabs: None,
        leads: None,
        pass_z: &[-1.0, -2.0],
        top_z: 0.0,
        safe_z: 5.0,
        feed: 1000.0,
        plunge_feed: 300.0,
        entry: Entry::Plunge {},
        tool_diameter: 6.0,
    };
    let moves = ring_moves(&cut, &ArcTable::default()).unwrap().moves;
    assert_eq!(moves[0], Move::rapid_z(5.0));
    assert_eq!(moves[1], Move::rapid_xy((0.0, 0.0)));
    assert_eq!(moves[2], Move::feed_z(-1.0, 300.0));
    assert_eq!(moves[5], Move::feed_xy((0.0, 0.0), 1000.0)); // closes the loop
    assert_eq!(moves[6], Move::feed_z(-2.0, 300.0)); // next pass without retracting
    assert_eq!(*moves.last().unwrap(), Move::rapid_z(5.0));
}

fn ring_at(x: f64, y: f64, depth: usize, id: usize) -> PathRing {
    PathRing {
        points: vec![(x, y), (x + 1.0, y), (x + 1.0, y + 1.0)],
        nesting_depth: depth,
        contour_id: id,
    }
}

#[test]
fn holes_are_cut_before_their_outline() {
    let rings = vec![
        ring_at(0.0, 0.0, 0, 1),
        ring_at(80.0, 80.0, 1, 2),
        ring_at(10.0, 10.0, 1, 3),
    ];
    let ids: Vec<usize> = order_rings(rings, (0.0, 0.0))
        .iter()
        .map(|r| r.contour_id)
        .collect();
    assert_eq!(ids, [3, 2, 1]);
}

#[test]
fn ordering_is_nearest_neighbour_from_origin() {
    let rings = vec![
        ring_at(100.0, 0.0, 0, 1),
        ring_at(10.0, 0.0, 0, 2),
        ring_at(50.0, 0.0, 0, 3),
    ];
    let ids: Vec<usize> = order_rings(rings, (0.0, 0.0))
        .iter()
        .map(|r| r.contour_id)
        .collect();
    assert_eq!(ids, [2, 3, 1]);
}

#[test]
fn on_profile_drops_near_duplicate_vertices() {
    let ring = polygon(&[(0.0, 0.0), (1e-9, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    let rings = offset_rings(&ring, Side::On, 3.0).unwrap();
    assert_eq!(rings, vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]]);
}

#[test]
fn degenerate_spike_is_refused_or_verified() {
    // Found by Hypothesis in the Python reference: a zero-width spike with denormal coordinates.
    let ring = polygon(&[
        (10.0, 0.0),
        (11.0, 2.4e-312),
        (12.0, 9.7e-227),
        (11.0, 1.3e-213),
        (8.776, 4.794),
        (5.403, 8.415),
        (-4.161, 9.093),
        (-9.9, 1.411),
        (0.0, -10.0),
    ]);
    // Either refused, or passed the built-in verification; both are safe.
    let _ = offset_rings(&ring, Side::Inside, 1.0);
}
