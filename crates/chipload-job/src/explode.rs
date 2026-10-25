// SPDX-License-Identifier: GPL-3.0-or-later
//! An imported path as drawn shapes, and open polylines whose ends lie within a small gap
//! joined into one (docs/specs/explode-and-join.md §2, §3). Plain values in, plain values
//! out; the pipeline applies them to a project.

use std::f64::consts::{PI, TAU};

use chipload_core::primitives::{Arc, Primitive};
use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};

use crate::edit::insert_point_after;
use crate::models::{Geometry, PathSegment, shape_problems};

/// The gap *Join* closes unless told otherwise, mm.
pub const DEFAULT_JOIN_GAP_MM: f64 = 0.01;
/// The widest gap *Join* takes, mm: further apart, a gap is usually meant.
pub const MAX_JOIN_GAP_MM: f64 = 1.0;

/// Whether `gap` (mm) is one *Join* takes: at least the chaining tolerance, at most
/// [`MAX_JOIN_GAP_MM`].
pub fn gap_allowed(gap: f64) -> bool {
    (CHAIN_TOL_MM..=MAX_JOIN_GAP_MM).contains(&gap)
}

/// An imported path's segments as drawn shapes (§2): a full circle as a circle (which
/// starts at angle 0, counter-clockwise, whatever the DXF's did), anything else as polylines, one per run of segments that follow on. Arcs of more
/// than a half circle are split at their middles; segments shorter than the chaining
/// tolerance are left out. Empty when nothing of the path is left.
pub fn path_to_drawn(segments: &[PathSegment]) -> Vec<Geometry> {
    let prims: Vec<Primitive> = segments
        .iter()
        .map(PathSegment::to_primitive)
        .filter(|p| p.length() > CHAIN_TOL_MM)
        .collect();
    if let [Primitive::Arc(arc)] = &prims[..]
        && (arc.sweep.abs() - TAU).abs() <= ANGLE_TOL_RAD
    {
        return vec![Geometry::Circle {
            center: [arc.center.0, arc.center.1],
            radius: arc.radius,
        }];
    }
    runs(&prims)
        .iter()
        .filter_map(|run| polyline(run))
        .collect()
}

/// `prims` split where one doesn't start where the one before ends.
fn runs(prims: &[Primitive]) -> Vec<Vec<Primitive>> {
    let mut runs: Vec<Vec<Primitive>> = Vec::new();
    for prim in prims {
        match runs.last_mut() {
            Some(run) if run.last().is_some_and(|p| close(p.end(), prim.start())) => {
                run.push(*prim);
            }
            _ => runs.push(vec![*prim]),
        }
    }
    runs
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) <= CHAIN_TOL_MM
}

/// One run of segments as a polyline, closed when it ends where it starts.
fn polyline(run: &[Primitive]) -> Option<Geometry> {
    let first = run.first()?.start();
    let mut points = vec![[first.0, first.1]];
    let mut bulges = Vec::new();
    for prim in run {
        for (end, bulge) in pieces(prim) {
            let last = points[points.len() - 1];
            if close((last[0], last[1]), end) {
                continue;
            }
            points.push([end.0, end.1]);
            bulges.push(bulge);
        }
    }
    let n = points.len();
    let closes = n > 2
        && close(
            (points[0][0], points[0][1]),
            (points[n - 1][0], points[n - 1][1]),
        );
    let mut geometry = if closes {
        points.pop();
        Geometry::Polyline {
            closed: true,
            points,
            bulges,
        }
    } else {
        Geometry::Polyline {
            closed: false,
            points,
            bulges,
        }
    };
    enough_points(&mut geometry);
    shape_problems(&geometry).is_empty().then_some(geometry)
}

/// A primitive as polyline segments: each one's end point and bulge. An arc of more than a
/// half circle becomes equal parts of at most a half circle each; the last ends exactly
/// where the arc does.
fn pieces(prim: &Primitive) -> Vec<((f64, f64), f64)> {
    match prim {
        Primitive::Line(line) => vec![(line.end, 0.0)],
        Primitive::Arc(arc) => {
            let count = (arc.sweep.abs() / PI - ANGLE_TOL_RAD).ceil().max(1.0) as usize;
            let sweep = arc.sweep / count as f64;
            (1..=count)
                .map(|i| {
                    let end = if i == count {
                        prim.end()
                    } else {
                        arc.point_at(arc.start_angle + sweep * i as f64)
                    };
                    (end, part_bulge(arc, sweep))
                })
                .collect()
        }
    }
}

