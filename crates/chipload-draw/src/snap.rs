// SPDX-License-Identifier: GPL-3.0-or-later
//! The points of a drawing the cursor snaps to (docs/specs/drawing-input.md §1, after
//! drawing-follow-ups.md §2). Candidates are found for the segments near the cursor only, so
//! a drawing of many thousands of segments costs what lies under the cursor.

use std::collections::BTreeSet;

use chipload_core::Point;
use chipload_core::order::Ordered;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::intersect;

/// What a snap point is; the order is the tie-break order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapKind {
    End,
    Intersection,
    Centre,
    Quadrant,
    Middle,
    Perpendicular,
    Tangent,
    /// The point of a segment nearest the cursor: taken only when nothing else is in reach.
    Nearest,
}

/// A point of the drawing the cursor can snap to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapPoint {
    pub at: Point,
    pub kind: SnapKind,
    /// The contour it belongs to.
    pub contour: usize,
    /// An intersection's other contour (may be the same one).
    pub other: Option<usize>,
}

/// One segment the cursor can snap to, with its box.
#[derive(Debug, Clone)]
struct Item {
    prim: Primitive,
    contour: usize,
    lo: Point,
    hi: Point,
}

/// The segments of a drawing, ready to snap to.
#[derive(Debug, Clone, Default)]
pub struct Snapper {
    items: Vec<Item>,
}

