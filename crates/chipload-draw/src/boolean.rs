// SPDX-License-Identifier: GPL-3.0-or-later
//! Union, subtraction and intersection of closed drawn shapes
//! (docs/specs/drawing-editing.md §5), with true arcs (`cavalier_contours`' boolean of two
//! polylines, ADR 0006). Several shapes are combined one at a time into a region: outlines
//! and the holes inside them. Every result is checked before it is returned.

use std::panic::{AssertUnwindSafe, catch_unwind};

use cavalier_contours::polyline::{
    BooleanOp, BooleanResult, BooleanResultInfo, BooleanResultPline, PlineOrientation, PlineSource,
    PlineSourceMut, Polyline,
};
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::offset::{Chain, from_pline, nearest_distance, to_pline};

/// Which combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combine {
    /// Everything any of the shapes covers.
    Union,
    /// The first shape with the others cut out of it.
    Subtract,
    /// What all the shapes cover.
    Intersect,
}

/// Why shapes can't be combined.
#[derive(Debug, Clone, PartialEq)]
pub enum BooleanError {
    /// Fewer than two shapes.
    TooFew,
    /// Input `index` is open: only closed shapes have an inside.
    Open(usize),
    /// Nothing is left: a subtraction that removes everything, an intersection of shapes
    /// that don't overlap.
    Empty,
    /// The boolean library failed (a panic or input it refused).
    Library,
    /// The result is not what the inputs give (crosses itself, wrong area): never returned.
    Unverified(String),
}

/// A closed result: an outline, or a hole inside one; `from` is the input it is, unchanged
/// (nothing touched it), so it can stay as it was drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub chain: Chain,
    pub hole: bool,
    pub from: Option<usize>,
}

/// Combine closed `inputs` (for a subtraction the first is the one cut). The result: the
/// outlines, largest first, then the holes, largest first.
pub fn combine(op: Combine, inputs: &[Chain]) -> Result<Vec<Piece>, BooleanError> {
    if inputs.len() < 2 {
        return Err(BooleanError::TooFew);
    }
    if let Some(open) = inputs.iter().position(|c| !c.closed || c.prims.is_empty()) {
        return Err(BooleanError::Open(open));
    }
    let plines: Vec<Polyline<f64>> = inputs.iter().map(counter_clockwise).collect();
    let region = match op {
        Combine::Union => union_of(&plines)?,
        Combine::Subtract => subtract(&plines[0], &plines[1..])?,
        Combine::Intersect => intersect(&plines)?,
    };
    if region.outers.is_empty() {
        return Err(BooleanError::Empty);
    }
    verify(op, &plines, &region)?;
    Ok(pieces(region, inputs))
}

/// Outlines and the holes inside them, every one counter-clockwise.
#[derive(Debug, Default)]
struct Region {
    outers: Vec<Polyline<f64>>,
    holes: Vec<Polyline<f64>>,
}

impl Region {
    fn area(&self) -> f64 {
        let sum = |ps: &[Polyline<f64>]| ps.iter().map(|p| p.area().abs()).sum::<f64>();
        sum(&self.outers) - sum(&self.holes)
    }
}

fn counter_clockwise(chain: &Chain) -> Polyline<f64> {
    let mut pline = to_pline(chain);
    ccw(&mut pline);
    pline
}

fn ccw(pline: &mut Polyline<f64>) {
    if pline.orientation() == PlineOrientation::Clockwise {
        pline.invert_direction_mut();
    }
}

/// `a op b`, a panic inside the library an error.
fn boolean(
    a: &Polyline<f64>,
    b: &Polyline<f64>,
    op: BooleanOp,
) -> Result<BooleanResult<Polyline<f64>>, BooleanError> {
    let result =
        catch_unwind(AssertUnwindSafe(|| a.boolean(b, op))).map_err(|_| BooleanError::Library)?;
    if matches!(result.result_info, BooleanResultInfo::InvalidInput) {
        return Err(BooleanError::Library);
    }
    Ok(result)
}

