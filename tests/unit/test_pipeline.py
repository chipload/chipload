import hashlib
import json
from pathlib import Path

import ezdxf
import pytest

from chipload.core.errors import (
    DxfImportError,
    GcodeCheckFailed,
    GeometryError,
    JobError,
    SelectorError,
)
from chipload.core.job.loader import load_job
from chipload.core.job.models import Job
from chipload.core.pipeline import run_job

CORPUS = Path(__file__).parent.parent / "corpus"
SIMPLE = CORPUS / "dxf" / "simple_part.dxf"
MESSY = CORPUS / "dxf" / "messy.dxf"
BASE = json.loads((CORPUS / "jobs" / "v2" / "simple_part_full.json").read_text())


def job(**changes) -> Job:
    """The v2 sample job with nested changes, e.g. job(stock={"z_zero": "bottom"})."""
    data = json.loads(json.dumps(BASE))
    for key, value in changes.items():
        if isinstance(value, dict) and isinstance(data.get(key), dict):
            data[key].update(value)
        else:
            data[key] = value
    return Job.model_validate(data)


def op(**changes):
    base = dict(BASE["operations"][1])
    base.update(changes)
    return base


def z_values(gcode: str) -> list[float]:
    return [float(w[1:]) for line in gcode.splitlines() for w in line.split() if w[0] == "Z"]


def test_sample_runs_and_passes_check():
    result = run_job(job(), SIMPLE)
    assert result.check_warnings == ()
    assert min(z_values(result.gcode)) == -12.0


def test_depth_equal_to_thickness_is_allowed():
    result = run_job(job(operations=[op(depth=12)]), SIMPLE)
    assert min(z_values(result.gcode)) == -12.0


def test_depth_beyond_thickness_is_refused():
    with pytest.raises(JobError, match="deeper than the stock"):
        run_job(job(operations=[op(depth=12.5)]), SIMPLE)


def test_z_zero_bottom_shifts_heights():
    result = run_job(job(stock={"z_zero": "bottom"}, machine={"safe_z": 17}), SIMPLE)
    zs = z_values(result.gcode)
    assert min(zs) == 0.0
    assert max(zs) == 17.0


def test_safe_z_must_clear_the_stock():
    with pytest.raises(JobError, match="safe_z"):
        run_job(job(stock={"z_zero": "bottom"}), SIMPLE)  # safe_z 5 < top 12


def test_tool_without_rpm_is_refused():
    with pytest.raises(JobError, match="rpm"):
        run_job(job(tools=[{"id": 1, "diameter": 6}]), SIMPLE)


def test_multiple_tools_are_refused():
    tools = [{"id": 1, "diameter": 6, "rpm": 18000}, {"id": 2, "diameter": 3, "rpm": 18000}]
    with pytest.raises(JobError, match="one tool"):
        run_job(job(tools=tools, operations=[op(), op(tool=2)]), SIMPLE)


def test_feed_above_machine_max_is_refused_not_clamped():
    with pytest.raises(JobError, match="max_feed"):
        run_job(job(operations=[op(feed=6000)]), SIMPLE)


def test_pocket_is_not_implemented_yet():
    pocket = {
        "type": "pocket", "select": {"layer": "HOLES"}, "tool": 1, "depth": 3,
        "step_down": 3, "stepover": 0.4, "feed": 1000, "plunge_feed": 300,
    }  # fmt: skip
    with pytest.raises(JobError, match="not implemented"):
        run_job(job(operations=[pocket]), SIMPLE)


def test_selector_matching_nothing_is_an_error():
    with pytest.raises(SelectorError, match="matches nothing"):
        run_job(job(operations=[op(select={"layer": "NOPE"})]), SIMPLE)


def test_layer_match_is_case_insensitive():
    run_job(job(operations=[op(select={"layer": "cut_outside"})]), SIMPLE)


def test_open_contour_cannot_be_profiled():
    data = job(source={"units": "mm"}, operations=[op(select={"layer": "OPEN"})])
    with pytest.raises(SelectorError, match="open"):
        run_job(data, MESSY)


def test_tool_larger_than_hole_names_the_contour():
    tools = [{"id": 1, "diameter": 32, "rpm": 18000}]
    holes = op(select={"layer": "HOLES"}, side="inside")
    with pytest.raises(GeometryError, match="contour #2"):
        run_job(job(tools=tools, operations=[holes]), SIMPLE)


def test_holes_are_cut_before_the_outline_even_if_listed_together(tmp_path):
    # One op selecting by handles across the outline and a hole: still hole first.
    doc = ezdxf.new()
    doc.header["$INSUNITS"] = 4
    msp = doc.modelspace()
    outline = msp.add_lwpolyline([(10, 10), (110, 10), (110, 90), (10, 90)], close=True)
    hole = msp.add_circle((60, 50), 10)
    path = tmp_path / "part.dxf"
    doc.saveas(path)
    sel = {"handles": [outline.dxf.handle, hole.dxf.handle]}
    gcode = run_job(job(operations=[op(select=sel, side="on")]), path).gcode
    first_xy = next(line for line in gcode.splitlines() if line.startswith("G0 X"))
    x = float(first_xy.split()[1][1:])
    assert 50 <= x <= 70  # the hole, not the outline at x=10


def test_sha256_mismatch_is_refused():
    with pytest.raises(JobError, match="sha256"):
        run_job(job(source={"sha256": "0" * 64}), SIMPLE)


def test_sha256_match_is_accepted():
    digest = hashlib.sha256(SIMPLE.read_bytes()).hexdigest()
    run_job(job(source={"sha256": digest}), SIMPLE)


def test_inch_drawing_with_mm_job_is_refused():
    with pytest.raises(DxfImportError, match="inch"):
        run_job(job(source={"units": "inch"}), SIMPLE)


def test_v1_sample_migrates_to_v2():
    loaded = load_job(CORPUS / "jobs" / "v1" / "example_job.json")
    assert loaded.schema_version == 2
    assert loaded.machine.spindle == "cw"
    assert loaded.operations[0].plunge_feed == pytest.approx(2000 / 3)


def test_checker_rejection_returns_no_gcode():
    small_machine = {"travel": {"x": 100, "y": 600, "z": 100}}  # part is 206 mm wide
    with pytest.raises(GcodeCheckFailed, match="exceeds machine travel"):
        run_job(job(machine=small_machine), SIMPLE)