fn part_bulge(arc: &Arc, sweep: f64) -> f64 {
    Arc { sweep, ..*arc }.bulge()
}

/// A closed polyline needs three points, or two with a bent segment: split its most bent
/// segments until it has them. One made of two straight segments can't be, and is left
/// open (back and forth).
fn enough_points(geometry: &mut Geometry) {
    loop {
        let Geometry::Polyline {
            closed,
            points,
            bulges,
        } = geometry
        else {
            return;
        };
        let bends = bulges.iter().any(|b| *b != 0.0);
        if !*closed || points.len() >= 3 || (points.len() == 2 && bends) {
            return;
        }
        let most = bulges
            .iter()
            .enumerate()
            .filter(|(_, b)| **b != 0.0)
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
        match most {
            Some((i, _)) => {
                insert_point_after(geometry, i);
            }
            None => {
                let first = points[0];
                points.push(first);
                *closed = false;
                bulges.truncate(points.len() - 1);
                bulges.resize(points.len() - 1, 0.0);
                return;
            }
        }
    }
}

/// One shape given to [`join`].
#[derive(Debug, Clone, Copy)]
pub struct JoinPiece<'a> {
    pub id: u64,
    pub layer: &'a str,
    pub geometry: &'a Geometry,
}

/// Pieces joined into one polyline (or one closed on itself).
#[derive(Debug, Clone, PartialEq)]
pub struct Joined {
    /// The id the joined shape keeps: the lowest of its pieces.
    pub id: u64,
    /// Every piece in it, the kept one first, then the others by id.
    pub pieces: Vec<u64>,
    /// Whether it closes.
    pub closed: bool,
    pub geometry: Geometry,
}

/// An end left open after joining, and how far the nearest other end on its layer is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenEnd {
    pub at: [f64; 2],
    /// None when no other end is on its layer.
    pub nearest: Option<f64>,
}

/// What [`join`] did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JoinOutcome {
    pub joined: Vec<Joined>,
    /// The widest gap closed, mm; 0 when none was.
    pub widest: f64,
    /// Ends that stay open, nearest other end first (ends at a branch aren't among them).
    pub open_ends: Vec<OpenEnd>,
    /// Where three or more ends lie within the gap: nothing joins there.
    pub branches: Vec<[f64; 2]>,
}

/// An end of a candidate: the piece's index and which end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct End {
    piece: usize,
    last: bool,
}

/// Join open polylines whose ends lie within `gap` (mm) of each other, on the same layer
/// (§3): an end joins another only when each is the other's only end that close; both move
/// to their midpoint. A chain that comes back to its start closes. Circles, rectangles and
/// closed polylines are left alone. The same pieces in any order give the same outcome.
pub fn join(pieces: &[JoinPiece], gap: f64) -> JoinOutcome {
    let mut candidates: Vec<&JoinPiece> = pieces
        .iter()
        .filter(|p| matches!(p.geometry, Geometry::Polyline { closed: false, points, .. } if points.len() >= 2))
        .collect();
    candidates.sort_by_key(|p| p.id);
    let layers: Vec<String> = candidates.iter().map(|p| p.layer.to_lowercase()).collect();
    let ends: Vec<End> = (0..candidates.len())
        .flat_map(|piece| [End { piece, last: false }, End { piece, last: true }])
        .collect();
    let at = |e: End| end_point(candidates[e.piece].geometry, e.last);
    let near = near_ends(&ends, &at, &layers, gap);

    let index = |e: End| 2 * e.piece + usize::from(e.last);
    let mut partner: Vec<Option<End>> = vec![None; ends.len()];
    let mut branches: Vec<[f64; 2]> = Vec::new();
    for &e in &ends {
        match near[index(e)][..] {
            [f] if near[index(f)][..] == [e] => partner[index(e)] = Some(f),
            [_, _, ..] => {
                let p = at(e);
                if !branches.iter().any(|b| distance(*b, p) <= 2.0 * gap) {
                    branches.push(p);
                }
            }
            _ => {}
        }
    }

    let mut outcome = JoinOutcome {
        branches,
        ..JoinOutcome::default()
    };
    let mut done = vec![false; candidates.len()];
    let mut free: Vec<End> = Vec::new();
    for k in 0..candidates.len() {
        if done[k] {
            continue;
        }
        let (chain, cycle) = chain_through(k, &partner);
        for &(piece, _) in &chain {
            done[piece] = true;
        }
        let joins = |outcome: &mut JoinOutcome| {
            let built = build(&chain, cycle, &candidates)?;
            let mut ids: Vec<u64> = chain.iter().map(|&(p, _)| candidates[p].id).collect();
            ids.sort_unstable();
            outcome.widest = outcome.widest.max(built.1);
            outcome.joined.push(Joined {
                id: ids[0],
                pieces: ids,
                closed: cycle,
                geometry: built.0,
            });
            Some(())
        };
        if (chain.len() > 1 || cycle) && joins(&mut outcome).is_some() {
            if !cycle {
                let (first, first_reversed) = chain[0];
                let (last, last_reversed) = chain[chain.len() - 1];
                free.push(End {
                    piece: first,
                    last: first_reversed,
                });
                free.push(End {
                    piece: last,
                    last: !last_reversed,
                });
            }
        } else {
            // Not joined (or its result would be no valid shape): every end stays.
            for &(piece, _) in &chain {
                free.push(End { piece, last: false });
                free.push(End { piece, last: true });
            }
        }
    }
    outcome.open_ends = open_ends(&free, &at, &layers, gap);
    outcome
}

