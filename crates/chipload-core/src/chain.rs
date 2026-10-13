// SPDX-License-Identifier: GPL-3.0-or-later
//! Chain primitives into contours.
//!
//! Endpoints within the chaining tolerance snap to shared nodes. Nodes with exactly two
//! connections are joined; any other node ends a chain. A contour is closed only when it
//! returns to its start and every node on it has exactly two connections.

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
    /// Which of the project's DXFs or images that shape came from; None for drawn shapes
    /// and while importing.
    pub from: Option<ShapeFrom>,
}

/// Where a project's shape came from (docs/specs/drawing-sources.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShapeFrom {
    /// The DXF with this id.
    Dxf(u64),
    /// The image with this id.
    Image(u64),
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
pub fn chain_segments_with(segments: &[Segment], tol: f64) -> ChainResult {
    let zero_length: Vec<Segment> = segments
        .iter()
        .filter(|s| s.prim.length() < tol)
        .cloned()
        .collect();
    let mut nodes = NodeIndex {
        tol,
        grid: BTreeMap::new(),
        positions: Vec::new(),
    };
    let (edges, duplicates) = build_edges(
        segments.iter().filter(|s| s.prim.length() >= tol),
        &mut nodes,
        tol,
    );

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
                chains.push(walk(&edges, &adjacency, &mut visited, edge, node));
            }
        }
    }
    // Whatever is left only touches degree-2 nodes, so it forms closed loops.
    for edge in 0..edges.len() {
        if !visited[edge] {
            chains.push(walk(&edges, &adjacency, &mut visited, edge, edges[edge].a));
        }
    }
    chains.sort_by_key(|c| c.edges.iter().copied().min().unwrap_or(usize::MAX));

    let contours = chains
        .into_iter()
        .enumerate()
        .map(|(i, chain)| Contour {
            id: i + 1,
            parts: chain
                .edges
                .iter()
                .zip(&chain.reversed)
                .map(|(&e, &rev)| OrientedSegment {
                    segment: edges[e].segment.clone(),
                    reversed: rev,
                })
                .collect(),
            closed: chain.closed,
            invalid_reason: None,
            nesting_depth: None,
        })
        .collect();
    let branch_points = adjacency
        .iter()
        .filter(|(_, incident)| incident.len() > 2)
        .map(|(&n, _)| nodes.positions[n])
        .collect();
    ChainResult {
        contours,
        zero_length,
        duplicates,
        branch_points,
    }
}

/// Snap endpoints to nodes and drop duplicates (same nodes and same midpoint).
fn build_edges<'a>(
    segments: impl Iterator<Item = &'a Segment>,
    nodes: &mut NodeIndex,
    tol: f64,
) -> (Vec<Edge>, Vec<Segment>) {
    let mut edges: Vec<Edge> = Vec::new();
    let mut duplicates = Vec::new();
    let mut by_nodes: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for segment in segments {
        let a = nodes.node_for(segment.prim.start());
        let b = nodes.node_for(segment.prim.end());
        let key = (a.min(b), a.max(b));
        let mid = segment.prim.midpoint();
        let same = by_nodes
            .get(&key)
            .into_iter()
            .flatten()
            .any(|&other| distance(edges[other].segment.prim.midpoint(), mid) <= tol);
        if same {
            duplicates.push(segment.clone());
            continue;
        }
        by_nodes.entry(key).or_default().push(edges.len());
        edges.push(Edge {
            segment: segment.clone(),
            a,
            b,
        });
    }
    (edges, duplicates)
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
