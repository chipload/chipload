// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::i18n::Language::En;
use chipload_core::primitives::Primitive;

#[test]
fn a_circle_is_its_centre_and_a_point_on_it() {
    let mut sketch = Sketch::new(Tool::Circle);
    assert_eq!(sketch.click((10.0, 20.0), false), Step::Continue);
    assert_eq!(
        sketch.click((13.0, 24.0), false),
        Step::Done(Geometry::Circle {
            center: [10.0, 20.0],
            radius: 5.0
        })
    );
}

#[test]
fn a_rectangle_is_two_opposite_corners_in_any_order() {
    let mut sketch = Sketch::new(Tool::Rectangle);
    sketch.click((40.0, 30.0), false);
    assert_eq!(
        sketch.click((10.0, 10.0), false),
        Step::Done(Geometry::Rectangle {
            center: [25.0, 20.0],
            length: 30.0,
            width: 20.0,
            angle: 0.0,
            corner_radius: 0.0,
        })
    );
    // A flat rectangle is refused, and drawing goes on.
    let mut flat = Sketch::new(Tool::Rectangle);
    flat.click((0.0, 0.0), false);
    assert!(matches!(
        flat.click((10.0, 0.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::Rectangle)))
    ));
    assert_eq!(flat.points.len(), 1);
}

#[test]
fn a_polyline_closes_on_its_first_point_or_finishes_open() {
    let mut sketch = Sketch::new(Tool::Polyline);
    assert!(matches!(sketch.finish(), Step::Ignored(_)));
    for p in [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)] {
        assert_eq!(sketch.click(p, false), Step::Continue);
    }
    // The second click of a double-click lands on the same point: ignored.
    assert_eq!(sketch.click((10.0, 5.0), false), Step::Ignored(None));
    let mut open = sketch.clone();
    assert_eq!(
        open.finish(),
        Step::Done(Geometry::Polyline {
            closed: false,
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]],
            bulges: vec![0.0, 0.0],
            radii: Vec::new(),
        })
    );
    assert!(matches!(
        sketch.click((0.0, 0.0), true),
        Step::Done(Geometry::Polyline { closed: true, .. })
    ));
    // With only two points, clicking the first one adds it instead (no 2-point loop).
    let mut short = Sketch::new(Tool::Polyline);
    short.click((0.0, 0.0), false);
    short.click((5.0, 0.0), false);
    assert_eq!(short.click((0.0, 0.0), true), Step::Continue);
    short.undo_point();
    assert_eq!(short.points, [(0.0, 0.0), (5.0, 0.0)]);
}

#[test]
fn ruler_ticks_sit_on_the_grid_and_number_its_stronger_lines() {
    // 2 px/mm: a tick every 5 mm, longer every 25, numbered every 50.
    let ticks = ruler_ticks(-12.0, 101.0, 2.0, Units::Mm);
    assert_eq!(ticks.first().unwrap().at, -10.0);
    assert_eq!(ticks.last().unwrap().at, 100.0);
    let numbered: Vec<_> = ticks.iter().filter_map(|t| t.label.clone()).collect();
    assert_eq!(numbered, ["0", "50", "100"]);
    let half: Vec<f64> = ticks
        .iter()
        .filter(|t| t.size == TickSize::Half)
        .map(|t| t.at)
        .collect();
    assert_eq!(half, [25.0, 75.0]);
    // Zoomed in: tenths, numbered every millimetre.
    let ticks = ruler_ticks(0.95, 2.05, 100.0, Units::Mm);
    let numbered: Vec<_> = ticks.iter().filter_map(|t| t.label.clone()).collect();
    assert_eq!(numbered, ["1", "2"]);
    // Inches: numbered in inches, every inch.
    let ticks = ruler_ticks(0.0, 3.0 * MM_PER_INCH, 4.0, Units::Inch);
    let numbered: Vec<_> = ticks.iter().filter_map(|t| t.label.clone()).collect();
    assert_eq!(numbered, ["0", "1", "2", "3"]);
}

