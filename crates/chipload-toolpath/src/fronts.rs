// SPDX-License-Identifier: GPL-3.0-or-later
//! Adaptive clearing by fronts (docs/specs/adaptive-fronts.md), designed and written for
//! Chipload.
//!
//! In tool-centre terms, the cleared area K grows from a start circle one cut width at a
//! time: K' = (K grown by w) ∩ R, where R is where the clearing passes may go. The new parts
//! of K's edge are the passes, exact arcs and lines. Passes are cut in an order that
//! finishes a branch before starting the next (a pass waits only for the passes it borders),
//! and joined by straight returns where the material grid says those run through cleared
//! material. The grid also measures the engagement of every step, for tests.

use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use chipload_core::Point;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};

use crate::ToolpathError;
use crate::adaptive::Grid;
use crate::arcs::{ArcTable, Circle};
use crate::geom::PrimIndex;
use crate::left::area_intersection;
use crate::offset::{pline_segments, point_distance};
use crate::pocket::{Area, first_point, geometry};

/// Most growth steps in one part of the clearing area, against runaway loops.
const MAX_STEPS: usize = 100_000;
/// Front points within this of the cleared area are already cut.
const OLD_TOL_MM: f64 = ARC_APPROX_TOL_MM;
/// A return may cut at most this share of a pass's cut per step (grid noise near edges).
const RETURN_CUT_SHARE: f64 = 0.25;
/// From one closed front to the next, the next starts this many cut widths ahead.
const SPIRAL_AHEAD: f64 = 4.0;
/// Points tried, nearest first, as the place a new path goes down.
const ENTRY_CANDIDATES: usize = 64;
/// Bisection rounds for the largest circle (the result is within 2^-30 of the bound).
const START_ROUNDS: usize = 30;

/// What the fronts need: where the passes go, how wide they cut, and the material model.
pub struct Setup<'a> {
    /// Parts of the area the tool centre may be in during clearing (the pocket shrunk by the
    /// tool-centre offset and the cut width).
    pub parts: &'a [Area],
    /// Cut width w, mm.
    pub width: f64,
    /// true: material on the right of the direction of travel.
    pub material_right: bool,
    /// The material the passes cut (1 = uncut), and the radius the model cuts with.
    pub material: Grid,
    pub reach: f64,
}

/// The passes (open paths, in cut order, arcs recorded in `arcs`) and the engagement of
/// every step along the fronts as a share of the cut width.
#[derive(Debug, Default)]
pub struct Fronts {
    pub paths: Vec<Vec<Point>>,
    pub engagement: Vec<f64>,
}

/// One new stretch of the cleared area's edge: a pass.
struct Stretch {
    step: usize,
    prims: Vec<Primitive>,
    closed: bool,
}

/// Clear every part, nearest first from `position`.
pub fn clear(
    setup: Setup<'_>,
    position: Point,
    arcs: &mut ArcTable,
) -> Result<Fronts, ToolpathError> {
    let Setup {
        parts,
        width,
        material_right,
        mut material,
        reach,
    } = setup;
    if width.is_nan() || width <= 0.0 {
        return Err(geometry(format!(
            "adaptive: cut width must be positive, got {width}"
        )));
    }
    let mut starts = Vec::new();
    for part in parts {
        if let Some(start) = largest_circle(part)? {
            starts.push((part, start));
        }
    }
    let mut out = Fronts::default();
    let mut runner = Runner {
        edge: PrimIndex::new(Vec::new(), 1.0),
        part: None,
        grid: &mut material,
        reach,
        width,
        arcs,
        out: &mut out,
        current: Vec::new(),
        last: Vec::new(),
        last_closed: false,
        visited: Vec::new(),
        position,
    };
    while !starts.is_empty() {
        let here = runner.position;
        let next = (0..starts.len())
            .min_by(|&a, &b| {
                distance(here, starts[a].1.0).total_cmp(&distance(here, starts[b].1.0))
            })
            .unwrap_or(0);
        let (part, (start, radius)) = starts.swap_remove(next);
        let stretches = grow(part, start, radius, width, material_right)?;
        runner.finish_path();
        runner.edge = PrimIndex::new(part.boundary(), width.max(1.0));
        runner.part = Some(part);
        runner.visited.clear();
        runner.cut_in_order(&stretches, width);
    }
    runner.finish_path();
    Ok(out)
}

