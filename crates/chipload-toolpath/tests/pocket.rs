// SPDX-License-Identifier: GPL-3.0-or-later
//! Pocket planning: unit tests and the no-gouge / clears-everything property.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::f64::consts::TAU;

use cavalier_contours::polyline::PlineSource;
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::fold::fold_point;
use chipload_core::primitives::{Primitive, distance, signed_area};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, OFFSET_CHECK_SLACK_MM};
use chipload_job::{Direction, Entry, Spindle};
use chipload_toolpath::ToolpathError;
use chipload_toolpath::geom::segment_distance;
use chipload_toolpath::model::MoveKind;
use chipload_toolpath::offset::{contour_pline, pline_segments, point_distance};
use chipload_toolpath::pocket::{
    Choices, PocketCut, PocketPlan, PocketRequest, build_region, plan_pocket, pocket_moves,
};
use common::{bbox, circle_id, polygon_id, rect};
use proptest::prelude::*;

fn outline() -> Contour {
    rect(1, 0.0, 0.0, 100.0, 60.0)
}

fn plan_with(
    contours: &[Contour],
    stepover: f64,
    direction: Direction,
    spindle: Spindle,
) -> Result<PocketPlan, ToolpathError> {
    let refs: Vec<&Contour> = contours.iter().collect();
    plan_pocket(&PocketRequest {
        direction,
        spindle,
        choices: Choices {
            spiral: true,
            ..Choices::default()
        },
        ..PocketRequest::new(&refs, 3.0, stepover)
    })
}

/// The moves of a piece cut in two 3 mm passes at the test feeds.
fn moves_of(
    piece: &chipload_toolpath::pocket::PocketPiece,
    pass_z: &[f64],
) -> Vec<chipload_toolpath::model::Move> {
    pocket_moves(
        piece,
        &PocketCut {
            pass_z,
            top_z: 0.0,
            safe_z: 5.0,
            feed: 1500.0,
            plunge_feed: 400.0,
            entry: Entry::Plunge {},
        },
    )
    .unwrap()
}

fn plan(contours: &[Contour]) -> PocketPlan {
    plan_with(contours, 2.4, Direction::Climb, Spindle::Cw).unwrap()
}

fn areas_of(contours: &[Contour]) -> Vec<(Vec<usize>, f64, usize)> {
    let refs: Vec<&Contour> = contours.iter().collect();
    build_region(&refs)
        .unwrap()
        .into_iter()
        .map(|a| {
            let holes: f64 = a.area.holes.iter().map(|h| h.area().abs()).sum();
            (
                a.contour_ids,
                a.area.outer.area().abs() - holes,
                a.area.holes.len(),
            )
        })
        .collect()
}

// -- region

#[test]
fn crossing_shapes_are_separate_pockets() {
    // User decision: crossing shapes are separate geometries, never merged into one region.
    let contours = [
        rect(1, 0.0, 0.0, 50.0, 10.0),
        rect(2, 20.0, -20.0, 30.0, 30.0),
    ];
    let mut areas = areas_of(&contours);
    areas.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(areas.len(), 2);
    assert_eq!((areas[0].0.clone(), areas[1].0.clone()), (vec![1], vec![2]));
    assert!(areas.iter().all(|a| (a.1 - 500.0).abs() < 1e-6));
    let result = plan(&contours);
    assert_eq!(result.pieces.len(), 2);
    assert!(result.notes.is_empty(), "{:?}", result.notes);
}

#[test]
fn selected_contour_inside_another_is_an_island() {
    let areas = areas_of(&[outline(), rect(2, 40.0, 20.0, 60.0, 40.0)]);
    assert_eq!(areas.len(), 1);
    assert_eq!(areas[0].0, [1, 2]);
    assert_eq!(areas[0].2, 1);
    assert!((areas[0].1 - (6000.0 - 400.0)).abs() < 1e-6);
}

#[test]
fn boundary_inside_an_island_is_cleared_again() {
    let areas = areas_of(&[
        outline(),
        rect(2, 20.0, 10.0, 80.0, 50.0),
        rect(3, 40.0, 20.0, 60.0, 40.0),
    ]);
    let ids: Vec<Vec<usize>> = areas.into_iter().map(|a| a.0).collect();
    assert_eq!(ids, [vec![1, 2], vec![3]]);
}

#[test]
fn island_touching_the_wall_is_cut_away() {
    let areas = areas_of(&[outline(), rect(2, 0.0, 20.0, 30.0, 40.0)]);
    assert_eq!(areas.len(), 1);
    assert_eq!(areas[0].2, 0, "a notch in the wall, not a hole");
    assert!((areas[0].1 - (6000.0 - 600.0)).abs() < 1e-6);
}

#[test]
fn crossing_islands_are_merged() {
    let areas = areas_of(&[
        outline(),
        rect(2, 20.0, 20.0, 50.0, 30.0),
        rect(3, 40.0, 10.0, 45.0, 50.0),
    ]);
    assert_eq!(areas.len(), 1);
    assert_eq!(areas[0].2, 1);
    assert!((areas[0].1 - (6000.0 - 300.0 - 200.0 + 50.0)).abs() < 1e-6);
}

#[test]
fn unselected_contours_are_not_islands() {
    let areas = areas_of(&[outline()]);
    assert_eq!(areas[0].2, 0);
}

#[test]
fn same_shape_twice_is_refused() {
    let reversed = polygon_id(2, &[(0.0, 60.0), (100.0, 60.0), (100.0, 0.0), (0.0, 0.0)]);
    let contours = [outline(), reversed];
    let refs: Vec<&Contour> = contours.iter().collect();
    let err = build_region(&refs).unwrap_err();
    assert!(
        matches!(&err, ToolpathError::Selector(m) if m.contains("same shape")),
        "{err}"
    );
}

