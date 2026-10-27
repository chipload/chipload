// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing a drawn shape's defining values: the handles the GUI shows, what dragging one
//! does, and inserting or removing polyline points (docs/specs/drawing-follow-ups.md
//! sections 1 and 3). Plain values in, plain values out, so every rule is tested here.

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
    /// Middle of polyline segment i: inserts a point there, or with `bend` sets its bulge.
    Middle(usize),
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
            let (sin, cos) = angle.sin_cos();
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
        let len = dx.hypot(dy);
        if len > 0.0 {
            [dy / len, -dx / len]
        } else {
            [0.0, 0.0]
        }
    };
    let radius_at = |center: &[f64; 2], radius: f64, angle: f64| {
        let (sin, cos) = angle.sin_cos();
        [
            center[0] + radius * cos / 2.0,
            center[1] + radius * sin / 2.0,
        ]
    };
    match geometry {
        Geometry::Path { .. } => Vec::new(),
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
                    let length = (b[0] - a[0]).hypot(b[1] - a[1]);
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
            },
            Dimension::Segment(i),
        ) if i < polyline_segment_count(points.len(), *closed)
            && bulges.get(i).copied().unwrap_or(0.0) == 0.0 =>
        {
            let n = points.len();
            let (a, b) = (points[i], points[(i + 1) % n]);
            let now = (b[0] - a[0]).hypot(b[1] - a[1]);
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
    let (sin, cos) = angle.sin_cos();
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
/// began) to `to`. `bend` is the Shift-drag of a segment middle. Always computed from
/// `start`, so a long drag doesn't drift. None when the result would be invalid (the caller
/// keeps the last valid shape) or the handle doesn't belong to the shape.
pub fn drag(start: &Geometry, handle: Handle, to: [f64; 2], bend: bool) -> Option<Geometry> {
    try_drag(start, handle, to, bend).ok()
}

/// As [`drag`], with the reason when the result would be invalid.
pub fn try_drag(
    start: &Geometry,
    handle: Handle,
    to: [f64; 2],
    bend: bool,
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
            *radius = dx.hypot(dy);
            *angle = dy.atan2(dx);
        }
        (Geometry::Circle { center, radius }, Handle::Radius) => {
            *radius = (to[0] - center[0]).hypot(to[1] - center[1]);
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
            let (sin, cos) = angle.sin_cos();
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
            *angle = dy.atan2(dx);
        }
        (Geometry::Polyline { points, .. }, Handle::Point(i)) if i < points.len() => {
            points[i] = to;
        }
        (
            Geometry::Polyline {
                closed,
                points,
                bulges,
            },
            Handle::Middle(i),
        ) if i < polyline_segment_count(points.len(), *closed) && i < bulges.len() => {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            if bend {
                bulges[i] = bulge_through(a, b, to);
            } else {
                // A new point at `to`; both halves straight.
                points.insert(i + 1, to);
                bulges[i] = 0.0;
                bulges.insert(i + 1, 0.0);
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
    let chord = dx.hypot(dy);
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
    let Geometry::Polyline {
        closed,
        points,
        bulges,
    } = geometry
    else {
        return false;
    };
    if i >= points.len() {
        return false;
    }
    let segments = polyline_segment_count(points.len(), *closed);
    if i < segments && i < bulges.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        let middle = segment_middle(a, b, bulges[i]);
        // tan(sweep / 4) for half the sweep: tan(atan(bulge) / 2).
        let half = (bulges[i].atan() / 2.0).tan();
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

/// Remove point `i`. Where it joined two segments, the one that replaces them is straight;
/// an end point of an open polyline takes its segment with it. False if there's no such
/// point.
pub fn remove_point(geometry: &mut Geometry, i: usize) -> bool {
    let Geometry::Polyline {
        closed,
        points,
        bulges,
    } = geometry
    else {
        return false;
    };
    let n = points.len();
    if i >= n || bulges.len() != polyline_segment_count(n, *closed) || bulges.is_empty() {
        return false;
    }
    points.remove(i);
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
    if let Geometry::Polyline {
        closed,
        points,
        bulges,
    } = geometry
        && *closed != close
    {
        *closed = close;
        bulges.resize(polyline_segment_count(points.len(), close), 0.0);
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
        } => Some(Geometry::Polyline {
            closed: *closed,
            points: points.iter().map(shift).collect(),
            bulges: bulges.clone(),
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
        } => Some(Geometry::Polyline {
            closed: *closed,
            points: points.iter().map(map).collect(),
            bulges: bulges.clone(),
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
        } => Some(Geometry::Polyline {
            closed: *closed,
            points: points.iter().map(flip).collect(),
            bulges: bulges.iter().map(|b| negated(*b)).collect(),
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
            let (s0, k0) = a0.sin_cos();
            let (dx, dy) = (p[0] - c0[0], p[1] - c0[1]);
            let u = (dx * k0 + dy * s0) / (l0 / 2.0);
            let v = (-dx * s0 + dy * k0) / (w0 / 2.0);
            let (x, y) = (u * l1 / 2.0, v * w1 / 2.0);
            let (s1, k1) = a1.sin_cos();
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
                let d = (p[0] - q[0]).hypot(p[1] - q[1]);
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
mod tests {
    use super::*;
    use crate::models::{PathSegment, Shape};
    use std::f64::consts::FRAC_PI_2;

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
    }

    fn rect(angle: f64) -> Geometry {
        Geometry::Rectangle {
            center: [10.0, 20.0],
            length: 8.0,
            width: 4.0,
            angle,
            corner_radius: 0.0,
        }
    }

    fn corners(g: &Geometry) -> [[f64; 2]; 4] {
        let Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            ..
        } = *g
        else {
            panic!("rectangle");
        };
        rectangle_corners(center, length, width, angle)
    }

    fn triangle() -> Geometry {
        Geometry::Polyline {
            closed: true,
            points: vec![[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]],
            bulges: vec![0.0, 0.4, 0.0],
        }
    }

    #[test]
    fn circle_handles_move_it_and_set_the_radius() {
        let circle = Geometry::Circle {
            center: [1.0, 2.0],
            radius: 5.0,
        };
        let list = handles(&circle, 3.0);
        assert_eq!(
            list,
            [(Handle::Center, [1.0, 2.0]), (Handle::Radius, [6.0, 2.0])]
        );
        assert_eq!(
            drag(&circle, Handle::Center, [7.0, 8.0], false),
            Some(Geometry::Circle {
                center: [7.0, 8.0],
                radius: 5.0
            })
        );
        assert_eq!(
            drag(&circle, Handle::Radius, [1.0, 5.0], false),
            Some(Geometry::Circle {
                center: [1.0, 2.0],
                radius: 3.0
            })
        );
        // Radius 0 is invalid: the drag stops (the caller keeps the last valid shape).
        assert_eq!(drag(&circle, Handle::Radius, [1.0, 2.0], false), None);
        // Handles of other kinds don't apply.
        assert_eq!(drag(&circle, Handle::Rotate, [0.0, 0.0], false), None);
    }

    #[test]
    fn a_slot_stays_a_slot_when_its_size_changes() {
        let rect = |width: f64, corner_radius: f64| Geometry::Rectangle {
            center: [0.0, 0.0],
            length: 40.0,
            width,
            angle: 0.0,
            corner_radius,
        };
        let slot = rect(10.0, 5.0);
        assert!(is_slot(&slot));
        assert!(!is_slot(&rect(10.0, 2.0)));
        // Wider through the form: the radius follows.
        assert_eq!(kept_slot(&slot, rect(16.0, 5.0)), rect(16.0, 8.0));
        // A radius typed with it wins; a plain rounded rectangle is left alone.
        assert_eq!(kept_slot(&slot, rect(16.0, 3.0)), rect(16.0, 3.0));
        assert_eq!(
            kept_slot(&rect(10.0, 2.0), rect(16.0, 2.0)),
            rect(16.0, 2.0)
        );
        // A corner drag narrows it: still a slot (a plain rectangle with that radius
        // would refuse to get narrower than its corners).
        let narrower = try_drag(&slot, Handle::Corner(2), [20.0, 2.0], false).unwrap();
        assert!(is_slot(&narrower));
    }

    #[test]
    fn a_polygon_has_a_centre_and_a_corner_handle_and_mirrors_and_turns() {
        let hexagon = Geometry::Polygon {
            center: [0.0, 0.0],
            radius: 10.0,
            sides: 6,
            angle: 0.0,
        };
        let list = handles(&hexagon, 5.0);
        assert_eq!(
            list,
            [(Handle::Center, [0.0, 0.0]), (Handle::Radius, [10.0, 0.0])]
        );
        // Dragging the corner sets the radius and the turn.
        let Ok(Geometry::Polygon { radius, angle, .. }) =
            try_drag(&hexagon, Handle::Radius, [0.0, 20.0], false)
        else {
            panic!()
        };
        assert_eq!(radius, 20.0);
        assert!((angle - FRAC_PI_2).abs() < 1e-12);
        // Mirrored left to right, its first corner points the other way.
        let turned = Geometry::Polygon {
            center: [5.0, 0.0],
            radius: 10.0,
            sides: 5,
            angle: 0.3,
        };
        let Some(Geometry::Polygon { center, angle, .. }) =
            mirrored(&turned, Mirror::LeftRight, 0.0)
        else {
            panic!()
        };
        assert_eq!(center, [-5.0, 0.0]);
        assert!((angle - (PI - 0.3)).abs() < 1e-12);
        // Turned by 90° and scaled by 2 about the origin.
        let Some(Geometry::Polygon {
            center,
            radius,
            angle,
            ..
        }) = transformed(&turned, [0.0, 0.0], [0.0, 0.0], 2.0, 90.0)
        else {
            panic!()
        };
        assert_eq!((center, radius), ([0.0, 10.0], 20.0));
        assert!((angle - (0.3 + FRAC_PI_2)).abs() < 1e-12);
        // A point on its first corner follows the corner.
        let bigger = Geometry::Polygon {
            center: [0.0, 0.0],
            radius: 20.0,
            sides: 6,
            angle: FRAC_PI_2,
        };
        let moved = followed_point([10.0, 0.0], &hexagon, &bigger).unwrap();
        assert!((moved[0]).abs() < 1e-9 && (moved[1] - 20.0).abs() < 1e-9);
    }

    #[test]
    fn lengths_show_and_can_be_typed() {
        let l_shape = Geometry::Polyline {
            closed: false,
            points: vec![[0.0, 0.0], [30.0, 0.0], [30.0, 10.0], [40.0, 10.0]],
            bulges: vec![0.0, 0.0, 0.5],
        };
        // The arc segment has no label; an open polyline's stand to the left.
        let shown = dimensions(&l_shape);
        let brief: Vec<_> = shown
            .iter()
            .map(|s| (s.dimension, s.at, s.value, s.outward))
            .collect();
        assert_eq!(
            brief,
            [
                (Dimension::Segment(0), [15.0, 0.0], 30.0, [0.0, 1.0]),
                (Dimension::Segment(1), [30.0, 5.0], 10.0, [-1.0, 0.0])
            ]
        );
        // Segment 0 made 50 long: its end moves along it, the rest stays.
        let Ok(Geometry::Polyline { points, .. }) =
            with_dimension(&l_shape, Dimension::Segment(0), 50.0)
        else {
            panic!()
        };
        assert_eq!(
            points,
            [[0.0, 0.0], [50.0, 0.0], [30.0, 10.0], [40.0, 10.0]]
        );
        assert!(with_dimension(&l_shape, Dimension::Segment(2), 5.0).is_err());
        assert!(with_dimension(&l_shape, Dimension::Segment(0), 0.0).is_err());
        let rect = Geometry::Rectangle {
            center: [0.0, 0.0],
            length: 40.0,
            width: 20.0,
            angle: 0.0,
            corner_radius: 0.0,
        };
        // On the bottom and the right side, standing outward.
        let brief: Vec<_> = dimensions(&rect)
            .iter()
            .map(|s| (s.dimension, s.at, s.value, s.outward))
            .collect();
        assert_eq!(
            brief,
            [
                (Dimension::Length, [0.0, -10.0], 40.0, [0.0, -1.0]),
                (Dimension::Width, [20.0, 0.0], 20.0, [1.0, 0.0])
            ]
        );
        let Ok(Geometry::Rectangle { width, center, .. }) =
            with_dimension(&rect, Dimension::Width, 30.0)
        else {
            panic!()
        };
        assert_eq!((width, center), (30.0, [0.0, 0.0]));
        assert!(with_dimension(&rect, Dimension::Radius, 3.0).is_err());
    }

    #[test]
    fn a_dragged_corner_keeps_the_opposite_corner_and_the_angle() {
        for angle in [0.0, 0.5, -2.0] {
            let start = rect(angle);
            let before = corners(&start);
            for k in 0..4 {
                let (sin, cos) = f64::sin_cos(angle);
                // 3 mm further out along the local X and 1 mm along local Y.
                let to = [
                    before[k][0] + 3.0 * cos - 1.0 * sin,
                    before[k][1] + 3.0 * sin + 1.0 * cos,
                ];
                let moved = drag(&start, Handle::Corner(k), to, false).unwrap();
                let after = corners(&moved);
                let Geometry::Rectangle { angle: a, .. } = moved else {
                    panic!()
                };
                assert_eq!(a, angle);
                assert!(close(after[(k + 2) % 4], before[(k + 2) % 4]), "{k}");
                // The dragged corner is where the cursor is (perhaps as another index
                // when the drag flipped a side).
                assert!(after.iter().any(|&c| close(c, to)), "{k} {after:?}");
            }
        }
        // Onto the opposite corner's line: no width left, refused.
        let start = rect(0.0);
        let opposite = corners(&start)[2];
        assert_eq!(
            drag(&start, Handle::Corner(0), [0.0, opposite[1]], false),
            None
        );
    }

    #[test]
    fn the_rotate_handle_changes_only_the_angle() {
        let start = rect(0.0);
        let rotate = handles(&start, 5.0)
            .into_iter()
            .find(|(h, _)| *h == Handle::Rotate)
            .unwrap();
        assert!(close(rotate.1, [10.0 + 4.0 + 5.0, 20.0]));
        let turned = drag(&start, Handle::Rotate, [10.0, 30.0], false).unwrap();
        assert_eq!(
            turned,
            Geometry::Rectangle {
                center: [10.0, 20.0],
                length: 8.0,
                width: 4.0,
                angle: std::f64::consts::FRAC_PI_2,
                corner_radius: 0.0,
            }
        );
        let moved = drag(&start, Handle::Center, [0.0, 0.0], false).unwrap();
        assert!(matches!(
            moved,
            Geometry::Rectangle {
                center: [0.0, 0.0],
                length: 8.0,
                ..
            }
        ));
    }

    #[test]
    fn polyline_points_move_and_middles_insert_or_bend() {
        let start = triangle();
        let list = handles(&start, 0.0);
        assert_eq!(list.len(), 6);
        // The middle of the bulged segment is on its arc.
        let shape = Shape {
            id: 1,
            layer: String::new(),
            name: None,
            geometry: start.clone(),
        };
        let PathSegment::Arc(arc) = shape.segments()[1] else {
            panic!("arc")
        };
        let mid = list[4].1;
        let r = (mid[0] - arc.center[0]).hypot(mid[1] - arc.center[1]);
        assert!((r - arc.radius).abs() < 1e-9);

        let moved = drag(&start, Handle::Point(2), [20.0, 40.0], false).unwrap();
        assert!(matches!(&moved, Geometry::Polyline { points, .. } if points[2] == [20.0, 40.0]));
        // On top of its neighbour: refused.
        assert_eq!(drag(&start, Handle::Point(2), [40.0, 0.0], false), None);

        let inserted = drag(&start, Handle::Middle(0), [20.0, -5.0], false).unwrap();
        assert_eq!(
            inserted,
            Geometry::Polyline {
                closed: true,
                points: vec![[0.0, 0.0], [20.0, -5.0], [40.0, 0.0], [20.0, 30.0]],
                bulges: vec![0.0, 0.0, 0.4, 0.0],
            }
        );
        // Shift: bend the first segment 10 mm below its chord: bulge 10 / 20.
        let bent = drag(&start, Handle::Middle(0), [20.0, -10.0], true).unwrap();
        assert!(matches!(&bent, Geometry::Polyline { bulges, .. } if bulges[0] == 0.5));
        // Further than a half circle: held at one.
        let full = drag(&start, Handle::Middle(0), [20.0, -50.0], true).unwrap();
        assert!(matches!(&full, Geometry::Polyline { bulges, .. } if bulges[0] == 1.0));
        let other = drag(&start, Handle::Middle(0), [20.0, 50.0], true).unwrap();
        assert!(matches!(&other, Geometry::Polyline { bulges, .. } if bulges[0] == -1.0));
    }

    #[test]
    fn inserting_splits_an_arc_into_two_halves_of_it() {
        let mut g = triangle();
        let before = Shape {
            id: 1,
            layer: String::new(),
            name: None,
            geometry: g.clone(),
        }
        .segments();
        assert!(insert_point_after(&mut g, 1));
        let after = Shape {
            id: 1,
            layer: String::new(),
            name: None,
            geometry: g.clone(),
        }
        .segments();
        let (PathSegment::Arc(whole), PathSegment::Arc(h1), PathSegment::Arc(h2)) =
            (&before[1], &after[1], &after[2])
        else {
            panic!("arcs")
        };
        for half in [*h1, *h2] {
            assert!(close(half.center, whole.center));
            assert!((half.radius - whole.radius).abs() < 1e-9);
            assert!((half.sweep - whole.sweep / 2.0).abs() < 1e-9);
        }
        // After the last point of an open polyline: 10 mm on, straight.
        let mut open = Geometry::straight_polyline(false, vec![[0.0, 0.0], [5.0, 0.0]]);
        assert!(insert_point_after(&mut open, 1));
        assert_eq!(
            open,
            Geometry::straight_polyline(false, vec![[0.0, 0.0], [5.0, 0.0], [15.0, 0.0]])
        );
    }

    #[test]
    fn removing_and_closing_keep_one_bulge_per_segment() {
        let mut g = Geometry::Polyline {
            closed: true,
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            bulges: vec![0.1, 0.2, 0.3, 0.4],
        };
        // Point 2 joins segments 1 and 2 into one straight one.
        assert!(remove_point(&mut g, 2));
        assert!(matches!(&g, Geometry::Polyline { bulges, .. } if *bulges == [0.1, 0.0, 0.4]));
        // Point 0 of a closed one joins the last segment and the first.
        assert!(remove_point(&mut g, 0));
        assert!(matches!(&g, Geometry::Polyline { bulges, .. } if *bulges == [0.0, 0.0]));

        let mut open = Geometry::Polyline {
            closed: false,
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]],
            bulges: vec![0.1, 0.2],
        };
        set_closed(&mut open, true);
        assert!(matches!(&open, Geometry::Polyline { bulges, .. } if *bulges == [0.1, 0.2, 0.0]));
        set_closed(&mut open, false);
        assert!(remove_point(&mut open, 0));
        assert!(matches!(&open, Geometry::Polyline { bulges, .. } if *bulges == [0.2]));
        assert!(!remove_point(&mut open, 5));
    }

    #[test]
    fn transformed_moves_turns_and_scales_every_kind() {
        let circle = Geometry::Circle {
            center: [10.0, 0.0],
            radius: 2.0,
        };
        assert_eq!(
            transformed(&circle, [0.0, 0.0], [5.0, 5.0], 2.0, 90.0),
            Some(Geometry::Circle {
                center: [5.0, 25.0],
                radius: 4.0
            })
        );
        let rectangle = Geometry::Rectangle {
            center: [1.0, 1.0],
            length: 4.0,
            width: 2.0,
            angle: 0.0,
            corner_radius: 0.5,
        };
        let Some(Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        }) = transformed(&rectangle, [1.0, 1.0], [1.0, 1.0], 1.5, 30.0)
        else {
            panic!();
        };
        assert_eq!(
            (center, length, width, corner_radius),
            ([1.0, 1.0], 6.0, 3.0, 0.75)
        );
        assert!((angle - 30f64.to_radians()).abs() < 1e-12);
        let polyline = Geometry::Polyline {
            closed: false,
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]],
            bulges: vec![0.0, 0.4],
        };
        // A quarter turn about (10, 0) is exact; bulges stay.
        assert_eq!(
            transformed(&polyline, [10.0, 0.0], [10.0, 0.0], 1.0, 90.0),
            Some(Geometry::Polyline {
                closed: false,
                points: vec![[10.0, -10.0], [10.0, 0.0], [5.0, 0.0]],
                bulges: vec![0.0, 0.4],
            })
        );
        // A whole turn gives the shape back, within rounding.
        let Some(Geometry::Polyline { points, .. }) =
            transformed(&polyline, [3.0, 4.0], [3.0, 4.0], 1.0, 360.0)
        else {
            panic!();
        };
        assert_eq!(points, vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]]);
        let Some(Geometry::Polyline { points, .. }) =
            transformed(&polyline, [3.0, 4.0], [3.0, 4.0], 1.0, 33.0)
                .and_then(|g| transformed(&g, [3.0, 4.0], [3.0, 4.0], 1.0, -33.0))
        else {
            panic!();
        };
        for (p, q) in points.iter().zip([[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]]) {
            assert!((p[0] - q[0]).hypot(p[1] - q[1]) < 1e-12);
        }
        assert_eq!(transformed(&polyline, [0.0; 2], [0.0; 2], 0.0, 0.0), None);
        assert_eq!(transformed(&polyline, [0.0; 2], [0.0; 2], -1.0, 0.0), None);
    }

    #[test]
    fn a_rectangle_turned_keeps_a_small_angle() {
        let rectangle = Geometry::Rectangle {
            center: [0.0, 0.0],
            length: 4.0,
            width: 2.0,
            angle: 3.0,
            corner_radius: 0.0,
        };
        let Some(Geometry::Rectangle { angle, .. }) =
            transformed(&rectangle, [0.0; 2], [0.0; 2], 1.0, 90.0)
        else {
            panic!();
        };
        assert!((angle - (3.0 + std::f64::consts::FRAC_PI_2 - TAU)).abs() < 1e-12);
    }
}
