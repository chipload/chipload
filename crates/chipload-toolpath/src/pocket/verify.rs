// SPDX-License-Identifier: GPL-3.0-or-later
//! Links kept clear of the walls, and the finished cut verified: material left uncut.

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::cancel::{self, Cancel};
use crate::cavalier::Area;
use crate::geom::PrimIndex;
use cavalier_contours::polyline::PlineSource;
use chipload_core::Point;
use chipload_core::order::Ordered;
use chipload_core::primitives::{Line, Primitive, distance};
use chipload_core::tolerances::{
    ARC_APPROX_TOL_MM, CHAIN_TOL_MM, OFFSET_CHECK_SLACK_MM, REST_TOL_MM,
};
use chipload_i18n::{Digits, Message, Value};
use std::f64::consts::SQRT_2;

use super::rounding::{UNCUT_CELLS_PER_CHECK, segment_is_clear, verification_failed};
use super::{PocketRing, ids_text};

pub(super) fn min_clearance(radius: f64) -> f64 {
    radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM
}

/// The segment keeps at least `clearance` from every primitive in the index.
pub(super) fn clear_of(index: &PrimIndex, a: Point, b: Point, clearance: f64) -> bool {
    index
        .segment_distance_within(a, b, clearance)
        .is_none_or(|d| d >= clearance)
}

/// Staying down is safe if the tool can't reach a wall or island and the link is short.
/// `offset`: how far the tool centre keeps from the walls; `radius`: the tool's.
pub(super) fn link_is_safe(
    region: &Area,
    walls: &PrimIndex,
    a: Point,
    b: Point,
    radius: f64,
    offset: f64,
) -> bool {
    let length = distance(a, b);
    if length > 2.0 * radius {
        return false;
    }
    if length < CHAIN_TOL_MM {
        return true;
    }
    region.contains(a) && clear_of(walls, a, b, min_clearance(offset))
}

/// Moving back at depth between passes is safe if the piece is cleared everywhere the tool
/// reaches and the move keeps the tool off every wall and island; any length, as it runs
/// through material already cut (docs/specs/pocket-stay-down.md §2).
pub(super) fn back_link_is_safe(
    region: &Area,
    walls: &PrimIndex,
    a: Point,
    b: Point,
    offset: f64,
) -> bool {
    if distance(a, b) < CHAIN_TOL_MM {
        return true;
    }
    region.contains(a) && clear_of(walls, a, b, min_clearance(offset))
}

/// Every cutting move as a segment: ring chords (closed) and links fed at depth.
pub(super) fn cutting_segments(rings: &[PocketRing], links: &[bool]) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();
    for ring in rings {
        let points = &ring.points;
        let n = points.len();
        match n {
            0 => {}
            1 => segments.push((points[0], points[0])),
            2 => segments.push((points[0], points[1])),
            _ if ring.open => segments.extend(points.windows(2).map(|w| (w[0], w[1]))),
            _ => segments.extend((0..n).map(|i| (points[i], points[(i + 1) % n]))),
        }
    }
    for (i, &stays_down) in links.iter().enumerate() {
        let (Some(a), b) = (rings[i].end(), rings[i + 1].points[0]) else {
            continue;
        };
        if stays_down && distance(a, b) >= CHAIN_TOL_MM {
            segments.push((a, b));
        }
    }
    segments
}

/// Post-condition: no cutting move comes closer than r (minus tolerance) to a wall or
/// island, and everything is inside the region. Never trust the offsets blindly.
pub(super) fn verify(
    region: &Area,
    walls: &PrimIndex,
    rings: &[PocketRing],
    links: &[bool],
    radius: f64,
    ids: &[usize],
    arcs: &ArcTable,
) -> Result<(), ToolpathError> {
    let clearance = min_clearance(radius);
    let inside = rings
        .iter()
        .all(|r| r.points.first().is_some_and(|&p| region.contains(p)));
    // Chords on arcs are cut as arcs when arcs are written: check where they bulge most too.
    let clear = cutting_segments(rings, links)
        .iter()
        .all(|&(a, b)| segment_is_clear(walls, a, b, clearance, arcs));
    if inside && clear {
        Ok(())
    } else {
        Err(verification_failed(ids))
    }
}