#[test]
fn circle_and_polygon_of_the_same_circle_are_the_same_shape() {
    let n = 720;
    let points: Vec<Point> = (0..n)
        .map(|i| {
            let a = TAU * f64::from(i) / f64::from(n);
            (50.0 + 20.0 * a.cos(), 30.0 + 20.0 * a.sin())
        })
        .collect();
    let contours = [circle_id(1, (50.0, 30.0), 20.0), polygon_id(2, &points)];
    let refs: Vec<&Contour> = contours.iter().collect();
    assert!(build_region(&refs).is_err());
}

// -- rings

#[test]
fn ring_count_and_inside_out_order() {
    let result = plan(&[outline()]);
    let piece = &result.pieces[0];
    // Level 0 is 94 x 54; each level shrinks 2.4 per side until the 54 side is gone: 12 levels.
    assert_eq!(piece.rings.len(), 12);
    let (x0, y0, x1, y1) = bbox(&piece.rings.last().unwrap().points);
    assert!(
        [(x0, 3.0), (y0, 3.0), (x1, 97.0), (y1, 57.0)]
            .iter()
            .all(|(a, b)| (a - b).abs() < 1e-9)
    );
    let areas: Vec<f64> = piece
        .rings
        .iter()
        .map(|r| signed_area(&r.points).abs())
        .collect();
    assert!(areas.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn tool_too_big_is_an_error() {
    let err = plan_with(
        &[rect(7, 0.0, 0.0, 5.0, 5.0)],
        2.4,
        Direction::Climb,
        Spindle::Cw,
    )
    .unwrap_err();
    assert!(err.to_string().contains("does not fit"), "{err}");
}

#[test]
fn narrow_neck_warns() {
    let dumbbell = polygon_id(
        1,
        &[
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
        ],
    );
    let result = plan(&[dumbbell]);
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.english().contains("narrow part"))
    );
}

/// Distance from `p` to the walls and islands of the selected contours.
fn wall_distance(contours: &[Contour], p: Point) -> f64 {
    contours
        .iter()
        .flat_map(|c| pline_segments(&contour_pline(c)))
        .map(|s| point_distance(&s, p))
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn material_side_of_every_ring() {
    // With M3, climb keeps the material (nearest wall or island) on the right of travel.
    let contours = [outline(), circle_id(2, (50.0, 30.0), 8.0)];
    let cases = [
        (Direction::Climb, Spindle::Cw, true),
        (Direction::Conventional, Spindle::Cw, false),
        (Direction::Climb, Spindle::Ccw, false),
        (Direction::Conventional, Spindle::Ccw, true),
    ];
    for (direction, spindle, material_on_right) in cases {
        let result = plan_with(&contours, 2.4, direction, spindle).unwrap();
        for ring in &result.pieces[0].rings {
            let open = ring.open;
            let ring = &ring.points;
            let n = ring.len();
            // An open path (a ring that steps out) doesn't close back to its start.
            let chords = if open { n - 1 } else { n };
            let (a, b) = (0..chords)
                .map(|i| (ring[i], ring[(i + 1) % n]))
                .max_by(|x, y| distance(x.0, x.1).total_cmp(&distance(y.0, y.1)))
                .unwrap();
            let length = distance(a, b);
            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let normal = ((b.1 - a.1) / length, -(b.0 - a.0) / length); // to the right
            let right = (mid.0 + 0.1 * normal.0, mid.1 + 0.1 * normal.1);
            let left = (mid.0 - 0.1 * normal.0, mid.1 - 0.1 * normal.1);
            let right_is_closer = wall_distance(&contours, right) < wall_distance(&contours, left);
            assert_eq!(right_is_closer, material_on_right);
        }
    }
}

/// Every chord of every ring keeps the tool radius (minus tolerance) from walls and islands.
fn assert_clear(contours: &[Contour], result: &PocketPlan, radius: f64) {
    let walls: Vec<Primitive> = contours
        .iter()
        .flat_map(|c| pline_segments(&contour_pline(c)))
        .collect();
    let clearance = radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM;
    for piece in &result.pieces {
        for ring in &piece.rings {
            let n = ring.points.len();
            // Open paths (rings with leads) don't go back to their start.
            let chords = if ring.open { n.saturating_sub(1) } else { n };
            for i in 0..chords {
                let (a, b) = (ring.points[i], ring.points[(i + 1) % n]);
                for wall in &walls {
                    let d = segment_distance(a, b, wall);
                    assert!(d >= clearance, "chord {a:?}-{b:?} is {d} from a wall");
                }
            }
        }
    }
}

#[test]
fn rings_keep_tool_radius_from_walls_and_islands() {
    let contours = [outline(), circle_id(2, (50.0, 30.0), 8.0)];
    assert_clear(&contours, &plan(&contours), 3.0);
}

// -- uncut material

#[test]
fn large_stepover_reports_uncut_material() {
    // stepover = D: corners are missed.
    let result = plan_with(&[outline()], 6.0, Direction::Climb, Spindle::Cw).unwrap();
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.english().contains("uncut material"))
    );
}

#[test]
fn normal_stepover_leaves_nothing() {
    assert!(plan(&[outline()]).notes.is_empty());
}

#[test]
fn a_slot_narrower_than_the_tool_is_warned() {
    // Two 40 x 40 rooms joined by a 4 mm wide, 20 mm long passage; tool D 6.
    let room = polygon_id(
        1,
        &[
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
        ],
    );
    let result = plan(&[room]);
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.english().contains("narrow part"))
    );
}

