// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::Path;

use chipload_job::load_job;

use super::*;

fn drawn_part() -> Job {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/jobs/v5/drawn_part.chipload");
    load_job(&path).unwrap()
}

fn geometry(job: &Job, id: u64) -> &Geometry {
    &job.drawing
        .shapes
        .iter()
        .find(|s| s.id == id)
        .unwrap()
        .geometry
}

#[test]
fn offsets_keep_the_kind_where_exact_and_go_on_the_layer() {
    // S1 rectangle 120 x 80, S2 circle r 12, S3 a triangle, all on layer "drawn".
    let job = drawn_part();
    let edited = offset_shapes(&job, &[1, 2, 3], 2.0, None, Corners::Round, 1).unwrap();
    assert_eq!(edited.new_ids, [4, 5, 6]);
    assert!(matches!(
        geometry(&edited.job, 4),
        Geometry::Rectangle { length, width, corner_radius, .. }
            if *length == 124.0 && *width == 84.0 && *corner_radius == 2.0
    ));
    assert!(matches!(geometry(&edited.job, 5), Geometry::Circle { radius, .. } if *radius == 14.0));
    assert!(matches!(
        geometry(&edited.job, 6),
        Geometry::Polyline { closed: true, .. }
    ));
    // The originals stay.
    assert_eq!(geometry(&edited.job, 1), geometry(&job, 1));
    // Sharp: the rectangle keeps sharp corners; inside, twice.
    let sharp = offset_shapes(&job, &[1], 2.0, None, Corners::Sharp, 1).unwrap();
    assert!(matches!(
        geometry(&sharp.job, 4),
        Geometry::Rectangle { corner_radius, .. } if *corner_radius == 0.0
    ));
    let rings =
        offset_shapes(&job, &[2], 5.0, Some(OffsetSide::Inside), Corners::Round, 2).unwrap();
    assert!(matches!(geometry(&rings.job, 5), Geometry::Circle { radius, .. } if *radius == 2.0));
    // A third ring would vanish: refused, and nothing is added.
    assert!(offset_shapes(&job, &[2], 5.0, Some(OffsetSide::Inside), Corners::Round, 3).is_err());
}

#[test]
fn corners_are_filleted_and_chamfered_keeping_the_id() {
    // S1: the turned 120 x 80 rectangle; S3: the triangle.
    let job = drawn_part();
    let round = cut_shape_corners(&job, 1, None, Cut::Fillet(10.0)).unwrap();
    assert!(matches!(
        geometry(&round, 1),
        Geometry::Rectangle { corner_radius, .. } if *corner_radius == 10.0
    ));
    let one = cut_shape_corners(&job, 1, Some(2), Cut::Chamfer(5.0)).unwrap();
    assert!(matches!(geometry(&one, 1), Geometry::Polyline { points, .. } if points.len() == 5));
    // A polyline's corners get their radius, its points staying at the sharp corners
    // (docs/specs/polyline-radii.md decision 3).
    let triangle = cut_shape_corners(&job, 3, None, Cut::Fillet(2.0)).unwrap();
    assert!(matches!(
        geometry(&triangle, 3),
        Geometry::Polyline { points, radii, .. } if points.len() == 3 && radii == &[2.0, 2.0, 2.0]
    ));
    // Chamfering it then cuts only a sharp corner; Shift has none left to cut.
    let one = cut_shape_corners(&triangle, 3, Some(1), Cut::Chamfer(1.0)).unwrap();
    assert!(matches!(
        geometry(&one, 3),
        Geometry::Polyline { points, radii, .. } if points.len() == 4 && radii == &[2.0, 0.0, 0.0, 2.0]
    ));
    assert!(cut_shape_corners(&triangle, 3, None, Cut::Chamfer(1.0)).is_err());
    // Too large: refused with the most that fits; a circle has no corners.
    let err = cut_shape_corners(&job, 3, None, Cut::Fillet(50.0)).unwrap_err();
    assert!(
        err.message().english().contains("at most"),
        "{}",
        err.message().english()
    );
    assert!(cut_shape_corners(&job, 2, None, Cut::Fillet(1.0)).is_err());
    assert_eq!(
        shape_corners(job.drawing.shapes.iter().find(|s| s.id == 3).unwrap(), true).len(),
        3
    );
}

