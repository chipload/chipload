"""2D primitives in millimetres. Angles in radians, counter-clockwise positive."""

from __future__ import annotations

import math
from dataclasses import dataclass

from chipload.core.tolerances import ARC_APPROX_TOL_MM

Point = tuple[float, float]

# Never approximate a full circle with fewer segments than this, however small.
MIN_SEGMENTS_PER_CIRCLE = 4


@dataclass(frozen=True, slots=True)
class Line:
    start: Point
    end: Point

    def length(self) -> float:
        return distance(self.start, self.end)

    def midpoint(self) -> Point:
        return ((self.start[0] + self.end[0]) / 2, (self.start[1] + self.end[1]) / 2)

    def points(self) -> list[Point]:
        return [self.start, self.end]


@dataclass(frozen=True, slots=True)
class Arc:
    """Circular arc from `start_angle`, sweeping `sweep` radians (positive = CCW, negative = CW)."""

    center: Point
    radius: float
    start_angle: float
    sweep: float

    @property
    def start(self) -> Point:
        return self.point_at(self.start_angle)

    @property
    def end(self) -> Point:
        return self.point_at(self.start_angle + self.sweep)

    def point_at(self, angle: float) -> Point:
        return (
            self.center[0] + self.radius * math.cos(angle),
            self.center[1] + self.radius * math.sin(angle),
        )

    def length(self) -> float:
        return abs(self.sweep) * self.radius

    def midpoint(self) -> Point:
        return self.point_at(self.start_angle + self.sweep / 2)

    def points(self) -> list[Point]:
        """Chord approximation; every chord is within ARC_APPROX_TOL_MM of the arc."""
        n = segment_count(self.radius, abs(self.sweep))
        step = self.sweep / n
        pts = [self.point_at(self.start_angle + i * step) for i in range(n)]
        pts.append(self.end)
        return pts


Primitive = Line | Arc


def distance(a: Point, b: Point) -> float:
    return math.hypot(a[0] - b[0], a[1] - b[1])


def max_segment_angle(radius: float, tol: float = ARC_APPROX_TOL_MM) -> float:
    """Largest chord angle whose sagitta r*(1 - cos(theta/2)) stays within `tol`."""
    if radius <= tol:
        return math.pi
    return 2.0 * math.acos(1.0 - tol / radius)


def segment_count(radius: float, sweep: float) -> int:
    by_tolerance = math.ceil(sweep / max_segment_angle(radius))
    by_minimum = math.ceil(sweep / (2.0 * math.pi / MIN_SEGMENTS_PER_CIRCLE))
    return max(1, by_tolerance, by_minimum)


def signed_area(ring: list[Point]) -> float:
    """Shoelace area of an implicitly closed ring. Positive = counter-clockwise."""
    total = 0.0
    for i, (x1, y1) in enumerate(ring):
        x2, y2 = ring[(i + 1) % len(ring)]
        total += x1 * y2 - x2 * y1
    return total / 2.0
