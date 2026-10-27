// SPDX-License-Identifier: GPL-3.0-or-later
//! Offsetting a drawn shape (docs/specs/drawing-editing.md §2): the curve at a distance
//! from it, with true arcs (`cavalier_contours`, ADR 0006), round corners or sharp ones,
//! checked before it is returned.

use std::f64::consts::PI;
use std::panic::{AssertUnwindSafe, catch_unwind};

use cavalier_contours::polyline::{
    PlineOrientation, PlineSource, PlineSourceMut, PlineVertex, Polyline,
};
use chipload_core::Point;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::intersect;

/// A drawn shape's outline: its segments in order, and whether the last ends where the
/// first starts.
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    pub prims: Vec<Primitive>,
    pub closed: bool,
}

impl Chain {
    /// As polyline points and bulges: each segment's start (and an open chain's end), and a
    /// bulge per segment; arcs past a half circle in two halves, as a bulge describes at
    /// most one.
    pub fn as_polyline(&self) -> (Vec<Point>, Vec<f64>) {
        let mut points = Vec::new();
        let mut bulges = Vec::new();
        for prim in &self.prims {
            match prim {
                Primitive::Line(line) => {
                    points.push(line.start);
                    bulges.push(0.0);
                }
                Primitive::Arc(arc) => {
                    let pieces: u32 = if arc.sweep.abs() > PI { 2 } else { 1 };
                    let sweep = arc.sweep / f64::from(pieces);
                    for i in 0..pieces {
                        let start = arc.point_at(arc.start_angle + sweep * f64::from(i));
                        points.push(start);
                        bulges.push((sweep / 4.0).tan());
                    }
                }
            }
        }
        if !self.closed
            && let Some(last) = self.prims.last()
        {
            points.push(last.end());
        }
        (points, bulges)
    }
}

/// Which way an offset goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// A closed shape's outside (bigger).
    Outside,
    /// A closed shape's inside (smaller).
    Inside,
    /// An open shape's left, going from its start.
    Left,
    /// An open shape's right.
    Right,
}

/// How an offset turns round a convex corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corners {
    /// An arc about the corner: the curve at exactly the distance everywhere.
    Round,
    /// The two straight sides extended until they meet, as a frame round a part; where an
    /// arc meets the corner it stays round.
    Sharp,
}

/// Why an offset can't be made.
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetError {
    /// Nothing is left at that distance (inward further than the shape is wide).
    Vanishes,
    /// The side doesn't fit the shape: inside or outside of an open one, left or right of
    /// a closed one.
    WrongSide,
    /// The distance isn't a positive number.
    BadDistance,
    /// The offset library failed (a panic or a result that isn't usable).
    Library,
    /// The result is not at the distance, or crosses the shape: never returned.
    Unverified(String),
}

/// The curves `distance` (mm, > 0) from `chain` on `side`, with `corners`. A shape can give
/// several (inward across a narrow waist), each checked: at the distance from the shape
/// (sharp corners: at least), on its side, never crossing it.
pub fn offset(
    chain: &Chain,
    distance: f64,
    side: Side,
    corners: Corners,
) -> Result<Vec<Chain>, OffsetError> {
    if !(distance.is_finite() && distance > 0.0) {
        return Err(OffsetError::BadDistance);
    }
    let fits = match side {
        Side::Outside | Side::Inside => chain.closed,
        Side::Left | Side::Right => !chain.closed,
    };
    if !fits || chain.prims.is_empty() {
        return Err(OffsetError::WrongSide);
    }
    let mut pline = to_pline(chain);
    // Positive offsets go left of the direction of travel: inward on a counter-clockwise
    // loop.
    let signed = match side {
        Side::Inside | Side::Left => distance,
        Side::Outside | Side::Right => -distance,
    };
    if chain.closed && pline.orientation() == PlineOrientation::Clockwise {
        pline.invert_direction_mut();
    }
    let raw = catch_unwind(AssertUnwindSafe(|| pline.parallel_offset(signed)))
        .map_err(|_| OffsetError::Library)?;
    let mut results: Vec<Chain> = raw
        .iter()
        .map(from_pline)
        .filter(|c| c.closed == chain.closed && !c.prims.is_empty())
        .collect();
    if results.is_empty() {
        return Err(OffsetError::Vanishes);
    }
    if corners == Corners::Sharp {
        let corners = vertices(chain);
        for result in &mut results {
            sharpen(result, &corners, distance);
        }
    }
    for result in &results {
        verify(chain, result, distance, side, corners)?;
    }
    Ok(results)
}