#[test]
fn operations_that_select_the_layer_are_named() {
    let mut job = drawn_part();
    if let Some(select) = job.operations[0].select_mut() {
        select.layer = Some("Drawn".into());
        select.shapes = None;
    }
    let edited = offset_shapes(&job, &[2], 1.0, None, Corners::Round, 1).unwrap();
    assert_eq!(edited.notes.len(), 1);
    let text = edited.notes[0].render(chipload_i18n::Language::En, Default::default());
    assert!(text.contains("operation 1"), "{text}");
}

#[test]
fn trimming_a_shape_keeps_its_id_for_the_first_piece() {
    // A polyline across the circle S2 (r 12 at (75, 80)): trimmed inside the circle,
    // it becomes two, and the second is a new shape on its layer.
    let job = drawn_part();
    let (job, id) = add_shape(
        &job,
        "drawn",
        Geometry::straight_polyline(false, vec![[50.0, 80.0], [100.0, 80.0]]),
    )
    .unwrap();
    let edited = trim_shape(&job, id, (75.0, 80.0)).unwrap();
    assert_eq!(edited.new_ids.len(), 1);
    let Geometry::Polyline { points, .. } = geometry(&edited.job, id) else {
        panic!()
    };
    assert_eq!(points.len(), 2);
    assert!((points[1][0] - 63.0).abs() < 1e-9);
    // The circle trimmed by the line: an open arc, same id.
    let arc = trim_shape(&job, 2, (75.0, 92.0)).unwrap();
    assert!(matches!(
        geometry(&arc.job, 2),
        Geometry::Polyline { closed: false, .. }
    ));
    // Nothing crosses the triangle S3 (the line ends at x 100, before it): refused.
    assert!(trim_shape(&job, 3, (125.0, 95.0)).is_err());
    // Extend the line's end to the triangle, and break it.
    let longer = extend_shape(&job, id, End::End).unwrap();
    let Geometry::Polyline { points, .. } = geometry(&longer.job, id) else {
        panic!()
    };
    assert!(points[1][0] > 100.0 && points[1][0] < 125.0);
    let broken = break_shape(&job, id, (60.0, 80.0)).unwrap();
    assert_eq!(broken.new_ids.len(), 1);
}

/// A slot cut by a line (the user, 2026-10-29): half the slot trimmed, then the line's
/// two ends; the half slot and the line between its ends become one closed polyline.
#[test]
fn a_trim_joins_pieces_whose_ends_meet() {
    let slot = Geometry::Rectangle {
        center: [300.0, 100.0],
        length: 80.0,
        width: 20.0,
        angle: 0.0,
        corner_radius: 10.0,
        radii: Vec::new(),
    };
    let line = Geometry::straight_polyline(false, vec![[300.0, 70.0], [300.0, 130.0]]);
    let (job, s) = add_shape(&drawn_part(), "cut", slot).unwrap();
    let (job, l) = add_shape(&job, "cut", line.clone()).unwrap();
    let half = trim_shape(&job, s, (340.0, 100.0)).unwrap();
    assert_eq!(half.joined, None, "the line's ends still stick out");
    // The line's lower end goes: what is left of it meets the half slot at (300, 90).
    let one = trim_shape(&half.job, l, (300.0, 75.0)).unwrap();
    assert_eq!(one.joined, Some(s));
    assert!(!ids(&one.job).contains(&l));
    assert!(matches!(
        geometry(&one.job, s),
        Geometry::Polyline { closed: false, .. }
    ));
    // Its upper end goes: the shape's own ends meet, and it closes.
    let closed = trim_shape(&one.job, s, (300.0, 125.0)).unwrap();
    assert_eq!(closed.joined, Some(s));
    let Geometry::Polyline {
        closed: true,
        points,
        ..
    } = geometry(&closed.job, s)
    else {
        panic!("{:?}", geometry(&closed.job, s))
    };
    assert_eq!(points.len(), 5);

    // Only shapes on the trimmed shape's layer, never construction geometry, and
    // nothing that merely touches elsewhere.
    for (layer, construction) in [("other", false), ("cut", true)] {
        let (job, s) = add_shape(&drawn_part(), "cut", slot_again()).unwrap();
        let (mut job, l) = add_shape(&job, layer, line.clone()).unwrap();
        if let Some(shape) = job.drawing.shapes.iter_mut().find(|x| x.id == l) {
            shape.construction = construction;
        }
        let half = trim_shape(&job, s, (340.0, 100.0)).unwrap();
        let cut = trim_shape(&half.job, l, (300.0, 75.0)).unwrap();
        assert_eq!(cut.joined, None, "{layer} {construction}");
        assert!(ids(&cut.job).contains(&l) && ids(&cut.job).contains(&s));
    }
    let (job, a) = add_shape(
        &job,
        "cut",
        Geometry::straight_polyline(false, vec![[0.0, 0.0], [10.0, 0.0]]),
    )
    .unwrap();
    let (job, b) = add_shape(
        &job,
        "cut",
        Geometry::straight_polyline(false, vec![[10.0, 0.0], [10.0, 10.0]]),
    )
    .unwrap();
    let half = trim_shape(&job, s, (340.0, 100.0)).unwrap();
    assert!(ids(&half.job).contains(&a) && ids(&half.job).contains(&b));
}

