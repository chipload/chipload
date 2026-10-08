"""Chain primitives into contours.

Endpoints within the chaining tolerance are snapped to shared nodes. Nodes with exactly two
connections are joined; any other node ends a chain. A contour is closed only when it returns
to its start and every node on it has exactly two connections.
"""

from __future__ import annotations

import math
from collections import defaultdict
from collections.abc import Sequence
from dataclasses import dataclass

from chipload.core.geometry.primitives import Point, Primitive, distance
from chipload.core.tolerances import CHAIN_TOL_MM


@dataclass(frozen=True, slots=True)
class Segment:
    """A primitive plus where it came from in the drawing."""

    prim: Primitive
    layer: str  # effective layer name as written in the DXF
    handle: str  # handle of the top-level entity (the INSERT for block content)
    source_type: str  # DXF type of the entity the primitive came from


@dataclass(frozen=True, slots=True)
class OrientedSegment:
    segment: Segment
    reversed: bool

    def points(self) -> list[Point]:
        pts = self.segment.prim.points()
        return pts[::-1] if self.reversed else pts


@dataclass(frozen=True, slots=True)
class Contour:
    id: int
    parts: tuple[OrientedSegment, ...]
    closed: bool
    invalid_reason: str | None = None  # set for closed contours that cannot be machined
    nesting_depth: int | None = None  # closed and valid only; even = outer, odd = hole

    @property
    def start(self) -> Point:
        return self.parts[0].points()[0]

    @property
    def end(self) -> Point:
        return self.parts[-1].points()[-1]

    @property
    def layers(self) -> tuple[str, ...]:
        return tuple(sorted({p.segment.layer for p in self.parts}))

    def points(self) -> list[Point]:
        """Polyline through the contour. For closed contours the end point is not repeated."""
        pts = list(self.parts[0].points())
        for part in self.parts[1:]:
            pts.extend(part.points()[1:])
        if self.closed:
            pts.pop()
        return pts


@dataclass(frozen=True, slots=True)
class ChainResult:
    contours: tuple[Contour, ...]
    zero_length: tuple[Segment, ...]
    duplicates: tuple[Segment, ...]
    branch_points: tuple[Point, ...]


class _NodeIndex:
    """Snaps points to nodes. The first point to create a node defines its position."""

    def __init__(self, tol: float) -> None:
        self._tol = tol
        self._grid: dict[tuple[int, int], list[int]] = defaultdict(list)
        self.positions: list[Point] = []

    def _cell(self, p: Point) -> tuple[int, int]:
        return (math.floor(p[0] / self._tol), math.floor(p[1] / self._tol))

    def node_for(self, p: Point) -> int:
        cx, cy = self._cell(p)
        candidates = [
            node
            for dx in (-1, 0, 1)
            for dy in (-1, 0, 1)
            for node in self._grid.get((cx + dx, cy + dy), [])
            if distance(self.positions[node], p) <= self._tol
        ]
        if candidates:
            return min(candidates)
        node = len(self.positions)
        self.positions.append(p)
        self._grid[(cx, cy)].append(node)
        return node


@dataclass(frozen=True, slots=True)
class _Edge:
    segment: Segment
    a: int  # node at the primitive's start
    b: int  # node at the primitive's end


def chain_segments(segments: Sequence[Segment], tol: float = CHAIN_TOL_MM) -> ChainResult:
    """Chain segments (in drawing order) into contours. Deterministic for a given input order."""
    zero_length = [s for s in segments if s.prim.length() < tol]
    nodes = _NodeIndex(tol)
    edges, duplicates = _build_edges([s for s in segments if s.prim.length() >= tol], nodes, tol)

    adjacency: dict[int, list[int]] = defaultdict(list)
    for i, edge in enumerate(edges):
        adjacency[edge.a].append(i)
        adjacency[edge.b].append(i)

    visited = [False] * len(edges)
    chains: list[_Chain] = []

    # Chains that end at a node with != 2 connections are open.
    for node in sorted(adjacency):
        if len(adjacency[node]) == 2:
            continue
        for edge_index in adjacency[node]:
            if not visited[edge_index]:
                chains.append(_walk(edges, adjacency, visited, edge_index, node))

    # Whatever is left only touches degree-2 nodes, so it forms closed loops.
    for edge_index in range(len(edges)):
        if not visited[edge_index]:
            chains.append(_walk(edges, adjacency, visited, edge_index, edges[edge_index].a))

    chains.sort(key=lambda chain: min(chain.edge_indices))
    contours = tuple(
        Contour(
            id=number,
            parts=tuple(
                OrientedSegment(edges[i].segment, rev)
                for i, rev in zip(chain.edge_indices, chain.reversed_flags, strict=True)
            ),
            closed=chain.closed,
        )
        for number, chain in enumerate(chains, start=1)
    )
    branch_points = tuple(nodes.positions[n] for n in sorted(adjacency) if len(adjacency[n]) > 2)
    return ChainResult(contours, tuple(zero_length), tuple(duplicates), branch_points)


@dataclass(slots=True)
class _Chain:
    edge_indices: list[int]
    reversed_flags: list[bool]
    closed: bool


def _build_edges(
    segments: Sequence[Segment], nodes: _NodeIndex, tol: float
) -> tuple[list[_Edge], list[Segment]]:
    """Snap endpoints to nodes and drop duplicates (same nodes and same midpoint)."""
    edges: list[_Edge] = []
    duplicates: list[Segment] = []
    by_nodes: dict[tuple[int, int], list[_Edge]] = defaultdict(list)
    for segment in segments:
        a = nodes.node_for(segment.prim.start)
        b = nodes.node_for(segment.prim.end)
        key = (min(a, b), max(a, b))
        mid = segment.prim.midpoint()
        if any(distance(other.segment.prim.midpoint(), mid) <= tol for other in by_nodes[key]):
            duplicates.append(segment)
            continue
        edge = _Edge(segment, a, b)
        by_nodes[key].append(edge)
        edges.append(edge)
    return edges, duplicates


def _walk(
    edges: list[_Edge],
    adjacency: dict[int, list[int]],
    visited: list[bool],
    first_edge: int,
    start_node: int,
) -> _Chain:
    chain = _Chain([], [], closed=False)
    edge_index = first_edge
    node = start_node
    while True:
        visited[edge_index] = True
        edge = edges[edge_index]
        is_reversed = edge.a != node
        chain.edge_indices.append(edge_index)
        chain.reversed_flags.append(is_reversed)
        node = edge.a if is_reversed else edge.b
        if len(adjacency[node]) != 2:
            return chain
        if node == start_node:
            chain.closed = True
            return chain
        unvisited = [i for i in adjacency[node] if not visited[i]]
        if not unvisited:
            return chain
        edge_index = unvisited[0]
