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
    let snapper = snapper(&contours);
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
    for bad in ["", "abc", "1,2,3", "4<", "<30", "4<x", "1;"] {
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
