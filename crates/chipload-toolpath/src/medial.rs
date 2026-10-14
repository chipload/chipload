// SPDX-License-Identifier: GPL-3.0-or-later
//! The medial axis (centre line) of an area, shared by V-carving (docs/specs/v-carve.md) and
//! adaptive clearing (docs/specs/adaptive-spiral.md).
//!
//! It comes from a segment Voronoi diagram of the outline split into chords (boostvoronoi,
//! ADR 0015). The diagram only says where the centre line runs: how far each point is from
//! the outline is the exact distance to the real outline (arcs, not chords).

use boostvoronoi::prelude::{Builder, Diagram, SourceCategory, VertexIndex};
use chipload_core::Point;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, VORONOI_GRID_MM};

use crate::ToolpathError;
use crate::cavalier::{Area, geometry, guarded};
use crate::geom::PrimIndex;
use crate::offset::{pline_segments, point_distance};

/// Deepest recursion when splitting a curve; 2^20 pieces is far beyond need.
const MAX_SPLITS: u32 = 20;

/// The real outline of one region (arcs and lines), for exact distances.
pub(crate) struct Outline {
    pub(crate) index: PrimIndex,
    /// Farther than any point of the region can be from its outline.
    pub(crate) reach: f64,
}

impl Outline {
    pub(crate) fn new(area: &Area) -> Self {
        let prims = area.boundary();
        let (lo, hi) = prims.iter().flat_map(|p| p.points()).fold(
            (
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |(lo, hi), p| {
                (
                    (lo.0.min(p.0), lo.1.min(p.1)),
                    (hi.0.max(p.0), hi.1.max(p.1)),
                )
            },
        );
        let reach = distance(lo, hi).max(CHAIN_TOL_MM);
        // Cells about a fiftieth of the region: a handful of chords each.
        let cell = (reach / 50.0).max(10.0 * ARC_APPROX_TOL_MM);
        Outline {
            index: PrimIndex::new(prims, cell),
            reach,
        }
    }

    /// Distance from `p` to the nearest part of the outline.
    pub(crate) fn distance(&self, p: Point) -> f64 {
        let mut limit = self.reach / 64.0;
        while limit < self.reach {
            if let Some(d) = self.index.distance_within(p, limit) {
                return d;
            }
            limit *= 2.0;
        }
        self.index
            .prims
            .iter()
            .map(|prim| point_distance(prim, p))
            .fold(f64::INFINITY, f64::min)
    }
}

/// The medial axis as a graph: nodes are Voronoi vertices, edges the bisectors between
/// them (as polylines), each point with its cut radius once [`Graph::set_cut`] ran.
#[derive(Debug, Default)]
pub(crate) struct Graph {
    pub(crate) nodes: Vec<Point>,
    pub(crate) edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
pub(crate) struct Edge {
    pub(crate) ends: [usize; 2],
    /// From the first end to the second, both included.
    pub(crate) points: Vec<Point>,
    /// Cut radius at each point.
    pub(crate) radii: Vec<f64>,
    pub(crate) alive: bool,
}

impl Edge {
    pub(crate) fn other(&self, node: usize) -> usize {
        if self.ends[0] == node {
            self.ends[1]
        } else {
            self.ends[0]
        }
    }

    /// Points and radii walked from `node`.
    pub(crate) fn from(&self, node: usize) -> Vec<(Point, f64)> {
        let mut walked: Vec<(Point, f64)> = self
            .points
            .iter()
            .copied()
            .zip(self.radii.iter().copied())
            .collect();
        if self.ends[0] != node {
            walked.reverse();
        }
        walked
    }
}

impl Graph {
    pub(crate) fn degrees(&self) -> Vec<usize> {
        let mut degree = vec![0; self.nodes.len()];
        for edge in self.edges.iter().filter(|e| e.alive) {
            degree[edge.ends[0]] += 1;
            degree[edge.ends[1]] += 1;
        }
        degree
    }

    pub(crate) fn alive_at(&self, node: usize) -> Vec<usize> {
        (0..self.edges.len())
            .filter(|&e| self.edges[e].alive && self.edges[e].ends.contains(&node))
            .collect()
    }

