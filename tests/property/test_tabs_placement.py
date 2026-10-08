"""Properties of tab placement on random closed paths."""

import math
from itertools import pairwise

from hypothesis import given, settings
from hypothesis import strategies as st

from chipload.core.errors import GeometryError
from chipload.core.toolpath.tabs import place_tabs, ring_length, tabbed_contour_moves


@st.composite
def convex_rings(draw):
    n = draw(st.integers(3, 40))
    angles = sorted(
        draw(
            st.lists(
                st.floats(0, 2 * math.pi, exclude_max=True), min_size=n, max_size=n, unique=True
            )
        )
    )
    radii = draw(st.lists(st.floats(20, 80), min_size=n, max_size=n))
    return [
        (round(r * math.cos(a), 6), round(r * math.sin(a), 6))
        for a, r in zip(angles, radii, strict=True)
    ]


@settings(max_examples=150, deadline=None)
@given(convex_rings(), st.integers(1, 12), st.floats(1, 20), st.floats(2, 12))
def test_tabs_never_overlap_cover_the_start_or_lose_material(ring, count, width, tool):
    try:
        layout = place_tabs(ring, count, width, tool)
    except GeometryError:
        length = ring_length(ring)
        assert length / count - (width + tool) < tool  # refused only when they really don't fit
        return
    spans = layout.spans
    assert len(spans) == count
    assert spans[0][0] >= tool / 2 - 1e-9  # start point keeps clear of the first tab
    assert spans[-1][1] <= layout.length - tool / 2 + 1e-9  # ... and of the last
    for (_, end), (start, _) in pairwise(spans):
        assert start - end >= tool - 1e-9  # at least one tool diameter of normal cut between
    for s0, s1 in spans:
        assert abs(s1 - s0 - (width + tool)) < 1e-9
    # The move builder verifies its own result (lifted length == tab gaps on every low pass).
    tabbed_contour_moves(ring, layout, [-3.0, -6.0], -4.0, 5.0, 1000, 300)
