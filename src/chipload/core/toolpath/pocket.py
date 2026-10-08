"""Pocket clearing: contour-parallel rings, cut from the inside out. See docs/specs/pocket.md."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass, field
from itertools import pairwise

import shapely
from shapely.geometry import LinearRing, LineString, Polygon
from shapely.geometry import Point as ShapelyPoint
from shapely.geometry.base import BaseGeometry
from shapely.ops import unary_union

from chipload.core.errors import GeometryError, SelectorError
from chipload.core.geometry.offset import (
    dedupe_ring,
    dilate_polygons,
    erode_polygons,
    quad_segs_for,
    ring_points,
)
from chipload.core.geometry.primitives import Point, distance, signed_area
from chipload.core.tolerances import (
    ARC_APPROX_TOL_MM,
    CHAIN_TOL_MM,
    OFFSET_CHECK_SLACK_MM,
    REST_TOL_MM,
)
from chipload.core.toolpath.model import Move, Spindle
from chipload.core.toolpath.profile import Direction, cut_clockwise

# Guard against runaway loops on absurd input; a 2 m pocket at 0.1 mm stepover is 10 000.
MAX_RING_LEVELS = 20_000


@dataclass(frozen=True, slots=True)
class PocketPiece:
    """One connected area to clear, with its rings in cut order."""

    rings: tuple[tuple[Point, ...], ...]  # each oriented, starting (and ending) at ring[0]
    feed_links: tuple[bool, ...]  # feed_links[i]: stay at depth from rings[i] into rings[i + 1]
    contour_ids: tuple[int, ...]  # selected contours that make up this piece, for messages


@dataclass(frozen=True, slots=True)
class PocketPlan:
    pieces: tuple[PocketPiece, ...]
    notes: tuple[str, ...]  # warnings: narrow parts, uncut material


def build_region(contours: Sequence[tuple[int, list[Point]]]) -> list[Polygon]:
    """Area to clear from selected closed contours (id, ring), as pieces sorted by (y, x).

    Nesting among the selected contours decides boundary (even depth) or island (odd depth);
    overlapping contours at the same depth are merged.
    """
    polygons = [(cid, Polygon(dedupe_ring(ring))) for cid, ring in contours]
    _refuse_duplicates(polygons)
    depth = {
        cid: sum(1 for other_id, other in polygons if other_id != cid and other.contains(poly))
        for cid, poly in polygons
    }
    region: BaseGeometry = Polygon()
    for level in range(max(depth.values(), default=-1) + 1):
        layer = unary_union([poly for cid, poly in polygons if depth[cid] == level])
        region = region.union(layer) if level % 2 == 0 else region.difference(layer)
    pieces = [
        p
        for p in shapely.get_parts(region)
        if isinstance(p, Polygon) and p.area > CHAIN_TOL_MM * CHAIN_TOL_MM
    ]
    return sorted(pieces, key=lambda p: (p.bounds[1], p.bounds[0]))


def plan_pocket(
    contours: Sequence[tuple[int, list[Point]]],
    tool_radius: float,
    stepover: float,
    direction: Direction,
    spindle: Spindle,
    keep_tool_down: bool,
    start: Point,
) -> PocketPlan:
    """Rings, cut order and links for every piece. `stepover` is in mm."""
    if stepover <= 0:
        raise GeometryError(f"stepover must be positive, got {stepover}")
    polygons = {cid: Polygon(dedupe_ring(ring)) for cid, ring in contours}
    region_pieces = build_region(contours)
    if not region_pieces:
        raise GeometryError(
            "Nothing to clear: the selected islands cover the selected boundaries "
            f"(contours {_ids(sorted(polygons))})"
        )
    pieces: list[PocketPiece] = []
    notes: list[str] = []
    position = start
    remaining = list(region_pieces)
    while remaining:
        here = ShapelyPoint(position)
        region = min(remaining, key=lambda p: (p.distance(here), p.bounds[1], p.bounds[0]))
        remaining.remove(region)
        ids = tuple(sorted(cid for cid, poly in polygons.items() if poly.intersects(region)))
        roots = _ring_tree(region, tool_radius, stepover, ids)
        if len(roots) > 1:
            notes.append(
                f"contours {_ids(ids)}: the tool does not fit through a narrow part; "
                f"cleared as {len(roots)} areas and material remains in between"
            )
        rings = _cut_order(roots, position, direction, spindle)
        links = tuple(
            keep_tool_down and _link_is_safe(region, a[0], b[0], tool_radius)
            for a, b in pairwise(rings)
        )
        _verify(region, rings, links, tool_radius, ids)
        notes.extend(_uncut_material(region, rings, links, tool_radius, ids))
        pieces.append(PocketPiece(tuple(tuple(r) for r in rings), links, ids))
        position = rings[0][0]  # every depth starts again at the first ring
    return PocketPlan(tuple(pieces), tuple(notes))


def pocket_moves(
    piece: PocketPiece, pass_z: list[float], safe_z: float, feed: float, plunge_feed: float
) -> list[Move]:
    """All depth passes of one piece. Each pass starts at the centre and ends retracted."""
    moves: list[Move] = []
    for z in pass_z:
        first = piece.rings[0][0]
        moves += [
            Move("rapid", z=safe_z),
            Move("rapid", x=first[0], y=first[1]),
            Move("feed", z=z, feed=plunge_feed),
        ]
        for index, ring in enumerate(piece.rings):
            start = ring[0]
            if index > 0:
                if piece.feed_links[index - 1]:
                    moves.append(Move("feed", x=start[0], y=start[1], feed=feed))
                else:
                    moves += [
                        Move("rapid", z=safe_z),
                        Move("rapid", x=start[0], y=start[1]),
                        Move("feed", z=z, feed=plunge_feed),
                    ]
            moves.extend(Move("feed", x=x, y=y, feed=feed) for x, y in ring[1:])
            moves.append(Move("feed", x=start[0], y=start[1], feed=feed))
        moves.append(Move("rapid", z=safe_z))
    return moves


# -- region


def _refuse_duplicates(polygons: list[tuple[int, Polygon]]) -> None:
    """The same shape selected twice can't be both boundary and island: refuse."""
    for i, (id_a, a) in enumerate(polygons):
        for id_b, b in polygons[i + 1 :]:
            if not a.envelope.intersects(b.envelope):
                continue
            slack = CHAIN_TOL_MM * max(a.length, b.length)
            if a.symmetric_difference(b).area <= slack:
                raise SelectorError(
                    f"contours #{id_a} and #{id_b} are the same shape; select only one of them"
                )