#[test]
fn the_grid_follows_the_zoom_and_snaps_to_round_values() {
    assert_eq!(grid_step(2.0, Units::Mm), 5.0); // 2 px/mm: the whole part on screen
    assert_eq!(grid_step(10.0, Units::Mm), 1.0);
    assert_eq!(grid_step(20.0, Units::Mm), 0.5);
    assert_eq!(grid_step(100.0, Units::Mm), 0.1);
    assert_eq!(grid_step(10_000.0, Units::Mm), 0.01);
    assert_eq!(grid_step(0.01, Units::Mm), 50.0);
    assert_eq!(snap((0.31, -0.26), 0.1, Units::Mm), (0.3, -0.3));
    assert_eq!(snap((12.26, 7.74), 0.5, Units::Mm), (12.5, 7.5));
    assert_eq!(snap((12.6, -2.4), 5.0, Units::Mm), (15.0, 0.0));
    // Inches: a 1/8" grid at 4 px/mm (1/16" would be 6.35 px), lines each inch.
    let eighth = grid_step(4.0, Units::Inch);
    assert_eq!(eighth, 0.125 * 25.4);
    assert_eq!(grid_major_every(eighth, Units::Inch), 8);
    assert_eq!(grid_major_every(25.4, Units::Inch), 12);
    assert_eq!(grid_major_every(5.0, Units::Mm), 10);
    // 10.1 mm is 0.3976": the nearest 1/8" is 3/8" = 9.525 mm.
    assert_eq!(snap((10.1, -0.1), eighth, Units::Inch), (0.375 * 25.4, 0.0));
}

fn contour(id: usize, prims: Vec<Primitive>) -> Contour {
    use chipload_core::chain::{OrientedSegment, Segment};
    Contour {
        id,
        parts: prims
            .into_iter()
            .map(|prim| OrientedSegment {
                segment: Segment {
                    construction: false,
                    prim,
                    layer: "0".into(),
                    handle: String::new(),
                    source_type: String::new(),
                    shape: None,
                    from: None,
                },
                reversed: false,
            })
            .collect(),
        closed: false,
        invalid_reason: None,
        nesting_depth: None,
    }
}

#[test]
fn the_cursor_snaps_to_the_nearest_point_then_the_grid() {
    use chipload_core::primitives::{Arc, Line};
    let line = |a: Point, b: Point| Primitive::Line(Line { start: a, end: b });
    let contours = vec![
        contour(1, vec![line((0.0, 0.0), (10.0, 0.0))]),
        contour(
            2,
            vec![Primitive::Arc(Arc {
                center: (5.0, 0.0),
                radius: 5.0,
                start_angle: 0.0,
                sweep: std::f64::consts::PI,
            })],
        ),
    ];
    let snapper = snapper(&contours, &[]);
    let none = BTreeSet::new();
    let cursor = |at: Point, skip| Cursor {
        at,
        from: None,
        skip,
    };
    // 10 px per mm: 8 px is 0.8 mm.
    let at = |p: Point| snap_cursor(&cursor(p, &none), &snapper, 10.0, 1.0, Units::Mm, false);
    let (p, hit) = at((9.5, 0.3));
    assert_eq!(p, (10.0, 0.0));
    // Line 1 ends at (10, 0), and so does the arc of #2: the end of #1 wins (lowest id).
    assert_eq!(snap_name(&hit.unwrap(), &En), "end of #1");
    // The line's middle and the arc's centre are both (5, 0): the centre wins.
    assert_eq!(snap_name(&at((5.2, 0.1)).1.unwrap(), &En), "centre of #2");
    // The arc's top is its quadrant (and its middle: the quadrant wins).
    assert_eq!(snap_name(&at((5.0, 4.6)).1.unwrap(), &En), "quadrant of #2");
    // Near the line only: on it.
    assert_eq!(snap_name(&at((2.6, 0.3)).1.unwrap(), &En), "on #1");
    // Nothing within 0.8 mm: the grid.
    assert_eq!(at((2.6, 2.4)), ((3.0, 2.0), None));
    // Alt: exactly where the cursor is.
    assert_eq!(
        snap_cursor(
            &cursor((9.5, 0.3), &none),
            &snapper,
            10.0,
            1.0,
            Units::Mm,
            true
        ),
        ((9.5, 0.3), None)
    );
    // The dragged shape's own points are skipped.
    let skip: BTreeSet<usize> = [1].into();
    let (_, hit) = snap_cursor(
        &cursor((9.5, 0.3), &skip),
        &snapper,
        10.0,
        1.0,
        Units::Mm,
        false,
    );
    assert_eq!(snap_name(&hit.unwrap(), &En), "end of #2");
    // Zoomed out, the same distance is within reach of more.
    let far = cursor((8.0, 2.0), &none);
    assert!(
        snap_cursor(&far, &snapper, 1.0, 1.0, Units::Mm, false)
            .1
            .is_some()
    );
    assert!(
        snap_cursor(&far, &snapper, 10.0, 1.0, Units::Mm, false)
            .1
            .is_none()
    );
}