#[test]
fn pocket_centre_is_cleared_where_geos_buffer_collapsed_too_early() {
    // Found by Hypothesis in the Python reference (GEOS returned empty too early): the centre
    // must be cleared.
    let ring = polygon_id(
        1,
        &[
            (30.0, 0.0),
            (26.327477, 14.382766),
            (2.122116, 29.92485),
            (-12.484405, 27.278923),
            (-24.034308, 17.954164),
            (-19.609309, -22.704075),
            (-6.323874, -29.325904),
        ],
    );
    let refs = [&ring];
    let result = plan_pocket(&PocketRequest::new(&refs, 1.0, 1.0)).unwrap();
    assert!(result.notes.is_empty(), "{:?}", result.notes);
}

#[test]
fn a_pocket_whose_island_offsets_just_touch_can_be_cleared() {
    // Found by the property test on 2026-10-10. At this exact stepover the offsets of islands
    // #2 and #4 meet almost exactly tangentially at one ring level; cavalier_contours then
    // asserts (debug) or returns rings that fail Chipload's verification (release): a safe
    // refusal of a valid drawing. The same pocket with the stepover 1e-6 mm smaller or larger
    // works, and without island #2 or #4 it works too, so the rings are retried with a
    // negligibly smaller stepover (docs/specs/offset-retry.md).
    let contours = [
        polygon_id(
            1,
            &[
                (30.0, 0.0),
                (30.322407, 1.705904),
                (5.054545, 37.482756),
                (-23.021876, 21.363316),
                (-25.829146, 19.140482),
                (-30.284893, 15.298),
                (-45.537702, -2.625687),
                (-58.104488, -5.359507),
                (-24.468721, -21.808497),
                (-22.392178, -19.964728),
                (-22.178358, -20.201991),
                (-22.174832, -20.205861),
                (-22.00122, -20.394762),
                (-21.99766, -20.398601),
                (-21.9941, -20.40244),
            ],
        ),
        polygon_id(
            2,
            &[
                (-9.272381, -2.474317),
                (-8.256644, -1.080425),
                (-8.4151, -1.118743),
                (-10.789403, -1.435533),
                (-13.201579, -0.639165),
                (-13.144529, -2.621),
                (-14.823898, -4.123532),
                (-12.349723, -6.245718),
            ],
        ),
        polygon_id(
            3,
            &[
                (-10.92616, -8.885295),
                (-10.506378, -9.53471),
                (-11.383609, -10.018626),
                (-11.08858, -12.68881),
                (-7.985534, -14.337199),
                (-6.678434, -11.361687),
            ],
        ),
        polygon_id(
            4,
            &[
                (1.445616, 0.069215),
                (3.533449, 0.823268),
                (0.213707, 0.942674),
                (-1.099894, 1.084898),
                (-2.706574, 2.254326),
                (-1.838731, 1.332842),
                (-2.488002, 0.823475),
                (-2.823642, -2.895785),
                (-0.813676, -2.389074),
                (0.16108, -1.48637),
                (0.521338, -2.941803),
                (2.204077, -3.460651),
                (1.499667, -0.225105),
            ],
        ),
    ];
    let refs: Vec<&Contour> = contours.iter().collect();
    let radius = 4.865292712794798;
    let result = plan_pocket(&PocketRequest::new(
        &refs,
        radius,
        0.18893806468913993 * 2.0 * radius,
    ))
    .unwrap();
    assert_clear(&contours, &result, radius);
}

#[test]
fn rings_get_past_a_wall_offset_that_just_touches() {
    // Found by the property tests: here the wall pass itself (the pocket shrunk by the tool
    // radius) hits the coincidence, so the retry moves it 1e-6 mm farther from the walls
    // (docs/specs/offset-retry.md).
    let contours = [
        polygon_id(
            1,
            &[
                (37.13323, 4.493615),
                (52.680761, 10.402409),
                (33.226386, 7.372197),
                (40.940788, 11.677743),
                (-47.224613, 33.834442),
                (-1.955255, -47.442586),
                (7.197566, -38.796799),
                (7.093812, -38.089252),
                (47.301213, -30.308581),
                (36.547621, -23.247548),
                (51.178369, -4.729656),
                (43.407037, -3.500072),
            ],
        ),
        polygon_id(
            2,
            &[
                (7.392563, -0.017588),
                (6.232092, 0.925335),
                (6.128889, 1.06124),
                (4.557844, 1.060071),
                (2.139117, -0.430789),
                (1.800265, -0.651998),
                (3.653979, -1.90574),
                (2.556696, -3.950995),
                (4.12737, -2.700044),
                (5.722765, -1.470621),
            ],
        ),
        polygon_id(
            3,
            &[
                (-1.591528, 8.765963),
                (-0.848709, 9.601491),
                (-1.712437, 9.695428),
                (-2.125371, 11.029932),
                (-2.658742, 9.616261),
                (-3.833042, 10.475096),
                (-4.995071, 10.693974),
                (-4.618075, 10.012663),
                (-3.84308, 8.975507),
                (-6.227303, 6.791432),
                (-2.177225, 6.516531),
                (-0.823733, 5.119389),
                (-1.353595, 6.21427),
                (-1.848735, 7.819246),
            ],
        ),
        polygon_id(
            4,
            &[
                (1.610987, -6.912785),
                (-0.131615, -6.653224),
                (-0.776742, -6.567334),
                (-2.094251, -3.976692),
                (-2.013122, -5.676233),
                (-2.169977, -6.690901),
                (-3.490718, -5.628196),
                (-3.160791, -7.343266),
                (-1.258303, -8.65368),
                (-0.352758, -9.543414),
                (-0.296498, -8.539492),
                (0.783847, -7.63515),
            ],
        ),
    ];
    let refs: Vec<&Contour> = contours.iter().collect();
    let radius = 3.073129921791429;
    let result = plan_pocket(&PocketRequest::new(&refs, radius, 0.4 * radius));
    assert!(result.is_ok(), "{:?}", result.err());
}

// -- links

#[test]
fn rings_feed_short_safe_links() {
    assert!(plan(&[outline()]).pieces[0].feed_links.iter().all(|&l| l));
}

