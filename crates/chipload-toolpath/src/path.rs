// SPDX-License-Identifier: GPL-3.0-or-later
//! Follow path (docs/specs/follow-path.md): the tool centre follows the selected shapes on
//! the line. Open paths are cut back and forth, one traversal per depth (a plunge at each
//! end) or descending while they travel (a ramp); closed ones are cut as a profile "on" the
//! line.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Direction, Entry, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::{ArcTable, Circle};
use crate::lap::{RingCut, ring_moves};
use crate::model::Move;
use crate::offset::{contour_pline, flatten_tagged};
use crate::profile::{cut_clockwise, orient};
use crate::ramp::{MAX_RAMP_LEGS, polyline_length};

/// One shape to follow: its points in cutting order (a closed one without the repeated
/// start), and whether it is closed.
#[derive(Debug, Clone, PartialEq)]
pub struct FollowedPath {
    pub points: Vec<Point>,
    pub closed: bool,
    pub contour_id: usize,
}

/// Everything a follow path needs besides its shapes.
#[derive(Debug, Clone, Copy)]
pub struct PathCut<'a> {
    pub pass_z: &'a [f64],
    /// The Z of the start depth, cut already.
    pub top_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    pub entry: Entry,
    pub tool_diameter: f64,
    pub spindle: Spindle,
}

/// The shapes as paths, in cutting order from `start`: each time the nearest, an open one
/// from its nearer end (docs/specs/follow-path.md §3). Shapes shorter than the chaining
/// tolerance are left out with a note. Chords on arcs are recorded in `arcs`.
pub fn order_paths(
    contours: &[&Contour],
    start: Point,
    spindle: Spindle,
    arcs: &mut ArcTable,
    notes: &mut Vec<chipload_i18n::Message>,
) -> Vec<FollowedPath> {
    let mut remaining: Vec<FollowedPath> = Vec::new();
    for contour in contours {
        let points = if contour.closed {
            let ring = flatten_tagged(&contour_pline(contour), arcs);
            // As a profile on the line, climb milling: the same way round and start.
            orient(&ring, cut_clockwise(Side::On, Direction::Climb, spindle))
        } else {
            open_points(contour, arcs)
        };
        let length = if contour.closed {
            crate::tabs::ring_length(&points)
        } else {
            polyline_length(&points)
        };
        if points.len() < 2 || length <= CHAIN_TOL_MM {
            notes.push(
                chipload_i18n::Message::new("toolpath-path-too-short")
                    .arg("contour", contour.id)
                    .arg(
                        "length",
                        chipload_i18n::Value::Length(length, chipload_i18n::Digits::Fixed(4)),
                    ),
            );
            continue;
        }
        remaining.push(FollowedPath {
            points,
            closed: contour.closed,
            contour_id: contour.id,
        });
    }
    let mut ordered = Vec::with_capacity(remaining.len());
    let mut here = start;
    while !remaining.is_empty() {
        // (distance, reverse, index): the nearest start, then the lowest contour id.
        let mut best: Option<(f64, bool, usize)> = None;
        for (i, path) in remaining.iter().enumerate() {
            let first = distance(here, path.points[0]);
            let last = path
                .points
                .last()
                .map_or(f64::INFINITY, |&p| distance(here, p));
            let (d, reverse) = if !path.closed && last < first {
                (last, true)
            } else {
                (first, false)
            };
            let better = match best {
                None => true,
                Some((bd, _, bi)) => {
                    d < bd || (d == bd && path.contour_id < remaining[bi].contour_id)
                }
            };
            if better {
                best = Some((d, reverse, i));
            }
        }
        let Some((_, reverse, index)) = best else {
            break;
        };
        let mut path = remaining.remove(index);
        if reverse {
            path.points.reverse();
        }
        here = if path.closed {
            path.points[0]
        } else {
            path.points.last().copied().unwrap_or(here)
        };
        ordered.push(path);
    }
    ordered
}

/// The points of an open contour in drawn order, arcs flattened within the arc tolerance,
/// points closer than the chaining tolerance merged; chords on an arc recorded in `arcs`.
fn open_points(contour: &Contour, arcs: &mut ArcTable) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::new();
    for part in &contour.parts {
        let circle = match part.segment.prim {
            Primitive::Arc(arc) => Some(Circle {
                center: arc.center,
                radius: arc.radius,
            }),
            Primitive::Line(_) => None,
        };
        for (k, p) in part.points().into_iter().enumerate() {
            let Some(&last) = points.last() else {
                points.push(p);
                continue;
            };
            if distance(last, p) <= CHAIN_TOL_MM {
                continue;
            }
            if k > 0
                && let Some(circle) = circle
            {
                arcs.insert(last, p, circle);
            }
            points.push(p);
        }
    }
    points
}