/// The centre and radius of the largest circle in `part` (by bisection on shrinking it),
/// None if it holds none wider than the chaining tolerance.
fn largest_circle(part: &Area) -> Result<Option<(Point, f64)>, ToolpathError> {
    let Some(extents) = part.outer.extents() else {
        return Ok(None);
    };
    let mut lo = 0.0;
    let mut hi = (extents.max_x - extents.min_x).min(extents.max_y - extents.min_y) / 2.0;
    let mut inner: Option<Area> = None;
    for _ in 0..START_ROUNDS {
        let mid = (lo + hi) / 2.0;
        match part.shrunk(mid)?.into_iter().next() {
            Some(area) => {
                lo = mid;
                inner = Some(area);
            }
            None => hi = mid,
        }
        if hi - lo < CHAIN_TOL_MM {
            break;
        }
    }
    let Some(inner) = inner else {
        return Ok(None);
    };
    if lo < 2.0 * CHAIN_TOL_MM {
        return Ok(None);
    }
    Ok(Some((middle_of(&inner), lo)))
}

/// A point of a small area near the middle of its box (the area is what is left of a
/// shrink just short of vanishing: a speck or a thin sliver along a centre line).
fn middle_of(area: &Area) -> Point {
    let points: Vec<Point> = pline_segments(&area.outer)
        .iter()
        .flat_map(Primitive::points)
        .collect();
    let Some(extents) = area.outer.extents() else {
        return first_point(&area.outer);
    };
    let centre = (
        (extents.min_x + extents.max_x) / 2.0,
        (extents.min_y + extents.max_y) / 2.0,
    );
    if area.contains(centre) {
        return centre;
    }
    points
        .into_iter()
        .min_by(|a, b| distance(*a, centre).total_cmp(&distance(*b, centre)))
        .unwrap_or_else(|| first_point(&area.outer))
}

/// A closed counter-clockwise circle.
fn circle_pline(centre: Point, radius: f64) -> Polyline<f64> {
    let mut pline = Polyline::new_closed();
    pline.add_vertex(PlineVertex::new(centre.0 + radius, centre.1, 1.0));
    pline.add_vertex(PlineVertex::new(centre.0 - radius, centre.1, 1.0));
    pline
}

/// The stretches of every front of one part, from the start circle out.
fn grow(
    part: &Area,
    start: Point,
    largest: f64,
    width: f64,
    material_right: bool,
) -> Result<Vec<Stretch>, ToolpathError> {
    let radius = width.min(largest);
    let first = Area::of(circle_pline(start, radius));
    // The first pass: out from the start and once round the start circle.
    let mut circle = oriented(&first.outer, material_right);
    rotate_to(&mut circle, (start.0 + radius, start.1));
    let mut lead = vec![Primitive::Line(Line {
        start,
        end: circle.first().map(Primitive::start).unwrap_or(start),
    })];
    lead.extend(circle);
    let mut stretches = vec![Stretch {
        step: 0,
        prims: lead,
        closed: false,
    }];
    let piece = (width / 2.0).clamp(CHAIN_TOL_MM, 1.0);
    let mut cleared = first;
    for step in 1..MAX_STEPS {
        let mut next: Option<Area> = None;
        for grown in cleared.grown(width)? {
            for area in area_intersection(&grown, part)? {
                if area.contains(start) {
                    next = Some(area);
                }
            }
        }
        let Some(next) = next else {
            break;
        };
        let old = PrimIndex::new(cleared.boundary(), width.max(1.0));
        let found = new_stretches(&next, &old, step, piece, material_right);
        if found.is_empty() {
            break;
        }
        stretches.extend(found);
        cleared = next;
    }
    Ok(stretches)
}