/// A chain as a cavalier polyline: a vertex at each segment's start with its bulge; arcs
/// past a half circle in two halves, as a bulge describes at most one.
pub fn to_pline(chain: &Chain) -> Polyline<f64> {
    let mut pline = if chain.closed {
        Polyline::new_closed()
    } else {
        Polyline::new()
    };
    for prim in &chain.prims {
        match prim {
            Primitive::Line(line) => pline.add(line.start.0, line.start.1, 0.0),
            Primitive::Arc(arc) => {
                let pieces: u32 = if arc.sweep.abs() > PI { 2 } else { 1 };
                let sweep = arc.sweep / f64::from(pieces);
                for i in 0..pieces {
                    let piece = Arc {
                        start_angle: arc.start_angle + sweep * f64::from(i),
                        sweep,
                        ..*arc
                    };
                    let start = piece.point_at(piece.start_angle);
                    pline.add(start.0, start.1, piece.bulge());
                }
            }
        }
    }
    if !chain.closed
        && let Some(last) = chain.prims.last()
    {
        let end = last.end();
        pline.add(end.0, end.1, 0.0);
    }
    pline
}

/// A cavalier polyline as a chain of lines and arcs.
pub fn from_pline(pline: &Polyline<f64>) -> Chain {
    let n = pline.vertex_count();
    let closed = pline.is_closed();
    let count = if closed { n } else { n.saturating_sub(1) };
    let prims = (0..count)
        .filter_map(|i| {
            let v: PlineVertex<f64> = pline.at(i);
            let next = pline.at((i + 1) % n);
            let (a, b) = ((v.x, v.y), (next.x, next.y));
            if distance(a, b) < CHAIN_TOL_MM {
                return None;
            }
            Some(match Arc::from_bulge(a, b, v.bulge) {
                Some(arc) => Primitive::Arc(arc),
                None => Primitive::Line(Line { start: a, end: b }),
            })
        })
        .collect();
    Chain { prims, closed }
}

/// The shape's corners: every point where two of its segments meet.
fn vertices(chain: &Chain) -> Vec<Point> {
    chain.prims.iter().map(Primitive::start).collect()
}

/// Round joins replaced by sharp corners: an arc of radius `distance` about one of the
/// shape's corners, between two lines, becomes the point where those lines meet.
fn sharpen(chain: &mut Chain, corners: &[Point], distance: f64) {
    let mut k = 0;
    while k < chain.prims.len() {
        let n = chain.prims.len();
        let joins = |arc: &Arc| {
            (arc.radius - distance).abs() < CHAIN_TOL_MM
                && corners.iter().any(|&c| same_point(c, arc.center))
        };
        let (prev, next) = if chain.closed {
            ((k + n - 1) % n, (k + 1) % n)
        } else if k == 0 || k + 1 == n {
            k += 1;
            continue;
        } else {
            (k - 1, k + 1)
        };
        let meet = match (chain.prims[prev], chain.prims[k], chain.prims[next]) {
            (Primitive::Line(a), Primitive::Arc(arc), Primitive::Line(b)) if joins(&arc) => {
                line_meet(&a, &b)
            }
            _ => None,
        };
        match meet {
            Some(p) if n >= 3 => {
                if let Primitive::Line(a) = &mut chain.prims[prev] {
                    a.end = p;
                }
                if let Primitive::Line(b) = &mut chain.prims[next] {
                    b.start = p;
                }
                chain.prims.remove(k);
            }
            _ => k += 1,
        }
    }
}

fn same_point(a: Point, b: Point) -> bool {
    distance(a, b) < CHAIN_TOL_MM
}

/// Where the lines through `a` and `b` meet; None when they are parallel.
fn line_meet(a: &Line, b: &Line) -> Option<Point> {
    let r = (a.end.0 - a.start.0, a.end.1 - a.start.1);
    let s = (b.end.0 - b.start.0, b.end.1 - b.start.1);
    let cross = r.0 * s.1 - r.1 * s.0;
    let lengths = r.0.hypot(r.1) * s.0.hypot(s.1);
    if lengths <= 0.0 || (cross / lengths).abs() < CHAIN_TOL_MM {
        return None;
    }
    let w = (b.start.0 - a.start.0, b.start.1 - a.start.1);
    let t = (w.0 * s.1 - w.1 * s.0) / cross;
    Some((a.start.0 + r.0 * t, a.start.1 + r.1 * t))
}