    /// Remove end branches that add nothing within `tol`: those every disc of
    /// which lies inside the disc of the junction they hang from, grown by the tolerance
    /// (spec §4.3). A chord-split arc gets one such branch at every chord corner. A node
    /// keeps how much of the tolerance it already relies on, so removals never add up to
    /// more than the tolerance.
    pub(crate) fn prune(&mut self, outline: &Outline, tol: f64) {
        let radius: Vec<f64> = self.nodes.iter().map(|&p| outline.distance(p)).collect();
        let mut slack = vec![0.0_f64; self.nodes.len()];
        let mut degree = self.degrees();
        let mut changed = true;
        while changed {
            changed = false;
            for leaf in 0..self.nodes.len() {
                if degree[leaf] != 1 {
                    continue;
                }
                let Some((chain, junction)) = self.chain_from(leaf, &degree) else {
                    continue;
                };
                let centre = self.nodes[junction];
                let mut needed = 0.0_f64;
                for &e in &chain {
                    let edge = &self.edges[e];
                    for (k, &p) in edge.points.iter().enumerate() {
                        let own = if k == 0 || k + 1 == edge.points.len() {
                            let node = if k == 0 { edge.ends[0] } else { edge.ends[1] };
                            if node == junction {
                                continue;
                            }
                            slack[node]
                        } else {
                            0.0
                        };
                        let reach = distance(p, centre) + outline.distance(p) + own;
                        needed = needed.max(reach - radius[junction]);
                    }
                }
                if needed <= tol {
                    for &e in &chain {
                        self.edges[e].alive = false;
                        let [a, b] = self.edges[e].ends;
                        degree[a] -= 1;
                        degree[b] -= 1;
                    }
                    slack[junction] = slack[junction].max(needed);
                    changed = true;
                }
            }
        }
    }

    /// The edges from a leaf to the first node with three or more edges, and that node. None
    /// when the leaf's whole branch is a chain (nothing to hang from).
    pub(crate) fn chain_from(&self, leaf: usize, degree: &[usize]) -> Option<(Vec<usize>, usize)> {
        let mut chain = Vec::new();
        let mut node = leaf;
        let mut came: Option<usize> = None;
        loop {
            let next = self.alive_at(node).into_iter().find(|&e| Some(e) != came)?;
            chain.push(next);
            node = self.edges[next].other(node);
            came = Some(next);
            match degree[node] {
                2 if node != leaf => continue,
                d if d >= 3 => return Some((chain, node)),
                _ => return None,
            }
        }
    }

