"""Job model (declarative description of how to machine a drawing).

All lengths are mm and all feeds mm/min, whatever units the drawing uses. `depth` is measured
down from the stock top.

Schema history:
- v1: initial format.
- v2: `machine.spindle` (required), `plunge_feed` on profile and pocket ops (required),
  optional `machine.travel`, selectors must name a layer or handles.
- v3: `direction` on pocket ops (required), optional `keep_tool_down` on pocket ops and
  `center_cutting` on tools.
"""

from __future__ import annotations

from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

CURRENT_VERSION = 3


class _Frozen(BaseModel):
    model_config = ConfigDict(frozen=True, extra="forbid")


class Source(_Frozen):
    file: str
    sha256: str | None = None
    units: Literal["mm", "inch"]


class Stock(_Frozen):
    x: float = Field(gt=0)
    y: float = Field(gt=0)
    thickness: float = Field(gt=0)
    origin: Literal["bottom_left", "center"] = "bottom_left"
    z_zero: Literal["top", "bottom"] = "top"

    def xy_bounds(self) -> tuple[tuple[float, float], tuple[float, float]]:
        """(min, max) corners of the stock in work coordinates; work zero is the DXF origin."""
        if self.origin == "bottom_left":
            return (0.0, 0.0), (self.x, self.y)
        return (-self.x / 2, -self.y / 2), (self.x / 2, self.y / 2)


class Travel(_Frozen):
    """Usable machine travel per axis, mm."""

    x: float = Field(gt=0)
    y: float = Field(gt=0)
    z: float = Field(gt=0)


class Machine(_Frozen):
    post: str
    safe_z: float
    spindle: Literal["cw", "ccw"]  # emitted as M3 / M4
    max_feed: float | None = None
    travel: Travel | None = None


class Tool(_Frozen):
    id: int
    type: Literal["endmill", "drill", "vbit"] = "endmill"
    diameter: float = Field(gt=0)
    flutes: int | None = None
    center_cutting: bool | None = None  # None = unknown; pockets plunge straight down
    rpm: float | None = None


class Selector(_Frozen):
    layer: str | None = None
    handles: list[str] | None = None

    @model_validator(mode="after")
    def _selects_something(self) -> Selector:
        if self.layer is None and not self.handles:
            raise ValueError("selector needs a layer or at least one handle")
        return self


class Tabs(_Frozen):
    """Holding tabs on a profile (docs/specs/tabs.md). Optional, so no schema bump."""

    count: int = Field(ge=1)
    width: float = Field(gt=0)  # bridge left on the part, mm (the tool lifts for width + D)
    height: float = Field(gt=0)  # bridge thickness up from the bottom of the cut, mm


class ProfileOp(_Frozen):
    type: Literal["profile"] = "profile"
    name: str = ""
    select: Selector
    side: Literal["inside", "outside", "on"]
    direction: Literal["climb", "conventional"]
    tool: int
    depth: float = Field(gt=0)
    step_down: float = Field(gt=0)
    feed: float = Field(gt=0)
    plunge_feed: float = Field(gt=0)
    tabs: Tabs | None = None


class PocketOp(_Frozen):
    type: Literal["pocket"] = "pocket"
    name: str = ""
    select: Selector
    tool: int
    depth: float = Field(gt=0)
    step_down: float = Field(gt=0)
    stepover: float = Field(gt=0, le=1)  # fraction of the tool diameter
    direction: Literal["climb", "conventional"]
    feed: float = Field(gt=0)
    plunge_feed: float = Field(gt=0)
    keep_tool_down: bool = False  # stay at depth between rings where that is safe


class DrillOp(_Frozen):
    type: Literal["drill"] = "drill"
    name: str = ""
    select: Selector
    tool: int
    peck: float | None = None


Operation = Annotated[ProfileOp | PocketOp | DrillOp, Field(discriminator="type")]


class Job(_Frozen):
    schema_version: int = CURRENT_VERSION
    written_by: str | None = None
    source: Source
    stock: Stock
    machine: Machine
    tools: list[Tool]
    operations: list[Operation]
