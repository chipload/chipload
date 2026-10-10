// SPDX-License-Identifier: GPL-3.0-or-later
//! Pocket planning: unit tests and the no-gouge / clears-everything property.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::f64::consts::TAU;

use cavalier_contours::polyline::PlineSource;
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::fold::fold_point;
use chipload_core::primitives::Line;
use chipload_core::primitives::{Primitive, distance, signed_area};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, OFFSET_CHECK_SLACK_MM, REST_TOL_MM};
use chipload_job::{Direction, Entry, Spindle};
use chipload_toolpath::ToolpathError;
use chipload_toolpath::geom::PrimIndex;
use chipload_toolpath::geom::segment_distance;
use chipload_toolpath::left::{LeftKind, LeftPiece};
use chipload_toolpath::model::MoveKind;
use chipload_toolpath::offset::{contour_pline, pline_segments, point_distance};
use chipload_toolpath::pocket::{PocketPlan, build_region, plan_pocket, pocket_moves};
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
    keep: bool,
) -> Result<PocketPlan, ToolpathError> {
    let refs: Vec<&Contour> = contours.iter().collect();
    plan_pocket(
        &refs,
        3.0,
        0.0,
        stepover,
        direction,
        spindle,
        keep,
        (0.0, 0.0),
    )
}

