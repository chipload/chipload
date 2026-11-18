// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing a drawn shape's defining values: the handles the GUI shows, what dragging one
//! does, and inserting or removing polyline points (docs/specs/drawing-follow-ups.md
//! sections 1 and 3). Plain values in, plain values out, so every rule is tested here.

use chipload_core::math;
use std::f64::consts::{PI, TAU};

use chipload_core::primitives::Arc;
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::models::{Geometry, polygon_corners, polyline_segment_count, rotated, shape_problems};

/// A draggable point of a drawn shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// Circle or rectangle centre: moves the shape.
    Center,
    /// A point on the circle: changes the radius.
    Radius,
    /// Rectangle corner, in the order of its segments (from local (−l/2, −w/2), counter-
    /// clockwise): resizes it, the opposite corner stays put.
    Corner(usize),
    /// Beyond the rectangle's local +X end: turns it about its centre.
    Rotate,
    /// Polyline point: moves it.
    Point(usize),
    /// Middle of polyline segment i: moves the segment, inserts a point there or bends it,
    /// as [`SegmentDrag`] says.
    Middle(usize),
}

/// What dragging a polyline segment does (the user, 2026-10-28): a plain drag moves it,
/// Ctrl inserts a point, Shift bends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SegmentDrag {
    /// Both its ends move with the pointer; the segments either side stretch to stay
    /// joined.
    #[default]
    Move,
    /// A new point where the pointer is, both halves straight.
    Insert,
    /// Its bulge: the arc bows out through the pointer.
    Bend,
}

/// Every handle of `geometry` and where it is. The rotation handle sits `rotate_reach` mm
/// beyond the rectangle's end (the GUI passes a fixed distance on screen). Imported paths
/// have none.
pub fn handles(geometry: &Geometry, rotate_reach: f64) -> Vec<(Handle, [f64; 2])> {
    match geometry {
        Geometry::Path { .. } => Vec::new(),
        Geometry::Circle { center, radius } => vec![
            (Handle::Center, *center),
            (Handle::Radius, [center[0] + radius, center[1]]),
        ],
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            ..
        } => {
            let mut list = vec![(Handle::Center, *center)];
            for (k, corner) in rectangle_corners(*center, *length, *width, *angle)
                .into_iter()
                .enumerate()
            {
                list.push((Handle::Corner(k), corner));
            }
            let (sin, cos) = math::sin_cos(*angle);
            let reach = length / 2.0 + rotate_reach;
            list.push((
                Handle::Rotate,
                [center[0] + reach * cos, center[1] + reach * sin],
            ));
            list
        }
        // The centre, and the first corner (radius and turn).
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => {
            let mut list = vec![(Handle::Center, *center)];
            if let Some(&first) = polygon_corners(*center, *radius, *sides, *angle).first() {
                list.push((Handle::Radius, first));
            }
            list
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
            ..
        } => {
            let mut list: Vec<(Handle, [f64; 2])> = points
                .iter()
                .enumerate()
                .map(|(i, &p)| (Handle::Point(i), p))
                .collect();
            for i in 0..polyline_segment_count(points.len(), *closed) {
                let (a, b) = (points[i], points[(i + 1) % points.len()]);
                let bulge = bulges.get(i).copied().unwrap_or(0.0);
                list.push((Handle::Middle(i), segment_middle(a, b, bulge)));
            }
            list
        }
        // A point the curve passes through, each (docs/specs/splines.md §2).
        Geometry::Spline { points, .. } => points
            .iter()
            .enumerate()
            .map(|(i, &p)| (Handle::Point(i), p))
            .collect(),
    }
}

/// A length of a drawn shape shown on the canvas, which can be typed
/// (docs/specs/drawing-input.md §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    /// A straight polyline segment's length.
    Segment(usize),
    /// A rectangle's length, along its own X axis.
    Length,
    /// A rectangle's width, along its own Y axis.
    Width,
    /// A circle's radius, or a polygon's to its corners.
    Radius,
}

