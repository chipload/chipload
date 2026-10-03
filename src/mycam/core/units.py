"""Unit handling. Internal geometry is always millimetres."""
from __future__ import annotations

from enum import Enum


class Units(str, Enum):
    MM = "mm"
    INCH = "inch"


_TO_MM = {Units.MM: 1.0, Units.INCH: 25.4}


def to_mm(value: float, units: Units) -> float:
    return value * _TO_MM[units]


def from_mm(value: float, units: Units) -> float:
    return value / _TO_MM[units]
