// SPDX-License-Identifier: GPL-3.0-or-later
//! Arc output (docs/specs/arc-output.md). Rings stay point lists for every decision (order,
//! links, tabs, verification); an [`ArcTable`] remembers which chords lie on which circle,
//! the planner tags those feed moves, and [`merge_arcs`] turns runs of tagged chords back
//! into G2/G3 arcs when the post wants arcs. Without that the tags are ignored, so the
//! output is exactly the chords.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, DEPTH_TOL_MM};

use crate::ToolpathError;
use crate::model::{Move, MoveKind, Program};

/// The circle a chord or an arc move lies on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

type Key = (u64, u64);

fn key(p: Point) -> Key {
    (p.0.to_bits(), p.1.to_bits())
}

/// Chords known to lie on an arc of the offset, keyed by their exact end points (in either
/// order). A chord that isn't in the table (a link, a split at a tab) is cut as a line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArcTable(BTreeMap<(Key, Key), Circle>);

impl ArcTable {
    pub fn insert(&mut self, a: Point, b: Point, circle: Circle) {
        self.0.insert((key(a), key(b)), circle);
    }

    pub fn get(&self, a: Point, b: Point) -> Option<Circle> {
        self.0
            .get(&(key(a), key(b)))
            .or_else(|| self.0.get(&(key(b), key(a))))
            .copied()
    }

