// SPDX-License-Identifier: GPL-3.0-or-later
//! Trimming, extending and breaking a drawn shape (docs/specs/drawing-editing.md §4). A
//! place along a chain is its segment and the share along it (0 at the segment's start,
//! 1 at its end); crossings with the cutting edges are found with `intersect`.

use chipload_core::Point;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::intersect;
use crate::offset::Chain;

/// How far ahead an end is looked along for something to extend to, mm.
pub const EXTEND_REACH_MM: f64 = 10_000.0;

/// A place along a chain: segment `seg`, share `t` along it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub seg: usize,
    pub t: f64,
}

impl Place {
    /// Ordered along the chain.
    fn key(self) -> f64 {
        self.seg as f64 + self.t
    }
}

/// Why a trim, extend or break can't be done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimError {
    /// Nothing crosses the shape (a closed one: fewer than two crossings): delete it instead.
    NothingCrosses,
    /// The end to extend is an arc, not a line.
    EndNotStraight,
    /// Nothing lies ahead of the end within `EXTEND_REACH_MM`.
    NothingAhead,
    /// The point is at an end of an open shape: nothing to break.
    AtAnEnd,
    /// The shape is closed: it has no ends to extend.
    Closed,
}

/// Which end of an open chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Start,
    End,
}

/// The chain with the piece under `at` removed: the piece between the crossings with
/// `cutters` nearest `at` on either side (or to the chain's end). An open chain gives the
/// pieces left on either side (none, one or two); a closed one, one open chain.
pub fn trim(chain: &Chain, cutters: &[Primitive], at: Point) -> Result<Vec<Chain>, TrimError> {
    let (lo, hi) = match trimmed_span(chain, cutters, at)? {
        // From the crossing after `at` round to the one before it.
        Span::Round(lo, hi) => return Ok(vec![slice_round(chain, hi, lo)]),
        Span::Along(lo, hi) => (lo, hi),
    };
    let mut pieces = Vec::new();
    if let Some(lo) = lo
        && lo.key() > 0.0
    {
        pieces.push(slice(chain, start_of(), lo));
    }
    if let Some(hi) = hi
        && hi.key() < chain.prims.len() as f64
    {
        pieces.push(slice(chain, hi, end_of(chain)));
    }
    Ok(pieces
        .into_iter()
        .filter(|c| length(c) > CHAIN_TOL_MM)
        .collect())
}

/// The piece a trim at `at` removes, for a preview.
pub fn trimmed_piece(chain: &Chain, cutters: &[Primitive], at: Point) -> Option<Chain> {
    Some(match trimmed_span(chain, cutters, at).ok()? {
        Span::Round(lo, hi) => slice_round(chain, lo, hi),
        Span::Along(lo, hi) => slice(
            chain,
            lo.unwrap_or_else(start_of),
            hi.unwrap_or_else(|| end_of(chain)),
        ),
    })
}

/// The crossings either side of the place trimmed.
enum Span {
    /// On a closed chain: there is always one before and one after (round the loop).
    Round(Place, Place),
    /// On an open chain: None is the chain's end.
    Along(Option<Place>, Option<Place>),
}

/// The crossings either side of `at`.
fn trimmed_span(chain: &Chain, cutters: &[Primitive], at: Point) -> Result<Span, TrimError> {
    let crossings = crossings(chain, cutters);
    let here = nearest_place(chain, at).ok_or(TrimError::NothingCrosses)?;
    if chain.closed {
        if crossings.len() < 2 {
            return Err(TrimError::NothingCrosses);
        }
        let before = crossings
            .iter()
            .rev()
            .find(|c| c.key() < here.key())
            .or(crossings.last());
        let after = crossings
            .iter()
            .find(|c| c.key() > here.key())
            .or(crossings.first());
        return match (before, after) {
            (Some(&before), Some(&after)) => Ok(Span::Round(before, after)),
            _ => Err(TrimError::NothingCrosses),
        };
    }
    if crossings.is_empty() {
        return Err(TrimError::NothingCrosses);
    }
    let before = crossings
        .iter()
        .rev()
        .find(|c| c.key() < here.key())
        .copied();
    let after = crossings.iter().find(|c| c.key() > here.key()).copied();
    Ok(Span::Along(before, after))
}