/// The result is at the distance from the shape (sharp corners: no nearer), on its side,
/// and never crosses it.
fn verify(
    source: &Chain,
    result: &Chain,
    distance: f64,
    side: Side,
    corners: Corners,
) -> Result<(), OffsetError> {
    let unverified = |what: String| Err(OffsetError::Unverified(what));
    for prim in &result.prims {
        for p in prim.points() {
            let near = nearest_distance(source, p);
            if near < distance - CHAIN_TOL_MM {
                return unverified(format!("a point {near:.4} mm from the shape"));
            }
            if corners == Corners::Round && near > distance + CHAIN_TOL_MM {
                return unverified(format!("a point {near:.4} mm from the shape"));
            }
        }
        for s in &source.prims {
            if !intersect::intersections(prim, s).is_empty() {
                return unverified("it crosses the shape".into());
            }
        }
    }
    if source.closed
        && let Some(first) = result.prims.first()
    {
        let ring: Vec<Point> = source.prims.iter().flat_map(Primitive::points).collect();
        let inside = winds_round(&ring, first.midpoint());
        if inside != (side == Side::Inside) {
            return unverified("it is on the wrong side".into());
        }
    }
    Ok(())
}

/// The side of the chain `p` is on: inside or outside a closed one; left or right of an
/// open one, as seen going along its nearest segment.
pub fn side_at(chain: &Chain, p: Point) -> Side {
    if chain.closed {
        let ring: Vec<Point> = chain.prims.iter().flat_map(Primitive::points).collect();
        return if winds_round(&ring, p) {
            Side::Inside
        } else {
            Side::Outside
        };
    }
    let nearest = chain
        .prims
        .iter()
        .map(|s| (distance(intersect::nearest_on(s, p), p), s))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let Some((_, prim)) = nearest else {
        return Side::Left;
    };
    let q = intersect::nearest_on(prim, p);
    // The direction of travel at q.
    let (dx, dy) = match prim {
        Primitive::Line(l) => (l.end.0 - l.start.0, l.end.1 - l.start.1),
        Primitive::Arc(a) => {
            let (rx, ry) = (q.0 - a.center.0, q.1 - a.center.1);
            if a.sweep >= 0.0 { (-ry, rx) } else { (ry, -rx) }
        }
    };
    if dx * (p.1 - q.1) - dy * (p.0 - q.0) >= 0.0 {
        Side::Left
    } else {
        Side::Right
    }
}

/// How far `p` is from the nearest point of the chain.
pub fn nearest_distance(chain: &Chain, p: Point) -> f64 {
    chain
        .prims
        .iter()
        .map(|s| distance(intersect::nearest_on(s, p), p))
        .fold(f64::INFINITY, f64::min)
}

