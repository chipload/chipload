"""G-code regression: same input + same job must give byte-identical output.

`pytest --update-golden` rewrites the expected files. Only use it when a change is intended,
review the diff, and explain it in the commit message.
"""

import difflib
from pathlib import Path

import pytest

from chipload.core.job.loader import load_job
from chipload.core.pipeline import run_job

CORPUS = Path(__file__).parent.parent / "corpus"

CASES = [
    # (golden name, job file, drawing)
    ("v1_example_job", "jobs/v1/example_job.json", "dxf/simple_part.dxf"),
    ("v2_simple_part_full", "jobs/v2/simple_part_full.json", "dxf/simple_part.dxf"),
]


@pytest.mark.parametrize(("name", "job", "dxf"), CASES, ids=[c[0] for c in CASES])
def test_golden_gcode(request, name, job, dxf):
    gcode = run_job(load_job(CORPUS / job), CORPUS / dxf).gcode
    expected_path = CORPUS / "expected" / f"{name}.nc"
    if request.config.getoption("--update-golden"):
        expected_path.write_text(gcode, encoding="ascii", newline="\n")
        return
    expected = expected_path.read_text(encoding="ascii")
    if gcode != expected:
        diff = "".join(
            difflib.unified_diff(
                expected.splitlines(keepends=True),
                gcode.splitlines(keepends=True),
                "expected",
                "actual",
                n=2,
            )
        )
        pytest.fail(f"G-code differs from {expected_path.name}:\n{diff[:4000]}")


def test_output_is_deterministic_across_runs():
    job = load_job(CORPUS / "jobs/v2/simple_part_full.json")
    first = run_job(job, CORPUS / "dxf/simple_part.dxf").gcode
    second = run_job(job, CORPUS / "dxf/simple_part.dxf").gcode
    assert first == second
