"""Job model (declarative description of how to machine a drawing).

All lengths are mm and all feeds mm/min, whatever units the drawing uses. `depth` is measured
down from the stock top.

Schema history:
- v1: initial format.
- v2: `machine.spindle` (required), `plunge_feed` on profile and pocket ops (required),
  optional `machine.travel`, selectors must name a layer or handles.
"""

from __future__ import annotations

from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

CURRENT_VERSION = 2


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
    rpm: float | None = None


class Selector(_Frozen):
    layer: str | None = None
    handles: list[str] | None = None

    @model_validator(mode="after")
    def _selects_something(self) -> Selector:
        if self.layer is None and not self.handles:
            raise ValueError("selector needs a layer or at least one handle")
        return self


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


class PocketOp(_Frozen):
    type: Literal["pocket"] = "pocket"
    name: str = ""
    select: Selector
    tool: int
    depth: float = Field(gt=0)
    step_down: float = Field(gt=0)
    stepover: float = Field(gt=0, le=1)
    feed: float = Field(gt=0)
    plunge_feed: float = Field(gt=0)


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