/// Where `cutters` cross the chain, in order along it, without its own joints and without
/// repeats.
pub fn crossings(chain: &Chain, cutters: &[Primitive]) -> Vec<Place> {
    let joints: Vec<Point> = joints(chain);
    let mut found: Vec<Place> = Vec::new();
    for (seg, prim) in chain.prims.iter().enumerate() {
        for cutter in cutters {
            if cutter == prim {
                continue;
            }
            for p in intersect::intersections(prim, cutter) {
                // Where the chain's own segments meet is a joint, not a crossing, unless a
                // cutter other than its own segments passes there.
                if joints.iter().any(|&j| distance(j, p) < CHAIN_TOL_MM)
                    && chain.prims.contains(cutter)
                {
                    continue;
                }
                let t = share_along(prim, p);
                found.push(Place { seg, t });
            }
        }
    }
    found.sort_by(|a, b| a.key().total_cmp(&b.key()));
    // One crossing found twice: by two cutters, or at the joint of two segments (the end
    // of one and the start of the next). A place the chain passes twice (where it crosses
    // itself) stays twice.
    let mut out: Vec<Place> = Vec::new();
    for place in found {
        let new = out
            .last()
            .is_none_or(|&last| between(chain, last, place) > CHAIN_TOL_MM);
        if new {
            out.push(place);
        }
    }
    out
}

/// How far along the chain from `a` to `b` (`a` first), when they are on the same or on
/// neighbouring segments; infinity otherwise.
fn between(chain: &Chain, a: Place, b: Place) -> f64 {
    let len = |seg: usize| chain.prims[seg].length();
    if a.seg == b.seg {
        (b.t - a.t) * len(a.seg)
    } else if b.seg == a.seg + 1 {
        (1.0 - a.t) * len(a.seg) + b.t * len(b.seg)
    } else {
        f64::INFINITY
    }
}

/// The points where consecutive segments of the chain meet (and a closed chain's start).
fn joints(chain: &Chain) -> Vec<Point> {
    let n = chain.prims.len();
    let mut points: Vec<Point> = chain.prims.iter().skip(1).map(Primitive::start).collect();
    if chain.closed && n > 0 {
        points.push(chain.prims[0].start());
    }
    points
}

/// The chain cut in two at the place nearest `at` (an open chain), or opened there (a
/// closed one).
pub fn break_at(chain: &Chain, at: Point) -> Result<Vec<Chain>, TrimError> {
    let here = nearest_place(chain, at).ok_or(TrimError::AtAnEnd)?;
    if chain.closed {
        return Ok(vec![slice_round(chain, here, here)]);
    }
    let first = slice(chain, start_of(), here);
    let second = slice(chain, here, end_of(chain));
    if length(&first) <= CHAIN_TOL_MM || length(&second) <= CHAIN_TOL_MM {
        return Err(TrimError::AtAnEnd);
    }
    Ok(vec![first, second])
}

/// The open chain with its `end` line made longer, along itself, to the nearest of
/// `cutters` ahead.
pub fn extend(chain: &Chain, cutters: &[Primitive], end: End) -> Result<Chain, TrimError> {
    if chain.closed {
        return Err(TrimError::Closed);
    }
    let (index, line) = match end {
        End::Start => (0, chain.prims.first()),
        End::End => (chain.prims.len().saturating_sub(1), chain.prims.last()),
    };
    let Some(Primitive::Line(line)) = line.copied() else {
        return Err(TrimError::EndNotStraight);
    };
    // Outward from the end, along the line.
    let (from, other) = match end {
        End::Start => (line.start, line.end),
        End::End => (line.end, line.start),
    };
    let length = distance(from, other);
    if length < CHAIN_TOL_MM {
        return Err(TrimError::EndNotStraight);
    }
    let dir = ((from.0 - other.0) / length, (from.1 - other.1) / length);
    let ray = Primitive::Line(Line {
        start: from,
        end: (
            from.0 + dir.0 * EXTEND_REACH_MM,
            from.1 + dir.1 * EXTEND_REACH_MM,
        ),
    });
    let ahead = cutters
        .iter()
        .flat_map(|c| intersect::intersections(&ray, c))
        .map(|p| distance(from, p))
        .filter(|&d| d > CHAIN_TOL_MM)
        .fold(f64::INFINITY, f64::min);
    if !ahead.is_finite() {
        return Err(TrimError::NothingAhead);
    }
    let to = (from.0 + dir.0 * ahead, from.1 + dir.1 * ahead);
    let mut out = chain.clone();
    out.prims[index] = Primitive::Line(match end {
        End::Start => Line { start: to, ..line },
        End::End => Line { end: to, ..line },
    });
    Ok(out)
}