#[test]
fn links_still_retract_around_an_island() {
    let result = plan(&[outline(), rect(2, 30.0, 20.0, 70.0, 40.0)]);
    let piece = &result.pieces[0];
    assert!(piece.feed_links.iter().any(|&l| l));
    assert!(!piece.feed_links.iter().all(|&l| l));
    for (i, &down) in piece.feed_links.iter().enumerate() {
        if down {
            assert!(distance(piece.rings[i].points[0], piece.rings[i + 1].points[0]) <= 6.0);
        }
    }
}

// -- spiral (docs/specs/pocket-spiral.md)

#[test]
fn inner_rings_step_out_onto_the_next_as_one_spiral() {
    let contours = [outline()];
    let refs: Vec<&Contour> = contours.iter().collect();
    // Rounded rings, as every project pocket has.
    let result = plan_pocket(&PocketRequest {
        choices: Choices {
            spiral: true,
            round_corners: true,
            ..Choices::default()
        },
        ..PocketRequest::new(&refs, 3.0, 2.4)
    })
    .unwrap();
    let piece = &result.pieces[0];
    let n = piece.rings.len();
    for (i, ring) in piece.rings.iter().enumerate() {
        if i + 1 < n {
            assert!(
                ring.steps_out && ring.open,
                "ring {i} steps out onto ring {}",
                i + 1
            );
            // It ends exactly where the next ring starts, so there is no link move.
            assert_eq!(ring.end(), Some(piece.rings[i + 1].points[0]));
            assert!(piece.feed_links[i]);
        } else {
            assert!(!ring.steps_out && !ring.open, "the wall ring closes");
        }
    }
    assert!(result.notes.is_empty(), "{:?}", result.notes);
    assert_clear(&contours, &result, 3.0);
    // The step-out is gentle: no chord of a spiral turns more than an arc's chords do.
    for ring in &piece.rings[..n - 1] {
        for w in ring.points.windows(3) {
            let u = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let v = (w[2].0 - w[1].0, w[2].1 - w[1].1);
            let turn = (u.0 * v.1 - u.1 * v.0).atan2(u.0 * v.0 + u.1 * v.1).abs();
            assert!(turn < 0.5, "turn of {turn} rad at {:?}", w[1]);
        }
    }
    // One plunge, one continuous path, one retract per depth.
    let moves = moves_of(piece, &[-3.0]);
    let plunges = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed && m.z.is_some())
        .count();
    let retracts = moves.iter().filter(|m| m.kind == MoveKind::Rapid).count();
    assert_eq!(
        (plunges, retracts),
        (1, 3),
        "rapid to safe, to the start, up at the end"
    );
    // No zero-length move where a ring lands on the next.
    for w in moves.windows(2) {
        if let (Some(ax), Some(ay), Some(bx), Some(by)) = (w[0].x, w[0].y, w[1].x, w[1].y) {
            assert!(
                distance((ax, ay), (bx, by)) > 0.0 || w[1].z.is_some(),
                "{:?}",
                w[1]
            );
        }
    }
}

// -- one island at a time (docs/specs/pocket-island-order.md)

/// A rectangle with a circle and a square island, each farther than a tool diameter from
/// the wall and from each other at every level.
fn two_islands() -> [Contour; 3] {
    [
        outline(),
        circle_id(2, (25.0, 30.0), 7.0),
        rect(3, 60.0, 23.0, 74.0, 37.0),
    ]
}

/// Which island an island ring goes round (its points' mean nearest), None for an outer
/// ring. Island rings run clockwise for climb with M3.
fn island_of(ring: &[Point]) -> Option<usize> {
    if signed_area(ring) >= 0.0 {
        return None;
    }
    let n = ring.len() as f64;
    let c = (
        ring.iter().map(|p| p.0).sum::<f64>() / n,
        ring.iter().map(|p| p.1).sum::<f64>() / n,
    );
    Some(if distance(c, (25.0, 30.0)) < distance(c, (67.0, 30.0)) {
        2
    } else {
        3
    })
}

#[test]
fn each_islands_rings_are_cut_one_after_another() {
    let contours = two_islands();
    let result = plan(&contours);
    let piece = &result.pieces[0];
    let islands: Vec<Option<usize>> = piece.rings.iter().map(|r| island_of(&r.points)).collect();
    for island in [2, 3] {
        let at: Vec<usize> = (0..islands.len())
            .filter(|&i| islands[i] == Some(island))
            .collect();
        assert!(at.len() > 2, "island {island}: {islands:?}");
        assert!(
            at.windows(2).all(|w| w[1] == w[0] + 1),
            "island {island}'s rings one after another: {islands:?}"
        );
    }
    let last = piece.rings.last().unwrap();
    assert!(
        last.wall && islands.last() == Some(&None),
        "the wall ring is last"
    );
    // Lifts only between the separate deep lobes (the corners and between the islands)
    // and the chains, not between every ring: 7, where cutting level by level lifted 14.
    let lifts = piece.feed_links.iter().filter(|f| !**f).count();
    assert!(lifts <= 7, "{lifts} lifts: {:?}", piece.feed_links);
    assert_clear(&contours, &result, 3.0);
}

#[test]
fn island_rings_spiral_out_to_the_island() {
    let contours = two_islands();
    let result = plan(&contours);
    let rings = &result.pieces[0].rings;
    for island in [2, 3] {
        let at: Vec<usize> = (0..rings.len())
            .filter(|&i| island_of(&rings[i].points) == Some(island))
            .collect();
        let (&edge, inner) = at.split_last().unwrap();
        assert!(
            rings[edge].wall && !rings[edge].steps_out,
            "the island's edge ring closes"
        );
        for &i in inner {
            assert!(rings[i].steps_out, "island {island}: ring {i} steps out");
            assert_eq!(rings[i].end(), Some(rings[i + 1].points[0]));
        }
    }
    // The wall ring never steps out.
    assert!(!rings.last().unwrap().steps_out);
    assert_clear(&contours, &result, 3.0);
}

