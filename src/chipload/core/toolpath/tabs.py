"""Holding tabs on profile rings. See docs/specs/tabs.md."""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass

from chipload.core.errors import GeometryError
from chipload.core.geometry.primitives import Point, distance
from chipload.core.tolerances import CHAIN_TOL_MM, DEPTH_TOL_MM
from chipload.core.toolpath.model import Move

TAB_MAX_TURN_RAD = math.radians(15.0)  # a tab gap turning more than this is "on a corner"
TAB_SLIDE_STEP_MM = 0.25  # resolution when sliding a tab off a corner


@dataclass(frozen=True, slots=True)
class TabLayout:
    spans: tuple[tuple[float, float], ...]  # (start, end) arc length of each lift on the ring
    length: float  # ring length
    on_curves: int  # tabs that could not be moved off a corner or curve


def ring_length(ring: Sequence[Point]) -> float:
    return sum(distance(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring)))


def place_tabs(ring: Sequence[Point], count: int, width: float, tool_diameter: float) -> TabLayout:
    """Equal spacing, then each tab slides (within its slot, at most a quarter of the spacing)
    to the nearest straight-enough stretch. Raises GeometryError if the tabs don't fit."""
    length = ring_length(ring)
    gap = width + tool_diameter
    spacing = length / count
    if spacing - gap < tool_diameter:
        fits = math.floor(length / (gap + tool_diameter))
        most = f"at most {fits} fit" if fits else "not even one fits"
        raise GeometryError(
            f"{count} tabs of {width:.3f} mm don't fit on a {length:.1f} mm path ({most})"
        )
    positions, turns = _vertex_turns(ring)
    spans: list[tuple[float, float]] = []
    on_curves = 0
    for i in range(count):
        nominal = (i + 0.5) * spacing
        lowest = max(i * spacing + tool_diameter / 2 + gap / 2, nominal - spacing / 4)
        highest = min((i + 1) * spacing - tool_diameter / 2 - gap / 2, nominal + spacing / 4)
        centre = _straightest_centre(nominal, lowest, highest, gap, positions, turns)
        if centre is None:
            centre = nominal
            on_curves += 1
        spans.append((centre - gap / 2, centre + gap / 2))
    return TabLayout(tuple(spans), length, on_curves)