#[test]
fn typed_values_read_in_every_form() {
    let mm = |t: &str| parse_entry(t, Units::Mm);
    assert_eq!(mm("20,10"), Ok(Entry::At((20.0, 10.0))));
    assert_eq!(mm(" @-5, 2.5 "), Ok(Entry::By((-5.0, 2.5))));
    assert_eq!(mm("40"), Ok(Entry::Distance(40.0)));
    assert_eq!(mm("40<90"), Ok(Entry::Polar(40.0, 90f64.to_radians())));
    assert_eq!(mm("@40<90"), mm("40<90"));
    // The decimal comma: with a semicolon, and always in d<a.
    assert_eq!(mm("12,5;30"), Ok(Entry::At((12.5, 30.0))));
    assert_eq!(
        mm("12,5<22,5"),
        Ok(Entry::Polar(12.5, 22.5f64.to_radians()))
    );
    // Units: a plain number is in the display units; any value can name its own.
    assert_eq!(parse_entry("1", Units::Inch), Ok(Entry::Distance(25.4)));
    assert_eq!(parse_entry("10mm", Units::Inch), Ok(Entry::Distance(10.0)));
    assert_eq!(mm("1/2in,1in"), Ok(Entry::At((12.7, 25.4))));
    // An angle alone: at the cursor's distance (an arc by its centre: its end).
    assert_eq!(mm("<30"), Ok(Entry::Angle(30f64.to_radians())));
    assert_eq!(
        typed_point(Entry::Angle(0.0), Some((1.0, 1.0)), (1.0, 6.0)),
        Ok((6.0, 1.0))
    );
    for bad in ["", "abc", "1,2,3", "4<", "@<30", "<x", "4<x", "1;"] {
        assert_eq!(mm(bad), Err(EntryError::Unreadable), "{bad:?}");
    }
}

#[test]
fn typed_values_count_from_the_last_click() {
    let last = Some((10.0, 10.0));
    let cursor = (10.0, 30.0); // straight up
    assert_eq!(
        typed_point(Entry::At((1.0, 2.0)), None, cursor),
        Ok((1.0, 2.0))
    );
    assert_eq!(
        typed_point(Entry::By((5.0, -1.0)), last, cursor),
        Ok((15.0, 9.0))
    );
    assert_eq!(
        typed_point(Entry::Distance(4.0), last, cursor),
        Ok((10.0, 14.0))
    );
    // Exactly at 90° and 180°: no rounding error.
    let up = typed_point(Entry::Polar(4.0, 90f64.to_radians()), last, cursor);
    assert_eq!(up, Ok((10.0, 14.0)));
    let left = typed_point(Entry::Polar(4.0, 180f64.to_radians()), last, cursor);
    assert_eq!(left, Ok((6.0, 10.0)));
    // A first point must be absolute; a distance needs a direction.
    assert_eq!(
        typed_point(Entry::Distance(4.0), None, cursor),
        Err(EntryError::NeedsLastPoint)
    );
    assert_eq!(
        typed_point(Entry::Distance(4.0), last, (10.0, 10.0)),
        Err(EntryError::NoDirection)
    );
    // A circle's typed distance is its radius.
    let mut circle = Sketch::new(Tool::Circle);
    circle.click((0.0, 0.0), false);
    let p = typed_point(Entry::Distance(7.5), circle.anchor(), (1.0, 0.0)).unwrap();
    assert!(matches!(
        circle.click(p, false),
        Step::Done(Geometry::Circle { radius, .. }) if radius == 7.5
    ));
}

#[test]
fn shift_holds_the_line_to_fifteen_degree_steps() {
    let o = (10.0, 10.0);
    // Nearly flat: exactly flat, the length on the 1 mm grid.
    assert_eq!(
        angle_step(o, (22.3, 10.4), Some(1.0), Units::Mm),
        (22.0, 10.0)
    );
    // Nearly vertical, downwards.
    assert_eq!(
        angle_step(o, (10.2, -5.6), Some(1.0), Units::Mm),
        (10.0, -6.0)
    );
    // 15°: on that line, 10 mm long.
    let p = angle_step(o, (19.7, 12.6), Some(1.0), Units::Mm);
    assert!((distance(o, p) - 10.0).abs() < 1e-9);
    assert!((math::atan2(p.1 - o.1, p.0 - o.0).to_degrees() - 15.0).abs() < 1e-9);
    // 345° is -15°.
    let q = angle_step(o, (19.7, 7.4), None, Units::Mm);
    assert!((math::atan2(q.1 - o.1, q.0 - o.0).to_degrees() + 15.0).abs() < 1e-9);
    assert_eq!(angle_step(o, o, Some(1.0), Units::Mm), o);
}

