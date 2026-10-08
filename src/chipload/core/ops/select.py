"""Match job selectors against imported contours. See spec section 3."""

from __future__ import annotations

from chipload.core.errors import SelectorError
from chipload.core.geometry.chain import Contour, Segment
from chipload.core.job.models import Selector


def select_contours(contours: tuple[Contour, ...], selector: Selector, op: str) -> list[Contour]:
    """Closed, valid contours whose every segment matches. Anything ambiguous is refused."""
    selected: list[Contour] = []
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
            raise SelectorError(
                f"{op}: contour #{contour.id} is open; v0 can only profile closed contours"
            )
        if contour.invalid_reason is not None:
            raise SelectorError(f"{op}: contour #{contour.id} is invalid: {contour.invalid_reason}")
        selected.append(contour)
    if not selected:
        raise SelectorError(f"{op}: selector {_describe(selector)} matches nothing")
    return selected


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
