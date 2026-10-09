// SPDX-License-Identifier: GPL-3.0-or-later
//! Pocket clearing: contour-parallel rings, cut from the inside out. See
//! docs/specs/pocket.md.
//!
//! Offsets use cavalier_contours shapes (outer loop plus islands, true arcs). Every result is
//! verified afterwards, and uncut material is found with exact distances instead of the
//! buffered-path union the Python reference used.

use std::f64::consts::SQRT_2;
use std::panic::{AssertUnwindSafe, catch_unwind};

use cavalier_contours::core::math::Vector2;
use cavalier_contours::polyline::{
    BooleanOp, PlineContainsResult, PlineOrientation, PlineSource, PlineSourceMut, Polyline,
};
use cavalier_contours::shape_algorithms::{Shape, ShapeOffsetOptions};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::nesting::{Ring, contains};
use chipload_core::primitives::{Line, Primitive, distance, signed_area};
use chipload_core::tolerances::{
    ARC_APPROX_TOL_MM, CHAIN_TOL_MM, OFFSET_CHECK_SLACK_MM, REST_TOL_MM,
};
use chipload_job::{Direction, Side, Spindle};

use crate::ToolpathError;
use crate::geom::PrimIndex;
use crate::left::{LeftKind, LeftPiece, reach, sort_pieces, unreachable_inside};
use crate::model::Move;
use crate::offset::{contour_pline, dedupe_ring, flatten, pline_segments, point_distance};
use crate::profile::cut_clockwise;

/// Guard against runaway loops on absurd input; a 2 m pocket at 0.1 mm stepover is 10 000.
pub const MAX_RING_LEVELS: usize = 20_000;

