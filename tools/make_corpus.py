"""Generate the synthetic DXFs in tests/corpus/dxf/.

Run once with `python tools/make_corpus.py` and commit the output. Generated files are frozen
corpus data: if the drawings need to change, add new files instead of regenerating old ones,
because golden G-code depends on them.
"""

from __future__ import annotations

import sys
from pathlib import Path

import ezdxf

CORPUS = Path(__file__).resolve().parent.parent / "tests" / "corpus" / "dxf"


def rounded_rect(x: float, y: float, w: float, h: float, r: float) -> list[tuple[float, ...]]:
    """LWPOLYLINE vertices (x, y, bulge) for a rectangle with quarter-circle corners."""
    b = 0.41421356237309503  # tan(90deg / 4): bulge of a CCW quarter circle
    return [
        (x + r, y, 0),
        (x + w - r, y, b),
        (x + w, y + r, 0),
        (x + w, y + h - r, b),
        (x + w - r, y + h, 0),
        (x + r, y + h, b),
        (x, y + h - r, 0),
        (x, y + r, b),
    ]


def simple_part() -> None:
    """A 200 x 120 mm plate with rounded corners, a round hole, a slot and a square hole."""
    doc = ezdxf.new("R2010")
    doc.header["$INSUNITS"] = 4  # mm
    msp = doc.modelspace()
    msp.add_lwpolyline(
        rounded_rect(50, 50, 200, 120, 10),
        format="xyb",
        close=True,
        dxfattribs={"layer": "CUT_OUTSIDE"},
    )
    msp.add_circle((90, 110), 15, dxfattribs={"layer": "HOLES"})
    msp.add_lwpolyline(
        [(140, 80, 0), (200, 80, 1), (200, 100, 0), (140, 100, 1)],
        format="xyb",
        close=True,
        dxfattribs={"layer": "HOLES"},
    )
    for start, end in (
        ((150, 120), (210, 120)),
        ((210, 120), (210, 150)),
        ((210, 150), (150, 150)),
        ((150, 150), (150, 120)),
    ):
        msp.add_line(start, end, dxfattribs={"layer": "HOLES"})
    doc.saveas(CORPUS / "simple_part.dxf")


def messy() -> None:
    """Unitless drawing with the kinds of problems real files have."""
    doc = ezdxf.new("R2010")
    doc.header["$INSUNITS"] = 0
    msp = doc.modelspace()
    msp.add_lwpolyline(
        [(0, 0), (100, 0), (100, 50), (0, 50)], close=True, dxfattribs={"layer": "CUT"}
    )
    msp.add_line((0, 0), (100, 0), dxfattribs={"layer": "CUT"})  # duplicate edge
    msp.add_line((20, 20), (20, 20), dxfattribs={"layer": "CUT"})  # zero length
    msp.add_lwpolyline([(120, 0), (150, 0), (150, 30)], dxfattribs={"layer": "OPEN"})
    msp.add_arc((200, 25), 10, 0, 90, dxfattribs={"layer": "MIRRORED", "extrusion": (0, 0, -1)})
    msp.add_spline([(0, 80), (30, 110), (60, 80), (90, 110)], dxfattribs={"layer": "SPLINE"})
    msp.add_text("LABEL", dxfattribs={"layer": "TEXT"})
    msp.add_line((0, 150, 5), (50, 150, 5), dxfattribs={"layer": "RAISED"})
    block = doc.blocks.new("HOLE")
    block.add_circle((0, 0), 4)
    for x in (20, 50, 80):
        msp.add_blockref("HOLE", (x, 25), dxfattribs={"layer": "HOLES"})
    doc.saveas(CORPUS / "messy.dxf")


def main() -> int:
    CORPUS.mkdir(parents=True, exist_ok=True)
    simple_part()
    messy()
    return 0


if __name__ == "__main__":
    sys.exit(main())