fn slot_again() -> Geometry {
    Geometry::Rectangle {
        center: [300.0, 100.0],
        length: 80.0,
        width: 20.0,
        angle: 0.0,
        corner_radius: 10.0,
        radii: Vec::new(),
    }
}

fn ids(job: &Job) -> Vec<u64> {
    job.drawing.shapes.iter().map(|s| s.id).collect()
}

#[test]
fn holes_cut_where_they_lie_leave_every_shape_as_drawn() {
    // S2 and S3 lie inside S1: cut out of it they are its holes, as they were.
    let job = drawn_part();
    let edited = combine_shapes(&job, Combine::Subtract, &[1, 2, 3]).unwrap();
    assert_eq!(edited.job.drawing.shapes, job.drawing.shapes);
    assert!(edited.new_ids.is_empty() && edited.notes.is_empty());
}

#[test]
fn a_union_keeps_the_first_shapes_id_and_flags_the_others_operations() {
    let job = drawn_part();
    let (job, s4) = add_shape(
        &job,
        "drawn",
        Geometry::Circle {
            center: [87.0, 80.0],
            radius: 5.0,
        },
    )
    .unwrap();
    let edited = combine_shapes(&job, Combine::Union, &[s4, 2]).unwrap();
    assert_eq!(ids(&edited.job), [1, 3, s4]);
    assert!(matches!(
        geometry(&edited.job, s4),
        Geometry::Polyline { closed: true, .. }
    ));
    // Operation 1 profiled S2 (and S3).
    assert_eq!(edited.notes.len(), 1);
    assert_eq!(edited.notes[0].id, "pipeline-shape-gone");
}

#[test]
fn an_intersection_that_is_one_of_the_shapes_keeps_that_shape() {
    // S2 lies inside S1: their intersection is S2, unchanged; S1 goes, and operation 2
    // that profiled it is flagged.
    let job = drawn_part();
    let edited = combine_shapes(&job, Combine::Intersect, &[1, 2]).unwrap();
    assert_eq!(ids(&edited.job), [2, 3]);
    assert_eq!(geometry(&edited.job, 2), geometry(&job, 2));
    assert_eq!(edited.notes.len(), 1);
}

#[test]
fn combining_refuses_open_and_twice_given_shapes() {
    let job = drawn_part();
    let (job, open) = add_shape(
        &job,
        "drawn",
        Geometry::Polyline {
            closed: false,
            points: vec![[0.0, 0.0], [10.0, 0.0]],
            bulges: vec![0.0],
            radii: Vec::new(),
        },
    )
    .unwrap();
    let key = |r: Result<Edited, PipelineError>| match r {
        Err(PipelineError::Job(m)) => m.id,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        key(combine_shapes(&job, Combine::Union, &[1, open])),
        "pipeline-combine-open"
    );
    assert_eq!(
        key(combine_shapes(&job, Combine::Union, &[1, 1])),
        "pipeline-shape-twice"
    );
    assert_eq!(
        key(combine_shapes(&job, Combine::Union, &[1])),
        "pipeline-combine-two"
    );
    assert_eq!(
        key(combine_shapes(&job, Combine::Subtract, &[2, 1])),
        "pipeline-combine-empty"
    );
}

