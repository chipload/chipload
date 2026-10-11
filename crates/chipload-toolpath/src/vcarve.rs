// SPDX-License-Identifier: GPL-3.0-or-later
//! V-carving: a V-bit follows the centre line (medial axis) of closed shapes, as deep as
//! makes its cut exactly as wide as the shape. See docs/specs/v-carve.md.
//!
//! The medial axis comes from a segment Voronoi diagram of the outline split into chords
//! (boostvoronoi, ADR 0015). It only says where the tool goes: how wide it cuts at every point
//! is the exact distance to the real outline (arcs, not chords), and every move is verified
//! against that distance afterwards.

use boostvoronoi::prelude::{Builder, Diagram, SourceCategory, VertexIndex};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{Primitive, distance};
use chipload_core::tolerances::{
    ARC_APPROX_TOL_MM, CHAIN_TOL_MM, DEPTH_TOL_MM, VCARVE_TOL_MM, VORONOI_GRID_MM,
};

use crate::ToolpathError;
use crate::geom::PrimIndex;
use crate::model::Move;
use crate::offset::{pline_segments, point_distance};
use crate::pocket::{Area, PocketArea, build_region, geometry, guarded};

/// Deepest recursion when splitting a curve or a move; 2^20 pieces is far beyond need.
const MAX_SPLITS: u32 = 20;

/// A V-bit's cutting geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VBit {
    /// Included angle, radians, 0 < angle < π.
    pub angle: f64,
    /// Flat tip, mm (0: pointed).
    pub tip_diameter: f64,
    /// Largest cutting diameter, mm.
    pub diameter: f64,
}

impl VBit {
    /// How much wider the cut gets at the surface per mm of depth, on each side.
    fn slope(&self) -> f64 {
        (self.angle / 2.0).tan()
    }

    /// Radius of the cut at the surface with the tip `depth` below it.
    pub fn radius_at(&self, depth: f64) -> f64 {
        self.tip_diameter / 2.0 + depth * self.slope()
    }

    /// Tip depth for a cut of `radius` at the surface; negative when the tip is wider.
    pub fn depth_for(&self, radius: f64) -> f64 {
        (radius - self.tip_diameter / 2.0) / self.slope()
    }

    /// The deepest it cuts with its cone: beyond, the shank would rub.
    pub fn deepest(&self) -> f64 {
        self.depth_for(self.diameter / 2.0)
    }
}