/// A length of a drawn shape as it shows: what it is, its value (mm), and where.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown {
    pub dimension: Dimension,
    pub value: f64,
    /// The middle of the side it measures (a radius: halfway out).
    pub at: [f64; 2],
    /// The side's outward direction, a unit vector, to stand the label off it; zero for a
    /// radius, whose label sits inside.
    pub outward: [f64; 2],
    /// How long the side is, mm (a radius: the radius), so a label that wouldn't fit
    /// beside it can be left out.
    pub span: f64,
}

/// The lengths of a drawn shape to show (docs/specs/drawing-input.md §6): each straight
/// polyline segment, a rectangle's length and width on its first two sides, a circle's or
/// polygon's radius halfway out to the first corner (a circle: at 45°). Arcs of a
/// polyline have none.
pub fn dimensions(geometry: &Geometry) -> Vec<Shown> {
    let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    // The right-hand normal of a → b: outward for a counter-clockwise loop.
    let right = |a: [f64; 2], b: [f64; 2]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = math::hypot(dx, dy);
        if len > 0.0 {
            [dy / len, -dx / len]
        } else {
            [0.0, 0.0]
        }
    };
    let radius_at = |center: &[f64; 2], radius: f64, angle: f64| {
        let (sin, cos) = math::sin_cos(angle);
        [
            center[0] + radius * cos / 2.0,
            center[1] + radius * sin / 2.0,
        ]
    };
    match geometry {
        // A curve has no straight sides to measure.
        Geometry::Path { .. } | Geometry::Spline { .. } => Vec::new(),
        Geometry::Circle { center, radius } => vec![Shown {
            dimension: Dimension::Radius,
            value: *radius,
            at: radius_at(center, *radius, std::f64::consts::FRAC_PI_4),
            outward: [0.0, 0.0],
            span: *radius,
        }],
        Geometry::Polygon {
            center,
            radius,
            angle,
            ..
        } => vec![Shown {
            dimension: Dimension::Radius,
            value: *radius,
            at: radius_at(center, *radius, *angle),
            outward: [0.0, 0.0],
            span: *radius,
        }],
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            ..
        } => {
            let c = rectangle_corners(*center, *length, *width, *angle);
            vec![
                Shown {
                    dimension: Dimension::Length,
                    value: *length,
                    at: mid(c[0], c[1]),
                    outward: right(c[0], c[1]),
                    span: *length,
                },
                Shown {
                    dimension: Dimension::Width,
                    value: *width,
                    at: mid(c[1], c[2]),
                    outward: right(c[1], c[2]),
                    span: *width,
                },
            ]
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
            ..
        } => {
            // Outward is to the right going counter-clockwise, to the left going clockwise;
            // an open polyline's labels stand to the left.
            let area: f64 = (0..points.len())
                .map(|i| {
                    let (a, b) = (points[i], points[(i + 1) % points.len()]);
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum();
            let flip = if *closed && area > 0.0 { 1.0 } else { -1.0 };
            (0..polyline_segment_count(points.len(), *closed))
                .filter(|&i| bulges.get(i).copied().unwrap_or(0.0) == 0.0)
                .map(|i| {
                    let (a, b) = (points[i], points[(i + 1) % points.len()]);
                    let n = right(a, b);
                    let length = math::hypot(b[0] - a[0], b[1] - a[1]);
                    Shown {
                        dimension: Dimension::Segment(i),
                        value: length,
                        at: mid(a, b),
                        outward: [n[0] * flip, n[1] * flip],
                        span: length,
                    }
                })
                .collect()
        }
    }
}

