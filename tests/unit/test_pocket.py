import math

import pytest
import shapely
from shapely.geometry import Polygon

from chipload.core.errors import GeometryError, SelectorError
from chipload.core.geometry.offset import erode_polygons
from chipload.core.geometry.primitives import distance, signed_area
from chipload.core.tolerances import ARC_APPROX_TOL_MM
from chipload.core.toolpath.pocket import build_region, plan_pocket, pocket_moves, ring_geometry

RECT = [(0.0, 0.0), (100.0, 0.0), (100.0, 60.0), (0.0, 60.0)]


def circle(cx, cy, r, n=72):
    return [
        (cx + r * math.cos(2 * math.pi * i / n), cy + r * math.sin(2 * math.pi * i / n))
        for i in range(n)
    ]


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def plan(contours, radius=3.0, stepover=2.4, direction="climb", spindle="cw", keep=False):
    return plan_pocket(contours, radius, stepover, direction, spindle, keep, (0.0, 0.0))


def bbox(ring):
    xs = [p[0] for p in ring]
    ys = [p[1] for p in ring]
    return min(xs), min(ys), max(xs), max(ys)


# -- region


def test_crossing_shapes_merge_into_one_region():
    pieces = build_region([(1, rect(0, 0, 50, 10)), (2, rect(20, -20, 30, 30))])
    assert len(pieces) == 1
    assert pieces[0].area == pytest.approx(500 + 500 - 100)


def test_selected_contour_inside_another_is_an_island():
    (piece,) = build_region([(1, RECT), (2, rect(40, 20, 60, 40))])
    assert len(piece.interiors) == 1
    assert piece.area == pytest.approx(6000 - 400)


def test_boundary_inside_an_island_is_cleared_again():
    pieces = build_region([(1, RECT), (2, rect(20, 10, 80, 50)), (3, rect(40, 20, 60, 40))])
    assert len(pieces) == 2


def test_unselected_contours_are_not_islands():
    (piece,) = build_region([(1, RECT)])
    assert len(piece.interiors) == 0


def test_same_shape_twice_is_refused():
    with pytest.raises(SelectorError, match="same shape"):
        build_region([(1, RECT), (2, list(reversed(RECT)))])


# -- rings


def test_ring_count_and_inside_out_order():
    (piece,) = plan([(1, RECT)]).pieces
    # Level 0 is 94 x 54; each level shrinks 2.4 per side until the 54 side is gone: 12 levels.
    assert len(piece.rings) == 12
    assert bbox(piece.rings[-1]) == pytest.approx((3, 3, 97, 57))  # the wall is cut last
    areas = [abs(signed_area(list(r))) for r in piece.rings]
    assert areas == sorted(areas)


def test_tool_too_big_is_an_error():
    with pytest.raises(GeometryError, match="does not fit"):
        plan([(7, rect(0, 0, 5, 5))])


def test_narrow_neck_warns():
    dumbbell = [
        (0.0, 0.0), (40.0, 0.0), (40.0, 18.0), (60.0, 18.0), (60.0, 0.0), (100.0, 0.0),
        (100.0, 40.0), (60.0, 40.0), (60.0, 22.0), (40.0, 22.0), (40.0, 40.0), (0.0, 40.0),
    ]  # fmt: skip
    result = plan([(1, dumbbell)])
    assert any("narrow part" in n for n in result.notes)


@pytest.mark.parametrize(
    ("direction", "spindle", "material_on_right"),
    [
        ("climb", "cw", True),
        ("conventional", "cw", False),
        ("climb", "ccw", False),
        ("conventional", "ccw", True),
    ],
)
def test_material_side_of_every_ring(direction, spindle, material_on_right):
    """With M3, climb keeps the material (nearest wall or island) on the right of travel."""
    contours = [(1, RECT), (2, circle(50, 30, 8))]
    region = build_region(contours)[0]
    (piece,) = plan(contours, direction=direction, spindle=spindle).pieces
    for ring in piece.rings:
        a, b = max(zip(ring, ring[1:] + ring[:1]), key=lambda s: distance(*s))
        mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
        length = distance(a, b)
        nx, ny = (b[1] - a[1]) / length, -(b[0] - a[0]) / length  # unit normal to the right
        right = shapely.Point(mx + 0.1 * nx, my + 0.1 * ny)
        left = shapely.Point(mx - 0.1 * nx, my - 0.1 * ny)
        right_is_closer = region.boundary.distance(right) < region.boundary.distance(left)
        assert right_is_closer is material_on_right


