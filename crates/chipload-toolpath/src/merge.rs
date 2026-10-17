// SPDX-License-Identifier: GPL-3.0-or-later
//! Overlapping outlines merged into ones that don't cross (docs/specs/more-fonts.md §4): the
//! letters of a text in a connected script, or a letter drawn as overlapping strokes.
//!
//! The loops are read by the non-zero fill rule, as fonts are: every loop of the main
//! orientation encloses fill, every loop of the other is a hole in it. The fill is united
//! with cavalier_contours' booleans (through `cavalier.rs` and `left.rs`), arcs kept as
//! arcs. Loops that touch nothing else are returned exactly as they came. The result is
//! checked against the input by sampling: a point is filled in both or in neither.

use cavalier_contours::core::math::Vector2;
use cavalier_contours::polyline::{
    BooleanOp, PlineContainsResult, PlineSource, PlineSourceMut, Polyline,
};
use chipload_core::Point;
use chipload_core::order::Ordered;
use chipload_core::primitives::{Arc, Primitive};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};

use crate::ToolpathError;
use crate::cavalier::{Area, boolean, ccw, cw, encloses, geometry, guarded, too_small};
use crate::left::minus_area;
use crate::offset::{pline_segments, point_distance};

/// Sample points per side of the box round a merged group, for the check.
const CHECK_SAMPLES: usize = 64;

/// `loops` (closed, lines and arcs end to end) as loops that don't cross: overlapping fill
/// united, holes kept where nothing covers them. `glyph` says which letter each loop is
/// of: a hole belongs to the smallest loop of its own letter round it. Loops that overlap
/// nothing come back unchanged and in their order, followed by the merged ones, each with
/// its letter (a merged loop: the first letter of those it joins).
pub fn merge_overlaps(
    loops: &[Vec<Primitive>],
    glyph: &[usize],
) -> Result<Vec<(Vec<Primitive>, usize)>, ToolpathError> {
    let plines: Vec<Polyline<f64>> = loops.iter().map(|l| prims_pline(l)).collect();
    let n = plines.len();
    if n < 2 {
        return Ok(loops.iter().cloned().zip(glyph.iter().copied()).collect());
    }
    // The main orientation: that of the largest loop no other loop encloses (an outline).
    let fill_sign = main_orientation(&plines);
    let is_fill: Vec<bool> = plines.iter().map(|p| p.area() * fill_sign > 0.0).collect();
    // Each hole's owner: the smallest fill loop of its letter round it.
    let owner: Vec<usize> = (0..n)
        .map(|i| {
            if is_fill[i] {
                return i;
            }
            let inside = first_point(&plines[i]);
            (0..n)
                .filter(|&j| {
                    is_fill[j] && glyph.get(j) == glyph.get(i) && encloses(&plines[j], inside)
                })
                .min_by_key(|&j| Ordered(plines[j].area().abs()))
                .unwrap_or(i)
        })
        .collect();
    // Families (a fill loop and its holes) that interact are merged together.
    let mut group: Vec<usize> = (0..n).map(|i| owner[i]).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            if owner[i] == owner[j] || !interact(&plines[i], &plines[j], is_fill[i], is_fill[j])? {
                continue;
            }
            let (a, b) = (root(&group, owner[i]), root(&group, owner[j]));
            if a != b {
                group[a.max(b)] = a.min(b);
            }
        }
    }
    let roots: Vec<usize> = (0..n).map(|i| root(&group, owner[i])).collect();
    let letter = |i: usize| glyph.get(i).copied().unwrap_or(0);
    let mut out: Vec<(Vec<Primitive>, usize)> = Vec::new();
    let mut merged_roots: Vec<usize> = Vec::new();
    for r in 0..n {
        let members: Vec<usize> = (0..n).filter(|&i| roots[i] == r).collect();
        let families = members.iter().filter(|&&i| is_fill[i]).count();
        if families > 1 {
            merged_roots.push(r);
        }
    }
    // Untouched loops first, exactly as they were.
    for i in 0..n {
        if !merged_roots.contains(&roots[i]) {
            out.push((loops[i].clone(), letter(i)));
        }
    }
    for &r in &merged_roots {
        let members: Vec<usize> = (0..n).filter(|&i| roots[i] == r).collect();
        let areas = merge_group(&plines, &members, &is_fill, &owner)?;
        check_group(&plines, &members, &areas)?;
        let first = members.iter().map(|&i| letter(i)).min().unwrap_or(0);
        for area in &areas {
            // A sliver left where two edges nearly coincide is no part of a letter.
            if sliver(&area.outer) {
                continue;
            }
            out.push((three_or_more(pline_segments(&area.outer)), first));
            out.extend(
                area.holes
                    .iter()
                    .filter(|h| !sliver(h))
                    .map(|h| (three_or_more(pline_segments(h)), first)),
            );
        }
    }
    Ok(out)
}

