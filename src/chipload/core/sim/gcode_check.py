"""Independent G-code checker.

Parses G-code *text* with its own tokenizer and modal state and checks it against safety rules
(docs/specs/v0-profile-pipeline.md section 6). It must never import or reuse the toolpath
generator or post-processor: its value is that it can disagree with them.
"""

from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from typing import Literal

from chipload.core.tolerances import ARC_APPROX_TOL_MM, CHAIN_TOL_MM

# Positional slack for comparisons against heights and bounds.
POSITION_TOL_MM = CHAIN_TOL_MM

ALLOWED_G = frozenset({0, 1, 2, 3, 4, 17, 20, 21, 90, 91, 94})
ALLOWED_M = frozenset({3, 4, 5, 30})
MOTION_G = frozenset({0, 1, 2, 3})
VALUE_LETTERS = frozenset("XYZIJFSPN")

_WORD = re.compile(r"\s*([A-Za-z])\s*([+-]?(?:\d+\.?\d*|\.\d+))")
_COMMENT = re.compile(r"\([^()]*\)")


@dataclass(frozen=True, slots=True)
class CheckLimits:
    """Everything the checker knows about the job. Heights in work coordinates, mm."""

    safe_z: float
    stock_top_z: float
    stock_bottom_z: float
    stock_min: tuple[float, float]
    stock_max: tuple[float, float]
    tool_radius: float
    travel: tuple[float, float, float] | None = None  # machine travel span per axis


@dataclass(frozen=True, slots=True)
class CheckIssue:
    line: int  # 1-based; 0 for whole-program issues
    message: str


@dataclass(frozen=True, slots=True)
class CheckReport:
    errors: tuple[CheckIssue, ...]
    warnings: tuple[CheckIssue, ...]

    @property
    def ok(self) -> bool:
        return not self.errors


@dataclass
class _State:
    units_mm: bool | None = None
    absolute: bool | None = None
    motion: int | None = None
    feed: float | None = None
    spindle: Literal["off", "cw", "ccw"] = "off"
    pos: dict[str, float | None] = field(default_factory=lambda: {"X": None, "Y": None, "Z": None})
    lo: dict[str, float] = field(default_factory=dict)
    hi: dict[str, float] = field(default_factory=dict)
    outside_stock: int = 0
    ended: bool = False


def check_gcode(text: str, limits: CheckLimits) -> CheckReport:
    errors: list[CheckIssue] = []
    warnings: list[CheckIssue] = []
    state = _State()
    for number, raw in enumerate(text.splitlines(), start=1):
        words, problem = _parse_line(raw)
        if problem is not None:
            errors.append(CheckIssue(number, problem))
            continue
        if words:
            if state.ended:
                errors.append(CheckIssue(number, "Code after program end (M30)"))
            for message in _execute(words, state, limits):
                errors.append(CheckIssue(number, message))

    if state.outside_stock:
        warnings.append(
            CheckIssue(
                0,
                f"{state.outside_stock} cutting moves end outside the stock "
                "(expanded by the tool radius)",
            )
        )
    if limits.travel is None:
        warnings.append(CheckIssue(0, "Machine travel not set; travel limits were not checked"))
    else:
        for axis, span_limit in zip("XYZ", limits.travel, strict=True):
            if axis in state.lo:
                span = state.hi[axis] - state.lo[axis]
                if span > span_limit + POSITION_TOL_MM:
                    errors.append(
                        CheckIssue(
                            0, f"{axis} span {span:.3f} mm exceeds machine travel {span_limit:.3f}"
                        )
                    )
    if not state.ended:
        warnings.append(CheckIssue(0, "No program end (M30)"))
    return CheckReport(tuple(errors), tuple(warnings))


def _parse_line(raw: str) -> tuple[list[tuple[str, float]], str | None]:
    line = raw.split(";", 1)[0]
    line = _COMMENT.sub(" ", line)
    if "(" in line or ")" in line:
        return [], "Unbalanced or nested comment"
    words: list[tuple[str, float]] = []
    pos = 0
    while pos < len(line):
        if line[pos].isspace():
            pos += 1
            continue
        match = _WORD.match(line, pos)
        if match is None:
            return [], f"Cannot parse {line[pos:].strip()!r}"
        words.append((match.group(1).upper(), float(match.group(2))))
        pos = match.end()
    return words, None


