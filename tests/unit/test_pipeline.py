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
from chipload.core.job.models import CURRENT_VERSION, Job
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


def test_drill_is_not_implemented_yet():
    drill = {"type": "drill", "select": {"layer": "HOLES"}, "tool": 1}
    with pytest.raises(JobError, match="not implemented"):
        run_job(job(operations=[drill]), SIMPLE)


def test_selector_matching_nothing_is_an_error():
    with pytest.raises(SelectorError, match="matches nothing"):
        run_job(job(operations=[op(select={"layer": "NOPE"})]), SIMPLE)


def test_layer_match_is_case_insensitive():
    run_job(job(operations=[op(select={"layer": "cut_outside"})]), SIMPLE)


def test_selection_of_only_open_contours_is_an_error():
    data = job(source={"units": "mm"}, operations=[op(select={"layer": "OPEN"})])
    with pytest.raises(SelectorError, match="only open contours"):
        run_job(data, MESSY)


def test_open_centreline_on_a_cutting_layer_is_skipped_and_reported():
    # Real drawing: the `upper` layer holds the lid's shapes plus an open centreline (#22).
    data = job(
        stock={"x": 200, "y": 120, "origin": "center"},
        operations=[op(select={"layer": "upper"}, side="on")],
    )
    result = run_job(data, CORPUS / "dxf" / "messy_2.dxf")
    skipped = [n for n in result.notes if "skipped open contour" in n]
    expected = (
        "Operation 1 (Outline): skipped open contour #22 on upper "
        "(64.750, 50.000) to (64.750, 20.000)"
    )
    assert skipped == [expected]


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


def test_v1_sample_migrates_to_current():
    loaded = load_job(CORPUS / "jobs" / "v1" / "example_job.json")
    assert loaded.schema_version == CURRENT_VERSION
    assert loaded.machine.spindle == "cw"
    assert loaded.operations[0].plunge_feed == pytest.approx(2000 / 3)


def test_checker_rejection_returns_no_gcode():
    small_machine = {"travel": {"x": 100, "y": 600, "z": 100}}  # part is 206 mm wide
    with pytest.raises(GcodeCheckFailed, match="exceeds machine travel"):
        run_job(job(machine=small_machine), SIMPLE)


def test_result_exposes_toolpath_and_contours():
    result = run_job(job(), SIMPLE)
    assert len(result.contours) == 4
    assert result.program.moves[0].kind == "rapid"
    assert result.program.spindle == "cw"


def test_checker_rejection_carries_the_toolpath_but_no_gcode():
    small_machine = {"travel": {"x": 100, "y": 600, "z": 100}}
    with pytest.raises(GcodeCheckFailed) as caught:
        run_job(job(machine=small_machine), SIMPLE)
    assert caught.value.program is not None
    assert not hasattr(caught.value, "gcode")


# -- pocket

MESSY_2 = CORPUS / "dxf" / "messy_2.dxf"
LOWER = json.loads((CORPUS / "jobs" / "v3" / "messy_2_lower.json").read_text())


def lower_job(tool_changes=None, pocket_changes=None) -> Job:
    data = json.loads(json.dumps(LOWER))
    data["tools"][0].update(tool_changes or {})
    data["operations"][0].update(pocket_changes or {})
    return Job.model_validate(data)


def test_pocket_job_runs_on_the_real_drawing():
    result = run_job(lower_job(), MESSY_2)
    assert result.check_warnings == ()
    assert min(z_values(result.gcode)) == -18.0  # the outline goes through


def test_pocket_refuses_a_tool_that_is_not_centre_cutting():
    with pytest.raises(JobError, match="not centre-cutting"):
        run_job(lower_job({"center_cutting": False}), MESSY_2)


def test_pocket_warns_when_centre_cutting_is_unknown():
    result = run_job(lower_job({"center_cutting": None}), MESSY_2)
    assert any("isn't marked center_cutting" in n for n in result.notes)


def test_pocket_refuses_a_vbit():
    with pytest.raises(JobError, match="need an endmill"):
        run_job(lower_job({"type": "vbit"}), MESSY_2)


def test_pocket_retracts_between_rings_by_default():
    down = run_job(lower_job(pocket_changes={"keep_tool_down": True}), MESSY_2).gcode
    retract = run_job(lower_job(pocket_changes={"keep_tool_down": False}), MESSY_2).gcode
    assert retract.count("G0 Z5.000") > down.count("G0 Z5.000")


def test_pocket_errors_name_the_operation():
    with pytest.raises(GeometryError, match="Operation 1 \\(Openings\\).*does not fit"):
        run_job(lower_job({"diameter": 16}), MESSY_2)


# -- tabs

TABS_JOB = json.loads((CORPUS / "jobs" / "v3" / "simple_part_tabs.json").read_text())


def tabs_job(tab_changes=None, op_changes=None) -> Job:
    data = json.loads(json.dumps(TABS_JOB))
    data["operations"][1]["tabs"].update(tab_changes or {})
    data["operations"][1].update(op_changes or {})
    return Job.model_validate(data)


def test_tabs_lift_only_the_last_pass_and_are_reported_for_display():
    result = run_job(tabs_job(), SIMPLE)
    assert len(result.tabs) == 4
    zs = z_values(result.gcode)
    assert -9.0 in zs  # tab top: cut bottom -12 + height 3
    assert min(zs) == -12.0


def test_tab_height_must_be_below_the_cut_depth():
    with pytest.raises(JobError, match="tab height"):
        run_job(tabs_job({"height": 12}), SIMPLE)


def test_too_many_tabs_names_the_contour():
    with pytest.raises(GeometryError, match="contour #1.*don't fit"):
        run_job(tabs_job({"count": 60}), SIMPLE)


def test_tabs_on_an_inside_profile_warn():
    holes = {"select": {"layer": "HOLES"}, "side": "inside"}
    result = run_job(tabs_job({"count": 1, "width": 3}, holes), SIMPLE)
    assert any("inside profile" in n for n in result.notes)
    assert any("corner or curve" in n for n in result.notes)  # the round hole has no straight
