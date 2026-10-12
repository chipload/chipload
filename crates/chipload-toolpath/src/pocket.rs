// SPDX-License-Identifier: GPL-3.0-or-later
//! Pocket clearing: contour-parallel rings, cut from the inside out. See
//! docs/specs/pocket.md.
//!
//! Offsets use cavalier_contours shapes (outer loop plus islands, true arcs). Every result is
//! verified afterwards, and uncut material is found with exact distances, not by buffering
//! the paths.

use std::f64::consts::{SQRT_2, TAU};
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
use chipload_job::{Direction, Entry, PocketStrategy, Side, Spindle};

use crate::ToolpathError;
use crate::adaptive::Grid;
use crate::arcs::{ArcTable, Circle, arc_midpoint};
use crate::fronts;
use crate::geom::PrimIndex;
use crate::helix::{Helix, helix_moves};
use crate::lead::{Leads, MIN_LEAD_DIAMETERS, fitted_leads, roomiest_start};
use crate::left::{LeftKind, LeftPiece, cleared, reach, sort_pieces, unreachable_inside};
use crate::model::Move;
use crate::offset::{contour_pline, dedupe_ring, flatten_tagged, pline_segments, point_distance};
use crate::profile::cut_clockwise;
use crate::retry::{Failure, Nudge, nudged};
use crate::spiral;
use crate::start::{assign, point_text, restart};

/// Guard against runaway loops on absurd input; a 2 m pocket at 0.1 mm stepover is 10 000.
pub const MAX_RING_LEVELS: usize = 20_000;

/// One connected area to clear, with its paths in cut order.
#[derive(Debug, Clone, PartialEq)]
pub struct PocketPiece {
    /// Each oriented and starting at ring[0]; a closed ring also ends there.
    pub rings: Vec<Vec<Point>>,
    /// open[i]: rings[i] is an open path (adaptive passes) ending at its last point, not
    /// closed back to its start.
    pub open: Vec<bool>,
    /// feed_links[i]: stay at depth from the end of rings[i] into rings[i + 1].
    pub feed_links: Vec<bool>,
    /// Selected contours that make up this piece, for messages.
    pub contour_ids: Vec<usize>,
    /// walls[i]: rings[i] is a wall ring (the last ring along a wall or island at each
    /// depth), whose start the user can move.
    pub walls: Vec<bool>,
    /// chosen[i]: the index in the operation's `starts` of the point that set rings[i]'s
    /// start (docs/specs/helix-and-leads.md part A).
    pub chosen: Vec<Option<usize>>,
    /// leads[i]: rings[i] is a wall ring with a lead-in before it and a lead-out after it,
    /// as one open path; the ring itself is rings[i][range] (part B).
    pub leads: Vec<Option<std::ops::Range<usize>>>,
    /// The helix each depth pass goes down round, then feeds at depth to rings[0][0]
    /// (part C); None: the first path's own entry.
    pub helix: Option<Helix>,
}

/// Where the tool is after cutting a path: back at its start if closed, at its end if open.
pub fn path_end(ring: &[Point], open: bool) -> Option<Point> {
    if open { ring.last() } else { ring.first() }.copied()
}

