"""Real-world drawing (LibreCAD/dxfrw 0.6.3, mm): pins down how it imports.

Bulged LWPOLYLINEs (rounded rectangles, keyhole shapes), circles, three layers, contours
that cross each other, and two single-line open contours.
"""

from pathlib import Path

import pytest

from chipload.core.io.dxf_import import import_dxf
from chipload.core.units import Units

DXF = Path(__file__).parent.parent / "corpus" / "dxf" / "messy_2.dxf"


@pytest.fixture(scope="module")
def result():
    return import_dxf(DXF, None)


def test_units_and_counts(result):
    report = result.report
    assert report.units == Units.MM
    assert report.units_from == "dxf"
    assert dict(report.imported) == {"CIRCLE": 5, "LWPOLYLINE": 17}
    assert report.ignored == ()
    assert report.flattened == ()


def test_extents(result):
    (x0, y0), (x1, y1) = result.report.bbox
    assert (x0, y0, x1, y1) == pytest.approx((-90.499, -50.0, 90.499, 55.0), abs=0.001)


def test_only_the_two_single_lines_are_open(result):
    issues = [i for i in result.report.issues]
    assert [i.code for i in issues] == ["open_contour", "open_contour"]
    open_contours = [c for c in result.contours if not c.closed]
    assert [(c.layers, c.start, c.end) for c in open_contours] == [
        (("lower",), pytest.approx((-64.75, 50.0)), pytest.approx((-64.75, 20.0))),
        (("upper",), pytest.approx((64.75, 50.0)), pytest.approx((64.75, 20.0))),
    ]


def test_every_closed_contour_is_valid(result):
    closed = [c for c in result.contours if c.closed]
    assert len(closed) == 20
    assert all(c.invalid_reason is None for c in closed)


def test_contours_per_layer(result):
    per_layer = {}
    for c in result.contours:
        per_layer[c.layers] = per_layer.get(c.layers, 0) + 1
    assert per_layer == {("0",): 3, ("lower",): 4, ("upper",): 15}
