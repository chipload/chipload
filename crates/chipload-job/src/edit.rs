// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing a drawn shape's defining values: the handles the GUI shows, what dragging one
//! does, and inserting or removing polyline points (docs/specs/drawing-follow-ups.md
//! sections 1 and 3). Plain values in, plain values out, so every rule is tested here.

use chipload_core::primitives::Arc;

use crate::models::{Geometry, polyline_segment_count, shape_problems};

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
    match (&mut geometry, handle) {
        (Geometry::Circle { center, .. }, Handle::Center)
        | (Geometry::Rectangle { center, .. }, Handle::Center) => *center = to,
        (Geometry::Circle { center, radius }, Handle::Radius) => {
            *radius = (to[0] - center[0]).hypot(to[1] - center[1]);
        }
        (
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
                ..
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
        Err(problems.join("; "))
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
    }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PathSegment, Shape};

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
            geometry: g.clone(),
        }
        .segments();
        assert!(insert_point_after(&mut g, 1));
        let after = Shape {
            id: 1,
            layer: String::new(),
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
}
