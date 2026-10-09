"""The Rust pocket planner must match the Python one (ADR 0004/0006).

Exact: the pieces and their contour ids, the number of rings per piece, which links stay at
depth, ring direction, the pass depths, and which warnings appear. Within tolerance: each
ring's shape and its start point. Offsets differ (GEOS on chords vs cavalier on true arcs),
so vertices do not match one to one.
"""

import copy
import json
import math
import subprocess
from pathlib import Path

import ezdxf
import pytest
from test_profile_parity import RING_TOL, farthest, signed_area

from chipload.core.io.dxf_import import import_dxf
from chipload.core.job.loader import load_job
from chipload.core.ops.select import select_contours
from chipload.core.pipeline import run_job
from chipload.core.toolpath.pocket import plan_pocket
from chipload.core.units import Units

CORPUS = Path(__file__).parent.parent / "corpus"

BASE_JOB = {
    "schema_version": 3,
    "source": {"file": "x.dxf", "units": "mm"},
    "stock": {"x": 600, "y": 400, "thickness": 18, "origin": "center", "z_zero": "top"},
    "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw",
                "travel": {"x": 800, "y": 600, "z": 100}},
    "tools": [{"id": 1, "type": "endmill", "diameter": 6, "rpm": 18000,
               "center_cutting": True}],
    "operations": [],
}  # fmt: skip


def pocket(layer: str, stepover=0.4, direction="climb", keep_tool_down=True) -> dict:
    return {"type": "pocket", "select": {"layer": layer}, "tool": 1, "depth": 6,
            "step_down": 3, "stepover": stepover, "direction": direction,
            "feed": 1500, "plunge_feed": 400, "keep_tool_down": keep_tool_down}  # fmt: skip


def rect(msp, x0, y0, x1, y1, layer="P"):
    msp.add_lwpolyline([(x0, y0), (x1, y0), (x1, y1), (x0, y1)], close=True,
                       dxfattribs={"layer": layer})  # fmt: skip


def drawing_island(msp):
    rect(msp, 0, 0, 80, 50)
    msp.add_circle((40, 25), 10, dxfattribs={"layer": "P"})


def drawing_crossing_bars(msp):
    rect(msp, 0, 20, 100, 32)
    rect(msp, 44, 0, 56, 60)


def drawing_neck(msp):
    msp.add_lwpolyline(
        [(0, 0), (40, 0), (40, 18), (60, 18), (60, 0), (100, 0),
         (100, 40), (60, 40), (60, 22), (40, 22), (40, 40), (0, 40)],
        close=True, dxfattribs={"layer": "P"},
    )  # fmt: skip


def drawing_sharp_triangle(msp):
    msp.add_lwpolyline([(0, 0), (90, 0), (10, 40)], close=True, dxfattribs={"layer": "P"})


def drawing_rounded_slot(msp):
    # Bulged ends: true arcs on the Rust side.
    msp.add_lwpolyline(
        [(10, 0, 0), (70, 0, 1), (70, 20, 0), (10, 20, 1)],
        format="xyb", close=True, dxfattribs={"layer": "P"},
    )  # fmt: skip


def drawing_island_touching_wall(msp):
    rect(msp, 0, 0, 80, 50)
    rect(msp, 0, 20, 30, 30)  # shares the left wall


DRAWINGS = {
    "island": drawing_island,
    "crossing_bars": drawing_crossing_bars,
    "neck": drawing_neck,
    "sharp_triangle": drawing_sharp_triangle,
    "rounded_slot": drawing_rounded_slot,
    "island_touching_wall": drawing_island_touching_wall,
}

# (name, drawing, pocket op, spindle)
CASES = [
    ("island_climb", "island", pocket("P"), "cw"),
    ("island_conventional_ccw", "island", pocket("P", direction="conventional"), "ccw"),
    ("island_always_retract", "island", pocket("P", keep_tool_down=False), "cw"),
    ("crossing_bars_separate", "crossing_bars", pocket("P"), "cw"),
    ("neck_warns", "neck", pocket("P"), "cw"),
    ("sharp_corners_uncut", "sharp_triangle", pocket("P", stepover=1.0), "cw"),
    ("sharp_corners_clean", "sharp_triangle", pocket("P", stepover=0.3), "cw"),
    ("rounded_slot", "rounded_slot", pocket("P", stepover=0.5), "cw"),
    ("island_touching_wall", "island_touching_wall", pocket("P"), "cw"),
]


