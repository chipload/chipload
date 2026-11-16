// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounded rings, each clearing attempt verified, the stepover shrunk until it is.

use chipload_core::Point;
use chipload_core::order::Ordered;
use chipload_core::primitives::{distance, signed_area};
use chipload_core::tolerances::ARC_APPROX_TOL_MM;
use chipload_i18n::Message;
use chipload_job::{Direction, Spindle};

use super::order::{Reach, cut_order};
use super::region::{Node, ring_tree};
use super::verify::{clear_of, cutting_segments, min_clearance, uncut_material};
use super::{PocketRing, ids_text};
use crate::ToolpathError;
use crate::arcs::{ArcTable, arc_midpoint};
use crate::cancel::{self, Cancel};
use crate::cavalier::{Area, geometry};
use crate::geom::PrimIndex;
use crate::retry::{Failure, Nudge, nudged_or_tight};
use crate::start::nearest_ties;

/// Rounds of the uncut check, each halving the radii of the rings near material left.
const ROUNDING_ROUNDS: usize = 12;
/// The uncut-material check looks at the cancel flag after this many cells.
pub(super) const UNCUT_CELLS_PER_CHECK: usize = 256;

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
pub(super) struct Attempt {
    pub(super) rings: Vec<PocketRing>,
    pub(super) level0: Vec<Area>,
    pub(super) arcs: ArcTable,
    pub(super) notes: Vec<Message>,
}

/// What a piece is cleared with, and where the tool comes from.
pub(super) struct Clearing<'a> {
    /// Round the corners of the inner rings (off only to compare, in tests).
    pub(super) round_corners: bool,
    pub(super) offset: f64,
    pub(super) tool_radius: f64,
    pub(super) stepover: f64,
    pub(super) position: Point,
    pub(super) direction: Direction,
    pub(super) spindle: Spindle,
    pub(super) cancel: Option<&'a Cancel>,
}

/// The paths of one piece, checked as soon as they are built and retried with nudged
/// offsets after a numerical failure (docs/specs/offset-retry.md).
pub(super) fn cleared_piece(
    region: &Area,
    ids: &[usize],
    c: &Clearing,
) -> Result<Attempt, ToolpathError> {
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
    let (mut nodes, roots) = ring_tree(
        region,
        c.offset,
        c.tool_radius,
        c.stepover,
        ids,
        nudge,
        c.cancel,
    )?;
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
        let (note, missed) =
            uncut_material(region, &level0, &rings, &[], c.tool_radius, ids, c.cancel)?;
        Ok::<_, ToolpathError>(note.map(|_| missed))
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
        let Some(left) = missed(nodes)? else {
            return Ok(());
        };
        // The uncut check and the search below are most of a big pocket's time: a cancel is
        // seen between them and ring by ring (docs/specs/background-generation.md).
        cancel::check(c.cancel)?;
        let mut changed = false;
        let points: Vec<Point> = left
            .iter()
            .flat_map(|piece| piece.iter().flatten().copied())
            .collect();
        for (i, node) in nodes.iter().enumerate() {
            cancel::check(c.cancel)?;
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
    cancel::check(c.cancel)?;
    if missed(nodes)?.is_none() {
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
pub(super) fn segment_is_clear(
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

pub(super) fn verification_failed(ids: &[usize]) -> ToolpathError {
    geometry(Message::new("toolpath-error-pocket-verification").arg("contours", ids_text(ids)))
}

/// Order for the travel direction and start at the vertex nearest `near` (ties, as
/// `nearest_ties` has them: y, x).
pub(super) fn orient_from(ring: &[Point], clockwise: bool, near: Point) -> Vec<Point> {
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
