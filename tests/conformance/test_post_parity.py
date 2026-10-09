"""The Rust post-processor and `chipload run` must match the Python ones (ADR 0004).

- Rendering is exact: the same toolpath through the same post gives the same bytes, so
  Python's toolpath rendered by Rust must reproduce Python's G-code and the golden files.
- `chipload run` output from Rust must pass the Python checker too (a second, independent
  implementation), be deterministic, and report the same import summary and warnings.
- `chipload inspect` prints the same text.
"""

import json
import subprocess
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from chipload.core.job.loader import load_job
from chipload.core.job.post_config import load_builtin_post
from chipload.core.pipeline import run_job
from chipload.core.post.render import render_program
from chipload.core.sim.gcode_check import check_gcode, limits_from_job
from chipload.core.toolpath.model import Move, Program

CORPUS = Path(__file__).parent.parent / "corpus"
GOLDEN = [
    ("v1_example_job", "jobs/v1/example_job.json", "simple_part.dxf"),
    ("v2_simple_part_full", "jobs/v2/simple_part_full.json", "simple_part.dxf"),
    ("v3_messy_2_lower", "jobs/v3/messy_2_lower.json", "messy_2.dxf"),
    ("v3_simple_part_tabs", "jobs/v3/simple_part_tabs.json", "simple_part.dxf"),
    ("v3_messy_2_lower_tabs", "jobs/v3/messy_2_lower_tabs.json", "messy_2.dxf"),
]
IDS = [g[0] for g in GOLDEN]


def program_json(program: Program) -> dict:
    return {
        "tool_id": program.tool_id,
        "tool_diameter": program.tool_diameter,
        "rpm": program.rpm,
        "spindle": program.spindle,
        "safe_z": program.safe_z,
        "moves": [
            {"kind": m.kind, "x": m.x, "y": m.y, "z": m.z, "feed": m.feed} for m in program.moves
        ],
    }


def rust_render(cli: Path, program: Program, tmp_path: Path) -> str:
    path = tmp_path / "program.json"
    path.write_text(json.dumps(program_json(program)))
    run = subprocess.run(
        [str(cli), "render", str(path), "--post", "grbl"],
        capture_output=True,
        check=False,  # the assert below shows stderr
    )
    assert run.returncode == 0, run.stderr
    return run.stdout.decode("ascii")


def run_cli(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(args, capture_output=True, text=True, check=False)


@pytest.mark.parametrize(("name", "job", "dxf"), GOLDEN, ids=IDS)
def test_rust_post_renders_python_toolpaths_byte_for_byte(rust_cli, tmp_path, name, job, dxf):
    result = run_job(load_job(CORPUS / job), CORPUS / "dxf" / dxf)
    rendered = rust_render(rust_cli, result.program, tmp_path)
    assert rendered == result.gcode
    assert rendered == (CORPUS / "expected" / f"{name}.nc").read_text()


coordinate = st.one_of(
    st.none(),
    st.floats(-1000, 1000, allow_nan=False),
    st.sampled_from([0.0, -0.0, -0.0004, 0.0005, -0.0005, 0.125, 2.5, 1e-9, -1e-9]),
)


@st.composite
def programs(draw) -> Program:
    moves = []
    for _ in range(draw(st.integers(0, 30))):
        kind = draw(st.sampled_from(["rapid", "feed"]))
        x, y, z = draw(coordinate), draw(coordinate), draw(coordinate)
        feed = draw(st.floats(1, 20000)) if kind == "feed" else None
        moves.append(Move(kind, x, y, z, feed))
    return Program(
        tool_id=draw(st.integers(1, 99)),
        tool_diameter=draw(st.floats(0.1, 50)),
        rpm=draw(st.floats(1000, 30000)),
        spindle=draw(st.sampled_from(["cw", "ccw"])),
        safe_z=draw(st.floats(1, 50)),
        moves=tuple(moves),
    )


@settings(max_examples=150, deadline=None)
@given(programs())
def test_random_toolpaths_render_identically(rust_cli, tmp_path_factory, program):
    tmp_path = tmp_path_factory.mktemp("render")
    python = render_program(program, load_builtin_post("grbl"))
    assert rust_render(rust_cli, program, tmp_path) == python


@pytest.mark.parametrize(("name", "job", "dxf"), GOLDEN, ids=IDS)
def test_rust_run_passes_the_python_checker_and_is_deterministic(
    rust_cli, tmp_path, name, job, dxf
):
    job_path, dxf_path = CORPUS / job, CORPUS / "dxf" / dxf
    outputs = []
    for attempt in range(2):
        out = tmp_path / f"{attempt}.nc"
        run = run_cli([str(rust_cli), "run", str(dxf_path), "--job", str(job_path), "-o", str(out)])
        assert run.returncode == 0, run.stderr
        outputs.append(out.read_bytes())
    assert outputs[0] == outputs[1]
    report = check_gcode(outputs[0].decode("ascii"), limits_from_job(load_job(job_path)))
    assert report.ok, report.errors

    # Same console report as Python, except the line count of the program.
    python = run_cli(
        ["chipload", "run", str(dxf_path), "--job", str(job_path), "-o", str(tmp_path / "py.nc")]
    )
    assert python.returncode == 0, python.stderr
    strip = [line for line in run.stdout.splitlines() if not line.startswith("Wrote ")]
    py_strip = [line for line in python.stdout.splitlines() if not line.startswith("Wrote ")]
    assert strip == py_strip


def test_rust_run_refuses_a_changed_drawing(rust_cli, tmp_path):
    job = json.loads((CORPUS / "jobs/v3/simple_part_tabs.json").read_text())
    job["source"]["sha256"] = "0" * 64
    job_path = tmp_path / "job.json"
    job_path.write_text(json.dumps(job))
    out = tmp_path / "out.nc"
    dxf = CORPUS / "dxf" / "simple_part.dxf"
    run = run_cli([str(rust_cli), "run", str(dxf), "--job", str(job_path), "-o", str(out)])
    assert run.returncode == 1
    assert "does not match the job's source.sha256" in run.stderr
    assert not out.exists()


@pytest.mark.parametrize("dxf", ["simple_part.dxf", "messy_2.dxf"])
def test_inspect_prints_the_same_text(rust_cli, dxf):
    path = CORPUS / "dxf" / dxf
    rust = run_cli([str(rust_cli), "inspect", str(path), "--units", "mm"])
    python = run_cli(["chipload", "inspect", str(path), "--units", "mm"])
    assert rust.returncode == python.returncode == 0
    assert rust.stdout == python.stdout