def write_dxf(path: Path, build) -> None:
    doc = ezdxf.new()
    doc.header["$INSUNITS"] = 4
    build(doc.modelspace())
    doc.saveas(path)


def python_plan(job_path: Path, dxf: Path) -> dict:
    """The pocket pieces of the first operation, plus the run's notes and moves."""
    job = load_job(job_path)
    result = run_job(job, dxf)
    op = job.operations[0]
    imported = import_dxf(dxf, Units(job.source.units))
    selection = select_contours(imported.contours, op.select, "x")
    diameter = job.tools[0].diameter
    plan = plan_pocket(
        [(c.id, c.points()) for c in selection.contours],
        diameter / 2,
        op.stepover * diameter,
        op.direction,
        job.machine.spindle,
        op.keep_tool_down,
        (0.0, 0.0),
    )
    return {
        "pockets": [
            {
                "contour_ids": list(p.contour_ids),
                "rings": [[list(pt) for pt in r] for r in p.rings],
                "feed_links": list(p.feed_links),
            }
            for p in plan.pieces
        ],
        "notes": list(result.notes),
        "moves": result.program.moves,
    }


def rust_plan(cli: Path, job_path: Path, dxf: Path) -> dict:
    run = subprocess.run(
        [str(cli), "plan", str(dxf), "--job", str(job_path), "--json"],
        capture_output=True,
        text=True,
        check=False,  # the assert below shows stderr
    )
    assert run.returncode == 0, run.stderr
    return json.loads(run.stdout)


def note_kinds(notes: list[str]) -> list[str]:
    kinds = []
    for note in notes:
        if "uncut material" in note:
            kinds.append("uncut")
        elif "narrow part" in note:
            kinds.append("narrow")
        else:
            kinds.append(note)  # anything else must match word for word
    return sorted(kinds)


def plunges_and_depths(moves) -> list[tuple[str, float]]:
    """Every Z move while cutting, as (kind, z): the pass structure without XY."""
    out = []
    for m in moves:
        kind, z = (m["kind"], m["z"]) if isinstance(m, dict) else (m.kind, m.z)
        if z is not None:
            out.append((kind, round(z, 9)))
    return out


def assert_same_pockets(rust: dict, python: dict) -> None:
    assert note_kinds(rust["notes"]) == note_kinds(python["notes"]), (
        rust["notes"],
        python["notes"],
    )
    assert plunges_and_depths(rust["program"]["moves"]) == plunges_and_depths(python["moves"])
    r_pieces, p_pieces = rust["pockets"], python["pockets"]
    assert [p["contour_ids"] for p in r_pieces] == [p["contour_ids"] for p in p_pieces]
    for r, p in zip(r_pieces, p_pieces, strict=True):
        label = f"piece {p['contour_ids']}"
        assert len(r["rings"]) == len(p["rings"]), f"{label}: ring count"
        assert r["feed_links"] == p["feed_links"], f"{label}: links"
        for i, (rr, pr) in enumerate(zip(r["rings"], p["rings"], strict=True)):
            rr, pr = [tuple(q) for q in rr], [tuple(q) for q in pr]
            where = f"{label}, ring {i}"
            if len(pr) >= 3 and len(rr) >= 3:
                assert (signed_area(rr) > 0) == (signed_area(pr) > 0), f"{where}: direction"
                assert farthest(rr, pr) <= RING_TOL, f"{where}: Rust ring off Python's"
                assert farthest(pr, rr) <= RING_TOL, f"{where}: Python ring off Rust's"
                # Rings start at the vertex nearest the tool; vertices differ by up to a chord.
                chords = [math.dist(pr[0], pr[1]), math.dist(pr[0], pr[-1])]
                start_tol = max(chords) + RING_TOL
            else:
                start_tol = RING_TOL  # collapsed ring: a point or a tiny stroke
            assert math.dist(rr[0], pr[0]) <= start_tol, f"{where}: start"


@pytest.mark.parametrize(("name", "drawing", "op", "spindle"), CASES, ids=[c[0] for c in CASES])
def test_synthetic_pockets(rust_cli, tmp_path, name, drawing, op, spindle):
    dxf = tmp_path / f"{drawing}.dxf"
    write_dxf(dxf, DRAWINGS[drawing])
    job = copy.deepcopy(BASE_JOB)
    job["operations"] = [op]
    job["machine"]["spindle"] = spindle
    job_path = tmp_path / f"{name}.json"
    job_path.write_text(json.dumps(job))
    assert_same_pockets(rust_plan(rust_cli, job_path, dxf), python_plan(job_path, dxf))


