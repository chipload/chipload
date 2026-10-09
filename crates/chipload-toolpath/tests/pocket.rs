// SPDX-License-Identifier: GPL-3.0-or-later
//! Port of tests/unit/test_pocket.py and tests/property/test_pocket_paths.py.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::f64::consts::TAU;

use cavalier_contours::polyline::PlineSource;
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Primitive, distance, signed_area};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, OFFSET_CHECK_SLACK_MM};
use chipload_job::{Direction, Spindle};
use chipload_toolpath::ToolpathError;
use chipload_toolpath::geom::segment_distance;
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
    plan_pocket(&refs, 3.0, stepover, direction, spindle, keep, (0.0, 0.0))
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
        for ring in &piece.rings {
            let n = ring.len();
            for i in 0..n {
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

#[test]
fn pocket_centre_is_cleared_where_geos_buffer_collapsed_too_early() {
    // The Python suite's GEOS false-empty case: must not leave the centre uncut here either.
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
        1.0,
        Direction::Climb,
        Spindle::Cw,
        false,
        (0.0, 0.0),
    )
    .unwrap();
    assert!(result.notes.is_empty(), "{:?}", result.notes);
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
    let moves = pocket_moves(piece, &[-3.0, -6.0], 5.0, 1500.0, 400.0);
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
    let moves = pocket_moves(&result.pieces[0], &[-3.0, -6.0], 5.0, 1500.0, 400.0);
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
    (star(8..16, 30.0, 60.0, 0.0, 0.0), islands).prop_map(|(boundary, islands)| {
        let outer = polygon_id(1, &boundary);
        let outer_pline = contour_pline(&outer);
        let mut kept: Vec<Contour> = vec![outer];
        for points in islands {
            let island = polygon_id(kept.len() + 1, &points);
            let pline = contour_pline(&island);
            if pline.scan_for_self_intersect() || pline.area().abs() < 1.0 {
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
        let result = plan_pocket(
            &refs, radius, fraction * 2.0 * radius, Direction::Climb, Spindle::Cw, keep, (0.0, 0.0),
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
