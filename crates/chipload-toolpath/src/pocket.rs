// SPDX-License-Identifier: GPL-3.0-or-later
//! Pocket clearing: contour-parallel rings, cut from the inside out. See
//! docs/specs/pocket.md.
//!
//! Offsets use cavalier_contours shapes (outer loop plus islands, true arcs). Every result is
//! verified afterwards, and uncut material is found with exact distances, not by buffering
//! the paths.

use std::f64::consts::SQRT_2;

use cavalier_contours::polyline::{
    BooleanOp, PlineContainsResult, PlineSource, PlineSourceMut, Polyline,
};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::fit::fit_path;
use chipload_core::nesting::{Ring, contains};
use chipload_core::order::Ordered;
use chipload_core::primitives::{Line, Primitive, distance, signed_area};
use chipload_core::tolerances::{
    ARC_APPROX_TOL_MM, CHAIN_TOL_MM, OFFSET_CHECK_SLACK_MM, REST_TOL_MM,
};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Direction, Entry, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::{ArcTable, Circle, arc_midpoint};
use crate::cancel::{self, Cancel};
use crate::cavalier::{
    Area, boolean, ccw, cw, encloses, first_point, geometry, shrunk_farther, step, too_small,
};
use crate::geom::PrimIndex;
use crate::helix::{Helix, helix_moves};
use crate::lead::{Leads, MIN_LEAD_DIAMETERS, fitted_leads, roomiest_start};
use crate::left::{cleared, left_by, rest_areas};
use crate::model::Move;
use crate::offset::{contour_pline, dedupe_ring, flatten_tagged, pline_segments};
use crate::profile::cut_clockwise;
use crate::retry::{Failure, Nudge, nudged_or_tight};
use crate::start::{assign, nearest_ties, onto, point_value, restart};

/// Guard against runaway loops on absurd input; a 2 m pocket at 0.1 mm stepover is 10 000.
pub const MAX_RING_LEVELS: usize = 20_000;

/// One path of a piece: a ring closed back to its start, or an open path, with what the
/// planner decided about it.
#[derive(Debug, Clone, PartialEq)]
pub struct PocketRing {
    /// Oriented and starting at points[0]; a closed ring also ends there.
    pub points: Vec<Point>,
    /// An open path ending at its last point, not closed back to its start.
    pub open: bool,
    /// A wall ring (the last ring along a wall or island at each depth), whose start the
    /// user can move.
    pub wall: bool,
    /// The index in the operation's `starts` of the point that set its start
    /// (docs/specs/helix-and-leads.md part A).
    pub chosen: Option<usize>,
    /// A wall ring with a lead-in before it and a lead-out after it, as one open path: the
    /// ring itself is `points[range]` (part B).
    pub lead: Option<std::ops::Range<usize>>,
    /// The ring steps out onto the next one (the ring outside it) over its last stretch
    /// instead of closing, so the two are one spiral (docs/specs/pocket-spiral.md). Set by
    /// the cut order where the next ring is the one outside; true only where it was done.
    pub steps_out: bool,
}

impl PocketRing {
    /// A closed ring whose start the planner chose; `wall` says whether it runs along a
    /// wall or island.
    pub fn closed(points: Vec<Point>, wall: bool) -> Self {
        PocketRing {
            points,
            open: false,
            wall,
            chosen: None,
            lead: None,
            steps_out: false,
        }
    }

    /// Where the tool is after cutting it: back at its start if closed, at its end if open.
    pub fn end(&self) -> Option<Point> {
        if self.open {
            self.points.last()
        } else {
            self.points.first()
        }
        .copied()
    }

    /// The ring itself, without its leads.
    pub fn plain(&self) -> &[Point] {
        match &self.lead {
            Some(range) => self.points.get(range.clone()).unwrap_or(&[]),
            None => &self.points,
        }
    }
}

/// One connected area to clear, with its paths in cut order.
#[derive(Debug, Clone, PartialEq)]
pub struct PocketPiece {
    pub rings: Vec<PocketRing>,
    /// feed_links[i]: stay at depth from the end of rings[i] into rings[i + 1].
    pub feed_links: Vec<bool>,
    /// Selected contours that make up this piece, for messages.
    pub contour_ids: Vec<usize>,
    /// The helix each depth pass goes down round, then feeds at depth to the first ring's
    /// start (part C); None: the first path's own entry.
    pub helix: Option<Helix>,
    /// Between depth passes the tool stays down: from the last ring's end, lifted off the
    /// floor, back to where the next pass starts (docs/specs/pocket-stay-down.md). False:
    /// it retracts to safe Z.
    pub back_link: bool,
}

/// How far the tool lifts off the floor just cut to move back between depth passes
/// (docs/specs/pocket-stay-down.md decision 2).
const BACK_LINK_LIFT_MM: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct PocketPlan {
    pub pieces: Vec<PocketPiece>,
    /// Warnings: narrow parts, uncut material.
    pub notes: Vec<Message>,
    /// Which ring chords lie on arcs (docs/specs/arc-output.md).
    pub arcs: ArcTable,
}

/// A length with 3 decimals, for notes.
fn fixed3(mm: f64) -> Value {
    Value::Length(mm, Digits::Fixed(3))
}