impl Snapper {
    /// From each segment and the contour it belongs to, in a fixed order.
    pub fn new(segments: impl IntoIterator<Item = (usize, Primitive)>) -> Self {
        let items = segments
            .into_iter()
            .map(|(contour, prim)| {
                let (lo, hi) = prim.bounds();
                Item {
                    prim,
                    contour,
                    lo,
                    hi,
                }
            })
            .collect();
        Snapper { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Where the cursor at `at` snaps: of the candidates within `reach` (mm), the nearest;
    /// ties by kind, then the lowest contour ids. Nearest-on-segment only when nothing
    /// else is in reach. `from` is the last point of the tool being used (for perpendicular
    /// and tangent points); segments of the contours in `skip` are left out.
    pub fn find(
        &self,
        at: Point,
        reach: f64,
        from: Option<Point>,
        skip: &BTreeSet<usize>,
    ) -> Option<SnapPoint> {
        let candidates = self.candidates(at, reach, from, skip);
        let key = |p: &SnapPoint| {
            (
                Ordered(distance(p.at, at)),
                p.kind,
                p.contour,
                p.other.unwrap_or(0),
            )
        };
        let in_reach = candidates.iter().filter(|p| distance(p.at, at) <= reach);
        let (nearest, others): (Vec<&SnapPoint>, Vec<&SnapPoint>) =
            in_reach.partition(|p| p.kind == SnapKind::Nearest);
        others
            .into_iter()
            .min_by_key(|p| key(p))
            .or_else(|| nearest.into_iter().min_by_key(|p| key(p)))
            .copied()
    }

    /// Every candidate of the segments whose box, grown by `reach`, holds `at`.
    pub fn candidates(
        &self,
        at: Point,
        reach: f64,
        from: Option<Point>,
        skip: &BTreeSet<usize>,
    ) -> Vec<SnapPoint> {
        let near: Vec<&Item> = self
            .items
            .iter()
            .filter(|item| !skip.contains(&item.contour))
            .filter(|item| {
                at.0 >= item.lo.0 - reach
                    && at.0 <= item.hi.0 + reach
                    && at.1 >= item.lo.1 - reach
                    && at.1 <= item.hi.1 + reach
            })
            .collect();
        let mut list = Vec::new();
        for item in &near {
            own_points(item, at, from, &mut list);
        }
        for (i, a) in near.iter().enumerate() {
            for b in &near[i + 1..] {
                for p in intersect::intersections(&a.prim, &b.prim) {
                    let (first, second) = if a.contour <= b.contour {
                        (a.contour, b.contour)
                    } else {
                        (b.contour, a.contour)
                    };
                    list.push(SnapPoint {
                        at: p,
                        kind: SnapKind::Intersection,
                        contour: first,
                        other: Some(second),
                    });
                }
            }
        }
        // Where a segment's end is also a crossing, the end point is enough.
        let ends: Vec<Point> = list
            .iter()
            .filter(|p| p.kind == SnapKind::End)
            .map(|p| p.at)
            .collect();
        list.retain(|p| {
            p.kind != SnapKind::Intersection
                || ends.iter().all(|&e| distance(e, p.at) > CHAIN_TOL_MM)
        });
        list
    }
}

/// The candidates of one segment on its own.
fn own_points(item: &Item, at: Point, from: Option<Point>, list: &mut Vec<SnapPoint>) {
    let prim = &item.prim;
    let mut add = |at: Point, kind: SnapKind| {
        list.push(SnapPoint {
            at,
            kind,
            contour: item.contour,
            other: None,
        });
    };
    add(prim.start(), SnapKind::End);
    add(prim.end(), SnapKind::End);
    add(prim.midpoint(), SnapKind::Middle);
    add(intersect::nearest_on(prim, at), SnapKind::Nearest);
    if let Primitive::Arc(arc) = prim {
        add(arc.center, SnapKind::Centre);
        for q in intersect::quadrants(arc) {
            add(q, SnapKind::Quadrant);
        }
    }
    if let Some(from) = from {
        for foot in intersect::perpendicular_feet(prim, from) {
            // From a point on the segment itself, the foot is that point: not worth naming.
            if distance(foot, from) > CHAIN_TOL_MM {
                add(foot, SnapKind::Perpendicular);
            }
        }
        if let Primitive::Arc(arc) = prim {
            for t in intersect::tangent_points(arc, from) {
                add(t, SnapKind::Tangent);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_core::primitives::{Arc, Line};
    use std::f64::consts::{PI, TAU};

    fn line(a: Point, b: Point) -> Primitive {
        Primitive::Line(Line { start: a, end: b })
    }

    fn circle(center: Point, radius: f64) -> Primitive {
        Primitive::Arc(Arc {
            center,
            radius,
            start_angle: 0.0,
            sweep: TAU,
        })
    }

    fn none() -> BTreeSet<usize> {
        BTreeSet::new()
    }

    #[test]
    fn the_nearest_candidate_wins_then_the_kind_then_the_contour() {
        let snapper = Snapper::new([
            (1, line((0.0, 0.0), (10.0, 0.0))),
            (
                2,
                Primitive::Arc(Arc {
                    center: (5.0, 0.0),
                    radius: 5.0,
                    start_angle: 0.0,
                    sweep: PI,
                }),
            ),
        ]);
        let find = |p: Point| snapper.find(p, 0.8, None, &none()).unwrap();
        // Both end at (10, 0): the end of #1 (lowest contour).
        let hit = find((9.5, 0.3));
        assert_eq!(
            (hit.at, hit.kind, hit.contour),
            ((10.0, 0.0), SnapKind::End, 1)
        );
        // The line's middle and the arc's centre are both (5, 0): the centre wins.
        assert_eq!(find((5.2, 0.1)).kind, SnapKind::Centre);
        // The arc's top is its middle and its 90° quadrant: the quadrant wins.
        assert_eq!(find((5.0, 4.6)).kind, SnapKind::Quadrant);
        // Nothing but the line near: the nearest point on it.
        let on = find((2.6, 0.5));
        assert_eq!((on.kind, on.at), (SnapKind::Nearest, (2.6, 0.0)));
        // Far from everything: nothing.
        assert!(snapper.find((2.6, 2.4), 0.8, None, &none()).is_none());
        // Skipped contours aren't snapped to.
        let skip: BTreeSet<usize> = [1].into();
        assert_eq!(
            snapper.find((9.5, 0.3), 0.8, None, &skip).unwrap().contour,
            2
        );
    }

    #[test]
    fn crossings_snap_but_not_where_an_end_already_is() {
        let snapper = Snapper::new([
            (3, line((0.0, 0.0), (10.0, 10.0))),
            (4, line((0.0, 10.0), (10.0, 0.0))),
            // A line ending on #3: its end, not an intersection.
            (5, line((5.0, 5.0), (5.0, 9.0))),
        ]);
        let hit = snapper.find((5.3, 4.8), 1.0, None, &none()).unwrap();
        assert_eq!(hit.kind, SnapKind::End);
        let crossings: Vec<SnapPoint> = snapper
            .candidates((5.0, 5.0), 1.0, None, &none())
            .into_iter()
            .filter(|p| p.kind == SnapKind::Intersection)
            .collect();
        assert!(crossings.is_empty());
        let plain = Snapper::new([
            (3, line((0.0, 0.0), (10.0, 10.0))),
            (4, line((0.0, 10.0), (10.0, 0.0))),
        ]);
        let hit = plain.find((5.3, 4.8), 1.0, None, &none()).unwrap();
        assert_eq!(
            (hit.kind, hit.contour, hit.other),
            (SnapKind::Intersection, 3, Some(4))
        );
    }

    #[test]
    fn perpendicular_and_tangent_need_a_last_point() {
        let snapper = Snapper::new([
            (1, line((0.0, 0.0), (10.0, 0.0))),
            (2, circle((20.0, 0.0), 5.0)),
        ]);
        let from = Some((4.0, 7.0));
        // Near (4, 0): the perpendicular foot from (4, 7) beats the nearest point.
        let hit = snapper.find((4.3, 0.2), 0.5, from, &none()).unwrap();
        assert_eq!((hit.kind, hit.at), (SnapKind::Perpendicular, (4.0, 0.0)));
        assert_eq!(
            snapper.find((4.3, 0.2), 0.5, None, &none()).unwrap().kind,
            SnapKind::Nearest
        );
        // Tangent from (10, 0) to the circle at (20, 0) r 5: at 60° from the axis.
        let t = (20.0 - 2.5, 5.0 * (PI / 3.0).sin());
        let hit = snapper.find((t.0 + 0.2, t.1 - 0.1), 0.5, Some((10.0, 0.0)), &none());
        assert_eq!(hit.unwrap().kind, SnapKind::Tangent);
    }

    #[test]
    fn only_segments_near_the_cursor_are_looked_at() {
        // A thousand short lines far away and one near: the candidates are the near one's.
        let mut segments: Vec<(usize, Primitive)> = (0..1000)
            .map(|i| (i, line((100.0 + i as f64, 0.0), (100.5 + i as f64, 0.0))))
            .collect();
        segments.push((5000, line((0.0, 0.0), (1.0, 0.0))));
        let snapper = Snapper::new(segments);
        let candidates = snapper.candidates((0.5, 0.2), 0.5, None, &none());
        assert!(candidates.iter().all(|p| p.contour == 5000));
    }
}