/// The order's one rule (docs/specs/pocket-island-order.md §2), checked from the rings
/// alone: where a ring is farther than `gap` from every ring cut before it, no deeper ring
/// (more stepovers from the walls) cut after it comes within `gap` of it. So a ring never
/// meets the material of a deeper ring that isn't cut yet.
fn assert_deeper_rings_first(contours: &[Contour], result: &PocketPlan, offset: f64, s: f64) {
    let walls: Vec<Primitive> = contours
        .iter()
        .flat_map(|c| pline_segments(&contour_pline(c)))
        .collect();
    let to_walls = |p: Point| {
        walls
            .iter()
            .map(|w| point_distance(w, p))
            .fold(f64::INFINITY, f64::min)
    };
    let chords = |points: &[Point]| -> Vec<Primitive> {
        points
            .windows(2)
            .map(|w| {
                Primitive::Line(chipload_core::primitives::Line {
                    start: w[0],
                    end: w[1],
                })
            })
            .collect()
    };
    let near = |path: &[Primitive], p: Point| {
        path.iter()
            .map(|c| point_distance(c, p))
            .fold(f64::INFINITY, f64::min)
    };
    let gap = 1.2 * s;
    for piece in &result.pieces {
        // Each ring's level: its points' median distance from the walls, in stepovers.
        let levels: Vec<i64> = piece
            .rings
            .iter()
            .map(|r| {
                let mut d: Vec<f64> = r.points.iter().map(|&p| to_walls(p)).collect();
                d.sort_by(f64::total_cmp);
                ((d[d.len() / 2] - offset) / s).round() as i64
            })
            .collect();
        let paths: Vec<Vec<Primitive>> = piece.rings.iter().map(|r| chords(&r.points)).collect();
        for (i, ring) in piece.rings.iter().enumerate() {
            let earlier: Vec<Primitive> = paths[..i].iter().flatten().cloned().collect();
            for p in sampled(&ring.points, 1.0) {
                if near(&earlier, p) <= gap {
                    continue;
                }
                for j in i + 1..piece.rings.len() {
                    if levels[j] > levels[i] {
                        let d = near(&paths[j], p);
                        assert!(
                            d > gap,
                            "ring {i} (level {}) at {p:?} meets ring {j} (level {}, {d:.2} \
                             away), cut after it",
                            levels[i],
                            levels[j]
                        );
                    }
                }
            }
        }
    }
}

/// Points along a path, at most `step` apart.
fn sampled(points: &[Point], step: f64) -> Vec<Point> {
    let mut out = Vec::new();
    for w in points.windows(2) {
        let n = (distance(w[0], w[1]) / step).ceil().max(1.0) as usize;
        for k in 0..n {
            let t = k as f64 / n as f64;
            out.push((
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            ));
        }
    }
    out
}

#[test]
fn deeper_rings_are_cut_first_round_islands_and_in_a_neck() {
    let islands = two_islands();
    assert_deeper_rings_first(&islands, &plan(&islands), 3.0, 2.4);
    // An island close to the wall: its rings and the wall's must alternate in the neck.
    let neck = [outline(), rect(2, 8.0, 20.0, 30.0, 40.0)];
    let result = plan(&neck);
    assert_deeper_rings_first(&neck, &result, 3.0, 2.4);
    assert_clear(&neck, &result, 3.0);
}

// -- moves

#[test]
fn moves_plunge_at_the_centre_for_every_depth() {
    let result = plan(&[outline()]);
    let piece = &result.pieces[0];
    let moves = moves_of(piece, &[-3.0, -6.0]);
    let plunges: Vec<_> = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed && m.z.is_some())
        .collect();
    // Every ring of a plain rectangle is linked: one plunge per depth.
    assert_eq!(plunges.len(), 2);
    assert!(plunges.iter().all(|m| m.feed == Some(400.0)));
    let last = moves.last().unwrap();
    assert!(last.kind == MoveKind::Rapid && last.z == Some(5.0));
}

#[test]
fn first_ring_starts_nearest_the_start_position() {
    let contours = [outline()];
    let refs: Vec<&Contour> = contours.iter().collect();
    let result = plan_pocket(&PocketRequest {
        start: (100.0, 60.0),
        ..PocketRequest::new(&refs, 3.0, 2.4)
    })
    .unwrap();
    let start = result.pieces[0].rings[0].points[0];
    assert!(start.0 > 50.0 && start.1 > 29.0, "{start:?}");
}

#[test]
fn pieces_are_cut_nearest_first() {
    let result = plan(&[
        rect(1, 200.0, 0.0, 250.0, 50.0),
        rect(2, 0.0, 0.0, 50.0, 50.0),
    ]);
    let ids: Vec<Vec<usize>> = result
        .pieces
        .iter()
        .map(|p| p.contour_ids.clone())
        .collect();
    assert_eq!(ids, [vec![2], vec![1]]);
}

#[test]
fn plan_is_deterministic() {
    let contours = [outline(), circle_id(2, (50.0, 30.0), 8.0)];
    assert_eq!(plan(&contours), plan(&contours));
}

// -- properties

/// A star-shaped closed polygon around (cx, cy), rounded to 1e-6 mm.
fn star(
    n: std::ops::Range<usize>,
    min_r: f64,
    max_r: f64,
    cx: f64,
    cy: f64,
) -> impl Strategy<Value = Vec<Point>> {
    n.prop_flat_map(move |n| {
        (
            prop::collection::btree_set(0u32..36_000, n),
            prop::collection::vec(min_r..max_r, n),
        )
            .prop_filter_map("needs n distinct angles", move |(angles, radii)| {
                (angles.len() == n).then(|| {
                    angles
                        .iter()
                        .zip(&radii)
                        .map(|(&a, &r)| {
                            let a = f64::from(a) / 36_000.0 * TAU;
                            let round = |v: f64| (v * 1e6).round() / 1e6;
                            (round(cx + r * a.cos()), round(cy + r * a.sin()))
                        })
                        .collect()
                })
            })
    })
}

