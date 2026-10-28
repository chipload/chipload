// SPDX-License-Identifier: GPL-3.0-or-later
//! Chain primitives into contours.
//!
//! A shape whose segments follow each other and return to their start is a closed contour as
//! drawn. Other segments chain per layer: endpoints within the chaining tolerance snap to
//! shared nodes, nodes with exactly two connections are joined, and any other node ends a
//! chain. Such a chain is closed only when it returns to its start and every node on it has
//! exactly two connections (docs/specs/chaining-per-layer.md).

use std::collections::BTreeMap;

use crate::Point;
use crate::primitives::{Primitive, distance};
use crate::tolerances::CHAIN_TOL_MM;

/// A primitive plus where it came from in the drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub prim: Primitive,
    /// Effective layer name as written in the DXF.
    pub layer: String,
    /// Handle of the top-level entity (the INSERT for block content).
    pub handle: String,
    /// DXF type of the entity the primitive came from.
    pub source_type: String,
    /// Id of the project shape it belongs to (None while importing a DXF).
    pub shape: Option<u64>,
    /// Which of the project's DXFs, images or texts that shape came from; None for drawn shapes
    /// and while importing.
    pub from: Option<ShapeFrom>,
    /// Construction geometry (docs/specs/dxf-export.md, part B): shown and snapped to, never
    /// cut; it chains only with other construction segments.
    pub construction: bool,
}