    /// Give every point its cut radius: the distance to the outline, at most `cap`. Edges
    /// are split until the radius between two points is linear within half of `tol`.
    pub(crate) fn set_cut(&mut self, outline: &Outline, cap: f64, tol: f64) {
        let cut = |p: Point| outline.distance(p).min(cap);
        for edge in self.edges.iter_mut().filter(|e| e.alive) {
            let mut points = Vec::new();
            let mut radii = Vec::new();
            for w in edge.points.windows(2) {
                let (a, b) = (w[0], w[1]);
                if points.is_empty() {
                    points.push(a);
                    radii.push(cut(a));
                }
                let mut stretch = Vec::new();
                split_linear(a, cut(a), b, cut(b), &cut, tol, MAX_SPLITS, &mut stretch);
                for (p, r) in stretch {
                    points.push(p);
                    radii.push(r);
                }
            }
            if points.is_empty() {
                // A single point (a degenerate edge): nothing to cut along.
                edge.alive = false;
                continue;
            }
            edge.points = points;
            edge.radii = radii;
        }
    }
}

/// Points after `a` up to and including `b`, split until the radius at every midpoint is
/// within half of `tol` of linear between its ends.
#[allow(clippy::too_many_arguments)]
fn split_linear(
    a: Point,
    ra: f64,
    b: Point,
    rb: f64,
    cut: &impl Fn(Point) -> f64,
    tol: f64,
    depth: u32,
    out: &mut Vec<(Point, f64)>,
) {
    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let rm = cut(m);
    if depth > 0 && ((ra + rb) / 2.0 - rm).abs() > tol / 2.0 {
        split_linear(a, ra, m, rm, cut, tol, depth - 1, out);
        split_linear(m, rm, b, rb, cut, tol, depth - 1, out);
    } else {
        out.push((b, rb));
    }
}

/// What a Voronoi cell belongs to: a corner point or a whole chord.
#[derive(Debug, Clone, Copy)]
enum Site {
    Point(Point),
    Segment(Point, Point),
}

/// The area's outline as integer chords for the Voronoi diagram (arcs split within a quarter
/// of `tol`), and the diagram's edges inside the area (without those between a chord and its
/// own end points). Errors name the contours `ids` (e.g. "#1, #2") and what the centre line
/// is for, `name` (e.g. "V-carve").
pub(crate) fn medial_axis(
    area: &Area,
    outline: &Outline,
    ids: &str,
    name: &str,
    tol: f64,
) -> Result<Graph, ToolpathError> {
    let mut chords: Vec<[i32; 4]> = Vec::new();
    let mut ends: Vec<(Point, Point)> = Vec::new();
    for pline in std::iter::once(&area.outer).chain(&area.holes) {
        let ring = snapped_ring(&fine_ring(pline, tol), name)?;
        let n = ring.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            chords.push([a.0, a.1, b.0, b.1]);
            ends.push((unsnap(a), unsnap(b)));
        }
    }
    if chords.is_empty() {
        return Ok(Graph::default());
    }
    let built = guarded(&format!("The {name} centre line"), || {
        Builder::<i32>::default()
            .with_segments(chords.iter())
            .and_then(Builder::build)
    })?;
    let failed = |e: boostvoronoi::prelude::BvError| {
        geometry(format!(
            "contours {ids}: the {name} centre line could not be built ({e}); does the outline \
             touch or cross itself?"
        ))
    };
    let diagram = built.map_err(failed)?;
    let mut graph = Graph::default();
    let mut node_of: std::collections::BTreeMap<usize, usize> = Default::default();
    let mut node = |graph: &mut Graph, v: VertexIndex, p: Point| -> usize {
        *node_of.entry(v.usize()).or_insert_with(|| {
            graph.nodes.push(p);
            graph.nodes.len() - 1
        })
    };
    for edge in diagram.edges() {
        let id = edge.id();
        let twin = edge.twin().map_err(failed)?;
        if twin.usize() < id.usize() || !edge.is_primary() {
            continue;
        }
        let (Some(v0), Some(v1)) = (
            edge.vertex0(),
            diagram.edge_get_vertex1(id).map_err(failed)?,
        ) else {
            continue; // infinite: outside the outer loop
        };
        let p0 = vertex_point(&diagram, v0).map_err(failed)?;
        let p1 = vertex_point(&diagram, v1).map_err(failed)?;
        let site_a = site(&diagram, edge.cell().map_err(failed)?, &ends, name)?;
        let twin_cell = diagram.edge(twin).map_err(failed)?.cell().map_err(failed)?;
        let site_b = site(&diagram, twin_cell, &ends, name)?;
        let points = bisector(p0, p1, site_a, site_b);
        let middle = if points.len() > 2 {
            points[points.len() / 2]
        } else {
            ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0)
        };
        if !area.contains(middle) || outline.distance(middle) <= 0.0 {
            continue;
        }
        let a = node(&mut graph, v0, p0);
        let b = node(&mut graph, v1, p1);
        graph.edges.push(Edge {
            ends: [a, b],
            points,
            radii: Vec::new(),
            alive: true,
        });
    }
    Ok(graph)
}

fn vertex_point(
    diagram: &Diagram,
    v: VertexIndex,
) -> Result<Point, boostvoronoi::prelude::BvError> {
    let vertex = diagram.vertex(v)?;
    Ok((vertex.x() * VORONOI_GRID_MM, vertex.y() * VORONOI_GRID_MM))
}

fn site(
    diagram: &Diagram,
    cell: boostvoronoi::prelude::CellIndex,
    ends: &[(Point, Point)],
    name: &str,
) -> Result<Site, ToolpathError> {
    let unknown = || geometry(format!("{name} centre line: a cell without its chord"));
    let cell = diagram.cell(cell).map_err(|_| unknown())?;
    let (index, category) = cell.source_index_2();
    let &(a, b) = ends.get(index.usize()).ok_or_else(unknown)?;
    Ok(match category {
        SourceCategory::SegmentStart => Site::Point(a),
        SourceCategory::SegmentEnd => Site::Point(b),
        SourceCategory::Segment => Site::Segment(a, b),
        SourceCategory::SinglePoint => return Err(unknown()),
    })
}

