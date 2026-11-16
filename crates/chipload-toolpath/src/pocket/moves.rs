// SPDX-License-Identifier: GPL-3.0-or-later
//! A planned pocket piece into moves: rings, links and slot laps.

use chipload_core::Point;
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_job::Entry;

use super::{BACK_LINK_LIFT_MM, PocketPiece, PocketRing};
use crate::ToolpathError;
use crate::helix::helix_moves;
use crate::model::Move;

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
    /// The feed of the full-width rings (docs/specs/cutting-data.md §4).
    pub slot_feed: f64,
    /// The deepest each step of a full-width ring goes, mm; None: down to the pass at once.
    pub slot_step: Option<f64>,
}

/// The depths a full-width ring goes down to from `cleared` to the pass at `z`: equal steps
/// no deeper than `step`, the last exactly `z` (docs/specs/cutting-data.md §4).
fn slot_levels(cleared: f64, z: f64, step: Option<f64>) -> Vec<f64> {
    let range = cleared - z;
    let Some(step) = step.filter(|s| *s > 0.0 && range - DEPTH_TOL_MM > *s) else {
        return vec![z];
    };
    // Small positive whole number: depths and step downs are validated job values.
    let count = ((range - DEPTH_TOL_MM) / step).ceil().max(1.0) as u32;
    (1..=count)
        .map(|k| {
            if k == count {
                z
            } else {
                cleared - range * f64::from(k) / f64::from(count)
            }
        })
        .collect()
}

/// One lap of a full-width ring at a slot step, ending back at its start: the closed ring
/// (the one it was before stepping out), or an open path and back along it.
fn slot_lap(ring: &PocketRing, feed: f64) -> Vec<Move> {
    let path: Vec<Point> = match &ring.unstepped {
        Some(closed) => crate::profile::closed_path(closed),
        None if !ring.open => crate::profile::closed_path(&ring.points),
        None => ring
            .points
            .iter()
            .chain(ring.points.iter().rev().skip(1))
            .copied()
            .collect(),
    };
    path[1..].iter().map(|&p| Move::feed_xy(p, feed)).collect()
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
        slot_feed,
        slot_step,
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
    let ring_feed = |ring: &PocketRing| if ring.full_width { slot_feed } else { feed };
    let descend = |ring: &PocketRing, from_z: f64, cleared: f64, z: f64| {
        let path = path_of(ring);
        let length = crate::ramp::polyline_length(&path);
        let f = ring_feed(ring);
        crate::ramp::enter(&path, length, from_z, cleared, z, entry, f, plunge_feed)
    };
    let mut cleared = top_z;
    // Where the tool is before a pass: at safe Z, or lifted off the floor of the pass
    // before (docs/specs/pocket-stay-down.md).
    let mut from_z = safe_z;
    for (pass, &z) in pass_z.iter().enumerate() {
        let down = from_z < safe_z;
        let levels = slot_levels(cleared, z, slot_step);
        let stepped = levels.len() > 1;
        // The first ring's levels: the slot steps when it cuts full width.
        let first_levels: &[f64] = if stepped && piece.rings[0].full_width {
            &levels
        } else {
            std::slice::from_ref(&z)
        };
        let first_feed = ring_feed(&piece.rings[0]);
        let mut level_top = cleared;
        for (step, &level) in first_levels.iter().enumerate() {
            let entered = step > 0;
            match (piece.helix, entry) {
                (Some(helix), Entry::Helix { ramp_angle, .. }) => {
                    // Down round the helix, then at depth to the first path (checked when
                    // the helix was fitted); after a slot step, back along that link.
                    if down || entered {
                        moves.push(Move::feed_xy(helix.start, first_feed));
                    } else {
                        moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(helix.start)]);
                    }
                    moves.extend(helix_moves(
                        &helix,
                        level_top,
                        level,
                        ramp_angle,
                        first_feed,
                        plunge_feed,
                    )?);
                    moves.push(Move::feed_xy(first, first_feed));
                }
                _ => {
                    let from = if entered {
                        level_top
                    } else {
                        if down {
                            moves.push(Move::feed_xy(first, first_feed));
                        } else {
                            moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(first)]);
                        }
                        from_z
                    };
                    moves.extend(descend(&piece.rings[0], from, level_top, level)?);
                }
            }
            if step + 1 < first_levels.len() {
                moves.extend(slot_lap(&piece.rings[0], first_feed));
            }
            level_top = level;
        }
        for (index, ring) in piece.rings.iter().enumerate() {
            let start = ring.points[0];
            let f = ring_feed(ring);
            if index > 0 {
                let slot_steps = stepped && ring.full_width;
                if piece.feed_links[index - 1] && !slot_steps {
                    // A ring that stepped out already ends on this start: no link move.
                    if piece.rings[index - 1].end() != Some(start) {
                        moves.push(Move::feed_xy(start, feed));
                    }
                } else if slot_steps {
                    // A full-width ring goes down in slot steps, from above its start.
                    moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(start)]);
                    let mut from = safe_z;
                    let mut top = cleared;
                    for &level in &levels {
                        if from < safe_z {
                            moves.extend(slot_lap(ring, f));
                        }
                        moves.extend(descend(ring, from, top, level)?);
                        (from, top) = (level, level);
                    }
                } else {
                    moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(start)]);
                    moves.extend(descend(ring, safe_z, cleared, z)?);
                }
            }
            moves.extend(ring.points[1..].iter().map(|&p| Move::feed_xy(p, f)));
            if !ring.open {
                moves.push(Move::feed_xy(start, f));
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