/// The shape with one of its lengths set to `value` (mm): a polyline segment's end point
/// moves along the segment (its start and the other points stay); a rectangle keeps its
/// centre; a circle or polygon its centre and turn. An error when the dimension isn't the
/// shape's, or the result is invalid (the reason, as for a drag).
pub fn with_dimension(
    geometry: &Geometry,
    dimension: Dimension,
    value: f64,
) -> Result<Geometry, String> {
    let mut out = geometry.clone();
    match (&mut out, dimension) {
        (Geometry::Circle { radius, .. }, Dimension::Radius)
        | (Geometry::Polygon { radius, .. }, Dimension::Radius) => *radius = value,
        (Geometry::Rectangle { length, .. }, Dimension::Length) => *length = value,
        (Geometry::Rectangle { width, .. }, Dimension::Width) => *width = value,
        (
            Geometry::Polyline {
                closed,
                points,
                bulges,
                ..
            },
            Dimension::Segment(i),
        ) if i < polyline_segment_count(points.len(), *closed)
            && bulges.get(i).copied().unwrap_or(0.0) == 0.0 =>
        {
            let n = points.len();
            let (a, b) = (points[i], points[(i + 1) % n]);
            let now = math::hypot(b[0] - a[0], b[1] - a[1]);
            if now <= 0.0 {
                return Err("the segment has no direction".into());
            }
            let k = value / now;
            points[(i + 1) % n] = [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k];
        }
        _ => return Err(format!("{dimension:?} isn't a length of this shape")),
    }
    let problems = shape_problems(&out);
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; "))
    }
}

/// Corners of a rectangle, counter-clockwise from local (−length/2, −width/2), as its
/// segments go.
pub fn rectangle_corners(center: [f64; 2], length: f64, width: f64, angle: f64) -> [[f64; 2]; 4] {
    let (sin, cos) = math::sin_cos(angle);
    let (hl, hw) = (length / 2.0, width / 2.0);
    [(-hl, -hw), (hl, -hw), (hl, hw), (-hl, hw)]
        .map(|(x, y)| [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos])
}

/// The point halfway along a polyline segment: on its arc when it bends.
pub fn segment_middle(a: [f64; 2], b: [f64; 2], bulge: f64) -> [f64; 2] {
    match Arc::from_bulge((a[0], a[1]), (b[0], b[1]), bulge) {
        Some(arc) => {
            let m = arc.point_at(arc.start_angle + arc.sweep / 2.0);
            [m.0, m.1]
        }
        None => [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
    }
}

/// A slot: a rectangle whose corner radius is half its shorter side, so its ends are half
/// circles (docs/specs/drawing-input.md §4).
pub fn is_slot(geometry: &Geometry) -> bool {
    match geometry {
        Geometry::Rectangle {
            length,
            width,
            corner_radius,
            ..
        } => {
            *corner_radius > 0.0 && (corner_radius - length.min(*width) / 2.0).abs() <= CHAIN_TOL_MM
        }
        _ => false,
    }
}

/// `new` with a slot kept a slot: when `old` is a slot and `new` resizes it with the corner
/// radius left as it was, the radius becomes half the new shorter side.
pub fn kept_slot(old: &Geometry, new: Geometry) -> Geometry {
    let (
        Geometry::Rectangle {
            length: old_length,
            width: old_width,
            corner_radius: old_radius,
            ..
        },
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        },
    ) = (old, &new)
    else {
        return new;
    };
    let resized = (length, width) != (old_length, old_width);
    if !is_slot(old) || !resized || corner_radius != old_radius {
        return new;
    }
    Geometry::Rectangle {
        center: *center,
        length: *length,
        width: *width,
        angle: *angle,
        corner_radius: length.min(*width) / 2.0,
    }
}

/// The shape after dragging `handle` from where it was in `start` (the shape when the drag
/// began) to `to`. `segment` says what a segment middle's drag does (other handles ignore
/// it). Always computed from `start`, so a long drag doesn't drift. None when the result
/// would be invalid (the caller keeps the last valid shape) or the handle doesn't belong to
/// the shape.
pub fn drag(
    start: &Geometry,
    handle: Handle,
    to: [f64; 2],
    segment: SegmentDrag,
) -> Option<Geometry> {
    try_drag(start, handle, to, segment).ok()
}