/// The loop's pieces in cutting order: as stored (outer counter-clockwise, holes clockwise)
/// keeps the cleared area on the left, the material on the right.
fn oriented(pline: &Polyline<f64>, material_right: bool) -> Vec<Primitive> {
    let prims = pline_segments(pline);
    if material_right {
        prims
    } else {
        prims.iter().rev().map(reversed).collect()
    }
}

fn reversed(prim: &Primitive) -> Primitive {
    match *prim {
        Primitive::Line(l) => Primitive::Line(Line {
            start: l.end,
            end: l.start,
        }),
        Primitive::Arc(a) => Primitive::Arc(Arc {
            start_angle: a.start_angle + a.sweep,
            sweep: -a.sweep,
            ..a
        }),
    }
}

/// Start the closed loop `length` further along its path.
fn advance(prims: &mut Vec<Primitive>, length: f64) {
    let mut along = 0.0;
    for k in 0..prims.len() {
        let size = prims[k].length();
        if along + size > length {
            let t = if size > 0.0 {
                (length - along) / size
            } else {
                0.0
            };
            start_at(prims, k, t);
            return;
        }
        along += size;
    }
}

/// Start the closed loop at the point of it nearest `p`.
fn rotate_to(prims: &mut Vec<Primitive>, p: Point) {
    let Some(k) = (0..prims.len())
        .min_by(|&a, &b| point_distance(&prims[a], p).total_cmp(&point_distance(&prims[b], p)))
    else {
        return;
    };
    let t = nearest_param(&prims[k], p);
    start_at(prims, k, t);
}

/// Where along `prim` (0 at its start, 1 at its end) the point nearest `p` lies.
fn nearest_param(prim: &Primitive, p: Point) -> f64 {
    match *prim {
        Primitive::Line(l) => {
            let (dx, dy) = (l.end.0 - l.start.0, l.end.1 - l.start.1);
            let length2 = dx * dx + dy * dy;
            if length2 <= 0.0 {
                return 0.0;
            }
            (((p.0 - l.start.0) * dx + (p.1 - l.start.1) * dy) / length2).clamp(0.0, 1.0)
        }
        Primitive::Arc(a) => {
            if a.sweep == 0.0 {
                return 0.0;
            }
            let angle = (p.1 - a.center.1).atan2(p.0 - a.center.0);
            let turn = std::f64::consts::TAU;
            let along = if a.sweep > 0.0 {
                (angle - a.start_angle).rem_euclid(turn)
            } else {
                (a.start_angle - angle).rem_euclid(turn)
            };
            if along <= a.sweep.abs() {
                along / a.sweep.abs()
            } else if distance(prim.start(), p) <= distance(prim.end(), p) {
                0.0
            } else {
                1.0
            }
        }
    }
}

/// `prim` split at `t` (0..1 along it).
fn split_at(prim: &Primitive, t: f64) -> (Primitive, Primitive) {
    match *prim {
        Primitive::Line(l) => {
            let m = (
                l.start.0 + (l.end.0 - l.start.0) * t,
                l.start.1 + (l.end.1 - l.start.1) * t,
            );
            (
                Primitive::Line(Line {
                    start: l.start,
                    end: m,
                }),
                Primitive::Line(Line {
                    start: m,
                    end: l.end,
                }),
            )
        }
        Primitive::Arc(a) => (
            Primitive::Arc(Arc {
                sweep: a.sweep * t,
                ..a
            }),
            Primitive::Arc(Arc {
                start_angle: a.start_angle + a.sweep * t,
                sweep: a.sweep * (1.0 - t),
                ..a
            }),
        ),
    }
}