fn ids_text(ids: &[usize]) -> String {
    ids.iter()
        .map(|i| format!("#{i}"))
        .collect::<Vec<_>>()
        .join(", ")
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
        (a.area.sort_key(), &a.contour_ids).cmp(&(b.area.sort_key(), &b.contour_ids))
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
            let common = boolean(step("comparing-shapes"), a, b, BooleanOp::And)?;
            let symmetric = a.area().abs() + b.area().abs() - 2.0 * boolean_area(&common);
            let slack = CHAIN_TOL_MM * a.path_length().max(b.path_length());
            if symmetric <= slack {
                return Err(ToolpathError::Selector(
                    Message::new("toolpath-error-same-shape")
                        .arg("a", *id_a)
                        .arg("b", *id_b),
                ));
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
            let mut union = boolean(step("merging-islands"), &current, &other, BooleanOp::Or)?;
            if union.pos_plines.len() != 1 || !union.neg_plines.is_empty() {
                return Err(geometry(
                    Message::new("toolpath-error-islands-enclose").arg("contours", ids_text(ids)),
                ));
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
                    let cut = boolean(
                        step("subtracting-an-island"),
                        &area.outer,
                        &island,
                        BooleanOp::Not,
                    )?;
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
    Ok(areas.into_iter().filter(|a| !too_small(&a.outer)).collect())
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
    let flat;
    let region = if nudge.flat {
        flat = region.flattened();
        &flat
    } else {
        region
    };
    for area in region
        .shrunk_with(wall_offset, nudge.tight)
        .map_err(numerical(true))?
    {
        roots.push(nodes.len());
        nodes.push(Node {
            area,
            children: Vec::new(),
        });
    }
    if roots.is_empty() {
        let message = if radius == tool_radius {
            Message::new("toolpath-error-pocket-no-fit")
        } else {
            Message::new("toolpath-error-pocket-no-fit-clearance").arg(
                "clearance",
                Value::Length(tool_radius - radius, Digits::Fixed(3)),
            )
        };
        return Err(Failure::Final(geometry(
            message.arg("contours", ids_text(ids)).arg(
                "diameter",
                Value::Length(2.0 * tool_radius, Digits::Fixed(3)),
            ),
        )));
    }
    let mut level = roots.clone();
    for k in 1..=MAX_RING_LEVELS {
        let areas = region
            .shrunk_with(wall_offset + k as f64 * stepover, nudge.tight)
            .map_err(numerical(false))?;
        if areas.is_empty() {
            return Ok((nodes, roots));
        }
        let mut next_level = Vec::new();
        for area in areas {
            let inside = first_point(&area.outer);
            let Some(&parent) = level.iter().find(|&&n| nodes[n].area.contains(inside)) else {
                return Err(Failure::Numerical {
                    error: geometry(
                        Message::new("toolpath-error-not-nested").arg("contours", ids_text(ids)),
                    ),
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
    Err(Failure::Final(geometry(
        Message::new("toolpath-error-too-many-rings")
            .arg("contours", ids_text(ids))
            .arg("most", MAX_RING_LEVELS),
    )))
}

/// Rounds of the uncut check, each halving the radii of the rings near material left.
const ROUNDING_ROUNDS: usize = 12;

/// The largest corner radius of a rounded ring, in stepovers (docs/specs/pocket-round-corners.md
/// as built 2026-10-15): enough to take the stop out of every corner, small enough that the
/// ring outside it meets at most about 1.5 stepovers of material in the corner.
const MAX_ROUNDING_STEPOVERS: f64 = 2.0;

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

/// One piece's paths from one attempt (docs/specs/offset-retry.md), with the arcs and
/// notes that attempt recorded: only the used attempt's are kept.
#[derive(Default)]
struct Attempt {
    rings: Vec<PocketRing>,
    level0: Vec<Area>,
    arcs: ArcTable,
    notes: Vec<Message>,
}

/// What a piece is cleared with, and where the tool comes from.
struct Clearing<'a> {
    /// Round the corners of the inner rings (off only to compare, in tests).
    round_corners: bool,
    offset: f64,
    tool_radius: f64,
    stepover: f64,
    position: Point,
    direction: Direction,
    spindle: Spindle,
    cancel: Option<&'a Cancel>,
}

/// The paths of one piece, checked as soon as they are built and retried with nudged
/// offsets after a numerical failure (docs/specs/offset-retry.md).
fn cleared_piece(region: &Area, ids: &[usize], c: &Clearing) -> Result<Attempt, ToolpathError> {
    cancel::check(c.cancel)?;
    let walls = PrimIndex::new(region.boundary(), c.tool_radius.max(1.0));
    nudged_or_tight(|nudge| {
        let a = attempt_piece(region, ids, c, nudge)?;
        if let Some(k) = first_unclear(region, &walls, &a.rings, c.offset, &a.arcs) {
            return Err(Failure::Numerical {
                error: verification_failed(ids),
                wall: a.rings.get(k).is_none_or(|r| r.wall),
            });
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
    let (mut nodes, roots) = ring_tree(region, c.offset, c.tool_radius, c.stepover, ids, nudge)?;
    if c.round_corners {
        let stepover = c.stepover - nudge.step;
        rounded_rings(region, &mut nodes, &roots, stepover, ids, c, &mut a.notes)
            .map_err(Failure::Final)?;
    }
    a.rings = cut_order(
        &nodes,
        &roots,
        Reach::new(c.tool_radius, c.stepover - nudge.step),
        c.position,
        c.direction,
        c.spindle,
        &mut a.arcs,
    );
    a.level0 = roots.iter().map(|&r| nodes[r].area.clone()).collect();
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
    notes: &mut Vec<Message>,
) -> Result<(), ToolpathError> {
    let level0: Vec<Area> = roots.iter().map(|&r| nodes[r].area.clone()).collect();
    let missed = |nodes: &[Node]| {
        let rings = cut_order(
            nodes,
            roots,
            Reach::new(c.tool_radius, stepover),
            c.position,
            c.direction,
            c.spindle,
            &mut ArcTable::default(),
        );
        let (note, missed) = uncut_material(region, &level0, &rings, &[], c.tool_radius, ids);
        note.map(|_| missed)
    };
    let sharp: Vec<Area> = nodes.iter().map(|n| n.area.clone()).collect();
    // Rings this far from material left are the ones that could have cut it.
    let near = c.tool_radius + stepover;
    let mut scales = vec![1.0; nodes.len()];
    for _ in 0..ROUNDING_ROUNDS {
        cancel::check(c.cancel)?;
        round_ring_corners(
            nodes,
            roots,
            stepover,
            MAX_ROUNDING_STEPOVERS * stepover,
            &scales,
        );
        let Some(left) = missed(nodes) else {
            return Ok(());
        };
        let mut changed = false;
        let points: Vec<Point> = left
            .iter()
            .flat_map(|piece| piece.iter().flatten().copied())
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
        notes.push(Message::new("toolpath-corners-not-rounded").arg("contours", ids_text(ids)));
    }
    Ok(())
}

/// The first path that comes closer than r (minus tolerance) to a wall or island or starts
/// outside the region: [`verify`] for the paths alone, before links and leads.
fn first_unclear(
    region: &Area,
    walls: &PrimIndex,
    rings: &[PocketRing],
    radius: f64,
    arcs: &ArcTable,
) -> Option<usize> {
    let clearance = min_clearance(radius);
    rings.iter().position(|ring| {
        let inside = ring.points.first().is_some_and(|&p| region.contains(p));
        let clear = cutting_segments(std::slice::from_ref(ring), &[])
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
    geometry(Message::new("toolpath-error-pocket-verification").arg("contours", ids_text(ids)))
}

/// Order for the travel direction and start at the vertex nearest `near` (ties, as
/// `nearest_ties` has them: y, x).
fn orient_from(ring: &[Point], clockwise: bool, near: Point) -> Vec<Point> {
    let mut ordered = ring.to_vec();
    if (signed_area(ring) < 0.0) != clockwise {
        ordered.reverse();
    }
    let start = nearest_ties((0..ordered.len()).map(|i| (i, distance(near, ordered[i]))))
        .into_iter()
        .min_by_key(|&i| (Ordered(ordered[i].1), Ordered(ordered[i].0)))
        .unwrap_or(0);
    ordered.rotate_left(start);
    ordered
}

/// One ring of the level tree: a node's outer ring, or the ring round one of its islands.
struct Loop {
    node: usize,
    /// Round an island (the node's hole); false: the node's outer ring.
    island: bool,
    /// Offset levels from the walls: 0 is the wall or island edge ring.
    level: usize,
    points: Vec<Point>,
    /// The rings one level in that must be cut first (docs/specs/pocket-island-order.md §2).
    deps: Vec<usize>,
}

/// The rings in cut order (docs/specs/pocket-island-order.md): deeper rings before the
/// rings next to them one level out, and otherwise one island at a time.
///
/// A ring is ready once the rings next to it one level in are cut (those within a tool
/// diameter, and at least 1.5 stepovers: a stepover apart along a side, and where the tool
/// at one reaches the other's material in a neck), and an outer ring also once its own
/// level's island rings are, so the wall is still the last ring at each depth. From the ring
/// just cut, the planner goes on outward to a ready ring of the same kind next to it, which
/// it steps out onto (pocket-spiral.md); otherwise it starts at the nearest ready ring.
fn cut_order(
    nodes: &[Node],
    roots: &[usize],
    reach: Reach,
    mut position: Point,
    direction: Direction,
    spindle: Spindle,
    arcs: &mut ArcTable,
) -> Vec<PocketRing> {
    let wall_cw = cut_clockwise(Side::Inside, direction, spindle); // material outside the ring
    let island_cw = cut_clockwise(Side::Outside, direction, spindle); // material inside it
    let loops = ring_loops(nodes, roots, reach, arcs);
    // The island rings of each node, which its outer ring waits for.
    let mut islands_of: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (i, l) in loops.iter().enumerate() {
        if l.island {
            islands_of[l.node].push(i);
        }
    }
    let mut done = vec![false; loops.len()];
    let ready = |i: usize, done: &[bool]| {
        let l = &loops[i];
        !done[i]
            && l.deps.iter().all(|&d| done[d])
            && (l.island || islands_of[l.node].iter().all(|&h| done[h]))
    };
    let mut rings: Vec<PocketRing> = Vec::new();
    let mut last: Option<usize> = None;
    for _ in 0..loops.len() {
        let here = position;
        // The nearest ring; equally near ones (`nearest_ties`) by level, node and kind.
        let nearest = |candidates: Vec<usize>| {
            let found = candidates.into_iter().map(|i| {
                let d = loops[i]
                    .points
                    .iter()
                    .map(|&p| distance(here, p))
                    .fold(f64::INFINITY, f64::min);
                (i, d)
            });
            nearest_ties(found).into_iter().min_by_key(|&i| {
                let l = &loops[i];
                (l.level, l.node, l.island)
            })
        };
        let onward = last.and_then(|from| {
            nearest(
                (0..loops.len())
                    .filter(|&i| {
                        ready(i, &done)
                            && loops[i].island == loops[from].island
                            && loops[i].deps.contains(&from)
                    })
                    .collect(),
            )
        });
        let Some(next) =
            onward.or_else(|| nearest((0..loops.len()).filter(|&i| ready(i, &done)).collect()))
        else {
            break; // every ring waits on another: cannot happen, levels only go outward
        };
        if onward.is_some()
            && let Some(previous) = rings.last_mut()
        {
            previous.steps_out = true;
        }
        let l = &loops[next];
        let clockwise = if l.island { island_cw } else { wall_cw };
        let ring = orient_from(&l.points, clockwise, position);
        position = ring[0];
        rings.push(PocketRing::closed(ring, l.level == 0));
        done[next] = true;
        last = Some(next);
    }
    rings
}

/// How far apart two rings one level apart may be and still have to be cut in order: a tool
/// diameter, and at least 1.5 stepovers (docs/specs/pocket-island-order.md §2).
#[derive(Clone, Copy)]
struct Reach(f64);

impl Reach {
    fn new(tool_radius: f64, stepover: f64) -> Self {
        Reach((2.0 * tool_radius).max(1.5 * stepover) + CHAIN_TOL_MM)
    }
}

/// Every node's rings, flattened, with their levels and the rings they wait for.
fn ring_loops(nodes: &[Node], roots: &[usize], reach: Reach, arcs: &mut ArcTable) -> Vec<Loop> {
    let mut level = vec![0usize; nodes.len()];
    let mut stack: Vec<usize> = roots.to_vec();
    while let Some(i) = stack.pop() {
        for &child in &nodes[i].children {
            level[child] = level[i] + 1;
            stack.push(child);
        }
    }
    let mut loops: Vec<Loop> = Vec::new();
    let mut of_node: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut shapes: Vec<&Polyline<f64>> = Vec::new();
    for (n, node) in nodes.iter().enumerate() {
        let rings = std::iter::once((false, &node.area.outer))
            .chain(node.area.holes.iter().map(|h| (true, h)));
        for (island, pline) in rings {
            of_node[n].push(loops.len());
            shapes.push(pline);
            loops.push(Loop {
                node: n,
                island,
                level: level[n],
                points: flatten_tagged(pline, arcs),
                deps: Vec::new(),
            });
        }
    }
    let Reach(limit) = reach;
    for (n, node) in nodes.iter().enumerate() {
        for &i in &of_node[n] {
            let index = PrimIndex::new(pline_segments(shapes[i]), limit);
            for &child in &node.children {
                for &j in &of_node[child] {
                    if loops[j].points.iter().any(|&p| index.any_within(p, limit)) {
                        loops[i].deps.push(j);
                    }
                }
            }
        }
    }
    loops
}

/// How far before its end a ring starts stepping out onto the next, in tool diameters
/// (docs/specs/pocket-spiral.md §2): a stepover sideways over two diameters along.
const STEP_OUT_DIAMETERS: f64 = 2.0;

/// The most a step-out may move the tool sideways, in stepovers. The next ring lies one
/// stepover out along a side, and √2 stepovers out on the diagonal of a rounded corner
/// (consecutive rings' corners have the same capped radius, so they are not concentric);
/// a start farther away than this is linked instead.
const STEP_OUT_MAX_STEPOVERS: f64 = 1.5;

/// Where a ring may step out onto the next (docs/specs/pocket-spiral.md §2): the next ring
/// is restarted at its point nearest the ring's start (unless the user chose its start or
/// it has leads), and the ring's last stretch drifts sideways onto that point, so the two
/// are cut as one spiral. Every step-out is checked against the walls like a ring; one
/// that doesn't fit, or whose landing point is too far, stays a closed ring with a link.
/// Returns whether any ring stepped out.
fn step_out(
    rings: &mut [PocketRing],
    arcs: &mut ArcTable,
    stepover: f64,
    tool_diameter: f64,
    region: &Area,
    walls: &PrimIndex,
    offset: f64,
) -> bool {
    let mut any = false;
    let clearance = min_clearance(offset);
    for i in 0..rings.len().saturating_sub(1) {
        if !rings[i].steps_out {
            continue;
        }
        rings[i].steps_out = false;
        if rings[i].open || rings[i].points.len() < 3 {
            continue;
        }
        let start = rings[i].points[0];
        let next = &mut rings[i + 1];
        if next.chosen.is_none() && next.lead.is_none() && !next.open {
            next.points = restart(&next.points, start, arcs);
        }
        let target = next.points[0];
        let shift = (target.0 - start.0, target.1 - start.1);
        if distance(start, target) > STEP_OUT_MAX_STEPOVERS * stepover + CHAIN_TOL_MM {
            continue;
        }
        let path = stepped_out(
            &rings[i].points,
            shift,
            STEP_OUT_DIAMETERS * tool_diameter,
            arcs,
        );
        let fits = path.iter().all(|&p| region.contains(p))
            && path
                .windows(2)
                .all(|w| clear_of(walls, w[0], w[1], clearance));
        if !fits {
            continue;
        }
        rings[i].points = path;
        rings[i].open = true;
        rings[i].steps_out = true;
        any = true;
    }
    any
}

/// The closed ring as a path that ends `shift` away from its start: its last `length` mm
/// drift sideways evenly, from nothing to the whole shift at the end. The stretch begins
/// at a vertex (one is inserted on the chord there, onto the arc if the chord is on one).
/// The drifted stretch is no longer on the ring's arcs, so it is fitted with lines and
/// arcs of its own (`chipload_core::fit`) and those are tagged in `arcs`.
fn stepped_out(ring: &[Point], shift: Point, length: f64, arcs: &mut ArcTable) -> Vec<Point> {
    let path = crate::profile::closed_path(ring);
    let mut along = vec![0.0];
    for w in path.windows(2) {
        along.push(along[along.len() - 1] + distance(w[0], w[1]));
    }
    let total = along[along.len() - 1];
    let stretch = length.min(total / 2.0);
    if stretch <= CHAIN_TOL_MM {
        return path;
    }
    let from = total - stretch;
    let mut out: Vec<Point> = Vec::with_capacity(path.len() + 1);
    // The drifting stretch, from the vertex where it starts (drift 0) to the end.
    let mut drift: Vec<Point> = Vec::new();
    for (k, &p) in path.iter().enumerate() {
        let d = along[k];
        if d <= from + CHAIN_TOL_MM {
            out.push(p);
            if (from - d).abs() <= CHAIN_TOL_MM {
                drift.push(p);
                continue;
            }
            // The stretch starts on the next chord: a vertex there, where the drift is 0.
            if k + 1 < path.len() && along[k + 1] > from + CHAIN_TOL_MM {
                let (a, b) = (p, path[k + 1]);
                let t = (from - d) / (along[k + 1] - d);
                let q = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                let v = match arcs.get(a, b) {
                    Some(circle) => {
                        let v = onto(circle, q);
                        arcs.insert(a, v, circle);
                        v
                    }
                    None => q,
                };
                out.push(v);
                drift.push(v);
            }
            continue;
        }
        let t = ((d - from) / stretch).clamp(0.0, 1.0);
        drift.push((p.0 + shift.0 * t, p.1 + shift.1 * t));
    }
    // Exactly on the next ring's start, whatever the float sums did.
    if let Some(last) = drift.last_mut() {
        *last = (ring[0].0 + shift.0, ring[0].1 + shift.1);
    }
    out.extend(fitted(&drift, arcs).into_iter().skip(1));
    out
}

/// `points` as the chords of the lines and arcs that fit them within the arc tolerance,
/// the arcs' chords tagged in `arcs`. Starts and ends on the given first and last points.
fn fitted(points: &[Point], arcs: &mut ArcTable) -> Vec<Point> {
    let pieces = fit_path(points, false, ARC_APPROX_TOL_MM, true);
    if pieces.is_empty() {
        return points.to_vec();
    }
    let mut out: Vec<Point> = vec![points[0]];
    for piece in &pieces {
        match piece {
            Primitive::Line(line) => out.push(line.end),
            Primitive::Arc(arc) => {
                let circle = Circle {
                    center: arc.center,
                    radius: arc.radius,
                };
                for p in piece.points().into_iter().skip(1) {
                    let last = out[out.len() - 1];
                    arcs.insert(last, p, circle);
                    out.push(p);
                }
            }
        }
    }
    // The fit's own end is a float away from the given one: the last chord ends exactly
    // there (and its tag, if any, is moved with it).
    if let (Some(&last), Some(&want)) = (points.last(), out.last())
        && last != want
    {
        let before = out[out.len() - 2];
        if let Some(circle) = arcs.get(before, want) {
            arcs.insert(before, last, circle);
        }
        let n = out.len();
        out[n - 1] = last;
    }
    out
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

/// Moving back at depth between passes is safe if the piece is cleared everywhere the tool
/// reaches and the move keeps the tool off every wall and island; any length, as it runs
/// through material already cut (docs/specs/pocket-stay-down.md §2).
fn back_link_is_safe(region: &Area, walls: &PrimIndex, a: Point, b: Point, offset: f64) -> bool {
    if distance(a, b) < CHAIN_TOL_MM {
        return true;
    }
    region.contains(a) && clear_of(walls, a, b, min_clearance(offset))
}

/// Every cutting move as a segment: ring chords (closed) and links fed at depth.
fn cutting_segments(rings: &[PocketRing], links: &[bool]) -> Vec<(Point, Point)> {
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
fn verify(
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
fn uncut_material(
    region: &Area,
    reachable: &[Area],
    rings: &[PocketRing],
    links: &[bool],
    radius: f64,
    ids: &[usize],
) -> (Option<Message>, Vec<Missed>) {
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
        return (None, missed);
    };
    let (x0, y0, x1, y1) = bounds(largest);
    // A point inside the piece: the centre of its biggest cell.
    let inside = largest
        .iter()
        .map(|&i| cells[i])
        .max_by_key(|c| Ordered(c.half));
    let Some(inside) = inside else {
        return (None, missed);
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
    (Some(note), missed)
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

/// What a pocket is cleared with (docs/specs/pocket.md): the shapes, the tool and how.
#[derive(Debug, Clone, Copy)]
pub struct PocketRequest<'a> {
    /// The selected closed contours: boundaries and the islands inside them.
    pub contours: &'a [&'a Contour],
    pub tool_radius: f64,
    /// Moves the cut into the kept material (docs/specs/depths-and-clearance.md): the tool
    /// centre runs `tool_radius - clearance` from the walls and islands.
    pub clearance: f64,
    /// Between rings, mm.
    pub stepover: f64,
    pub direction: Direction,
    pub spindle: Spindle,
    /// Where the tool comes from.
    pub start: Point,
    /// Start points, leads, helix, rounded corners and rest machining
    /// (docs/specs/helix-and-leads.md); none by default.
    pub choices: Choices<'a>,
}

impl<'a> PocketRequest<'a> {
    /// A pocket cut climb with a clockwise spindle from the origin, the tool centre
    /// `tool_radius` from the walls, with no choices made.
    pub fn new(contours: &'a [&'a Contour], tool_radius: f64, stepover: f64) -> Self {
        PocketRequest {
            contours,
            tool_radius,
            clearance: 0.0,
            stepover,
            direction: Direction::Climb,
            spindle: Spindle::Cw,
            start: (0.0, 0.0),
            choices: Choices::default(),
        }
    }
}

/// Where the wall rings start, their leads, and the helix the tool goes down round
/// (docs/specs/helix-and-leads.md); the default is none of them, and sharp rings.
#[derive(Debug, Clone, Copy, Default)]
pub struct Choices<'a> {
    /// Start points, moved with the copy (part A).
    pub starts: &'a [Point],
    /// Lead radius (part B).
    pub lead: Option<f64>,
    /// Helix radius (part C).
    pub helix: Option<f64>,
    /// Round the inner rings' corners (docs/specs/pocket-round-corners.md). Every project
    /// pocket has it; off is for comparing with sharp rings in tests.
    pub round_corners: bool,
    /// Rings step out onto the next as one spiral (docs/specs/pocket-spiral.md). Every
    /// project pocket has it; off is for tests that measure the rings themselves.
    pub spiral: bool,
    /// Stop with [`ToolpathError::Cancelled`] once set (docs/specs/background-generation.md).
    pub cancel: Option<&'a Cancel>,
    /// Rest machining: the radius of the larger tool whose leftovers are all this pocket
    /// clears (docs/specs/rest-machining.md).
    pub rest: Option<f64>,
}

/// One piece's paths before its links are chosen and it is verified.
struct Draft {
    region: Area,
    ids: Vec<usize>,
    rings: Vec<PocketRing>,
    level0: Vec<Area>,
}

/// Rings, cut order and links for every piece of the pocket `request` asks for.
pub fn plan_pocket(request: &PocketRequest) -> Result<PocketPlan, ToolpathError> {
    let PocketRequest {
        contours,
        tool_radius,
        clearance,
        stepover,
        direction,
        spindle,
        start,
        choices:
            Choices {
                starts,
                lead,
                helix,
                round_corners,
                spiral,
                cancel,
                rest,
            },
    } = *request;
    if stepover <= 0.0 || stepover.is_nan() {
        return Err(geometry(
            Message::new("toolpath-error-stepover")
                .arg("got", Value::Number(stepover, Digits::Shortest)),
        ));
    }
    let offset = tool_radius - clearance;
    if offset <= 0.0 || offset.is_nan() {
        return Err(geometry(
            Message::new("toolpath-clearance-radius")
                .arg("clearance", Value::Length(clearance, Digits::Shortest))
                .arg("radius", Value::Length(tool_radius, Digits::Shortest)),
        ));
    }
    let mut remaining = build_region(contours)?;
    if remaining.is_empty() {
        let mut ids: Vec<usize> = contours.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        return Err(geometry(
            Message::new("toolpath-error-nothing-to-clear").arg("contours", ids_text(&ids)),
        ));
    }
    let mut pieces = Vec::new();
    let mut notes = Vec::new();
    let mut arcs = ArcTable::default();
    // Rest machining: only the larger tool's leftovers are cleared.
    if let Some(large) = rest {
        let found = rest_pieces(&remaining, large, tool_radius, clearance, offset)?;
        remaining = found.pieces;
        notes.extend(found.notes);
    }
    let mut position = start;
    let mut drafts: Vec<Draft> = Vec::new();
    while !remaining.is_empty() {
        cancel::check(cancel)?;
        let here = position;
        // The nearest region; equally near ones (`nearest_ties`) by place and contours.
        let index =
            nearest_ties((0..remaining.len()).map(|i| (i, remaining[i].area.distance(here))))
                .into_iter()
                .min_by_key(|&i| {
                    (
                        remaining[i].area.sort_key(),
                        remaining[i].contour_ids.clone(),
                    )
                })
                .unwrap_or(0);
        let PocketArea {
            area: region,
            contour_ids: ids,
        } = remaining.remove(index);
        let clearing = Clearing {
            round_corners,
            offset,
            tool_radius,
            stepover,
            position,
            direction,
            spindle,
            cancel,
        };
        let Attempt {
            rings,
            level0,
            arcs: used_arcs,
            notes: used_notes,
        } = cleared_piece(&region, &ids, &clearing)?;
        arcs.extend(used_arcs);
        notes.extend(used_notes);
        if level0.len() > 1 {
            notes.push(
                Message::new("toolpath-pocket-narrow")
                    .arg("contours", ids_text(&ids))
                    .arg("areas", level0.len()),
            );
        }
        position = rings[0].points[0]; // every depth starts again at the first ring
        drafts.push(Draft {
            region,
            ids,
            rings,
            level0,
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
        } = draft;
        let walls = PrimIndex::new(region.boundary(), tool_radius.max(1.0));
        if let Some(radius) = lead {
            // Climb with M3 keeps the walls on the right: the cleared side is on the left.
            let waste_left = !cut_clockwise(Side::Inside, direction, spindle);
            let fits =
                |l: &Leads| l.all_points(|p| region.contains(p)) && l.clear_of(&walls, offset);
            let min_radius = 2.0 * tool_radius * MIN_LEAD_DIAMETERS;
            for ring in rings.iter_mut().filter(|r| r.wall) {
                if ring.chosen.is_none()
                    && let Some(at) = roomiest_start(&ring.points)
                {
                    ring.points = restart(&ring.points, at, &mut arcs);
                }
                match fitted_leads(&ring.points, radius, min_radius, waste_left, fits) {
                    Some((l, used)) => {
                        if used < radius {
                            notes.push(
                                Message::new("toolpath-lead-reduced")
                                    .arg("contours", ids_text(&ids))
                                    .arg("used", fixed3(used))
                                    .arg("radius", fixed3(radius)),
                            );
                        }
                        l.tag(&mut arcs);
                        let plain = std::mem::take(&mut ring.points);
                        let before = l.lead_in.len() - 1;
                        let mut path = l.lead_in[..before].to_vec();
                        path.extend(plain.iter().copied());
                        path.extend(l.lead_out.iter().copied());
                        ring.lead = Some(before..before + plain.len());
                        ring.points = path;
                        ring.open = true;
                    }
                    None => notes.push(
                        Message::new("toolpath-no-lead")
                            .arg("contours", ids_text(&ids))
                            .arg("least", fixed3(min_radius)),
                    ),
                }
            }
        }
        // Rings step out onto the next where they can (docs/specs/pocket-spiral.md); if
        // that leaves material the uncut check finds, the piece is cut with closed rings
        // and links instead, as before.
        let plain = rings.clone();
        let spiralled = spiral
            && step_out(
                &mut rings,
                &mut arcs,
                stepover,
                2.0 * tool_radius,
                &region,
                &walls,
                offset,
            );
        let linked = |rings: &[PocketRing]| -> Result<(Vec<bool>, Option<Message>), ToolpathError> {
            // Between rings the tool stays down wherever the link is safe (docs/specs/pocket.md
            // as built 2026-10-15): verified with the rings.
            let links: Vec<bool> = rings
                .windows(2)
                .map(|pair| {
                    pair[0].end().is_some_and(|from| {
                        link_is_safe(
                            &region,
                            &walls,
                            from,
                            pair[1].points[0],
                            tool_radius,
                            offset,
                        )
                    })
                })
                .collect();
            verify(&region, &walls, rings, &links, offset, &ids, &arcs)?;
            let (uncut, _) = uncut_material(&region, &level0, rings, &links, tool_radius, &ids);
            Ok((links, uncut))
        };
        let (mut links, mut uncut) = linked(&rings)?;
        if spiralled && uncut.is_some() {
            let (plain_links, plain_uncut) = linked(&plain)?;
            if plain_uncut.is_none() {
                rings = plain;
                links = plain_links;
                uncut = plain_uncut;
            }
        }
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
        cancel::check(cancel)?;
        let next_start = helix
            .as_ref()
            .map(|h| h.start)
            .or_else(|| rings.first().and_then(|r| r.points.first().copied()));
        let back_link = uncut.is_none()
            && match (rings.last().and_then(PocketRing::end), next_start) {
                (Some(from), Some(to)) => back_link_is_safe(&region, &walls, from, to, offset),
                _ => false,
            };
        notes.extend(uncut);
        pieces.push(PocketPiece {
            rings,
            feed_links: links,
            contour_ids: ids,
            helix,
            back_link,
        });
    }
    Ok(PocketPlan {
        pieces,
        notes,
        arcs,
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
    rings: &[PocketRing],
    ids: &[usize],
    notes: &mut Vec<Message>,
) -> Result<Option<Helix>, ToolpathError> {
    let Some(&first) = rings.first().and_then(|r| r.points.first()) else {
        return Ok(None);
    };
    let min_radius = 0.2 * fit.tool_radius; // a tenth of the diameter
    let without = |why: Message| why.arg("contours", ids_text(ids));
    // The areas a helix centre of radius r may lie in.
    let room = |r: f64| fit.region.shrunk(fit.offset + r);
    let mut used = radius;
    if room(radius)?.is_empty() {
        if room(min_radius)?.is_empty() {
            notes.push(without(
                Message::new("toolpath-no-helix-room").arg("least", fixed3(min_radius)),
            ));
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
        notes.push(
            Message::new("toolpath-helix-reduced")
                .arg("contours", ids_text(ids))
                .arg("used", fixed3(used))
                .arg("radius", fixed3(radius)),
        );
    }
    let areas = room(used)?;
    let nearest = nearest_ties(areas.iter().map(|a| (a, a.distance(first))))
        .into_iter()
        .min_by_key(|a| a.sort_key());
    let Some(nearest) = nearest else {
        notes.push(without(Message::new("toolpath-no-helix-narrow")));
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
        notes.push(without(Message::new("toolpath-no-helix-way")));
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
    notes: &mut Vec<Message>,
) {
    if starts.is_empty() {
        return;
    }
    let mut at: Vec<(usize, usize)> = Vec::new();
    for (d, draft) in drafts.iter().enumerate() {
        at.extend(
            (0..draft.rings.len())
                .filter(|&k| draft.rings[k].wall)
                .map(|k| (d, k)),
        );
    }
    let rings: Vec<&[Point]> = at
        .iter()
        .map(|&(d, k)| drafts[d].rings[k].points.as_slice())
        .collect();
    let (chosen, duplicates) = assign(starts, &rings);
    for (&(d, k), choice) in at.iter().zip(&chosen) {
        let Some(choice) = choice else { continue };
        let ring = &mut drafts[d].rings[k];
        ring.points = restart(&ring.points, choice.stored, arcs);
        ring.chosen = Some(choice.index);
        if choice.distance > far {
            notes.push(
                Message::new("toolpath-start-far-wall")
                    .arg("contours", ids_text(&drafts[d].ids))
                    .arg("point", point_value(choice.stored))
                    .arg("distance", fixed3(choice.distance)),
            );
        }
    }
    for (index, r) in duplicates {
        let (d, _) = at[r];
        notes.push(
            Message::new("toolpath-start-twice-wall")
                .arg("contours", ids_text(&drafts[d].ids))
                .arg("point", point_value(starts[index])),
        );
    }
}

/// What a rest pocket clears (docs/specs/rest-machining.md §3).
struct RestPieces {
    /// The areas to pocket, each with its region's contours.
    pieces: Vec<PocketArea>,
    /// The larger tool's leftovers in every region.
    left: Vec<Area>,
    notes: Vec<Message>,
}

/// The areas a rest pocket clears in each region: within a diameter of what a tool of
/// radius `large` leaves. An area this pocket's tool fits nowhere in is skipped, with a
/// note.
fn rest_pieces(
    regions: &[PocketArea],
    large: f64,
    tool_radius: f64,
    clearance: f64,
    offset: f64,
) -> Result<RestPieces, ToolpathError> {
    let mut found = RestPieces {
        pieces: Vec::new(),
        left: Vec::new(),
        notes: Vec::new(),
    };
    // (width, height) of the leftovers too small for this tool too.
    let mut skipped: Vec<(f64, f64)> = Vec::new();
    for region in regions {
        let level0 = shrunk_farther(&region.area, large - clearance)?;
        let mut material = Vec::new();
        for target in cleared(&region.area, clearance)? {
            material.extend(left_by(&target, &level0, large)?);
        }
        for area in rest_areas(&region.area, &material, 2.0 * tool_radius)? {
            if shrunk_farther(&area, offset)?.is_empty() {
                skipped.extend(
                    material
                        .iter()
                        .filter(|m| area.distance(first_point(&m.outer)) <= ARC_APPROX_TOL_MM)
                        .filter_map(|m| m.outer.extents())
                        .map(|e| (e.max_x - e.min_x, e.max_y - e.min_y)),
                );
                continue;
            }
            found.pieces.push(PocketArea {
                area,
                contour_ids: region.contour_ids.clone(),
            });
        }
        found.left.extend(material);
    }
    if found.left.is_empty() {
        found.notes.push(Message::new("toolpath-rest-nothing-left"));
    }
    if let Some(&(w, h)) = skipped.iter().max_by_key(|&&(w, h)| Ordered(w * h)) {
        found.notes.push(
            Message::new("toolpath-rest-too-small")
                .arg("count", skipped.len())
                .arg("width", Value::LengthNumber(w, Digits::Fixed(2)))
                .arg("height", Value::Length(h, Digits::Fixed(2))),
        );
    }
    Ok(found)
}

/// The depths and feeds a piece is cut at (docs/specs/ramp-entry.md).
#[derive(Debug, Clone, Copy)]
pub struct PocketCut<'a> {
    /// The Z of each depth pass, going down.
    pub pass_z: &'a [f64],
    /// The Z of the start depth, cut already: where every descent begins.
    pub top_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    /// How the tool goes down into each pass, along the path that follows.
    pub entry: Entry,
}

/// All depth passes of one piece. Each pass starts at the centre and ends retracted.
pub fn pocket_moves(piece: &PocketPiece, cut: &PocketCut) -> Result<Vec<Move>, ToolpathError> {
    let PocketCut {
        pass_z,
        top_z,
        safe_z,
        feed,
        plunge_feed,
        entry,
    } = *cut;
    let mut moves = Vec::new();
    let Some(first) = piece.rings.first().and_then(|r| r.points.first().copied()) else {
        return Ok(moves);
    };
    // The path the tool follows from a ring's start, to ramp along.
    let path_of = |ring: &PocketRing| {
        if ring.open {
            ring.points.clone()
        } else {
            crate::profile::closed_path(&ring.points)
        }
    };
    let descend = |ring: &PocketRing, from_z: f64, cleared: f64, z: f64| {
        let path = path_of(ring);
        let length = crate::ramp::polyline_length(&path);
        crate::ramp::enter(&path, length, from_z, cleared, z, entry, feed, plunge_feed)
    };
    let mut cleared = top_z;
    // Where the tool is before a pass: at safe Z, or lifted off the floor of the pass
    // before (docs/specs/pocket-stay-down.md).
    let mut from_z = safe_z;
    for (pass, &z) in pass_z.iter().enumerate() {
        let down = from_z < safe_z;
        match (piece.helix, entry) {
            (Some(helix), Entry::Helix { ramp_angle, .. }) => {
                // Down round the helix, then at depth to the first path (checked when the
                // helix was fitted).
                if down {
                    moves.push(Move::feed_xy(helix.start, feed));
                } else {
                    moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(helix.start)]);
                }
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
                if down {
                    moves.push(Move::feed_xy(first, feed));
                } else {
                    moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(first)]);
                }
                moves.extend(descend(&piece.rings[0], from_z, cleared, z)?);
            }
        }
        for (index, ring) in piece.rings.iter().enumerate() {
            let start = ring.points[0];
            if index > 0 {
                if piece.feed_links[index - 1] {
                    // A ring that stepped out already ends on this start: no link move.
                    if piece.rings[index - 1].end() != Some(start) {
                        moves.push(Move::feed_xy(start, feed));
                    }
                } else {
                    moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(start)]);
                    moves.extend(descend(ring, safe_z, cleared, z)?);
                }
            }
            moves.extend(ring.points[1..].iter().map(|&p| Move::feed_xy(p, feed)));
            if !ring.open {
                moves.push(Move::feed_xy(start, feed));
            }
        }
        cleared = z;
        if piece.back_link && pass + 1 < pass_z.len() {
            from_z = (z + BACK_LINK_LIFT_MM).min(safe_z);
            moves.push(Move::feed_z(from_z, plunge_feed));
        } else {
            from_z = safe_z;
            moves.push(Move::rapid_z(safe_z));
        }
    }
    Ok(moves)
}