    pub fn extend(&mut self, other: ArcTable) {
        self.0.extend(other.0);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The point of the arc halfway between the chord's ends (the chord is less than a half
/// circle, as every flattened chord is).
pub fn arc_midpoint(a: Point, b: Point, circle: Circle) -> Point {
    let c = circle.center;
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let (dx, dy) = (mid.0 - c.0, mid.1 - c.1);
    let len = dx.hypot(dy);
    if len == 0.0 {
        return mid;
    }
    (
        c.0 + dx / len * circle.radius,
        c.1 + dy / len * circle.radius,
    )
}

/// Mark every XY feed move that is a chord in `table` with its circle.
pub fn tag_moves(moves: &mut [Move], table: &ArcTable) {
    if table.is_empty() {
        return;
    }
    let (mut x, mut y): (Option<f64>, Option<f64>) = (None, None);
    for m in moves {
        let from = x.zip(y);
        let to = m.x.or(x).zip(m.y.or(y));
        if m.kind == MoveKind::Feed
            && m.z.is_none()
            && let (Some(from), Some(to)) = (from, to)
        {
            // Moves tagged when they were made (a helix's flat circle) keep their tag.
            m.circle = table.get(from, to).or(m.circle);
        }
        (x, y) = (m.x.or(x), m.y.or(y));
    }
}

/// Signed angle swept from `a` to `b` around `c` (positive counter-clockwise), the short way.
fn swept(c: Point, a: Point, b: Point) -> f64 {
    let (ax, ay) = (a.0 - c.0, a.1 - c.1);
    let (bx, by) = (b.0 - c.0, b.1 - c.1);
    (ax * by - ay * bx).atan2(ax * bx + ay * by)
}

/// Turn runs of tagged chords into arc moves of at most 180°; arcs that bulge no more than
/// `ARC_APPROX_TOL_MM` over their chord stay lines. Every tag is dropped from the result.
///
/// A run is flat (XY chords) or helical (chords that also go down or up, at the same rate
/// per radian all along: a helix, docs/specs/helix-and-leads.md part C); a helical piece
/// ends at its last chord's Z.
pub fn merge_arcs(program: &Program) -> Result<Program, ToolpathError> {
    let mut out = Vec::with_capacity(program.moves.len());
    let moves = &program.moves;
    let (mut x, mut y, mut z): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    let mut i = 0;
    while i < moves.len() {
        let m = moves[i];
        let start = x.zip(y);
        let helical = m.z.is_some();
        if let (Some(circle), Some(start), MoveKind::Feed) = (m.circle, start, m.kind)
            && (!helical || z.is_some())
        {
            // The run: consecutive chords of the same circle, same feed, same direction, and
            // (helical) the same descent per radian.
            let mut vertices = vec![start];
            let mut heights: Vec<Option<f64>> = vec![None];
            let mut sweeps: Vec<f64> = Vec::new();
            let mut rate: Option<f64> = None;
            let mut j = i;
            while j < moves.len() {
                let n = moves[j];
                let (Some(nx), Some(ny)) = (n.x.or(x), n.y.or(y)) else {
                    break;
                };
                let last = vertices[vertices.len() - 1];
                let sweep = swept(circle.center, last, (nx, ny));
                let mut same = n.kind == MoveKind::Feed
                    && n.z.is_some() == helical
                    && n.circle == Some(circle)
                    && n.feed == m.feed
                    && sweeps
                        .first()
                        .is_none_or(|s: &f64| s.signum() == sweep.signum());
                if same && let (Some(nz), Some(cz)) = (n.z, z) {
                    let step = (nz - cz) / sweep;
                    same = sweep != 0.0
                        && rate.is_none_or(|r| ((step - r) * sweep).abs() <= DEPTH_TOL_MM);
                    rate = rate.or(Some(step));
                }
                if !same {
                    break;
                }
                vertices.push((nx, ny));
                heights.push(n.z);
                sweeps.push(sweep);
                (x, y, z) = (Some(nx), Some(ny), n.z.or(z));
                j += 1;
            }
            let first = out.len();
            emit_run(&mut out, &vertices, &heights, &sweeps, circle, m.feed)?;
            // A comment on the run's first chord stays on its first piece.
            if let Some(piece) = out.get_mut(first)
                && m.comment.is_some()
            {
                piece.comment = m.comment;
            }
            i = j;
            continue;
        }
        let mut plain = m;
        plain.circle = None;
        out.push(plain);
        (x, y, z) = (m.x.or(x), m.y.or(y), m.z.or(z));
        i += 1;
    }
    Ok(Program {
        moves: out,
        ..program.clone()
    })
}

/// Pieces of at most 180° from a run of chords on one circle; `heights[k]` is the Z the
/// run reaches at vertex k (None for a flat run).
fn emit_run(
    out: &mut Vec<Move>,
    vertices: &[Point],
    heights: &[Option<f64>],
    sweeps: &[f64],
    circle: Circle,
    feed: Option<f64>,
) -> Result<(), ToolpathError> {
    for &v in vertices {
        if (distance(v, circle.center) - circle.radius).abs() > CHAIN_TOL_MM {
            return Err(ToolpathError::Geometry(format!(
                "arc output: ({:.4}, {:.4}) is not on the circle it was tagged with",
                v.0, v.1
            )));
        }
    }
    let mut first = 0; // index of the piece's first vertex
    let mut total = 0.0;
    for (k, &sweep) in sweeps.iter().enumerate() {
        if (total + sweep).abs() > PI + ARC_TOL_RAD {
            push_piece(
                out,
                &vertices[first..=k],
                &heights[first..=k],
                total,
                circle,
                feed,
            );
            first = k;
            total = 0.0;
        }
        total += sweep;
    }
    push_piece(
        out,
        &vertices[first..],
        &heights[first..],
        total,
        circle,
        feed,
    );
    Ok(())
}

/// Float slack on the 180° limit: a half circle made of chords sums to π give or take a few
/// ulps. Exactly 180° is fine with I/J (only the R form is ambiguous there).
const ARC_TOL_RAD: f64 = 1e-9;

fn push_piece(
    out: &mut Vec<Move>,
    vertices: &[Point],
    heights: &[Option<f64>],
    sweep: f64,
    circle: Circle,
    feed: Option<f64>,
) {
    let Some(&end) = vertices.last() else {
        return;
    };
    if vertices.len() < 2 {
        return;
    }
    let sagitta = circle.radius * (1.0 - (sweep.abs() / 2.0).cos());
    if sagitta <= ARC_APPROX_TOL_MM {
        // Nearly straight: the chords themselves, as lines.
        for (&v, &h) in vertices[1..].iter().zip(&heights[1..]) {
            out.push(Move {
                kind: MoveKind::Feed,
                x: Some(v.0),
                y: Some(v.1),
                z: h,
                feed,
                circle: None,
                comment: None,
            });
        }
        return;
    }
    out.push(Move {
        kind: if sweep < 0.0 {
            MoveKind::ArcCw
        } else {
            MoveKind::ArcCcw
        },
        x: Some(end.0),
        y: Some(end.1),
        z: heights.last().copied().flatten(),
        feed,
        circle: Some(circle),
        comment: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::Spindle;

    fn circle_chords(c: Point, r: f64, n: usize, ccw: bool) -> (Vec<Point>, ArcTable) {
        let circle = Circle {
            center: c,
            radius: r,
        };
        let mut points: Vec<Point> = (0..=n)
            .map(|i| {
                let a = -PI / 2.0 + std::f64::consts::TAU * i as f64 / n as f64;
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect();
        if !ccw {
            points.reverse();
        }
        points[n] = points[0]; // closed exactly
        let mut table = ArcTable::default();
        for w in points.windows(2) {
            table.insert(w[0], w[1], circle);
        }
        (points, table)
    }

    fn program(points: &[Point], table: &ArcTable) -> Program {
        let mut moves = vec![
            Move::rapid_z(5.0),
            Move::rapid_xy(points[0]),
            Move::feed_z(-1.0, 300.0),
        ];
        moves.extend(points[1..].iter().map(|&p| Move::feed_xy(p, 1000.0)));
        tag_moves(&mut moves, table);
        Program {
            tool_id: 1,
            tool_diameter: 6.0,
            rpm: 18000.0,
            spindle: Spindle::Cw,
            safe_z: 5.0,
            moves,
            comments: Vec::new(),
        }
    }

    #[test]
    fn a_full_circle_becomes_two_arcs_of_at_most_a_half() {
        for ccw in [true, false] {
            let (points, table) = circle_chords((10.0, 20.0), 5.0, 72, ccw);
            let merged = merge_arcs(&program(&points, &table)).unwrap();
            let arcs: Vec<&Move> = merged
                .moves
                .iter()
                .filter(|m| matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw))
                .collect();
            assert_eq!(arcs.len(), 2, "a full circle is two halves");
            let kind = if ccw {
                MoveKind::ArcCcw
            } else {
                MoveKind::ArcCw
            };
            assert!(arcs.iter().all(|m| m.kind == kind));
            // Ends at the start again; no feed lines left over.
            let last = merged.moves.last().unwrap();
            assert_eq!((last.x, last.y), (Some(points[0].0), Some(points[0].1)));
            assert!(
                !merged
                    .moves
                    .iter()
                    .any(|m| m.kind == MoveKind::Feed && m.x.is_some())
            );
        }
    }

    #[test]
    fn nearly_straight_arcs_and_untagged_chords_stay_lines() {
        // A huge radius: 5 degrees of it bulge less than 0.01 mm? No: pick a tiny sweep.
        let (points, table) = circle_chords((0.0, 0.0), 1000.0, 36_000, true);
        let few = &points[..4]; // 3 chords of 0.01 degree: sagitta ~ 4e-5 mm
        let merged = merge_arcs(&program(few, &table)).unwrap();
        assert!(
            merged
                .moves
                .iter()
                .all(|m| !matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw))
        );
        assert_eq!(merged.moves.len(), 6);
        // Without a table nothing is an arc and nothing changes.
        let plain = program(&points[..10], &ArcTable::default());
        assert_eq!(merge_arcs(&plain).unwrap().moves, plain.moves);
    }

    #[test]
    fn a_chord_tagged_with_the_wrong_circle_is_an_error() {
        let (points, _) = circle_chords((0.0, 0.0), 5.0, 8, true);
        let mut wrong = ArcTable::default();
        for w in points.windows(2) {
            wrong.insert(
                w[0],
                w[1],
                Circle {
                    center: (0.0, 0.0),
                    radius: 6.0,
                },
            );
        }
        assert!(merge_arcs(&program(&points, &wrong)).is_err());
    }

    #[test]
    fn the_arc_midpoint_lies_on_the_circle() {
        let circle = Circle {
            center: (1.0, 1.0),
            radius: 2.0,
        };
        let m = arc_midpoint((3.0, 1.0), (1.0, 3.0), circle);
        assert!((distance(m, circle.center) - 2.0).abs() < 1e-12);
        assert!(m.0 > 2.0 && m.1 > 2.0);
    }
}