def _execute(words: list[tuple[str, float]], state: _State, limits: CheckLimits) -> list[str]:
    problems: list[str] = []
    g_codes: list[int] = []
    m_codes: list[int] = []
    values: dict[str, float] = {}
    for letter, value in words:
        if letter in ("G", "M"):
            if value != int(value):
                return [f"{letter}{value} is not supported"]
            (g_codes if letter == "G" else m_codes).append(int(value))
        elif letter in VALUE_LETTERS:
            if letter in values:
                return [f"{letter} appears twice on one line"]
            values[letter] = value
        else:
            return [f"Unsupported word {letter}"]

    for g in g_codes:
        if g not in ALLOWED_G:
            problems.append(f"G{g} is not supported")
    for m in m_codes:
        if m not in ALLOWED_M:
            problems.append(f"M{m} is not supported")
    motions = [g for g in g_codes if g in MOTION_G]
    if len(motions) > 1:
        problems.append("More than one motion command on one line")
    if problems:
        return problems

    # Modal settings first, in the order a controller applies them.
    if "F" in values:
        if values["F"] <= 0:
            problems.append(f"Feed F{values['F']} must be positive")
        state.feed = values["F"]
    if 20 in g_codes:
        problems.append("G20 (inch) used; programs must be in mm")
        state.units_mm = False
    if 21 in g_codes:
        state.units_mm = True
    if 91 in g_codes:
        problems.append("G91 (relative) is not supported")
        state.absolute = False
    if 90 in g_codes:
        state.absolute = True
    if 3 in m_codes:
        state.spindle = "cw"
    if 4 in m_codes:
        state.spindle = "ccw"
    if 4 in g_codes and "P" not in values:
        problems.append("G4 dwell without P")
    if motions:
        state.motion = motions[0]

    axes = {a: values[a] for a in "XYZ" if a in values}
    if axes or (motions and motions[0] in (2, 3)):
        if state.motion is None:
            problems.append("Axis words without a motion mode")
        else:
            problems.extend(_move(state.motion, axes, values, state, limits))

    if 5 in m_codes:
        state.spindle = "off"
    if 30 in m_codes:
        state.ended = True
    return problems


def _move(
    motion: int,
    axes: dict[str, float],
    values: dict[str, float],
    state: _State,
    limits: CheckLimits,
) -> list[str]:
    problems: list[str] = []
    if state.units_mm is not True:
        problems.append("Motion before G21 (mm)")
    if state.absolute is not True:
        problems.append("Motion before G90 (absolute)")
    start = dict(state.pos)
    target = {a: axes.get(a, start[a]) for a in "XYZ"}
    xy_changes = any(a in axes and axes[a] != start[a] for a in "XY")

    if motion == 0:
        z_now = start["Z"]
        if xy_changes and (z_now is None or z_now < limits.safe_z - POSITION_TOL_MM):
            problems.append("Rapid XY move while Z is below safe height (retract first)")
        target_z = target["Z"]
        if target_z is not None and target_z < limits.stock_top_z - POSITION_TOL_MM:
            problems.append(f"Rapid to Z{target_z:.3f}, below the stock top (rapid into material)")
    else:
        if state.feed is None:
            problems.append("Cutting move with no feed set")
        if state.spindle == "off":
            problems.append("Cutting move with the spindle off")
        if any(v is None for v in target.values()):
            problems.append("Cutting move from an unknown position")
        if motion in (2, 3):
            problems.extend(_check_arc(start, target, values))
        x, y = target["X"], target["Y"]
        if x is not None and y is not None and _outside_stock(x, y, limits):
            state.outside_stock += 1

    z = target["Z"]
    if z is not None and z < limits.stock_bottom_z - POSITION_TOL_MM:
        problems.append(f"Z{z:.3f} is below the stock bottom ({limits.stock_bottom_z:.3f})")

    state.pos = target
    for axis, v in target.items():
        if v is not None:
            state.lo[axis] = min(state.lo.get(axis, v), v)
            state.hi[axis] = max(state.hi.get(axis, v), v)
    return problems


def _check_arc(
    start: dict[str, float | None], target: dict[str, float | None], values: dict[str, float]
) -> list[str]:
    if "I" not in values and "J" not in values:
        return ["Arc without I/J centre offsets"]
    sx, sy, ex, ey = start["X"], start["Y"], target["X"], target["Y"]
    if sx is None or sy is None or ex is None or ey is None:
        return ["Arc from an unknown position"]
    cx = sx + values.get("I", 0.0)
    cy = sy + values.get("J", 0.0)
    r_start = math.hypot(sx - cx, sy - cy)
    r_end = math.hypot(ex - cx, ey - cy)
    if abs(r_start - r_end) > ARC_APPROX_TOL_MM:
        return [f"Arc radius mismatch: start {r_start:.4f} vs end {r_end:.4f}"]
    return []


def _outside_stock(x: float, y: float, limits: CheckLimits) -> bool:
    margin = limits.tool_radius + POSITION_TOL_MM
    return not (
        limits.stock_min[0] - margin <= x <= limits.stock_max[0] + margin
        and limits.stock_min[1] - margin <= y <= limits.stock_max[1] + margin
    )
