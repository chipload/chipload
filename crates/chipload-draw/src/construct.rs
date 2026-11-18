// SPDX-License-Identifier: GPL-3.0-or-later
//! Shapes from the points of a drawing tool (docs/specs/drawing-input.md §4,
//! drawing-variants.md §2): the arc through three points, the arc of a radius between two,
//! an arc as polyline segments, rectangles by two corners, by their centre or aligned to
//! an edge, circles by two or three points, arcs by their centre, regular polygons by a
//! side's middle or on an edge, and arc slots.

use chipload_core::math;
use std::f64::consts::{PI, TAU};

use chipload_core::Point;
use chipload_core::primitives::{Arc, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;

/// The arc from `start` through `through` to `end`: the circle through the three points,
/// swept the way that passes `through`. None when two of them coincide or the three lie
/// on one line (within `CHAIN_TOL_MM` of it).
pub fn arc_through(start: Point, through: Point, end: Point) -> Option<Arc> {
    let chord = distance(start, end);
    if chord < CHAIN_TOL_MM
        || distance(start, through) < CHAIN_TOL_MM
        || distance(through, end) < CHAIN_TOL_MM
    {
        return None;
    }
    // How far `through` lies off the chord.
    let (ex, ey) = (end.0 - start.0, end.1 - start.1);
    let (tx, ty) = (through.0 - start.0, through.1 - start.1);
    let cross = ex * ty - ey * tx;
    if cross.abs() / chord < CHAIN_TOL_MM {
        return None;
    }
    // The circumcentre, relative to `start`.
    let d = 2.0 * cross;
    let (e2, t2) = (ex * ex + ey * ey, tx * tx + ty * ty);
    let cx = (ty * e2 - ey * t2) / d;
    let cy = (ex * t2 - tx * e2) / d;
    let center = (start.0 + cx, start.1 + cy);
    let angle = |p: Point| math::atan2(p.1 - center.1, p.0 - center.0);
    let start_angle = angle(start);
    let to_end = (angle(end) - start_angle).rem_euclid(TAU);
    let to_through = (angle(through) - start_angle).rem_euclid(TAU);
    // Counter-clockwise when `through` comes before `end` that way.
    let sweep = if to_through < to_end {
        to_end
    } else {
        to_end - TAU
    };
    Some(Arc {
        center,
        radius: distance(center, start),
        start_angle,
        sweep,
    })
}

/// The middle point of the shorter arc of `radius` from `start` to `end`, bowing to the
/// side of `side`. None when the radius is less than half the distance between the ends,
/// or `side` lies on the line through them.
pub fn arc_middle_for_radius(start: Point, end: Point, radius: f64, side: Point) -> Option<Point> {
    let chord = distance(start, end);
    let half = chord / 2.0;
    if chord < CHAIN_TOL_MM || radius < half {
        return None;
    }
    let (ux, uy) = ((end.0 - start.0) / chord, (end.1 - start.1) / chord);
    // The left normal, turned to the side's side.
    let off = (side.0 - start.0) * -uy + (side.1 - start.1) * ux;
    if off.abs() < CHAIN_TOL_MM {
        return None;
    }
    let (nx, ny) = if off > 0.0 { (-uy, ux) } else { (uy, -ux) };
    let sagitta = radius - (radius * radius - half * half).max(0.0).sqrt();
    let middle = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
    Some((middle.0 + nx * sagitta, middle.1 + ny * sagitta))
}

/// An arc as open polyline points and bulges: one segment, or two halves when it sweeps
/// more than a half circle (a bulge is at most 1).
pub fn arc_as_polyline(arc: &Arc) -> (Vec<Point>, Vec<f64>) {
    let start = arc.point_at(arc.start_angle);
    let end = arc.point_at(arc.start_angle + arc.sweep);
    if arc.sweep.abs() <= PI {
        return (vec![start, end], vec![math::tan(arc.sweep / 4.0)]);
    }
    let middle = arc.point_at(arc.start_angle + arc.sweep / 2.0);
    let half = math::tan(arc.sweep / 8.0);
    (vec![start, middle, end], vec![half, half])
}

/// A rectangle by its defining values, as a drawn rectangle stores them: its centre, its
/// length along `angle` (radians, counter-clockwise from +X) and its width across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    pub center: Point,
    pub length: f64,
    pub width: f64,
    pub angle: f64,
}