/// For each end (by `2 · piece + last`), the other ends on its layer within `gap`, in
/// order. Ends sorted by x, so each looks only at those within `gap` in x.
fn near_ends(
    ends: &[End],
    at: &impl Fn(End) -> [f64; 2],
    layers: &[String],
    gap: f64,
) -> Vec<Vec<End>> {
    let mut near: Vec<Vec<End>> = vec![Vec::new(); ends.len()];
    let mut by_x: Vec<End> = ends.to_vec();
    by_x.sort_by(|a, b| at(*a)[0].total_cmp(&at(*b)[0]).then(a.cmp(b)));
    for (i, &e) in by_x.iter().enumerate() {
        let p = at(e);
        for &f in &by_x[i + 1..] {
            let q = at(f);
            if q[0] - p[0] > gap {
                break;
            }
            if layers[e.piece] == layers[f.piece] && distance(p, q) <= gap {
                near[2 * e.piece + usize::from(e.last)].push(f);
                near[2 * f.piece + usize::from(f.last)].push(e);
            }
        }
    }
    for list in &mut near {
        list.sort();
    }
    near
}

/// The ends in `free` with the distance to the nearest other free end on their layer, the
/// nearest first; ends with another within `gap` (a branch) left out.
fn open_ends(
    free: &[End],
    at: &impl Fn(End) -> [f64; 2],
    layers: &[String],
    gap: f64,
) -> Vec<OpenEnd> {
    let mut out: Vec<OpenEnd> = free
        .iter()
        .map(|&e| {
            let p = at(e);
            let nearest = free
                .iter()
                .filter(|&&f| f != e && layers[f.piece] == layers[e.piece])
                .map(|&f| distance(p, at(f)))
                .min_by(f64::total_cmp);
            OpenEnd { at: p, nearest }
        })
        .filter(|o| o.nearest.is_none_or(|d| d > gap))
        .collect();
    out.sort_by(|a, b| {
        let key = |o: &OpenEnd| o.nearest.unwrap_or(f64::INFINITY);
        key(a)
            .total_cmp(&key(b))
            .then(a.at[0].total_cmp(&b.at[0]))
            .then(a.at[1].total_cmp(&b.at[1]))
    });
    out
}

/// The chain of pieces through piece `k`, each with whether it runs reversed, following
/// `k`'s direction; and whether it's a cycle (then it starts at `k`).
fn chain_through(k: usize, partner: &[Option<End>]) -> (Vec<(usize, bool)>, bool) {
    // Back from k's start to the chain's first piece.
    let mut begin = (k, false);
    let mut cycle = false;
    loop {
        let start = End {
            piece: begin.0,
            last: begin.1,
        };
        let Some(before) = partner[2 * start.piece + usize::from(start.last)] else {
            break;
        };
        // The piece before runs so that its last end (in the chain's direction) is `before`.
        let previous = (before.piece, !before.last);
        if previous.0 == k {
            cycle = true;
            begin = (k, false);
            break;
        }
        begin = previous;
    }
    let mut chain = vec![begin];
    loop {
        let (piece, reversed) = chain[chain.len() - 1];
        let Some(next) = partner[2 * piece + usize::from(!reversed)] else {
            break;
        };
        if next.piece == begin.0 {
            break;
        }
        chain.push((next.piece, next.last));
    }
    (chain, cycle)
}

