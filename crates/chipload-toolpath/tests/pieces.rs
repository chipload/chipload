// SPDX-License-Identifier: GPL-3.0-or-later
//! Pieces (docs/specs/pieces.md §12): cuts trimmed to each piece's reach together cover
//! the whole cut, and no piece's cut leaves its reach.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::Point;
use chipload_core::math;
use chipload_core::primitives::{Line, Primitive};
use chipload_job::Joint;
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::pieces::{JointShape, Trimmer, layout, reach};
use proptest::prelude::*;

/// How close a point must come to a kept cut to count as covered: chords of the reach's
/// arcs lie within the arc tolerance of the true curve.
const TOL: f64 = 0.02;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Primitive> {
    let pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    (0..4)
        .map(|i| {
            Primitive::Line(Line {
                start: pts[i],
                end: pts[(i + 1) % 4],
            })
        })
        .collect()
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    math::hypot(p.0 - (a.0 + t * dx), p.1 - (a.1 + t * dy))
}

/// The cutting stretches of a trimmed program, as (from, to) in XY.
fn cuts(moves: &[Move]) -> Vec<(Point, Point)> {
    let mut p = (0.0, 0.0);
    let mut out = Vec::new();
    for m in moves {
        let q = (m.x.unwrap_or(p.0), m.y.unwrap_or(p.1));
        if m.kind == MoveKind::Feed {
            out.push((p, q));
        }
        p = q;
    }
    out
}

fn inside_or_near(ring: &[Point], p: Point) -> bool {
    let mut odd = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if segment_distance(p, a, b) <= TOL {
            return true;
        }
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0) {
            odd = !odd;
        }
    }
    odd
}

proptest! {
    #![proptest_config(common::proptest_config(24))]

    /// A random zig-zag cut over a board in 3 × 2 pieces with knobs or fingers, trimmed to
    /// every piece: each point of the cut is kept by some piece, and every kept cut lies in
    /// its piece's reach.
    #[test]
    fn trimmed_pieces_cover_the_cut_and_stay_in_reach(
        points in prop::collection::vec((-20.0f64..920.0, -20.0f64..520.0), 2..12),
        fingers in any::<bool>(),
        radius in 1.0f64..6.0,
    ) {
        let joint = if fingers { Joint::Fingers } else { Joint::Knobs };
        let (size, spacing) = joint.defaults();
        let shape = JointShape { joint, size, spacing, round: 3.2 };
        let outline = rect(0.0, 0.0, 900.0, 500.0);
        let lay = layout(&[outline], ((0.0, 0.0), (900.0, 500.0)), (3, 2), (&[], &[]), shape);
        let mut moves = vec![
            Move::rapid_z(5.0),
            Move::rapid_xy(points[0]),
            Move::rapid_z(1.0),
            Move::feed_z(-2.0, 300.0),
        ];
        moves.extend(points[1..].iter().map(|&p| Move::feed_xy(p, 1500.0)));
        moves.push(Move::rapid_z(5.0));
        let mut kept: Vec<(Point, Point)> = Vec::new();
        for c in 0..lay.columns() {
            for r in 0..lay.rows() {
                let ring = reach(&lay.cell(c, r), radius).unwrap();
                let mut t = Trimmer::new(5.0, 1.0);
                t.add(&moves, (0.0, 0.0, 5.0), Some(&ring), 300.0).unwrap();
                t.finish();
                for &(a, b) in &cuts(&t.moves) {
                    for k in 0..=4 {
                        let s = f64::from(k) / 4.0;
                        let p = (a.0 + s * (b.0 - a.0), a.1 + s * (b.1 - a.1));
                        prop_assert!(inside_or_near(&ring, p), "{p:?} out of piece {c},{r}");
                    }
                }
                // Every move down below the board's top is a feed.
                let mut z = 5.0;
                for m in &t.moves {
                    if let Some(nz) = m.z {
                        if nz < 1.0 && nz < z {
                            prop_assert_eq!(m.kind, MoveKind::Feed);
                        }
                        z = nz;
                    }
                }
                kept.extend(cuts(&t.moves));
            }
        }
        for w in points.windows(2) {
            for k in 0..=20 {
                let s = f64::from(k) / 20.0;
                let p = (w[0].0 + s * (w[1].0 - w[0].0), w[0].1 + s * (w[1].1 - w[0].1));
                let d = kept
                    .iter()
                    .map(|&(a, b)| segment_distance(p, a, b))
                    .fold(f64::INFINITY, f64::min);
                prop_assert!(d <= TOL, "{p:?} cut by no piece ({d})");
            }
        }
    }
}