fn pocket_with_islands() -> impl Strategy<Value = Vec<Contour>> {
    // Many vertices keep the boundary roughly round, so islands near the centre fit inside it.
    let islands = prop::collection::vec(
        (-12.0f64..12.0, -12.0f64..12.0).prop_flat_map(|(cx, cy)| star(3..16, 1.0, 4.0, cx, cy)),
        0..=3,
    );
    // Outlines that fold back on themselves are refused on import
    // (docs/specs/folded-contours.md), so they never reach pocketing.
    let outline = star(8..16, 30.0, 60.0, 0.0, 0.0)
        .prop_filter("folds back on itself", |ring| fold_point(ring).is_none());
    (outline, islands).prop_map(|(boundary, islands)| {
        let outer = polygon_id(1, &boundary);
        let outer_pline = contour_pline(&outer);
        let mut kept: Vec<Contour> = vec![outer];
        for points in islands {
            let island = polygon_id(kept.len() + 1, &points);
            let pline = contour_pline(&island);
            if pline.scan_for_self_intersect()
                || pline.area().abs() < 1.0
                || fold_point(&points).is_some()
            {
                continue;
            }
            // Keep 0.5 mm from the wall and from other islands, as the Python property does.
            let walls = pline_segments(&outer_pline);
            let near = |segs: &[Primitive]| {
                pline_segments(&pline).iter().any(|s| {
                    segs.iter()
                        .any(|w| segment_distance(s.start(), s.end(), w) < 0.5)
                })
            };
            let inside = outer_pline.winding_number(cavalier_contours::core::math::Vector2::new(
                points[0].0,
                points[0].1,
            )) != 0;
            let clashes = kept[1..]
                .iter()
                .any(|other| near(&pline_segments(&contour_pline(other))));
            if inside && !near(&walls) && !clashes {
                kept.push(island);
            }
        }
        kept
    })
}

proptest! {
    #![proptest_config(common::proptest_config(60))]

    #[test]
    fn pocket_never_gouges_and_clears_everything(
        contours in pocket_with_islands(),
        radius in 1.0f64..5.0,
        fraction in 0.1f64..0.5,
    ) {
        let refs: Vec<&Contour> = contours.iter().collect();
        let result = plan_pocket(&PocketRequest::new(&refs, radius, fraction * 2.0 * radius));
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &result, radius);
        prop_assert!(
            !result.notes.iter().any(|n| n.english().contains("uncut material")),
            "{:?}", result.notes
        );
    }
}

// -- clearance (docs/specs/depths-and-clearance.md)

fn with_clearance(contours: &[Contour], clearance: f64) -> PocketPlan {
    let refs: Vec<&Contour> = contours.iter().collect();
    plan_pocket(&PocketRequest {
        clearance,
        ..PocketRequest::new(&refs, 3.0, 2.4)
    })
    .unwrap()
}

/// (min x, min y, max x, max y) over every ring.
fn extent(result: &PocketPlan) -> (f64, f64, f64, f64) {
    result
        .pieces
        .iter()
        .flat_map(|p| p.rings.iter().flat_map(|r| &r.points))
        .fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(a, b, c, d), &(x, y)| (a.min(x), b.min(y), c.max(x), d.max(y)),
        )
}

#[test]
fn clearance_makes_the_pocket_larger_and_islands_smaller() {
    let island_centre = (50.0, 30.0);
    let contours = [outline(), circle_id(2, island_centre, 8.0)];
    // The ring nearest the island: its distance from the island's centre.
    let island_ring = |result: &PocketPlan| {
        result
            .pieces
            .iter()
            .flat_map(|p| p.rings.iter().flat_map(|r| &r.points))
            .map(|&p| distance(p, island_centre))
            .fold(f64::INFINITY, f64::min)
    };
    let none = with_clearance(&contours, 0.0);
    let more = with_clearance(&contours, 0.5);
    let less = with_clearance(&contours, -0.5);
    // Walls: tool centre 3 mm in, then 0.5 mm further out or in.
    let close = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
        (a.0 - b.0).abs() < 1e-6
            && (a.1 - b.1).abs() < 1e-6
            && (a.2 - b.2).abs() < 1e-6
            && (a.3 - b.3).abs() < 1e-6
    };
    assert!(
        close(extent(&none), (3.0, 3.0, 97.0, 57.0)),
        "{:?}",
        extent(&none)
    );
    assert!(
        close(extent(&more), (2.5, 2.5, 97.5, 57.5)),
        "{:?}",
        extent(&more)
    );
    assert!(
        close(extent(&less), (3.5, 3.5, 96.5, 56.5)),
        "{:?}",
        extent(&less)
    );
    // Island ring at 8 + 3 - c from its centre (chords lie inside the true circle).
    for (result, expected) in [(&none, 11.0), (&more, 10.5), (&less, 11.5)] {
        let d = island_ring(result);
        assert!((d - expected).abs() < 0.02, "{d} vs {expected}");
    }
    // No gouge beyond the clearance.
    assert_clear(&contours, &more, 2.5);
}

#[test]
fn clearance_must_be_less_than_the_tool_radius() {
    let refs = [&outline()];
    let err = plan_pocket(&PocketRequest {
        clearance: 3.0,
        ..PocketRequest::new(&refs, 3.0, 2.4)
    })
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("must be less than the tool radius"),
        "{err}"
    );
}