def test_rings_keep_tool_radius_from_walls_and_islands():
    region = build_region([(1, RECT), (2, circle(50, 30, 8))])[0]
    (piece,) = plan([(1, RECT), (2, circle(50, 30, 8))]).pieces
    for ring in piece.rings:
        assert (
            shapely.distance(region.boundary, ring_geometry(ring)) >= 3.0 - ARC_APPROX_TOL_MM - 1e-6
        )


# -- uncut material


def test_large_stepover_reports_uncut_material():
    result = plan([(1, RECT)], stepover=6.0)  # stepover = D: corners are missed
    assert any("uncut material" in n for n in result.notes)


def test_normal_stepover_leaves_nothing():
    assert plan([(1, RECT)], stepover=2.4).notes == ()


# -- links


def test_default_always_retracts_between_rings():
    (piece,) = plan([(1, RECT)]).pieces
    assert not any(piece.feed_links)


def test_keep_tool_down_feeds_short_safe_links():
    (piece,) = plan([(1, RECT)], keep=True).pieces
    assert all(piece.feed_links)


def test_keep_tool_down_still_retracts_around_an_island():
    (piece,) = plan([(1, RECT), (2, rect(30, 20, 70, 40))], keep=True).pieces
    assert any(piece.feed_links)
    assert not all(piece.feed_links)
    for i, stays_down in enumerate(piece.feed_links):
        if stays_down:
            assert distance(piece.rings[i][0], piece.rings[i + 1][0]) <= 6.0


# -- moves


def test_moves_plunge_at_the_centre_for_every_depth():
    (piece,) = plan([(1, RECT)]).pieces
    moves = pocket_moves(piece, [-3.0, -6.0], safe_z=5.0, feed=1500.0, plunge_feed=400.0)
    plunges = [m for m in moves if m.kind == "feed" and m.z is not None]
    assert len(plunges) == 2 * len(piece.rings)  # retract mode: one plunge per ring per depth
    assert {m.z for m in plunges} == {-3.0, -6.0}
    assert all(m.feed == 400.0 for m in plunges)
    assert moves[-1].kind == "rapid" and moves[-1].z == 5.0


def test_keep_tool_down_plunges_once_per_depth():
    (piece,) = plan([(1, RECT)], keep=True).pieces
    moves = pocket_moves(piece, [-3.0, -6.0], safe_z=5.0, feed=1500.0, plunge_feed=400.0)
    assert sum(1 for m in moves if m.kind == "feed" and m.z is not None) == 2


def test_first_ring_starts_nearest_the_start_position():
    near_top_right = plan_pocket([(1, RECT)], 3.0, 2.4, "climb", "cw", False, (100.0, 60.0))
    start = near_top_right.pieces[0].rings[0][0]
    assert start[0] > 50 and start[1] > 29


def test_pieces_are_cut_nearest_first():
    result = plan([(1, rect(200, 0, 250, 50)), (2, rect(0, 0, 50, 50))])
    assert [p.contour_ids for p in result.pieces] == [(2,), (1,)]


def test_plan_is_deterministic():
    contours = [(1, RECT), (2, circle(50, 30, 8))]
    assert plan(contours, keep=True) == plan(contours, keep=True)


# Found by Hypothesis: GEOS buffer(-21) of this convex polygon returns empty although its
# inscribed radius is 22.44 mm, which left the pocket centre uncut.
GEOS_COLLAPSE_CASE = [
    (30.0, 0.0), (26.327477, 14.382766), (2.122116, 29.92485), (-12.484405, 27.278923),
    (-24.034308, 17.954164), (-19.609309, -22.704075), (-6.323874, -29.325904),
]  # fmt: skip


def test_pocket_centre_is_cleared_where_geos_buffer_collapses_too_early():
    result = plan([(1, GEOS_COLLAPSE_CASE)], radius=1.0, stepover=1.0)
    assert result.notes == ()


def test_erode_does_not_collapse_early():
    polygon = Polygon(GEOS_COLLAPSE_CASE)
    assert polygon.buffer(-21).is_empty  # the GEOS behaviour being worked around
    (core,) = erode_polygons(polygon, 21.0)
    assert core.contains(shapely.Point(-0.0065, 4.6197))
