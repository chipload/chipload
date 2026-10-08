import math

from chipload.core.geometry.chain import Segment, chain_segments
from chipload.core.geometry.primitives import Arc, Line, Primitive
from chipload.core.tolerances import CHAIN_TOL_MM


def seg(prim: Primitive, layer: str = "L") -> Segment:
    return Segment(prim, layer, "H", "LINE")


def square(size: float = 10.0) -> list[Segment]:
    pts = [(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)]
    return [seg(Line(pts[i], pts[(i + 1) % 4])) for i in range(4)]


def test_square_is_one_closed_contour():
    result = chain_segments(square())
    assert len(result.contours) == 1
    contour = result.contours[0]
    assert contour.closed
    assert len(contour.points()) == 4


def test_segments_in_mixed_direction_and_order_still_chain():
    a, b, c, d = square()
    flipped = seg(Line(c.prim.end, c.prim.start))
    result = chain_segments([d, a, flipped, b])
    assert [c.closed for c in result.contours] == [True]


def test_gap_within_tolerance_is_joined():
    a, b, c, _ = square()
    nudged = seg(Line((0.0, 10.0), (0.0, CHAIN_TOL_MM / 2)))
    result = chain_segments([a, b, c, nudged])
    assert result.contours[0].closed


def test_gap_above_tolerance_gives_open_contour():
    a, b, c, _ = square()
    short = seg(Line((0.0, 10.0), (0.0, CHAIN_TOL_MM * 5)))
    result = chain_segments([a, b, c, short])
    assert [c.closed for c in result.contours] == [False]


def test_zero_length_and_duplicates_are_removed_and_returned():
    segments = square()
    dup = seg(Line(segments[0].prim.end, segments[0].prim.start))  # same line, reversed
    tiny = seg(Line((5.0, 5.0), (5.0, 5.0 + CHAIN_TOL_MM / 10)))
    result = chain_segments([*segments, dup, tiny])
    assert result.duplicates == (dup,)
    assert result.zero_length == (tiny,)
    assert [c.closed for c in result.contours] == [True]


def test_two_different_arcs_between_same_points_are_not_duplicates():
    upper = seg(Arc((0.0, 0.0), 5.0, 0.0, math.pi))
    lower = seg(Arc((0.0, 0.0), 5.0, math.pi, math.pi))
    result = chain_segments([upper, lower])
    assert result.duplicates == ()
    assert [c.closed for c in result.contours] == [True]


def test_full_circle_is_closed():
    result = chain_segments([seg(Arc((3.0, 4.0), 2.0, 0.0, 2 * math.pi))])
    assert result.contours[0].closed


def test_branch_point_splits_into_open_contours():
    segments = [*square(), seg(Line((10.0, 10.0), (20.0, 20.0)))]
    result = chain_segments(segments)
    assert result.branch_points == ((10.0, 10.0),)
    assert not any(c.closed for c in result.contours)


def test_contour_ids_follow_drawing_order():
    first = square(10.0)
    second = [
        seg(Line((p[0] + 50, p[1]), (q[0] + 50, q[1])))
        for p, q in ((s.prim.start, s.prim.end) for s in square(5.0) if isinstance(s.prim, Line))
    ]
    result = chain_segments([*first, *second])
    assert [c.id for c in result.contours] == [1, 2]
    assert result.contours[0].points()[0] == (0.0, 0.0)