/// The bisector between two sites from `p0` to `p1`: straight between two chords or two
/// points, a parabola between a point and a chord (split within `ARC_APPROX_TOL_MM`).
fn bisector(p0: Point, p1: Point, a: Site, b: Site) -> Vec<Point> {
    let (focus, (s, e)) = match (a, b) {
        (Site::Point(f), Site::Segment(s, e)) | (Site::Segment(s, e), Site::Point(f)) => {
            (f, (s, e))
        }
        _ => return vec![p0, p1],
    };
    let length = distance(s, e);
    if length <= 0.0 {
        return vec![p0, p1];
    }
    let u = ((e.0 - s.0) / length, (e.1 - s.1) / length);
    let mut n = (-u.1, u.0);
    let mut lift = (focus.0 - s.0) * n.0 + (focus.1 - s.1) * n.1;
    if lift < 0.0 {
        n = (-n.0, -n.1);
        lift = -lift;
    }
    if lift <= CHAIN_TOL_MM {
        return vec![p0, p1];
    }
    // The point above the chord's line at foot parameter t, as far from it as from the focus.
    let at = |t: f64| {
        let foot = (s.0 + u.0 * t, s.1 + u.1 * t);
        let h = ((foot.0 - focus.0).powi(2) + (foot.1 - focus.1).powi(2)) / (2.0 * lift);
        (foot.0 + n.0 * h, foot.1 + n.1 * h)
    };
    let along = |p: Point| (p.0 - s.0) * u.0 + (p.1 - s.1) * u.1;
    let (t0, t1) = (along(p0), along(p1));
    let mut points = vec![p0];
    split_parabola(&at, t0, p0, t1, p1, MAX_SPLITS, &mut points);
    points
}

/// Points after `pa` up to `pb` (which is pushed as given), splitting until the curve is
/// within `ARC_APPROX_TOL_MM` of its chords.
fn split_parabola(
    at: &impl Fn(f64) -> Point,
    ta: f64,
    pa: Point,
    tb: f64,
    pb: Point,
    depth: u32,
    out: &mut Vec<Point>,
) {
    let tm = (ta + tb) / 2.0;
    let pm = at(tm);
    let chord_mid = ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0);
    if depth > 0 && distance(pm, chord_mid) > ARC_APPROX_TOL_MM {
        split_parabola(at, ta, pa, tm, pm, depth - 1, out);
        split_parabola(at, tm, pm, tb, pb, depth - 1, out);
    } else {
        out.push(pb);
    }
}

/// The loop's points, arcs split into chords within a quarter of `tol`: the centre line
/// between chords wobbles by up to their sag, and end branches into chord corners are only
/// dropped when they add less than the tolerance (v-carve.md §4.3).
fn fine_ring(pline: &cavalier_contours::polyline::Polyline<f64>, tol: f64) -> Vec<Point> {
    let sag = tol / 4.0;
    let mut ring = Vec::new();
    for prim in pline_segments(pline) {
        match prim {
            Primitive::Line(line) => ring.push(line.start),
            Primitive::Arc(arc) => {
                let step = if arc.radius > sag {
                    2.0 * (1.0 - sag / arc.radius).acos()
                } else {
                    std::f64::consts::PI
                };
                // A few hundred pieces at most for any arc a drawing has.
                let pieces = (arc.sweep.abs() / step).ceil().clamp(1.0, 1.0e6) as usize;
                for k in 0..pieces {
                    let a = arc.start_angle + arc.sweep * k as f64 / pieces as f64;
                    ring.push((
                        arc.center.0 + arc.radius * a.cos(),
                        arc.center.1 + arc.radius * a.sin(),
                    ));
                }
            }
        }
    }
    ring
}

/// A closed ring on the Voronoi grid, without repeated points or spikes that double back.
fn snapped_ring(points: &[Point], name: &str) -> Result<Vec<(i32, i32)>, ToolpathError> {
    let snap = |v: f64| -> Result<i32, ToolpathError> {
        let units = (v / VORONOI_GRID_MM).round();
        if units.is_finite() && units.abs() < f64::from(i32::MAX) {
            // In range, checked just above.
            Ok(units as i32)
        } else {
            Err(geometry(format!(
                "a {name} outline at {v:.3} mm is too far from the drawing origin (the \
                 centre line works within ±{:.0} mm)",
                f64::from(i32::MAX) * VORONOI_GRID_MM
            )))
        }
    };
    let mut ring: Vec<(i32, i32)> = Vec::with_capacity(points.len());
    for p in points {
        let q = (snap(p.0)?, snap(p.1)?);
        if ring.last() != Some(&q) {
            ring.push(q);
        }
    }
    while ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    // Drop spikes a -> b -> a, which would overlap themselves.
    let mut changed = true;
    while changed && ring.len() >= 3 {
        changed = false;
        let n = ring.len();
        for i in 0..n {
            if ring[(i + n - 1) % n] == ring[(i + 1) % n] {
                ring.remove(i);
                let n = ring.len();
                if n > 0 {
                    ring.remove(i % n);
                }
                changed = true;
                break;
            }
        }
    }
    Ok(ring)
}

fn unsnap(q: (i32, i32)) -> Point {
    (
        f64::from(q.0) * VORONOI_GRID_MM,
        f64::from(q.1) * VORONOI_GRID_MM,
    )
}
