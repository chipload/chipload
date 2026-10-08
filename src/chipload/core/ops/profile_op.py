"""Turn a ProfileOp into ordered, oriented tool-centre rings."""

from __future__ import annotations

from dataclasses import dataclass

from chipload.core.errors import GeometryError
from chipload.core.geometry.chain import Contour
from chipload.core.geometry.primitives import Point
from chipload.core.job.models import ProfileOp
from chipload.core.toolpath.model import Spindle
from chipload.core.toolpath.ordering import PathRing, order_rings
from chipload.core.toolpath.profile import cut_clockwise, offset_rings, orient


@dataclass(frozen=True, slots=True)
class ProfilePlan:
    rings: tuple[PathRing, ...]
    notes: tuple[str, ...]  # warnings for the user, e.g. material left in a narrow neck


def plan_profile(
    op: ProfileOp,
    label: str,
    contours: list[Contour],
    tool_radius: float,
    spindle: Spindle,
    start: Point,
) -> ProfilePlan:
    clockwise = cut_clockwise(op.side, op.direction, spindle)
    rings: list[PathRing] = []
    notes: list[str] = []
    for contour in contours:
        try:
            paths = offset_rings(contour.points(), op.side, tool_radius)
        except GeometryError as exc:
            raise GeometryError(f"{label}, contour #{contour.id}: {exc}") from exc
        if len(paths) > 1:
            notes.append(
                f"{label}, contour #{contour.id}: tool does not fit through a narrow part; "
                f"cut as {len(paths)} separate loops and material remains in between"
            )
        depth = contour.nesting_depth if contour.nesting_depth is not None else 0
        rings.extend(PathRing(tuple(orient(p, clockwise)), depth, contour.id) for p in paths)
    return ProfilePlan(tuple(order_rings(rings, start)), tuple(notes))