/// One continuous cut: points and the tip depth below the carved surface at each (>= 0).
#[derive(Debug, Clone, PartialEq)]
pub struct CarvePath {
    pub points: Vec<Point>,
    pub depths: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VcarvePlan {
    /// In cut order.
    pub paths: Vec<CarvePath>,
    /// Warnings: strokes wider than the depth allows, strokes narrower than the tip.
    pub notes: Vec<String>,
    /// The deepest any path goes, mm below the carved surface.
    pub deepest: f64,
}

/// Paths for the regions the selected closed contours make (as pockets do: outer minus the
/// contours inside it). `max_depth` is the cap below the carved surface; `start` is where the
/// tool is, for the cut order.
pub fn plan_vcarve(
    contours: &[&Contour],
    bit: &VBit,
    max_depth: f64,
    start: Point,
) -> Result<VcarvePlan, ToolpathError> {
    if !(bit.angle > 0.0 && bit.angle < std::f64::consts::PI) {
        return Err(geometry(format!(
            "the V-bit angle must be more than 0 and less than 180 degrees, got {:.3}",
            bit.angle.to_degrees()
        )));
    }
    let cap = max_depth.min(bit.deepest());
    let cap_radius = bit.radius_at(cap);
    let mut graph = Graph::default();
    let mut notes = Vec::new();
    for region in build_region(contours)? {
        let ids = ids_text(&region.contour_ids);
        let outline = Outline::new(&region.area);
        let mut axis = medial_axis(&region, &outline)?;
        axis.prune(&outline);
        let widest = axis.widest(&outline);
        axis.set_cut(&outline, cap_radius);
        axis.verify(&region.area, &outline, &ids)?;
        if widest > cap_radius + VCARVE_TOL_MM {
            let limit = if bit.deepest() < max_depth {
                format!(
                    "the V-bit ({:.3} mm across), which carves at most {cap:.3} mm deep",
                    bit.diameter
                )
            } else {
                format!("a carve at most {cap:.3} mm deep")
            };
            notes.push(format!(
                "contours {ids}: too wide for {limit} ({:.3} mm wide at the surface); the \
                 edges of the widest parts aren't reached (they need {:.3} mm). Allow deeper, \
                 use a wider angle, or clear the floor",
                2.0 * cap_radius,
                bit.depth_for(widest)
            ));
        }
        let narrow = axis.narrow_length(bit.tip_diameter / 2.0);
        if narrow > VCARVE_TOL_MM {
            notes.push(format!(
                "contours {ids}: {narrow:.3} mm of centre line is narrower than the tip \
                 ({:.3} mm) and isn't carved",
                bit.tip_diameter
            ));
        }
        graph.append(axis);
    }
    let mut paths = Vec::new();
    for walked in graph.walk(start) {
        paths.extend(carve_pieces(&walked, bit));
    }
    let deepest = paths
        .iter()
        .flat_map(|p| p.depths.iter().copied())
        .fold(0.0, f64::max);
    Ok(VcarvePlan {
        paths,
        notes,
        deepest,
    })
}

/// Moves for the paths: pass by pass, each no deeper than its level (depths below the
/// carved surface, ascending; the last is the deepest). A later pass only runs where the
/// previous one stopped short. Starts and ends retracted to `safe_z`.
pub fn vcarve_moves(
    paths: &[CarvePath],
    levels: &[f64],
    surface_z: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
) -> Vec<Move> {
    let mut moves = vec![Move::rapid_z(safe_z)];
    let mut previous: Option<f64> = None;
    for &level in levels {
        for path in paths {
            let pieces = match previous {
                None => vec![path.clone()],
                Some(above) => deeper_than(path, above),
            };
            for piece in pieces {
                let Some(&first) = piece.points.first() else {
                    continue;
                };
                let z_of = |d: f64| surface_z - d.min(level);
                moves.push(Move::rapid_xy(first));
                let mut z = z_of(piece.depths[0]);
                moves.push(Move::feed_z(z, plunge_feed));
                let mut at = first;
                for (&p, &d) in piece.points.iter().zip(&piece.depths).skip(1) {
                    let next = z_of(d);
                    // Steeper than 45° down: the plunge feed (decision 10).
                    let rate = if z - next > distance(at, p) {
                        plunge_feed
                    } else {
                        feed
                    };
                    moves.push(Move::feed_xyz(p, next, rate));
                    (at, z) = (p, next);
                }
                moves.push(Move::rapid_z(safe_z));
            }
        }
        previous = Some(level);
    }
    moves
}

/// Pass depths below the carved surface: one pass at `deepest`, or equal passes no deeper
/// than `step_down`, the last exactly `deepest`.
pub fn carve_levels(deepest: f64, step_down: Option<f64>) -> Result<Vec<f64>, ToolpathError> {
    match step_down {
        Some(step) if deepest > DEPTH_TOL_MM => crate::profile::pass_depths(0.0, deepest, step),
        _ => Ok(vec![deepest]),
    }
}

/// The parts of `path` deeper than `above`, each starting and ending at `above`
/// (interpolated) where it crosses that level.
fn deeper_than(path: &CarvePath, above: f64) -> Vec<CarvePath> {
    split_where(path, |d| d > above + DEPTH_TOL_MM, above)
}

/// Pieces of `path` where `keep(depth)`, cut at `level` where the depth crosses it.
fn split_where(path: &CarvePath, keep: impl Fn(f64) -> bool, level: f64) -> Vec<CarvePath> {
    let mut pieces = Vec::new();
    let mut current = CarvePath {
        points: Vec::new(),
        depths: Vec::new(),
    };
    let n = path.points.len();
    for i in 0..n {
        let (p, d) = (path.points[i], path.depths[i]);
        if i > 0 {
            let (q, e) = (path.points[i - 1], path.depths[i - 1]);
            if keep(e) != keep(d) && (d - e).abs() > 0.0 {
                let t = (level - e) / (d - e);
                let at = (q.0 + (p.0 - q.0) * t, q.1 + (p.1 - q.1) * t);
                current.points.push(at);
                current.depths.push(level);
                if !keep(d) {
                    pieces.push(std::mem::replace(
                        &mut current,
                        CarvePath {
                            points: Vec::new(),
                            depths: Vec::new(),
                        },
                    ));
                }
            }
        }
        if keep(d) {
            current.points.push(p);
            current.depths.push(d);
        }
    }
    pieces.push(current);
    pieces.retain(|piece| piece.points.len() >= 2);
    pieces
}

/// A walked path (points and cut radius) as cuts: the tip depth for each radius, without the
/// parts narrower than the tip.
fn carve_pieces(walked: &[(Point, f64)], bit: &VBit) -> Vec<CarvePath> {
    let path = CarvePath {
        points: walked.iter().map(|w| w.0).collect(),
        depths: walked.iter().map(|w| bit.depth_for(w.1)).collect(),
    };
    split_where(&path, |d| d >= -DEPTH_TOL_MM, 0.0)
        .into_iter()
        .map(|mut piece| {
            for d in &mut piece.depths {
                *d = d.max(0.0);
            }
            piece
        })
        .filter(|piece| piece.depths.iter().any(|&d| d > DEPTH_TOL_MM))
        .collect()
}

fn ids_text(ids: &[usize]) -> String {
    ids.iter()
        .map(|i| format!("#{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The real outline of one region (arcs and lines), for exact distances.
struct Outline {
    index: PrimIndex,
    /// Farther than any point of the region can be from its outline.
    reach: f64,
}

impl Outline {
    fn new(area: &Area) -> Self {
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
    fn distance(&self, p: Point) -> f64 {
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
struct Graph {
    nodes: Vec<Point>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
struct Edge {
    ends: [usize; 2],
    /// From the first end to the second, both included.
    points: Vec<Point>,
    /// Cut radius at each point.
    radii: Vec<f64>,
    alive: bool,
}

impl Edge {
    fn other(&self, node: usize) -> usize {
        if self.ends[0] == node {
            self.ends[1]
        } else {
            self.ends[0]
        }
    }

    /// Points and radii walked from `node`.
    fn from(&self, node: usize) -> Vec<(Point, f64)> {
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
    fn degrees(&self) -> Vec<usize> {
        let mut degree = vec![0; self.nodes.len()];
        for edge in self.edges.iter().filter(|e| e.alive) {
            degree[edge.ends[0]] += 1;
            degree[edge.ends[1]] += 1;
        }
        degree
    }

    fn alive_at(&self, node: usize) -> Vec<usize> {
        (0..self.edges.len())
            .filter(|&e| self.edges[e].alive && self.edges[e].ends.contains(&node))
            .collect()
    }

    /// Remove end branches that add nothing within `VCARVE_TOL_MM`: those every disc of
    /// which lies inside the disc of the junction they hang from, grown by the tolerance
    /// (spec §4.3). A chord-split arc gets one such branch at every chord corner. A node
    /// keeps how much of the tolerance it already relies on, so removals never add up to
    /// more than the tolerance.
    fn prune(&mut self, outline: &Outline) {
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
                if needed <= VCARVE_TOL_MM {
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
    fn chain_from(&self, leaf: usize, degree: &[usize]) -> Option<(Vec<usize>, usize)> {
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

    /// The largest distance to the outline along the axis: how wide the widest part is.
    fn widest(&self, outline: &Outline) -> f64 {
        self.edges
            .iter()
            .filter(|e| e.alive)
            .flat_map(|e| e.points.iter().map(|&p| outline.distance(p)))
            .fold(0.0, f64::max)
    }

    /// Give every point its cut radius: the distance to the outline, at most `cap`. Edges
    /// are split until the radius between two points is linear within half the tolerance.
    fn set_cut(&mut self, outline: &Outline, cap: f64) {
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
                split_linear(a, cut(a), b, cut(b), &cut, MAX_SPLITS, &mut stretch);
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

    /// No gouge (spec §1, rule 1): along every edge, the cut's circle at the surface stays
    /// within the outline, checked at points between the stored ones against the exact
    /// distance; and every point is inside the region.
    fn verify(&self, area: &Area, outline: &Outline, ids: &str) -> Result<(), ToolpathError> {
        for edge in self.edges.iter().filter(|e| e.alive) {
            for k in 0..edge.points.len().saturating_sub(1) {
                let (a, b) = (edge.points[k], edge.points[k + 1]);
                let (ra, rb) = (edge.radii[k], edge.radii[k + 1]);
                for step in 0..=4 {
                    let t = f64::from(step) / 4.0;
                    let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                    let r = ra + (rb - ra) * t;
                    let room = outline.distance(p);
                    let inside = area.contains(p) || room <= VCARVE_TOL_MM;
                    if !inside || r > room + VCARVE_TOL_MM {
                        return Err(geometry(format!(
                            "contours {ids}: V-carve verification failed: the cut would \
                             cross the outline near ({:.3}, {:.3})",
                            p.0, p.1
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Length of axis where the stroke is narrower than the tip, not counting the tips of
    /// corners (from the corner, where the width is 0, up to where the tip fits).
    fn narrow_length(&self, tip_radius: f64) -> f64 {
        if tip_radius <= 0.0 {
            return 0.0;
        }
        let mut total = 0.0;
        for edge in self.edges.iter().filter(|e| e.alive) {
            let corner_at = |k: usize| edge.radii[k] <= CHAIN_TOL_MM;
            let mut run = 0.0;
            let mut from_corner = false;
            for k in 0..edge.points.len() {
                let narrow = edge.radii[k] < tip_radius - CHAIN_TOL_MM;
                if narrow && run == 0.0 {
                    from_corner = corner_at(k);
                }
                if k > 0 && narrow && edge.radii[k - 1] < tip_radius - CHAIN_TOL_MM {
                    run += distance(edge.points[k - 1], edge.points[k]);
                }
                let ends = k + 1 == edge.points.len();
                if !narrow || ends {
                    let to_corner = narrow && corner_at(k);
                    if !(from_corner || to_corner) {
                        total += run;
                    }
                    run = 0.0;
                    from_corner = false;
                }
            }
        }
        total
    }

    /// Add another region's graph (its nodes are never shared).
    fn append(&mut self, other: Graph) {
        let offset = self.nodes.len();
        self.nodes.extend(other.nodes);
        self.edges
            .extend(other.edges.into_iter().filter(|e| e.alive).map(|mut e| {
                e.ends = [e.ends[0] + offset, e.ends[1] + offset];
                e
            }));
    }

    /// Every alive edge once, in continuous paths found by a greedy walk: each starts at the
    /// nearest end of the axis with an edge left (a corner, at depth 0), else the nearest
    /// node with an odd number of edges left, else any; ties by (y, x). The walk goes on
    /// where it can, so dead ends are left for later paths to start from.
    fn walk(&self, start: Point) -> Vec<Vec<(Point, f64)>> {
        let mut used: Vec<bool> = self.edges.iter().map(|e| !e.alive).collect();
        let mut left: Vec<usize> = self.degrees();
        // Ends of the axis: corners, where the cut starts at depth 0.
        let end: Vec<bool> = left.iter().map(|&d| d == 1).collect();
        let mut at = start;
        let mut paths = Vec::new();
        while left.iter().any(|&d| d > 0) {
            let key = |n: usize| {
                let p = self.nodes[n];
                (
                    !end[n],
                    left[n].is_multiple_of(2),
                    distance(at, p),
                    p.1,
                    p.0,
                )
            };
            let Some(mut node) = (0..self.nodes.len())
                .filter(|&n| left[n] > 0)
                .min_by(|&a, &b| {
                    let (ka, kb) = (key(a), key(b));
                    ka.0.cmp(&kb.0)
                        .then(ka.1.cmp(&kb.1))
                        .then(ka.2.total_cmp(&kb.2))
                        .then(ka.3.total_cmp(&kb.3))
                        .then(ka.4.total_cmp(&kb.4))
                })
            else {
                break;
            };
            let mut path: Vec<(Point, f64)> = Vec::new();
            // Onwards where the walk can go on, so dead ends (corners) are left for later
            // paths to start from, at depth 0.
            let next = |node: usize, used: &[bool], left: &[usize]| {
                let open: Vec<usize> = (0..self.edges.len())
                    .filter(|&e| !used[e] && self.edges[e].ends.contains(&node))
                    .collect();
                open.iter()
                    .copied()
                    .find(|&e| left[self.edges[e].other(node)] > 1)
                    .or_else(|| open.first().copied())
            };
            while let Some(e) = next(node, &used, &left) {
                used[e] = true;
                let edge = &self.edges[e];
                left[edge.ends[0]] -= 1;
                left[edge.ends[1]] -= 1;
                let walked = edge.from(node);
                let skip = usize::from(!path.is_empty());
                path.extend(walked.into_iter().skip(skip));
                node = edge.other(node);
            }
            if let Some(&(end, _)) = path.last() {
                at = end;
            }
            paths.push(path);
        }
        paths
    }
}

/// Points after `a` up to and including `b`, split until the radius at every midpoint is
/// within half the tolerance of linear between its ends.
fn split_linear(
    a: Point,
    ra: f64,
    b: Point,
    rb: f64,
    cut: &impl Fn(Point) -> f64,
    depth: u32,
    out: &mut Vec<(Point, f64)>,
) {
    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let rm = cut(m);
    if depth > 0 && ((ra + rb) / 2.0 - rm).abs() > VCARVE_TOL_MM / 2.0 {
        split_linear(a, ra, m, rm, cut, depth - 1, out);
        split_linear(m, rm, b, rb, cut, depth - 1, out);
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

/// One region's outline as integer chords for the Voronoi diagram, and the diagram's edges
/// inside the region (without those between a chord and its own end points).
fn medial_axis(region: &PocketArea, outline: &Outline) -> Result<Graph, ToolpathError> {
    let area = &region.area;
    let mut chords: Vec<[i32; 4]> = Vec::new();
    let mut ends: Vec<(Point, Point)> = Vec::new();
    for pline in std::iter::once(&area.outer).chain(&area.holes) {
        let ring = snapped_ring(&fine_ring(pline))?;
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
    let built = guarded("The V-carve centre line", || {
        Builder::<i32>::default()
            .with_segments(chords.iter())
            .and_then(Builder::build)
    })?;
    let ids = ids_text(&region.contour_ids);
    let failed = |e: boostvoronoi::prelude::BvError| {
        geometry(format!(
            "contours {ids}: the V-carve centre line could not be built ({e}); does the outline \
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
        let site_a = site(&diagram, edge.cell().map_err(failed)?, &ends)?;
        let twin_cell = diagram.edge(twin).map_err(failed)?.cell().map_err(failed)?;
        let site_b = site(&diagram, twin_cell, &ends)?;
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
) -> Result<Site, ToolpathError> {
    let unknown = || geometry("V-carve centre line: a cell without its chord".into());
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

/// The loop's points, arcs split into chords within a quarter of `VCARVE_TOL_MM`: the centre
/// line between chords wobbles by up to their sag, and end branches into chord corners are
/// only dropped when they add less than the tolerance (§4.3).
fn fine_ring(pline: &cavalier_contours::polyline::Polyline<f64>) -> Vec<Point> {
    let sag = VCARVE_TOL_MM / 4.0;
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
fn snapped_ring(points: &[Point]) -> Result<Vec<(i32, i32)>, ToolpathError> {
    let snap = |v: f64| -> Result<i32, ToolpathError> {
        let units = (v / VORONOI_GRID_MM).round();
        if units.is_finite() && units.abs() < f64::from(i32::MAX) {
            // In range, checked just above.
            Ok(units as i32)
        } else {
            Err(geometry(format!(
                "a V-carve outline at {v:.3} mm is too far from the drawing origin (the \
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
