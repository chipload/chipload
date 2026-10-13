// SPDX-License-Identifier: GPL-3.0-or-later
//! Holes (docs/specs/holes.md): every selected circle, drilled at its centre with a drill
//! (straight down, in pecks) or bored with an endmill (a helix at the wall, then one level
//! circle).

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_job::{Direction, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::Circle;
use crate::helix::{Helix, helix_moves};
use crate::model::Move;
use crate::profile::cut_clockwise;

/// How far a drill's diameter may differ from a circle's for the drill to cut it, mm
/// (docs/specs/holes.md decision 2).
pub const DRILL_FIT_MM: f64 = 0.1;

/// How far above the depth already drilled a peck's rapid back into the hole stops, mm
/// (docs/specs/holes.md §3). The checker keeps its own copy.
pub const PECK_CLEARANCE_MM: f64 = 0.5;

/// A round hole of the drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hole {
    pub center: Point,
    pub radius: f64,
    pub contour_id: usize,
}

/// The circle a contour is: closed, made only of arcs with one centre and one radius
/// (within the chaining tolerance) that sweep one full turn. None for anything else.
pub fn circle_of(contour: &Contour) -> Option<Hole> {
    if !contour.closed {
        return None;
    }
    let mut first: Option<(Point, f64)> = None;
    let mut sweep = 0.0;
    for part in &contour.parts {
        let Primitive::Arc(arc) = part.segment.prim else {
            return None;
        };
        match first {
            None => first = Some((arc.center, arc.radius)),
            Some((center, radius)) => {
                if distance(center, arc.center) > CHAIN_TOL_MM
                    || (radius - arc.radius).abs() > CHAIN_TOL_MM
                {
                    return None;
                }
            }
        }
        sweep += arc.sweep.abs();
    }
    let (center, radius) = first?;
    // One full turn, measured along the circle.
    if ((sweep - TAU) * radius).abs() > CHAIN_TOL_MM {
        return None;
    }
    Some(Hole {
        center,
        radius,
        contour_id: contour.id,
    })
}

/// The circles among `contours`, and how many contours aren't circles.
pub fn find_holes(contours: &[&Contour]) -> (Vec<Hole>, usize) {
    let mut holes = Vec::new();
    let mut skipped = 0;
    for contour in contours {
        match circle_of(contour) {
            Some(hole) => holes.push(hole),
            None => skipped += 1,
        }
    }
    (holes, skipped)
}

/// The holes in cutting order from `start`: each time the nearest centre, the lowest contour
/// id on a tie (docs/specs/holes.md §5).
pub fn order_holes(mut remaining: Vec<Hole>, start: Point) -> Vec<Hole> {
    let mut ordered = Vec::with_capacity(remaining.len());
    let mut here = start;
    while !remaining.is_empty() {
        let mut best = 0;
        for (i, hole) in remaining.iter().enumerate() {
            let d = distance(here, hole.center);
            let b = distance(here, remaining[best].center);
            if d < b || (d == b && hole.contour_id < remaining[best].contour_id) {
                best = i;
            }
        }
        let hole = remaining.remove(best);
        here = hole.center;
        ordered.push(hole);
    }
    ordered
}

/// Everything a hole needs besides its circle.
#[derive(Debug, Clone, Copy)]
pub struct HoleCut {
    /// Z of the start depth (cut already, or the stock top).
    pub top_z: f64,
    /// Z of the end depth.
    pub end_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    pub tool_radius: f64,
}

/// A drill cycle as plain moves (docs/specs/holes.md §3), from above the hole to above it:
/// straight down at the centre, `pecks` (Z of each peck's bottom, the last the end depth),
/// the tool rising to the safe height between pecks and rapiding back into the hole to
/// [`PECK_CLEARANCE_MM`] above the depth drilled.
pub fn drill_moves(center: Point, pecks: &[f64], cut: &HoleCut) -> Vec<Move> {
    let mut moves = vec![Move::rapid_z(cut.safe_z), Move::rapid_xy(center)];
    let mut drilled: Option<f64> = None;
    for &z in pecks {
        if let Some(bottom) = drilled {
            moves.push(Move::rapid_z(cut.safe_z));
            let back = bottom + PECK_CLEARANCE_MM;
            if back < cut.safe_z {
                moves.push(Move::rapid_z(back));
            }
        }
        moves.push(Move::feed_z(z, cut.plunge_feed));
        drilled = Some(z);
    }
    moves.push(Move::rapid_z(cut.safe_z));
    moves
}

/// The circle the tool centre follows to bore `hole`: radius hole radius minus tool radius.
pub fn bore_radius(hole: &Hole, tool_radius: f64) -> f64 {
    hole.radius - tool_radius
}

/// How a bore goes down.
#[derive(Debug, Clone, Copy)]
pub struct Bore {
    pub step_down: f64,
    pub ramp_angle: f64,
    pub direction: Direction,
    pub spindle: Spindle,
}

/// A helical bore (docs/specs/holes.md §4), from above the hole to above it: down at the
/// helix's start to the start depth, a helix at most `step_down` per turn and never
/// steeper than the ramp angle to the end depth, one level circle, then off the wall
/// toward the centre (only when the centre is cut: no core) and up.
pub fn bore_moves(hole: &Hole, bore: &Bore, cut: &HoleCut) -> Result<Vec<Move>, ToolpathError> {
    let rc = bore_radius(hole, cut.tool_radius);
    let start = (hole.center.0 + rc, hole.center.1);
    let helix = Helix {
        circle: Circle {
            center: hole.center,
            radius: rc,
        },
        start,
        // A hole is an inside contour: the same way round as an inside profile.
        clockwise: cut_clockwise(Side::Inside, bore.direction, bore.spindle),
    };
    // At most `step_down` per turn: no steeper than that turn's slope.
    let angle = bore.ramp_angle.min((bore.step_down / (TAU * rc)).atan());
    let mut moves = vec![Move::rapid_z(cut.safe_z), Move::rapid_xy(start)];
    moves.extend(helix_moves(
        &helix,
        cut.top_z,
        cut.end_z,
        angle,
        cut.feed,
        cut.plunge_feed,
    )?);
    // With the centre cut there is room to step off the wall; with a core, straight up
    // at the wall, as a profile retracts.
    if rc <= cut.tool_radius + CHAIN_TOL_MM {
        let off = rc.min(cut.tool_radius / 2.0);
        moves.push(Move::feed_xy((start.0 - off, start.1), cut.feed));
    }
    moves.push(Move::rapid_z(cut.safe_z));
    Ok(moves)
}
