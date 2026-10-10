// SPDX-License-Identifier: GPL-3.0-or-later
//! Going down into the material: straight, or ramping along the path (docs/specs/
//! ramp-entry.md). A ramp goes out along the path and back, descending all the way and
//! never steeper than the ramp angle, until it is back where it began at the pass depth.
//! So it ends exactly where a plunge would have left the tool, and it only ever moves on the
//! path the pass cuts anyway.

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_job::Entry;

use crate::ToolpathError;
use crate::model::Move;

/// A ramp needing more legs than this means the path is far too short for it.
pub const MAX_RAMP_LEGS: usize = 1000;

/// Moves that take the tool, standing at `from_z` above `path[0]`, down to `z`.
///
/// - Plunge: straight down to `z`.
/// - Ramp: straight down to `cleared` (the depth already cut there, so only air or cut
///   material is passed), then along `path` and back to `path[0]` at `z`. At most
///   `available` mm of the path is used.
///
/// Ramp moves use the cutting `feed` (they move mostly sideways), straight ones
/// `plunge_feed`.
#[allow(clippy::too_many_arguments)]
pub fn enter(
    path: &[Point],
    available: f64,
    from_z: f64,
    cleared: f64,
    z: f64,
    entry: Entry,
    feed: f64,
    plunge_feed: f64,
) -> Result<Vec<Move>, ToolpathError> {
    let ramp_angle = match entry {
        Entry::Ramp { ramp_angle } if cleared - z > DEPTH_TOL_MM => ramp_angle,
        _ => return Ok(vec![Move::feed_z(z, plunge_feed)]),
    };
    let mut moves = Vec::new();
    if (from_z - cleared).abs() > DEPTH_TOL_MM {
        moves.push(Move::feed_z(cleared, plunge_feed));
    }
    moves.extend(zigzag(path, available, cleared, z, ramp_angle, feed)?);
    Ok(moves)
}

/// Length of an open polyline.
pub fn polyline_length(path: &[Point]) -> f64 {
    path.windows(2).map(|w| distance(w[0], w[1])).sum()
}

/// The first `length` mm of `path`, ending on an interpolated point; with the distance of
/// each point from path[0].
fn stretch(path: &[Point], length: f64) -> Vec<(Point, f64)> {
    let Some(&first) = path.first() else {
        return Vec::new();
    };
    let mut out = vec![(first, 0.0)];
    let mut travelled = 0.0;
    for w in path.windows(2) {
        let seg = distance(w[0], w[1]);
        if seg <= 0.0 {
            continue;
        }
        if travelled + seg >= length {
            let t = (length - travelled) / seg;
            let end = (
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            );
            out.push((end, length));
            return out;
        }
        travelled += seg;
        out.push((w[1], travelled));
    }
    out
}