/// The moves for one path, from a retract above its start to a retract above its end; and
/// that end.
pub fn path_moves(
    path: &FollowedPath,
    cut: &PathCut,
    arcs: &ArcTable,
) -> Result<(Vec<Move>, Point), ToolpathError> {
    if path.closed {
        let ring = RingCut {
            ring: &path.points,
            tabs: None,
            pass_z: cut.pass_z,
            top_z: cut.top_z,
            safe_z: cut.safe_z,
            feed: cut.feed,
            plunge_feed: cut.plunge_feed,
            ramp_angle: cut.entry.ramp_angle(),
            tool_diameter: cut.tool_diameter,
        };
        let done = ring_moves(&ring, arcs)?;
        return Ok((done.moves, done.end));
    }
    open_moves(&path.points, cut, arcs)
}

/// An open path, back and forth (docs/specs/follow-path.md §4).
fn open_moves(
    points: &[Point],
    cut: &PathCut,
    arcs: &ArcTable,
) -> Result<(Vec<Move>, Point), ToolpathError> {
    let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
        return Ok((Vec::new(), (0.0, 0.0)));
    };
    let length = polyline_length(points);
    let mut moves = vec![Move::rapid_z(cut.safe_z), Move::rapid_xy(first)];
    // Traversals, alternately forward and back: (Z at the start, Z at the end).
    let mut traversals: Vec<(f64, f64)> = Vec::new();
    match cut.entry.ramp_angle() {
        None => {
            for &z in cut.pass_z {
                traversals.push((z, z));
            }
        }
        Some(ramp_angle) => {
            moves.push(Move::feed_z(cut.top_z, cut.plunge_feed));
            let mut z = cut.top_z;
            for &pass in cut.pass_z {
                let drop = z - pass;
                if drop <= DEPTH_TOL_MM {
                    continue;
                }
                let needed = ((drop / ramp_angle.tan() - CHAIN_TOL_MM) / length)
                    .ceil()
                    .max(1.0);
                if needed > MAX_RAMP_LEGS as f64 {
                    return Err(ToolpathError::Geometry(
                        Message::new("toolpath-error-path-ramp")
                            .arg("drop", Value::Length(drop, Digits::Fixed(3)))
                            .arg(
                                "angle",
                                Value::Number(ramp_angle.to_degrees(), Digits::Fixed(1)),
                            )
                            .arg("length", Value::Length(length, Digits::Fixed(3))),
                    ));
                }
                // Small positive whole number (checked above).
                let count = needed as usize;
                for k in 0..count {
                    let from = z - drop * k as f64 / count as f64;
                    let to = if k + 1 == count {
                        pass
                    } else {
                        z - drop * (k + 1) as f64 / count as f64
                    };
                    traversals.push((from, to));
                }
                z = pass;
            }
            // One level traversal cuts the last slope away.
            traversals.push((z, z));
        }
    }
    let mut forward = true;
    let mut here = first;
    // Where the tool is in Z: above the path until the first descent.
    let mut current = moves.last().and_then(|m| m.z).unwrap_or(cut.safe_z);
    for (from, to) in traversals {
        let order: Vec<Point> = if forward {
            points.to_vec()
        } else {
            points.iter().rev().copied().collect()
        };
        if current != from {
            // A plunge entry: straight down at the end where the tool is.
            moves.push(Move::feed_z(from, cut.plunge_feed));
        }
        let mut travelled = 0.0;
        let steps = order.len() - 1;
        for (k, &p) in order[1..].iter().enumerate() {
            travelled += distance(here, p);
            let mut m = if from == to {
                Move::feed_xy(p, cut.feed)
            } else {
                // The traversal's last point is exactly at its end depth.
                let z = if k + 1 == steps {
                    to
                } else {
                    from + (to - from) * travelled / length
                };
                Move::feed_xyz(p, z, cut.feed)
            };
            m.circle = arcs.get(here, p);
            moves.push(m);
            here = p;
        }
        current = to;
        forward = !forward;
    }
    moves.push(Move::rapid_z(cut.safe_z));
    let end = if forward { first } else { last };
    Ok((moves, end))
}
