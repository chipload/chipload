"""Properties of pocket toolpaths on random shapes with random islands.

- No cutting move comes closer than the tool radius (minus tolerance) to a wall or island.
- With stepover <= half the tool diameter nothing reachable is left uncut. (Any reachable point
  lies within the stepover of some ring level, and stepover <= radius.)
"""

import math

import shapely
from hypothesis import event, given, reject, settings
from hypothesis import strategies as st
from shapely.geometry import Polygon

from chipload.core.errors import GeometryError
from chipload.core.tolerances import ARC_APPROX_TOL_MM, OFFSET_CHECK_SLACK_MM
from chipload.core.toolpath.pocket import build_region, plan_pocket, ring_geometry


@st.composite
def star(draw, min_r, max_r, cx=0.0, cy=0.0, n_min=3):
    n = draw(st.integers(min_value=n_min, max_value=16))
    angles = sorted(
        draw(
            st.lists(
                st.floats(0, 2 * math.pi, exclude_max=True), min_size=n, max_size=n, unique=True
            )
        )
    )
    radii = draw(st.lists(st.floats(min_r, max_r), min_size=n, max_size=n))
    ring = [
        (round(cx + r * math.cos(a), 6), round(cy + r * math.sin(a), 6))
        for a, r in zip(angles, radii, strict=True)
    ]
    if not Polygon(ring).is_valid or Polygon(ring).area < 1:
        reject()
    return ring


@st.composite
def pocket_with_islands(draw):
    # Many vertices keep the boundary roughly round, so islands near the centre fit inside it.
    boundary = draw(star(30, 60, n_min=8))
    outer = Polygon(boundary)
    islands = []
    for _ in range(draw(st.integers(0, 3))):
        cx = draw(st.floats(-12, 12))
        cy = draw(st.floats(-12, 12))
        island = draw(star(1, 4, cx, cy))
        poly = Polygon(island)
        if not outer.buffer(-0.5).contains(poly):
            continue
        if any(Polygon(i).buffer(0.5).intersects(poly) for i in islands):
            continue
        islands.append(island)
    return [(1, boundary), *[(i + 2, ring) for i, ring in enumerate(islands)]]


@settings(max_examples=150, deadline=None)
@given(pocket_with_islands(), st.floats(1.0, 5.0), st.floats(0.1, 0.5), st.booleans())
def test_pocket_never_gouges_and_clears_everything(contours, radius, stepover_fraction, keep):
    try:
        result = plan_pocket(
            contours, radius, stepover_fraction * 2 * radius, "climb", "cw", keep, (0.0, 0.0)
        )
    except GeometryError as exc:
        assert "does not fit" in str(exc)
        event("tool does not fit")
        return
    event(f"islands: {len(contours) - 1}")
    event(f"rings: {'<10' if sum(len(p.rings) for p in result.pieces) < 10 else '10+'}")
    regions = build_region(contours)
    clearance = radius - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM
    for piece in result.pieces:
        for ring in piece.rings:
            region = next(a.polygon for a in regions if a.contour_ids == piece.contour_ids)
            assert shapely.distance(region.boundary, ring_geometry(ring)) >= clearance
    assert not any("uncut material" in n for n in result.notes), result.notes