impl Rectangle {
    /// None when a side is shorter than `CHAIN_TOL_MM`: the rectangle would have no size.
    fn sized(self) -> Option<Rectangle> {
        (self.length >= CHAIN_TOL_MM && self.width >= CHAIN_TOL_MM).then_some(self)
    }
}

/// The rectangle with opposite corners `a` and `b`, square to the axes.
pub fn rectangle_by_corners(a: Point, b: Point) -> Option<Rectangle> {
    Rectangle {
        center: ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
        length: (b.0 - a.0).abs(),
        width: (b.1 - a.1).abs(),
        angle: 0.0,
    }
    .sized()
}

/// The rectangle centred on `center` with a corner at `corner`, square to the axes.
pub fn rectangle_by_centre(center: Point, corner: Point) -> Option<Rectangle> {
    Rectangle {
        center,
        length: 2.0 * (corner.0 - center.0).abs(),
        width: 2.0 * (corner.1 - center.1).abs(),
        angle: 0.0,
    }
    .sized()
}

/// How far `p` lies to the left of the line from `a` to `b` (negative: to its right); 0
/// when `a` and `b` coincide.
pub fn left_of(a: Point, b: Point, p: Point) -> f64 {
    let between = distance(a, b);
    if between < CHAIN_TOL_MM {
        return 0.0;
    }
    ((b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)) / between
}

/// The rectangle with one side from `a` to `b`, reaching across to `p`'s side as far as
/// `p` lies from that side: length the side, width `p`'s distance, angle the direction
/// from `a` to `b`.
pub fn rectangle_aligned(a: Point, b: Point, p: Point) -> Option<Rectangle> {
    let between = distance(a, b);
    if between < CHAIN_TOL_MM {
        return None;
    }
    let off = left_of(a, b, p);
    let (nx, ny) = (-(b.1 - a.1) / between, (b.0 - a.0) / between);
    Rectangle {
        center: (
            (a.0 + b.0) / 2.0 + nx * off / 2.0,
            (a.1 + b.1) / 2.0 + ny * off / 2.0,
        ),
        length: between,
        width: off.abs(),
        angle: math::atan2(b.1 - a.1, b.0 - a.0),
    }
    .sized()
}

/// The circle with `a` and `b` at the ends of a diameter: its centre and radius. None
/// when they coincide.
pub fn circle_by_diameter(a: Point, b: Point) -> Option<(Point, f64)> {
    let radius = distance(a, b) / 2.0;
    (radius >= CHAIN_TOL_MM).then_some((((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), radius))
}

/// The circle through `a`, `b` and `c`: its centre and radius. None when two coincide or
/// the three lie on one line, as for [`arc_through`].
pub fn circle_through(a: Point, b: Point, c: Point) -> Option<(Point, f64)> {
    arc_through(a, b, c).map(|arc| (arc.center, arc.radius))
}

/// The arc about `center` from `start` to the direction of `end` (only its direction
/// counts: the radius is `start`'s), counter-clockwise when `turned` (the angle the cursor
/// went round since the start, radians) is positive or zero, else clockwise. None when the
/// radius is zero or the end lies in the start's direction.
pub fn arc_by_centre(center: Point, start: Point, end: Point, turned: f64) -> Option<Arc> {
    let radius = distance(center, start);
    if radius < CHAIN_TOL_MM || distance(center, end) < CHAIN_TOL_MM {
        return None;
    }
    let start_angle = math::atan2(start.1 - center.1, start.0 - center.0);
    let end_angle = math::atan2(end.1 - center.1, end.0 - center.0);
    let ccw = (end_angle - start_angle).rem_euclid(TAU);
    let sweep = if turned >= 0.0 { ccw } else { ccw - TAU };
    // The end as far round as the start: no arc (its chord within the chaining tolerance).
    let chord = 2.0 * radius * math::sin(sweep.abs().min(TAU - sweep.abs()) / 2.0);
    (chord >= CHAIN_TOL_MM).then_some(Arc {
        center,
        radius,
        start_angle,
        sweep,
    })
}

/// A regular polygon by its values: the centre, the radius to its corners and the angle of
/// its first corner (radians, counter-clockwise from +X), as a drawn polygon stores them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Polygon {
    pub center: Point,
    pub radius: f64,
    pub angle: f64,
}