#[test]
fn the_readout_gives_the_size_being_drawn() {
    let mut line = Sketch::new(Tool::Polyline);
    assert_eq!(line.readout((1.0, 1.0), Units::Mm), None);
    line.click((0.0, 0.0), false);
    assert_eq!(
        line.readout((0.0, 20.0), Units::Mm).unwrap(),
        "L 20.000  90.0°"
    );
    let mut rect = Sketch::new(Tool::Rectangle);
    rect.click((0.0, 0.0), false);
    assert_eq!(
        rect.readout((-40.0, 20.0), Units::Mm).unwrap(),
        "40.000 × 20.000"
    );
    let mut circle = Sketch::new(Tool::Circle);
    circle.click((0.0, 0.0), false);
    assert_eq!(
        circle.readout((0.0, 25.4), Units::Inch).unwrap(),
        "R 1.0000"
    );
}

#[test]
fn an_arc_is_start_end_and_a_point_on_it() {
    let mut arc = Sketch::new(Tool::Arc);
    assert_eq!(arc.click((0.0, 0.0), false), Step::Continue);
    assert_eq!(arc.click((10.0, 0.0), false), Step::Continue);
    // On the line between them: refused, drawing goes on.
    assert_eq!(
        arc.click((5.0, 0.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::Arc)))
    );
    // Through (5, -5): the lower half circle, clockwise from (0, 0), one segment.
    let Step::Done(Geometry::Polyline {
        closed,
        points,
        bulges,
        ..
    }) = arc.click((5.0, -5.0), false)
    else {
        panic!()
    };
    assert!(!closed);
    assert_eq!(points.len(), 2);
    assert!((bulges[0] - 1.0).abs() < 1e-9);
    // A typed radius bows to the cursor's side; too small is refused.
    let mut typed = Sketch::new(Tool::Arc);
    typed.click((0.0, 0.0), false);
    typed.click((10.0, 0.0), false);
    let p = typed.typed(Entry::Distance(5.0), (3.0, 2.0)).unwrap();
    assert!(distance(p, (5.0, 5.0)) < 1e-9);
    assert_eq!(
        typed.typed(Entry::Distance(4.0), (3.0, 2.0)),
        Err(EntryError::RadiusTooSmall)
    );
    assert_eq!(
        typed.typed(Entry::Distance(6.0), (3.0, 0.0)),
        Err(EntryError::NoSide)
    );
    assert_eq!(typed.readout((5.0, 5.0), Units::Mm).unwrap(), "R 5.000");
}

#[test]
fn a_slot_is_a_rectangle_with_half_circle_ends() {
    let mut slot = Sketch::new(Tool::Slot);
    slot.click((0.0, 0.0), false);
    slot.click((0.0, 30.0), false);
    assert_eq!(slot.readout((4.0, 10.0), Units::Mm).unwrap(), "W 8.000");
    // Its width typed: 8 mm.
    let p = slot.typed(Entry::Distance(8.0), (100.0, 100.0)).unwrap();
    let Step::Done(geometry) = slot.click(p, false) else {
        panic!()
    };
    let Geometry::Rectangle {
        center,
        length,
        width,
        angle,
        corner_radius,
    } = geometry
    else {
        panic!()
    };
    assert_eq!(center, [0.0, 15.0]);
    assert!((length - 38.0).abs() < 1e-9 && (width - 8.0).abs() < 1e-9);
    assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert!((corner_radius - 4.0).abs() < 1e-9);
    assert!(chipload_job::edit::is_slot(&geometry));
    // No width: refused.
    let mut flat = Sketch::new(Tool::Slot);
    flat.click((0.0, 0.0), false);
    flat.click((10.0, 0.0), false);
    assert_eq!(
        flat.click((20.0, 0.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::Slot)))
    );
}

#[test]
fn previews_follow_the_cursor() {
    let mut sketch = Sketch::new(Tool::Rectangle);
    assert!(sketch.preview((5.0, 5.0)).is_empty());
    sketch.click((0.0, 0.0), false);
    let preview = sketch.preview((4.0, 2.0));
    assert_eq!(preview[0].len(), 5); // four corners, closed
    let mut line = Sketch::new(Tool::Polyline);
    line.click((0.0, 0.0), false);
    assert_eq!(line.preview((1.0, 1.0)), [vec![(0.0, 0.0), (1.0, 1.0)]]);
    let circle = outline(&Geometry::Circle {
        center: [0.0, 0.0],
        radius: 10.0,
    });
    assert!(circle[0].len() > 8);
    assert!(
        circle[0]
            .iter()
            .all(|&p| (distance(p, (0.0, 0.0)) - 10.0).abs() < 1e-9)
    );
}