fn plan(contours: &[Contour], keep: bool) -> PocketPlan {
    plan_with(contours, 2.4, Direction::Climb, Spindle::Cw, keep).unwrap()
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
    let result = plan(&contours, false);
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
    let result = plan(&[outline()], false);
    let piece = &result.pieces[0];
    // Level 0 is 94 x 54; each level shrinks 2.4 per side until the 54 side is gone: 12 levels.
    assert_eq!(piece.rings.len(), 12);
    let (x0, y0, x1, y1) = bbox(piece.rings.last().unwrap());
    assert!(
        [(x0, 3.0), (y0, 3.0), (x1, 97.0), (y1, 57.0)]
            .iter()
            .all(|(a, b)| (a - b).abs() < 1e-9)
    );
    let areas: Vec<f64> = piece.rings.iter().map(|r| signed_area(r).abs()).collect();
    assert!(areas.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn tool_too_big_is_an_error() {
    let err = plan_with(
        &[rect(7, 0.0, 0.0, 5.0, 5.0)],
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
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
    let result = plan(&[dumbbell], false);
    assert!(result.notes.iter().any(|n| n.contains("narrow part")));
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
        let result = plan_with(&contours, 2.4, direction, spindle, false).unwrap();
        for ring in &result.pieces[0].rings {
            let n = ring.len();
            let (a, b) = (0..n)
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
        for (k, ring) in piece.rings.iter().enumerate() {
            let n = ring.len();
            // Open paths (adaptive passes) don't go back to their start.
            let chords = if piece.open[k] {
                n.saturating_sub(1)
            } else {
                n
            };
            for i in 0..chords {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
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
    assert_clear(&contours, &plan(&contours, false), 3.0);
}

// -- uncut material

#[test]
fn large_stepover_reports_uncut_material() {
    // stepover = D: corners are missed.
    let result = plan_with(&[outline()], 6.0, Direction::Climb, Spindle::Cw, false).unwrap();
    assert!(result.notes.iter().any(|n| n.contains("uncut material")));
}

#[test]
fn normal_stepover_leaves_nothing() {
    assert!(plan(&[outline()], false).notes.is_empty());
}

// -- material left (docs/specs/material-left.md)

/// Even-odd point in rings, as the GUI fills them.
fn in_rings(rings: &[Vec<Point>], p: Point) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0) {
                inside = !inside;
            }
        }
    }
    inside
}

fn of_kind(result: &PocketPlan, kind: LeftKind) -> Vec<&LeftPiece> {
    result.left.iter().filter(|m| m.kind == kind).collect()
}

#[test]
fn sharp_corners_are_unreachable_and_nothing_is_missed() {
    let result = plan(&[outline()], false);
    assert_eq!(of_kind(&result, LeftKind::Unreachable).len(), 4);
    assert!(of_kind(&result, LeftKind::Missed).is_empty());
}

#[test]
fn crossing_pockets_in_one_operation_clear_each_others_corners() {
    // A T: the crossbar's end corners lie inside the upright, which clears them.
    let contours = [
        rect(1, 0.0, 0.0, 20.0, 60.0),
        rect(2, 10.0, 40.0, 80.0, 50.0),
    ];
    let result = plan(&contours, false);
    let unreachable = of_kind(&result, LeftKind::Unreachable);
    // Upright: 4 corners; crossbar: only its 2 right-hand corners.
    assert_eq!(unreachable.len(), 6);
    assert!(!unreachable.iter().any(|m| in_rings(&m.rings, (10.3, 40.3))));
    assert!(unreachable.iter().any(|m| in_rings(&m.rings, (79.7, 40.3))));
}

#[test]
fn missed_material_is_shown_where_the_warning_says() {
    let result = plan_with(&[outline()], 6.0, Direction::Climb, Spindle::Cw, false).unwrap();
    let note = result
        .notes
        .iter()
        .find(|n| n.contains("uncut material"))
        .unwrap();
    // "... near (x, y); reduce the stepover ..."
    let near = note.split("near (").nth(1).unwrap();
    let (x, rest) = near.split_once(", ").unwrap();
    let y = rest.split_once(')').unwrap().0;
    let at: Point = (x.parse().unwrap(), y.parse().unwrap());
    let missed = of_kind(&result, LeftKind::Missed);
    assert!(!missed.is_empty());
    // The note rounds to 0.1 mm, so look within that.
    let hit = missed.iter().any(|m| {
        [
            (0.0, 0.0),
            (0.05, 0.05),
            (-0.05, -0.05),
            (0.05, -0.05),
            (-0.05, 0.05),
        ]
        .iter()
        .any(|(dx, dy)| in_rings(&m.rings, (at.0 + dx, at.1 + dy)))
    });
    assert!(hit, "{note}");
}

#[test]
fn a_slot_narrower_than_the_tool_is_unreachable_and_still_warned() {
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
    let result = plan(&[room], false);
    assert!(result.notes.iter().any(|n| n.contains("narrow part")));
    let unreachable = of_kind(&result, LeftKind::Unreachable);
    assert!(unreachable.iter().any(|m| in_rings(&m.rings, (50.0, 20.0))));
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
    let result = plan_pocket(
        &refs,
        1.0,
        0.0,
        1.0,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
    )
    .unwrap();
    assert!(result.notes.is_empty(), "{:?}", result.notes);
}

#[test]
#[ignore = "known: a near-tangent ring level fails verification (docs/specs/offset-retry.md)"]
fn a_pocket_whose_island_offsets_just_touch_can_be_cleared() {
    // Found by the property test on 2026-10-10. At this exact stepover the offsets of islands
    // #2 and #4 meet almost exactly tangentially at one ring level; cavalier_contours then
    // asserts (debug) or returns rings that fail Chipload's verification (release): a safe
    // refusal of a valid drawing. The same pocket with the stepover 1e-6 mm smaller or larger
    // works, and without island #2 or #4 it works too.
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
    let result = plan_pocket(
        &refs,
        radius,
        0.0,
        0.18893806468913993 * 2.0 * radius,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
    )
    .unwrap();
    assert_clear(&contours, &result, radius);
}

// -- links

#[test]
fn default_always_retracts_between_rings() {
    assert!(
        !plan(&[outline()], false).pieces[0]
            .feed_links
            .iter()
            .any(|&l| l)
    );
}

#[test]
fn keep_tool_down_feeds_short_safe_links() {
    assert!(
        plan(&[outline()], true).pieces[0]
            .feed_links
            .iter()
            .all(|&l| l)
    );
}

#[test]
fn keep_tool_down_still_retracts_around_an_island() {
    let result = plan(&[outline(), rect(2, 30.0, 20.0, 70.0, 40.0)], true);
    let piece = &result.pieces[0];
    assert!(piece.feed_links.iter().any(|&l| l));
    assert!(!piece.feed_links.iter().all(|&l| l));
    for (i, &down) in piece.feed_links.iter().enumerate() {
        if down {
            assert!(distance(piece.rings[i][0], piece.rings[i + 1][0]) <= 6.0);
        }
    }
}

// -- moves

#[test]
fn moves_plunge_at_the_centre_for_every_depth() {
    let result = plan(&[outline()], false);
    let piece = &result.pieces[0];
    let moves = pocket_moves(
        piece,
        &[-3.0, -6.0],
        0.0,
        5.0,
        1500.0,
        400.0,
        Entry::Plunge {},
    )
    .unwrap();
    let plunges: Vec<_> = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed && m.z.is_some())
        .collect();
    // Retract mode: one plunge per ring per depth.
    assert_eq!(plunges.len(), 2 * piece.rings.len());
    assert!(plunges.iter().all(|m| m.feed == Some(400.0)));
    let last = moves.last().unwrap();
    assert!(last.kind == MoveKind::Rapid && last.z == Some(5.0));
}

#[test]
fn keep_tool_down_plunges_once_per_depth() {
    let result = plan(&[outline()], true);
    let moves = pocket_moves(
        &result.pieces[0],
        &[-3.0, -6.0],
        0.0,
        5.0,
        1500.0,
        400.0,
        Entry::Plunge {},
    )
    .unwrap();
    let plunges = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed && m.z.is_some())
        .count();
    assert_eq!(plunges, 2);
}