/// Where a project's shape came from (docs/specs/drawing-sources.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShapeFrom {
    /// The DXF with this id.
    Dxf(u64),
    /// The image with this id.
    Image(u64),
    /// The text with this id (docs/specs/text.md).
    Text(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrientedSegment {
    pub segment: Segment,
    pub reversed: bool,
}

impl OrientedSegment {
    pub fn points(&self) -> Vec<Point> {
        let mut pts = self.segment.prim.points();
        if self.reversed {
            pts.reverse();
        }
        pts
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    pub id: usize,
    pub parts: Vec<OrientedSegment>,
    pub closed: bool,
    /// Set for closed contours that cannot be machined.
    pub invalid_reason: Option<String>,
    /// Closed and valid only; even = outer, odd = hole.
    pub nesting_depth: Option<usize>,
}

impl Contour {
    /// Construction geometry: never cut (docs/specs/dxf-export.md §B3).
    pub fn is_construction(&self) -> bool {
        self.parts.iter().any(|p| p.segment.construction)
    }

    /// The same contour moved by `offset` (a copy of the part, docs/specs/copies-grid.md).
    pub fn translated(&self, offset: Point) -> Contour {
        let mut out = self.clone();
        for part in &mut out.parts {
            part.segment.prim = part.segment.prim.translated(offset);
        }
        out
    }

    pub fn start(&self) -> Point {
        self.parts.first().map_or((0.0, 0.0), |p| p.points()[0])
    }

    pub fn end(&self) -> Point {
        self.parts
            .last()
            .and_then(|p| p.points().last().copied())
            .unwrap_or((0.0, 0.0))
    }

    /// Sorted, distinct layer names.
    pub fn layers(&self) -> Vec<String> {
        let mut layers: Vec<String> = self.parts.iter().map(|p| p.segment.layer.clone()).collect();
        layers.sort();
        layers.dedup();
        layers
    }

    /// Polyline through the contour. For closed contours the end point is not repeated.
    pub fn points(&self) -> Vec<Point> {
        let mut pts = Vec::new();
        for (i, part) in self.parts.iter().enumerate() {
            let p = part.points();
            pts.extend(if i == 0 { &p[..] } else { &p[1..] });
        }
        if self.closed {
            pts.pop();
        }
        pts
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChainResult {
    pub contours: Vec<Contour>,
    pub zero_length: Vec<Segment>,
    pub duplicates: Vec<Segment>,
    pub branch_points: Vec<Point>,
}

/// Snaps points to nodes. The first point to create a node defines its position.
struct NodeIndex {
    tol: f64,
    grid: BTreeMap<(i64, i64), Vec<usize>>,
    positions: Vec<Point>,
}

impl NodeIndex {
    fn cell(&self, p: Point) -> (i64, i64) {
        // Imported coordinates are limited (chipload-dxf's MAX_COORDINATE_MM), which keeps
        // these far inside i64; `as` saturates and the neighbours below add saturating, so
        // anything else can't overflow either.
        (
            (p.0 / self.tol).floor() as i64,
            (p.1 / self.tol).floor() as i64,
        )
    }

    fn node_for(&mut self, p: Point) -> usize {
        let (cx, cy) = self.cell(p);
        let mut best: Option<usize> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                let near = (cx.saturating_add(dx), cy.saturating_add(dy));
                for &node in self.grid.get(&near).into_iter().flatten() {
                    if distance(self.positions[node], p) <= self.tol {
                        best = Some(best.map_or(node, |b| b.min(node)));
                    }
                }
            }
        }
        if let Some(node) = best {
            return node;
        }
        let node = self.positions.len();
        self.positions.push(p);
        self.grid.entry((cx, cy)).or_default().push(node);
        node
    }
}

struct Edge {
    segment: Segment,
    a: usize,
    b: usize,
    /// The segment's index in the input, for drawing order.
    index: usize,
}

struct Chain {
    edges: Vec<usize>,
    reversed: Vec<bool>,
    closed: bool,
}

pub fn chain_segments(segments: &[Segment]) -> ChainResult {
    chain_segments_with(segments, CHAIN_TOL_MM)
}

/// Chain segments (in drawing order) into contours. Deterministic for a given input order.
///
/// A shape's segments (consecutive segments with the same `shape`) that follow each other
/// and return to their start are a closed contour as drawn: never cut where other lines touch
/// them, never joined to others. Everything else chains with the segments of its own layer
/// only (docs/specs/chaining-per-layer.md).
pub fn chain_segments_with(segments: &[Segment], tol: f64) -> ChainResult {
    let zero_length: Vec<Segment> = segments
        .iter()
        .filter(|s| s.prim.length() < tol)
        .cloned()
        .collect();
    let kept: Vec<(usize, &Segment)> = segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.prim.length() >= tol)
        .collect();
    let (closed, loose): (Vec<Run>, Vec<Run>) = shape_runs(&kept)
        .into_iter()
        .partition(|run| closes_as_drawn(run, tol));

    let mut layers: BTreeMap<String, LayerGraph> = BTreeMap::new();
    let mut duplicates: Vec<(usize, Segment)> = Vec::new();
    // (first segment's index in the input, parts, closed)
    let mut found: Vec<(usize, Vec<OrientedSegment>, bool)> = Vec::new();
    for run in closed {
        let Some(&(first, segment)) = run.first() else {
            continue;
        };
        let graph = layer_graph(&mut layers, segment, tol);
        let keys: Vec<EdgeKey> = run.iter().map(|(_, s)| graph.key(s)).collect();
        if keys.iter().all(|k| graph.known(k)) {
            duplicates.extend(run.iter().map(|&(i, s)| (i, s.clone())));
            continue;
        }
        for key in keys {
            graph.remember(key);
        }
        let parts = run
            .iter()
            .map(|(_, s)| OrientedSegment {
                segment: (*s).clone(),
                reversed: false,
            })
            .collect();
        found.push((first, parts, true));
    }
    for &(index, segment) in loose.iter().flat_map(|run| run.iter()) {
        let graph = layer_graph(&mut layers, segment, tol);
        let key = graph.key(segment);
        if graph.known(&key) {
            duplicates.push((index, segment.clone()));
            continue;
        }
        let (a, b) = (key.a, key.b);
        graph.remember(key);
        graph.edges.push(Edge {
            segment: segment.clone(),
            a,
            b,
            index,
        });
    }

    let mut branch_points = Vec::new();
    for graph in layers.values() {
        let (chains, branches) = chain_edges(&graph.edges);
        branch_points.extend(branches.into_iter().map(|n| graph.nodes.positions[n]));
        for chain in chains {
            let first = chain
                .edges
                .iter()
                .map(|&e| graph.edges[e].index)
                .min()
                .unwrap_or(usize::MAX);
            let parts = chain
                .edges
                .iter()
                .zip(&chain.reversed)
                .map(|(&e, &rev)| OrientedSegment {
                    segment: graph.edges[e].segment.clone(),
                    reversed: rev,
                })
                .collect();
            found.push((first, parts, chain.closed));
        }
    }
    found.sort_by_key(|(first, _, _)| *first);
    duplicates.sort_by_key(|(index, _)| *index);

    let contours = found
        .into_iter()
        .enumerate()
        .map(|(i, (_, parts, closed))| Contour {
            id: i + 1,
            parts,
            closed,
            invalid_reason: None,
            nesting_depth: None,
        })
        .collect();
    ChainResult {
        contours,
        zero_length,
        duplicates: duplicates.into_iter().map(|(_, s)| s).collect(),
        branch_points,
    }
}

/// Segments with their index in the input.
type Run<'a, 's> = &'a [(usize, &'s Segment)];

/// The segments in runs: a shape's consecutive segments together, a segment without a shape
/// alone.
fn shape_runs<'a, 's>(kept: Run<'a, 's>) -> Vec<Run<'a, 's>> {
    let mut runs = Vec::new();
    let mut start = 0;
    for i in 1..=kept.len() {
        let same =
            i < kept.len() && kept[i].1.shape.is_some() && kept[i].1.shape == kept[start].1.shape;
        if !same {
            runs.push(&kept[start..i]);
            start = i;
        }
    }
    runs
}