/// As [`drag`], with the reason when the result would be invalid.
pub fn try_drag(
    start: &Geometry,
    handle: Handle,
    to: [f64; 2],
    segment: SegmentDrag,
) -> Result<Geometry, String> {
    let mut geometry = start.clone();
    let slot = is_slot(start);
    match (&mut geometry, handle) {
        (Geometry::Circle { center, .. }, Handle::Center)
        | (Geometry::Rectangle { center, .. }, Handle::Center)
        | (Geometry::Polygon { center, .. }, Handle::Center) => *center = to,
        (
            Geometry::Polygon {
                center,
                radius,
                angle,
                ..
            },
            Handle::Radius,
        ) => {
            let (dx, dy) = (to[0] - center[0], to[1] - center[1]);
            if dx == 0.0 && dy == 0.0 {
                return Err("the corner handle is on the centre".into());
            }
            *radius = math::hypot(dx, dy);
            *angle = math::atan2(dy, dx);
        }
        (Geometry::Circle { center, radius }, Handle::Radius) => {
            *radius = math::hypot(to[0] - center[0], to[1] - center[1]);
        }
        (
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
                corner_radius,
            },
            Handle::Corner(k),
        ) if k < 4 => {
            let opposite = rectangle_corners(*center, *length, *width, *angle)[(k + 2) % 4];
            let (sin, cos) = math::sin_cos(*angle);
            let d = [to[0] - opposite[0], to[1] - opposite[1]];
            // In the rectangle's own axes; the angle stays.
            *length = (d[0] * cos + d[1] * sin).abs();
            *width = (-d[0] * sin + d[1] * cos).abs();
            *center = [(opposite[0] + to[0]) / 2.0, (opposite[1] + to[1]) / 2.0];
            // A slot stays a slot.
            if slot {
                *corner_radius = length.min(*width) / 2.0;
            }
        }
        (Geometry::Rectangle { center, angle, .. }, Handle::Rotate) => {
            let (dx, dy) = (to[0] - center[0], to[1] - center[1]);
            if dx == 0.0 && dy == 0.0 {
                return Err("the rotation handle is on the centre".into());
            }
            *angle = math::atan2(dy, dx);
        }
        (Geometry::Polyline { points, .. } | Geometry::Spline { points, .. }, Handle::Point(i))
            if i < points.len() =>
        {
            points[i] = to;
        }
        (
            Geometry::Polyline {
                closed,
                points,
                bulges,
                radii,
            },
            Handle::Middle(i),
        ) if i < polyline_segment_count(points.len(), *closed) && i < bulges.len() => {
            let j = (i + 1) % points.len();
            let (a, b) = (points[i], points[j]);
            match segment {
                SegmentDrag::Move => {
                    // The segment's middle goes to `to`: both ends move by as much, the
                    // neighbours (straight or arcs, their bulges kept) follow its ends.
                    let middle = segment_middle(a, b, bulges[i]);
                    let d = [to[0] - middle[0], to[1] - middle[1]];
                    points[i] = [a[0] + d[0], a[1] + d[1]];
                    points[j] = [b[0] + d[0], b[1] + d[1]];
                }
                SegmentDrag::Insert => {
                    // A new point at `to`; both halves straight.
                    points.insert(i + 1, to);
                    bulges[i] = 0.0;
                    bulges.insert(i + 1, 0.0);
                    if !radii.is_empty() {
                        radii.insert(i + 1, 0.0);
                    }
                }
                SegmentDrag::Bend => bulges[i] = bulge_through(a, b, to),
            }
        }
        _ => return Err(format!("{handle:?} isn't a handle of this shape")),
    }
    let problems = shape_problems(&geometry);
    if problems.is_empty() {
        Ok(geometry)
    } else {
        Err(problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; "))
    }
}

/// The bulge whose arc from `a` to `b` bows out as far as `to` lies from the chord (signed:
/// to the right of a → b is positive, as a counter-clockwise arc bows), at most a half
/// circle either way.
pub fn bulge_through(a: [f64; 2], b: [f64; 2], to: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let chord = math::hypot(dx, dy);
    if chord == 0.0 {
        return 0.0;
    }
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let right = ((to[0] - mid[0]) * dy - (to[1] - mid[1]) * dx) / chord;
    (right / (chord / 2.0)).clamp(-1.0, 1.0)
}

