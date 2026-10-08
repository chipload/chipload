"""Match job selectors against imported contours. See spec section 3."""

from __future__ import annotations

from dataclasses import dataclass

from chipload.core.errors import SelectorError
from chipload.core.geometry.chain import Contour, Segment
from chipload.core.job.models import Selector


@dataclass(frozen=True, slots=True)
class Selection:
    contours: tuple[Contour, ...]
    notes: tuple[str, ...]  # open contours that matched and were skipped


def select_contours(contours: tuple[Contour, ...], selector: Selector, op: str) -> Selection:
    """Closed, valid contours whose every segment matches.

    Open contours that match (e.g. centrelines on a cutting layer) are skipped and listed in
    `notes`. Partly matched or invalid contours are refused, and so is a selection with no
    closed contour in it.
    """
    selected: list[Contour] = []
    skipped: list[Contour] = []
    for contour in contours:
        matches = [_matches(part.segment, selector) for part in contour.parts]
        if not any(matches):
            continue
        if not all(matches):
            raise SelectorError(
                f"{op}: contour #{contour.id} is only partly selected "
                f"(layers {', '.join(contour.layers)}); fix the drawing or the selector"
            )
        if not contour.closed:
            skipped.append(contour)
            continue
        if contour.invalid_reason is not None:
            raise SelectorError(f"{op}: contour #{contour.id} is invalid: {contour.invalid_reason}")
        selected.append(contour)
    if not selected:
        if skipped:
            raise SelectorError(
                f"{op}: selector {_describe(selector)} matches only open contours "
                f"({', '.join(f'#{c.id}' for c in skipped)}); this operation needs closed ones"
            )
        raise SelectorError(f"{op}: selector {_describe(selector)} matches nothing")
    notes = tuple(
        f"{op}: skipped open contour #{c.id} on {', '.join(c.layers)} "
        f"({c.start[0]:.3f}, {c.start[1]:.3f}) to ({c.end[0]:.3f}, {c.end[1]:.3f})"
        for c in skipped
    )
    return Selection(tuple(selected), notes)


def _matches(segment: Segment, selector: Selector) -> bool:
    # DXF layer names and handles are case-insensitive.
    if selector.layer is not None and segment.layer.casefold() != selector.layer.casefold():
        return False
    if selector.handles:
        wanted = {h.casefold() for h in selector.handles}
        if segment.handle.casefold() not in wanted:
            return False
    return True


def _describe(selector: Selector) -> str:
    parts = []
    if selector.layer is not None:
        parts.append(f"layer {selector.layer!r}")
    if selector.handles:
        parts.append(f"handles {', '.join(selector.handles)}")
    return " and ".join(parts)