# -- rings


@dataclass
class _Node:
    polygon: Polygon
    children: list[_Node] = field(default_factory=list)


def _ring_tree(
    region: Polygon, radius: float, stepover: float, ids: tuple[int, ...]
) -> list[_Node]:
    """Offset levels as a tree: every level k+1 polygon lies inside one level k polygon."""
    roots = [_Node(p) for p in erode_polygons(region, radius)]
    if not roots:
        raise GeometryError(
            f"contours {_ids(ids)}: tool (diameter {2 * radius:.3f} mm) does not fit in the pocket"
        )
    level = roots
    for k in range(1, MAX_RING_LEVELS + 1):
        polygons = erode_polygons(region, radius + k * stepover)
        if not polygons:
            return roots
        next_level: list[_Node] = []
        for polygon in polygons:
            inside = polygon.representative_point()
            parent = next((n for n in level if n.polygon.contains(inside)), None)
            if parent is None:
                raise GeometryError(f"contours {_ids(ids)}: pocket offsets are not nested")
            node = _Node(polygon)
            parent.children.append(node)
            next_level.append(node)
        level = next_level
    raise GeometryError(f"contours {_ids(ids)}: more than {MAX_RING_LEVELS} pocket rings")


def _cut_order(
    roots: list[_Node], position: Point, direction: Direction, spindle: Spindle
) -> list[list[Point]]:
    """Children before parents (inside out); siblings nearest first. A polygon's island rings
    are cut before its outer ring, so the wall is the last ring at each depth.

    Iterative, because a large pocket with a small stepover nests deeper than Python's
    recursion limit.
    """
    wall_cw = cut_clockwise("inside", direction, spindle)  # material outside the ring
    island_cw = cut_clockwise("outside", direction, spindle)  # material inside the ring
    rings: list[list[Point]] = []
    stack: list[tuple[list[_Node], _Node | None]] = [(list(roots), None)]
    while stack:
        siblings, waiting = stack[-1]
        if siblings:
            here = ShapelyPoint(position)
            node = min(
                siblings,
                key=lambda n: (n.polygon.distance(here), n.polygon.bounds[1], n.polygon.bounds[0]),
            )
            siblings.remove(node)
            stack.append((list(node.children), node))
            continue
        stack.pop()
        if waiting is None:
            continue
        islands = [ring_points(i.coords) for i in waiting.polygon.interiors]
        while islands:
            nearest = min(islands, key=lambda r: min(distance(position, p) for p in r))
            islands.remove(nearest)
            rings.append(_orient_from(nearest, island_cw, position))
            position = rings[-1][0]
        rings.append(_orient_from(ring_points(waiting.polygon.exterior.coords), wall_cw, position))
        position = rings[-1][0]
    return rings


