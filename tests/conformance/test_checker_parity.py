"""The Rust G-code checker must give exactly the same report as the Python one."""

import json
import random
import subprocess
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from chipload.core.job.loader import load_job
from chipload.core.sim.gcode_check import check_gcode, limits_from_job

CORPUS = Path(__file__).parent.parent / "corpus"
GOLDEN = [
    ("v1_example_job", "jobs/v1/example_job.json"),
    ("v2_simple_part_full", "jobs/v2/simple_part_full.json"),
    ("v3_messy_2_lower", "jobs/v3/messy_2_lower.json"),
    ("v3_simple_part_tabs", "jobs/v3/simple_part_tabs.json"),
    ("v3_messy_2_lower_tabs", "jobs/v3/messy_2_lower_tabs.json"),
]

# Limits equal to tests/unit/test_gcode_check.py's LIMITS, expressed as a job.
CHECK_JOB = {
    "schema_version": 3,
    "source": {"file": "x.dxf", "units": "mm"},
    "stock": {"x": 600, "y": 400, "thickness": 18},
    "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw",
                "travel": {"x": 800, "y": 600, "z": 100}},
    "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
    "operations": [{"type": "profile", "select": {"layer": "L"}, "side": "on",
                    "direction": "climb", "tool": 1, "depth": 1, "step_down": 1,
                    "feed": 100, "plunge_feed": 100}],
}  # fmt: skip


def python_report(text: str, job_path: Path) -> dict:
    report = check_gcode(text, limits_from_job(load_job(job_path)))
    return {
        "ok": report.ok,
        "errors": [{"line": i.line, "message": i.message} for i in report.errors],
        "warnings": [{"line": i.line, "message": i.message} for i in report.warnings],
    }


def rust_report(cli: Path, gcode: Path, job_path: Path) -> dict:
    run = subprocess.run(
        [str(cli), "check", str(gcode), "--job", str(job_path), "--json"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert run.returncode in (0, 1), run.stderr
    return json.loads(run.stdout)


@pytest.fixture(scope="module")
def check_job(tmp_path_factory) -> Path:
    path = tmp_path_factory.mktemp("job") / "job.json"
    path.write_text(json.dumps(CHECK_JOB))
    return path


def assert_same(cli, tmp_path, text, job_path, name="program.nc"):
    gcode = tmp_path / name
    gcode.write_bytes(text.encode("utf-8"))
    assert rust_report(cli, gcode, job_path) == python_report(text, job_path)


@pytest.mark.parametrize(("name", "job"), GOLDEN, ids=[g[0] for g in GOLDEN])
def test_golden_programs(rust_cli, name, job):
    gcode = CORPUS / "expected" / f"{name}.nc"
    job_path = CORPUS / job
    assert rust_report(rust_cli, gcode, job_path) == python_report(gcode.read_text(), job_path)


# Same building blocks as tests/unit/test_gcode_check.py.
HEADER = "G21 G90 G17 G94\nG0 Z5\nM3 S18000\nG4 P3\n"
FOOTER = "G0 Z5\nM5\nM30\n"
GOOD_BODY = "G0 X10 Y10\nG1 Z-6 F600\nG1 X50 F1800\nG1 Y50\nG1 X10\nG1 Y10\n"
GOOD = HEADER + GOOD_BODY + FOOTER


def bad_programs() -> list[str]:
    h = HEADER
    return [
        "G21 G90\nG0 Z5\nG0 X10 Y10\nG1 Z-1 F100\n",
        h.replace("G21 ", "") + GOOD_BODY,
        h.replace("G21", "G20") + GOOD_BODY,
        h.replace(" G90", "") + GOOD_BODY,
        h + "G91\n",
        h + "G0 X10 Y10\nG1 Z-1\n",
        h + "G0 X10 Y10\nG1 Z-1 F0\n",
        h + "G0 X10 Y10\nG1 Z-1 F100\nG0 X20\n",
        "G21 G90\nG0 X10 Y10\n",
        h + "G0 X10 Y10\nG0 Z-1\n",
        h + "G0 X10 Y10\nG1 Z-18.5 F100\n",
        h + "G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10 I4 J0\n",
        h + "G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10\n",
        h + "G28\n",
        h + "M8\n",
        h + "G0 X10 Y10 X20\n",
        h + "G0 G1 X10\n",
        h + "G0 X10 Y10 (unclosed\n",
        h + "G0 X10 Y10 @\n",
        h + "T1\n",
        GOOD + "G0 Z10\n",
        h + "G0 X0 Y0\nG0 X900\n" + FOOTER,
        h + "G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10 I5 J0\nG3 X10 Y10 I-5 J0\n" + FOOTER,
        h + "G0.5 X1\n",
        "G21 G90\rG0 Z5\r\nM3\x0cG1 X1 F1\n",  # Python line boundaries
        h + "G1 X1. Y.5 Z-.25 F+600\n",
        h + "G1 X1 Y1 Z1e3\n",
        h + "((a)\n",
        h + "G1 X1 F-0\n",
    ]


def test_bad_programs(rust_cli, tmp_path, check_job):
    for index, text in enumerate(bad_programs()):
        assert_same(rust_cli, tmp_path, text, check_job, f"bad{index}.nc")


def test_mutated_golden_programs(rust_cli, tmp_path, check_job):
    """Delete, duplicate, swap and edit lines of a real program; reports must still agree."""
    lines = (CORPUS / "expected" / "v2_simple_part_full.nc").read_text().splitlines()
    rng = random.Random(1234)
    for index in range(150):
        mutated = list(lines)
        for _ in range(rng.randint(1, 4)):
            i = rng.randrange(len(mutated))
            kind = rng.choice(["delete", "duplicate", "swap", "edit"])
            if kind == "delete":
                del mutated[i]
            elif kind == "duplicate":
                mutated.insert(i, mutated[i])
            elif kind == "swap":
                j = rng.randrange(len(mutated))
                mutated[i], mutated[j] = mutated[j], mutated[i]
            else:
                mutated[i] = mutated[i].replace("Z", "Z-2").replace("G0", "G1")
        assert_same(rust_cli, tmp_path, "\n".join(mutated) + "\n", check_job, f"m{index}.nc")


WORD = st.builds(
    lambda letter, value: f"{letter}{value}",
    st.sampled_from(list("GMXYZIJFSPNTgx")),
    st.one_of(
        st.sampled_from(["0", "1", "2", "3", "4", "5", "17", "20", "21", "30", "90", "91"]),
        st.floats(-50, 50, allow_nan=False).map(lambda v: f"{v:.3f}"),
    ),
)
LINE = st.one_of(
    st.lists(WORD, max_size=5).map(" ".join),
    st.just("(comment)"),
    st.just("G0 X1 ; tail"),
    st.just("(open"),
    st.just("%"),
)


@settings(max_examples=60, deadline=None)
@given(st.lists(LINE, min_size=1, max_size=25))
def test_random_programs(rust_cli, tmp_path_factory, check_job, lines):
    tmp = tmp_path_factory.mktemp("rand")
    assert_same(rust_cli, tmp, "\n".join(lines) + "\n", check_job)
