// SPDX-License-Identifier: GPL-3.0-or-later
//! Machine-independent toolpath: what the post-processor turns into G-code. Units are mm.

use chipload_core::Point;
use chipload_job::Spindle;

use crate::arcs::Circle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Rapid,
    /// A straight cutting move.
    Feed,
    /// G2 / G3 in the XY plane, ending at (x, y) around `circle` (only after merge_arcs).
    ArcCw,
    ArcCcw,
}

/// One move. Axes left as None keep their current value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    pub kind: MoveKind,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    /// mm/min; set for cutting moves, None for rapids.
    pub feed: Option<f64>,
    /// On a feed: the chord lies on this circle (docs/specs/arc-output.md). On an arc: its
    /// circle. None otherwise.
    pub circle: Option<Circle>,
    /// A comment written before this move: an index into the program's `comments` (a
    /// part's start, docs/specs/parts.md §6). None for nearly every move.
    pub comment: Option<u32>,
}

impl Move {
    pub fn rapid_z(z: f64) -> Self {
        Move {
            kind: MoveKind::Rapid,
            x: None,
            y: None,
            z: Some(z),
            feed: None,
            circle: None,
            comment: None,
        }
    }

    pub fn rapid_xy(p: Point) -> Self {
        Move {
            kind: MoveKind::Rapid,
            x: Some(p.0),
            y: Some(p.1),
            z: None,
            feed: None,
            circle: None,
            comment: None,
        }
    }

    pub fn feed_z(z: f64, feed: f64) -> Self {
        Move {
            kind: MoveKind::Feed,
            x: None,
            y: None,
            z: Some(z),
            feed: Some(feed),
            circle: None,
            comment: None,
        }
    }

    /// A cutting move in X, Y and Z at once (a ramp).
    pub fn feed_xyz(p: Point, z: f64, feed: f64) -> Self {
        Move {
            kind: MoveKind::Feed,
            x: Some(p.0),
            y: Some(p.1),
            z: Some(z),
            feed: Some(feed),
            circle: None,
            comment: None,
        }
    }

    pub fn feed_xy(p: Point, feed: f64) -> Self {
        Move {
            kind: MoveKind::Feed,
            x: Some(p.0),
            y: Some(p.1),
            z: None,
            feed: Some(feed),
            circle: None,
            comment: None,
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
    /// Texts of the moves' comments, by index.
    pub comments: Vec<String>,
}
