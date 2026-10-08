"""Profile toolpaths around closed contours. See docs/specs/v0-profile-pipeline.md section 4."""

from __future__ import annotations

import math
from typing import Literal

import shapely
from shapely.geometry import Polygon

from chipload.core.errors import GeometryError
from chipload.core.geometry.offset import buffer_polygons, dedupe_ring, ring_points
from chipload.core.geometry.primitives import Point, signed_area
from chipload.core.tolerances import (
    ARC_APPROX_TOL_MM,
    DEPTH_TOL_MM,
    OFFSET_CHECK_SLACK_MM,
)
from chipload.core.toolpath.model import Move, Spindle

Side = Literal["inside", "outside", "on"]
Direction = Literal["climb", "conventional"]


def offset_rings(ring: list[Point], side: Side, tool_radius: float) -> list[list[Point]]:
    """Tool-centre rings for one closed contour.

    `side` is relative to the given loop itself. Returns one ring for outside/on; inside can
    return several when the offset splits at a narrow neck. Raises GeometryError if the tool
    does not fit.
    """
    ring = dedupe_ring(ring)
    if side == "on":
        return [ring]
    polygon = Polygon(ring)
    signed = tool_radius if side == "outside" else -tool_radius
    polygons = buffer_polygons(polygon, signed)
    if not polygons:
        raise GeometryError(
            f"Tool (diameter {2 * tool_radius:.3f} mm) does not fit inside the contour"
        )
    rings = [ring_points(p.exterior.coords) for p in polygons]
    for path in rings:
        _verify_offset(polygon, path, side, tool_radius)
    return sorted(rings, key=lambda r: _start_key(r))


def _verify_offset(polygon: Polygon, path: list[Point], side: Side, tool_radius: float) -> None:
    """Post-condition: never trust the buffer blindly. Every vertex must be r from the contour,
    every chord midpoint at least r - tolerance (no gouge), all on the requested side."""
    midpoints = [
        ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        for a, b in zip(path, path[1:] + path[:1], strict=True)
    ]
    vertices = shapely.points(path)
    vertex_dist = [float(d) for d in shapely.distance(polygon.exterior, vertices)]
    mid_dist = [float(d) for d in shapely.distance(polygon.exterior, shapely.points(midpoints))]
    inside = [bool(v) for v in shapely.contains(polygon, vertices)]
    lowest = tool_radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM
    highest = tool_radius + OFFSET_CHECK_SLACK_MM
    wrong_side = any(inside) if side == "outside" else not all(inside)
    if max(vertex_dist) > highest or min(vertex_dist + mid_dist) < lowest or wrong_side:
        raise GeometryError(
            "Offset failed verification (path not at tool radius from the contour); "
            "the contour may be degenerate"
        )


def cut_clockwise(side: Side, direction: Direction, spindle: Spindle) -> bool:
    """Travel direction around the ring.

    With a clockwise (M3) spindle, climb milling keeps the material on the right of the
    direction of travel. Outside (and `on`, by convention) has the part on the inside of the
    loop, so climb is clockwise; inside has the material outside the loop, so climb is
    counter-clockwise. Conventional and a counter-clockwise spindle each flip the result.
    """
    clockwise = (side != "inside") == (direction == "climb")
    return clockwise if spindle == "cw" else not clockwise


def orient(ring: list[Point], clockwise: bool) -> list[Point]:
    """Order the ring for the travel direction and start at the lowest (then leftmost) vertex."""
    is_clockwise = signed_area(ring) < 0
    ordered = list(ring) if is_clockwise == clockwise else list(reversed(ring))
    start = min(range(len(ordered)), key=lambda i: _start_key([ordered[i]]))
    return ordered[start:] + ordered[:start]


def pass_depths(depth: float, step_down: float) -> list[float]:
    """Equal passes no deeper than step_down; the last one is exactly `depth`."""
    if depth <= 0 or step_down <= 0:
        raise GeometryError(f"depth ({depth}) and step_down ({step_down}) must be positive")
    count = max(1, math.ceil((depth - DEPTH_TOL_MM) / step_down))
    return [depth * i / count for i in range(1, count + 1)]


def contour_moves(
    ring: list[Point], pass_z: list[float], safe_z: float, feed: float, plunge_feed: float
) -> list[Move]:
    """Retract, travel, then for each pass plunge and go once around. Ends retracted."""
    start = ring[0]
    moves = [Move("rapid", z=safe_z), Move("rapid", x=start[0], y=start[1])]
    for z in pass_z:
        moves.append(Move("feed", z=z, feed=plunge_feed))
        moves.extend(Move("feed", x=x, y=y, feed=feed) for x, y in ring[1:])
        moves.append(Move("feed", x=start[0], y=start[1], feed=feed))
    moves.append(Move("rapid", z=safe_z))
    return moves


def _start_key(ring: list[Point]) -> tuple[float, float]:
    lowest = min(ring, key=lambda p: (p[1], p[0]))
    return (lowest[1], lowest[0])