/// Start the closed loop at `t` along its primitive `k`.
fn start_at(prims: &mut Vec<Primitive>, k: usize, t: f64) {
    let (before, after) = split_at(&prims[k], t);
    let mut loop_: Vec<Primitive> = Vec::with_capacity(prims.len() + 1);
    loop_.push(after);
    loop_.extend(prims[k + 1..].iter().copied());
    loop_.extend(prims[..k].iter().copied());
    loop_.push(before);
    loop_.retain(|p| p.length() >= CHAIN_TOL_MM);
    *prims = loop_;
}

/// Two consecutive pieces of the same line or arc as one (None for a line and an arc).
fn joined(first: &Primitive, second: &Primitive) -> Option<Primitive> {
    match (*first, *second) {
        (Primitive::Line(a), Primitive::Line(b)) => Some(Primitive::Line(Line {
            start: a.start,
            end: b.end,
        })),
        (Primitive::Arc(a), Primitive::Arc(b)) => Some(Primitive::Arc(Arc {
            sweep: a.sweep + b.sweep,
            ..a
        })),
        _ => None,
    }
}

/// `prim` in pieces no longer than `most`.
fn split(prim: &Primitive, most: f64) -> Vec<Primitive> {
    let n = (prim.length() / most).ceil().max(1.0) as usize;
    match *prim {
        Primitive::Line(l) => (0..n)
            .map(|i| {
                let at = |k: usize| {
                    let t = k as f64 / n as f64;
                    (
                        l.start.0 + (l.end.0 - l.start.0) * t,
                        l.start.1 + (l.end.1 - l.start.1) * t,
                    )
                };
                let end = if i + 1 == n { l.end } else { at(i + 1) };
                let start = if i == 0 { l.start } else { at(i) };
                Primitive::Line(Line { start, end })
            })
            .collect(),
        Primitive::Arc(a) => (0..n)
            .map(|i| {
                Primitive::Arc(Arc {
                    start_angle: a.start_angle + a.sweep * i as f64 / n as f64,
                    sweep: a.sweep / n as f64,
                    ..a
                })
            })
            .collect(),
    }
}

/// The parts of `next`'s edge farther than `OLD_TOL_MM` from the old edge, as stretches.
/// Where a front meets the part's edge, the stretch runs along the edge to the old front
/// at both ends, so nothing is skipped where it meets the edge at a shallow angle.
fn new_stretches(
    next: &Area,
    old: &PrimIndex,
    step: usize,
    piece: f64,
    material_right: bool,
) -> Vec<Stretch> {
    let mut found = Vec::new();
    for pline in std::iter::once(&next.outer).chain(&next.holes) {
        // Each piece with the index of the edge primitive it is part of.
        let (pieces, origin): (Vec<Primitive>, Vec<usize>) = oriented(pline, material_right)
            .iter()
            .enumerate()
            .flat_map(|(k, p)| split(p, piece).into_iter().map(move |q| (q, k)))
            .unzip();
        let new: Vec<bool> = pieces
            .iter()
            .map(|p| {
                [p.start(), p.midpoint(), p.end()]
                    .iter()
                    .any(|&q| old.distance_within(q, OLD_TOL_MM).is_none())
            })
            .collect();
        let n = pieces.len();
        if n == 0 || !new.iter().any(|&b| b) {
            continue;
        }
        if new.iter().all(|&b| b) {
            // Closed: whole, it is started where the tool is when it is cut.
            found.push(Stretch {
                step,
                prims: oriented(pline, material_right),
                closed: true,
            });
            continue;
        }
        // Start just after an old piece, then collect the runs of new ones.
        let Some(first_old) = new.iter().position(|&b| !b) else {
            continue;
        };
        let mut run: Vec<Primitive> = Vec::new();
        let mut run_origin: Option<usize> = None;
        for k in 1..=n {
            let i = (first_old + k) % n;
            if new[i] {
                // Pieces of the same edge primitive join up again.
                let whole = match run.last() {
                    Some(last) if run_origin == Some(origin[i]) => joined(last, &pieces[i]),
                    _ => None,
                };
                match (whole, run.last_mut()) {
                    (Some(whole), Some(last)) => *last = whole,
                    _ => run.push(pieces[i]),
                }
                run_origin = Some(origin[i]);
            } else {
                run_origin = None;
                if !run.is_empty() {
                    found.push(Stretch {
                        step,
                        prims: std::mem::take(&mut run),
                        closed: false,
                    });
                }
            }
        }
        if !run.is_empty() {
            found.push(Stretch {
                step,
                prims: run,
                closed: false,
            });
        }
    }
    found
}

