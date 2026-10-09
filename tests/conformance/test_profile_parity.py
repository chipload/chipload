"""The Rust profile toolpath must match the Python one geometrically (ADR 0004/0006).

The offset libraries differ (Shapely/GEOS on chords vs cavalier_contours on true arcs), so
vertices differ. What must match exactly: the order of rings, their direction, the pass
depths and feeds. What must match within tolerance: each ring's shape and its start point.
"""

import copy
import json
import math
import subprocess
from pathlib import Path

import pytest

from chipload.core.job.loader import load_job
from chipload.core.pipeline import run_job
from chipload.core.tolerances import ARC_APPROX_TOL_MM
from chipload.core.toolpath.tabs import TAB_SLIDE_STEP_MM

CORPUS = Path(__file__).parent.parent / "corpus"

# Python's path is up to 2 x ARC_APPROX_TOL_MM inside the true offset (flattened input arcs,
# then flattened round joins); Rust's is up to 1 x (it offsets the true arcs, then flattens).
RING_TOL = 3 * ARC_APPROX_TOL_MM
EXACT = 1e-9
# Tabs slide off corners in TAB_SLIDE_STEP_MM steps, and "straight enough" sums the turning
# at each side's own vertices: on a small corner arc one chord turns ~9 degrees against the
# 15 degree limit, so a tab may land a step plus one nearby chord (on either path) apart.
LIFT_NEIGHBOURHOOD_MM = 20.0

BASE_JOB = {
    "schema_version": 3,
    "source": {"file": "x.dxf", "units": "mm"},
    "stock": {"x": 600, "y": 400, "thickness": 18, "origin": "center", "z_zero": "top"},
    "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw",
                "travel": {"x": 800, "y": 600, "z": 100}},
    "tools": [{"id": 1, "type": "endmill", "diameter": 6, "rpm": 18000}],
    "operations": [],
}  # fmt: skip


def profile(
    select: dict, side: str, direction: str = "climb", depth: float = 12, tabs: dict | None = None
) -> dict:
    op = {
        "type": "profile",
        "select": select,
        "side": side,
        "direction": direction,
        "tool": 1,
        "depth": depth,
        "step_down": 5,
        "feed": 1500,
        "plunge_feed": 500,
    }
    if tabs is not None:
        op["tabs"] = tabs
    return op  # fmt: skip


TABS = {"count": 4, "width": 8, "height": 3}


# (name, dxf, operations, spindle)
SYNTHETIC = [
    ("messy_2_lower_outline", "messy_2.dxf", [profile({"handles": ["50"]}, "outside")], "cw"),
    ("messy_2_upper_holes", "messy_2.dxf",
     [profile({"layer": "upper"}, "inside", "conventional")], "cw"),
    ("messy_2_shared_outline_on", "messy_2.dxf", [profile({"layer": "0"}, "on")], "ccw"),
    ("messy_blocks_and_mirror", "messy.dxf",
     [profile({"layer": "HOLES"}, "inside"), profile({"layer": "CUT"}, "outside")], "ccw"),
    ("simple_part_both_sides", "simple_part.dxf",
     [profile({"layer": "HOLES"}, "outside", "conventional"),
      profile({"layer": "CUT_OUTSIDE"}, "inside")], "cw"),
    ("messy_2_lower_outline_tabs", "messy_2.dxf",
     [profile({"handles": ["50"]}, "outside", tabs=TABS)], "cw"),
    ("messy_2_shared_outline_tabs", "messy_2.dxf",
     [profile({"layer": "0"}, "outside", "conventional", depth=18,
              tabs={"count": 6, "width": 5, "height": 8})], "ccw"),
    ("messy_holes_tabs_on_curves", "messy.dxf",
     [profile({"layer": "HOLES"}, "outside", tabs={"count": 2, "width": 2, "height": 2})], "cw"),
]  # fmt: skip

CORPUS_JOBS = [
    ("v1_example_job", "jobs/v1/example_job.json", "simple_part.dxf"),
    ("v2_simple_part_full", "jobs/v2/simple_part_full.json", "simple_part.dxf"),
    ("v3_simple_part_tabs", "jobs/v3/simple_part_tabs.json", "simple_part.dxf"),
]


def python_moves(job_path: Path, dxf: Path) -> list[dict]:
    program = run_job(load_job(job_path), dxf).program
    return [{"kind": m.kind, "x": m.x, "y": m.y, "z": m.z, "feed": m.feed} for m in program.moves]


def rust_moves(cli: Path, job_path: Path, dxf: Path) -> list[dict]:
    run = subprocess.run(
        [str(cli), "plan", str(dxf), "--job", str(job_path), "--json"],
        capture_output=True,
        text=True,
        check=False,  # the assert below shows stderr
    )
    assert run.returncode == 0, run.stderr
    return json.loads(run.stdout)["program"]["moves"]