/// A loop of fewer than three pieces (a lens: a line and an arc) with its arcs split in
/// two, so it has three corners: a shape needs three distinct points.
fn three_or_more(prims: Vec<Primitive>) -> Vec<Primitive> {
    if prims.len() >= 3 {
        return prims;
    }
    prims
        .into_iter()
        .flat_map(|p| match p {
            Primitive::Arc(arc) => {
                let half = arc.sweep / 2.0;
                vec![
                    Primitive::Arc(Arc { sweep: half, ..arc }),
                    Primitive::Arc(Arc {
                        start_angle: arc.start_angle + half,
                        sweep: half,
                        ..arc
                    }),
                ]
            }
            line => vec![line],
        })
        .collect()
}

/// A loop thinner than the arc tolerance on average (twice its area over its length): a
/// hairline the booleans leave where two edges nearly coincide.
fn sliver(pline: &Polyline<f64>) -> bool {
    let length = pline.path_length();
    length <= 0.0 || 2.0 * pline.area().abs() / length < ARC_APPROX_TOL_MM
}

/// The loop as a closed polyline with bulges; arcs over half a turn are split.
fn prims_pline(prims: &[Primitive]) -> Polyline<f64> {
    let mut pline = Polyline::new_closed();
    for prim in prims {
        match *prim {
            Primitive::Line(line) => pline.add(line.start.0, line.start.1, 0.0),
            Primitive::Arc(arc) => {
                let pieces = if arc.sweep.abs() > std::f64::consts::PI {
                    2
                } else {
                    1
                };
                let sweep = arc.sweep / f64::from(pieces);
                for i in 0..pieces {
                    let piece = Arc {
                        start_angle: arc.start_angle + sweep * f64::from(i),
                        sweep,
                        ..arc
                    };
                    let start = piece.point_at(piece.start_angle);
                    pline.add(start.0, start.1, piece.bulge());
                }
            }
        }
    }
    pline
}

fn first_point(pline: &Polyline<f64>) -> Point {
    let v = pline.at(0);
    (v.x, v.y)
}

/// +1 for counter-clockwise fill, -1 for clockwise: the orientation of the largest loop
/// that no other loop encloses.
fn main_orientation(plines: &[Polyline<f64>]) -> f64 {
    let outermost = (0..plines.len())
        .filter(|&i| {
            let p = first_point(&plines[i]);
            !(0..plines.len()).any(|j| j != i && encloses(&plines[j], p))
        })
        .max_by_key(|&i| Ordered(plines[i].area().abs()));
    match outermost {
        Some(i) if plines[i].area() < 0.0 => -1.0,
        _ => 1.0,
    }
}

/// Whether two loops of different families affect each other's fill: they cross, or one
/// lies inside the other's fill.
fn interact(
    a: &Polyline<f64>,
    b: &Polyline<f64>,
    a_fill: bool,
    b_fill: bool,
) -> Result<bool, ToolpathError> {
    Ok(match contact(&ccw(a.clone()), &ccw(b.clone()))? {
        Contact::Overlap => true,
        // A fill inside the other's fill loop, or a hole inside it: the fill rule decides.
        Contact::BInsideA => a_fill,
        Contact::AInsideB => b_fill,
        Contact::Apart => false,
    })
}

/// How two counter-clockwise loops lie.
enum Contact {
    Apart,
    AInsideB,
    BInsideA,
    Overlap,
}

/// Loops whose edges meet only where they touch (at a point, or along an edge with no area
/// in common) are apart: letters that touch stay apart (§4).
fn contact(a: &Polyline<f64>, b: &Polyline<f64>) -> Result<Contact, ToolpathError> {
    match a.contains(b) {
        PlineContainsResult::Disjoint => Ok(Contact::Apart),
        PlineContainsResult::Pline1InsidePline2 => Ok(Contact::AInsideB),
        PlineContainsResult::Pline2InsidePline1 => Ok(Contact::BInsideA),
        PlineContainsResult::Intersected => {
            let common = boolean("Joining letters", a, b, BooleanOp::And)?;
            let shared: f64 = common.pos_plines.iter().map(|p| p.pline.area().abs()).sum();
            Ok(if shared <= CHAIN_TOL_MM * CHAIN_TOL_MM {
                Contact::Apart
            } else {
                Contact::Overlap
            })
        }
        PlineContainsResult::InvalidInput => Err(geometry(
            "an outline the geometry library can't read".into(),
        )),
    }
}

fn root(group: &[usize], mut i: usize) -> usize {
    while group[i] != i {
        i = group[i];
    }
    i
}

