// SPDX-License-Identifier: GPL-3.0-or-later
//! Helical entry for pockets (docs/specs/helix-and-leads.md part C): the tool goes down
//! round a small circle, so its side teeth cut and a tool that isn't centre-cutting can
//! open a pocket. Then one flat circle levels the floor.

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::primitives::segment_count;
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_i18n::{Digits, Message, Value};

use crate::ToolpathError;
use crate::arcs::Circle;
use crate::model::Move;

/// A helix needing more turns than this means the angle is far too shallow for the depth.
pub const MAX_HELIX_TURNS: usize = 1000;

/// Where and how a pocket piece's helix runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Helix {
    /// The circle the tool centre follows.
    pub circle: Circle,
    /// Where on it the tool goes down and where it ends, after the flat circle.
    pub start: Point,
    /// Clockwise seen from above (the walls' cut direction, so it climbs as they do).
    pub clockwise: bool,
}

impl Helix {
    /// The point `turns` of a turn round from the start, going the helix's way.
    fn at(&self, turns: f64) -> Point {
        let c = self.circle.center;
        let from = (self.start.1 - c.1).atan2(self.start.0 - c.0);
        let sign = if self.clockwise { -1.0 } else { 1.0 };
        let a = from + sign * TAU * turns;
        (
            c.0 + self.circle.radius * a.cos(),
            c.1 + self.circle.radius * a.sin(),
        )
    }

    /// Chords per turn.
    fn chords(&self) -> usize {
        segment_count(self.circle.radius, TAU).max(4)
    }

    /// Every chord of the helix and its flat circle, ends included, for checking.
    pub fn points(&self) -> Vec<Point> {
        let n = self.chords();
        (0..=n).map(|k| self.at(k as f64 / n as f64)).collect()
    }
}

/// Moves that take the tool, standing at `safe_z` above the helix's start, down to `z`: a
/// straight descent to `cleared` (cut already, so through air or cut material), whole turns
/// down to `z` never steeper than `ramp_angle`, then one flat turn. The tool ends at the
/// start, at `z`. Turning moves use the cutting `feed` and carry the circle, so the post
/// can write them as (helical) G2/G3.
pub fn helix_moves(
    helix: &Helix,
    cleared: f64,
    z: f64,
    ramp_angle: f64,
    feed: f64,
    plunge_feed: f64,
) -> Result<Vec<Move>, ToolpathError> {
    let mut moves = vec![Move::feed_z(cleared, plunge_feed)];
    let n = helix.chords();
    let drop = cleared - z;
    let circle = Some(helix.circle);
    if drop > DEPTH_TOL_MM {
        let per_turn = TAU * helix.circle.radius * ramp_angle.tan();
        // Whole turns: the slope is at most the angle. A small positive whole number.
        let turns = (drop / per_turn).ceil().max(1.0) as usize;
        if turns > MAX_HELIX_TURNS {
            return Err(ToolpathError::Geometry(
                Message::new("toolpath-error-helix-turns")
                    .arg(
                        "radius",
                        Value::Length(helix.circle.radius, Digits::Fixed(3)),
                    )
                    .arg(
                        "angle",
                        Value::Number(ramp_angle.to_degrees(), Digits::Fixed(2)),
                    )
                    .arg("turns", turns)
                    .arg("drop", Value::Length(drop, Digits::Fixed(3))),
            ));
        }
        let steps = turns * n;
        for k in 1..=steps {
            let p = if k % n == 0 {
                helix.start
            } else {
                helix.at(k as f64 / n as f64)
            };
            let depth = if k == steps {
                z
            } else {
                cleared - drop * k as f64 / steps as f64
            };
            let mut m = Move::feed_xyz(p, depth, feed);
            m.circle = circle;
            moves.push(m);
        }
    } else {
        moves.push(Move::feed_z(z, plunge_feed));
    }
    // The flat turn: the floor the helix left sloping is cut level.
    for k in 1..=n {
        let p = if k == n {
            helix.start
        } else {
            helix.at(k as f64 / n as f64)
        };
        let mut m = Move::feed_xy(p, feed);
        m.circle = circle;
        moves.push(m);
    }
    Ok(moves)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_core::primitives::distance;

    fn helix(clockwise: bool) -> Helix {
        Helix {
            circle: Circle {
                center: (10.0, 10.0),
                radius: 1.5,
            },
            start: (11.5, 10.0),
            clockwise,
        }
    }

    const THREE_DEG: f64 = 0.052_359_877_559_829_88;

    #[test]
    fn turns_go_down_no_steeper_than_the_angle_then_one_flat_turn() {
        let h = helix(false);
        let moves = helix_moves(&h, -2.0, -5.0, THREE_DEG, 1000.0, 300.0).unwrap();
        assert_eq!(moves[0], Move::feed_z(-2.0, 300.0));
        // 2π · 1.5 · tan 3° = 0.494 mm per turn: 3 mm takes 7 turns.
        let n = h.chords();
        assert_eq!(moves.len(), 1 + 7 * n + n);
        let mut previous = (h.start, -2.0);
        for m in &moves[1..=7 * n] {
            let p = (m.x.unwrap(), m.y.unwrap());
            let z = m.z.unwrap();
            assert!((distance(p, h.circle.center) - 1.5).abs() < 1e-9);
            let run = distance(previous.0, p);
            assert!(previous.1 - z > 0.0 && (previous.1 - z) / run <= THREE_DEG.tan() + 1e-9);
            assert_eq!(m.circle, Some(h.circle));
            previous = (p, z);
        }
        assert_eq!(
            previous,
            (h.start, -5.0),
            "ends exactly at the start and the depth"
        );
        for m in &moves[1 + 7 * n..] {
            assert_eq!(m.z, None);
            assert_eq!(m.circle, Some(h.circle));
        }
        let last = moves.last().unwrap();
        assert_eq!((last.x.unwrap(), last.y.unwrap()), h.start);
    }

    #[test]
    fn the_direction_follows_the_walls() {
        let ccw = helix_moves(&helix(false), 0.0, -1.0, THREE_DEG, 1000.0, 300.0).unwrap();
        let cw = helix_moves(&helix(true), 0.0, -1.0, THREE_DEG, 1000.0, 300.0).unwrap();
        // From (11.5, 10) counter-clockwise goes up (+y), clockwise down.
        assert!(ccw[1].y.unwrap() > 10.0);
        assert!(cw[1].y.unwrap() < 10.0);
    }

    #[test]
    fn nothing_to_go_down_is_a_flat_turn() {
        let h = helix(false);
        let moves = helix_moves(&h, -3.0, -3.0, THREE_DEG, 1000.0, 300.0).unwrap();
        assert_eq!(moves.len(), 2 + h.chords());
    }

    #[test]
    fn far_too_many_turns_are_refused() {
        let err = helix_moves(&helix(false), 0.0, -600.0, THREE_DEG, 1000.0, 300.0)
            .unwrap_err()
            .to_string();
        assert!(err.contains("turns to go down"), "{err}");
    }
}
