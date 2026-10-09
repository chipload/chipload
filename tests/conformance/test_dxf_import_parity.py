"""The Rust DXF import must find the same contours and issues as the Python one.

Structure (contours, closed/open, layers, handles, nesting, issue codes and messages) must
match exactly, geometry within GEOMETRY_TOL. The Rust importer reads the drawing as drawn
rather than reproducing ezdxf (ADR 0005), so known differences are listed explicitly.
"""

import json
import re
import subprocess
from pathlib import Path

import pytest

from chipload.core.io.dxf_import import import_dxf
from chipload.core.units import Units

DXF = Path(__file__).parent.parent / "corpus" / "dxf"

# Both sides chain the same Line/Arc primitives with the same arc flattening, so points agree
# to float noise; this is far below CHAIN_TOL_MM.
GEOMETRY_TOL = 1e-6

# messy.dxf has a SPLINE with fit points only. ezdxf interpolates it with tangents of its own
# choosing; Rust refuses to guess the curve and skips it with a warning.
FIT_POINT_SPLINE_LAYER = "SPLINE"


def python_import(path: Path) -> dict:
    result = import_dxf(path, Units.MM)
    report = result.report
    return {
        "units": report.units.value,
        "units_from": report.units_from,
        "imported": dict(report.imported),
        "flattened": dict(report.flattened),
        "ignored": dict(report.ignored),
        "issues": [
            {
                "severity": i.severity,
                "code": i.code,
                "message": i.message,
                "location": list(i.location) if i.location else None,
            }
            for i in report.issues
        ],
        "bbox": [list(report.bbox[0]), list(report.bbox[1])] if report.bbox else None,
        "contours": [
            {
                "id": c.id,
                "closed": c.closed,
                "layers": list(c.layers),
                "handles": sorted({p.segment.handle for p in c.parts}),
                "nesting_depth": c.nesting_depth,
                "invalid_reason": c.invalid_reason,
                "length": sum(p.segment.prim.length() for p in c.parts),
                "points": [list(p) for p in c.points()],
            }
            for c in result.contours
        ],
    }


def rust_import(cli: Path, path: Path) -> dict:
    run = subprocess.run(
        [str(cli), "inspect", str(path), "--units", "mm", "--json"],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(run.stdout)


def without_layer(result: dict, layer: str) -> dict:
    """Drop contours on `layer` and the open-contour issues about them."""
    dropped = [c for c in result["contours"] if c["layers"] == [layer]]
    starts = [c["points"][0] for c in dropped]
    result["contours"] = [c for c in result["contours"] if c not in dropped]
    result["issues"] = [
        i for i in result["issues"] if not (i["code"] == "open_contour" and i["location"] in starts)
    ]
    return result


def normalise_ids(message: str) -> str:
    return re.sub(r"Contour \d+", "Contour #", message)


def assert_close(a, b, what: str) -> None:
    if a is None or b is None:
        assert a == b, what
        return
    flat_a = [x for p in a for x in (p if isinstance(p, list) else [p])]
    flat_b = [x for p in b for x in (p if isinstance(p, list) else [p])]
    assert len(flat_a) == len(flat_b), f"{what}: {len(flat_a)} vs {len(flat_b)} values"
    worst = max((abs(x - y) for x, y in zip(flat_a, flat_b, strict=True)), default=0.0)
    assert worst <= GEOMETRY_TOL, f"{what}: off by {worst}"


def assert_same_import(rust: dict, python: dict) -> None:
    for key in ("units", "units_from", "imported", "flattened", "ignored"):
        assert rust[key] == python[key], key
    assert_close(rust["bbox"], python["bbox"], "bbox")

    assert len(rust["issues"]) == len(python["issues"]), (
        [i["code"] for i in rust["issues"]],
        [i["code"] for i in python["issues"]],
    )
    for r, p in zip(rust["issues"], python["issues"], strict=True):
        assert (r["severity"], r["code"]) == (p["severity"], p["code"])
        assert normalise_ids(r["message"]) == normalise_ids(p["message"])
        assert_close(r["location"], p["location"], f"location of {p['message']}")

    assert len(rust["contours"]) == len(python["contours"])
    for r, p in zip(rust["contours"], python["contours"], strict=True):
        label = f"contour {p['id']} on {p['layers']}"
        for key in ("closed", "layers", "handles", "nesting_depth"):
            assert r[key] == p[key], f"{label}: {key}"
        assert (r["invalid_reason"] is None) == (p["invalid_reason"] is None), label
        assert abs(r["length"] - p["length"]) <= GEOMETRY_TOL, f"{label}: length"
        assert_close(r["points"], p["points"], f"{label}: points")


@pytest.mark.parametrize("name", ["simple_part", "messy_2"])
def test_same_import(rust_cli, name):
    path = DXF / f"{name}.dxf"
    assert_same_import(rust_import(rust_cli, path), python_import(path))


def test_messy_matches_except_the_fit_point_spline(rust_cli):
    path = DXF / "messy.dxf"
    rust = rust_import(rust_cli, path)
    python = python_import(path)

    # The one known difference, stated precisely.
    skipped = [i for i in rust["issues"] if i["code"] == "entity_skipped"]
    assert [i["message"] for i in skipped] == [
        "SPLINE (34): has fit points only; re-export it with control points"
    ]
    assert rust["ignored"].pop("SPLINE") == 1
    assert python["imported"].pop("SPLINE") == 1
    assert python["flattened"].pop("SPLINE") == 1
    rust["issues"].remove(skipped[0])
    python = without_layer(python, FIT_POINT_SPLINE_LAYER)
    # bbox: the spline is the only geometry above y = 110.
    python["bbox"] = rust["bbox"]

    assert_same_import(rust, python)