/// The joined polyline of `chain`, and the widest gap it closed; None when the result
/// isn't a valid shape (a piece shorter than the gap collapsed).
fn build(
    chain: &[(usize, bool)],
    cycle: bool,
    candidates: &[&JoinPiece],
) -> Option<(Geometry, f64)> {
    let mut points: Vec<[f64; 2]> = Vec::new();
    let mut bulges: Vec<f64> = Vec::new();
    let mut widest: f64 = 0.0;
    for &(piece, reversed) in chain {
        let (mut p, mut b) = polyline_values(candidates[piece].geometry)?;
        if reversed {
            p.reverse();
            b.reverse();
            b.iter_mut().for_each(|x| *x = -*x);
        }
        if let Some(last) = points.last_mut() {
            widest = widest.max(distance(*last, p[0]));
            *last = midpoint(*last, p[0]);
            points.extend_from_slice(&p[1..]);
        } else {
            points = p;
        }
        bulges.extend(b);
    }
    if cycle {
        let last = points.pop()?;
        widest = widest.max(distance(last, points[0]));
        points[0] = midpoint(last, points[0]);
    }
    let mut geometry = Geometry::Polyline {
        closed: cycle,
        points,
        bulges,
    };
    enough_points(&mut geometry);
    let valid = matches!(geometry, Geometry::Polyline { closed, .. } if closed == cycle)
        && shape_problems(&geometry).is_empty();
    valid.then_some((geometry, widest))
}

fn polyline_values(geometry: &Geometry) -> Option<(Vec<[f64; 2]>, Vec<f64>)> {
    match geometry {
        Geometry::Polyline { points, bulges, .. } => Some((points.clone(), bulges.clone())),
        _ => None,
    }
}