#[derive(Debug, Clone, PartialEq)]
pub struct PocketPlan {
    pub pieces: Vec<PocketPiece>,
    /// Warnings: narrow parts, uncut material.
    pub notes: Vec<String>,
    /// Material left, for display (unreachable corners and slots, missed spots).
    pub left: Vec<LeftPiece>,
    /// Which ring chords lie on arcs (docs/specs/arc-output.md).
    pub arcs: ArcTable,
    /// Adaptive clearing: every step's cut as a share of the target (docs/specs/adaptive.md);
    /// empty for the other strategies.
    pub engagement: Vec<f64>,
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

impl Area {
    /// The area grown by `d` (≥ 0): the outline offset outwards and each hole shrunk, one
    /// loop at a time. Growing can't make two loops meet (the outline moves away from the
    /// holes, and shrinking holes move apart), so nothing is lost by not offsetting them as
    /// one shape, and cavalier's shape offset drops a hole at some distances: two islands
    /// whose offsets merge lost their hole when grown by r + 0.005 to r + 0.05 mm but not by
    /// r or r + 0.1 (found by the material-left property test).
    pub(crate) fn grown(&self, d: f64) -> Result<Vec<Area>, ToolpathError> {
        if d <= 0.0 {
            return self.shrunk(-d);
        }
        let keep = |p: &Polyline<f64>| p.area().abs() > CHAIN_TOL_MM * CHAIN_TOL_MM;
        let mut outers: Vec<Polyline<f64>> = Vec::new();
        let mut holes: Vec<Polyline<f64>> = Vec::new();
        // Positive offsets go inwards for counter-clockwise loops.
        let outline = ccw(self.outer.clone());
        for loop_ in closed_loops(guarded("Pocket offset", || outline.parallel_offset(-d))?)? {
            if !keep(&loop_) {
                continue;
            }
            // A bay that closes up while growing leaves a clockwise void.
            if loop_.area() > 0.0 {
                outers.push(loop_);
            } else {
                holes.push(loop_);
            }
        }
        for hole in &self.holes {
            let inside = ccw(hole.clone());
            for loop_ in closed_loops(guarded("Pocket offset", || inside.parallel_offset(d))?)? {
                if keep(&loop_) {
                    holes.push(cw(loop_));
                }
            }
        }
        let mut areas: Vec<Area> = outers
            .into_iter()
            .map(|outer| Area {
                outer,
                holes: Vec::new(),
            })
            .collect();
        for hole in holes {
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

/// The loops of a closed loop's offset, all closed. cavalier_contours can return a loop
/// that closes up while offsetting (a bay pinched off as the outline grows) as an open
/// polyline whose ends lie a hair apart; dropping it lost a hole of the tool's reach, and
/// material left went unshown (material-left property test, 2026-10-12). Such a loop is
/// closed; one whose ends lie farther apart than `CHAIN_TOL_MM` is an error, never dropped.
fn closed_loops(loops: Vec<Polyline<f64>>) -> Result<Vec<Polyline<f64>>, ToolpathError> {
    let mut closed = Vec::with_capacity(loops.len());
    for mut loop_ in loops {
        if !loop_.is_closed() {
            let n = loop_.vertex_count();
            let ends = match (loop_.get(0), loop_.get(n.saturating_sub(1))) {
                (Some(a), Some(b)) if n > 2 => ((a.x, a.y), (b.x, b.y)),
                _ => continue,
            };
            if distance(ends.0, ends.1) > CHAIN_TOL_MM {
                return Err(geometry(
                    "pocket offset returned an open loop where a closed one was expected".into(),
                ));
            }
            loop_.set_is_closed(true);
        }
        closed.push(loop_);
    }
    Ok(closed)
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

/// `radius` is the tool-centre offset from the walls (tool radius minus clearance);
/// `tool_radius` is only for messages. `nudge` moves the offsets for a retry
/// (docs/specs/offset-retry.md).
fn ring_tree(
    region: &Area,
    radius: f64,
    tool_radius: f64,
    stepover: f64,
    ids: &[usize],
    nudge: Nudge,
) -> Result<(Vec<Node>, Vec<usize>), Failure> {
    let numerical = |wall: bool| move |error| Failure::Numerical { error, wall };
    let wall_offset = radius + nudge.wall;
    let stepover = stepover - nudge.step;
    let mut nodes: Vec<Node> = Vec::new();
    let mut roots = Vec::new();
    for area in region.shrunk(wall_offset).map_err(numerical(true))? {
        roots.push(nodes.len());
        nodes.push(Node {
            area,
            children: Vec::new(),
        });
    }
    if roots.is_empty() {
        return Err(Failure::Final(geometry(format!(
            "contours {}: tool (diameter {:.3} mm{}) does not fit in the pocket",
            ids_text(ids),
            2.0 * tool_radius,
            if radius == tool_radius {
                String::new()
            } else {
                format!(", clearance {:.3} mm", tool_radius - radius)
            }
        ))));
    }
    let mut level = roots.clone();
    for k in 1..=MAX_RING_LEVELS {
        let areas = region
            .shrunk(wall_offset + k as f64 * stepover)
            .map_err(numerical(false))?;
        if areas.is_empty() {
            return Ok((nodes, roots));
        }
        let mut next_level = Vec::new();
        for area in areas {
            let inside = first_point(&area.outer);
            let Some(&parent) = level.iter().find(|&&n| nodes[n].area.contains(inside)) else {
                return Err(Failure::Numerical {
                    error: geometry(format!(
                        "contours {}: pocket offsets are not nested",
                        ids_text(ids)
                    )),
                    wall: false,
                });
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
    Err(Failure::Final(geometry(format!(
        "contours {}: more than {MAX_RING_LEVELS} pocket rings",
        ids_text(ids)
    ))))
}

/// Rounds of the uncut check, each halving the radii of the rings near material left.
const ROUNDING_ROUNDS: usize = 12;

/// The smallest scale of a ring's radius before it is left sharp: an eighth.
const MIN_ROUNDING_SCALE: f64 = 0.125;

/// Radii tried for one ring, each half the one before.
const ROUNDING_TRIES: usize = 6;

/// Round the corners of every ring but the wall rings (docs/specs/pocket-round-corners.md).
/// A ring `k` levels in gets the radius `scale · min(k · stepover, max_radius)` (its own
/// scale, 1 unless rounding there would leave material): its area is
/// shrunk by that radius and grown back, which turns each corner sharper than the radius into
/// a tangent arc and leaves the rest where it was. Inner rings come first (they have the
/// higher indices) and cap the ring outside them at their radius plus a stepover, so every
/// rounded ring still lies inside the rounded ring outside it. Where rounding would split a
/// ring, drop part of it or fail, the radius is halved (a narrow inner ring still gets a
/// small one), up to `ROUNDING_TRIES` radii; then the ring stays sharp.
fn round_ring_corners(
    nodes: &mut [Node],
    roots: &[usize],
    stepover: f64,
    max_radius: f64,
    scales: &[f64],
) {
    let mut depth = vec![0usize; nodes.len()];
    let mut stack: Vec<usize> = roots.to_vec();
    while let Some(i) = stack.pop() {
        for &child in &nodes[i].children {
            depth[child] = depth[i] + 1;
            stack.push(child);
        }
    }
    let mut radius = vec![0.0f64; nodes.len()];
    for i in (0..nodes.len()).rev() {
        if depth[i] == 0 {
            continue;
        }
        let nominal = scales[i] * (depth[i] as f64 * stepover).min(max_radius);
        let cap = nodes[i]
            .children
            .iter()
            .map(|&c| radius[c] + stepover)
            .fold(f64::INFINITY, f64::min);
        let mut rho = nominal.min(cap);
        for _ in 0..ROUNDING_TRIES {
            if rho <= ARC_APPROX_TOL_MM {
                break;
            }
            if let Some(area) = opened(&nodes[i].area, rho) {
                nodes[i].area = area;
                radius[i] = rho;
                break;
            }
            rho /= 2.0;
        }
    }
}

/// `area` shrunk by `rho` and grown back: one area with as many holes as before, or None
/// (it would split, lose a part or a hole, or an offset failed).
fn opened(area: &Area, rho: f64) -> Option<Area> {
    let shrunk = area.shrunk(rho).ok()?;
    let [core] = shrunk.as_slice() else {
        return None;
    };
    let mut grown = core.grown(rho).ok()?;
    if grown.len() != 1 || grown[0].holes.len() != area.holes.len() {
        return None;
    }
    grown.pop()
}

/// One piece's paths from one attempt (docs/specs/offset-retry.md), with the arcs,
/// engagement and notes that attempt recorded: only the used attempt's are kept.
#[derive(Default)]
struct Attempt {
    rings: Vec<Vec<Point>>,
    level0: Vec<Area>,
    open: Vec<bool>,
    walls: Vec<bool>,
    arcs: ArcTable,
    engagement: Vec<f64>,
    notes: Vec<String>,
}

/// What a piece is cleared with, and where the tool comes from.
struct Clearing {
    strategy: PocketStrategy,
    /// Rings strategy: round the corners of the inner rings.
    round_corners: bool,
    offset: f64,
    tool_radius: f64,
    stepover: f64,
    position: Point,
    direction: Direction,
    spindle: Spindle,
}

/// The paths of one piece, checked as soon as they are built and retried with nudged
/// offsets after a numerical failure (docs/specs/offset-retry.md).
fn cleared_piece(region: &Area, ids: &[usize], c: &Clearing) -> Result<Attempt, ToolpathError> {
    let walls = PrimIndex::new(region.boundary(), c.tool_radius.max(1.0));
    let rings_only = matches!(c.strategy, PocketStrategy::Rings {});
    nudged(|nudge| {
        let mut a = attempt_piece(region, ids, c, nudge)?;
        if let Some(k) = first_unclear(region, &walls, &a.rings, &a.open, c.offset, &a.arcs) {
            return Err(Failure::Numerical {
                error: verification_failed(ids),
                // Adaptive and trochoidal paths have no stepover between rings to nudge.
                wall: a.walls.get(k).copied().unwrap_or(true) || !rings_only,
            });
        }
        if a.open.is_empty() {
            a.open = vec![false; a.rings.len()];
        }
        Ok(a)
    })
}

/// One attempt at a piece's paths, with the wall offset and stepover nudged.
fn attempt_piece(
    region: &Area,
    ids: &[usize],
    c: &Clearing,
    nudge: Nudge,
) -> Result<Attempt, Failure> {
    let mut a = Attempt::default();
    // Adaptive and trochoidal errors include the library's, so every one may be numerical;
    // one a nudge can't change just fails again and the first error is returned.
    let numerical = |error| Failure::Numerical { error, wall: true };
    let offset = c.offset + nudge.wall;
    match c.strategy {
        PocketStrategy::Rings {} => {
            let (mut nodes, roots) =
                ring_tree(region, c.offset, c.tool_radius, c.stepover, ids, nudge)?;
            if c.round_corners {
                let stepover = c.stepover - nudge.step;
                rounded_rings(region, &mut nodes, &roots, stepover, ids, c, &mut a.notes);
            }
            (a.rings, a.walls) = cut_order(
                &nodes,
                &roots,
                c.position,
                c.direction,
                c.spindle,
                &mut a.arcs,
            );
            a.level0 = roots.iter().map(|&r| nodes[r].area.clone()).collect();
        }
        PocketStrategy::Adaptive {} => {
            (a.rings, a.level0, a.open, a.walls) = adaptive_rings(
                region,
                offset,
                c.tool_radius,
                c.stepover,
                ids,
                c.position,
                c.direction,
                c.spindle,
                &mut a.arcs,
                &mut a.engagement,
            )
            .map_err(numerical)?;
        }
        PocketStrategy::Trochoidal { loop_radius, step } => {
            let tool_radius = c.tool_radius;
            // How far either side of its backbone a row of loops surely cuts: a point
            // midway between two loop centres is half a step further from them, and
            // chords lie up to the arc tolerance inside their arcs.
            let reach = ((loop_radius + tool_radius).powi(2) - (step / 2.0).powi(2)).sqrt()
                - ARC_APPROX_TOL_MM;
            // Backbones no further apart than that, so every point between two levels
            // (even where a lobe of the pocket ends) is within reach of a row of loops.
            let trochoid = Trochoid {
                offset,
                tool_radius,
                loop_radius,
                step,
                band: (2.0 * loop_radius + c.stepover).min(reach) - nudge.step,
            };
            (a.rings, a.level0, a.walls) = trochoidal_rings(
                region,
                &trochoid,
                ids,
                c.position,
                c.direction,
                c.spindle,
                &mut a.arcs,
                &mut a.notes,
            )
            .map_err(numerical)?;
        }
    }
    Ok(a)
}

/// Round the inner rings' corners (docs/specs/pocket-round-corners.md §3). The
/// uncut-material check (without links: they only cut more) says where the rounding would
/// leave material; the rings near there get half their radius, down to an eighth, then
/// none, and the check runs again. If that doesn't clear it, every ring stays sharp, with a
/// note when the rounding was the reason.
fn rounded_rings(
    region: &Area,
    nodes: &mut [Node],
    roots: &[usize],
    stepover: f64,
    ids: &[usize],
    c: &Clearing,
    notes: &mut Vec<String>,
) {
    let level0: Vec<Area> = roots.iter().map(|&r| nodes[r].area.clone()).collect();
    let missed = |nodes: &[Node]| {
        let (rings, _) = cut_order(
            nodes,
            roots,
            c.position,
            c.direction,
            c.spindle,
            &mut ArcTable::default(),
        );
        let open = vec![false; rings.len()];
        let (note, missed) =
            uncut_material(region, &level0, &rings, &open, &[], c.tool_radius, ids);
        note.map(|_| missed)
    };
    let sharp: Vec<Area> = nodes.iter().map(|n| n.area.clone()).collect();
    // Rings this far from material left are the ones that could have cut it.
    let near = c.tool_radius + stepover;
    let mut scales = vec![1.0; nodes.len()];
    for _ in 0..ROUNDING_ROUNDS {
        round_ring_corners(nodes, roots, stepover, 2.0 * c.tool_radius, &scales);
        let Some(left) = missed(nodes) else {
            return;
        };
        let mut changed = false;
        let points: Vec<Point> = left
            .iter()
            .flat_map(|piece| piece.rings.iter().flatten().copied())
            .collect();
        for (i, node) in nodes.iter().enumerate() {
            let edges = PrimIndex::new(node.area.boundary(), near);
            let close = points.iter().any(|&p| edges.any_within(p, near));
            if close && scales[i] > 0.0 {
                scales[i] = if scales[i] > MIN_ROUNDING_SCALE {
                    scales[i] / 2.0
                } else {
                    0.0
                };
                changed = true;
            }
        }
        for (node, area) in nodes.iter_mut().zip(&sharp) {
            node.area = area.clone();
        }
        if !changed {
            break;
        }
    }
    for (node, area) in nodes.iter_mut().zip(&sharp) {
        node.area = area.clone();
    }
    if missed(nodes).is_none() {
        notes.push(format!(
            "contours {}: ring corners not rounded; rounding would leave material",
            ids_text(ids)
        ));
    }
}

/// The first path that comes closer than r (minus tolerance) to a wall or island or starts
/// outside the region: [`verify`] for the paths alone, before links and leads. `open` may
/// be empty: all closed.
fn first_unclear(
    region: &Area,
    walls: &PrimIndex,
    rings: &[Vec<Point>],
    open: &[bool],
    radius: f64,
    arcs: &ArcTable,
) -> Option<usize> {
    let clearance = min_clearance(radius);
    rings.iter().enumerate().position(|(k, ring)| {
        let is_open = open.get(k).copied().unwrap_or(false);
        let inside = ring.first().is_some_and(|&p| region.contains(p));
        let clear = cutting_segments(std::slice::from_ref(ring), &[is_open], &[])
            .iter()
            .all(|&(a, b)| segment_is_clear(walls, a, b, clearance, arcs));
        !(inside && clear)
    })
}

/// The cutting move from `a` to `b` keeps `clearance` from every wall, also where it bulges
/// most if it is cut as an arc.
fn segment_is_clear(
    walls: &PrimIndex,
    a: Point,
    b: Point,
    clearance: f64,
    arcs: &ArcTable,
) -> bool {
    clear_of(walls, a, b, clearance)
        && arcs.get(a, b).is_none_or(|circle| {
            let top = arc_midpoint(a, b, circle);
            clear_of(walls, top, top, clearance)
        })
}

fn verification_failed(ids: &[usize]) -> ToolpathError {
    geometry(format!(
        "contours {}: pocket toolpath failed verification (too close to a wall or island); \
         the drawing may be degenerate",
        ids_text(ids)
    ))
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
    arcs: &mut ArcTable,
) -> (Vec<Vec<Point>>, Vec<bool>) {
    let wall_cw = cut_clockwise(Side::Inside, direction, spindle); // material outside the ring
    let island_cw = cut_clockwise(Side::Outside, direction, spindle); // material inside it
    let mut rings: Vec<Vec<Point>> = Vec::new();
    let mut walls: Vec<bool> = Vec::new();
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
        let wall = roots.contains(&node);
        let mut islands: Vec<Vec<Point>> = area
            .holes
            .iter()
            .map(|hole| flatten_tagged(hole, arcs))
            .collect();
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
            walls.push(wall);
        }
        let outer = orient_from(&flatten_tagged(&area.outer, arcs), wall_cw, position);
        position = outer[0];
        rings.push(outer);
        walls.push(wall);
    }
    (rings, walls)
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
/// `offset`: how far the tool centre keeps from the walls; `radius`: the tool's.
fn link_is_safe(
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

/// Every cutting move as a segment: ring chords (closed) and links fed at depth.
fn cutting_segments(rings: &[Vec<Point>], open: &[bool], links: &[bool]) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();
    for (k, ring) in rings.iter().enumerate() {
        let n = ring.len();
        let is_open = open.get(k).copied().unwrap_or(false);
        match n {
            0 => {}
            1 => segments.push((ring[0], ring[0])),
            2 => segments.push((ring[0], ring[1])),
            _ if is_open => segments.extend(ring.windows(2).map(|w| (w[0], w[1]))),
            _ => segments.extend((0..n).map(|i| (ring[i], ring[(i + 1) % n]))),
        }
    }
    for (i, &stays_down) in links.iter().enumerate() {
        let is_open = open.get(i).copied().unwrap_or(false);
        let (Some(a), b) = (path_end(&rings[i], is_open), rings[i + 1][0]) else {
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
#[allow(clippy::too_many_arguments)]
fn verify(
    region: &Area,
    walls: &PrimIndex,
    rings: &[Vec<Point>],
    open: &[bool],
    links: &[bool],
    radius: f64,
    ids: &[usize],
    arcs: &ArcTable,
) -> Result<(), ToolpathError> {
    let clearance = min_clearance(radius);
    let inside = rings
        .iter()
        .all(|r| r.first().is_some_and(|&p| region.contains(p)));
    // Chords on arcs are cut as arcs when arcs are written: check where they bulge most too.
    let clear = cutting_segments(rings, open, links)
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
fn uncut_material(
    region: &Area,
    reachable: &[Area],
    rings: &[Vec<Point>],
    open: &[bool],
    links: &[bool],
    radius: f64,
    ids: &[usize],
) -> (Option<String>, Vec<LeftPiece>) {
    let eps = REST_TOL_MM;
    let thin = ARC_APPROX_TOL_MM;
    let cell_size = radius.max(1.0);
    let paths = PrimIndex::new(
        cutting_segments(rings, open, links)
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
/// `clearance` moves the cut into the kept material (docs/specs/depths-and-clearance.md):
/// the tool centre runs `tool_radius - clearance` from the walls and islands.
#[allow(clippy::too_many_arguments)]
pub fn plan_pocket(
    contours: &[&Contour],
    tool_radius: f64,
    clearance: f64,
    stepover: f64,
    direction: Direction,
    spindle: Spindle,
    keep_tool_down: bool,
    start: Point,
) -> Result<PocketPlan, ToolpathError> {
    plan_pocket_with(
        contours,
        tool_radius,
        clearance,
        stepover,
        direction,
        spindle,
        keep_tool_down,
        start,
        PocketStrategy::Rings {},
    )
}

/// [`plan_pocket`] with a clearing strategy (docs/specs/trochoidal.md).
#[allow(clippy::too_many_arguments)]
pub fn plan_pocket_with(
    contours: &[&Contour],
    tool_radius: f64,
    clearance: f64,
    stepover: f64,
    direction: Direction,
    spindle: Spindle,
    keep_tool_down: bool,
    start: Point,
    strategy: PocketStrategy,
) -> Result<PocketPlan, ToolpathError> {
    plan_pocket_starting(
        contours,
        tool_radius,
        clearance,
        stepover,
        direction,
        spindle,
        keep_tool_down,
        start,
        strategy,
        Choices::default(),
    )
}

/// Where the wall rings start, their leads, and the helix the tool goes down round
/// (docs/specs/helix-and-leads.md); the default is none of them.
#[derive(Debug, Clone, Copy, Default)]
pub struct Choices<'a> {
    /// Start points, moved with the copy (part A).
    pub starts: &'a [Point],
    /// Lead radius (part B).
    pub lead: Option<f64>,
    /// Helix radius (part C).
    pub helix: Option<f64>,
    /// Rings strategy: round the inner rings' corners (docs/specs/pocket-round-corners.md).
    pub round_corners: bool,
}

/// One piece's paths before its links are chosen and it is verified.
struct Draft {
    region: Area,
    ids: Vec<usize>,
    rings: Vec<Vec<Point>>,
    level0: Vec<Area>,
    open: Vec<bool>,
    walls: Vec<bool>,
    chosen: Vec<Option<usize>>,
    leads: Vec<Option<std::ops::Range<usize>>>,
}

/// [`plan_pocket_with`] with start points and leads for the wall rings, and a helix to go
/// down round (docs/specs/helix-and-leads.md).
#[allow(clippy::too_many_arguments)]
pub fn plan_pocket_starting(
    contours: &[&Contour],
    tool_radius: f64,
    clearance: f64,
    stepover: f64,
    direction: Direction,
    spindle: Spindle,
    keep_tool_down: bool,
    start: Point,
    strategy: PocketStrategy,
    choices: Choices,
) -> Result<PocketPlan, ToolpathError> {
    let Choices {
        starts,
        lead,
        helix,
        round_corners,
    } = choices;
    if stepover <= 0.0 || stepover.is_nan() {
        return Err(geometry(format!(
            "stepover must be positive, got {stepover}"
        )));
    }
    let offset = tool_radius - clearance;
    if offset <= 0.0 || offset.is_nan() {
        return Err(geometry(format!(
            "clearance {clearance} must be less than the tool radius {tool_radius}"
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
    let mut arcs = ArcTable::default();
    let mut engagement = Vec::new();
    // Every region with where the tool centre can go in it, for the material left.
    let mut reachable: Vec<(Area, Vec<Area>)> = Vec::new();
    let mut position = start;
    let mut drafts: Vec<Draft> = Vec::new();
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
        if let PocketStrategy::Trochoidal { loop_radius, .. } = strategy
            && loop_radius > tool_radius
        {
            return Err(geometry(format!(
                "contours {}: loop radius {loop_radius} must be at most the tool radius \
                 {tool_radius}, or each loop leaves an uncut core",
                ids_text(&ids)
            )));
        }
        let clearing = Clearing {
            strategy,
            round_corners,
            offset,
            tool_radius,
            stepover,
            position,
            direction,
            spindle,
        };
        let Attempt {
            rings,
            level0,
            open,
            walls,
            arcs: used_arcs,
            engagement: used_engagement,
            notes: used_notes,
        } = cleared_piece(&region, &ids, &clearing)?;
        arcs.extend(used_arcs);
        engagement.extend(used_engagement);
        notes.extend(used_notes);
        if level0.len() > 1 {
            notes.push(format!(
                "contours {}: the tool does not fit through a narrow part; cleared as {} \
                 areas and material remains in between",
                ids_text(&ids),
                level0.len()
            ));
        }
        position = rings[0][0]; // every depth starts again at the first ring
        let chosen = vec![None; rings.len()];
        let leads = vec![None; rings.len()];
        drafts.push(Draft {
            region,
            ids,
            rings,
            level0,
            open,
            walls,
            chosen,
            leads,
        });
    }
    // The order and the clearing were chosen from the planner's starts; only the wall
    // rings move to the user's.
    choose_starts(
        &mut drafts,
        starts,
        2.0 * tool_radius,
        &mut arcs,
        &mut notes,
    );
    for draft in drafts {
        let Draft {
            region,
            ids,
            mut rings,
            level0,
            mut open,
            walls: wall_rings,
            chosen,
            mut leads,
        } = draft;
        let walls = PrimIndex::new(region.boundary(), tool_radius.max(1.0));
        if let Some(radius) = lead {
            // Climb with M3 keeps the walls on the right: the cleared side is on the left.
            let waste_left = !cut_clockwise(Side::Inside, direction, spindle);
            let fits =
                |l: &Leads| l.all_points(|p| region.contains(p)) && l.clear_of(&walls, offset);
            let min_radius = 2.0 * tool_radius * MIN_LEAD_DIAMETERS;
            for k in (0..rings.len()).filter(|&k| wall_rings[k]) {
                if chosen[k].is_none()
                    && let Some(at) = roomiest_start(&rings[k])
                {
                    rings[k] = restart(&rings[k], at, &mut arcs);
                }
                match fitted_leads(&rings[k], radius, min_radius, waste_left, fits) {
                    Some((l, used)) => {
                        if used < radius {
                            notes.push(format!(
                                "contours {}: lead radius reduced to {used:.3} mm (no room \
                                 for {radius:.3} mm)",
                                ids_text(&ids)
                            ));
                        }
                        l.tag(&mut arcs);
                        let ring = std::mem::take(&mut rings[k]);
                        let before = l.lead_in.len() - 1;
                        let mut path = l.lead_in[..before].to_vec();
                        path.extend(ring.iter().copied());
                        path.extend(l.lead_out.iter().copied());
                        leads[k] = Some(before..before + ring.len());
                        rings[k] = path;
                        open[k] = true;
                    }
                    None => notes.push(format!(
                        "contours {}: no room for a lead (not even {min_radius:.3} mm); the \
                         wall is cut without one",
                        ids_text(&ids)
                    )),
                }
            }
        }
        let links: Vec<bool> = (1..rings.len())
            .map(|k| {
                let from = path_end(&rings[k - 1], open[k - 1]);
                keep_tool_down
                    && from.is_some_and(|from| {
                        link_is_safe(&region, &walls, from, rings[k][0], tool_radius, offset)
                    })
            })
            .collect();
        verify(&region, &walls, &rings, &open, &links, offset, &ids, &arcs)?;
        let helix = match helix {
            Some(radius) => {
                let wall_cw = cut_clockwise(Side::Inside, direction, spindle);
                let fit = HelixFit {
                    region: &region,
                    walls: &walls,
                    offset,
                    tool_radius,
                    clockwise: wall_cw,
                };
                fit_helix(&fit, radius, &rings, &ids, &mut notes)?
            }
            None => None,
        };
        let (uncut, missed) =
            uncut_material(&region, &level0, &rings, &open, &links, tool_radius, &ids);
        notes.extend(uncut);
        left.extend(missed);
        reachable.push((region, level0));
        pieces.push(PocketPiece {
            rings,
            open,
            feed_links: links,
            contour_ids: ids,
            walls: wall_rings,
            chosen,
            leads,
            helix,
        });
    }
    // Display only: any failure, even a panic in the geometry library, becomes a note.
    let unreachable = guarded("Material left", || {
        unreachable_in_pockets(&reachable, tool_radius, clearance)
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
        arcs,
        engagement,
    })
}

/// What a helix must fit: the piece, its walls, and how far the tool centre keeps from them.
struct HelixFit<'a> {
    region: &'a Area,
    walls: &'a PrimIndex,
    offset: f64,
    tool_radius: f64,
    clockwise: bool,
}

/// The helix for a piece (docs/specs/helix-and-leads.md part C): round the point furthest
/// from the walls in the area nearest the first path, its circle at least the tool centre's
/// offset from every wall. The radius shrinks to the largest that fits, down to a tenth of
/// the tool diameter; the helix and the feed from it to the first path are checked against
/// the walls. None, with a note, if no helix fits: the piece then ramps in.
fn fit_helix(
    fit: &HelixFit,
    radius: f64,
    rings: &[Vec<Point>],
    ids: &[usize],
    notes: &mut Vec<String>,
) -> Result<Option<Helix>, ToolpathError> {
    let Some(&first) = rings.first().and_then(|r| r.first()) else {
        return Ok(None);
    };
    let min_radius = 0.2 * fit.tool_radius; // a tenth of the diameter
    let without = |why: &str| {
        format!(
            "contours {}: no room for a helix ({why}); ramping in instead",
            ids_text(ids)
        )
    };
    // The areas a helix centre of radius r may lie in.
    let room = |r: f64| fit.region.shrunk(fit.offset + r);
    let mut used = radius;
    if room(radius)?.is_empty() {
        if room(min_radius)?.is_empty() {
            notes.push(without(&format!("not even {min_radius:.3} mm")));
            return Ok(None);
        }
        // The largest radius that fits, to within a thousandth of the range.
        let (mut lo, mut hi) = (min_radius, radius);
        for _ in 0..10 {
            let mid = (lo + hi) / 2.0;
            if room(mid)?.is_empty() {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        used = lo;
        notes.push(format!(
            "contours {}: helix radius reduced to {used:.3} mm (no room for {radius:.3} mm)",
            ids_text(ids)
        ));
    }
    let areas = room(used)?;
    let Some(nearest) = areas
        .iter()
        .min_by(|a, b| a.distance(first).total_cmp(&b.distance(first)))
    else {
        notes.push(without("the pocket is too narrow"));
        return Ok(None);
    };
    // Deepest: the last non-empty shrink of that area, a tool radius at a time.
    let mut center = first_point(&nearest.outer);
    for k in 1..MAX_RING_LEVELS {
        let inner = nearest.shrunk(k as f64 * fit.tool_radius)?;
        let Some(area) = inner.first() else {
            break;
        };
        center = first_point(&area.outer);
    }
    // Start on the side towards the first path, so the feed to it is short.
    let towards = match distance(center, first) {
        d if d > 0.0 => ((first.0 - center.0) / d, (first.1 - center.1) / d),
        _ => (1.0, 0.0),
    };
    let helix = Helix {
        circle: Circle {
            center,
            radius: used,
        },
        start: (center.0 + used * towards.0, center.1 + used * towards.1),
        clockwise: fit.clockwise,
    };
    let clearance = min_clearance(fit.offset);
    let points = helix.points();
    let clear = points.iter().all(|&p| fit.region.contains(p))
        && points
            .windows(2)
            .all(|w| clear_of(fit.walls, w[0], w[1], clearance))
        && clear_of(fit.walls, helix.start, first, clearance);
    if !clear {
        notes.push(without("its way to the first path passes a wall or island"));
        return Ok(None);
    }
    Ok(Some(helix))
}

/// Restart each wall ring at the stored point nearest to it (among all the pieces' wall
/// rings), with notes for points far from their ring (more than `far`) and for a second
/// point on one ring.
fn choose_starts(
    drafts: &mut [Draft],
    starts: &[Point],
    far: f64,
    arcs: &mut ArcTable,
    notes: &mut Vec<String>,
) {
    if starts.is_empty() {
        return;
    }
    let mut at: Vec<(usize, usize)> = Vec::new();
    for (d, draft) in drafts.iter().enumerate() {
        at.extend(
            (0..draft.rings.len())
                .filter(|&k| draft.walls[k])
                .map(|k| (d, k)),
        );
    }
    let rings: Vec<&[Point]> = at
        .iter()
        .map(|&(d, k)| drafts[d].rings[k].as_slice())
        .collect();
    let (chosen, duplicates) = assign(starts, &rings);
    for (&(d, k), choice) in at.iter().zip(&chosen) {
        let Some(choice) = choice else { continue };
        let draft = &mut drafts[d];
        draft.rings[k] = restart(&draft.rings[k], choice.stored, arcs);
        draft.chosen[k] = Some(choice.index);
        if choice.distance > far {
            notes.push(format!(
                "contours {}: start point {} is {:.3} mm from the wall; the nearest point is \
                 used",
                ids_text(&draft.ids),
                point_text(choice.stored),
                choice.distance
            ));
        }
    }
    for (index, r) in duplicates {
        let (d, _) = at[r];
        notes.push(format!(
            "contours {}: start point {} is on a wall that already has one; it is ignored",
            ids_text(&drafts[d].ids),
            point_text(starts[index])
        ));
    }
}

/// Material no pocket of the operation reaches. Crossing pockets clear each other's
/// corners, so each region is checked against the reach of all of them.
fn unreachable_in_pockets(
    regions: &[(Area, Vec<Area>)],
    radius: f64,
    clearance: f64,
) -> Result<Vec<LeftPiece>, ToolpathError> {
    let mut reached = Vec::new();
    for (_, level0) in regions {
        reached.extend(reach(level0, radius)?);
    }
    let mut pieces = Vec::new();
    for (region, _) in regions {
        for target in cleared(region, clearance)? {
            pieces.extend(unreachable_inside(&target, &reached)?);
        }
    }
    Ok(pieces)
}

/// A piece's paths in cut order, its wall-pass areas (the level 0 of the uncut and
/// material-left checks), which paths are open, and which are wall rings.
type PiecePaths = (Vec<Vec<Point>>, Vec<Area>, Vec<bool>, Vec<bool>);

/// A piece's closed paths in cut order, its wall-pass areas, and which are wall rings.
type ClosedPaths = (Vec<Vec<Point>>, Vec<Area>, Vec<bool>);

/// Adaptive passes (open) and then the wall pass, with the wall-pass areas
/// (docs/specs/adaptive-fronts.md). `width`: the cut width (stepover), mm.
#[allow(clippy::too_many_arguments)]
fn adaptive_rings(
    region: &Area,
    offset: f64,
    tool_radius: f64,
    width: f64,
    ids: &[usize],
    position: Point,
    direction: Direction,
    spindle: Spindle,
    arcs: &mut ArcTable,
    engagement: &mut Vec<f64>,
) -> Result<PiecePaths, ToolpathError> {
    let walls = region.shrunk(offset)?;
    if walls.is_empty() {
        return Err(geometry(format!(
            "contours {}: tool (diameter {:.3} mm) does not fit in the pocket",
            ids_text(ids),
            2.0 * tool_radius
        )));
    }
    let diameter = 2.0 * tool_radius;
    let h = (diameter / 30.0).min(ADAPTIVE_CELL_MM);
    let Some(extents) = region.outer.extents() else {
        return Err(geometry("adaptive clearing: empty pocket".into()));
    };
    let (lo, hi) = (
        (extents.min_x - tool_radius, extents.min_y - tool_radius),
        (extents.max_x + tool_radius, extents.max_y + tool_radius),
    );
    let rings_of = |area: &Area| -> Vec<Vec<Point>> {
        std::iter::once(&area.outer)
            .chain(&area.holes)
            .map(flatten_tagged_plain)
            .collect()
    };
    // Where the passes go: all of the wall pass's area. The fronts reach the walls
    // themselves, so no band is left that a tool on its edge would sweep sideways where a
    // front meets a wall at a shallow angle; the wall pass then only takes the cusps
    // between front ends (docs/specs/adaptive-fronts.md §10).
    let parts = walls.clone();
    // The material the passes cut, for returns and engagement: cells whose whole square the
    // tool covers from somewhere in the parts (centres within r − 2h; the model cuts with
    // r − h/√2).
    let mut material = Grid::new(lo, hi, h);
    for part in &parts {
        for grown in part.grown(tool_radius - 2.0 * h)? {
            let mut one = Grid::new(lo, hi, h);
            one.fill(&rings_of(&grown));
            material.or(&one);
        }
    }
    // Material on the right is climb with an M3 spindle, as for ring walls.
    let material_right = !cut_clockwise(Side::Inside, direction, spindle);
    let reach = tool_radius - h * std::f64::consts::FRAC_1_SQRT_2;
    // A spiral for every part that suits one (docs/specs/adaptive-spiral.md §3.2); fronts
    // for the others, until the spiral covers them too.
    let theta0 = spiral::target_angle(width, tool_radius);
    let mut rings: Vec<Vec<Point>> = Vec::new();
    let mut rest = Vec::new();
    for part in &parts {
        let plan = spiral::plan(
            &spiral::Spiral {
                area: part,
                tool_radius,
                width,
                material_right,
            },
            arcs,
        )?;
        match plan {
            Some(path) => {
                // Measured on its own finer grid (see ADAPTIVE_CELL_MM).
                let fine = h / 2.0;
                let mut grid = Grid::new(lo, hi, fine);
                for grown in part.grown(tool_radius - 2.0 * fine)? {
                    grid.fill(&rings_of(&grown));
                }
                let reach = tool_radius - fine * std::f64::consts::FRAC_1_SQRT_2;
                engagement.extend(spiral::measure(
                    &mut grid,
                    &path.points,
                    path.opening,
                    reach,
                    theta0,
                ));
                rings.push(path.points);
            }
            None => rest.push(part.clone()),
        }
    }
    let here = rings
        .last()
        .and_then(|p| p.last())
        .copied()
        .unwrap_or(position);
    let setup = fronts::Setup {
        parts: &rest,
        width,
        material_right,
        material,
        reach,
    };
    let passes = fronts::clear(setup, here, arcs)?;
    engagement.extend(passes.engagement);
    rings.extend(passes.paths);
    let mut open = vec![true; rings.len()];
    let here = rings
        .last()
        .and_then(|p| p.last())
        .copied()
        .unwrap_or(position);
    let wall_nodes: Vec<Node> = walls
        .iter()
        .map(|area| Node {
            area: area.clone(),
            children: Vec::new(),
        })
        .collect();
    let roots: Vec<usize> = (0..wall_nodes.len()).collect();
    let (wall_rings, wall_flags) = cut_order(&wall_nodes, &roots, here, direction, spindle, arcs);
    let mut flags = vec![false; rings.len()];
    flags.extend(wall_flags);
    open.extend(vec![false; wall_rings.len()]);
    rings.extend(wall_rings);
    Ok((rings, walls, open, flags))
}

/// Flattened points of a ring, without recording arcs (the material model only needs the
/// shape).
fn flatten_tagged_plain(pline: &Polyline<f64>) -> Vec<Point> {
    flatten_tagged(pline, &mut ArcTable::default())
}

/// Largest cell of the adaptive material grid, mm (smaller for tools under 6 mm: D / 30).
/// The spiral's engagement angle is measured on a grid of half that: with 0.2 mm cells, at
/// a 37° target the angle read varied by about 30 % from step to step where the cut was
/// steady.
const ADAPTIVE_CELL_MM: f64 = 0.2;

/// Sizes for trochoidal clearing, mm (docs/specs/trochoidal.md section 3).
struct Trochoid {
    /// Tool centre to walls (tool radius minus clearance).
    offset: f64,
    tool_radius: f64,
    loop_radius: f64,
    /// How far each loop moves on along its backbone.
    step: f64,
    /// Spacing of the backbones: 2 loop radii plus the stepover, at most the loops' reach.
    band: f64,
}

/// The backbones' loop paths from the inside out, then the wall pass, and the wall-pass
/// areas (the "level 0" the uncut and material-left checks start from).
#[allow(clippy::too_many_arguments)]
fn trochoidal_rings(
    region: &Area,
    t: &Trochoid,
    ids: &[usize],
    position: Point,
    direction: Direction,
    spindle: Spindle,
    arcs: &mut ArcTable,
    notes: &mut Vec<String>,
) -> Result<ClosedPaths, ToolpathError> {
    let walls = region.shrunk(t.offset)?;
    if walls.is_empty() {
        return Err(geometry(format!(
            "contours {}: tool (diameter {:.3} mm) does not fit in the pocket",
            ids_text(ids),
            2.0 * t.tool_radius
        )));
    }
    let loop_cw = cut_clockwise(Side::Inside, direction, spindle); // material outside a loop
    let mut rings: Vec<Vec<Point>> = Vec::new();
    let mut here = position;
    if region.shrunk(t.offset + t.loop_radius)?.is_empty() {
        notes.push(format!(
            "contours {}: the loops (radius {:.3} mm) don't fit anywhere; only the wall pass \
             cuts this pocket",
            ids_text(ids),
            t.loop_radius
        ));
    } else {
        let (nodes, roots) = ring_tree(
            region,
            t.offset + t.loop_radius,
            t.tool_radius,
            t.band,
            ids,
            Nudge::NONE,
        )
        .map_err(Failure::into_error)?;
        // The backbones themselves aren't cut: their chords don't go in `arcs`.
        let (backbones, _) = cut_order(
            &nodes,
            &roots,
            here,
            direction,
            spindle,
            &mut ArcTable::default(),
        );
        for backbone in &backbones {
            let path = loops_along(backbone, t.loop_radius, t.step, loop_cw, arcs);
            // A path ends where it starts (the last loop links back to the first).
            if let Some(&start) = path.first() {
                here = start;
            }
            rings.push(path);
        }
    }
    // The wall pass: a ring at the tool radius from every wall and island, as the last ring
    // of a ring pocket.
    let wall_nodes: Vec<Node> = walls
        .iter()
        .map(|area| Node {
            area: area.clone(),
            children: Vec::new(),
        })
        .collect();
    let roots: Vec<usize> = (0..wall_nodes.len()).collect();
    let (wall_rings, wall_flags) = cut_order(&wall_nodes, &roots, here, direction, spindle, arcs);
    let mut flags = vec![false; rings.len()];
    flags.extend(wall_flags);
    rings.extend(wall_rings);
    Ok((rings, walls, flags))
}

/// One closed path of loops along a closed `backbone` (cut direction already set): loop
/// centres every `step` or a little less (evenly by length), at each a full circle of
/// `radius` starting and ending at its point towards the next centre, consecutive loops
/// linked by a straight move. The circles' chords are recorded in `arcs`.
fn loops_along(
    backbone: &[Point],
    radius: f64,
    step: f64,
    clockwise: bool,
    arcs: &mut ArcTable,
) -> Vec<Point> {
    let n = backbone.len();
    if n == 0 {
        return Vec::new();
    }
    let mut lengths = vec![0.0];
    for k in 0..n {
        let next = lengths[k] + distance(backbone[k], backbone[(k + 1) % n]);
        lengths.push(next);
    }
    let total = lengths[n];
    let count = ((total / step).ceil() as usize).max(1);
    let at = |s: f64| -> Point {
        let k = lengths
            .partition_point(|&l| l <= s)
            .saturating_sub(1)
            .min(n - 1);
        let (a, b) = (backbone[k], backbone[(k + 1) % n]);
        let span = lengths[k + 1] - lengths[k];
        let t = if span > 0.0 {
            (s - lengths[k]) / span
        } else {
            0.0
        };
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    };
    let centres: Vec<Point> = (0..count)
        .map(|i| at(total * i as f64 / count as f64))
        .collect();
    let circle_sweep = if clockwise { -TAU } else { TAU };
    let mut path = Vec::new();
    let mut heading = 0.0;
    for (i, &c) in centres.iter().enumerate() {
        let next = centres[(i + 1) % count];
        if distance(c, next) > CHAIN_TOL_MM {
            heading = (next.1 - c.1).atan2(next.0 - c.0);
        }
        let circle = Primitive::Arc(chipload_core::primitives::Arc {
            center: c,
            radius,
            start_angle: heading,
            sweep: circle_sweep,
        });
        let mut points = circle.points();
        points.pop(); // the end repeats the start; close on the exact start instead
        let Some(&first) = points.first() else {
            continue;
        };
        points.push(first);
        let tag = Circle { center: c, radius };
        for w in points.windows(2) {
            arcs.insert(w[0], w[1], tag);
        }
        path.extend(points);
    }
    path
}

/// All depth passes of one piece. Each pass starts at the centre and ends retracted.
/// `top_z` is the Z of the start depth, cut already; every descent into a pass goes
/// down as `entry` says, along the path that follows (docs/specs/ramp-entry.md).
#[allow(clippy::too_many_arguments)]
pub fn pocket_moves(
    piece: &PocketPiece,
    pass_z: &[f64],
    top_z: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
    entry: Entry,
) -> Result<Vec<Move>, ToolpathError> {
    let mut moves = Vec::new();
    let Some(first) = piece.rings.first().and_then(|r| r.first().copied()) else {
        return Ok(moves);
    };
    // The path the tool follows from a ring's start, to ramp along.
    let path_of = |index: usize| {
        let ring = &piece.rings[index];
        if piece.open.get(index).copied().unwrap_or(false) {
            ring.clone()
        } else {
            crate::profile::closed_path(ring)
        }
    };
    let descend = |index: usize, cleared: f64, z: f64| {
        let path = path_of(index);
        let length = crate::ramp::polyline_length(&path);
        crate::ramp::enter(&path, length, safe_z, cleared, z, entry, feed, plunge_feed)
    };
    let mut cleared = top_z;
    for &z in pass_z {
        match (piece.helix, entry) {
            (Some(helix), Entry::Helix { ramp_angle, .. }) => {
                // Down round the helix, then at depth to the first path (checked when the
                // helix was fitted).
                moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(helix.start)]);
                moves.extend(helix_moves(
                    &helix,
                    cleared,
                    z,
                    ramp_angle,
                    feed,
                    plunge_feed,
                )?);
                moves.push(Move::feed_xy(first, feed));
            }
            _ => {
                moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(first)]);
                moves.extend(descend(0, cleared, z)?);
            }
        }
        for (index, ring) in piece.rings.iter().enumerate() {
            let start = ring[0];
            let open = piece.open.get(index).copied().unwrap_or(false);
            if index > 0 {
                if piece.feed_links[index - 1] {
                    moves.push(Move::feed_xy(start, feed));
                } else {
                    moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(start)]);
                    moves.extend(descend(index, cleared, z)?);
                }
            }
            moves.extend(ring[1..].iter().map(|&p| Move::feed_xy(p, feed)));
            if !open {
                moves.push(Move::feed_xy(start, feed));
            }
        }
        moves.push(Move::rapid_z(safe_z));
        cleared = z;
    }
    Ok(moves)
}