/// One group's fill as areas: the fill loops united, then each hole, less every other
/// family's fill, cut out.
fn merge_group(
    plines: &[Polyline<f64>],
    members: &[usize],
    is_fill: &[bool],
    owner: &[usize],
) -> Result<Vec<Area>, ToolpathError> {
    let fills: Vec<usize> = members.iter().copied().filter(|&i| is_fill[i]).collect();
    // The fill loops united, counter-clockwise; enclosed gaps the union makes are holes.
    let mut outers: Vec<Polyline<f64>> = Vec::new();
    let mut gaps: Vec<Polyline<f64>> = Vec::new();
    for &i in &fills {
        let mut current = ccw(plines[i].clone());
        let mut k = 0;
        while k < outers.len() {
            match contact(&current, &outers[k])? {
                Contact::Apart => k += 1,
                Contact::BInsideA => {
                    outers.swap_remove(k);
                }
                Contact::AInsideB => {
                    current = outers.swap_remove(k);
                }
                Contact::Overlap => {
                    let other = outers.swap_remove(k);
                    let union = boolean("Joining letters", &current, &other, BooleanOp::Or)?;
                    let mut pos: Vec<Polyline<f64>> = union
                        .pos_plines
                        .into_iter()
                        .map(|p| p.pline)
                        .filter(|p| !too_small(p))
                        .collect();
                    if pos.len() != 1 {
                        return Err(geometry(format!(
                            "joining two overlapping outlines gave {} outlines",
                            pos.len()
                        )));
                    }
                    current = ccw(pos.swap_remove(0));
                    gaps.extend(
                        union
                            .neg_plines
                            .into_iter()
                            .map(|p| p.pline)
                            .filter(|p| !too_small(p))
                            .map(ccw),
                    );
                    // Start again: the larger outline may now reach ones passed over.
                    k = 0;
                }
            }
        }
        outers.push(current);
    }
    let mut pieces: Vec<Area> = outers.into_iter().map(Area::of).collect();
    // A gap the union made stays open only where no fill loop covers it.
    let fill_areas: Vec<Area> = fills
        .iter()
        .map(|&i| Area {
            outer: ccw(plines[i].clone()),
            holes: members
                .iter()
                .copied()
                .filter(|&h| !is_fill[h] && owner[h] == i)
                .map(|h| cw(plines[h].clone()))
                .collect(),
        })
        .collect();
    let mut cutters: Vec<Area> = Vec::new();
    for gap in gaps {
        let mut open = vec![Area::of(gap)];
        for fill in &fill_areas {
            open = minus_area(open, fill)?;
        }
        cutters.extend(open);
    }
    // A hole stays open where no other family's fill covers it.
    for (k, fill) in fill_areas.iter().enumerate() {
        for hole in &fill.holes {
            let mut open = vec![Area::of(ccw(hole.clone()))];
            for (m, other) in fill_areas.iter().enumerate() {
                if m != k {
                    open = minus_area(open, other)?;
                }
            }
            cutters.extend(open);
        }
    }
    for cutter in &cutters {
        pieces = minus_area(pieces, cutter)?;
    }
    Ok(pieces)
}

/// The merged areas fill what the loops fill by the non-zero rule: checked on a grid over
/// the group's box, away from every edge (where rounding may put a point either side).
fn check_group(
    plines: &[Polyline<f64>],
    members: &[usize],
    areas: &[Area],
) -> Result<(), ToolpathError> {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &i in members {
        if let Some(e) = plines[i].extents() {
            lo = (lo.0.min(e.min_x), lo.1.min(e.min_y));
            hi = (hi.0.max(e.max_x), hi.1.max(e.max_y));
        }
    }
    let edges: Vec<Primitive> = members
        .iter()
        .flat_map(|&i| pline_segments(&plines[i]))
        .chain(areas.iter().flat_map(Area::boundary))
        .collect();
    let margin = 2.0 * ARC_APPROX_TOL_MM;
    for sx in 0..=CHECK_SAMPLES {
        for sy in 0..=CHECK_SAMPLES {
            let p = (
                lo.0 + (hi.0 - lo.0) * sx as f64 / CHECK_SAMPLES as f64,
                lo.1 + (hi.1 - lo.1) * sy as f64 / CHECK_SAMPLES as f64,
            );
            if edges.iter().any(|e| point_distance(e, p) <= margin) {
                continue;
            }
            let winding: i32 = members
                .iter()
                .map(|&i| {
                    guarded("Checking joined letters", || {
                        plines[i].winding_number(Vector2::new(p.0, p.1))
                    })
                })
                .sum::<Result<i32, ToolpathError>>()?;
            // The non-zero rule: a hole inside its own outline winds back to 0.
            let filled = winding != 0;
            let merged = areas.iter().any(|a| a.contains(p));
            if filled != merged {
                return Err(geometry(format!(
                    "joining overlapping outlines changed what is filled near ({:.3}, {:.3}) mm",
                    p.0, p.1
                )));
            }
        }
    }
    // No loop of the result crosses another.
    let loops: Vec<&Polyline<f64>> = areas
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(a.holes.iter()))
        .collect();
    for (k, a) in loops.iter().enumerate() {
        for b in &loops[..k] {
            if matches!(
                ccw((*a).clone()).contains(&ccw((*b).clone())),
                PlineContainsResult::Intersected
            ) {
                return Err(geometry("joined outlines cross each other".into()));
            }
        }
    }
    Ok(())
}
