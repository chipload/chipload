"""Property: the tool centre path keeps the cutter on the requested side and never gouges.

For a tool of radius r, every point on the tool-centre path must be at least r - tol from the
contour (otherwise the cutter cuts into the part), and on the requested side of it.
"""

import math

from hypothesis import given, reject, settings
from hypothesis import strategies as st
from shapely.geometry import LineString, Point, Polygon

from chipload.core.errors import GeometryError
from chipload.core.tolerances import ARC_APPROX_TOL_MM
from chipload.core.tolerances import OFFSET_CHECK_SLACK_MM as NUMERIC_SLACK_MM
from chipload.core.toolpath.profile import offset_rings


@st.composite
def star_polygons(draw):
    """Star-shaped polygons are always simple: sorted angles, positive radii."""
    n = draw(st.integers(min_value=3, max_value=24))
    angles = sorted(
        draw(
            st.lists(
                st.floats(0, 2 * math.pi, exclude_max=True), min_size=n, max_size=n, unique=True
            )
        )
    )
    radii = draw(st.lists(st.floats(10, 100), min_size=n, max_size=n))
    # Rounded to 1e-6 mm, realistic DXF precision; avoids denormal coordinates like 1e-312.
    ring = [
        (round(r * math.cos(a), 6), round(r * math.sin(a), 6))
        for a, r in zip(angles, radii, strict=True)
    ]
    polygon = Polygon(ring)
    if not polygon.is_valid or polygon.area < 1:
        reject()
    return ring


def sample(ring, step=0.5):
    """Points along the closed ring, including mid-chord points where flattening cuts inside."""
    line = LineString([*ring, ring[0]])
    count = max(2, int(line.length / step))
    return [line.interpolate(i * line.length / count) for i in range(count)]


@settings(max_examples=200, deadline=None)
@given(star_polygons(), st.floats(0.5, 10))
def test_outside_path_never_gouges(ring, radius):
    part = Polygon(ring)
    (path,) = offset_rings(ring, "outside", radius)
    for p in sample(path):
        assert not part.contains(p)
        assert part.exterior.distance(p) >= radius - ARC_APPROX_TOL_MM - NUMERIC_SLACK_MM


@settings(max_examples=200, deadline=None)
@given(star_polygons(), st.floats(0.5, 10))
def test_inside_path_never_gouges(ring, radius):
    hole = Polygon(ring)
    try:
        paths = offset_rings(ring, "inside", radius)
    except GeometryError:
        return  # tool does not fit: refused, which is the safe outcome
    for path in paths:
        for p in sample(path):
            assert hole.contains(p) or hole.exterior.distance(p) <= NUMERIC_SLACK_MM
            assert hole.exterior.distance(p) >= radius - ARC_APPROX_TOL_MM - NUMERIC_SLACK_MM


@settings(max_examples=100, deadline=None)
@given(star_polygons(), st.floats(0.5, 10))
def test_outside_path_stays_close(ring, radius):
    """The path hugs the part: no point is further than r from it (no wasted air cut)."""
    part = Polygon(ring)
    (path,) = offset_rings(ring, "outside", radius)
    for x, y in path:
        assert part.exterior.distance(Point(x, y)) <= radius + NUMERIC_SLACK_MM