def _orient_from(ring: list[Point], clockwise: bool, near: Point) -> list[Point]:
    """Order for the travel direction and start at the vertex nearest `near` (ties: y, x)."""
    ordered = list(ring) if (signed_area(ring) < 0) == clockwise else list(reversed(ring))
    start = min(
        range(len(ordered)),
        key=lambda i: (distance(near, ordered[i]), ordered[i][1], ordered[i][0]),
    )
    return ordered[start:] + ordered[:start]


# -- links and checks


def ring_geometry(ring: Sequence[Point]) -> BaseGeometry:
    """A ring as Shapely geometry. The last offset levels can collapse to one or two points:
    the tool then just plunges there, or moves a tiny distance and back."""
    if len(ring) == 1:
        return ShapelyPoint(ring[0])
    if len(ring) == 2:
        return LineString(ring)
    return LinearRing(ring)


def _min_clearance(radius: float) -> float:
    return radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM


def _link_is_safe(region: Polygon, a: Point, b: Point, radius: float) -> bool:
    """Staying down is safe if the tool can't reach a wall or island and the link is short."""
    if distance(a, b) > 2 * radius:
        return False
    if distance(a, b) < CHAIN_TOL_MM:
        return True
    segment = LineString([a, b])
    return bool(
        region.contains(ShapelyPoint(a))
        and shapely.distance(region.boundary, segment) >= _min_clearance(radius)
    )


def _verify(
    region: Polygon,
    rings: list[list[Point]],
    links: tuple[bool, ...],
    radius: float,
    ids: tuple[int, ...],
) -> None:
    """Post-condition: no cutting move comes closer than r (minus tolerance) to a wall or
    island, and everything is inside the region. Never trust the offsets blindly."""
    paths: list[BaseGeometry] = [ring_geometry(r) for r in rings]
    paths += [
        LineString([rings[i][0], rings[i + 1][0]])
        for i, stays_down in enumerate(links)
        if stays_down and distance(rings[i][0], rings[i + 1][0]) >= CHAIN_TOL_MM
    ]
    for path in paths:
        inside = region.contains(ShapelyPoint(path.coords[0]))
        if not inside or shapely.distance(region.boundary, path) < _min_clearance(radius):
            raise GeometryError(
                f"contours {_ids(ids)}: pocket toolpath failed verification (too close to a "
                "wall or island); the drawing may be degenerate"
            )


def _uncut_material(
    region: Polygon,
    rings: list[list[Point]],
    links: tuple[bool, ...],
    radius: float,
    ids: tuple[int, ...],
) -> list[str]:
    """Material the tool could reach but the rings leave behind (spec section 3)."""
    quad_segs = quad_segs_for(radius)
    reachable = unary_union(
        [q for p in erode_polygons(region, radius) for q in dilate_polygons(p, radius)]
    )
    swept_parts: list[BaseGeometry] = [
        ring_geometry(r).buffer(radius, quad_segs=quad_segs) for r in rings
    ]
    swept_parts += [
        LineString([rings[i][0], rings[i + 1][0]]).buffer(radius, quad_segs=quad_segs)
        for i, stays_down in enumerate(links)
        if stays_down and distance(rings[i][0], rings[i + 1][0]) >= CHAIN_TOL_MM
    ]
    leftover = reachable.difference(unary_union(swept_parts))
    spots = [
        piece
        for piece in shapely.get_parts(leftover)
        if isinstance(piece, Polygon) and not piece.buffer(-REST_TOL_MM).is_empty
    ]
    if not spots:
        return []
    largest = max(spots, key=lambda p: (p.area, -p.bounds[1], -p.bounds[0]))
    x0, y0, x1, y1 = largest.bounds
    centre = largest.representative_point()
    places = f"{len(spots)} place{'s' if len(spots) > 1 else ''}"
    note = (
        f"contours {_ids(ids)}: uncut material in {places}, largest about "
        f"{x1 - x0:.2f} x {y1 - y0:.2f} mm near ({centre.x:.1f}, {centre.y:.1f}); "
        "reduce the stepover to clear it"
    )
    return [note]


def _ids(ids: Sequence[int]) -> str:
    return ", ".join(f"#{i}" for i in ids)