/// Whether `p` lies inside the closed ring of points (crossings of a ray to its right).
fn winds_round(ring: &[Point], p: Point) -> bool {
    let n = ring.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) {
            let x = a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if x > p.0 {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    fn square(size: f64) -> Chain {
        let c = [(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)];
        Chain {
            prims: (0..4)
                .map(|i| {
                    Primitive::Line(Line {
                        start: c[i],
                        end: c[(i + 1) % 4],
                    })
                })
                .collect(),
            closed: true,
        }
    }

    fn arcs(chain: &Chain) -> usize {
        chain
            .prims
            .iter()
            .filter(|p| matches!(p, Primitive::Arc(_)))
            .count()
    }

    #[test]
    fn a_square_offset_out_round_or_sharp_and_in() {
        let out = offset(&square(10.0), 2.0, Side::Outside, Corners::Round).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].prims.len(), arcs(&out[0])), (8, 4));
        let sharp = offset(&square(10.0), 2.0, Side::Outside, Corners::Sharp).unwrap();
        assert_eq!((sharp[0].prims.len(), arcs(&sharp[0])), (4, 0));
        let corners: Vec<Point> = sharp[0].prims.iter().map(Primitive::start).collect();
        for c in [(-2.0, -2.0), (12.0, -2.0), (12.0, 12.0), (-2.0, 12.0)] {
            assert!(corners.iter().any(|&p| distance(p, c) < 1e-9), "{c:?}");
        }
        let inward = offset(&square(10.0), 2.0, Side::Inside, Corners::Round).unwrap();
        assert_eq!((inward[0].prims.len(), arcs(&inward[0])), (4, 0));
        assert_eq!(
            offset(&square(10.0), 6.0, Side::Inside, Corners::Round),
            Err(OffsetError::Vanishes)
        );
    }

    #[test]
    fn the_side_must_fit_the_shape_and_the_distance_be_positive() {
        assert_eq!(
            offset(&square(10.0), 1.0, Side::Left, Corners::Round),
            Err(OffsetError::WrongSide)
        );
        assert_eq!(
            offset(&square(10.0), 0.0, Side::Outside, Corners::Round),
            Err(OffsetError::BadDistance)
        );
        let open = Chain {
            prims: vec![Primitive::Line(Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            })],
            closed: false,
        };
        assert_eq!(
            offset(&open, 1.0, Side::Outside, Corners::Round),
            Err(OffsetError::WrongSide)
        );
        let left = offset(&open, 1.0, Side::Left, Corners::Round).unwrap();
        assert!(!left[0].closed);
        assert!(distance(left[0].prims[0].start(), (0.0, 1.0)) < 1e-9);
        let right = offset(&open, 1.0, Side::Right, Corners::Round).unwrap();
        assert!(distance(right[0].prims[0].start(), (0.0, -1.0)) < 1e-9);
    }

    #[test]
    fn a_circle_offsets_to_a_circle_and_clockwise_shapes_too() {
        let circle = Chain {
            prims: vec![Primitive::Arc(Arc {
                center: (0.0, 0.0),
                radius: 5.0,
                start_angle: 0.0,
                sweep: TAU,
            })],
            closed: true,
        };
        let out = offset(&circle, 1.0, Side::Outside, Corners::Sharp).unwrap();
        for prim in &out[0].prims {
            let Primitive::Arc(arc) = prim else { panic!() };
            assert!((arc.radius - 6.0).abs() < 1e-9);
        }
        // The same square drawn clockwise offsets the same way.
        let mut clockwise = square(10.0);
        clockwise.prims.reverse();
        for p in &mut clockwise.prims {
            if let Primitive::Line(l) = p {
                std::mem::swap(&mut l.start, &mut l.end);
            }
        }
        let out = offset(&clockwise, 2.0, Side::Outside, Corners::Sharp).unwrap();
        let x_low = out[0]
            .prims
            .iter()
            .map(|p| p.start().0)
            .fold(f64::INFINITY, f64::min);
        assert!((x_low + 2.0).abs() < 1e-9);
    }

    #[test]
    fn the_cursor_side_is_inside_outside_left_or_right() {
        assert_eq!(side_at(&square(10.0), (5.0, 5.0)), Side::Inside);
        assert_eq!(side_at(&square(10.0), (15.0, 5.0)), Side::Outside);
        let open = Chain {
            prims: vec![Primitive::Line(Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            })],
            closed: false,
        };
        assert_eq!(side_at(&open, (5.0, 1.0)), Side::Left);
        assert_eq!(side_at(&open, (5.0, -1.0)), Side::Right);
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// A random convex polygon offset either way lies at the distance and never
            /// crosses it (offset() checks that itself; here it must not refuse).
            #[test]
            fn polygons_offset_cleanly(
                n in 3usize..9,
                radius in 5.0..50.0f64,
                d in 0.1..4.0f64,
                outward in any::<bool>(),
                sharp in any::<bool>(),
            ) {
                let c: Vec<Point> = (0..n)
                    .map(|k| {
                        let t = k as f64 * TAU / n as f64;
                        (radius * t.cos(), radius * t.sin())
                    })
                    .collect();
                let chain = Chain {
                    prims: (0..n)
                        .map(|i| Primitive::Line(Line { start: c[i], end: c[(i + 1) % n] }))
                        .collect(),
                    closed: true,
                };
                let side = if outward { Side::Outside } else { Side::Inside };
                let corners = if sharp { Corners::Sharp } else { Corners::Round };
                // Inward, a polygon has room for d when d is under its inradius.
                let inradius = radius * (PI / n as f64).cos();
                prop_assume!(outward || d < inradius - 0.1);
                let result = offset(&chain, d, side, corners);
                prop_assert!(result.is_ok(), "{result:?}");
            }
        }
    }
}
