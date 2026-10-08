"""Safe polygon offsetting shared by all operations (see ADR 0002)."""

from __future__ import annotations

import math
import warnings

import shapely
from shapely.geometry import LineString, MultiPolygon, Polygon
from shapely.geometry.base import BaseGeometry

from chipload.core.errors import GeometryError
from chipload.core.geometry.primitives import Point, distance, max_segment_angle
from chipload.core.tolerances import CHAIN_TOL_MM

# GEOS rounds the number of chords in a round join to the nearest integer, so one chord can
# span up to 1.5x the nominal angle (pi/2 / quad_segs). Measured with Shapely 2.2 / GEOS;
# tests/property/test_profile_offset.py guards it.
GEOS_JOIN_CHORD_FACTOR = 1.5


def quad_segs_for(radius: float) -> int:
    """Segments per quarter circle so round joins of this radius stay within tolerance."""
    chord_angle = max_segment_angle(radius) / GEOS_JOIN_CHORD_FACTOR
    return max(1, math.ceil((math.pi / 2) / chord_angle))


def buffer_polygons(geometry: BaseGeometry, signed_distance: float) -> list[Polygon]:
    """Round-join buffer (positive grows, negative shrinks), as a list of non-empty polygons
    sorted by (min y, min x). GEOS numeric warnings become GeometryError."""
    with warnings.catch_warnings():
        warnings.simplefilter("error", RuntimeWarning)
        try:
            result = geometry.buffer(
                signed_distance, quad_segs=quad_segs_for(abs(signed_distance)), join_style="round"
            )
        except RuntimeWarning as exc:
            raise GeometryError(f"Offset failed numerically: {exc}") from exc
    return sorted(polygons_of(result), key=lambda p: (p.bounds[1], p.bounds[0]))


def polygons_of(geometry: BaseGeometry) -> list[Polygon]:
    if geometry.is_empty:
        return []
    if isinstance(geometry, Polygon):
        return [geometry]
    if isinstance(geometry, MultiPolygon):
        return [p for p in geometry.geoms if not p.is_empty]
    raise GeometryError(f"Offset produced unexpected geometry: {geometry.geom_type}")


def dedupe_ring(points: list[Point]) -> list[Point]:
    """Drop consecutive points closer than CHAIN_TOL_MM, including a repeated closing point.

    Near-zero edges make GEOS misjudge corner directions, so this runs before and after buffering.
    """
    ring: list[Point] = []
    for p in points:
        if not ring or distance(ring[-1], p) >= CHAIN_TOL_MM:
            ring.append(p)
    while len(ring) > 1 and distance(ring[-1], ring[0]) < CHAIN_TOL_MM:
        ring.pop()
    return ring


def ring_points(coords: object) -> list[Point]:
    """Shapely ring coordinates as a deduplicated, implicitly closed list of points."""
    return dedupe_ring([(float(c[0]), float(c[1])) for c in coords])  # type: ignore[attr-defined]


def erode_polygons(geometry: BaseGeometry, distance_mm: float) -> list[Polygon]:
    """Shrink by `distance_mm`: everything farther than that from the boundary.

    Computed as the geometry minus the union of round-capped buffers of every boundary
    segment, not with a negative buffer. GEOS's negative buffer has a "ring is eroded
    completely" shortcut that misfires near collapse: for a convex polygon with an inscribed
    radius of 22.44 mm, buffer(-21) returned empty (Shapely 2.2 / GEOS 3.14). The capsule union
    gave no wrong result in 10 000 random cases where buffer(-d) gave 40.
    """
    if distance_mm <= 0:
        raise GeometryError(f"erode distance must be positive, got {distance_mm}")
    return _sorted_polygons(geometry.difference(_boundary_band(geometry, distance_mm)))


def dilate_polygons(geometry: BaseGeometry, distance_mm: float) -> list[Polygon]:
    """Grow by `distance_mm`, the same way (holes shrink but are never dropped wrongly)."""
    if distance_mm <= 0:
        raise GeometryError(f"dilate distance must be positive, got {distance_mm}")
    return _sorted_polygons(geometry.union(_boundary_band(geometry, distance_mm)))


def _boundary_band(geometry: BaseGeometry, distance_mm: float) -> BaseGeometry:
    segments = []
    for polygon in polygons_of(geometry):
        for ring in (polygon.exterior, *polygon.interiors):
            coords = list(ring.coords)
            segments += [LineString(coords[i : i + 2]) for i in range(len(coords) - 1)]
    with warnings.catch_warnings():
        warnings.simplefilter("error", RuntimeWarning)
        try:
            capsules = shapely.buffer(segments, distance_mm, quad_segs=quad_segs_for(distance_mm))
            return shapely.union_all(capsules)
        except RuntimeWarning as exc:
            raise GeometryError(f"Offset failed numerically: {exc}") from exc


def _sorted_polygons(geometry: BaseGeometry) -> list[Polygon]:
    parts = [
        p
        for p in shapely.get_parts(geometry)
        if isinstance(p, Polygon) and p.area > CHAIN_TOL_MM * CHAIN_TOL_MM
    ]
    return sorted(parts, key=lambda p: (p.bounds[1], p.bounds[0]))