/// The library's result outlines, counter-clockwise, with points along a straight side or
/// one arc merged; outlines with no area (the library returns a shared side of two touching
/// shapes as one) left out.
fn plines_of(result: Vec<BooleanResultPline<Polyline<f64>>>) -> Vec<Polyline<f64>> {
    result
        .into_iter()
        .filter_map(|r| {
            let mut pline = r.pline.remove_redundant(CHAIN_TOL_MM).unwrap_or(r.pline);
            if pline.vertex_count() < 2 || pline.area().abs() <= CHAIN_TOL_MM * pline.path_length()
            {
                return None;
            }
            ccw(&mut pline);
            Some(pline)
        })
        .collect()
}

/// Whether `inner` (closed, not crossing `outer`) lies inside `outer`.
fn inside(inner: &Polyline<f64>, outer: &Polyline<f64>) -> bool {
    inner.vertex_count() > 0 && outer.winding_number(inner.at(0).pos()) != 0
}

fn union_of(plines: &[Polyline<f64>]) -> Result<Region, BooleanError> {
    let mut region = Region::default();
    for pline in plines {
        add(&mut region, pline)?;
    }
    Ok(region)
}

/// Region ∪ `shape`: holes it covers shrink or go; outlines it meets merge with it; a shape
/// wholly inside a hole is an island of its own.
fn add(region: &mut Region, shape: &Polyline<f64>) -> Result<(), BooleanError> {
    let mut island = false;
    let mut holes = Vec::new();
    for hole in std::mem::take(&mut region.holes) {
        let r = boolean(&hole, shape, BooleanOp::Not)?;
        match r.result_info {
            BooleanResultInfo::Disjoint => holes.push(hole),
            BooleanResultInfo::Pline2InsidePline1 => {
                island = true;
                holes.push(hole);
            }
            BooleanResultInfo::Intersected => holes.extend(plines_of(r.pos_plines)),
            // The hole lies inside the shape (or is it): filled.
            _ => {}
        }
    }
    region.holes = holes;
    let mut merged = shape.clone();
    let mut outers = Vec::new();
    for outer in std::mem::take(&mut region.outers) {
        let r = boolean(&outer, &merged, BooleanOp::Or)?;
        match r.result_info {
            BooleanResultInfo::Disjoint => outers.push(outer),
            // In a hole of this outline: the outline stays and the shape is an island.
            BooleanResultInfo::Pline2InsidePline1 if island => outers.push(outer),
            _ => {
                let mut pos = plines_of(r.pos_plines);
                pos.sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
                let mut pos = pos.into_iter();
                match pos.next() {
                    Some(largest) => merged = largest,
                    None => return Err(BooleanError::Library),
                }
                outers.extend(pos);
                region.holes.extend(plines_of(r.neg_plines));
            }
        }
    }
    outers.push(merged);
    region.outers = outers;
    Ok(())
}

/// A piece of the result with the holes cut in it so far.
struct Part {
    outer: Polyline<f64>,
    holes: Vec<Polyline<f64>>,
}

/// `base` with `cutters` cut out: the cutters are joined first (a ring of them cuts a ring),
/// then each of their outlines, largest first, is cut from every part; material inside a
/// ring's hole stays.
fn subtract(base: &Polyline<f64>, cutters: &[Polyline<f64>]) -> Result<Region, BooleanError> {
    let mut cut = union_of(cutters)?;
    cut.outers
        .sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
    let mut parts = vec![Part {
        outer: base.clone(),
        holes: Vec::new(),
    }];
    for outer in &cut.outers {
        let mut next = Vec::new();
        for part in parts {
            next.extend(cut_part(part, outer)?);
        }
        // Material of the base inside this cutter's holes (a ring) is kept.
        for hole in cut.holes.iter().filter(|h| inside(h, outer)) {
            let r = boolean(base, hole, BooleanOp::And)?;
            next.extend(plines_of(r.pos_plines).into_iter().map(|outer| Part {
                outer,
                holes: Vec::new(),
            }));
        }
        parts = next;
    }
    Ok(Region {
        outers: parts.iter().map(|p| p.outer.clone()).collect(),
        holes: parts.into_iter().flat_map(|p| p.holes).collect(),
    })
}

