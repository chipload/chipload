import math
from itertools import pairwise

import pytest

from chipload.core.errors import GeometryError
from chipload.core.toolpath.profile import offset_rings, orient
from chipload.core.toolpath.tabs import (
    TAB_MAX_TURN_RAD,
    place_tabs,
    ring_length,
    split_ring,
    tab_polylines,
    tabbed_contour_moves,
)

RECT = [(0.0, 0.0), (100.0, 0.0), (100.0, 60.0), (0.0, 60.0)]
D = 6.0


def rect_path():
    (ring,) = offset_rings(RECT, "outside", D / 2)
    return orient(ring, True)


def circle_path(r=15.0, n=90):
    ring = [
        (r * math.cos(2 * math.pi * i / n), r * math.sin(2 * math.pi * i / n)) for i in range(n)
    ]
    return orient(ring, True)


def test_gap_on_the_path_is_width_plus_tool_diameter():
    layout = place_tabs(rect_path(), 4, 8.0, D)
    assert all(s1 - s0 == pytest.approx(8.0 + D) for s0, s1 in layout.spans)


def test_tabs_on_straight_edges_stay_at_nominal_positions():
    ring = rect_path()
    layout = place_tabs(ring, 4, 8.0, D)
    spacing = ring_length(ring) / 4
    centres = [(s0 + s1) / 2 for s0, s1 in layout.spans]
    assert centres == pytest.approx([(i + 0.5) * spacing for i in range(4)])
    assert layout.on_curves == 0


def test_tabs_slide_off_corners_within_their_slot():
    ring = rect_path()
    layout = place_tabs(ring, 8, 8.0, D)
    spacing = ring_length(ring) / 8
    centres = [(s0 + s1) / 2 for s0, s1 in layout.spans]
    nominal = [(i + 0.5) * spacing for i in range(8)]
    moved = [i for i, (c, n) in enumerate(zip(centres, nominal, strict=True)) if abs(c - n) > 1e-9]
    assert moved  # two nominal centres sit on the rounded corners of this rectangle
    assert layout.on_curves == 0
    for i, (s0, s1) in enumerate(layout.spans):
        assert i * spacing + D / 2 <= s0 + 1e-9
        assert s1 <= (i + 1) * spacing - D / 2 + 1e-9
        assert abs((s0 + s1) / 2 - nominal[i]) <= spacing / 4 + 1e-9


def test_slid_tabs_avoid_turning():
    ring = rect_path()
    layout = place_tabs(ring, 8, 8.0, D)
    for line in tab_polylines(ring, layout):
        headings = [math.atan2(b[1] - a[1], b[0] - a[0]) for a, b in pairwise(line)]
        turning = sum(abs(math.remainder(b - a, 2 * math.pi)) for a, b in pairwise(headings))
        assert turning < TAB_MAX_TURN_RAD + 1e-9  # placement sums the same angles in float


def test_round_part_keeps_nominal_tabs_and_reports_them():
    layout = place_tabs(circle_path(), 3, 5.0, D)
    assert layout.on_curves == 3


def test_too_many_tabs_is_an_error_naming_what_fits():
    with pytest.raises(GeometryError, match="at most 16 fit"):
        place_tabs(rect_path(), 20, 8.0, D)


def test_no_tab_fits_on_a_tiny_path():
    with pytest.raises(GeometryError, match="not even one fits"):
        place_tabs(circle_path(r=2.0), 1, 8.0, D)


def test_one_tab_that_just_fits():
    ring = circle_path(r=10.0)
    length = ring_length(ring)
    width = length - 2 * D  # spacing - gap == D exactly
    assert len(place_tabs(ring, 1, width - 1e-6, D).spans) == 1


def test_tool_wider_than_the_tab_is_fine():
    layout = place_tabs(rect_path(), 4, 2.0, 12.0)
    assert all(s1 - s0 == pytest.approx(14.0) for s0, s1 in layout.spans)


def test_start_point_is_never_inside_a_tab():
    ring = rect_path()
    layout = place_tabs(ring, 8, 8.0, D)
    assert all(s0 > 0 and s1 < layout.length for s0, s1 in layout.spans)


def test_split_inserts_exact_boundaries():
    ring = rect_path()
    layout = place_tabs(ring, 4, 8.0, D)
    path = split_ring(ring, layout.spans)
    in_tab = 0.0
    previous = ring[0]
    for point, flag in path:
        if flag:
            in_tab += math.dist(previous, point)
        previous = point
    assert in_tab == pytest.approx(4 * (8.0 + D))
    assert path[-1][0] == ring[0]


def z_levels(moves):
    return sorted({m.z for m in moves if m.z is not None})


def test_only_passes_below_the_tab_top_lift():
    ring = rect_path()
    layout = place_tabs(ring, 4, 8.0, D)
    moves = tabbed_contour_moves(ring, layout, [-6.0, -12.0, -18.0], -15.0, 5.0, 1800, 600)
    assert z_levels(moves) == [-18.0, -15.0, -12.0, -6.0, 5.0]
    lifts = [m for m in moves if m.z == -15.0]
    assert len(lifts) == 4  # once per tab, on the last pass only


def test_taller_tabs_lift_on_more_passes():
    ring = rect_path()
    layout = place_tabs(ring, 4, 8.0, D)
    moves = tabbed_contour_moves(ring, layout, [-6.0, -12.0, -18.0], -10.0, 5.0, 1800, 600)
    assert len([m for m in moves if m.z == -10.0]) == 8  # passes -12 and -18
