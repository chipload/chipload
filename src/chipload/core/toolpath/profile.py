"""Profile toolpaths around closed contours. See docs/specs/v0-profile-pipeline.md section 4."""

from __future__ import annotations

import math
from typing import Literal

from shapely.geometry import MultiPolygon, Polygon
from shapely.geometry.base import BaseGeometry

from chipload.core.errors import GeometryError
from chipload.core.geometry.primitives import Point, distance, max_segment_angle, signed_area
from chipload.core.tolerances import CHAIN_TOL_MM, DEPTH_TOL_MM
from chipload.core.toolpath.model import Move, Spindle

# GEOS rounds the number of chords in a round join to the nearest integer, so one chord can
# span up to 1.5x the nominal angle (pi/2 / quad_segs). Measured with Shapely 2.2 / GEOS;
# tests/property/test_profile_offset.py guards it.
GEOS_JOIN_CHORD_FACTOR = 1.5

Side = Literal["inside", "outside", "on"]
Direction = Literal["climb", "conventional"]


def offset_rings(ring: list[Point], side: Side, tool_radius: float) -> list[list[Point]]:
    """Tool-centre rings for one closed contour.

    `side` is relative to the given loop itself. Returns one ring for outside/on; inside can
    return several when the offset splits at a narrow neck. Raises GeometryError if the tool
    does not fit.
    """
    if side == "on":
        return [list(ring)]
    polygon = Polygon(ring)
    chord_angle = max_segment_angle(tool_radius) / GEOS_JOIN_CHORD_FACTOR
    quad_segs = max(1, math.ceil((math.pi / 2) / chord_angle))
    signed = tool_radius if side == "outside" else -tool_radius
    result = polygon.buffer(signed, quad_segs=quad_segs, join_style="round")
    polygons = _polygons(result)
    if not polygons:
        raise GeometryError(
            f"Tool (diameter {2 * tool_radius:.3f} mm) does not fit inside the contour"
        )
    rings = [_clean_ring(list(p.exterior.coords)) for p in polygons]
    return sorted(rings, key=lambda r: _start_key(r))


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


def _polygons(geometry: BaseGeometry) -> list[Polygon]:
    if geometry.is_empty:
        return []
    if isinstance(geometry, Polygon):
        return [geometry]
    if isinstance(geometry, MultiPolygon):
        return [p for p in geometry.geoms if not p.is_empty]
    raise GeometryError(f"Offset produced unexpected geometry: {geometry.geom_type}")


def _clean_ring(coords: list[tuple[float, ...]]) -> list[Point]:
    """Drop the repeated closing point and consecutive points closer than CHAIN_TOL_MM."""
    ring: list[Point] = []
    for c in coords[:-1]:
        p = (float(c[0]), float(c[1]))
        if not ring or distance(ring[-1], p) >= CHAIN_TOL_MM:
            ring.append(p)
    while len(ring) > 1 and distance(ring[-1], ring[0]) < CHAIN_TOL_MM:
        ring.pop()
    return ring


def _start_key(ring: list[Point]) -> tuple[float, float]:
    lowest = min(ring, key=lambda p: (p[1], p[0]))
    return (lowest[1], lowest[0])
