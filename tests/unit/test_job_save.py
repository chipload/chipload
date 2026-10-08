import json
from pathlib import Path

from chipload.core.job.loader import dump_job, load_job, save_job
from chipload.core.job.models import CURRENT_VERSION

JOBS = Path(__file__).parent.parent / "corpus" / "jobs"


def test_round_trip_keeps_the_job(tmp_path):
    original = load_job(JOBS / "v2" / "simple_part_full.json")
    path = tmp_path / "job.json"
    save_job(original, path)
    reloaded = load_job(path)
    assert reloaded.model_copy(update={"written_by": None}) == original.model_copy(
        update={"written_by": None}
    )


def test_old_version_is_saved_as_current(tmp_path):
    path = tmp_path / "job.json"
    save_job(load_job(JOBS / "v1" / "example_job.json"), path)
    data = json.loads(path.read_text())
    assert data["schema_version"] == CURRENT_VERSION
    assert data["written_by"].startswith("chipload ")
    assert data["machine"]["spindle"] == "cw"


def test_defaults_are_written_explicitly():
    data = json.loads(dump_job(load_job(JOBS / "v1" / "example_job.json")))
    assert data["stock"]["origin"] == "bottom_left"
    assert data["stock"]["z_zero"] == "top"
    assert "max_feed" in data["machine"]


def test_output_is_deterministic():
    job = load_job(JOBS / "v2" / "simple_part_full.json")
    assert dump_job(job) == dump_job(job)
    assert dump_job(job).endswith("}\n")