/// One connected area to clear, with its rings in cut order.
#[derive(Debug, Clone, PartialEq)]
pub struct PocketPiece {
    /// Each oriented, starting (and ending) at ring[0].
    pub rings: Vec<Vec<Point>>,
    /// feed_links[i]: stay at depth from rings[i] into rings[i + 1].
    pub feed_links: Vec<bool>,
    /// Selected contours that make up this piece, for messages.
    pub contour_ids: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PocketPlan {
    pub pieces: Vec<PocketPiece>,
    /// Warnings: narrow parts, uncut material.
    pub notes: Vec<String>,
    /// Material left, for display (unreachable corners and slots, missed spots).
    pub left: Vec<LeftPiece>,
}

pub(crate) fn geometry(message: String) -> ToolpathError {
    ToolpathError::Geometry(message)
}

fn ids_text(ids: &[usize]) -> String {
    ids.iter()
        .map(|i| format!("#{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Run a cavalier operation; a panic inside the library becomes a GeometryError.
pub(crate) fn guarded<T>(what: &str, f: impl FnOnce() -> T) -> Result<T, ToolpathError> {
    catch_unwind(AssertUnwindSafe(f))
        .map_err(|_| geometry(format!("{what} failed inside the geometry library")))
}

pub(crate) fn ccw(mut pline: Polyline<f64>) -> Polyline<f64> {
    if pline.orientation() == PlineOrientation::Clockwise {
        pline.invert_direction_mut();
    }
    pline
}

pub(crate) fn cw(mut pline: Polyline<f64>) -> Polyline<f64> {
    if pline.orientation() == PlineOrientation::CounterClockwise {
        pline.invert_direction_mut();
    }
    pline
}

pub(crate) fn first_point(pline: &Polyline<f64>) -> Point {
    let v = pline.at(0);
    (v.x, v.y)
}

pub(crate) fn encloses(pline: &Polyline<f64>, p: Point) -> bool {
    pline.winding_number(Vector2::new(p.0, p.1)) != 0
}

fn min_corner(pline: &Polyline<f64>) -> Point {
    pline.extents().map_or((0.0, 0.0), |e| (e.min_x, e.min_y))
}

/// An outer loop (counter-clockwise) minus holes (clockwise), all with true arcs.
#[derive(Debug, Clone)]
pub struct Area {
    pub outer: Polyline<f64>,
    pub holes: Vec<Polyline<f64>>,
}

impl Area {
    /// An area without holes.
    pub fn of(outer: Polyline<f64>) -> Self {
        Area {
            outer,
            holes: Vec::new(),
        }
    }

    pub(crate) fn contains(&self, p: Point) -> bool {
        encloses(&self.outer, p) && !self.holes.iter().any(|h| encloses(h, p))
    }

    pub(crate) fn boundary(&self) -> Vec<Primitive> {
        let mut prims = pline_segments(&self.outer);
        for hole in &self.holes {
            prims.extend(pline_segments(hole));
        }
        prims
    }

    /// Distance from `p` to the area: 0 inside.
    pub(crate) fn distance(&self, p: Point) -> f64 {
        if self.contains(p) {
            return 0.0;
        }
        self.edge_distance(p)
    }

    /// Distance from `p` to the nearest edge, inside or out.
    pub(crate) fn edge_distance(&self, p: Point) -> f64 {
        self.boundary()
            .iter()
            .map(|s| point_distance(s, p))
            .fold(f64::INFINITY, f64::min)
    }

    /// (min y, min x) of the outer loop, for deterministic ordering.
    fn sort_key(&self) -> (f64, f64) {
        let (x, y) = min_corner(&self.outer);
        (y, x)
    }

    /// The area shrunk by `d`: every part farther than `d` from the boundary.
    /// Negative `d` grows it.
    pub(crate) fn shrunk(&self, d: f64) -> Result<Vec<Area>, ToolpathError> {
        let plines = std::iter::once(self.outer.clone()).chain(self.holes.iter().cloned());
        let shape = Shape::from_plines(plines);
        let offset = guarded("Pocket offset", || {
            shape.parallel_offset(d, &ShapeOffsetOptions::new())
        })?;
        let keep = |p: &Polyline<f64>| p.area().abs() > CHAIN_TOL_MM * CHAIN_TOL_MM;
        let mut areas: Vec<Area> = offset
            .ccw_plines
            .into_iter()
            .map(|p| p.polyline)
            .filter(keep)
            .map(|outer| Area {
                outer,
                holes: Vec::new(),
            })
            .collect();
        for hole in offset
            .cw_plines
            .into_iter()
            .map(|p| p.polyline)
            .filter(keep)
        {
            let inside = first_point(&hole);
            let parent = areas
                .iter_mut()
                .filter(|a| encloses(&a.outer, inside))
                .min_by(|a, b| a.outer.area().abs().total_cmp(&b.outer.area().abs()));
            match parent {
                Some(area) => area.holes.push(hole),
                None => {
                    return Err(geometry(
                        "pocket offset produced an island outside every area".into(),
                    ));
                }
            }
        }
        areas.sort_by(|a, b| {
            let (ka, kb) = (a.sort_key(), b.sort_key());
            ka.0.total_cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        });
        Ok(areas)
    }
}

/// One selected boundary minus the selected islands inside it.
#[derive(Debug, Clone)]
pub struct PocketArea {
    pub area: Area,
    /// The boundary first, then its islands.
    pub contour_ids: Vec<usize>,
}

/// Areas to clear from selected closed contours, sorted by (y, x).
///
/// Every selected contour at even nesting depth (among the selection) is its own pocket, even
/// if it crosses another one: crossing shapes are separate geometries and are never merged
/// (user decision 2026-10-08). Selected contours lying completely inside it one level deeper
/// are its islands.
pub fn build_region(contours: &[&Contour]) -> Result<Vec<PocketArea>, ToolpathError> {
    let shapes: Vec<(usize, Polyline<f64>, Ring)> = contours
        .iter()
        .map(|c| {
            let points = dedupe_ring(&c.points());
            let mut pline = Polyline::new_closed();
            for &(x, y) in &points {
                pline.add(x, y, 0.0);
            }
            (c.id, ccw(contour_pline(c)), Ring { points, pline })
        })
        .collect();
    refuse_duplicates(&shapes)?;
    let depth: Vec<usize> = shapes
        .iter()
        .map(|(id, _, ring)| {
            shapes
                .iter()
                .filter(|(other, _, outer)| other != id && contains(outer, ring))
                .count()
        })
        .collect();
    let mut result = Vec::new();
    for (i, (id, boundary, ring)) in shapes.iter().enumerate() {
        if depth[i] % 2 == 1 {
            continue;
        }
        let islands: Vec<(usize, Polyline<f64>)> = shapes
            .iter()
            .enumerate()
            .filter(|(j, (_, _, island))| depth[*j] == depth[i] + 1 && contains(ring, island))
            .map(|(_, (iid, pline, _))| (*iid, pline.clone()))
            .collect();
        let mut ids = vec![*id];
        ids.extend(islands.iter().map(|(iid, _)| *iid));
        let islands = merge_islands(islands.into_iter().map(|(_, p)| p).collect(), &ids)?;
        for area in subtract_islands(boundary.clone(), islands)? {
            result.push(PocketArea {
                area,
                contour_ids: ids.clone(),
            });
        }
    }
    result.sort_by(|a, b| {
        let (ka, kb) = (a.area.sort_key(), b.area.sort_key());
        ka.0.total_cmp(&kb.0)
            .then(ka.1.total_cmp(&kb.1))
            .then(a.contour_ids.cmp(&b.contour_ids))
    });
    Ok(result)
}

fn boolean_area(result: &cavalier_contours::polyline::BooleanResult<Polyline<f64>>) -> f64 {
    let pos: f64 = result.pos_plines.iter().map(|p| p.pline.area().abs()).sum();
    let neg: f64 = result.neg_plines.iter().map(|p| p.pline.area().abs()).sum();
    pos - neg
}

/// The same shape selected twice can't be both boundary and island: refuse.
fn refuse_duplicates(shapes: &[(usize, Polyline<f64>, Ring)]) -> Result<(), ToolpathError> {
    for (i, (id_a, a, _)) in shapes.iter().enumerate() {
        for (id_b, b, _) in &shapes[i + 1..] {
            let (Some(ea), Some(eb)) = (a.extents(), b.extents()) else {
                continue;
            };
            if ea.max_x < eb.min_x
                || eb.max_x < ea.min_x
                || ea.max_y < eb.min_y
                || eb.max_y < ea.min_y
            {
                continue;
            }
            let common = guarded("Comparing shapes", || a.boolean(b, BooleanOp::And))?;
            let symmetric = a.area().abs() + b.area().abs() - 2.0 * boolean_area(&common);
            let slack = CHAIN_TOL_MM * a.path_length().max(b.path_length());
            if symmetric <= slack {
                return Err(ToolpathError::Selector(format!(
                    "contours #{id_a} and #{id_b} are the same shape; select only one of them"
                )));
            }
        }
    }
    Ok(())
}

/// Islands that cross or touch each other become one island.
fn merge_islands(
    islands: Vec<Polyline<f64>>,
    ids: &[usize],
) -> Result<Vec<Polyline<f64>>, ToolpathError> {
    let mut merged: Vec<Polyline<f64>> = Vec::new();
    for island in islands {
        let mut current = island;
        while let Some(j) = merged
            .iter()
            .position(|other| !matches!(current.contains(other), PlineContainsResult::Disjoint))
        {
            let other = merged.remove(j);
            let mut union = guarded("Merging islands", || current.boolean(&other, BooleanOp::Or))?;
            if union.pos_plines.len() != 1 || !union.neg_plines.is_empty() {
                return Err(geometry(format!(
                    "contours {}: overlapping islands enclose an area between them, which \
                     pockets don't support yet",
                    ids_text(ids)
                )));
            }
            current = ccw(union.pos_plines.swap_remove(0).pline);
        }
        merged.push(current);
    }
    Ok(merged)
}

/// The boundary minus islands. An island strictly inside becomes a hole; one touching or
/// crossing the boundary is cut away, which can split the area.
fn subtract_islands(
    boundary: Polyline<f64>,
    islands: Vec<Polyline<f64>>,
) -> Result<Vec<Area>, ToolpathError> {
    let mut areas = vec![Area {
        outer: boundary,
        holes: Vec::new(),
    }];
    for island in islands {
        let mut next = Vec::new();
        for mut area in areas {
            match area.outer.contains(&island) {
                PlineContainsResult::Pline2InsidePline1 => {
                    area.holes.push(cw(island.clone()));
                    next.push(area);
                }
                PlineContainsResult::Intersected | PlineContainsResult::Pline1InsidePline2 => {
                    let cut = guarded("Subtracting an island", || {
                        area.outer.boolean(&island, BooleanOp::Not)
                    })?;
                    let mut pieces: Vec<Area> = cut
                        .pos_plines
                        .into_iter()
                        .map(|p| Area {
                            outer: ccw(p.pline),
                            holes: Vec::new(),
                        })
                        .collect();
                    let holes = area
                        .holes
                        .into_iter()
                        .chain(cut.neg_plines.into_iter().map(|p| cw(p.pline)));
                    for hole in holes {
                        let inside = first_point(&hole);
                        if let Some(piece) = pieces.iter_mut().find(|p| encloses(&p.outer, inside))
                        {
                            piece.holes.push(hole);
                        }
                    }
                    next.extend(pieces);
                }
                PlineContainsResult::Disjoint | PlineContainsResult::InvalidInput => {
                    next.push(area);
                }
            }
        }
        areas = next;
    }
    Ok(areas
        .into_iter()
        .filter(|a| a.outer.area().abs() > CHAIN_TOL_MM * CHAIN_TOL_MM)
        .collect())
}

/// Offset levels as a tree: every level k+1 area lies inside one level k area.
struct Node {
    area: Area,
    children: Vec<usize>,
}

fn ring_tree(
    region: &Area,
    radius: f64,
    stepover: f64,
    ids: &[usize],
) -> Result<(Vec<Node>, Vec<usize>), ToolpathError> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut roots = Vec::new();
    for area in region.shrunk(radius)? {
        roots.push(nodes.len());
        nodes.push(Node {
            area,
            children: Vec::new(),
        });
    }
    if roots.is_empty() {
        return Err(geometry(format!(
            "contours {}: tool (diameter {:.3} mm) does not fit in the pocket",
            ids_text(ids),
            2.0 * radius
        )));
    }
    let mut level = roots.clone();
    for k in 1..=MAX_RING_LEVELS {
        let areas = region.shrunk(radius + k as f64 * stepover)?;
        if areas.is_empty() {
            return Ok((nodes, roots));
        }
        let mut next_level = Vec::new();
        for area in areas {
            let inside = first_point(&area.outer);
            let Some(&parent) = level.iter().find(|&&n| nodes[n].area.contains(inside)) else {
                return Err(geometry(format!(
                    "contours {}: pocket offsets are not nested",
                    ids_text(ids)
                )));
            };
            let id = nodes.len();
            nodes.push(Node {
                area,
                children: Vec::new(),
            });
            nodes[parent].children.push(id);
            next_level.push(id);
        }
        level = next_level;
    }
    Err(geometry(format!(
        "contours {}: more than {MAX_RING_LEVELS} pocket rings",
        ids_text(ids)
    )))
}

/// Order for the travel direction and start at the vertex nearest `near` (ties: y, x).
fn orient_from(ring: &[Point], clockwise: bool, near: Point) -> Vec<Point> {
    let mut ordered = ring.to_vec();
    if (signed_area(ring) < 0.0) != clockwise {
        ordered.reverse();
    }
    let key = |p: Point| (distance(near, p), p.1, p.0);
    let start = (0..ordered.len())
        .min_by(|&i, &j| {
            let (a, b) = (key(ordered[i]), key(ordered[j]));
            a.0.total_cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then(a.2.total_cmp(&b.2))
        })
        .unwrap_or(0);
    ordered.rotate_left(start);
    ordered
}

/// Children before parents (inside out); siblings nearest first. An area's island rings are
/// cut before its outer ring, so the wall is the last ring at each depth.
fn cut_order(
    nodes: &[Node],
    roots: &[usize],
    mut position: Point,
    direction: Direction,
    spindle: Spindle,
) -> Vec<Vec<Point>> {
    let wall_cw = cut_clockwise(Side::Inside, direction, spindle); // material outside the ring
    let island_cw = cut_clockwise(Side::Outside, direction, spindle); // material inside it
    let mut rings: Vec<Vec<Point>> = Vec::new();
    let mut stack: Vec<(Vec<usize>, Option<usize>)> = vec![(roots.to_vec(), None)];
    while let Some((siblings, waiting)) = stack.last_mut() {
        if !siblings.is_empty() {
            let here = position;
            let key = |n: usize| {
                let (y, x) = nodes[n].area.sort_key();
                (nodes[n].area.distance(here), y, x)
            };
            let index = (0..siblings.len())
                .min_by(|&i, &j| {
                    let (a, b) = (key(siblings[i]), key(siblings[j]));
                    a.0.total_cmp(&b.0)
                        .then(a.1.total_cmp(&b.1))
                        .then(a.2.total_cmp(&b.2))
                })
                .unwrap_or(0);
            let node = siblings.remove(index);
            stack.push((nodes[node].children.clone(), Some(node)));
            continue;
        }
        let waiting = *waiting;
        stack.pop();
        let Some(node) = waiting else { continue };
        let area = &nodes[node].area;
        let mut islands: Vec<Vec<Point>> = area.holes.iter().map(flatten).collect();
        while !islands.is_empty() {
            let here = position;
            let nearest = (0..islands.len())
                .min_by(|&i, &j| {
                    let d = |r: &[Point]| {
                        r.iter()
                            .map(|&p| distance(here, p))
                            .fold(f64::INFINITY, f64::min)
                    };
                    d(&islands[i]).total_cmp(&d(&islands[j]))
                })
                .unwrap_or(0);
            let island = islands.remove(nearest);
            let ring = orient_from(&island, island_cw, position);
            position = ring[0];
            rings.push(ring);
        }
        let wall = orient_from(&flatten(&area.outer), wall_cw, position);
        position = wall[0];
        rings.push(wall);
    }
    rings
}

fn min_clearance(radius: f64) -> f64 {
    radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM
}

/// The segment keeps at least `clearance` from every primitive in the index.
fn clear_of(index: &PrimIndex, a: Point, b: Point, clearance: f64) -> bool {
    index
        .segment_distance_within(a, b, clearance)
        .is_none_or(|d| d >= clearance)
}

/// Staying down is safe if the tool can't reach a wall or island and the link is short.
fn link_is_safe(region: &Area, walls: &PrimIndex, a: Point, b: Point, radius: f64) -> bool {
    let length = distance(a, b);
    if length > 2.0 * radius {
        return false;
    }
    if length < CHAIN_TOL_MM {
        return true;
    }
    region.contains(a) && clear_of(walls, a, b, min_clearance(radius))
}

/// Every cutting move as a segment: ring chords (closed) and links fed at depth.
fn cutting_segments(rings: &[Vec<Point>], links: &[bool]) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();
    for ring in rings {
        let n = ring.len();
        match n {
            0 => {}
            1 => segments.push((ring[0], ring[0])),
            2 => segments.push((ring[0], ring[1])),
            _ => segments.extend((0..n).map(|i| (ring[i], ring[(i + 1) % n]))),
        }
    }
    for (i, &stays_down) in links.iter().enumerate() {
        let (a, b) = (rings[i][0], rings[i + 1][0]);
        if stays_down && distance(a, b) >= CHAIN_TOL_MM {
            segments.push((a, b));
        }
    }
    segments
}

/// Post-condition: no cutting move comes closer than r (minus tolerance) to a wall or
/// island, and everything is inside the region. Never trust the offsets blindly.
fn verify(
    region: &Area,
    walls: &PrimIndex,
    rings: &[Vec<Point>],
    links: &[bool],
    radius: f64,
    ids: &[usize],
) -> Result<(), ToolpathError> {
    let clearance = min_clearance(radius);
    let inside = rings
        .iter()
        .all(|r| r.first().is_some_and(|&p| region.contains(p)));
    let clear = cutting_segments(rings, links)
        .iter()
        .all(|&(a, b)| clear_of(walls, a, b, clearance));
    if inside && clear {
        Ok(())
    } else {
        Err(geometry(format!(
            "contours {}: pocket toolpath failed verification (too close to a wall or \
             island); the drawing may be degenerate",
            ids_text(ids)
        )))
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
/// The Python reference computed the leftover `reachable − ∪ buffer(path, r)` and reports the pieces that
/// survive `buffer(−REST_TOL_MM)`, where reachable is everything within r of the level-0
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
fn uncut_material(
    region: &Area,
    reachable: &[Area],
    rings: &[Vec<Point>],
    links: &[bool],
    radius: f64,
    ids: &[usize],
) -> (Option<String>, Vec<LeftPiece>) {
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
        return (None, Vec::new());
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
    while let Some(cell) = todo.pop() {
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
        return (None, Vec::new());
    }
    let mut missed: Vec<LeftPiece> = spots
        .iter()
        .map(|spot| LeftPiece {
            kind: LeftKind::Missed,
            rings: spot.iter().map(|&i| square(cells[i])).collect(),
        })
        .collect();
    sort_pieces(&mut missed);
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
    let largest = spots.iter().max_by(|a, b| {
        let (ba, bb) = (bounds(a), bounds(b));
        area(a)
            .total_cmp(&area(b))
            .then(bb.1.total_cmp(&ba.1))
            .then(bb.0.total_cmp(&ba.0))
    });
    let Some(largest) = largest else {
        return (None, missed);
    };
    let (x0, y0, x1, y1) = bounds(largest);
    // A point inside the piece: the centre of its biggest cell.
    let inside = largest
        .iter()
        .map(|&i| cells[i])
        .max_by(|a, b| a.half.total_cmp(&b.half));
    let Some(inside) = inside else {
        return (None, missed);
    };
    let count = spots.len();
    let places = format!("{count} place{}", if count > 1 { "s" } else { "" });
    let note = format!(
        "contours {}: uncut material in {places}, largest about {:.2} x {:.2} mm near \
         ({:.1}, {:.1}); reduce the stepover to clear it",
        ids_text(ids),
        x1 - x0,
        y1 - y0,
        inside.centre.0,
        inside.centre.1
    );
    (Some(note), missed)
}

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

/// Rings, cut order and links for every piece. `stepover` is in mm.
pub fn plan_pocket(
    contours: &[&Contour],
    tool_radius: f64,
    stepover: f64,
    direction: Direction,
    spindle: Spindle,
    keep_tool_down: bool,
    start: Point,
) -> Result<PocketPlan, ToolpathError> {
    if stepover <= 0.0 || stepover.is_nan() {
        return Err(geometry(format!(
            "stepover must be positive, got {stepover}"
        )));
    }
    let mut remaining = build_region(contours)?;
    if remaining.is_empty() {
        let mut ids: Vec<usize> = contours.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        return Err(geometry(format!(
            "Nothing to clear: the selected islands cover the selected boundaries \
             (contours {})",
            ids_text(&ids)
        )));
    }
    let mut pieces = Vec::new();
    let mut notes = Vec::new();
    let mut left = Vec::new();
    // Every region with where the tool centre can go in it, for the material left.
    let mut reachable: Vec<(Area, Vec<Area>)> = Vec::new();
    let mut position = start;
    while !remaining.is_empty() {
        let here = position;
        let key = |a: &PocketArea| {
            let (y, x) = a.area.sort_key();
            (a.area.distance(here), y, x)
        };
        let index = (0..remaining.len())
            .min_by(|&i, &j| {
                let (a, b) = (key(&remaining[i]), key(&remaining[j]));
                a.0.total_cmp(&b.0)
                    .then(a.1.total_cmp(&b.1))
                    .then(a.2.total_cmp(&b.2))
                    .then(remaining[i].contour_ids.cmp(&remaining[j].contour_ids))
            })
            .unwrap_or(0);
        let PocketArea {
            area: region,
            contour_ids: ids,
        } = remaining.remove(index);
        let (nodes, roots) = ring_tree(&region, tool_radius, stepover, &ids)?;
        if roots.len() > 1 {
            notes.push(format!(
                "contours {}: the tool does not fit through a narrow part; cleared as {} \
                 areas and material remains in between",
                ids_text(&ids),
                roots.len()
            ));
        }
        let rings = cut_order(&nodes, &roots, position, direction, spindle);
        let walls = PrimIndex::new(region.boundary(), tool_radius.max(1.0));
        let links: Vec<bool> = rings
            .windows(2)
            .map(|w| keep_tool_down && link_is_safe(&region, &walls, w[0][0], w[1][0], tool_radius))
            .collect();
        verify(&region, &walls, &rings, &links, tool_radius, &ids)?;
        let level0: Vec<Area> = roots.iter().map(|&r| nodes[r].area.clone()).collect();
        let (uncut, missed) = uncut_material(&region, &level0, &rings, &links, tool_radius, &ids);
        notes.extend(uncut);
        left.extend(missed);
        reachable.push((region, level0));
        position = rings[0][0]; // every depth starts again at the first ring
        pieces.push(PocketPiece {
            rings,
            feed_links: links,
            contour_ids: ids,
        });
    }
    // Display only: any failure, even a panic in the geometry library, becomes a note.
    let unreachable = guarded("Material left", || {
        unreachable_in_pockets(&reachable, tool_radius)
    })
    .and_then(|result| result);
    match unreachable {
        Ok(pieces) => left.extend(pieces),
        Err(_) => {
            let mut ids: Vec<usize> = contours.iter().map(|c| c.id).collect();
            ids.sort_unstable();
            notes.push(format!(
                "contours {}: could not compute the material left for display",
                ids_text(&ids)
            ));
        }
    }
    Ok(PocketPlan {
        pieces,
        notes,
        left,
    })
}

/// Material no pocket of the operation reaches. Crossing pockets clear each other's
/// corners, so each region is checked against the reach of all of them.
fn unreachable_in_pockets(
    regions: &[(Area, Vec<Area>)],
    radius: f64,
) -> Result<Vec<LeftPiece>, ToolpathError> {
    let mut reached = Vec::new();
    for (_, level0) in regions {
        reached.extend(reach(level0, radius)?);
    }
    let mut pieces = Vec::new();
    for (region, _) in regions {
        pieces.extend(unreachable_inside(region, &reached)?);
    }
    Ok(pieces)
}

/// All depth passes of one piece. Each pass starts at the centre and ends retracted.
pub fn pocket_moves(
    piece: &PocketPiece,
    pass_z: &[f64],
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
) -> Vec<Move> {
    let mut moves = Vec::new();
    let Some(first) = piece.rings.first().and_then(|r| r.first().copied()) else {
        return moves;
    };
    for &z in pass_z {
        moves.extend([
            Move::rapid_z(safe_z),
            Move::rapid_xy(first),
            Move::feed_z(z, plunge_feed),
        ]);
        for (index, ring) in piece.rings.iter().enumerate() {
            let start = ring[0];
            if index > 0 {
                if piece.feed_links[index - 1] {
                    moves.push(Move::feed_xy(start, feed));
                } else {
                    moves.extend([
                        Move::rapid_z(safe_z),
                        Move::rapid_xy(start),
                        Move::feed_z(z, plunge_feed),
                    ]);
                }
            }
            moves.extend(ring[1..].iter().map(|&p| Move::feed_xy(p, feed)));
            moves.push(Move::feed_xy(start, feed));
        }
        moves.push(Move::rapid_z(safe_z));
    }
    moves
}