/// The regular polygon of `sides` about `center` with the middle of a side at `middle`:
/// across flats twice their distance. None when they coincide or `sides` is under 3.
pub fn polygon_by_side(center: Point, middle: Point, sides: u32) -> Option<Polygon> {
    let apothem = distance(center, middle);
    if apothem < CHAIN_TOL_MM || sides < 3 {
        return None;
    }
    let half = PI / f64::from(sides);
    Some(Polygon {
        center,
        radius: apothem / math::cos(half),
        angle: math::atan2(middle.1 - center.1, middle.0 - center.0) - half,
    })
}

/// The regular polygon of `sides` with one side from `a` to `b`, lying to its left (its
/// corners run counter-clockwise from `a` to `b`). None when they coincide or `sides` is
/// under 3.
pub fn polygon_on_edge(a: Point, b: Point, sides: u32) -> Option<Polygon> {
    let side = distance(a, b);
    if side < CHAIN_TOL_MM || sides < 3 {
        return None;
    }
    let half = PI / f64::from(sides);
    let apothem = side / (2.0 * math::tan(half));
    let (nx, ny) = (-(b.1 - a.1) / side, (b.0 - a.0) / side);
    let center = (
        (a.0 + b.0) / 2.0 + nx * apothem,
        (a.1 + b.1) / 2.0 + ny * apothem,
    );
    Some(Polygon {
        center,
        radius: side / (2.0 * math::sin(half)),
        angle: math::atan2(a.1 - center.1, a.0 - center.0),
    })
}