/// Insert a point after point `i` (the form's "+"): in the middle of the segment that
/// follows it, which splits into two halves of the same arc (half the sweep each), or
/// 10 mm on in X after the last point of an open polyline. False if there's no such point.
pub fn insert_point_after(geometry: &mut Geometry, i: usize) -> bool {
    if let Geometry::Spline { closed, points } = geometry {
        return insert_spline_point_after(*closed, points, i);
    }
    let Geometry::Polyline {
        closed,
        points,
        bulges,
        radii,
    } = geometry
    else {
        return false;
    };
    if i >= points.len() {
        return false;
    }
    // A new point starts sharp (docs/specs/polyline-radii.md §4).
    if !radii.is_empty() {
        radii.insert(i + 1, 0.0);
    }
    let segments = polyline_segment_count(points.len(), *closed);
    if i < segments && i < bulges.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        let middle = segment_middle(a, b, bulges[i]);
        // tan(sweep / 4) for half the sweep: tan(atan(bulge) / 2).
        let half = math::tan(math::atan(bulges[i]) / 2.0);
        points.insert(i + 1, middle);
        bulges[i] = half;
        bulges.insert(i + 1, half);
    } else {
        let p = points[i];
        points.push([p[0] + 10.0, p[1]]);
        bulges.push(0.0);
    }
    true
}

/// A spline's point after point `i`: on the curve halfway along the piece that follows it,
/// or 10 mm on in X after the last point of an open spline.
fn insert_spline_point_after(closed: bool, points: &mut Vec<[f64; 2]>, i: usize) -> bool {
    if i >= points.len() {
        return false;
    }
    let at: Vec<chipload_core::Point> = points.iter().map(|p| (p[0], p[1])).collect();
    let pieces = chipload_core::spline::beziers(&at, closed);
    match pieces.get(i) {
        Some(piece) if i + 1 < points.len() || closed => {
            let (x, y) = chipload_core::spline::point_at(piece, 0.5);
            points.insert(i + 1, [x, y]);
        }
        _ => {
            let p = points[i];
            points.insert(i + 1, [p[0] + 10.0, p[1]]);
        }
    }
    true
}

/// Remove point `i`. Where it joined two segments, the one that replaces them is straight;
/// an end point of an open polyline takes its segment with it. False if there's no such
/// point.
pub fn remove_point(geometry: &mut Geometry, i: usize) -> bool {
    if let Geometry::Spline { points, .. } = geometry {
        if i >= points.len() {
            return false;
        }
        points.remove(i);
        return true;
    }
    let Geometry::Polyline {
        closed,
        points,
        bulges,
        radii,
    } = geometry
    else {
        return false;
    };
    let n = points.len();
    if i >= n || bulges.len() != polyline_segment_count(n, *closed) || bulges.is_empty() {
        return false;
    }
    points.remove(i);
    if radii.len() == n {
        radii.remove(i);
        // An open polyline's new end has no corner.
        if !*closed {
            if let Some(first) = radii.first_mut() {
                *first = 0.0;
            }
            if let Some(last) = radii.last_mut() {
                *last = 0.0;
            }
        }
    }
    if !*closed && i == 0 {
        bulges.remove(0);
    } else if !*closed && i == n - 1 {
        bulges.pop();
    } else {
        // Segments i − 1 and i (cyclically) become one.
        let before = (i + n - 1) % n;
        bulges.remove(i);
        let joined = if before > i { before - 1 } else { before };
        bulges[joined] = 0.0;
    }
    true
}

/// Open or close a polyline: closing adds a straight segment back to the first point,
/// opening removes that last segment.
pub fn set_closed(geometry: &mut Geometry, close: bool) {
    if let Geometry::Spline { closed, .. } = geometry {
        *closed = close;
        return;
    }
    if let Geometry::Polyline {
        closed,
        points,
        bulges,
        radii,
    } = geometry
        && *closed != close
    {
        *closed = close;
        bulges.resize(polyline_segment_count(points.len(), close), 0.0);
        // Opened: its ends are no corners.
        if !close {
            if let Some(first) = radii.first_mut() {
                *first = 0.0;
            }
            if let Some(last) = radii.last_mut() {
                *last = 0.0;
            }
        }
    }
}

