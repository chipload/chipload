"""Machine-independent toolpath: what the post-processor turns into G-code. Units are mm."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

Spindle = Literal["cw", "ccw"]


@dataclass(frozen=True, slots=True)
class Move:
    """A straight move. Axes left as None keep their current value."""

    kind: Literal["rapid", "feed"]
    x: float | None = None
    y: float | None = None
    z: float | None = None
    feed: float | None = None  # mm/min; required for feed moves, None for rapids


@dataclass(frozen=True, slots=True)
class Program:
    tool_id: int
    tool_diameter: float
    rpm: float
    spindle: Spindle
    safe_z: float
    moves: tuple[Move, ...]