/// Cuts the stretches into paths on the material grid.
struct Runner<'a> {
    /// The part being cleared and its edge: returns stay inside it.
    part: Option<&'a Area>,
    edge: PrimIndex,
    grid: &'a mut Grid,
    reach: f64,
    width: f64,
    arcs: &'a mut ArcTable,
    out: &'a mut Fronts,
    /// The path being built (stays down from pass to pass).
    current: Vec<Point>,
    /// The points of the pass cut last, and whether it was a closed front.
    last: Vec<Point>,
    last_closed: bool,
    /// Points the tool has been at in this part: the material round them is cut.
    visited: Vec<Point>,
    position: Point,
}

impl Runner<'_> {
    fn finish_path(&mut self) {
        if !self.current.is_empty() {
            self.out.paths.push(std::mem::take(&mut self.current));
        }
    }

    /// Grid step along paths, mm.
    fn step(&self) -> f64 {
        self.grid.cell_side()
    }

    /// Cells a step of the cut width should cut.
    fn target(&self) -> f64 {
        self.width / self.step()
    }

    /// Cut order: a stretch waits for the earlier ones it borders. Among those ready, a
    /// neighbour of the last one cut goes first (a branch is finished before the next),
    /// then the nearest.
    fn cut_in_order(&mut self, stretches: &[Stretch], width: f64) {
        let deps = dependencies(stretches, width);
        let mut done = vec![false; stretches.len()];
        let mut last: Option<usize> = None;
        for _ in 0..stretches.len() {
            let ready = |i: usize| !done[i] && deps[i].iter().all(|&d| done[d]);
            let here = self.position;
            let nearest = |candidates: &mut dyn Iterator<Item = usize>| {
                candidates.min_by(|&a, &b| {
                    distance(here, stretches[a].prims[0].start())
                        .total_cmp(&distance(here, stretches[b].prims[0].start()))
                })
            };
            let follow = last.and_then(|l| {
                nearest(&mut (0..stretches.len()).filter(|&i| ready(i) && deps[i].contains(&l)))
            });
            let Some(next) =
                follow.or_else(|| nearest(&mut (0..stretches.len()).filter(|&i| ready(i))))
            else {
                break;
            };
            done[next] = true;
            last = Some(next);
            self.cut(&stretches[next]);
        }
    }

    /// Cut one stretch: joined to the path so far if the return runs through cleared
    /// material, else as a new path. A stretch that cuts nothing is skipped.
    fn cut(&mut self, stretch: &Stretch) {
        let mut prims = stretch.prims.clone();
        if stretch.closed {
            rotate_to(&mut prims, self.position);
            if self.last_closed {
                // From one closed front out to the next: start a few cut widths ahead, so
                // the way across climbs gently instead of cutting a slot.
                advance(&mut prims, SPIRAL_AHEAD * self.width);
            }
        }
        let (points, circles) = flattened(&prims);
        let Some(&start) = points.first() else {
            return;
        };
        let (step, target) = (self.step(), self.target());
        // It cuts something if uncut material lies within reach of any point along it.
        let mut cuts = false;
        walk(&points, step, |p| {
            cuts = cuts || self.grid.count_disk(p, self.reach) > 0
        });
        if !cuts {
            return;
        }
        match self.route_to(start, stretch.closed) {
            Some(route) => {
                sweep(self.grid, &route, self.reach, step, target, None);
                self.current.extend(route.iter().skip(1));
            }
            None => {
                // A new path: it goes down where the material is already cut and moves
                // across to the start, if it can.
                self.finish_path();
                if let Some(entry) = self.entry_for(start, stretch.closed) {
                    sweep(self.grid, &[entry, start], self.reach, step, target, None);
                    self.current.push(entry);
                }
            }
        }
        // The opening round the plunge is an entry, not a sideways cut: not measured.
        let samples = (stretch.step > 0).then_some(&mut self.out.engagement);
        sweep(self.grid, &points, self.reach, step, target, samples);
        for (k, pair) in points.windows(2).enumerate() {
            if let Some(circle) = circles[k] {
                self.arcs.insert(pair[0], pair[1], circle);
            }
        }
        if self.current.last() == Some(&start) {
            self.current.extend(points.iter().skip(1));
        } else {
            self.current.extend(points.iter().copied());
        }
        if let Some(&end) = points.last() {
            self.position = end;
        }
        let spacing = (self.width / 2.0).max(self.step());
        for p in &points {
            if self
                .visited
                .last()
                .is_none_or(|&q| distance(*p, q) >= spacing)
            {
                self.visited.push(*p);
            }
        }
        self.last = points;
        // The opening ends on its circle: the next closed front spirals out from it.
        self.last_closed = stretch.closed || stretch.step == 0;
    }

    /// The way from the end of the path so far to `start` at depth, through cleared material:
    /// straight, or back along the pass just cut until a straight line is clear. From one
    /// closed front out to the next, the short step across counts as clear.
    fn route_to(&self, start: Point, closed: bool) -> Option<Vec<Point>> {
        let here = self.position;
        if self.current.is_empty() {
            return None;
        }
        let spiral = closed
            && self.last_closed
            && distance(here, start) <= (SPIRAL_AHEAD + 1.5) * self.width;
        if self.inside(here, start) && (spiral || self.cuts_nothing(here, start)) {
            return Some(vec![here, start]);
        }
        let mut back: Vec<Point> = vec![here];
        let mut since = 0.0;
        let spacing = (self.width / 2.0).max(self.step());
        for pair in self.last.windows(2).rev() {
            let p = pair[0];
            since += distance(p, back[back.len() - 1]);
            back.push(p);
            if since >= spacing {
                since = 0.0;
                if self.inside(p, start) && self.cuts_nothing(p, start) {
                    back.push(start);
                    return Some(back);
                }
            }
        }
        None
    }

    /// Where a new path to `start` can go down: a point the tool has been at (so the
    /// material round it is cut), nearest first, from which the way to `start` is clear.
    fn entry_for(&self, start: Point, closed: bool) -> Option<Point> {
        let mut near: Vec<Point> = self.visited.clone();
        near.sort_by(|a, b| distance(*a, start).total_cmp(&distance(*b, start)));
        near.into_iter().take(ENTRY_CANDIDATES).find(|&q| {
            let spiral = closed && distance(q, start) <= (SPIRAL_AHEAD + 1.5) * self.width;
            distance(q, start) >= CHAIN_TOL_MM
                && self.inside(q, start)
                && (spiral || self.cuts_nothing(q, start))
        })
    }

    /// Whether the straight move a–b stays in the part (its ends may lie on the edge).
    fn inside(&self, a: Point, b: Point) -> bool {
        let Some(part) = self.part else {
            return false;
        };
        let length = distance(a, b);
        if length < CHAIN_TOL_MM {
            return true;
        }
        if length <= 4.0 * OLD_TOL_MM {
            return false;
        }
        let t = OLD_TOL_MM / length;
        let along = |t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        part.contains(along(0.5))
            && self
                .edge
                .segment_distance_within(along(2.0 * t), along(1.0 - 2.0 * t), OLD_TOL_MM)
                .is_none()
    }

    /// Whether the straight move a–b cuts (almost) nothing on the grid.
    fn cuts_nothing(&self, a: Point, b: Point) -> bool {
        let limit = RETURN_CUT_SHARE * self.target();
        let mut ok = true;
        walk(&[a, b], self.step(), |p| {
            if f64::from(self.grid.count_disk(p, self.reach)) > limit {
                ok = false;
            }
        });
        ok
    }
}