/// A drawn shape moved by `by` (copy and paste, docs/specs/small-items.md §4). None for an
/// imported path, which only changes with the DXF.
pub fn translated(geometry: &Geometry, by: [f64; 2]) -> Option<Geometry> {
    let shift = |p: &[f64; 2]| [p[0] + by[0], p[1] + by[1]];
    match geometry {
        Geometry::Path { .. } => None,
        Geometry::Circle { center, radius } => Some(Geometry::Circle {
            center: shift(center),
            radius: *radius,
        }),
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } => Some(Geometry::Rectangle {
            center: shift(center),
            length: *length,
            width: *width,
            angle: *angle,
            corner_radius: *corner_radius,
        }),
        Geometry::Polyline {
            closed,
            points,
            bulges,
            radii,
        } => Some(Geometry::Polyline {
            closed: *closed,
            points: points.iter().map(shift).collect(),
            bulges: bulges.clone(),
            radii: radii.clone(),
        }),
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => Some(Geometry::Polygon {
            center: shift(center),
            radius: *radius,
            sides: *sides,
            angle: *angle,
        }),
        Geometry::Spline { closed, points } => Some(Geometry::Spline {
            closed: *closed,
            points: points.iter().map(shift).collect(),
        }),
    }
}

/// `p` moved, turned and scaled as [`transformed`] moves a shape's points:
/// `to + scale · turn(p − from)`.
pub fn transformed_point(
    p: [f64; 2],
    from: [f64; 2],
    to: [f64; 2],
    scale: f64,
    degrees: f64,
) -> [f64; 2] {
    let turned = rotated([p[0] - from[0], p[1] - from[1]], degrees);
    [to[0] + scale * turned[0], to[1] + scale * turned[1]]
}

/// A drawn shape moved, turned and scaled: each point p goes to
/// `to + scale · turn(p − from)`, `degrees` counter-clockwise (docs/specs/drawn-shapes-box.md
/// §2). A circle's radius and a rectangle's sides and corner radius scale; a rectangle's
/// angle turns; a polyline's bulges stay, since an even scale and a turn keep every arc's
/// sweep. Turns by multiples of 90° are exact. None for an imported path or a scale that
/// isn't a positive number.
pub fn transformed(
    geometry: &Geometry,
    from: [f64; 2],
    to: [f64; 2],
    scale: f64,
    degrees: f64,
) -> Option<Geometry> {
    if !(scale.is_finite() && scale > 0.0 && degrees.is_finite()) {
        return None;
    }
    let map = |p: &[f64; 2]| transformed_point(*p, from, to, scale, degrees);
    match geometry {
        Geometry::Path { .. } => None,
        Geometry::Circle { center, radius } => Some(Geometry::Circle {
            center: map(center),
            radius: radius * scale,
        }),
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } => Some(Geometry::Rectangle {
            center: map(center),
            length: length * scale,
            width: width * scale,
            angle: turned_angle(*angle, degrees),
            corner_radius: corner_radius * scale,
        }),
        Geometry::Polyline {
            closed,
            points,
            bulges,
            radii,
        } => Some(Geometry::Polyline {
            closed: *closed,
            points: points.iter().map(map).collect(),
            bulges: bulges.clone(),
            radii: radii.iter().map(|r| r * scale).collect(),
        }),
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => Some(Geometry::Polygon {
            center: map(center),
            radius: radius * scale,
            sides: *sides,
            angle: turned_angle(*angle, degrees),
        }),
        // The curve depends on distances only: it moves, turns and scales with its points
        // (docs/specs/splines.md §5).
        Geometry::Spline { closed, points } => Some(Geometry::Spline {
            closed: *closed,
            points: points.iter().map(map).collect(),
        }),
    }
}

/// A rectangle's angle (radians) turned by `degrees`, in (−π, π]: a rectangle turned half
/// a turn is the same rectangle, but keeping the angle small keeps the file readable.
fn turned_angle(angle: f64, degrees: f64) -> f64 {
    if degrees == 0.0 {
        return angle;
    }
    within_half_turn(angle + degrees.to_radians())
}

/// An angle (radians) in (−π, π].
fn within_half_turn(angle: f64) -> f64 {
    let turned = angle.rem_euclid(TAU);
    if turned > PI { turned - TAU } else { turned }
}

