// SPDX-License-Identifier: GPL-3.0-or-later
//! A pocket's finishing pass (docs/specs/pocket-finish.md): the roughing leaves an allowance
//! on the walls and the floor; then, for each piece, one level pass takes the floor and one
//! lap at full depth takes each wall. This module holds the wall laps and the check that the
//! roughing reached every wall; the floor pass is the roughing's rings once more, at the end
//! depth (`pocket_moves`).

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Message, Value};

use crate::ToolpathError;
use crate::lap::{farthest_point_from, nearest_on_rings};
use crate::model::Move;
use crate::pocket::{PocketPiece, PocketRing};
use crate::start::point_value;

/// Where and how fast a finishing lap cuts.
#[derive(Debug, Clone, Copy)]
pub struct LapCut {
    /// The end depth's Z: the lap's, and the roughed area's around it.
    pub z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
}

/// The rings of a roughing piece the tool can go down beside, as closed rings: the closed
/// rings, and the wall rings without their leads. Open paths are left out, as closing them
/// would cross material.
pub fn slot_rings(piece: &PocketPiece) -> Vec<&[Point]> {
    piece
        .rings
        .iter()
        .filter(|r| !r.open || r.lead.is_some())
        .map(PocketRing::plain)
        .filter(|r| r.len() > 1)
        .collect()
}

/// The finishing lap of one wall ring (docs/specs/pocket-finish.md §3): down in the roughed
/// area, onto the wall (along the lead, or straight across from the nearest roughing ring),
/// one level lap, off the same way, retract. Returns the moves and where the tool ends.
pub fn finish_lap(ring: &PocketRing, slot: &[&[Point]], cut: &LapCut) -> (Vec<Move>, Point) {
    let Some(&first) = ring.points.first() else {
        return (Vec::new(), (0.0, 0.0));
    };
    // A lead starts and ends in the cleared area already.
    let beside = |p: Point| match ring.lead {
        Some(_) => p,
        None => nearest_on_rings(slot, p).map_or(p, |(q, _)| q),
    };
    let into = beside(first);
    let mut moves = vec![
        Move::rapid_z(cut.safe_z),
        Move::rapid_xy(into),
        Move::feed_z(cut.z, cut.plunge_feed),
    ];
    if distance(into, first) > CHAIN_TOL_MM {
        moves.push(Move::feed_xy(first, cut.feed));
    }
    moves.extend(ring.points[1..].iter().map(|&p| Move::feed_xy(p, cut.feed)));
    if !ring.open {
        moves.push(Move::feed_xy(first, cut.feed));
    }
    let end = ring.end().unwrap_or(first);
    let out = beside(end);
    if distance(end, out) > CHAIN_TOL_MM {
        moves.push(Move::feed_xy(out, cut.feed));
    }
    moves.push(Move::rapid_z(cut.safe_z));
    (moves, out)
}

/// The wall lap must run beside the roughing (docs/specs/pocket-finish.md §5): refused where
/// it is a tool radius or more from the roughing rings (the roughing couldn't get there with
/// the allowance), a note where it cuts more than twice the allowance.
pub fn reach_note(
    ring: &PocketRing,
    slot: &[&[Point]],
    tool_radius: f64,
    wall: f64,
) -> Result<Option<Message>, ToolpathError> {
    let (farthest, at) = farthest_point_from(ring.plain(), slot, tool_radius / 2.0);
    if farthest >= tool_radius {
        return Err(narrow(at, wall));
    }
    Ok((farthest > 2.0 * wall + CHAIN_TOL_MM).then(|| {
        Message::new("toolpath-finish-lap-cuts")
            .arg("most", Value::mm(farthest))
            .arg("stock", Value::mm(wall))
    }))
}

/// The pocket is too narrow near `at` to rough with `wall` mm left on the walls.
pub fn narrow(at: Point, wall: f64) -> ToolpathError {
    ToolpathError::Geometry(
        Message::new("toolpath-pocket-finish-narrow")
            .arg("at", point_value(at))
            .arg("wall", Value::mm(wall)),
    )
}