@pytest.mark.parametrize("job", ["messy_2_lower", "messy_2_lower_tabs"])
def test_messy_2_corpus_jobs(rust_cli, job):
    job_path = CORPUS / "jobs" / "v3" / f"{job}.json"
    dxf = CORPUS / "dxf" / "messy_2.dxf"
    rust = rust_plan(rust_cli, job_path, dxf)
    python = python_plan(job_path, dxf)
    assert_same_pockets(rust, python)


# -- the uncut-material detector, judged independently


def drawing_circle(msp):
    msp.add_circle((0, 0), 20, dxfattribs={"layer": "P"})


def drawing_square(msp):
    rect(msp, 0, 0, 50, 50)


SIZE_TOL = 0.3

UNCUT_CASES = [
    ("sharp_triangle", drawing_sharp_triangle, 1.0),
    ("circle_wide_stepover", drawing_circle, 1.0),
    ("square_wide_stepover", drawing_square, 0.95),
]


def shapely_spots(region, rings, links, radius):
    """Python's definition (pocket.py `_uncut_material`) applied to the given rings, without
    leftover thinner than the chord tolerance: GEOS buffers are chords inside the true arcs,
    so their leftover has slivers up to ARC_APPROX_TOL_MM wide that can join real pieces."""
    import shapely
    from shapely.geometry import LinearRing, LineString, Polygon
    from shapely.ops import unary_union

    from chipload.core.geometry.offset import dilate_polygons, erode_polygons, quad_segs_for
    from chipload.core.tolerances import ARC_APPROX_TOL_MM, REST_TOL_MM

    qs = quad_segs_for(radius)
    reach = unary_union(
        [q for p in erode_polygons(region, radius) for q in dilate_polygons(p, radius)]
    )
    swept = [
        (LinearRing(r) if len(r) > 2 else LineString(r)).buffer(radius, quad_segs=qs) for r in rings
    ]
    swept += [
        LineString([rings[i][0], rings[i + 1][0]]).buffer(radius, quad_segs=qs)
        for i, down in enumerate(links)
        if down
    ]
    thin = ARC_APPROX_TOL_MM * 1.1
    left = reach.difference(unary_union(swept)).buffer(-thin / 2).buffer(thin / 2)
    pieces = [p for p in shapely.get_parts(left) if isinstance(p, Polygon)]
    return [p for p in pieces if not p.buffer(-REST_TOL_MM).is_empty]


@pytest.mark.parametrize(
    ("name", "build", "stepover"), UNCUT_CASES, ids=[c[0] for c in UNCUT_CASES]
)
def test_uncut_detector_agrees_with_shapely(rust_cli, tmp_path, name, build, stepover):
    import re

    from shapely.geometry import Polygon

    dxf = tmp_path / f"{name}.dxf"
    write_dxf(dxf, build)
    job = copy.deepcopy(BASE_JOB)
    job["operations"] = [pocket("P", stepover=stepover)]
    job_path = tmp_path / f"{name}.json"
    job_path.write_text(json.dumps(job))
    rust = rust_plan(rust_cli, job_path, dxf)
    (piece,) = rust["pockets"]
    contour = import_dxf(dxf, Units.MM).contours[0]
    spots = shapely_spots(
        Polygon(contour.points()),
        [[tuple(p) for p in r] for r in piece["rings"]],
        piece["feed_links"],
        3.0,
    )
    notes = [n for n in rust["notes"] if "uncut material" in n]
    assert spots, "the case should leave material"
    assert len(notes) == 1
    found = re.search(r"in (\d+) places?, largest about ([\d.]+) x ([\d.]+) mm", notes[0])
    assert found is not None
    count, width, height = int(found[1]), float(found[2]), float(found[3])
    assert count == len(spots)
    # The largest piece (by area; near-ties may pick either) has a matching size. Rust finds
    # leftover with REST_TOL_MM-sized cells, so the thin tips of cusp-shaped corner spots
    # (narrower than a cell) are missed: sizes agree to a few tenths, fine for "about".
    sizes = [(p.bounds[2] - p.bounds[0], p.bounds[3] - p.bounds[1]) for p in spots]
    assert any(abs(w - width) <= SIZE_TOL and abs(h - height) <= SIZE_TOL for w, h in sizes), (
        (width, height),
        sizes,
    )