/// Which way a mirror flips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirror {
    /// About a vertical line `x = about`: left becomes right.
    LeftRight,
    /// About a horizontal line `y = about`: up becomes down.
    UpDown,
}

/// A drawn shape mirrored about the line `about` (docs/specs/small-items.md §4). A
/// rectangle's angle becomes −angle (half a turn gives the same rectangle, so this holds
/// both ways); a polyline's bulges change sign, so its arcs bend the mirrored way. None for
/// an imported path.
pub fn mirrored(geometry: &Geometry, mirror: Mirror, about: f64) -> Option<Geometry> {
    let flip = |p: &[f64; 2]| match mirror {
        Mirror::LeftRight => [2.0 * about - p[0], p[1]],
        Mirror::UpDown => [p[0], 2.0 * about - p[1]],
    };
    // `0.0 - v`, not `-v`: a 0 stays 0, never -0, in the saved file.
    let negated = |v: f64| 0.0 - v;
    match geometry {
        Geometry::Path { .. } => None,
        Geometry::Circle { center, radius } => Some(Geometry::Circle {
            center: flip(center),
            radius: *radius,
        }),
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } => Some(Geometry::Rectangle {
            center: flip(center),
            length: *length,
            width: *width,
            angle: negated(*angle),
            corner_radius: *corner_radius,
        }),
        Geometry::Polyline {
            closed,
            points,
            bulges,
            radii,
        } => Some(Geometry::Polyline {
            closed: *closed,
            points: points.iter().map(flip).collect(),
            bulges: bulges.iter().map(|b| negated(*b)).collect(),
            radii: radii.clone(),
        }),
        // The first corner is mirrored: its angle reflects about the mirror's direction.
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => Some(Geometry::Polygon {
            center: flip(center),
            radius: *radius,
            sides: *sides,
            angle: match mirror {
                Mirror::LeftRight => within_half_turn(PI - angle),
                Mirror::UpDown => negated(*angle),
            },
        }),
        Geometry::Spline { closed, points } => Some(Geometry::Spline {
            closed: *closed,
            points: points.iter().map(flip).collect(),
        }),
    }
}

/// `p` reflected about the line through `a` and `b` (docs/specs/drawing-improvements.md
/// §1); `p` itself when they coincide. A horizontal or vertical line reflects exactly.
pub fn reflected_point(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = math::hypot(dx, dy);
    if len == 0.0 {
        return p;
    }
    let (ux, uy) = (dx / len, dy / len);
    let (px, py) = (p[0] - a[0], p[1] - a[1]);
    let along = px * ux + py * uy;
    // The foot on the line, then as far again beyond it.
    let foot = [a[0] + along * ux, a[1] + along * uy];
    [2.0 * foot[0] - p[0], 2.0 * foot[1] - p[1]]
}

/// A drawn shape reflected about the line through `a` and `b`: points reflected, a
/// rectangle's angle and a polygon's first corner turned to 2θ − angle (θ the line's
/// direction), a polyline's bulges negated. None for an imported path, or when `a` and `b`
/// coincide.
pub fn mirrored_about(geometry: &Geometry, a: [f64; 2], b: [f64; 2]) -> Option<Geometry> {
    if a == b {
        return None;
    }
    let flip = |p: &[f64; 2]| reflected_point(*p, a, b);
    let theta = math::atan2(b[1] - a[1], b[0] - a[0]);
    let turned = |angle: f64| within_half_turn(2.0 * theta - angle);
    let negated = |v: f64| 0.0 - v;
    // A rectangle is the same turned half a turn: about a horizontal or vertical line
    // (2θ a whole number of half turns) its angle is exactly −angle, so mirroring twice
    // gives it back bit for bit.
    let rectangle_turned = |angle: f64| {
        let shift = (2.0 * theta).rem_euclid(PI);
        if shift == 0.0 {
            negated(angle)
        } else {
            within_half_turn(shift - angle)
        }
    };
    match geometry {
        Geometry::Path { .. } => None,
        Geometry::Circle { center, radius } => Some(Geometry::Circle {
            center: flip(center),
            radius: *radius,
        }),
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } => Some(Geometry::Rectangle {
            center: flip(center),
            length: *length,
            width: *width,
            angle: rectangle_turned(*angle),
            corner_radius: *corner_radius,
        }),
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => Some(Geometry::Polygon {
            center: flip(center),
            radius: *radius,
            sides: *sides,
            angle: turned(*angle),
        }),
        Geometry::Polyline {
            closed,
            points,
            bulges,
            radii,
        } => Some(Geometry::Polyline {
            closed: *closed,
            points: points.iter().map(flip).collect(),
            bulges: bulges.iter().map(|b| negated(*b)).collect(),
            radii: radii.clone(),
        }),
        Geometry::Spline { closed, points } => Some(Geometry::Spline {
            closed: *closed,
            points: points.iter().map(flip).collect(),
        }),
    }
}