fn end_point(geometry: &Geometry, last: bool) -> [f64; 2] {
    match geometry {
        Geometry::Polyline { points, .. } if !points.is_empty() => {
            if last {
                points[points.len() - 1]
            } else {
                points[0]
            }
        }
        _ => [f64::NAN, f64::NAN],
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn midpoint(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ArcSegment, LineSegment};

    fn line(a: [f64; 2], b: [f64; 2]) -> PathSegment {
        PathSegment::Line(LineSegment { start: a, end: b })
    }

    fn arc(center: [f64; 2], radius: f64, start_angle: f64, sweep: f64) -> PathSegment {
        PathSegment::Arc(ArcSegment {
            center,
            radius,
            start_angle,
            sweep,
        })
    }

    fn open(points: Vec<[f64; 2]>) -> Geometry {
        Geometry::straight_polyline(false, points)
    }

    /// Every end point of `a` within `tol` of the matching one of `b`.
    fn same_path(a: &[PathSegment], b: &[PathSegment], tol: f64) {
        let ends = |s: &[PathSegment]| -> Vec<(f64, f64)> {
            s.iter()
                .flat_map(|s| {
                    let p = s.to_primitive();
                    [p.start(), p.end()]
                })
                .collect()
        };
        let (ea, eb) = (ends(a), ends(b));
        assert_eq!(ea.len(), eb.len(), "{a:?}\n{b:?}");
        for (p, q) in ea.iter().zip(&eb) {
            assert!((p.0 - q.0).hypot(p.1 - q.1) <= tol, "{p:?} vs {q:?}");
        }
    }

    #[test]
    fn a_circle_from_angle_0_stays_a_circle() {
        let drawn = path_to_drawn(&[arc([5.0, 6.0], 2.0, 0.0, TAU)]);
        assert_eq!(
            drawn,
            vec![Geometry::Circle {
                center: [5.0, 6.0],
                radius: 2.0
            }]
        );
    }

    #[test]
    fn a_turned_or_clockwise_circle_is_a_circle_too() {
        for (start, sweep) in [(1.0, TAU), (0.0, -TAU)] {
            let drawn = path_to_drawn(&[arc([0.0, 0.0], 2.0, start, sweep)]);
            assert_eq!(
                drawn,
                vec![Geometry::Circle {
                    center: [0.0, 0.0],
                    radius: 2.0
                }]
            );
        }
    }

    #[test]
    fn a_polyline_circle_of_two_half_circles_keeps_its_two_points() {
        let path = [arc([0.0, 0.0], 2.0, 0.0, PI), arc([0.0, 0.0], 2.0, PI, PI)];
        let drawn = path_to_drawn(&path);
        let [
            Geometry::Polyline {
                closed: true,
                points,
                bulges,
            },
        ] = &drawn[..]
        else {
            panic!("{drawn:?}");
        };
        assert_eq!(points.len(), 2);
        assert!(bulges.iter().all(|b| (b - 1.0).abs() < 1e-12));
        assert!(shape_problems(&drawn[0]).is_empty());
    }

    #[test]
    fn a_closed_polyline_of_two_straight_segments_is_refused() {
        let back_and_forth = Geometry::Polyline {
            closed: true,
            points: vec![[0.0, 0.0], [5.0, 0.0]],
            bulges: vec![0.0, 0.0],
        };
        assert!(!shape_problems(&back_and_forth).is_empty());
        let mut kept = back_and_forth.clone();
        enough_points(&mut kept);
        assert!(
            matches!(kept, Geometry::Polyline { closed: false, ref points, .. } if points.len() == 3)
        );
    }

    #[test]
    fn lines_and_arcs_keep_their_points() {
        let path = [
            line([0.0, 0.0], [10.0, 0.0]),
            arc([10.0, 5.0], 5.0, -PI / 2.0, PI / 2.0),
            line([15.0, 5.0], [0.0, 5.0]),
            line([0.0, 5.0], [0.0, 0.0]),
        ];
        let drawn = path_to_drawn(&path);
        assert_eq!(drawn.len(), 1);
        assert!(matches!(drawn[0], Geometry::Polyline { closed: true, .. }));
        let shape = crate::models::Shape {
            id: 1,
            layer: "0".into(),
            name: None,
            geometry: drawn[0].clone(),
        };
        same_path(&shape.segments(), &path, 1e-9);
    }

    #[test]
    fn an_arc_over_a_half_circle_is_split_and_a_clockwise_one_keeps_its_way() {
        let path = [arc([0.0, 0.0], 3.0, 0.0, -1.5 * PI)];
        let drawn = path_to_drawn(&path);
        let Geometry::Polyline { points, bulges, .. } = &drawn[0] else {
            panic!();
        };
        assert_eq!(points.len(), 3);
        assert!(bulges.iter().all(|b| *b < 0.0 && b.abs() <= 1.0));
        let shape = crate::models::Shape {
            id: 1,
            layer: "0".into(),
            name: None,
            geometry: drawn[0].clone(),
        };
        let segments = shape.segments();
        let sweep: f64 = segments
            .iter()
            .map(|s| match s {
                PathSegment::Arc(a) => a.sweep,
                PathSegment::Line(_) => 0.0,
            })
            .sum();
        assert!((sweep + 1.5 * PI).abs() < 1e-9);
    }

    #[test]
    fn a_path_in_two_runs_is_two_polylines_and_tiny_segments_go() {
        let path = [
            line([0.0, 0.0], [10.0, 0.0]),
            line([10.0, 0.0], [10.0, 0.0005]),
            line([10.0, 0.0005], [10.0, 10.0]),
            line([20.0, 0.0], [30.0, 0.0]),
        ];
        let drawn = path_to_drawn(&path);
        assert_eq!(drawn.len(), 2);
        let Geometry::Polyline { points, .. } = &drawn[0] else {
            panic!();
        };
        assert_eq!(points, &vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]);
    }

    #[test]
    fn two_pieces_join_within_the_gap_and_not_beyond() {
        let a = open(vec![[0.0, 0.0], [10.0, 0.0]]);
        let b = open(vec![[10.03, 0.0], [20.0, 0.0]]);
        let pieces = [
            JoinPiece {
                id: 4,
                layer: "Cut",
                geometry: &b,
            },
            JoinPiece {
                id: 2,
                layer: "cut",
                geometry: &a,
            },
        ];
        let outcome = join(&pieces, DEFAULT_JOIN_GAP_MM);
        assert!(outcome.joined.is_empty());
        assert_eq!(outcome.open_ends.len(), 4);
        assert!((outcome.open_ends[0].nearest.unwrap() - 0.03).abs() < 1e-9);

        let outcome = join(&pieces, 0.05);
        assert_eq!(outcome.joined.len(), 1);
        let joined = &outcome.joined[0];
        assert_eq!(
            (joined.id, &joined.pieces[..], joined.closed),
            (2, &[2, 4][..], false)
        );
        let Geometry::Polyline { points, .. } = &joined.geometry else {
            panic!();
        };
        assert_eq!(points.len(), 3);
        assert!((points[1][0] - 10.015).abs() < 1e-12 && points[1][1] == 0.0);
        assert!((outcome.widest - 0.03).abs() < 1e-9);
    }

    #[test]
    fn reversed_pieces_turn_and_a_loop_closes() {
        // A square of four lines, the second and fourth drawn the other way, with gaps.
        let pieces = [
            open(vec![[0.0, 0.0], [10.0, 0.0]]),
            open(vec![[10.0, 10.0], [10.0, 0.004]]),
            open(vec![[10.0, 10.003], [0.0, 10.0]]),
            Geometry::Polyline {
                closed: false,
                points: vec![[0.0, 0.002], [0.0, 10.0]],
                bulges: vec![0.3],
            },
        ];
        let given: Vec<JoinPiece> = pieces
            .iter()
            .enumerate()
            .map(|(i, g)| JoinPiece {
                id: i as u64 + 1,
                layer: "0",
                geometry: g,
            })
            .collect();
        let outcome = join(&given, DEFAULT_JOIN_GAP_MM);
        assert_eq!(outcome.joined.len(), 1);
        let joined = &outcome.joined[0];
        assert!(joined.closed);
        assert_eq!(joined.pieces, vec![1, 2, 3, 4]);
        let Geometry::Polyline {
            closed,
            points,
            bulges,
        } = &joined.geometry
        else {
            panic!();
        };
        assert!(*closed);
        assert_eq!(points.len(), 4);
        assert_eq!(points[0], [0.0, 0.001]);
        assert_eq!(points[1], [10.0, 0.002]);
        // The fourth piece runs backwards: its bulge turns.
        assert_eq!(bulges, &vec![0.0, 0.0, 0.0, -0.3]);
        assert!(outcome.open_ends.is_empty());
    }

    #[test]
    fn a_branch_stays_open_and_other_layers_and_closed_shapes_dont_join() {
        let a = open(vec![[0.0, 0.0], [10.0, 0.0]]);
        let b = open(vec![[10.0, 0.001], [20.0, 0.0]]);
        let c = open(vec![[10.0, -0.001], [10.0, -10.0]]);
        let d = open(vec![[20.0, 0.002], [30.0, 0.0]]);
        let closed = Geometry::straight_polyline(true, vec![[30.0, 0.0], [40.0, 0.0], [35.0, 5.0]]);
        let pieces = [
            JoinPiece {
                id: 1,
                layer: "0",
                geometry: &a,
            },
            JoinPiece {
                id: 2,
                layer: "0",
                geometry: &b,
            },
            JoinPiece {
                id: 3,
                layer: "0",
                geometry: &c,
            },
            JoinPiece {
                id: 4,
                layer: "other",
                geometry: &d,
            },
            JoinPiece {
                id: 5,
                layer: "0",
                geometry: &closed,
            },
        ];
        let outcome = join(&pieces, DEFAULT_JOIN_GAP_MM);
        assert!(outcome.joined.is_empty());
        assert_eq!(outcome.branches.len(), 1);
    }

    #[test]
    fn the_same_pieces_in_any_order_join_the_same() {
        let shapes: Vec<Geometry> = (0..6)
            .map(|i| {
                let x = i as f64 * 10.0;
                open(vec![[x + 0.002 * (i % 3) as f64, 0.0], [x + 10.0, 0.0]])
            })
            .collect();
        let mut given: Vec<JoinPiece> = shapes
            .iter()
            .enumerate()
            .map(|(i, g)| JoinPiece {
                id: i as u64 + 10,
                layer: "0",
                geometry: g,
            })
            .collect();
        let first = join(&given, DEFAULT_JOIN_GAP_MM);
        given.reverse();
        assert_eq!(join(&given, DEFAULT_JOIN_GAP_MM), first);
        assert_eq!(first.joined.len(), 1);
        assert_eq!(first.joined[0].pieces, vec![10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn gaps_have_limits() {
        assert!(gap_allowed(DEFAULT_JOIN_GAP_MM));
        assert!(gap_allowed(MAX_JOIN_GAP_MM));
        assert!(!gap_allowed(0.0));
        assert!(!gap_allowed(1.5));
        assert!(!gap_allowed(f64::NAN));
    }
}