/// `part` without `cutter`: unchanged when they don't meet, with a new hole when the cutter
/// lies inside it (not inside one of its holes), in pieces when they cross.
fn cut_part(part: Part, cutter: &Polyline<f64>) -> Result<Vec<Part>, BooleanError> {
    let r = boolean(&part.outer, cutter, BooleanOp::Not)?;
    Ok(match r.result_info {
        BooleanResultInfo::Disjoint => vec![part],
        BooleanResultInfo::Pline2InsidePline1 => {
            let mut part = part;
            if !part.holes.iter().any(|h| inside(cutter, h)) {
                part.holes.push(cutter.clone());
            }
            vec![part]
        }
        BooleanResultInfo::Intersected => {
            let pieces = plines_of(r.pos_plines);
            let mut parts: Vec<Part> = pieces
                .into_iter()
                .map(|outer| Part {
                    outer,
                    holes: Vec::new(),
                })
                .collect();
            for hole in part.holes {
                if let Some(home) = parts.iter_mut().find(|p| inside(&hole, &p.outer)) {
                    home.holes.push(hole);
                }
            }
            parts
        }
        // The cutter covers the part.
        _ => Vec::new(),
    })
}

/// What every one of `plines` covers.
fn intersect(plines: &[Polyline<f64>]) -> Result<Region, BooleanError> {
    let mut outers = vec![plines[0].clone()];
    for pline in &plines[1..] {
        let mut next = Vec::new();
        for outer in &outers {
            let r = boolean(outer, pline, BooleanOp::And)?;
            next.extend(plines_of(r.pos_plines));
        }
        outers = next;
    }
    Ok(Region {
        outers,
        holes: Vec::new(),
    })
}

/// Every result closed and not crossing itself, and its area as the inputs' allow: a
/// union's at least the largest input's and at most their sum, an intersection's at most
/// the smallest's, a subtraction's at most the base's.
fn verify(op: Combine, inputs: &[Polyline<f64>], region: &Region) -> Result<(), BooleanError> {
    let unverified = |what: &str| Err(BooleanError::Unverified(what.to_string()));
    for pline in region.outers.iter().chain(&region.holes) {
        if !pline.is_closed() || pline.vertex_count() < 2 {
            return unverified("a result is not closed");
        }
        if pline.scan_for_self_intersect() {
            return unverified("a result crosses itself");
        }
    }
    let areas: Vec<f64> = inputs.iter().map(|p| p.area().abs()).collect();
    let area = region.area();
    // An area tolerance: the chaining tolerance along the inputs' whole length.
    let slack = CHAIN_TOL_MM * inputs.iter().map(|p| p.path_length()).sum::<f64>();
    let largest = areas.iter().copied().fold(0.0, f64::max);
    let smallest = areas.iter().copied().fold(f64::INFINITY, f64::min);
    let ok = match op {
        Combine::Union => area >= largest - slack && area <= areas.iter().sum::<f64>() + slack,
        Combine::Intersect => area > 0.0 && area <= smallest + slack,
        Combine::Subtract => area > 0.0 && area <= areas[0] + slack,
    };
    if !ok {
        return unverified("its area doesn't fit the shapes'");
    }
    Ok(())
}