/// Cut along the points on `grid` with the model's `reach`, every `step` mm, returning the
/// cells cut; the share of `target` cut by every step goes to `engagement`.
fn sweep(
    grid: &mut Grid,
    points: &[Point],
    reach: f64,
    step: f64,
    target: f64,
    mut engagement: Option<&mut Vec<f64>>,
) -> u64 {
    let mut total = 0u64;
    walk(points, step, |p| {
        let cut = grid.count_disk(p, reach);
        grid.clear_disk(p, reach);
        total += u64::from(cut);
        if let Some(samples) = engagement.as_deref_mut() {
            samples.push(f64::from(cut) / target);
        }
    });
    total
}

/// Visit points along the polyline `step` apart, measured along it, from its start; the
/// end is visited too.
fn walk(points: &[Point], step: f64, mut visit: impl FnMut(Point)) {
    let Some(&first) = points.first() else {
        return;
    };
    visit(first);
    // How far along the current chord the next point is.
    let mut next = step;
    let mut last = first;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = distance(a, b);
        while next <= length {
            let t = next / length;
            last = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            visit(last);
            next += step;
        }
        next -= length;
    }
    if let Some(&end) = points.last()
        && distance(end, last) > CHAIN_TOL_MM
    {
        visit(end);
    }
}

/// The stretch's points (arcs split into chords) and, for each chord, its circle.
fn flattened(prims: &[Primitive]) -> (Vec<Point>, Vec<Option<Circle>>) {
    let mut points: Vec<Point> = Vec::new();
    let mut circles: Vec<Option<Circle>> = Vec::new();
    for prim in prims {
        let circle = match prim {
            Primitive::Arc(a) => Some(Circle {
                center: a.center,
                radius: a.radius,
            }),
            Primitive::Line(_) => None,
        };
        for p in prim.points() {
            match points.last() {
                Some(&q) if distance(p, q) < CHAIN_TOL_MM => {}
                Some(_) => {
                    points.push(p);
                    circles.push(circle);
                }
                None => points.push(p),
            }
        }
    }
    (points, circles)
}

