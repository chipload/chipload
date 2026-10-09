// SPDX-License-Identifier: GPL-3.0-or-later
//! Machine-independent toolpath: what the post-processor turns into G-code. Units are mm.

use chipload_core::Point;
use chipload_job::Spindle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Rapid,
    Feed,
}

/// A straight move. Axes left as None keep their current value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    pub kind: MoveKind,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    /// mm/min; set for feed moves, None for rapids.
    pub feed: Option<f64>,
}

impl Move {
    pub fn rapid_z(z: f64) -> Self {
        Move {
            kind: MoveKind::Rapid,
            x: None,
            y: None,
            z: Some(z),
            feed: None,
        }
    }

    pub fn rapid_xy(p: Point) -> Self {
        Move {
            kind: MoveKind::Rapid,
            x: Some(p.0),
            y: Some(p.1),
            z: None,
            feed: None,
        }
    }

    pub fn feed_z(z: f64, feed: f64) -> Self {
        Move {
            kind: MoveKind::Feed,
            x: None,
            y: None,
            z: Some(z),
            feed: Some(feed),
        }
    }

    pub fn feed_xy(p: Point, feed: f64) -> Self {
        Move {
            kind: MoveKind::Feed,
            x: Some(p.0),
            y: Some(p.1),
            z: None,
            feed: Some(feed),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub tool_id: i64,
    pub tool_diameter: f64,
    pub rpm: f64,
    pub spindle: Spindle,
    pub safe_z: f64,
    pub moves: Vec<Move>,
}