/// Whether a shape's segments follow each other and end where they start, with no segment
/// drawn twice: a closed contour as drawn.
fn closes_as_drawn(run: &[(usize, &Segment)], tol: f64) -> bool {
    let (Some(first), Some(last)) = (run.first(), run.last()) else {
        return false;
    };
    if first.1.shape.is_none() || distance(last.1.prim.end(), first.1.prim.start()) > tol {
        return false;
    }
    let follows = run
        .windows(2)
        .all(|w| distance(w[0].1.prim.end(), w[1].1.prim.start()) <= tol);
    let mut seen = LayerGraph::new(tol);
    let twice = run.iter().any(|(_, s)| {
        let key = seen.key(s);
        let known = seen.known(&key);
        seen.remember(key);
        known
    });
    follows && !twice
}

/// One layer's nodes and edges.
struct LayerGraph {
    nodes: NodeIndex,
    /// The midpoints of the segments between two nodes, to find duplicates.
    between: BTreeMap<(usize, usize), Vec<Point>>,
    /// Loose segments, to chain.
    edges: Vec<Edge>,
}

/// A segment's two nodes and its midpoint.
struct EdgeKey {
    a: usize,
    b: usize,
    mid: Point,
}

impl LayerGraph {
    fn new(tol: f64) -> Self {
        LayerGraph {
            nodes: NodeIndex {
                tol,
                grid: BTreeMap::new(),
                positions: Vec::new(),
            },
            between: BTreeMap::new(),
            edges: Vec::new(),
        }
    }

    fn key(&mut self, segment: &Segment) -> EdgeKey {
        EdgeKey {
            a: self.nodes.node_for(segment.prim.start()),
            b: self.nodes.node_for(segment.prim.end()),
            mid: segment.prim.midpoint(),
        }
    }

    /// The same nodes and the same midpoint as a segment already here.
    fn known(&self, key: &EdgeKey) -> bool {
        self.between
            .get(&(key.a.min(key.b), key.a.max(key.b)))
            .into_iter()
            .flatten()
            .any(|&mid| distance(mid, key.mid) <= self.nodes.tol)
    }

    fn remember(&mut self, key: EdgeKey) {
        self.between
            .entry((key.a.min(key.b), key.a.max(key.b)))
            .or_default()
            .push(key.mid);
    }
}

/// The graph of a layer; layers are compared ignoring case, as selectors compare them.
/// The graph `segment` chains in: its layer's (DXF layer names ignore case), construction
/// geometry apart from the rest so the two never join.
fn layer_graph<'a>(
    layers: &'a mut BTreeMap<String, LayerGraph>,
    segment: &Segment,
    tol: f64,
) -> &'a mut LayerGraph {
    let mut key = segment.layer.to_lowercase();
    if segment.construction {
        key.push_str("\u{0}construction");
    }
    layers.entry(key).or_insert_with(|| LayerGraph::new(tol))
}

/// Chain one layer's loose edges; also the nodes where more than two meet.
fn chain_edges(edges: &[Edge]) -> (Vec<Chain>, Vec<usize>) {
    let mut adjacency: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, edge) in edges.iter().enumerate() {
        adjacency.entry(edge.a).or_default().push(i);
        adjacency.entry(edge.b).or_default().push(i);
    }

    let mut visited = vec![false; edges.len()];
    let mut chains: Vec<Chain> = Vec::new();
    // Chains that end at a node with != 2 connections are open.
    for (&node, incident) in &adjacency {
        if incident.len() == 2 {
            continue;
        }
        for &edge in incident {
            if !visited[edge] {
                chains.push(walk(edges, &adjacency, &mut visited, edge, node));
            }
        }
    }
    // Whatever is left only touches degree-2 nodes, so it forms closed loops.
    for edge in 0..edges.len() {
        if !visited[edge] {
            chains.push(walk(edges, &adjacency, &mut visited, edge, edges[edge].a));
        }
    }
    let branches = adjacency
        .iter()
        .filter(|(_, incident)| incident.len() > 2)
        .map(|(&n, _)| n)
        .collect();
    (chains, branches)
}

fn walk(
    edges: &[Edge],
    adjacency: &BTreeMap<usize, Vec<usize>>,
    visited: &mut [bool],
    first_edge: usize,
    start_node: usize,
) -> Chain {
    let degree = |n: usize| adjacency.get(&n).map_or(0, Vec::len);
    let mut chain = Chain {
        edges: Vec::new(),
        reversed: Vec::new(),
        closed: false,
    };
    let mut edge_index = first_edge;
    let mut node = start_node;
    loop {
        visited[edge_index] = true;
        let edge = &edges[edge_index];
        let is_reversed = edge.a != node;
        chain.edges.push(edge_index);
        chain.reversed.push(is_reversed);
        node = if is_reversed { edge.a } else { edge.b };
        if degree(node) != 2 {
            return chain;
        }
        if node == start_node {
            chain.closed = true;
            return chain;
        }
        let next = adjacency
            .get(&node)
            .into_iter()
            .flatten()
            .copied()
            .find(|&i| !visited[i]);
        match next {
            Some(i) => edge_index = i,
            None => return chain,
        }
    }
}