proptest! {
    #![proptest_config(common::proptest_config(30))]

    /// With clearance c the tool centre keeps r - c from every wall and island: the cut goes
    /// exactly c into the kept material, never further.
    #[test]
    fn clearance_never_cuts_further_than_asked(
        contours in pocket_with_islands(),
        radius in 1.0f64..5.0,
        share in -0.5f64..0.8,
        fraction in 0.1f64..0.5,
    ) {
        let clearance = share * radius;
        let refs: Vec<&Contour> = contours.iter().collect();
        let result = plan_pocket(&PocketRequest {
            clearance,
            ..PocketRequest::new(&refs, radius, fraction * 2.0 * radius)
        });
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &result, radius - clearance);
    }
}

proptest! {
    #![proptest_config(common::proptest_config(30))]

    /// docs/specs/arc-output.md: the rings of any pocket merge into arcs of at most a half
    /// circle whose ends lie on their circle, and merging never changes where lines go.
    #[test]
    fn pocket_rings_merge_into_valid_arcs(
        contours in pocket_with_islands(),
        radius in 1.0f64..5.0,
        fraction in 0.1f64..0.5,
    ) {
        use chipload_toolpath::arcs::{merge_arcs, tag_moves};
        use chipload_toolpath::model::{MoveKind, Program};
        let refs: Vec<&Contour> = contours.iter().collect();
        let Ok(plan) = plan_pocket(&PocketRequest::new(&refs, radius, fraction * 2.0 * radius)) else {
            return Ok(());
        };
        let cut = PocketCut {
            pass_z: &[-1.0], top_z: 0.0, safe_z: 5.0, feed: 1000.0, plunge_feed: 300.0,
            entry: Entry::Plunge {},
        };
        let mut moves = Vec::new();
        for piece in &plan.pieces {
            moves.extend(pocket_moves(piece, &cut).unwrap());
        }
        tag_moves(&mut moves, &plan.arcs);
        let program = Program {
            tool_id: 1, tool_diameter: 2.0 * radius, rpm: 18000.0, spindle: Spindle::Cw,
            safe_z: 5.0, moves,
            comments: Vec::new(),
        };
        let merged = merge_arcs(&program).unwrap();
        let (mut x, mut y) = (0.0, 0.0);
        let mut arcs = 0;
        for m in &merged.moves {
            let (nx, ny) = (m.x.unwrap_or(x), m.y.unwrap_or(y));
            if matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw) {
                let c = m.circle.unwrap();
                for p in [(x, y), (nx, ny)] {
                    prop_assert!((distance(p, c.center) - c.radius).abs() < 1e-6);
                }
                // At most a half circle: the chord is at least as long as the sagitta allows.
                let (ax, ay) = (x - c.center.0, y - c.center.1);
                let (bx, by) = (nx - c.center.0, ny - c.center.1);
                let sweep = (ax * by - ay * bx).atan2(ax * bx + ay * by);
                let cw = m.kind == MoveKind::ArcCw;
                prop_assert!(sweep.abs() <= std::f64::consts::PI + 1e-9);
                // The direction matches, except at exactly a half circle, where the short-way
                // angle can't tell clockwise from counter-clockwise.
                let half = sweep.abs() > std::f64::consts::PI - 1e-6;
                prop_assert!(sweep == 0.0 || (sweep < 0.0) == cw || half);
                arcs += 1;
            }
            (x, y) = (nx, ny);
        }
        // Rings round an island's corners are arcs (a polygon turns at least 45° at one of
        // its at most 8 corners). A convex outline without islands shrinks to sharp corners
        // only, so it may have none.
        prop_assert!(arcs > 0 || contours.len() == 1);
    }
}

proptest! {
    #![proptest_config(common::proptest_config(40))]

    /// Leads and moved starts (docs/specs/helix-and-leads.md parts A and B) never gouge
    /// either: every chord, leads included, keeps the tool radius from walls and islands.
    #[test]
    fn pockets_with_leads_and_starts_never_gouge(
        contours in pocket_with_islands(),
        radius in 1.0f64..5.0,
        fraction in 0.1f64..0.5,
        lead in 0.5f64..8.0,
        starts in prop::collection::vec((-60.0f64..60.0, -60.0f64..60.0), 0..3),
    ) {
        let refs: Vec<&Contour> = contours.iter().collect();
        let result = plan_pocket(&PocketRequest {
            choices: Choices {
                starts: &starts,
                lead: Some(lead),
                ..Choices::default()
            },
            ..PocketRequest::new(&refs, radius, fraction * 2.0 * radius)
        });
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &result, radius);
        for piece in &result.pieces {
            for ring in &piece.rings {
                if let Some(range) = &ring.lead {
                    prop_assert!(ring.wall && ring.open);
                    prop_assert!(range.start > 0 && range.end < ring.points.len());
                }
            }
        }
    }
}