/// For every stretch, the earlier ones within about a cut width of it: their material is
/// what it cuts against.
fn dependencies(stretches: &[Stretch], width: f64) -> Vec<Vec<usize>> {
    // Points along each stretch this far apart; the reach allows for half of it.
    let sample = (width / 2.0).clamp(CHAIN_TOL_MM, 1.0);
    let reach = 1.25 * width + sample;
    // Indexed in short pieces: a long arc would sit in every cell of its box.
    let cell = width.max(1.0);
    let mut prims = Vec::new();
    let mut owner = Vec::new();
    for (i, s) in stretches.iter().enumerate() {
        for p in s.prims.iter().flat_map(|p| split(p, cell)) {
            prims.push(p);
            owner.push(i);
        }
    }
    let index = PrimIndex::new(prims, cell);
    stretches
        .iter()
        .map(|s| {
            let mut deps: Vec<usize> = Vec::new();
            let samples = s.prims.iter().flat_map(|p| {
                split(p, sample)
                    .into_iter()
                    .map(|q| q.end())
                    .chain(std::iter::once(p.start()))
            });
            for q in samples {
                for k in index.near(q, q, reach) {
                    let j = owner[k];
                    if stretches[j].step < s.step
                        && !deps.contains(&j)
                        && point_distance(&index.prims[k], q) <= reach
                    {
                        deps.push(j);
                    }
                }
            }
            deps
        })
        .collect()
}