#[test]
fn a_rectangle_by_its_centre_is_twice_the_corners_offset() {
    let mut sketch = Sketch::new(Tool::RectangleCentre);
    assert_eq!(sketch.click((10.0, 10.0), false), Step::Continue);
    assert_eq!(
        sketch.click((25.0, 0.0), false),
        Step::Done(Geometry::Rectangle {
            center: [10.0, 10.0],
            length: 30.0,
            width: 20.0,
            angle: 0.0,
            corner_radius: 0.0,
        })
    );
    // A typed `@dx,dy` counts from the centre: half the size.
    let mut typed = Sketch::new(Tool::RectangleCentre);
    typed.click((0.0, 0.0), false);
    let corner = typed.typed(Entry::By((20.0, 5.0)), (1.0, 1.0)).unwrap();
    assert_eq!(corner, (20.0, 5.0));
    assert_eq!(typed.readout(corner, Units::Mm).unwrap(), "40.000 × 10.000");
    // On the centre's line: no size, and drawing goes on.
    assert_eq!(
        typed.click((7.0, 0.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::RectangleCentre)))
    );
    assert_eq!(typed.last_point(), None);
}

#[test]
fn an_aligned_rectangle_is_an_edge_then_its_width() {
    let mut sketch = Sketch::new(Tool::RectangleAligned);
    assert_eq!(sketch.click((0.0, 0.0), false), Step::Continue);
    // The edge's preview is a line from its first end.
    assert_eq!(
        sketch.preview((0.0, 10.0)),
        vec![vec![(0.0, 0.0), (0.0, 10.0)]]
    );
    assert_eq!(sketch.last_point(), Some((0.0, 0.0)));
    assert_eq!(sketch.click((0.0, 10.0), false), Step::Continue);
    // On the edge's line: no width.
    assert_eq!(
        sketch.click((0.0, 30.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::RectangleAligned)))
    );
    assert_eq!(
        sketch.readout((-4.0, 3.0), Units::Mm).unwrap(),
        "10.000 × 4.000"
    );
    // Up the Y axis, the width to the left: a rectangle at 90°, centred at x -2.
    let Step::Done(Geometry::Rectangle {
        center,
        length,
        width,
        angle,
        corner_radius,
    }) = sketch.click((-4.0, 3.0), false)
    else {
        panic!("a rectangle");
    };
    assert!((center[0] + 2.0).abs() < 1e-12 && (center[1] - 5.0).abs() < 1e-12);
    assert_eq!((length, width, corner_radius), (10.0, 4.0, 0.0));
    assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

#[test]
fn an_aligned_rectangles_typed_width_goes_to_the_cursors_side() {
    let mut sketch = Sketch::new(Tool::RectangleAligned);
    sketch.click((0.0, 0.0), false);
    // The edge typed as a length at an angle.
    let end = sketch.typed(Entry::Polar(10.0, 0.0), (5.0, 5.0)).unwrap();
    assert_eq!(end, (10.0, 0.0));
    sketch.click(end, false);
    let below = sketch.typed(Entry::Distance(6.0), (3.0, -1.0)).unwrap();
    assert!(distance(below, (10.0, -6.0)) < 1e-12);
    let above = sketch.typed(Entry::Distance(6.0), (3.0, 1.0)).unwrap();
    assert!(distance(above, (10.0, 6.0)) < 1e-12);
    assert!(matches!(
        sketch.click(below, false),
        Step::Done(Geometry::Rectangle { width: 6.0, .. })
    ));
}

#[test]
fn a_shapes_variants_go_round_from_its_first() {
    assert_eq!(Tool::Rectangle.next_variant(), Tool::RectangleCentre);
    assert_eq!(Tool::RectangleCentre.next_variant(), Tool::RectangleAligned);
    assert_eq!(Tool::RectangleAligned.next_variant(), Tool::Rectangle);
    assert_eq!(Tool::RectangleAligned.shape(), Tool::Rectangle);
    // A shape with one way of drawing stays itself.
    assert_eq!(Tool::Polyline.next_variant(), Tool::Polyline);
    assert_eq!(Tool::Polyline.variants(), &[Tool::Polyline]);
}

#[test]
fn each_rectangle_variant_says_what_to_click() {
    let status = |tool: Tool, clicks: &[Point]| {
        let mut sketch = Sketch::new(tool);
        for &p in clicks {
            sketch.click(p, false);
        }
        sketch.status((3.0, 4.0), 1.0, Units::Mm, &En)
    };
    let texts = [
        status(Tool::RectangleCentre, &[]),
        status(Tool::RectangleCentre, &[(0.0, 0.0)]),
        status(Tool::RectangleAligned, &[]),
        status(Tool::RectangleAligned, &[(0.0, 0.0)]),
        status(Tool::RectangleAligned, &[(0.0, 0.0), (10.0, 0.0)]),
    ];
    assert!(texts[1].contains("6.000 x 8.000"), "{}", texts[1]);
    assert!(texts[3].contains("edge 5.000"), "{}", texts[3]);
    assert!(texts[4].contains("10.000 x 4.000"), "{}", texts[4]);
    for text in &texts {
        assert!(text.contains("grid 1"), "{text}");
    }
}