/// The place on the chain nearest `p`.
pub fn nearest_place(chain: &Chain, p: Point) -> Option<Place> {
    chain
        .prims
        .iter()
        .enumerate()
        .map(|(seg, prim)| {
            let q = intersect::nearest_on(prim, p);
            (
                distance(p, q),
                Place {
                    seg,
                    t: share_along(prim, q),
                },
            )
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, place)| place)
}

/// How far along `prim` the point `p` (on it) lies: 0 at its start, 1 at its end.
fn share_along(prim: &Primitive, p: Point) -> f64 {
    match prim {
        Primitive::Line(l) => {
            let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
            let squared = dx * dx + dy * dy;
            if squared == 0.0 {
                0.0
            } else {
                (((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / squared).clamp(0.0, 1.0)
            }
        }
        Primitive::Arc(a) => {
            if a.sweep == 0.0 {
                return 0.0;
            }
            let angle = (p.1 - a.center.1).atan2(p.0 - a.center.0);
            let tau = std::f64::consts::TAU;
            let along = if a.sweep >= 0.0 {
                (angle - a.start_angle).rem_euclid(tau)
            } else {
                (a.start_angle - angle).rem_euclid(tau)
            };
            // Just before the start (by the tolerance) counts as the start.
            let along = if along > a.sweep.abs() + (tau - a.sweep.abs()) / 2.0 {
                0.0
            } else {
                along
            };
            (along / a.sweep.abs()).clamp(0.0, 1.0)
        }
    }
}

fn start_of() -> Place {
    Place { seg: 0, t: 0.0 }
}

fn end_of(chain: &Chain) -> Place {
    Place {
        seg: chain.prims.len().saturating_sub(1),
        t: 1.0,
    }
}

/// The part of `prim` from share `t0` to `t1`.
fn piece(prim: &Primitive, t0: f64, t1: f64) -> Primitive {
    match *prim {
        Primitive::Line(l) => {
            let at = |t: f64| {
                (
                    l.start.0 + (l.end.0 - l.start.0) * t,
                    l.start.1 + (l.end.1 - l.start.1) * t,
                )
            };
            Primitive::Line(Line {
                start: at(t0),
                end: at(t1),
            })
        }
        Primitive::Arc(a) => Primitive::Arc(Arc {
            start_angle: a.start_angle + a.sweep * t0,
            sweep: a.sweep * (t1 - t0),
            ..a
        }),
    }
}

/// The open chain from `from` to `to` along it (`from` before `to`).
fn slice(chain: &Chain, from: Place, to: Place) -> Chain {
    let mut prims = Vec::new();
    for seg in from.seg..=to.seg.min(chain.prims.len().saturating_sub(1)) {
        let t0 = if seg == from.seg { from.t } else { 0.0 };
        let t1 = if seg == to.seg { to.t } else { 1.0 };
        let p = piece(&chain.prims[seg], t0, t1);
        if p.length() > CHAIN_TOL_MM {
            prims.push(p);
        }
    }
    Chain {
        prims,
        closed: false,
    }
}

/// The open chain from `from` round a closed chain to `to` (all the way round when they
/// are the same place).
fn slice_round(chain: &Chain, from: Place, to: Place) -> Chain {
    if to.key() > from.key() {
        return slice(chain, from, to);
    }
    let mut first = slice(chain, from, end_of(chain));
    let second = slice(chain, start_of(), to);
    first.prims.extend(second.prims);
    first
}

fn length(chain: &Chain) -> f64 {
    chain.prims.iter().map(Primitive::length).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    fn line(a: Point, b: Point) -> Primitive {
        Primitive::Line(Line { start: a, end: b })
    }

    fn open(points: &[Point]) -> Chain {
        Chain {
            prims: points.windows(2).map(|w| line(w[0], w[1])).collect(),
            closed: false,
        }
    }

    fn total(chains: &[Chain]) -> f64 {
        chains.iter().map(length).sum()
    }

    #[test]
    fn trimming_a_line_between_two_crossings_leaves_its_ends() {
        let target = open(&[(0.0, 0.0), (30.0, 0.0)]);
        let cutters = [
            line((10.0, -5.0), (10.0, 5.0)),
            line((20.0, -5.0), (20.0, 5.0)),
        ];
        let left = trim(&target, &cutters, (15.0, 0.5)).unwrap();
        assert_eq!(left.len(), 2);
        assert!((length(&left[0]) - 10.0).abs() < 1e-9);
        assert!((left[1].prims[0].start().0 - 20.0).abs() < 1e-9);
        // At an end: only one piece stays.
        let one = trim(&target, &cutters, (2.0, 0.0)).unwrap();
        assert_eq!(one.len(), 1);
        assert!((one[0].prims[0].start().0 - 10.0).abs() < 1e-9);
        // Nothing crosses: refused.
        assert_eq!(
            trim(&target, &[], (2.0, 0.0)),
            Err(TrimError::NothingCrosses)
        );
    }

    #[test]
    fn trimming_a_circle_leaves_an_arc() {
        let circle = Chain {
            prims: vec![Primitive::Arc(Arc {
                center: (0.0, 0.0),
                radius: 10.0,
                start_angle: 0.0,
                sweep: TAU,
            })],
            closed: true,
        };
        let cutter = [line((-20.0, 0.0), (20.0, 0.0))];
        // Trim the top half: the bottom half is left, one open chain.
        let left = trim(&circle, &cutter, (0.0, 10.0)).unwrap();
        assert_eq!(left.len(), 1);
        assert!(!left[0].closed);
        assert!((total(&left) - std::f64::consts::PI * 10.0).abs() < 1e-6);
        assert!(left[0].prims.iter().all(|p| p.midpoint().1 < 0.0));
        let removed = trimmed_piece(&circle, &cutter, (0.0, 10.0)).unwrap();
        assert!(removed.prims.iter().all(|p| p.midpoint().1 > 0.0));
        // One crossing (a tangent line) isn't enough to trim a closed shape.
        assert_eq!(
            trim(&circle, &[line((-20.0, 10.0), (20.0, 10.0))], (0.0, -10.0)),
            Err(TrimError::NothingCrosses)
        );
    }

    #[test]
    fn a_shapes_own_corners_are_not_crossings() {
        // An L: its corner isn't a crossing with itself.
        let l = open(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        assert!(crossings(&l, &l.prims).is_empty());
        // A Z that crosses itself does.
        let z = open(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
        assert_eq!(crossings(&z, &z.prims).len(), 2);
    }

    #[test]
    fn breaking_and_extending() {
        let target = open(&[(0.0, 0.0), (30.0, 0.0)]);
        let two = break_at(&target, (12.0, 1.0)).unwrap();
        assert_eq!(two.len(), 2);
        assert!((length(&two[0]) - 12.0).abs() < 1e-9);
        assert_eq!(break_at(&target, (0.0, 0.0)), Err(TrimError::AtAnEnd));
        let wall = [line((50.0, -5.0), (50.0, 5.0))];
        let longer = extend(&target, &wall, End::End).unwrap();
        assert!((longer.prims[0].end().0 - 50.0).abs() < 1e-9);
        assert_eq!(
            extend(&target, &wall, End::Start),
            Err(TrimError::NothingAhead)
        );
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// A trim never adds length: what is left is the shape less a piece.
            #[test]
            fn trims_never_add_length(
                x in 1.0..29.0f64,
                cuts in proptest::collection::vec(0.5..29.5f64, 1..5),
            ) {
                let target = open(&[(0.0, 0.0), (30.0, 0.0)]);
                let cutters: Vec<Primitive> =
                    cuts.iter().map(|&c| line((c, -1.0), (c, 1.0))).collect();
                if let Ok(left) = trim(&target, &cutters, (x, 0.0)) {
                    prop_assert!(total(&left) <= 30.0 + 1e-9);
                }
            }
        }
    }
}
