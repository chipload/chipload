from pathlib import Path

from gncam.core.job.loader import load_job

SAMPLE = Path(__file__).parent.parent / "corpus" / "jobs" / "v1" / "example_job.json"


def test_v1_sample_loads():
    job = load_job(SAMPLE)
    assert job.operations[0].type == "profile"