#[test]
fn a_circle_by_two_points_has_them_at_a_diameters_ends() {
    let mut sketch = Sketch::new(Tool::CircleTwoPoints);
    assert_eq!(sketch.click((0.0, 0.0), false), Step::Continue);
    // The first end is a last point: tangent snaps work from it.
    assert_eq!(sketch.last_point(), Some((0.0, 0.0)));
    assert_eq!(
        sketch.readout((6.0, 8.0), Units::Mm).unwrap(),
        "Ø 10.000  R 5.000"
    );
    // A typed length is the diameter, towards the cursor.
    let end = sketch.typed(Entry::Distance(20.0), (0.0, 1.0)).unwrap();
    assert_eq!(end, (0.0, 20.0));
    assert_eq!(
        sketch.click(end, false),
        Step::Done(Geometry::Circle {
            center: [0.0, 10.0],
            radius: 10.0
        })
    );
}

#[test]
fn a_circle_by_three_points_goes_through_them() {
    let mut sketch = Sketch::new(Tool::CircleThreePoints);
    sketch.click((5.0, 0.0), false);
    // Before the last click the preview is the line so far.
    assert_eq!(
        sketch.preview((0.0, 5.0)),
        vec![vec![(5.0, 0.0), (0.0, 5.0)]]
    );
    sketch.click((0.0, 5.0), false);
    assert_eq!(sketch.readout((-5.0, 0.0), Units::Mm).unwrap(), "R 5.000");
    assert_eq!(sketch.preview((-5.0, 0.0)).len(), 1);
    // On the line through the first two: refused, and drawing goes on.
    assert_eq!(
        sketch.click((10.0, -5.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::CircleThreePoints)))
    );
    assert_eq!(
        Why::NoSize(Tool::CircleThreePoints).text(&En),
        "the three points lie on a line: no circle goes through them"
    );
    let Step::Done(Geometry::Circle { center, radius }) = sketch.click((-5.0, 0.0), false) else {
        panic!("a circle");
    };
    assert!(center[0].abs() < 1e-9 && center[1].abs() < 1e-9);
    assert!((radius - 5.0).abs() < 1e-9);
}

#[test]
fn each_circle_variant_says_what_to_click() {
    let status = |tool: Tool, clicks: &[Point]| {
        let mut sketch = Sketch::new(tool);
        for &p in clicks {
            sketch.click(p, false);
        }
        sketch.status((3.0, 4.0), 1.0, Units::Mm, &En)
    };
    assert!(status(Tool::CircleTwoPoints, &[]).contains("one end of a diameter"));
    assert!(status(Tool::CircleTwoPoints, &[(0.0, 0.0)]).contains("diameter 5.000"));
    assert!(status(Tool::CircleThreePoints, &[]).contains("first point"));
    assert!(status(Tool::CircleThreePoints, &[(0.0, 0.0)]).contains("second point"));
    let third = status(Tool::CircleThreePoints, &[(-5.0, 0.0), (5.0, 0.0)]);
    assert!(third.contains("radius 5.000"), "{third}");
    assert_eq!(Tool::Circle.next_variant(), Tool::CircleTwoPoints);
    assert_eq!(Tool::CircleThreePoints.next_variant(), Tool::Circle);
    assert_eq!(Tool::CircleThreePoints.shape(), Tool::Circle);
}

#[test]
fn an_arc_by_its_centre_goes_round_the_way_the_cursor_did() {
    let arc = |turn: &[Point]| {
        let mut sketch = Sketch::new(Tool::ArcCentre);
        sketch.click((0.0, 0.0), false);
        assert_eq!(sketch.last_point(), Some((0.0, 0.0)));
        sketch.click((10.0, 0.0), false);
        assert_eq!(sketch.last_point(), None);
        for &p in turn {
            sketch.follow(p);
        }
        sketch.click((0.0, 10.0), false)
    };
    // Up from the start: a quarter, counter-clockwise, one segment.
    let Step::Done(Geometry::Polyline {
        closed: false,
        points,
        bulges,
        ..
    }) = arc(&[(10.0, 5.0), (5.0, 9.0)])
    else {
        panic!("an arc");
    };
    assert_eq!(points.len(), 2);
    assert!((bulges[0] - math::tan(std::f64::consts::FRAC_PI_2 / 4.0)).abs() < 1e-12);
    // Down first and round: three quarters clockwise, two segments.
    let Step::Done(Geometry::Polyline { points, bulges, .. }) =
        arc(&[(10.0, -5.0), (0.0, -10.0), (-10.0, 0.0)])
    else {
        panic!("an arc");
    };
    assert_eq!(points.len(), 3);
    assert!(bulges.iter().all(|&b| b < 0.0));
}

