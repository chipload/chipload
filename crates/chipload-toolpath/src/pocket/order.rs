// SPDX-License-Identifier: GPL-3.0-or-later
//! Rings into loops in cut order, stepped out from one to the next.

use crate::arcs::{ArcTable, Circle};
use crate::cavalier::Area;
use crate::geom::PrimIndex;
use crate::offset::{flatten_tagged, pline_segments};
use crate::profile::cut_clockwise;
use crate::start::{nearest_ties, onto, restart};
use cavalier_contours::polyline::Polyline;
use chipload_core::Point;
use chipload_core::fit::fit_path;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_job::{Direction, Side, Spindle};

use super::PocketRing;
use super::region::Node;
use super::rounding::orient_from;
use super::verify::{clear_of, min_clearance};

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
pub(super) fn cut_order(
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
        let mut ring = PocketRing::closed(ring, l.level == 0);
        ring.full_width = l.deps.is_empty();
        rings.push(ring);
        done[next] = true;
        last = Some(next);
    }
    rings
}

/// How far apart two rings one level apart may be and still have to be cut in order: a tool
/// diameter, and at least 1.5 stepovers (docs/specs/pocket-island-order.md §2).
#[derive(Clone, Copy)]
pub(super) struct Reach(f64);

impl Reach {
    pub(super) fn new(tool_radius: f64, stepover: f64) -> Self {
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
pub(super) fn step_out(
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
        let closed = std::mem::replace(&mut rings[i].points, path);
        rings[i].unstepped = Some(closed);
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
