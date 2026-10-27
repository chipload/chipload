// SPDX-License-Identifier: GPL-3.0-or-later
//! Selecting with a box (docs/specs/drawing-input.md §3): a contour is caught when it lies
//! wholly inside the box, or, dragged from right to left, when it touches it at all.

use chipload_core::Point;
use chipload_core::primitives::{Line, Primitive};

use crate::intersect;

/// How a box catches contours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxMode {
    /// Dragged left to right: only what lies wholly inside.
    Inside,
    /// Dragged right to left: also what crosses or touches it.
    Touching,
}

impl BoxMode {
    /// The mode of a box dragged from `from` to `to`.
    pub fn of_drag(from: Point, to: Point) -> Self {
        if to.0 >= from.0 {
            BoxMode::Inside
        } else {
            BoxMode::Touching
        }
    }
}

/// Whether the box with corners `a` and `b` (any two opposite corners) catches the contour
/// made of `segments`.
pub fn caught(segments: &[Primitive], a: Point, b: Point, mode: BoxMode) -> bool {
    let lo = (a.0.min(b.0), a.1.min(b.1));
    let hi = (a.0.max(b.0), a.1.max(b.1));
    if segments.is_empty() {
        return false;
    }
    match mode {
        BoxMode::Inside => segments.iter().all(|s| {
            let (s_lo, s_hi) = s.bounds();
            inside(s_lo, lo, hi) && inside(s_hi, lo, hi)
        }),
        BoxMode::Touching => segments.iter().any(|s| touches(s, lo, hi)),
    }
}

fn inside(p: Point, lo: Point, hi: Point) -> bool {
    p.0 >= lo.0 && p.0 <= hi.0 && p.1 >= lo.1 && p.1 <= hi.1
}

/// Whether a segment has a point in the box: an end inside, or a crossing with a side.
fn touches(segment: &Primitive, lo: Point, hi: Point) -> bool {
    let (s_lo, s_hi) = segment.bounds();
    if s_hi.0 < lo.0 || s_lo.0 > hi.0 || s_hi.1 < lo.1 || s_lo.1 > hi.1 {
        return false;
    }
    if inside(segment.start(), lo, hi) || inside(segment.end(), lo, hi) {
        return true;
    }
    let corners = [lo, (hi.0, lo.1), hi, (lo.0, hi.1)];
    (0..4).any(|k| {
        let side = Primitive::Line(Line {
            start: corners[k],
            end: corners[(k + 1) % 4],
        });
        !intersect::intersections(segment, &side).is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_core::primitives::Arc;
    use std::f64::consts::{PI, TAU};

    fn line(a: Point, b: Point) -> Primitive {
        Primitive::Line(Line { start: a, end: b })
    }

    #[test]
    fn inside_takes_only_what_lies_wholly_in_the_box() {
        let square = [
            line((1.0, 1.0), (3.0, 1.0)),
            line((3.0, 1.0), (3.0, 3.0)),
            line((3.0, 3.0), (1.0, 1.0)),
        ];
        assert!(caught(&square, (0.0, 0.0), (4.0, 4.0), BoxMode::Inside));
        // Corners in any order.
        assert!(caught(&square, (4.0, 0.0), (0.0, 4.0), BoxMode::Inside));
        assert!(!caught(&square, (0.0, 0.0), (2.0, 4.0), BoxMode::Inside));
        assert!(caught(&square, (0.0, 0.0), (2.0, 4.0), BoxMode::Touching));
        assert!(!caught(&square, (5.0, 5.0), (6.0, 6.0), BoxMode::Touching));
    }

    #[test]
    fn arcs_count_by_their_curve_not_their_ends() {
        // A half circle over (0, 0)..(10, 0) bulging up to y = 5.
        let arc = [Primitive::Arc(Arc {
            center: (5.0, 0.0),
            radius: 5.0,
            start_angle: 0.0,
            sweep: PI,
        })];
        // A box under its top: the ends are outside, the curve crosses it.
        assert!(caught(&arc, (4.0, 4.0), (6.0, 6.0), BoxMode::Touching));
        // A box that holds the ends but not the top: not wholly inside.
        assert!(!caught(&arc, (-1.0, -1.0), (11.0, 4.0), BoxMode::Inside));
        // A circle round the box doesn't touch it.
        let circle = [Primitive::Arc(Arc {
            center: (0.0, 0.0),
            radius: 10.0,
            start_angle: 0.0,
            sweep: TAU,
        })];
        assert!(!caught(
            &circle,
            (-1.0, -1.0),
            (1.0, 1.0),
            BoxMode::Touching
        ));
        assert!(!caught(&circle, (-1.0, -1.0), (1.0, 1.0), BoxMode::Inside));
        assert!(caught(&circle, (8.0, -1.0), (12.0, 1.0), BoxMode::Touching));
    }

    #[test]
    fn the_drag_direction_gives_the_mode() {
        assert_eq!(BoxMode::of_drag((0.0, 0.0), (5.0, -5.0)), BoxMode::Inside);
        assert_eq!(BoxMode::of_drag((5.0, 0.0), (0.0, 5.0)), BoxMode::Touching);
    }
}