#[test]
fn an_arc_by_its_centre_takes_its_radius_and_its_end_typed() {
    let mut sketch = Sketch::new(Tool::ArcCentre);
    sketch.click((0.0, 0.0), false);
    // A length from the centre is the radius, towards the cursor.
    let start = sketch.typed(Entry::Distance(20.0), (5.0, 0.0)).unwrap();
    assert_eq!(start, (20.0, 0.0));
    assert_eq!(sketch.readout((3.0, 4.0), Units::Mm).unwrap(), "R 5.000");
    sketch.click(start, false);
    // The end at 90° on the circle, whatever the length.
    let end = sketch
        .typed(Entry::Angle(std::f64::consts::FRAC_PI_2), (1.0, 1.0))
        .unwrap();
    assert!(distance(end, (0.0, 20.0)) < 1e-12);
    assert_eq!(
        sketch
            .typed(Entry::Polar(3.0, std::f64::consts::FRAC_PI_2), (1.0, 1.0))
            .unwrap(),
        end
    );
    sketch.follow((14.0, 14.0));
    assert_eq!(sketch.readout(end, Units::Mm).unwrap(), "R 20.000  90.0°");
    assert!(
        sketch
            .status(end, 1.0, Units::Mm, &En)
            .contains("radius 20.000, 90.0°")
    );
    // Back on the start's direction: no arc.
    assert_eq!(
        sketch.click((30.0, 0.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::ArcCentre)))
    );
}

#[test]
fn a_polygon_by_its_side_and_on_an_edge() {
    // A hexagon 20 across flats: the flat's middle straight up from the centre.
    let mut side = Sketch::new(Tool::PolygonSide);
    side.click((5.0, 5.0), false);
    assert_eq!(side.last_point(), Some((5.0, 5.0)));
    assert!(
        side.status((5.0, 15.0), 1.0, Units::Mm, &En)
            .contains("20.000 across flats")
    );
    let Step::Done(Geometry::Polygon {
        center,
        radius,
        sides,
        angle,
    }) = side.click((5.0, 15.0), false)
    else {
        panic!("a polygon");
    };
    assert_eq!((center, sides), ([5.0, 5.0], 6));
    assert!((radius - 20.0 / 3f64.sqrt()).abs() < 1e-12);
    assert!((angle - std::f64::consts::FRAC_PI_3).abs() < 1e-12);
    // A square on an edge, with four sides chosen: above the edge drawn left to right.
    let mut edge = Sketch::new(Tool::PolygonEdge);
    edge.sides = 4;
    edge.click((0.0, 0.0), false);
    assert_eq!(edge.last_point(), Some((0.0, 0.0)));
    let Step::Done(Geometry::Polygon { center, sides, .. }) = edge.click((10.0, 0.0), false) else {
        panic!("a polygon");
    };
    assert_eq!(sides, 4);
    assert!((center[0] - 5.0).abs() < 1e-12 && (center[1] - 5.0).abs() < 1e-12);
    assert_eq!(Tool::Polygon.next_variant(), Tool::PolygonSide);
    assert_eq!(Tool::PolygonEdge.next_variant(), Tool::Polygon);
    assert_eq!(Tool::ArcCentre.next_variant(), Tool::Arc);
}

#[test]
fn a_spline_is_points_finished_open_or_closed_on_the_first() {
    let mut open = Sketch::new(Tool::Spline);
    assert_eq!(
        open.finish(),
        Step::Ignored(Some(Why::TooFewPoints(Tool::Spline)))
    );
    assert_eq!(
        Why::TooFewPoints(Tool::Spline).text(&En),
        "a spline needs at least 2 points"
    );
    for p in [(0.0, 0.0), (20.0, 15.0), (45.0, 5.0)] {
        assert_eq!(open.click(p, false), Step::Continue);
    }
    assert_eq!(open.last_point(), Some((45.0, 5.0)));
    // The preview is the curve through the points and the cursor: from the first point
    // to the cursor, bending smoothly, many points.
    let preview = open.preview((60.0, 20.0));
    assert_eq!(preview.len(), 1);
    assert!(preview[0].len() > 10);
    assert_eq!(preview[0][0], (0.0, 0.0));
    assert!(distance(*preview[0].last().unwrap(), (60.0, 20.0)) < 1e-9);
    assert!(
        open.status((60.0, 20.0), 1.0, Units::Mm, &En)
            .contains("3 points")
    );
    assert_eq!(
        open.finish(),
        Step::Done(Geometry::Spline {
            closed: false,
            points: vec![[0.0, 0.0], [20.0, 15.0], [45.0, 5.0]],
        })
    );
    // The first point again closes it.
    let mut closed = Sketch::new(Tool::Spline);
    for p in [(0.0, 0.0), (20.0, 15.0), (45.0, 5.0)] {
        closed.click(p, false);
    }
    assert!(matches!(
        closed.click((0.0, 0.0), true),
        Step::Done(Geometry::Spline { closed: true, ref points }) if points.len() == 3
    ));
}