#[test]
fn a_grid_array_copies_on_the_layer_in_columns_and_rows() {
    let job = drawn_part();
    let edited = array_grid(&job, &[2], 3, 2, (30.0, -40.0)).unwrap();
    assert_eq!(edited.new_ids, [4, 5, 6, 7, 8]);
    let centres: Vec<[f64; 2]> = edited
        .new_ids
        .iter()
        .map(|&id| match geometry(&edited.job, id) {
            Geometry::Circle { center, .. } => *center,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        centres,
        [
            [105.0, 80.0],
            [135.0, 80.0],
            [75.0, 40.0],
            [105.0, 40.0],
            [135.0, 40.0]
        ]
    );
    assert!(array_grid(&job, &[2], 1, 1, (30.0, 0.0)).is_err());
    assert!(array_grid(&job, &[2], 3, 1, (0.0, 10.0)).is_err());
    assert!(array_grid(&job, &[2], 200, 200, (1.0, 1.0)).is_err());
}

#[test]
fn a_linear_array_steps_in_any_direction() {
    let job = drawn_part();
    let edited = array_linear(&job, &[2], 3, (30.0, 40.0)).unwrap();
    assert_eq!(edited.new_ids, [4, 5]);
    let centres: Vec<[f64; 2]> = edited
        .new_ids
        .iter()
        .map(|&id| match geometry(&edited.job, id) {
            Geometry::Circle { center, .. } => *center,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(centres, [[105.0, 120.0], [135.0, 160.0]]);
    assert!(array_linear(&job, &[2], 1, (30.0, 0.0)).is_err());
    assert!(array_linear(&job, &[2], 3, (0.0, 0.0)).is_err());
    assert!(array_linear(&job, &[2], ARRAY_MOST + 1, (1.0, 1.0)).is_err());
}

#[test]
fn a_polar_array_of_a_whole_turn_has_no_copy_on_the_original() {
    let job = drawn_part();
    let polar = Polar {
        count: 4,
        center: (100.0, 80.0),
        angle_deg: 360.0,
        rotate: true,
    };
    let edited = array_polar(&job, &[2], polar).unwrap();
    let centres: Vec<(f64, f64)> = edited
        .new_ids
        .iter()
        .map(|&id| match geometry(&edited.job, id) {
            Geometry::Circle { center, .. } => (
                (center[0] * 1e6).round() / 1e6,
                (center[1] * 1e6).round() / 1e6,
            ),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(centres, [(100.0, 55.0), (125.0, 80.0), (100.0, 105.0)]);
    // A sweep: the first and last at its ends.
    let half = array_polar(
        &job,
        &[2],
        Polar {
            count: 3,
            angle_deg: 180.0,
            ..polar
        },
    )
    .unwrap();
    assert!(matches!(geometry(&half.job, half.new_ids[1]),
        Geometry::Circle { center, .. } if (center[0] - 125.0).abs() < 1e-9 && (center[1] - 80.0).abs() < 1e-9));
    assert!(array_polar(&job, &[2], Polar { count: 1, ..polar }).is_err());
    assert!(
        array_polar(
            &job,
            &[2],
            Polar {
                angle_deg: 0.0,
                ..polar
            }
        )
        .is_err()
    );
}

#[test]
fn a_polar_array_turns_its_copies_unless_asked_not_to() {
    // S1, a rectangle at 10°, copied a quarter turn round its own centre.
    let job = drawn_part();
    let polar = Polar {
        count: 2,
        center: (100.0, 80.0),
        angle_deg: 90.0,
        rotate: true,
    };
    let turned = array_polar(&job, &[1], polar).unwrap();
    assert!(matches!(geometry(&turned.job, 4),
        Geometry::Rectangle { angle, .. } if (angle.to_degrees() - 100.0).abs() < 1e-9));
    let moved = array_polar(
        &job,
        &[1],
        Polar {
            rotate: false,
            ..polar
        },
    )
    .unwrap();
    assert!(matches!(geometry(&moved.job, 4),
        Geometry::Rectangle { angle, .. } if (angle.to_degrees() - 10.0).abs() < 1e-9));
}

fn bounds_of(job: &Job, id: u64) -> ((f64, f64), (f64, f64)) {
    crate::shapes::shapes_bounds(job, &[id]).unwrap()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn shapes_align_to_their_box_or_the_stock() {
    // S2 a circle at 75,80 r 12; S3 a triangle from 110,65 to 140,95.
    let job = drawn_part();
    let left = align_shapes(&job, &[2, 3], AlignTo::Left, AlignWith::All).unwrap();
    assert!(near(bounds_of(&left, 3).0.0, 63.0));
    assert!(near(bounds_of(&left, 2).0.0, 63.0));
    let top = align_shapes(&job, &[2, 3], AlignTo::Top, AlignWith::All).unwrap();
    assert!(near(bounds_of(&top, 2).1.1, 95.0));
    let stock = job.stock_bounds();
    let middle = align_shapes(&job, &[2], AlignTo::Centre, AlignWith::Stock).unwrap();
    assert!(near(
        (bounds_of(&middle, 2).0.0 + bounds_of(&middle, 2).1.0) / 2.0,
        (stock.0.0 + stock.1.0) / 2.0
    ));
    assert!(align_shapes(&job, &[2], AlignTo::Left, AlignWith::All).is_err());
}

#[test]
fn distributing_evens_the_gaps_and_keeps_the_ends() {
    let job = drawn_part();
    let (job, a) = add_shape(
        &job,
        "drawn",
        Geometry::Circle {
            center: [0.0, 0.0],
            radius: 5.0,
        },
    )
    .unwrap();
    let (job, b) = add_shape(
        &job,
        "drawn",
        Geometry::Circle {
            center: [12.0, 0.0],
            radius: 5.0,
        },
    )
    .unwrap();
    let (job, c) = add_shape(
        &job,
        "drawn",
        Geometry::Circle {
            center: [100.0, 0.0],
            radius: 10.0,
        },
    )
    .unwrap();
    let out = distribute_shapes(&job, &[c, a, b], true).unwrap();
    // From -5 to 110, 40 mm of circles: two gaps of 37.5.
    assert!(near(bounds_of(&out, a).0.0, -5.0));
    assert!(near(bounds_of(&out, b).0.0, 42.5));
    assert!(near(bounds_of(&out, c).0.0, 90.0));
    assert!(distribute_shapes(&job, &[a, b], true).is_err());
}

// -- mirror about a line (docs/specs/drawing-improvements.md §1)

#[test]
fn shapes_mirror_about_any_line_in_place_or_as_copies() {
    use crate::shapes::mirror_shapes_about;
    let job = drawn_part();
    let before: Vec<Geometry> = [1, 2, 3]
        .iter()
        .map(|&id| geometry(&job, id).clone())
        .collect();
    // About the vertical line x = 0, in place: ids kept, each geometry the job crate's.
    let (edited, joined) =
        mirror_shapes_about(&job, &[1, 2, 3], [0.0, 0.0], [0.0, 1.0], false).unwrap();
    assert!(edited.new_ids.is_empty() && joined.is_empty());
    for (id, old) in [1, 2, 3].iter().zip(&before) {
        let want = chipload_job::edit::mirrored_about(old, [0.0, 0.0], [0.0, 1.0]).unwrap();
        assert_eq!(geometry(&edited.job, *id), &want);
    }
    // Twice is the shape again, exactly, about a vertical line.
    let (again, _) =
        mirror_shapes_about(&edited.job, &[1, 2, 3], [0.0, 0.0], [0.0, 1.0], false).unwrap();
    for (id, old) in [1, 2, 3].iter().zip(&before) {
        assert_eq!(geometry(&again.job, *id), old);
    }
    // As copies about a slanted line: new shapes on the same layer, the originals stay.
    let (copied, joined) =
        mirror_shapes_about(&job, &[1, 2], [0.0, 0.0], [1.0, 2.0], true).unwrap();
    assert_eq!(copied.new_ids.len(), 2);
    assert!(joined.is_empty());
    assert_eq!(geometry(&copied.job, 1), &before[0]);
    // A line of one point is refused.
    assert!(mirror_shapes_about(&job, &[1], [3.0, 3.0], [3.0, 3.0], true).is_err());
}

#[test]
fn a_half_outline_mirrored_against_its_centre_line_becomes_one_closed_shape() {
    use crate::shapes::{add_shape, mirror_shapes_about};
    let job = drawn_part();
    // Half a bottle against x = 300: up from the line, out and round, back to the line.
    let half = Geometry::Polyline {
        closed: false,
        points: vec![[300.0, 0.0], [320.0, 0.0], [320.0, 40.0], [300.0, 60.0]],
        bulges: vec![0.0, 0.3, 0.0],
        radii: Vec::new(),
    };
    let (job, id) = add_shape(&job, "drawn", half).unwrap();
    let curve = Geometry::Spline {
        closed: false,
        points: vec![[300.0, 100.0], [330.0, 120.0], [300.0, 150.0]],
    };
    let (job, spline) = add_shape(&job, "drawn", curve).unwrap();
    let (edited, joined) =
        mirror_shapes_about(&job, &[id, spline], [300.0, 0.0], [300.0, 10.0], true).unwrap();
    assert_eq!(joined, [id, spline]);
    assert!(edited.new_ids.is_empty());
    assert_eq!(edited.notes.len(), 1);
    let Geometry::Polyline {
        closed: true,
        points,
        bulges,
        ..
    } = geometry(&edited.job, id)
    else {
        panic!("one closed polyline");
    };
    assert_eq!(
        points,
        &[
            [300.0, 0.0],
            [320.0, 0.0],
            [320.0, 40.0],
            [300.0, 60.0],
            [280.0, 40.0],
            [280.0, 0.0]
        ]
    );
    // The arc on the way back bends the mirrored way round: the same bulge, run backwards.
    assert_eq!(bulges, &[0.0, 0.3, 0.0, 0.0, 0.3, 0.0]);
    let Geometry::Spline {
        closed: true,
        points,
    } = geometry(&edited.job, spline)
    else {
        panic!("one closed spline");
    };
    assert_eq!(points.len(), 4);
    // The whole shape is symmetric: its box is centred on the line.
    let ((x0, _), (x1, _)) = crate::shapes::drawn_box(&edited.job, &[id]).unwrap();
    assert!(((x0 + x1) / 2.0 - 300.0).abs() < 1e-9);
}

#[test]
fn a_rectangle_or_polygon_corner_gets_its_own_radius() {
    // docs/specs/polyline-radii.md §4a. S1: the turned 120 x 80 rectangle.
    let job = drawn_part();
    let one = cut_shape_corners(&job, 1, Some(2), Cut::Fillet(10.0)).unwrap();
    assert!(matches!(
        geometry(&one, 1),
        Geometry::Rectangle { corner_radius, radii, .. }
            if *corner_radius == 0.0 && radii == &[0.0, 0.0, 10.0, 0.0]
    ));
    // Every corner alike again: back to its one corner radius.
    let all = cut_shape_corners(&one, 1, None, Cut::Fillet(4.0)).unwrap();
    assert!(matches!(
        geometry(&all, 1),
        Geometry::Rectangle { corner_radius, radii, .. } if *corner_radius == 4.0 && radii.is_empty()
    ));
    // Too large: refused with the most that fits.
    let err = cut_shape_corners(&job, 1, Some(0), Cut::Fillet(90.0)).unwrap_err();
    assert!(
        err.message().english().contains("at most 80"),
        "{}",
        err.message().english()
    );
    // Chamfering a rounded rectangle's sharp corner keeps the round one.
    let cut = cut_shape_corners(&one, 1, Some(0), Cut::Chamfer(2.0)).unwrap();
    assert!(matches!(
        geometry(&cut, 1),
        Geometry::Polyline { radii, .. } if radii.iter().filter(|&&r| r == 10.0).count() == 1
    ));
}

#[test]
fn a_polyline_corner_beside_an_arc_is_rounded_and_made_sharp_again() {
    // docs/specs/fillet-between-shapes.md §4: a bracket, a line into an arc at point 1.
    let mut job = drawn_part();
    let mut bracket = job.drawing.shapes[2].clone();
    bracket.id = 9;
    bracket.geometry = Geometry::Polyline {
        closed: true,
        points: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
        bulges: vec![0.0, 0.5, 0.0, 0.0],
        radii: Vec::new(),
    };
    job.drawing.shapes.push(bracket);
    job.drawing.next_shape_id = 10;
    let corners: Vec<usize> = shape_corners(job.drawing.shapes.last().unwrap(), true)
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    assert_eq!(corners, [0, 1, 2, 3]);
    // A chamfer still only cuts between straight sides.
    let straight: Vec<usize> = shape_corners(job.drawing.shapes.last().unwrap(), false)
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    assert_eq!(straight, [0, 3]);
    let round = cut_shape_corners(&job, 9, Some(1), Cut::Fillet(3.0)).unwrap();
    assert!(matches!(
        geometry(&round, 9),
        Geometry::Polyline { radii, .. } if radii == &[0.0, 3.0, 0.0, 0.0]
    ));
    // Too large beside the arc: refused with the most that fits.
    let err = cut_shape_corners(&job, 9, Some(1), Cut::Fillet(500.0)).unwrap_err();
    assert!(
        err.message().english().contains("at most"),
        "{}",
        err.message().english()
    );
    // Radius 0: sharp again.
    let sharp = cut_shape_corners(&round, 9, Some(1), Cut::Fillet(0.0)).unwrap();
    assert!(matches!(
        geometry(&sharp, 9),
        Geometry::Polyline { radii, .. } if radii.iter().all(|&r| r == 0.0)
    ));
    assert!(cut_shape_corners(&job, 9, Some(1), Cut::Chamfer(0.0)).is_err());
}

/// An open polyline to add: its id, points, bulges and layer.
type OpenShape<'a> = (u64, &'a [(f64, f64)], &'a [f64], &'a str);

/// `drawn_part` with open polylines added.
fn with_open(shapes: &[OpenShape]) -> Job {
    let mut job = drawn_part();
    for &(id, points, bulges, layer) in shapes {
        let mut shape = job.drawing.shapes[2].clone();
        shape.id = id;
        shape.layer = layer.to_string();
        shape.geometry = Geometry::Polyline {
            closed: false,
            points: points.iter().map(|p| [p.0, p.1]).collect(),
            bulges: bulges.to_vec(),
            radii: Vec::new(),
        };
        job.drawing.shapes.push(shape);
        job.drawing.next_shape_id = job.drawing.next_shape_id.max(id + 1);
    }
    job
}

fn polyline(job: &Job, id: u64) -> (Vec<[f64; 2]>, Vec<f64>, Vec<f64>) {
    match geometry(job, id) {
        Geometry::Polyline {
            points,
            bulges,
            radii,
            ..
        } => (points.clone(), bulges.clone(), radii.clone()),
        other => panic!("a polyline: {other:?}"),
    }
}

fn at(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

const ACROSS: &[(f64, f64)] = &[(-10.0, 0.0), (10.0, 0.0)];
const UP: &[(f64, f64)] = &[(0.0, -10.0), (0.0, 10.0)];

#[test]
fn two_crossing_lines_are_trimmed_to_a_round_kept_as_the_corners_radius() {
    // docs/specs/fillet-between-shapes.md §5.
    let job = with_open(&[(10, ACROSS, &[0.0], "drawn"), (11, UP, &[0.0], "drawn")]);
    let edited = fillet_between(
        &job,
        (10, (8.0, 0.0)),
        (11, (0.0, 8.0)),
        Between::Fillet(3.0),
    )
    .unwrap();
    assert_eq!(edited.joined, Some(10));
    assert!(edited.job.drawing.shapes.iter().all(|s| s.id != 11));
    let (points, bulges, radii) = polyline(&edited.job, 10);
    assert_eq!(points.len(), 3, "{points:?}");
    assert!(at(points[1], [0.0, 0.0]), "{points:?}");
    assert_eq!(bulges, [0.0, 0.0]);
    assert_eq!(radii, [0.0, 3.0, 0.0]);
    let ends = [points[0], points[2]];
    assert!(ends.iter().any(|&p| at(p, [10.0, 0.0])));
    assert!(ends.iter().any(|&p| at(p, [0.0, 10.0])));
}

#[test]
fn lines_that_fall_short_are_extended_and_radius_zero_meets_sharp() {
    let job = with_open(&[
        (10, &[(-10.0, 0.0), (-2.0, 0.0)], &[0.0], "drawn"),
        (11, &[(0.0, 2.0), (0.0, 10.0)], &[0.0], "drawn"),
    ]);
    let round = fillet_between(
        &job,
        (10, (-8.0, 0.0)),
        (11, (0.0, 8.0)),
        Between::Fillet(1.0),
    )
    .unwrap();
    let (points, _, radii) = polyline(&round.job, 10);
    assert_eq!(points.len(), 3);
    assert!(
        at(points[1], [0.0, 0.0]) && radii[1] == 1.0,
        "{points:?} {radii:?}"
    );
    let sharp = fillet_between(
        &job,
        (10, (-8.0, 0.0)),
        (11, (0.0, 8.0)),
        Between::Fillet(0.0),
    )
    .unwrap();
    let (points, _, radii) = polyline(&sharp.job, 10);
    assert_eq!(points.len(), 3);
    assert!(
        at(points[1], [0.0, 0.0]) && radii.is_empty(),
        "{points:?} {radii:?}"
    );
}

#[test]
fn on_different_layers_the_round_joins_only_the_first() {
    // The round goes on the first shape's layer: it joins that one, the other stays apart.
    let job = with_open(&[(10, ACROSS, &[0.0], "drawn"), (11, UP, &[0.0], "other")]);
    let edited = fillet_between(
        &job,
        (10, (8.0, 0.0)),
        (11, (0.0, 8.0)),
        Between::Fillet(3.0),
    )
    .unwrap();
    assert_eq!(edited.joined, Some(10));
    assert!(edited.new_ids.is_empty(), "{:?}", edited.new_ids);
    let (one, bulges, _) = polyline(&edited.job, 10);
    assert_eq!(one.len(), 3, "{one:?}");
    assert!(at(one[1], [3.0, 0.0]) || at(one[1], [0.0, 3.0]), "{one:?}");
    assert_eq!(bulges.iter().filter(|&&b| b != 0.0).count(), 1);
    // The other kept on its clicked side, ending where the round touches it.
    let (two, _, _) = polyline(&edited.job, 11);
    assert!(at(two[0], [0.0, 3.0]) || at(two[1], [0.0, 3.0]), "{two:?}");
    assert!(two.iter().any(|&p| at(p, [0.0, 10.0])));
}

#[test]
fn a_line_and_an_arc_are_joined_by_a_tangent_round() {
    // An arc above the line, bulging down towards it (its circle's centre near (0, 30),
    // its lowest point 12 above the line).
    let job = with_open(&[
        (10, &[(-20.0, 0.0), (20.0, 0.0)], &[0.0], "drawn"),
        (11, &[(-10.0, 15.0), (10.0, 15.0)], &[0.3], "drawn"),
    ]);
    let edited = fillet_between(
        &job,
        (10, (15.0, 0.0)),
        (11, (9.0, 14.0)),
        Between::Fillet(8.0),
    )
    .unwrap();
    let kept = edited.joined.unwrap();
    let shape = edited
        .job
        .drawing
        .shapes
        .iter()
        .find(|s| s.id == kept)
        .unwrap();
    let rounds: Vec<f64> = shape
        .segments()
        .iter()
        .filter_map(|s| match s.to_primitive() {
            chipload_core::primitives::Primitive::Arc(a) => Some(a.radius),
            _ => None,
        })
        .collect();
    assert!(rounds.iter().any(|r| (r - 8.0).abs() < 1e-6), "{rounds:?}");
    // Too small to reach across the gap of 12: no round, and none smaller fits either.
    let none = fillet_between(
        &job,
        (10, (15.0, 0.0)),
        (11, (9.0, 14.0)),
        Between::Fillet(2.0),
    )
    .unwrap_err()
    .message()
    .english();
    assert!(none.contains("no round of 2"), "{none}");
}

#[test]
fn chamfers_join_lines_and_the_refusals_say_why() {
    let job = with_open(&[
        (10, ACROSS, &[0.0], "drawn"),
        (11, UP, &[0.0], "drawn"),
        (12, &[(-10.0, 4.0), (10.0, 4.0)], &[0.0], "drawn"),
        (13, &[(-10.0, 15.0), (10.0, 15.0)], &[0.3], "drawn"),
        (
            14,
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (20.0, 10.0)],
            &[0.0; 3],
            "drawn",
        ),
        (15, &[(0.0, 20.0), (30.0, 20.0)], &[0.0], "drawn"),
    ]);
    let cut = fillet_between(
        &job,
        (10, (8.0, 0.0)),
        (11, (0.0, 8.0)),
        Between::Chamfer(2.0),
    )
    .unwrap();
    let (points, _, _) = polyline(&cut.job, 10);
    assert_eq!(points.len(), 4, "{points:?}");
    assert!(points.iter().any(|&p| at(p, [2.0, 0.0])));
    assert!(points.iter().any(|&p| at(p, [0.0, 2.0])));
    let english = |r: Result<Edited, PipelineError>| r.unwrap_err().message().english();
    let same = english(fillet_between(
        &job,
        (10, (8.0, 0.0)),
        (10, (0.0, 0.0)),
        Between::Fillet(1.0),
    ));
    assert!(same.contains("two different"), "{same}");
    let closed = english(fillet_between(
        &job,
        (1, (0.0, 0.0)),
        (10, (0.0, 0.0)),
        Between::Fillet(1.0),
    ));
    assert!(closed.contains("S1 isn't an open"), "{closed}");
    let parallel = english(fillet_between(
        &job,
        (10, (8.0, 0.0)),
        (12, (8.0, 4.0)),
        Between::Fillet(1.0),
    ));
    assert!(parallel.contains("parallel"), "{parallel}");
    let arc = english(fillet_between(
        &job,
        (10, (8.0, 0.0)),
        (13, (9.0, 14.0)),
        Between::Chamfer(1.0),
    ));
    assert!(arc.contains("S13") && arc.contains("arc"), "{arc}");
    let middle = english(fillet_between(
        &job,
        (14, (10.0, 5.0)),
        (15, (25.0, 20.0)),
        Between::Fillet(2.0),
    ));
    assert!(middle.contains("middle side of S14"), "{middle}");
    // Too large for the lines' halves kept: the most that fits is 10.
    let large = english(fillet_between(
        &job,
        (10, (8.0, 0.0)),
        (11, (0.0, 8.0)),
        Between::Fillet(50.0),
    ));
    assert!(
        large.contains("at most 10.000") || large.contains("at most 9.99"),
        "{large}"
    );
}
