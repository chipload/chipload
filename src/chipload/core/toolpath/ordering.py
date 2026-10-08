"""Cut order within an operation. See docs/specs/v0-profile-pipeline.md section 4, "Order"."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from chipload.core.geometry.primitives import Point, distance


@dataclass(frozen=True, slots=True)
class PathRing:
    """An oriented tool-centre ring, starting at points[0]."""

    points: tuple[Point, ...]
    nesting_depth: int
    contour_id: int


def order_rings(rings: Sequence[PathRing], start: Point = (0.0, 0.0)) -> list[PathRing]:
    """Deepest nesting first (holes before the outline that contains them), then greedy
    nearest-neighbour from the previous ring's start. Ties break on start y, x, contour id."""
    ordered: list[PathRing] = []
    position = start
    for depth in sorted({r.nesting_depth for r in rings}, reverse=True):
        remaining = [r for r in rings if r.nesting_depth == depth]
        while remaining:
            here = position
            nearest = min(
                remaining,
                key=lambda r: (
                    distance(here, r.points[0]),
                    r.points[0][1],
                    r.points[0][0],
                    r.contour_id,
                ),
            )
            remaining.remove(nearest)
            ordered.append(nearest)
            position = nearest.points[0]
    return ordered