#[test]
fn typed_values_take_arithmetic() {
    // docs/specs/drawing-improvements.md §2.
    let mm = |t: &str| parse_entry(t, Units::Mm);
    assert_eq!(mm("(100-6)/2"), Ok(Entry::Distance(47.0)));
    assert_eq!(mm("600/4,18+0.2"), Ok(Entry::At((150.0, 18.2))));
    assert_eq!(mm("@-10*2,5"), Ok(Entry::By((-20.0, 5.0))));
    assert_eq!(mm("<360/8"), Ok(Entry::Angle(45f64.to_radians())));
    assert_eq!(mm("20<90+45"), Ok(Entry::Polar(20.0, 135f64.to_radians())));
    assert_eq!(mm("12,5+1;3"), Ok(Entry::At((13.5, 3.0))));
    assert_eq!(mm("<1in"), Err(EntryError::Unreadable));
}

#[test]
fn an_arc_slot_is_its_centre_ends_and_width() {
    // docs/specs/drawing-improvements.md §3: round the origin at radius 40, a quarter,
    // 8 wide.
    let mut slot = Sketch::new(Tool::SlotArc);
    slot.click((0.0, 0.0), false);
    assert_eq!(slot.last_point(), Some((0.0, 0.0)));
    // The radius typed, then the end at 90°.
    let start = slot.typed(Entry::Distance(40.0), (1.0, 0.0)).unwrap();
    assert_eq!(start, (40.0, 0.0));
    slot.click(start, false);
    slot.follow((30.0, 30.0));
    let end = slot
        .typed(Entry::Angle(std::f64::consts::FRAC_PI_2), (1.0, 1.0))
        .unwrap();
    assert!(distance(end, (0.0, 40.0)) < 1e-12);
    assert_eq!(slot.click(end, false), Step::Continue);
    // The width typed: 4 out from the centre line, towards the cursor.
    let at = slot.typed(Entry::Distance(8.0), (30.0, 30.0)).unwrap();
    assert!((distance(at, (0.0, 0.0)) - 44.0).abs() < 1e-9);
    assert!(slot.readout(at, Units::Mm).unwrap().starts_with("W 8.000"));
    let Step::Done(Geometry::Polyline {
        closed: true,
        points,
        bulges,
        ..
    }) = slot.click(at, false)
    else {
        panic!("a closed polyline");
    };
    assert_eq!(points.len(), 4);
    assert_eq!((bulges[1], bulges[3]), (1.0, 1.0));
    // Wider than the radius allows: refused, drawing goes on.
    let mut wide = Sketch::new(Tool::SlotArc);
    for p in [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)] {
        wide.click(p, false);
    }
    assert_eq!(
        wide.click((0.0, 25.0), false),
        Step::Ignored(Some(Why::NoSize(Tool::SlotArc)))
    );
    assert_eq!(Tool::Slot.next_variant(), Tool::SlotArc);
    assert_eq!(Tool::SlotArc.shape(), Tool::Slot);
}

#[test]
fn a_polyline_drawn_with_a_radius_has_its_corners_rounded() {
    // docs/specs/polyline-radii.md §4: the status bar's radius on every corner as drawn.
    let mut sketch = Sketch::new(Tool::Polyline);
    sketch.corner_radius = 5.0;
    for p in [(0.0, 0.0), (100.0, 0.0), (100.0, 60.0), (0.0, 60.0)] {
        sketch.click(p, false);
    }
    // The preview is already rounded.
    assert!(sketch.preview((0.0, 30.0))[0].len() > 6);
    let Step::Done(Geometry::Polyline { radii, points, .. }) = sketch.click((0.0, 0.0), true)
    else {
        panic!("a closed polyline");
    };
    assert_eq!(points.len(), 4);
    assert_eq!(radii, [5.0, 5.0, 5.0, 5.0]);
    // Open: the ends stay sharp; a corner too short for the radius takes what fits.
    let mut open = Sketch::new(Tool::Polyline);
    open.corner_radius = 20.0;
    for p in [(0.0, 0.0), (10.0, 0.0), (10.0, 50.0)] {
        open.click(p, false);
    }
    let Step::Done(Geometry::Polyline { radii, .. }) = open.finish() else {
        panic!("an open polyline");
    };
    assert_eq!(radii[0], 0.0);
    assert!((radii[1] - 10.0).abs() < 1e-9);
    assert_eq!(radii[2], 0.0);
}