/// A slot of `width` bent along `arc` (its centre line) (docs/specs/drawing-improvements.md
/// §3), as a closed polyline: the outer arc from the start to the end, a half-circle cap
/// round the end, the inner arc back, and a cap round the start; the points and one bulge
/// per segment. Arcs past a half circle are split in two. The loop runs the way `arc` does.
/// None when the width is zero, reaches the centre (the inner arc would vanish), or the
/// caps would meet across the gap of a slot that goes nearly all the way round.
pub fn arc_slot(arc: &Arc, width: f64) -> Option<(Vec<Point>, Vec<f64>)> {
    let half = width / 2.0;
    let (r_out, r_in) = (arc.radius + half, arc.radius - half);
    if !(width >= CHAIN_TOL_MM && r_in >= CHAIN_TOL_MM && arc.sweep.is_finite()) {
        return None;
    }
    let sweep = arc.sweep;
    if sweep.abs() < f64::EPSILON || sweep.abs() >= TAU {
        return None;
    }
    // The caps' centres are the arc's ends; across the gap they must be a width apart.
    let gap = TAU - sweep.abs();
    if gap < PI && 2.0 * arc.radius * math::sin(gap / 2.0) < width + CHAIN_TOL_MM {
        return None;
    }
    let (a0, a1) = (arc.start_angle, arc.start_angle + sweep);
    let at = |r: f64, a: f64| {
        let (sin, cos) = math::sin_cos(a);
        (arc.center.0 + r * cos, arc.center.1 + r * sin)
    };
    let cap = if sweep > 0.0 { 1.0 } else { -1.0 };
    let mut points = Vec::new();
    let mut bulges = Vec::new();
    // An arc of radius r from angle `from` turning by `turn`: its points but the last, with
    // their bulges (split in halves past a half circle).
    let arc_to = |points: &mut Vec<Point>, bulges: &mut Vec<f64>, r: f64, from: f64, turn: f64| {
        if turn.abs() > PI {
            let b = math::tan(turn / 8.0);
            points.push(at(r, from));
            bulges.push(b);
            points.push(at(r, from + turn / 2.0));
            bulges.push(b);
        } else {
            points.push(at(r, from));
            bulges.push(math::tan(turn / 4.0));
        }
    };
    arc_to(&mut points, &mut bulges, r_out, a0, sweep);
    // The end cap: from the outer end to the inner end.
    points.push(at(r_out, a1));
    bulges.push(cap);
    arc_to(&mut points, &mut bulges, r_in, a1, -sweep);
    // The start cap: from the inner start back to the outer start (the loop closes).
    points.push(at(r_in, a0));
    bulges.push(cap);
    Some((points, bulges))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn close(a: Point, b: Point) -> bool {
        distance(a, b) < 1e-9
    }

    #[test]
    fn three_points_give_the_arc_through_them() {
        // The upper half of the circle r 5 at (5, 0): counter-clockwise from (10, 0).
        let arc = arc_through((10.0, 0.0), (5.0, 5.0), (0.0, 0.0)).unwrap();
        assert!(close(arc.center, (5.0, 0.0)));
        assert!((arc.radius - 5.0).abs() < 1e-9);
        assert!((arc.sweep - PI).abs() < 1e-9);
        // The other way round: clockwise.
        let back = arc_through((0.0, 0.0), (5.0, 5.0), (10.0, 0.0)).unwrap();
        assert!((back.sweep + PI).abs() < 1e-9);
        // Through a point below: the lower half, three quarters when it goes round.
        let big = arc_through((10.0, 0.0), (5.0, -5.0), (5.0, 5.0)).unwrap();
        assert!((big.sweep + 3.0 * FRAC_PI_2).abs() < 1e-9);
        // On a line, or a repeated point: none.
        assert!(arc_through((0.0, 0.0), (5.0, 0.0), (10.0, 0.0)).is_none());
        assert!(arc_through((0.0, 0.0), (0.0, 0.0), (10.0, 0.0)).is_none());
    }

    #[test]
    fn a_radius_gives_the_shorter_arc_on_the_cursors_side() {
        let up = arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 5.0, (3.0, 1.0)).unwrap();
        assert!(close(up, (5.0, 5.0)));
        let down = arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 13.0, (3.0, -1.0)).unwrap();
        assert!(close(down, (5.0, -1.0)));
        assert!(arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 4.0, (3.0, 1.0)).is_none());
        assert!(arc_middle_for_radius((0.0, 0.0), (10.0, 0.0), 6.0, (3.0, 0.0)).is_none());
    }

    #[test]
    fn arcs_past_a_half_circle_become_two_segments() {
        let half = arc_through((10.0, 0.0), (5.0, 5.0), (0.0, 0.0)).unwrap();
        let (points, bulges) = arc_as_polyline(&half);
        assert_eq!(points.len(), 2);
        assert!((bulges[0] - 1.0).abs() < 1e-9);
        let big = arc_through((10.0, 0.0), (5.0, -5.0), (5.0, 5.0)).unwrap();
        let (points, bulges) = arc_as_polyline(&big);
        assert_eq!(points.len(), 3);
        assert!(close(points[2], (5.0, 5.0)));
        assert!(bulges.iter().all(|b| b.abs() <= 1.0 && *b < 0.0));
        // The bulges give the same arc back.
        let first = Arc::from_bulge(points[0], points[1], bulges[0]).unwrap();
        assert!(close(first.center, big.center));
    }

    #[test]
    fn rectangles_by_corners_and_by_centre() {
        let r = rectangle_by_corners((10.0, 20.0), (0.0, 0.0)).unwrap();
        assert!(close(r.center, (5.0, 10.0)));
        assert_eq!((r.length, r.width, r.angle), (10.0, 20.0, 0.0));
        let c = rectangle_by_centre((5.0, 10.0), (0.0, 25.0)).unwrap();
        assert!(close(c.center, (5.0, 10.0)));
        assert_eq!((c.length, c.width), (10.0, 30.0));
        // A corner on the centre's line: no size.
        assert!(rectangle_by_centre((5.0, 10.0), (9.0, 10.0)).is_none());
        assert!(rectangle_by_corners((0.0, 0.0), (0.0, 5.0)).is_none());
    }

    #[test]
    fn an_aligned_rectangle_stands_on_its_edge_towards_the_third_point() {
        // Along +X, the third point above: the rectangle above the edge.
        let up = rectangle_aligned((0.0, 0.0), (10.0, 0.0), (3.0, 4.0)).unwrap();
        assert!(close(up.center, (5.0, 2.0)));
        assert!((up.length - 10.0).abs() < 1e-12 && (up.width - 4.0).abs() < 1e-12);
        assert!(up.angle.abs() < 1e-12);
        // Below: the same edge, the rectangle under it.
        let down = rectangle_aligned((0.0, 0.0), (10.0, 0.0), (30.0, -4.0)).unwrap();
        assert!(close(down.center, (5.0, -2.0)));
        // Straight up the Y axis, the third point to the right: centre at x 3.
        let side = rectangle_aligned((0.0, 0.0), (0.0, 10.0), (6.0, 1.0)).unwrap();
        assert!(close(side.center, (3.0, 5.0)));
        assert!((side.angle - FRAC_PI_2).abs() < 1e-12);
        // At 30°: the corners are the edge's ends and the edge moved across by the width.
        let (s, c) = (0.5, 3f64.sqrt() / 2.0);
        let b = (10.0 * c, 10.0 * s);
        let r = rectangle_aligned((0.0, 0.0), b, (-2.0 * s, 2.0 * c)).unwrap();
        assert!((r.angle - PI / 6.0).abs() < 1e-12);
        assert!((r.width - 2.0).abs() < 1e-12);
        assert!(close(r.center, (5.0 * c - s, 5.0 * s + c)));
        // The third point on the edge's line, or the edge a point: no size.
        assert!(rectangle_aligned((0.0, 0.0), (10.0, 0.0), (20.0, 0.0)).is_none());
        assert!(rectangle_aligned((1.0, 1.0), (1.0, 1.0), (5.0, 5.0)).is_none());
    }

    #[test]
    fn circles_by_a_diameter_and_through_three_points() {
        let (c, r) = circle_by_diameter((0.0, 0.0), (6.0, 8.0)).unwrap();
        assert!(close(c, (3.0, 4.0)) && (r - 5.0).abs() < 1e-12);
        assert!(circle_by_diameter((1.0, 1.0), (1.0, 1.0)).is_none());
        // Any order of the three points gives the same circle.
        for (a, b, p) in [
            ((5.0, 0.0), (0.0, 5.0), (-5.0, 0.0)),
            ((-5.0, 0.0), (5.0, 0.0), (0.0, -5.0)),
            ((3.0, 4.0), (-4.0, -3.0), (0.0, 5.0)),
        ] {
            let (c, r) = circle_through(a, b, p).unwrap();
            assert!(close(c, (0.0, 0.0)) && (r - 5.0).abs() < 1e-9);
        }
        assert!(circle_through((0.0, 0.0), (5.0, 5.0), (10.0, 10.0)).is_none());
        assert!(circle_through((0.0, 0.0), (0.0, 0.0), (10.0, 0.0)).is_none());
    }

    #[test]
    fn an_arc_by_its_centre_goes_the_way_the_cursor_turned() {
        let ccw = arc_by_centre((0.0, 0.0), (5.0, 0.0), (0.0, 20.0), 0.3).unwrap();
        assert!((ccw.radius - 5.0).abs() < 1e-12);
        assert!((ccw.sweep - FRAC_PI_2).abs() < 1e-12);
        let cw = arc_by_centre((0.0, 0.0), (5.0, 0.0), (0.0, 20.0), -0.3).unwrap();
        assert!((cw.sweep + 3.0 * FRAC_PI_2).abs() < 1e-12);
        // Not turned at all: counter-clockwise.
        let zero = arc_by_centre((0.0, 0.0), (5.0, 0.0), (-1.0, 0.0), 0.0).unwrap();
        assert!((zero.sweep - PI).abs() < 1e-12);
        // The end in the start's direction, or on the centre: none.
        assert!(arc_by_centre((0.0, 0.0), (5.0, 0.0), (9.0, 0.0), 1.0).is_none());
        assert!(arc_by_centre((0.0, 0.0), (5.0, 0.0), (0.0, 0.0), 1.0).is_none());
        assert!(arc_by_centre((0.0, 0.0), (0.0, 0.0), (1.0, 1.0), 1.0).is_none());
    }

    #[test]
    fn a_polygon_by_a_sides_middle_has_its_flats_there() {
        // A hexagon 20 across flats, a flat's middle straight up.
        let hex = polygon_by_side((0.0, 0.0), (0.0, 10.0), 6).unwrap();
        assert!((hex.radius - 20.0 / 3f64.sqrt()).abs() < 1e-12);
        assert!((hex.angle - (FRAC_PI_2 - PI / 6.0)).abs() < 1e-12);
        // Its first two corners are the ends of that flat.
        let corner = |k: f64| {
            let a = hex.angle + k * TAU / 6.0;
            (hex.radius * math::cos(a), hex.radius * math::sin(a))
        };
        let (c0, c1) = (corner(0.0), corner(1.0));
        assert!(close(
            ((c0.0 + c1.0) / 2.0, (c0.1 + c1.1) / 2.0),
            (0.0, 10.0)
        ));
        assert!(polygon_by_side((0.0, 0.0), (0.0, 0.0), 6).is_none());
    }

    #[test]
    fn a_polygon_on_an_edge_lies_to_its_left() {
        // A square on (0,0)-(10,0): above it.
        let up = polygon_on_edge((0.0, 0.0), (10.0, 0.0), 4).unwrap();
        assert!(close(up.center, (5.0, 5.0)));
        assert!((up.radius - 50f64.sqrt()).abs() < 1e-12);
        // The other way round: below.
        let down = polygon_on_edge((10.0, 0.0), (0.0, 0.0), 4).unwrap();
        assert!(close(down.center, (5.0, -5.0)));
        // A triangle: its first corner at a, the next at b.
        for (a, b) in [((0.0, 0.0), (10.0, 0.0)), ((3.0, 1.0), (-2.0, 7.0))] {
            let tri = polygon_on_edge(a, b, 3).unwrap();
            let at = |k: f64| {
                let ang = tri.angle + k * TAU / 3.0;
                (
                    tri.center.0 + tri.radius * math::cos(ang),
                    tri.center.1 + tri.radius * math::sin(ang),
                )
            };
            assert!(close(at(0.0), a) && close(at(1.0), b));
        }
        assert!(polygon_on_edge((1.0, 1.0), (1.0, 1.0), 6).is_none());
    }

    #[test]
    fn an_arc_slot_is_two_arcs_and_two_round_caps() {
        // A quarter round the origin at radius 40, 8 wide, counter-clockwise from +X.
        let arc = Arc {
            center: (0.0, 0.0),
            radius: 40.0,
            start_angle: 0.0,
            sweep: FRAC_PI_2,
        };
        let (points, bulges) = arc_slot(&arc, 8.0).unwrap();
        assert_eq!(points.len(), 4);
        assert!(close(points[0], (44.0, 0.0)));
        assert!(close(points[1], (0.0, 44.0)));
        assert!(close(points[2], (0.0, 36.0)));
        assert!(close(points[3], (36.0, 0.0)));
        let quarter = math::tan(FRAC_PI_2 / 4.0);
        assert!((bulges[0] - quarter).abs() < 1e-12 && (bulges[2] + quarter).abs() < 1e-12);
        assert_eq!((bulges[1], bulges[3]), (1.0, 1.0));
        // The caps are half circles round the arc's ends, 4 out.
        let cap = Arc::from_bulge(points[1], points[2], bulges[1]).unwrap();
        assert!(close(cap.center, (0.0, 40.0)) && (cap.radius - 4.0).abs() < 1e-9);
        // Clockwise, and past a half circle: the arcs split, the caps the other way.
        let big = Arc {
            sweep: -1.5 * PI,
            ..arc
        };
        let (points, bulges) = arc_slot(&big, 8.0).unwrap();
        assert_eq!(points.len(), 6);
        assert!(bulges.iter().all(|b| b.abs() <= 1.0));
        assert_eq!((bulges[2], bulges[5]), (-1.0, -1.0));
        // Too wide for the radius, no width, or caps meeting across a small gap: none.
        assert!(arc_slot(&arc, 80.0).is_none());
        assert!(arc_slot(&arc, 0.0).is_none());
        let nearly_round = Arc {
            sweep: TAU - 0.1,
            ..arc
        };
        assert!(arc_slot(&nearly_round, 8.0).is_none());
        assert!(
            arc_slot(
                &Arc {
                    sweep: TAU - 0.5,
                    ..arc
                },
                8.0
            )
            .is_some()
        );
    }
}