/// A rest pocket whose areas' offsets put an island across an area at the small tool's
/// radius and the next 3 nm, but not 10 nm on (docs/specs/offset-retry.md §7; the rest
/// machining property test's minimal case, 2026-10-13).
#[test]
fn a_rest_pocket_offset_failing_for_a_few_nanometres_is_retried_farther() {
    let contours = vec![
        polygon_id(
            1,
            &[
                (33.595689, 0.252138),
                (57.688623, 1.309138),
                (36.454244, 25.449755),
                (-36.677502, 31.793465),
                (-46.573885, 20.697082),
                (-40.873847, -10.03958),
                (-47.235603, -25.263857),
                (12.941601, -48.638113),
                (20.771191, -25.269273),
            ],
        ),
        polygon_id(
            2,
            &[
                (-2.421503, -4.78619),
                (-2.984589, -4.932658),
                (-1.412154, -3.60878),
                (-3.884336, -4.056389),
                (-4.498733, -4.183733),
                (-7.992499, -3.737546),
                (-5.481322, -6.63786),
            ],
        ),
        polygon_id(
            3,
            &[
                (6.682977, 4.830564),
                (4.340816, 4.525362),
                (4.686365, 4.94042),
                (3.969669, 4.637116),
                (4.656649, 5.433751),
                (4.429437, 7.469242),
                (0.274271, 6.55713),
                (-0.527002, 4.962413),
                (-0.438116, 3.590918),
                (1.663206, 2.843283),
                (2.648945, 2.109079),
                (4.391235, 0.610591),
            ],
        ),
        polygon_id(
            4,
            &[
                (-2.710797, -11.325537),
                (-4.753909, -10.203427),
                (-8.011285, -10.463343),
                (-4.782148, -11.887069),
            ],
        ),
    ];
    let (radius, times, fraction) = (2.447143409889017, 1.8999644467026537, 0.3796415167916837);
    let refs: Vec<&Contour> = contours.iter().collect();
    let result = plan_pocket(&PocketRequest {
        choices: Choices {
            rest: Some(times * radius),
            ..Choices::default()
        },
        ..PocketRequest::new(&refs, radius, fraction * 2.0 * radius)
    })
    .unwrap();
    assert_clear(&contours, &result, radius);
}

/// A pocket whose rings failed verification with every 1 nm nudge and the tight tolerance,
/// and passes with a 10 nm one (docs/specs/offset-retry.md §7; the pocket property test's
/// minimal case, 2026-10-13).
#[test]
fn a_pocket_failing_for_a_few_nanometres_is_retried_farther() {
    let contours = vec![
        polygon_id(
            1,
            &[
                (47.283164, 0.0),
                (53.889635, 0.028217),
                (33.657884, 16.642072),
                (27.535261, 41.054136),
                (-4.164544, 55.386662),
                (-9.665888, 35.748941),
                (-38.375347, 7.278848),
                (-26.303816, -45.140099),
                (-12.363046, -52.545086),
                (10.982632, -47.495362),
                (43.162672, -20.108745),
            ],
        ),
        polygon_id(
            2,
            &[
                (12.241347, 1.209205),
                (13.022205, 1.471434),
                (12.503767, 4.378144),
                (11.513291, 4.637524),
                (10.677452, 2.284448),
                (10.058932, 2.513899),
                (7.646611, 3.098123),
                (8.405537, 2.410719),
                (8.737981, 0.66109),
                (11.841748, -2.036709),
                (11.798304, 0.159574),
                (11.892798, 0.319051),
                (13.454511, -0.001826),
            ],
        ),
        polygon_id(
            3,
            &[
                (-8.190897, -3.867558),
                (-9.315381, -4.512003),
                (-10.260768, -3.041486),
                (-10.9834, -3.554382),
                (-12.062186, -3.248025),
                (-11.704315, -5.070747),
                (-11.809718, -5.896462),
                (-12.912228, -7.495292),
                (-12.033023, -7.069513),
                (-11.419557, -7.44209),
                (-10.67087, -7.738487),
                (-10.463305, -8.387925),
                (-8.391145, -8.140723),
                (-8.460356, -7.085073),
                (-8.530779, -6.353565),
            ],
        ),
    ];
    let (radius, fraction) = (3.8034814349297092, 0.4429968779362151);
    let refs: Vec<&Contour> = contours.iter().collect();
    let result = plan_pocket(&PocketRequest::new(&refs, radius, fraction * 2.0 * radius)).unwrap();
    assert_clear(&contours, &result, radius);
}

proptest! {
    #![proptest_config(common::proptest_config(40))]

    /// Rest machining (docs/specs/rest-machining.md §8): the rest pieces keep the small
    /// tool's radius from every wall and island, and clear what they reach.
    #[test]
    fn rest_pockets_never_gouge(
        contours in pocket_with_islands(),
        radius in 1.0f64..3.0,
        times in 1.5f64..4.0,
        fraction in 0.1f64..0.5,
    ) {
        let refs: Vec<&Contour> = contours.iter().collect();
        let result = plan_pocket(&PocketRequest {
            choices: Choices {
                rest: Some(times * radius),
                ..Choices::default()
            },
            ..PocketRequest::new(&refs, radius, fraction * 2.0 * radius)
        });
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &result, radius);
        prop_assert!(
            !result.notes.iter().any(|n| n.english().contains("uncut material")),
            "{:?}", result.notes
        );
    }
}

proptest! {
    #![proptest_config(common::proptest_config(40))]

    /// A helix (docs/specs/helix-and-leads.md part C), and the feed from it to the first
    /// path, keep the tool radius from every wall and island.
    #[test]
    fn pockets_with_a_helix_never_gouge(
        contours in pocket_with_islands(),
        radius in 1.0f64..5.0,
        fraction in 0.1f64..0.5,
        helix in 0.1f64..0.95,
    ) {
        let refs: Vec<&Contour> = contours.iter().collect();
        let result = plan_pocket(&PocketRequest {
            choices: Choices {
                helix: Some(helix * radius),
                ..Choices::default()
            },
            ..PocketRequest::new(&refs, radius, fraction * 2.0 * radius)
        });
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &result, radius);
        let walls: Vec<Primitive> = contours
            .iter()
            .flat_map(|c| pline_segments(&contour_pline(c)))
            .collect();
        let clearance = radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM;
        for piece in &result.pieces {
            let Some(h) = piece.helix else { continue };
            prop_assert!(h.circle.radius <= helix * radius + 1e-12);
            let mut path = h.points();
            path.push(piece.rings[0].points[0]);
            for w in path.windows(2) {
                for wall in &walls {
                    let d = segment_distance(w[0], w[1], wall);
                    prop_assert!(d >= clearance, "helix chord {:?} is {} from a wall", w, d);
                }
            }
        }
    }
}