def rings_of(moves: list[dict]) -> list[dict]:
    """Split a profile program into rings.

    Each ring: its start, every Z change while cutting (z, feed, XY where it happens: the
    plunges, and the lifts over tabs), the XY feed rates used, and the path of the first
    pass (every XY point cut, in order, without the closing return to the start).
    """
    rings: list[dict] = []
    x = y = None
    for m in moves:
        if m["kind"] == "rapid" and m["x"] is not None:
            x, y = m["x"], m["y"]
            rings.append({"start": (x, y), "z_events": [], "feeds": set(), "path": [(x, y)]})
        elif m["kind"] == "feed" and m["z"] is not None:
            rings[-1]["z_events"].append((m["z"], m["feed"], (x, y)))
        elif m["kind"] == "feed":
            ring = rings[-1]
            x, y = m["x"], m["y"]
            ring["feeds"].add(m["feed"])
            if not ring.get("lap_done"):
                if (x, y) == ring["start"]:
                    ring["lap_done"] = True
                else:
                    ring["path"].append((x, y))
    return rings


def signed_area(points: list[tuple[float, float]]) -> float:
    return (
        sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(points, points[1:] + points[:1], strict=True))
        / 2
    )


def segment_distance(p, a, b) -> float:
    dx, dy = b[0] - a[0], b[1] - a[1]
    length2 = dx * dx + dy * dy
    t = (
        0.0
        if length2 == 0
        else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length2))
    )
    return math.hypot(p[0] - a[0] - dx * t, p[1] - a[1] - dy * t)


def farthest(points, ring) -> float:
    """Largest distance from any of `points` (and chord midpoints) to the closed polyline."""
    closed = list(zip(ring, ring[1:] + ring[:1], strict=True))
    samples = list(points) + [
        ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        for a, b in zip(points, points[1:] + points[:1], strict=True)
    ]
    return max(min(segment_distance(p, a, b) for a, b in closed) for p in samples)


def longest_chord_near(path, point) -> float:
    closed = zip(path, path[1:] + path[:1], strict=True)
    return max(
        (math.dist(a, b) for a, b in closed if math.dist(a, point) <= LIFT_NEIGHBOURHOOD_MM),
        default=0.0,
    )


def lift_tol(r_path, p_path, point) -> float:
    quantisation = longest_chord_near(r_path, point) + longest_chord_near(p_path, point)
    return TAB_SLIDE_STEP_MM + RING_TOL + quantisation


def assert_same_toolpath(rust: list[dict], python: list[dict]) -> None:
    r_rings, p_rings = rings_of(rust), rings_of(python)
    assert len(r_rings) == len(p_rings), "ring count"
    for i, (r, p) in enumerate(zip(r_rings, p_rings, strict=True)):
        label = f"ring {i} (Python start {p['start']})"
        r_path, p_path = r["path"], p["path"]
        assert (signed_area(r_path) > 0) == (signed_area(p_path) > 0), f"{label}: direction"
        assert farthest(r_path, p_path) <= RING_TOL, f"{label}: Rust ring off Python's"
        assert farthest(p_path, r_path) <= RING_TOL, f"{label}: Python ring off Rust's"
        # Python starts at the lowest vertex of its chords, Rust at the true lowest point, so
        # they may differ by up to one of Python's chords next to its start.
        chords = [math.dist(p_path[0], p_path[1]), math.dist(p_path[0], p_path[-1])]
        start_tol = max(chords) + RING_TOL
        assert abs(r["start"][1] - p["start"][1]) <= RING_TOL, f"{label}: start y"
        assert math.dist(r["start"], p["start"]) <= start_tol, f"{label}: start"

        assert r["feeds"] == p["feeds"], f"{label}: XY feeds"
        r_events, p_events = r["z_events"], p["z_events"]
        assert len(r_events) == len(p_events), f"{label}: number of plunges and tab lifts"
        for (rz, rf, rxy), (pz, pf, pxy) in zip(r_events, p_events, strict=True):
            assert abs(rz - pz) <= EXACT and rf == pf, f"{label}: Z {rz} vs {pz}"
            tol = start_tol if pxy == p["start"] else lift_tol(r_path, p_path, pxy)
            assert math.dist(rxy, pxy) <= tol, f"{label}: Z change at {rxy} vs {pxy}"


@pytest.mark.parametrize(("name", "job", "dxf"), CORPUS_JOBS, ids=[c[0] for c in CORPUS_JOBS])
def test_corpus_jobs(rust_cli, name, job, dxf):
    job_path, dxf_path = CORPUS / job, CORPUS / "dxf" / dxf
    assert_same_toolpath(rust_moves(rust_cli, job_path, dxf_path), python_moves(job_path, dxf_path))


@pytest.mark.parametrize(("name", "dxf", "operations", "spindle"), SYNTHETIC,
                         ids=[c[0] for c in SYNTHETIC])  # fmt: skip
def test_synthetic_jobs(rust_cli, tmp_path, name, dxf, operations, spindle):
    job = copy.deepcopy(BASE_JOB)
    job["operations"] = operations
    job["machine"]["spindle"] = spindle
    job_path = tmp_path / f"{name}.json"
    job_path.write_text(json.dumps(job))
    dxf_path = CORPUS / "dxf" / dxf
    assert_same_toolpath(rust_moves(rust_cli, job_path, dxf_path), python_moves(job_path, dxf_path))