/// A square cell of the uncut-material search: centre and half its side.
#[derive(Debug, Clone, Copy)]
struct Cell {
    centre: Point,
    half: f64,
}

/// Material the tool could reach but the rings leave behind (spec section 3).
///
/// Leftover is `reachable − ∪ buffer(path, r)`, reported where a piece survives
/// `buffer(−REST_TOL_MM)`, where reachable is everything within r of the level-0
/// areas L0. The edges of L0 are the level-0 rings, which are cut, so any reachable point
/// outside L0 lies within r of a path (up to the chord tolerance): leftover only exists
/// inside L0. Here the sets are described by distances, with ε = REST_TOL_MM:
/// - leftover: p inside L0 and dist(p, paths) > r. Leftover thinner than the chord
///   tolerance is ignored, so slivers along flattened arcs can't join separate pieces.
/// - deep: p inside L0 and dist(p, paths) > r + ε, i.e. p is the centre of an uncut disk of
///   radius ε, which is what surviving the shrink means.
///
/// A quadtree decides whole cells from bounds on these distances and only refines down to
/// ε-sized cells where the answer changes. Leftover cells are grouped into pieces; pieces
/// containing a deep cell are reported, as before.
pub(super) fn uncut_material(
    region: &Area,
    reachable: &[Area],
    rings: &[PocketRing],
    links: &[bool],
    radius: f64,
    ids: &[usize],
    cancel: Option<&Cancel>,
) -> Result<(Option<Message>, Vec<Missed>), ToolpathError> {
    let eps = REST_TOL_MM;
    let thin = ARC_APPROX_TOL_MM;
    let cell_size = radius.max(1.0);
    let paths = PrimIndex::new(
        cutting_segments(rings, links)
            .into_iter()
            .map(|(start, end)| Primitive::Line(Line { start, end }))
            .collect(),
        cell_size,
    );
    let level0 = PrimIndex::new(
        reachable.iter().flat_map(Area::boundary).collect(),
        cell_size,
    );

    let Some(extents) = region.outer.extents() else {
        return Ok((None, Vec::new()));
    };
    let side = (extents.max_x - extents.min_x).max(extents.max_y - extents.min_y);
    let mut todo = vec![Cell {
        centre: (
            (extents.min_x + extents.max_x) / 2.0,
            (extents.min_y + extents.max_y) / 2.0,
        ),
        half: side / 2.0 + eps,
    }];
    // (cell, whether it holds a deep point)
    let mut leftover: Vec<(Cell, bool)> = Vec::new();
    let mut looked = 0usize;
    while let Some(cell) = todo.pop() {
        // A big pocket's check can take a while: a cancel is seen within it too
        // (docs/specs/background-generation.md).
        looked += 1;
        if looked.is_multiple_of(UNCUT_CELLS_PER_CHECK) {
            cancel::check(cancel)?;
        }
        let spread = cell.half * SQRT_2; // farthest any point of the cell is from its centre
        if paths.any_within(cell.centre, radius + thin - spread) {
            continue; // a path passes close enough to sweep the whole cell
        }
        let inside = reachable.iter().any(|a| a.contains(cell.centre));
        // Whether the L0 edge passes through the cell (None: farther than `spread`).
        let edge_near = level0.distance_within(cell.centre, spread).is_some();
        if !inside && !edge_near {
            continue; // entirely outside L0
        }
        // Only thresholds matter, so ask "is any path within d?", which stops at the first
        // hit, instead of computing the exact distance.
        let all_inside = inside && !edge_near;
        let all_leftover = all_inside && !paths.any_within(cell.centre, radius + thin + spread);
        let all_deep = all_inside && !paths.any_within(cell.centre, radius + eps + spread);
        let no_deep = paths.any_within(cell.centre, radius + eps - spread);
        if all_leftover && (all_deep || no_deep) {
            leftover.push((cell, all_deep));
            continue;
        }
        if cell.half * 2.0 <= eps {
            if inside && !paths.any_within(cell.centre, radius + thin) {
                leftover.push((cell, !paths.any_within(cell.centre, radius + eps)));
            }
            continue;
        }
        let h = cell.half / 2.0;
        for (dx, dy) in [(-h, -h), (h, -h), (-h, h), (h, h)] {
            todo.push(Cell {
                centre: (cell.centre.0 + dx, cell.centre.1 + dy),
                half: h,
            });
        }
    }
    let cells: Vec<Cell> = leftover.iter().map(|(c, _)| *c).collect();
    // Neighbouring cells share edges up to float noise from repeated halving.
    let spots: Vec<Vec<usize>> = connected(&cells, CHAIN_TOL_MM)
        .into_iter()
        .filter(|piece| piece.iter().any(|&i| leftover[i].1))
        .collect();
    if spots.is_empty() {
        return Ok((None, Vec::new()));
    }
    let missed: Vec<Missed> = spots
        .iter()
        .map(|spot| spot.iter().map(|&i| square(cells[i])).collect())
        .collect();
    let area =
        |spot: &[usize]| -> f64 { spot.iter().map(|&i| (2.0 * cells[i].half).powi(2)).sum() };
    let bounds = |spot: &[usize]| {
        spot.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(x0, y0, x1, y1), &i| {
                let c = cells[i];
                (
                    x0.min(c.centre.0 - c.half),
                    y0.min(c.centre.1 - c.half),
                    x1.max(c.centre.0 + c.half),
                    y1.max(c.centre.1 + c.half),
                )
            },
        )
    };
    // Largest spot by area; ties go to the lowest, then leftmost (as before).
    let largest = spots.iter().max_by_key(|spot| {
        let b = bounds(spot);
        (Ordered(area(spot)), Ordered(-b.1), Ordered(-b.0))
    });
    let Some(largest) = largest else {
        return Ok((None, missed));
    };
    let (x0, y0, x1, y1) = bounds(largest);
    // A point inside the piece: the centre of its biggest cell.
    let inside = largest
        .iter()
        .map(|&i| cells[i])
        .max_by_key(|c| Ordered(c.half));
    let Some(inside) = inside else {
        return Ok((None, missed));
    };
    let note = Message::new("toolpath-uncut")
        .arg("contours", ids_text(ids))
        .arg("count", spots.len())
        .arg("width", Value::LengthNumber(x1 - x0, Digits::Fixed(2)))
        .arg("height", Value::Length(y1 - y0, Digits::Fixed(2)))
        .arg(
            "near",
            Value::Point(inside.centre.0, inside.centre.1, Digits::Fixed(1)),
        );
    Ok((Some(note), missed))
}

/// A spot of missed material: the squares it is made of.
type Missed = Vec<Vec<Point>>;

/// A cell as a counter-clockwise square.
fn square(cell: Cell) -> Vec<Point> {
    let ((x, y), h) = (cell.centre, cell.half);
    vec![
        (x - h, y - h),
        (x + h, y - h),
        (x + h, y + h),
        (x - h, y + h),
    ]
}

/// Groups of cells that touch (within `gap`), as index lists in a deterministic order.
fn connected(cells: &[Cell], gap: f64) -> Vec<Vec<usize>> {
    let n = cells.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let (ca, cb) = (cells[a], cells[b]);
        (ca.centre.0 - ca.half).total_cmp(&(cb.centre.0 - cb.half))
    });
    for (k, &a) in order.iter().enumerate() {
        let ca = cells[a];
        for &b in &order[k + 1..] {
            let cb = cells[b];
            if cb.centre.0 - cb.half > ca.centre.0 + ca.half + gap {
                break; // sorted by left edge: nothing further right can touch
            }
            let touch_y = (ca.centre.1 - cb.centre.1).abs() <= ca.half + cb.half + gap;
            if touch_y {
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for i in 0..n {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    groups.into_values().collect()
}