#[test]
fn first_ring_starts_nearest_the_start_position() {
    let contours = [outline()];
    let refs: Vec<&Contour> = contours.iter().collect();
    let result = plan_pocket(
        &refs,
        3.0,
        0.0,
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
        (100.0, 60.0),
    )
    .unwrap();
    let start = result.pieces[0].rings[0][0];
    assert!(start.0 > 50.0 && start.1 > 29.0, "{start:?}");
}

#[test]
fn pieces_are_cut_nearest_first() {
    let result = plan(
        &[
            rect(1, 200.0, 0.0, 250.0, 50.0),
            rect(2, 0.0, 0.0, 50.0, 50.0),
        ],
        false,
    );
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
    assert_eq!(plan(&contours, true), plan(&contours, true));
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
    #![proptest_config(ProptestConfig::with_cases(60))]

    #[test]
    fn pocket_never_gouges_and_clears_everything(
        contours in pocket_with_islands(),
        radius in 1.0f64..5.0,
        fraction in 0.1f64..0.5,
        keep in any::<bool>(),
    ) {
        let refs: Vec<&Contour> = contours.iter().collect();
        let result = plan_pocket(&refs, radius, 0.0, fraction * 2.0 * radius, Direction::Climb, Spindle::Cw, keep, (0.0, 0.0),
        );
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &result, radius);
        prop_assert!(
            !result.notes.iter().any(|n| n.contains("uncut material")),
            "{:?}", result.notes
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Unreachable material lies in the pocket, out of the tool's reach, and everything the
    /// tool leaves (beyond noise) is shown as unreachable or missed.
    #[test]
    fn material_left_is_out_of_reach_and_complete(
        contours in pocket_with_islands(),
        radius in 1.0f64..5.0,
        fraction in 0.1f64..0.5,
    ) {
        let refs: Vec<&Contour> = contours.iter().collect();
        let Ok(result) = plan_pocket(&refs, radius, 0.0, fraction * 2.0 * radius, Direction::Climb, Spindle::Cw, false,
            (0.0, 0.0),
        ) else {
            return Ok(());
        };
        prop_assert!(
            !result.notes.iter().any(|n| n.contains("could not compute")),
            "{:?}", result.notes
        );
        let plines: Vec<_> = contours.iter().map(contour_pline).collect();
        let in_region = |p: Point| {
            let wind = |i: usize| {
                plines[i].winding_number(cavalier_contours::core::math::Vector2::new(p.0, p.1))
                    != 0
            };
            wind(0) && !(1..plines.len()).any(wind)
        };
        let walls = PrimIndex::new(
            plines.iter().flat_map(pline_segments).collect(),
            radius,
        );
        let cuts = PrimIndex::new(
            result
                .pieces
                .iter()
                .flat_map(|piece| &piece.rings)
                .flat_map(|ring| {
                    let n = ring.len();
                    (0..n).map(move |i| Primitive::Line(Line { start: ring[i], end: ring[(i + 1) % n] }))
                })
                .collect(),
            radius,
        );
        let reach = radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM;
        for piece in &result.left {
            prop_assert_eq!(piece.kind, LeftKind::Unreachable);
            for &p in piece.rings.iter().flatten() {
                let on_wall = walls.any_within(p, ARC_APPROX_TOL_MM);
                prop_assert!(in_region(p) || on_wall, "{:?} outside the pocket", p);
                prop_assert!(!cuts.any_within(p, reach), "{:?} within reach", p);
            }
        }
        let step = 0.5;
        for i in -130..=130 {
            for j in -130..=130 {
                let p = (f64::from(i) * step, f64::from(j) * step);
                if !in_region(p) {
                    continue;
                }
                let shown = result.left.iter().any(|m| in_rings(&m.rings, p));
                if shown {
                    prop_assert!(!cuts.any_within(p, reach), "{:?} shown but within reach", p);
                } else {
                    prop_assert!(
                        cuts.any_within(p, radius + 2.0 * REST_TOL_MM),
                        "{:?} is left but not shown", p
                    );
                }
            }
        }
    }
}

// -- clearance (docs/specs/depths-and-clearance.md)

fn with_clearance(contours: &[Contour], clearance: f64) -> PocketPlan {
    let refs: Vec<&Contour> = contours.iter().collect();
    plan_pocket(
        &refs,
        3.0,
        clearance,
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
    )
    .unwrap()
}

/// (min x, min y, max x, max y) over every ring.
fn extent(result: &PocketPlan) -> (f64, f64, f64, f64) {
    result
        .pieces
        .iter()
        .flat_map(|p| p.rings.iter().flatten())
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
            .flat_map(|p| p.rings.iter().flatten())
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
    let err = plan_pocket(
        &refs,
        3.0,
        3.0,
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("must be less than the tool radius"),
        "{err}"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

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
        let result = plan_pocket(
            &refs, radius, clearance, fraction * 2.0 * radius, Direction::Climb, Spindle::Cw,
            false, (0.0, 0.0),
        );
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
    #![proptest_config(ProptestConfig::with_cases(30))]

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
        let Ok(plan) = plan_pocket(
            &refs, radius, 0.0, fraction * 2.0 * radius, Direction::Climb, Spindle::Cw, false,
            (0.0, 0.0),
        ) else {
            return Ok(());
        };
        let mut moves = Vec::new();
        for piece in &plan.pieces {
            moves.extend(pocket_moves(piece, &[-1.0], 0.0, 5.0, 1000.0, 300.0, Entry::Plunge {}).unwrap());
        }
        tag_moves(&mut moves, &plan.arcs);
        let program = Program {
            tool_id: 1, tool_diameter: 2.0 * radius, rpm: 18000.0, spindle: Spindle::Cw,
            safe_z: 5.0, moves,
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

// -- trochoidal clearing (docs/specs/trochoidal.md)

fn trochoidal(contours: &[Contour], radius: f64, loop_radius: f64, step: f64) -> PocketPlan {
    let refs: Vec<&Contour> = contours.iter().collect();
    chipload_toolpath::pocket::plan_pocket_with(
        &refs,
        radius,
        0.0,
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
        chipload_job::PocketStrategy::Trochoidal { loop_radius, step },
    )
    .unwrap()
}

/// Arc moves after merging the plan's rings (one depth) into G2/G3.
fn arc_moves(plan: &PocketPlan) -> usize {
    use chipload_toolpath::arcs::{merge_arcs, tag_moves};
    use chipload_toolpath::model::{MoveKind, Program};
    let mut moves = Vec::new();
    for piece in &plan.pieces {
        moves.extend(
            pocket_moves(piece, &[-1.0], 0.0, 5.0, 1000.0, 300.0, Entry::Plunge {}).unwrap(),
        );
    }
    tag_moves(&mut moves, &plan.arcs);
    let program = Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves,
    };
    merge_arcs(&program)
        .unwrap()
        .moves
        .iter()
        .filter(|m| matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw))
        .count()
}

#[test]
fn a_square_pocket_is_cleared_by_loops_then_a_wall_pass() {
    let contours = [outline()]; // 100 x 60
    let plan = trochoidal(&contours, 3.0, 1.5, 0.6);
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
    // Backbones 4.5 mm in, then every min(2 * 1.5 + 2.4, reach) mm, where the loops' reach is
    // sqrt(4.5^2 - 0.3^2) - 0.01 = 4.480 mm: six levels, perimeters 284, 248.2, 212.3,
    // 176.5, 140.6 and 104.8 mm, so 474 + 414 + 354 + 295 + 235 + 175 = 1947 loops of two
    // halves each.
    assert_eq!(arc_moves(&plan), 3894);
    // Seven paths: the six backbones' loops, then the wall pass, the same as a ring pocket's
    // last ring.
    let rings = &plan.pieces[0].rings;
    assert_eq!(rings.len(), 7);
    let ring_pocket = plan_with(&contours, 2.4, Direction::Climb, Spindle::Cw, false).unwrap();
    let wall = ring_pocket.pieces[0].rings.last().unwrap();
    let (mut a, mut b) = (rings[6].clone(), wall.clone());
    a.sort_by(|p, q| p.partial_cmp(q).unwrap());
    b.sort_by(|p, q| p.partial_cmp(q).unwrap());
    assert_eq!(a, b, "the wall pass is the ring pocket's wall ring");
    // Every loop keeps the tool radius from the walls.
    assert_clear(&contours, &plan, 3.0);
}

#[test]
fn loops_turn_by_climb_or_conventional_and_the_spindle() {
    use chipload_toolpath::profile::cut_clockwise;
    for direction in [Direction::Climb, Direction::Conventional] {
        for spindle in [Spindle::Cw, Spindle::Ccw] {
            let refs = [&outline()];
            let plan = chipload_toolpath::pocket::plan_pocket_with(
                &refs,
                3.0,
                0.0,
                2.4,
                direction,
                spindle,
                false,
                (0.0, 0.0),
                chipload_job::PocketStrategy::Trochoidal {
                    loop_radius: 1.5,
                    step: 0.6,
                },
            )
            .unwrap();
            // The first loop: from the path's start round to the start again.
            let path = &plan.pieces[0].rings[0];
            let back = path[1..].iter().position(|&p| p == path[0]).unwrap() + 1;
            let clockwise = signed_area(&path[..back]) < 0.0;
            let expected = cut_clockwise(chipload_job::Side::Inside, direction, spindle);
            assert_eq!(clockwise, expected, "{direction:?} {spindle:?}");
        }
    }
}

#[test]
fn loops_larger_than_the_tool_radius_are_refused() {
    let refs = [&outline()];
    let err = chipload_toolpath::pocket::plan_pocket_with(
        &refs,
        3.0,
        0.0,
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
        chipload_job::PocketStrategy::Trochoidal {
            loop_radius: 3.5,
            step: 1.0,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("uncut core"), "{err}");
}

#[test]
fn a_slot_too_narrow_for_loops_gets_only_the_wall_pass() {
    // 7 mm wide: a 6 mm tool fits, loops of 1.5 mm radius need more than 9 mm.
    let slot = [rect(1, 0.0, 0.0, 80.0, 7.0)];
    let plan = trochoidal(&slot, 3.0, 1.5, 0.6);
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("loops (radius 1.500 mm) don't fit")),
        "{:?}",
        plan.notes
    );
    assert_eq!(plan.pieces[0].rings.len(), 1, "only the wall pass");
    assert!(!plan.notes.iter().any(|n| n.contains("uncut")));
}

#[test]
fn ring_pockets_are_unchanged_by_the_strategy_setting() {
    let contours = [outline(), circle_id(2, (50.0, 30.0), 8.0)];
    let refs: Vec<&Contour> = contours.iter().collect();
    let with = chipload_toolpath::pocket::plan_pocket_with(
        &refs,
        3.0,
        0.0,
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
        chipload_job::PocketStrategy::Rings {},
    )
    .unwrap();
    assert_eq!(with, plan(&contours, false));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Trochoidal loops never come closer than the tool radius to a wall or island, and
    /// with the stepover kept at most half the tool no material is missed.
    #[test]
    fn trochoidal_never_gouges(
        contours in pocket_with_islands(),
        radius in 1.5f64..4.0,
        loop_share in 0.15f64..0.4,
        step_share in 0.2f64..1.0,
    ) {
        let loop_radius = loop_share * 2.0 * radius;
        let step = step_share * loop_radius;
        let refs: Vec<&Contour> = contours.iter().collect();
        // Outlines the offset library already mishandles for ring pockets (the known case
        // parked as docs/specs/offset-retry.md) aren't what this property is about.
        prop_assume!(
            plan_pocket(&refs, radius, 0.0, 0.4 * radius, Direction::Climb, Spindle::Cw, false,
                (0.0, 0.0)).is_ok()
        );
        let result = chipload_toolpath::pocket::plan_pocket_with(
            &refs, radius, 0.0, 0.4 * radius, Direction::Climb, Spindle::Cw, false,
            (0.0, 0.0),
            chipload_job::PocketStrategy::Trochoidal { loop_radius, step },
        );
        let plan = match result {
            Ok(plan) => plan,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &plan, radius);
        // Bands 2 * loop radius + 0.2 D apart overlap (each clears 2 * loop radius + D).
        prop_assert!(
            !plan.notes.iter().any(|n| n.contains("uncut material")),
            "{:?}", plan.notes
        );
        prop_assert!(arc_moves(&plan) > 0);
    }
}

// -- adaptive clearing (docs/specs/adaptive.md)

fn adaptive(contours: &[Contour], radius: f64, load: f64) -> PocketPlan {
    let refs: Vec<&Contour> = contours.iter().collect();
    chipload_toolpath::pocket::plan_pocket_with(
        &refs,
        radius,
        0.0,
        2.4,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
        chipload_job::PocketStrategy::Adaptive { load },
    )
    .unwrap()
}

/// (median, share within 50 %) of the engagement samples, as shares of the target.
fn engagement_stats(plan: &PocketPlan) -> (f64, f64) {
    let mut e = plan.engagement.clone();
    e.sort_by(f64::total_cmp);
    let median = e[e.len() / 2];
    let near = e.iter().filter(|&&x| (0.5..=1.5).contains(&x)).count() as f64 / e.len() as f64;
    (median, near)
}

#[test]
fn adaptive_clears_a_square_pocket_near_the_target_load() {
    let contours = [outline()];
    let plan = adaptive(&contours, 3.0, 0.1);
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
    assert_clear(&contours, &plan, 3.0);
    let (median, near) = engagement_stats(&plan);
    println!("square: median {median:.3}, within 50 % {near:.3}");
    // The last path is the wall pass, the ring pocket's wall ring; the others are open.
    let piece = &plan.pieces[0];
    assert!(!piece.open[piece.rings.len() - 1]);
    assert!(piece.open[..piece.rings.len() - 1].iter().all(|&o| o));
    // Deterministic.
    assert_eq!(adaptive(&contours, 3.0, 0.1), plan);
}

#[test]
fn adaptive_handles_islands_and_l_shapes() {
    let island = [outline(), circle_id(2, (50.0, 30.0), 8.0)];
    let l_shape = [polygon_id(
        1,
        &[
            (0.0, 0.0),
            (80.0, 0.0),
            (80.0, 30.0),
            (30.0, 30.0),
            (30.0, 70.0),
            (0.0, 70.0),
        ],
    )];
    for contours in [&island[..], &l_shape[..]] {
        let plan = adaptive(contours, 3.0, 0.1);
        assert!(plan.notes.is_empty(), "{:?}", plan.notes);
        assert_clear(contours, &plan, 3.0);
        let (median, near) = engagement_stats(&plan);
        println!("median {median:.3}, within 50 % {near:.3}");
        assert!((median - 1.0).abs() <= 0.2, "median {median}");
        assert!(near >= 0.9, "within 50 %: {near}");
    }
}

#[test]
fn adaptive_plans_a_large_pocket_in_reasonable_time() {
    // 200 x 150 mm with a 6 mm tool (docs/specs/adaptive.md section 6). The bound is loose
    // for debug builds; release is several times faster.
    let big = [rect(1, 0.0, 0.0, 200.0, 150.0)];
    let started = std::time::Instant::now();
    let plan = adaptive(&big, 3.0, 0.1);
    let took = started.elapsed();
    println!("200 x 150 mm: {took:?}, {} steps", plan.engagement.len());
    assert!(plan.notes.is_empty(), "{:?}", plan.notes);
    let limit = if cfg!(debug_assertions) { 30.0 } else { 2.0 };
    assert!(took.as_secs_f64() < limit, "{took:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(12))]

    /// Adaptive passes never come closer than the tool radius to a wall or island and leave
    /// no material behind (the uncut check runs on the real path).
    #[test]
    fn adaptive_never_gouges_or_leaves_material(
        contours in pocket_with_islands(),
        radius in 1.5f64..4.0,
        load in 0.05f64..0.3,
    ) {
        let refs: Vec<&Contour> = contours.iter().collect();
        // Outlines ring pockets already refuse (docs/specs/offset-retry.md) aren't the point.
        prop_assume!(
            plan_pocket(&refs, radius, 0.0, 0.4 * radius, Direction::Climb, Spindle::Cw, false,
                (0.0, 0.0)).is_ok()
        );
        let plan = chipload_toolpath::pocket::plan_pocket_with(
            &refs, radius, 0.0, 0.4 * radius, Direction::Climb, Spindle::Cw, false, (0.0, 0.0),
            chipload_job::PocketStrategy::Adaptive { load },
        );
        let plan = match plan {
            Ok(plan) => plan,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &plan, radius);
        prop_assert!(
            !plan.notes.iter().any(|n| n.contains("uncut material")),
            "{:?}", plan.notes
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

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
        let result = chipload_toolpath::pocket::plan_pocket_starting(
            &refs,
            radius,
            0.0,
            fraction * 2.0 * radius,
            Direction::Climb,
            Spindle::Cw,
            false,
            (0.0, 0.0),
            chipload_job::PocketStrategy::Rings {},
            &starts,
            Some(lead),
        );
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("does not fit"), "{}", err);
                return Ok(());
            }
        };
        assert_clear(&contours, &result, radius);
        for piece in &result.pieces {
            for (k, range) in piece.leads.iter().enumerate() {
                if let Some(range) = range {
                    prop_assert!(piece.walls[k] && piece.open[k]);
                    prop_assert!(range.start > 0 && range.end < piece.rings[k].len());
                }
            }
        }
    }
}
