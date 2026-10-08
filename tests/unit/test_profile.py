import math

import pytest

from chipload.core.errors import GeometryError
from chipload.core.geometry.primitives import signed_area
from chipload.core.toolpath.model import Move
from chipload.core.toolpath.ordering import PathRing, order_rings
from chipload.core.toolpath.profile import (
    contour_moves,
    cut_clockwise,
    offset_rings,
    orient,
    pass_depths,
)

SQUARE = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]


def bbox(ring):
    xs = [p[0] for p in ring]
    ys = [p[1] for p in ring]
    return min(xs), min(ys), max(xs), max(ys)


# -- offset side


def test_outside_offset_grows_by_tool_radius():
    (ring,) = offset_rings(SQUARE, "outside", 3.0)
    assert bbox(ring) == pytest.approx((-3, -3, 103, 103))


def test_inside_offset_shrinks_by_tool_radius():
    (ring,) = offset_rings(SQUARE, "inside", 3.0)
    assert bbox(ring) == pytest.approx((3, 3, 97, 97))


def test_on_follows_the_contour():
    assert offset_rings(SQUARE, "on", 3.0) == [SQUARE]


def test_tool_larger_than_hole_is_an_error():
    hole = [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)]
    with pytest.raises(GeometryError, match="does not fit"):
        offset_rings(hole, "inside", 3.0)


def test_tool_exactly_as_wide_as_slot_is_an_error():
    slot = [(0.0, 0.0), (50.0, 0.0), (50.0, 6.0), (0.0, 6.0)]
    with pytest.raises(GeometryError):
        offset_rings(slot, "inside", 3.0)


def test_narrow_neck_splits_inside_offset():
    # Two 40 mm squares joined by a 4 mm wide neck; a 6 mm tool cannot pass the neck.
    dumbbell = [
        (0.0, 0.0), (40.0, 0.0), (40.0, 18.0), (60.0, 18.0), (60.0, 0.0), (100.0, 0.0),
        (100.0, 40.0), (60.0, 40.0), (60.0, 22.0), (40.0, 22.0), (40.0, 40.0), (0.0, 40.0),
    ]  # fmt: skip
    rings = offset_rings(dumbbell, "inside", 3.0)
    assert len(rings) == 2


# -- direction


@pytest.mark.parametrize(
    ("side", "direction", "spindle", "clockwise"),
    [
        ("outside", "climb", "cw", True),
        ("outside", "conventional", "cw", False),
        ("inside", "climb", "cw", False),
        ("inside", "conventional", "cw", True),
        ("on", "climb", "cw", True),
        ("on", "conventional", "cw", False),
        ("outside", "climb", "ccw", False),
        ("outside", "conventional", "ccw", True),
        ("inside", "climb", "ccw", True),
        ("inside", "conventional", "ccw", False),
        ("on", "climb", "ccw", False),
        ("on", "conventional", "ccw", True),
    ],
)
def test_direction_table(side, direction, spindle, clockwise):
    assert cut_clockwise(side, direction, spindle) is clockwise
    (ring,) = offset_rings(SQUARE, side, 3.0)
    oriented = orient(ring, cut_clockwise(side, direction, spindle))
    assert (signed_area(oriented) < 0) is clockwise


def test_orient_starts_at_lowest_then_leftmost_vertex():
    ring = [(100.0, 100.0), (0.0, 100.0), (0.0, 0.0), (100.0, 0.0)]
    assert orient(ring, clockwise=True)[0] == (0.0, 0.0)
    assert orient(ring, clockwise=False)[0] == (0.0, 0.0)


# -- depth passes


@pytest.mark.parametrize(
    ("depth", "step_down", "expected"),
    [
        (18.0, 6.0, [6.0, 12.0, 18.0]),  # exact multiple: no extra sliver pass
        (6.0, 6.0, [6.0]),  # depth equal to step down
        (2.0, 6.0, [2.0]),  # shallower than one step
        (10.0, 6.0, [5.0, 10.0]),  # not a multiple: equal passes
        (18.0005, 6.0, [6.0001666, 12.000333, 18.0005]),  # within DEPTH_TOL of a multiple
    ],
)
def test_pass_depths(depth, step_down, expected):
    assert pass_depths(depth, step_down) == pytest.approx(expected)


def test_pass_depths_never_exceed_step_down():
    for depth in (0.1, 5.9, 6.0, 6.1, 17.99, 18.0, 18.01, 100.0):
        passes = pass_depths(depth, 6.0)
        steps = [b - a for a, b in zip([0.0, *passes], passes, strict=False)]
        assert max(steps) <= 6.0 + 1e-9
        assert passes[-1] == depth


def test_pass_depths_rejects_non_positive():
    with pytest.raises(GeometryError):
        pass_depths(0.0, 6.0)


# -- moves


def test_contour_moves_sequence():
    ring = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
    moves = contour_moves(ring, [-1.0, -2.0], safe_z=5.0, feed=1000.0, plunge_feed=300.0)
    assert moves[0] == Move("rapid", z=5.0)
    assert moves[1] == Move("rapid", x=0.0, y=0.0)
    assert moves[2] == Move("feed", z=-1.0, feed=300.0)
    assert moves[5] == Move("feed", x=0.0, y=0.0, feed=1000.0)  # closes the loop
    assert moves[6] == Move("feed", z=-2.0, feed=300.0)  # next pass without retracting
    assert moves[-1] == Move("rapid", z=5.0)


# -- ordering


def ring_at(x, y, depth, cid):
    return PathRing(((x, y), (x + 1, y), (x + 1, y + 1)), depth, cid)


def test_holes_are_cut_before_their_outline():
    outline = ring_at(0, 0, 0, 1)
    hole_far = ring_at(80, 80, 1, 2)
    hole_near = ring_at(10, 10, 1, 3)
    ordered = order_rings([outline, hole_far, hole_near])
    assert [r.contour_id for r in ordered] == [3, 2, 1]


def test_ordering_is_nearest_neighbour_from_origin():
    rings = [ring_at(100, 0, 0, 1), ring_at(10, 0, 0, 2), ring_at(50, 0, 0, 3)]
    assert [r.contour_id for r in order_rings(rings)] == [2, 3, 1]


def test_full_circle_offset_stays_round():
    circle = [(50 * math.cos(a), 50 * math.sin(a)) for a in (i * math.pi / 180 for i in range(360))]
    (ring,) = offset_rings(circle, "outside", 3.0)
    radii = [math.hypot(*p) for p in ring]
    assert min(radii) == pytest.approx(53.0, abs=0.02)
    assert max(radii) == pytest.approx(53.0, abs=0.02)
