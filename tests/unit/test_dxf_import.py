import math

import ezdxf
import pytest

from chipload.core.errors import DxfImportError
from chipload.core.geometry.primitives import Arc, distance
from chipload.core.io.dxf_import import import_document
from chipload.core.units import Units


def new_doc(insunits: int = 4) -> ezdxf.document.Drawing:
    doc = ezdxf.new()
    doc.header["$INSUNITS"] = insunits
    return doc


def codes(result) -> list[str]:
    return [i.code for i in result.report.issues]


def add_square(msp, x=0.0, y=0.0, size=10.0, layer="CUT"):
    msp.add_lwpolyline(
        [(x, y), (x + size, y), (x + size, y + size), (x, y + size)],
        close=True,
        dxfattribs={"layer": layer},
    )


# -- units


def test_mm_file_with_mm_job():
    doc = new_doc(4)
    add_square(doc.modelspace())
    result = import_document(doc, Units.MM)
    assert result.report.units == Units.MM
    assert result.report.units_from == "dxf"


def test_inch_file_is_scaled_to_mm():
    doc = new_doc(1)
    add_square(doc.modelspace(), size=1.0)
    result = import_document(doc, Units.INCH)
    pts = result.contours[0].points()
    assert max(p[0] for p in pts) == pytest.approx(25.4)


def test_unit_mismatch_is_refused():
    doc = new_doc(1)
    with pytest.raises(DxfImportError, match="inch"):
        import_document(doc, Units.MM)


def test_unitless_file_uses_job_units_and_says_so():
    doc = new_doc(0)
    add_square(doc.modelspace(), size=1.0)
    result = import_document(doc, Units.INCH)
    assert result.report.units_from == "job"
    assert "units_from_job" in codes(result)
    assert max(p[0] for p in result.contours[0].points()) == pytest.approx(25.4)


def test_unsupported_units_are_refused():
    with pytest.raises(DxfImportError, match="INSUNITS=6"):
        import_document(new_doc(6), Units.MM)  # metres


# -- entities


def test_bulged_polyline_arcs_end_on_vertices():
    doc = new_doc()
    # A slot: two straight sides and two half-circle ends (bulge 1 = 180 degrees).
    doc.modelspace().add_lwpolyline(
        [(0, 0, 0), (20, 0, 1), (20, 10, 0), (0, 10, 1)], format="xyb", close=True
    )
    result = import_document(doc, Units.MM)
    contour = result.contours[0]
    assert contour.closed
    arcs = [p.segment.prim for p in contour.parts if isinstance(p.segment.prim, Arc)]
    assert len(arcs) == 2
    assert {(round(a.center[0], 6), round(a.center[1], 6)) for a in arcs} == {(20, 5), (0, 5)}
    # The slot bulges outwards, so it extends 5 mm beyond the vertices at both ends.
    xs = [p[0] for p in contour.points()]
    assert min(xs) == pytest.approx(-5, abs=0.01)
    assert max(xs) == pytest.approx(25, abs=0.01)


def test_negative_bulge_curves_the_other_way():
    doc = new_doc()
    doc.modelspace().add_lwpolyline([(0, 0, -1), (10, 0, 0)], format="xyb")
    arc = import_document(doc, Units.MM).contours[0].parts[0].segment.prim
    assert isinstance(arc, Arc)
    assert distance(arc.start, (0, 0)) < 1e-9
    assert distance(arc.end, (10, 0)) < 1e-9
    assert arc.midpoint()[1] == pytest.approx(5.0)  # bulge -1 from left to right goes up (CW)


def test_mirrored_arc_extrusion_is_converted_to_wcs():
    doc = new_doc()
    # OCS with extrusion -Z: X is mirrored. CCW 0..90 in OCS is a CW arc from (-5,0) to (0,5).
    doc.modelspace().add_arc((0, 0), 5, 0, 90, dxfattribs={"extrusion": (0, 0, -1)})
    arc = import_document(doc, Units.MM).contours[0].parts[0].segment.prim
    assert isinstance(arc, Arc)
    assert distance(arc.start, (-5, 0)) < 1e-9
    assert distance(arc.end, (0, 5)) < 1e-9
    assert arc.sweep == pytest.approx(-math.pi / 2)


def test_insert_is_exploded_with_transform_and_layer():
    doc = new_doc()
    block = doc.blocks.new("PART")
    add_square(block, layer="0")
    msp = doc.modelspace()
    insert = msp.add_blockref(
        "PART", (100, 50), dxfattribs={"layer": "CUT", "rotation": 90, "xscale": 2, "yscale": 2}
    )
    result = import_document(doc, Units.MM)
    contour = result.contours[0]
    assert contour.layers == ("CUT",)
    assert contour.parts[0].segment.handle == insert.dxf.handle
    xs = [p[0] for p in contour.points()]
    ys = [p[1] for p in contour.points()]
    assert (min(xs), max(xs)) == pytest.approx((80, 100))
    assert (min(ys), max(ys)) == pytest.approx((50, 70))


def test_spline_is_flattened_and_reported():
    doc = new_doc()
    doc.modelspace().add_spline([(0, 0), (10, 10), (20, 0), (30, 10)])
    result = import_document(doc, Units.MM)
    assert dict(result.report.flattened) == {"SPLINE": 1}


def test_unsupported_entities_are_counted_not_dropped_silently():
    doc = new_doc()
    msp = doc.modelspace()
    msp.add_text("hello")
    msp.add_point((1, 1))
    add_square(msp)
    result = import_document(doc, Units.MM)
    assert dict(result.report.ignored) == {"POINT": 1, "TEXT": 1}


def test_nonzero_z_is_reported():
    doc = new_doc()
    doc.modelspace().add_line((0, 0, 3), (10, 0, 3))
    assert "nonzero_z" in codes(import_document(doc, Units.MM))


def test_duplicates_and_zero_length_are_reported():
    doc = new_doc()
    msp = doc.modelspace()
    add_square(msp)
    msp.add_line((0, 0), (10, 0))  # duplicates a square edge
    msp.add_line((5, 5), (5, 5))
    result = import_document(doc, Units.MM)
    assert codes(result).count("duplicate") == 1
    assert codes(result).count("zero_length") == 1
    assert [c.closed for c in result.contours] == [True]


def test_open_contour_is_reported_with_endpoints():
    doc = new_doc()
    doc.modelspace().add_lwpolyline([(0, 0), (10, 0), (10, 10)])
    result = import_document(doc, Units.MM)
    assert "open_contour" in codes(result)
    assert not result.contours[0].closed


def test_self_intersecting_contour_is_invalid():
    doc = new_doc()
    doc.modelspace().add_lwpolyline([(0, 0), (10, 10), (10, 0), (0, 10)], close=True)
    result = import_document(doc, Units.MM)
    assert result.contours[0].invalid_reason is not None
    assert "invalid_contour" in codes(result)


def test_nesting_depth():
    doc = new_doc()
    msp = doc.modelspace()
    add_square(msp, 0, 0, 100)
    add_square(msp, 10, 10, 30)
    add_square(msp, 15, 15, 5)
    add_square(msp, 200, 0, 10)
    depths = [c.nesting_depth for c in import_document(doc, Units.MM).contours]
    assert depths == [0, 1, 2, 0]


def test_without_job_units_the_file_units_are_used():
    doc = new_doc(1)
    add_square(doc.modelspace(), size=1.0)
    assert import_document(doc, None).report.units == Units.INCH


def test_without_job_units_a_unitless_file_is_refused():
    with pytest.raises(DxfImportError, match="no units"):
        import_document(new_doc(0), None)