/// Out along the path and back, as often as needed, from `top` down to `z`.
fn zigzag(
    path: &[Point],
    available: f64,
    top: f64,
    z: f64,
    ramp_angle: f64,
    feed: f64,
) -> Result<Vec<Move>, ToolpathError> {
    let drop = top - z;
    // The path length a ramp at this angle needs from `top` to `z`.
    let full = drop / ramp_angle.tan();
    let reach = (full / 2.0).min(available).min(polyline_length(path));
    let too_short = || {
        ToolpathError::Geometry(format!(
            "The path is too short to ramp down {drop:.3} mm at {:.1}° (only {reach:.3} mm \
             to ramp along); use a steeper ramp angle or a straight plunge with a \
             centre-cutting tool",
            ramp_angle.to_degrees()
        ))
    };
    if reach <= CHAIN_TOL_MM {
        return Err(too_short());
    }
    // An even number of legs ends back at path[0]; each drops at most reach · tan(angle).
    let needed = (full / reach - DEPTH_TOL_MM).ceil().max(1.0);
    if needed > MAX_RAMP_LEGS as f64 {
        return Err(too_short());
    }
    // Small positive whole number (checked above).
    let mut legs = needed as usize;
    legs += legs % 2;
    let points = stretch(path, reach);
    let per_leg = drop / legs as f64;
    let mut moves = Vec::with_capacity(legs * points.len());
    for leg in 0..legs {
        let leg_top = top - per_leg * leg as f64;
        let forward = leg % 2 == 0;
        let order: Vec<&(Point, f64)> = if forward {
            points.iter().skip(1).collect()
        } else {
            points.iter().rev().skip(1).collect()
        };
        for &(point, along) in order {
            let gone = if forward { along } else { reach - along };
            let depth = if leg + 1 == legs && !forward && gone >= reach - CHAIN_TOL_MM {
                z // the very end: exactly the pass depth
            } else {
                leg_top - per_leg * gone / reach
            };
            moves.push(Move::feed_xyz(point, depth, feed));
        }
    }
    Ok(moves)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MoveKind;

    const THREE_DEG: f64 = 3.0 * std::f64::consts::PI / 180.0;

    /// (x, y, z) after each move, starting at (0, 0, from).
    fn trace(moves: &[Move], from: f64) -> Vec<(f64, f64, f64)> {
        let (mut x, mut y, mut z) = (0.0, 0.0, from);
        moves
            .iter()
            .map(|m| {
                x = m.x.unwrap_or(x);
                y = m.y.unwrap_or(y);
                z = m.z.unwrap_or(z);
                (x, y, z)
            })
            .collect()
    }

    fn steepest(points: &[(f64, f64, f64)], from: (f64, f64, f64)) -> f64 {
        let mut last = from;
        let mut worst: f64 = 0.0;
        for &p in points {
            let run = (p.0 - last.0).hypot(p.1 - last.1);
            if run > 0.0 {
                worst = worst.max((last.2 - p.2) / run);
            }
            last = p;
        }
        worst
    }

    #[test]
    fn a_plunge_is_straight_down() {
        let path = [(0.0, 0.0), (100.0, 0.0)];
        let moves = enter(
            &path,
            100.0,
            5.0,
            -1.0,
            -3.0,
            Entry::Plunge {},
            1000.0,
            200.0,
        )
        .unwrap();
        assert_eq!(moves, [Move::feed_z(-3.0, 200.0)]);
    }

    #[test]
    fn a_long_path_ramps_out_and_back_once() {
        let path = [(0.0, 0.0), (100.0, 0.0)];
        let entry = Entry::Ramp {
            ramp_angle: THREE_DEG,
        };
        let moves = enter(&path, 100.0, 5.0, -1.0, -3.0, entry, 1000.0, 200.0).unwrap();
        // Down to the cleared depth at the plunge feed, then two legs at the cutting feed.
        assert_eq!(moves[0], Move::feed_z(-1.0, 200.0));
        assert_eq!(moves.len(), 3);
        let reach = 2.0 / THREE_DEG.tan() / 2.0;
        let points = trace(&moves[1..], -1.0);
        assert!((points[0].0 - reach).abs() < 1e-9 && (points[0].2 + 2.0).abs() < 1e-9);
        assert_eq!(points[1], (0.0, 0.0, -3.0));
        assert!(
            moves[1..]
                .iter()
                .all(|m| m.kind == MoveKind::Feed && m.feed == Some(1000.0))
        );
        assert!(steepest(&points, (0.0, 0.0, -1.0)) <= THREE_DEG.tan() + 1e-12);
    }

    #[test]
    fn a_short_path_takes_more_legs_and_never_gets_steeper() {
        // 5 mm of path for a 2 mm drop at 3° (38 mm of ramp): 8 legs.
        let path = [(0.0, 0.0), (3.0, 0.0), (3.0, 4.0)];
        let entry = Entry::Ramp {
            ramp_angle: THREE_DEG,
        };
        let moves = enter(&path, 5.0, 0.0, 0.0, -2.0, entry, 1000.0, 200.0).unwrap();
        let points = trace(&moves, 0.0);
        assert_eq!(points.last(), Some(&(0.0, 0.0, -2.0)));
        assert!(steepest(&points, (0.0, 0.0, 0.0)) <= THREE_DEG.tan() + 1e-12);
        // Down all the way: no leg climbs.
        assert!(points.windows(2).all(|w| w[1].2 <= w[0].2));
        let legs = points.iter().filter(|p| p.0 == 0.0 && p.1 == 0.0).count();
        assert_eq!(legs % 2, 0);
        // The ramp stays on the path's first `available` mm.
        assert!(points.iter().all(|p| p.0 <= 3.0 && p.1 <= 2.0 + 1e-9));
    }

    #[test]
    fn nothing_to_ramp_when_already_at_depth() {
        let path = [(0.0, 0.0), (10.0, 0.0)];
        let entry = Entry::Ramp {
            ramp_angle: THREE_DEG,
        };
        let moves = enter(&path, 10.0, 5.0, -2.0, -2.0, entry, 1000.0, 200.0).unwrap();
        assert_eq!(moves, [Move::feed_z(-2.0, 200.0)]);
    }

    #[test]
    fn a_path_far_too_short_is_refused() {
        let path = [(0.0, 0.0), (0.01, 0.0)];
        let entry = Entry::Ramp {
            ramp_angle: THREE_DEG,
        };
        let err = enter(&path, 0.01, 0.0, 0.0, -5.0, entry, 1000.0, 200.0).unwrap_err();
        assert!(err.to_string().contains("too short to ramp"), "{err}");
    }
}