def split_ring(
    ring: Sequence[Point], spans: Sequence[tuple[float, float]]
) -> list[tuple[Point, bool]]:
    """The closed ring from ring[0] back to ring[0], with points inserted at every span boundary.
    Each entry is (end point of a segment, whether that segment lies inside a tab)."""
    cuts = sorted(b for span in spans for b in span)
    result: list[tuple[Point, bool]] = []
    travelled = 0.0
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        seg = distance(a, b)
        inner = [c for c in cuts if travelled + CHAIN_TOL_MM < c < travelled + seg - CHAIN_TOL_MM]
        stops = [*((c - travelled) / seg for c in inner), 1.0]
        previous = 0.0
        for t in stops:
            mid = travelled + seg * (previous + t) / 2
            point = b if t == 1.0 else (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
            result.append((point, any(s0 < mid < s1 for s0, s1 in spans)))
            previous = t
        travelled += seg
    return result


def tabbed_contour_moves(
    ring: Sequence[Point],
    layout: TabLayout,
    pass_z: list[float],
    z_tab: float,
    safe_z: float,
    feed: float,
    plunge_feed: float,
) -> list[Move]:
    """Like contour_moves, but passes below the tab top lift over every tab: up at plunge feed,
    across at feed, back down at plunge feed."""
    start = ring[0]
    path = split_ring(ring, layout.spans)
    moves = [Move("rapid", z=safe_z), Move("rapid", x=start[0], y=start[1])]
    for z in pass_z:
        moves.append(Move("feed", z=z, feed=plunge_feed))
        if z >= z_tab - DEPTH_TOL_MM:
            moves.extend(Move("feed", x=x, y=y, feed=feed) for x, y in [*ring[1:], start])
            continue
        lifted = False
        for (x, y), in_tab in path:
            if in_tab != lifted:
                moves.append(Move("feed", z=z_tab if in_tab else z, feed=plunge_feed))
                lifted = in_tab
            moves.append(Move("feed", x=x, y=y, feed=feed))
        if lifted:
            moves.append(Move("feed", z=z, feed=plunge_feed))
    moves.append(Move("rapid", z=safe_z))
    _verify(moves, layout, start, z_tab)
    return moves


def tab_polylines(ring: Sequence[Point], layout: TabLayout) -> list[tuple[Point, ...]]:
    """The tool-path stretch of each tab, for display."""
    path = split_ring(ring, layout.spans)
    lines: list[tuple[Point, ...]] = []
    current: list[Point] = []
    previous = ring[0]
    for point, in_tab in path:
        if in_tab:
            current = current or [previous]
            current.append(point)
        elif current:
            lines.append(tuple(current))
            current = []
        previous = point
    if current:
        lines.append(tuple(current))
    return lines


def _vertex_turns(ring: Sequence[Point]) -> tuple[list[float], list[float]]:
    """Arc-length position and absolute turning angle of every vertex."""
    positions: list[float] = []
    turns: list[float] = []
    travelled = 0.0
    n = len(ring)
    for i in range(n):
        prev, here, nxt = ring[i - 1], ring[i], ring[(i + 1) % n]
        a_in = math.atan2(here[1] - prev[1], here[0] - prev[0])
        a_out = math.atan2(nxt[1] - here[1], nxt[0] - here[0])
        turn = abs(math.remainder(a_out - a_in, 2 * math.pi))
        positions.append(travelled)
        turns.append(turn)
        travelled += distance(here, nxt)
    return positions, turns


def _straightest_centre(
    nominal: float,
    lowest: float,
    highest: float,
    gap: float,
    positions: list[float],
    turns: list[float],
) -> float | None:
    """Closest centre to `nominal` (forward first on ties) whose gap turns < TAB_MAX_TURN_RAD."""
    steps = int((max(highest - nominal, nominal - lowest)) / TAB_SLIDE_STEP_MM)
    for k in range(steps + 1):
        for centre in (nominal + k * TAB_SLIDE_STEP_MM, nominal - k * TAB_SLIDE_STEP_MM):
            if not lowest <= centre <= highest:
                continue
            s0, s1 = centre - gap / 2, centre + gap / 2
            turning = sum(t for s, t in zip(positions, turns, strict=True) if s0 < s < s1)
            if turning < TAB_MAX_TURN_RAD:
                return centre
            if k == 0:
                break  # nominal: don't test the same centre twice
    return None


def _verify(moves: list[Move], layout: TabLayout, start: Point, z_tab: float) -> None:
    """Post-condition: on every pass below the tab top, the length cut low equals the ring
    length minus the tab gaps, and the lifted length equals the tab gaps."""
    gaps = sum(s1 - s0 for s0, s1 in layout.spans)
    x, y = start
    z: float | None = None
    low: dict[float, float] = {}
    lifted: dict[float, float] = {}
    pass_z: float | None = None
    for move in moves:
        nx = x if move.x is None else move.x
        ny = y if move.y is None else move.y
        if move.z is not None:
            z = move.z
            if move.kind == "feed" and z < z_tab - DEPTH_TOL_MM:
                pass_z = z
        step = distance((x, y), (nx, ny))
        if move.kind == "feed" and pass_z is not None and z is not None and step > 0:
            bucket = lifted if z >= z_tab - DEPTH_TOL_MM else low
            bucket[pass_z] = bucket.get(pass_z, 0.0) + step
        x, y = nx, ny
    slack = CHAIN_TOL_MM * (len(layout.spans) + 1)
    for depth, cut in low.items():
        if abs(cut - (layout.length - gaps)) > slack or abs(lifted.get(depth, 0.0) - gaps) > slack:
            raise GeometryError(
                f"Tab toolpath failed verification at Z{depth:.3f}: cut {cut:.3f} mm low, "
                f"expected {layout.length - gaps:.3f}"
            )