/// Where a point on a drawn shape goes when the shape changes from `old` to `new` through
/// its own handles or form (docs/specs/dogbone-per-corner.md §4): a rectangle's point keeps
/// its place in the rectangle (a corner stays that corner), a circle's its angle and share
/// of the radius, and a polyline's its vertex or its place along the same segment, if the
/// polyline keeps its number of points. None when there is no such place (another kind, or
/// points added or removed): the point stays.
pub fn followed_point(p: [f64; 2], old: &Geometry, new: &Geometry) -> Option<[f64; 2]> {
    match (old, new) {
        (
            Geometry::Rectangle {
                center: c0,
                length: l0,
                width: w0,
                angle: a0,
                ..
            },
            Geometry::Rectangle {
                center: c1,
                length: l1,
                width: w1,
                angle: a1,
                ..
            },
        ) => {
            if *l0 <= 0.0 || *w0 <= 0.0 {
                return None;
            }
            // In the old rectangle's frame, as shares of its half sides.
            let (s0, k0) = math::sin_cos(*a0);
            let (dx, dy) = (p[0] - c0[0], p[1] - c0[1]);
            let u = (dx * k0 + dy * s0) / (l0 / 2.0);
            let v = (-dx * s0 + dy * k0) / (w0 / 2.0);
            let (x, y) = (u * l1 / 2.0, v * w1 / 2.0);
            let (s1, k1) = math::sin_cos(*a1);
            Some([c1[0] + x * k1 - y * s1, c1[1] + x * s1 + y * k1])
        }
        (
            Geometry::Circle {
                center: c0,
                radius: r0,
            },
            Geometry::Circle {
                center: c1,
                radius: r1,
            },
        ) => {
            if *r0 <= 0.0 {
                return None;
            }
            let k = r1 / r0;
            Some([c1[0] + (p[0] - c0[0]) * k, c1[1] + (p[1] - c0[1]) * k])
        }
        // As a circle's, turned with the first corner.
        (
            Geometry::Polygon {
                center: c0,
                radius: r0,
                angle: a0,
                ..
            },
            Geometry::Polygon {
                center: c1,
                radius: r1,
                angle: a1,
                ..
            },
        ) => {
            if *r0 <= 0.0 {
                return None;
            }
            let k = r1 / r0;
            let turned = rotated([p[0] - c0[0], p[1] - c0[1]], (a1 - a0).to_degrees());
            Some([c1[0] + turned[0] * k, c1[1] + turned[1] * k])
        }
        (Geometry::Polyline { points: p0, .. }, Geometry::Polyline { points: p1, .. })
            if p0.len() == p1.len() && !p0.is_empty() =>
        {
            let n = p0.len();
            // Its vertex, else the nearest chord and the share along it.
            let mut best: Option<(f64, usize, f64)> = None;
            for i in 0..n {
                let (a, b) = (p0[i], p0[(i + 1) % n]);
                let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
                let len2 = ex * ex + ey * ey;
                let t = if len2 > 0.0 {
                    (((p[0] - a[0]) * ex + (p[1] - a[1]) * ey) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let q = [a[0] + ex * t, a[1] + ey * t];
                let d = math::hypot(p[0] - q[0], p[1] - q[1]);
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, i, t));
                }
            }
            let (_, i, t) = best?;
            let (a, b) = (p1[i], p1[(i + 1) % n]);
            Some([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