/// The region as pieces in order (outlines, then holes, each largest first), each marked
/// with the input it is when it is one unchanged.
fn pieces(region: Region, inputs: &[Chain]) -> Vec<Piece> {
    let by_area = |mut ps: Vec<Polyline<f64>>| {
        ps.sort_by(|a, b| b.area().abs().total_cmp(&a.area().abs()));
        ps
    };
    let outers = by_area(region.outers).into_iter().map(|p| (p, false));
    let holes = by_area(region.holes).into_iter().map(|p| (p, true));
    let mut taken = vec![false; inputs.len()];
    outers
        .chain(holes)
        .map(|(pline, hole)| {
            let chain = from_pline(&pline);
            let from =
                (0..inputs.len()).find(|&i| !taken[i] && same_outline(&pline, &chain, &inputs[i]));
            if let Some(i) = from {
                taken[i] = true;
            }
            Piece { chain, hole, from }
        })
        .collect()
}

/// Whether a result is input `input` itself: the same area, and each one's points on the
/// other.
fn same_outline(pline: &Polyline<f64>, chain: &Chain, input: &Chain) -> bool {
    let input_pline = to_pline(input);
    let slack = CHAIN_TOL_MM * input_pline.path_length();
    if (pline.area().abs() - input_pline.area().abs()).abs() > slack {
        return false;
    }
    let on = |a: &Chain, b: &Chain| {
        a.prims
            .iter()
            .flat_map(|p| [p.start(), p.midpoint()])
            .all(|p| nearest_distance(b, p) < CHAIN_TOL_MM)
    };
    on(chain, input) && on(input, chain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_core::primitives::{Arc, Line, Primitive};
    use std::f64::consts::PI;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Chain {
        let c = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
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

    fn circle(x: f64, y: f64, r: f64) -> Chain {
        Chain {
            prims: vec![Primitive::Arc(Arc {
                center: (x, y),
                radius: r,
                start_angle: 0.0,
                sweep: 2.0 * PI,
            })],
            closed: true,
        }
    }

    fn area(piece: &Piece) -> f64 {
        to_pline(&piece.chain).area().abs()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn two_overlapping_squares_unite_into_one_outline() {
        let out = combine(
            Combine::Union,
            &[rect(0.0, 0.0, 10.0, 10.0), rect(5.0, 5.0, 15.0, 15.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert!(close(area(&out[0]), 175.0));
        assert_eq!(out[0].from, None);
        assert!(!out[0].hole);
    }

    #[test]
    fn disjoint_shapes_unite_unchanged() {
        let out = combine(
            Combine::Union,
            &[rect(0.0, 0.0, 10.0, 10.0), circle(30.0, 0.0, 5.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].from, Some(0));
        assert_eq!(out[1].from, Some(1));
    }

    #[test]
    fn a_shape_inside_another_is_swallowed_by_a_union() {
        let out = combine(
            Combine::Union,
            &[circle(5.0, 5.0, 2.0), rect(0.0, 0.0, 10.0, 10.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].from, Some(1));
    }

    #[test]
    fn touching_squares_unite_along_their_shared_side() {
        let out = combine(
            Combine::Union,
            &[rect(0.0, 0.0, 10.0, 10.0), rect(10.0, 0.0, 20.0, 10.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert!(close(area(&out[0]), 200.0));
    }

    #[test]
    fn a_ring_of_bars_unites_into_an_outline_and_a_hole() {
        let out = combine(
            Combine::Union,
            &[
                rect(0.0, 0.0, 30.0, 5.0),
                rect(25.0, 0.0, 30.0, 30.0),
                rect(0.0, 25.0, 30.0, 30.0),
                rect(0.0, 0.0, 5.0, 30.0),
            ],
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert!(!out[0].hole && out[1].hole);
        assert!(close(area(&out[0]), 900.0));
        assert!(close(area(&out[1]), 400.0));
    }

    #[test]
    fn a_shape_in_the_hole_of_a_ring_stays_an_island() {
        let out = combine(
            Combine::Union,
            &[
                rect(0.0, 0.0, 30.0, 5.0),
                rect(25.0, 0.0, 30.0, 30.0),
                rect(0.0, 25.0, 30.0, 30.0),
                rect(0.0, 0.0, 5.0, 30.0),
                circle(15.0, 15.0, 3.0),
            ],
        )
        .unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[1].from, Some(4));
        assert!(out[2].hole);
    }

    #[test]
    fn a_shape_bridging_a_ring_splits_its_hole() {
        let out = combine(
            Combine::Union,
            &[
                rect(0.0, 0.0, 30.0, 5.0),
                rect(25.0, 0.0, 30.0, 30.0),
                rect(0.0, 25.0, 30.0, 30.0),
                rect(0.0, 0.0, 5.0, 30.0),
                rect(14.0, 0.0, 16.0, 30.0),
            ],
        )
        .unwrap();
        assert_eq!(out.iter().filter(|p| p.hole).count(), 2);
        assert!(close(area(&out[0]), 900.0));
    }

    #[test]
    fn a_hole_inside_the_base_is_cut_as_a_hole() {
        let out = combine(
            Combine::Subtract,
            &[rect(0.0, 0.0, 20.0, 20.0), circle(10.0, 10.0, 3.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].from, Some(0));
        assert!(out[1].hole);
        assert_eq!(out[1].from, Some(1));
    }

    #[test]
    fn a_notch_cut_into_the_base_keeps_true_arcs() {
        let out = combine(
            Combine::Subtract,
            &[rect(0.0, 0.0, 20.0, 20.0), circle(20.0, 10.0, 5.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert!(close(area(&out[0]), 400.0 - PI * 25.0 / 2.0));
        assert!(
            out[0]
                .chain
                .prims
                .iter()
                .any(|p| matches!(p, Primitive::Arc(a) if close(a.radius, 5.0)))
        );
    }

    #[test]
    fn a_bar_across_the_base_cuts_it_in_two() {
        let out = combine(
            Combine::Subtract,
            &[rect(0.0, 0.0, 20.0, 10.0), rect(9.0, -1.0, 11.0, 11.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert!(close(area(&out[0]), 90.0) && close(area(&out[1]), 90.0));
    }

    #[test]
    fn overlapping_holes_are_cut_as_one() {
        let out = combine(
            Combine::Subtract,
            &[
                rect(0.0, 0.0, 30.0, 30.0),
                circle(12.0, 15.0, 4.0),
                circle(18.0, 15.0, 4.0),
            ],
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert!(out[1].hole);
    }

    #[test]
    fn subtracting_a_ring_keeps_what_lies_in_its_hole() {
        let out = combine(
            Combine::Subtract,
            &[
                rect(0.0, 0.0, 30.0, 30.0),
                rect(5.0, 5.0, 25.0, 8.0),
                rect(22.0, 5.0, 25.0, 25.0),
                rect(5.0, 22.0, 25.0, 25.0),
                rect(5.0, 5.0, 8.0, 25.0),
            ],
        )
        .unwrap();
        // The base with a ring-shaped slot: its outline, the slot's outer edge as a hole,
        // and the square island in the middle.
        let outers: Vec<f64> = out.iter().filter(|p| !p.hole).map(area).collect();
        assert_eq!(outers.len(), 2);
        assert!(close(outers[0], 900.0) && close(outers[1], 196.0));
        assert_eq!(out.iter().filter(|p| p.hole).count(), 1);
    }

    #[test]
    fn subtracting_everything_is_refused() {
        let out = combine(
            Combine::Subtract,
            &[rect(2.0, 2.0, 8.0, 8.0), rect(0.0, 0.0, 10.0, 10.0)],
        );
        assert_eq!(out, Err(BooleanError::Empty));
    }

    #[test]
    fn a_cutter_that_misses_leaves_the_base_unchanged() {
        let out = combine(
            Combine::Subtract,
            &[rect(0.0, 0.0, 10.0, 10.0), circle(30.0, 0.0, 2.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].from, Some(0));
    }

    #[test]
    fn overlapping_shapes_intersect_in_their_common_part() {
        let out = combine(
            Combine::Intersect,
            &[
                rect(0.0, 0.0, 10.0, 10.0),
                rect(5.0, 5.0, 15.0, 15.0),
                rect(0.0, 7.0, 20.0, 20.0),
            ],
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert!(close(area(&out[0]), 15.0));
    }

    #[test]
    fn nested_shapes_intersect_in_the_inner_one() {
        let out = combine(
            Combine::Intersect,
            &[rect(0.0, 0.0, 10.0, 10.0), circle(5.0, 5.0, 2.0)],
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].from, Some(1));
    }

    #[test]
    fn disjoint_shapes_have_no_intersection() {
        let out = combine(
            Combine::Intersect,
            &[rect(0.0, 0.0, 10.0, 10.0), circle(30.0, 0.0, 2.0)],
        );
        assert_eq!(out, Err(BooleanError::Empty));
    }

    #[test]
    fn open_shapes_and_single_shapes_are_refused() {
        let mut open = rect(0.0, 0.0, 10.0, 10.0);
        open.closed = false;
        open.prims.pop();
        assert_eq!(
            combine(Combine::Union, &[rect(0.0, 0.0, 1.0, 1.0), open]),
            Err(BooleanError::Open(1))
        );
        assert_eq!(
            combine(Combine::Union, &[rect(0.0, 0.0, 1.0, 1.0)]),
            Err(BooleanError::TooFew)
        );
    }

    #[test]
    fn a_clockwise_input_combines_as_a_counter_clockwise_one() {
        let mut cw = rect(5.0, 5.0, 15.0, 15.0);
        cw.prims.reverse();
        for p in &mut cw.prims {
            if let Primitive::Line(l) = p {
                std::mem::swap(&mut l.start, &mut l.end);
            }
        }
        let out = combine(Combine::Union, &[rect(0.0, 0.0, 10.0, 10.0), cw]).unwrap();
        assert_eq!(out.len(), 1);
        assert!(close(area(&out[0]), 175.0));
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// A union covers every input: each input's points lie inside its outline (or
            /// on it), never in a hole.
            #[test]
            fn a_union_contains_every_input(
                shapes in prop::collection::vec(
                    (-20.0..20.0f64, -20.0..20.0f64, 1.0..10.0f64, 1.0..10.0f64, any::<bool>()),
                    2..5,
                ),
            ) {
                let inputs: Vec<Chain> = shapes
                    .iter()
                    .map(|&(x, y, w, h, round)| if round {
                        circle(x, y, w)
                    } else {
                        rect(x, y, x + w, y + h)
                    })
                    .collect();
                let Ok(out) = combine(Combine::Union, &inputs) else {
                    // A refusal is allowed (a result the check doesn't trust); a wrong
                    // result is not.
                    return Ok(());
                };
                let outers: Vec<Polyline<f64>> =
                    out.iter().filter(|p| !p.hole).map(|p| to_pline(&p.chain)).collect();
                let holes: Vec<Polyline<f64>> =
                    out.iter().filter(|p| p.hole).map(|p| to_pline(&p.chain)).collect();
                for input in &inputs {
                    for prim in &input.prims {
                        let p = prim.midpoint();
                        let on_edge = out.iter().any(|q| nearest_distance(&q.chain, p) < 1e-3);
                        if on_edge {
                            continue;
                        }
                        let v = cavalier_contours::core::math::Vector2::new(p.0, p.1);
                        let depth = outers.iter().filter(|o| o.winding_number(v) != 0).count()
                            as i64
                            - holes.iter().filter(|h| h.winding_number(v) != 0).count() as i64;
                        prop_assert!(depth > 0, "{p:?} of an input is outside the union");
                    }
                }
            }
        }
    }
}
