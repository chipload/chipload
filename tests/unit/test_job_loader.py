import json
from pathlib import Path

import pytest

from chipload.core.errors import JobFileError, UnsupportedJobVersion
from chipload.core.job.loader import MIGRATIONS, load_job, migrate
from chipload.core.job.models import CURRENT_VERSION


def test_missing_version_rejected():
    with pytest.raises(UnsupportedJobVersion):
        migrate({})


def test_newer_version_rejected():
    with pytest.raises(UnsupportedJobVersion):
        migrate({"schema_version": CURRENT_VERSION + 1})


def test_current_version_passes_through():
    data = {"schema_version": CURRENT_VERSION}
    assert migrate(data) == data


def test_boolean_version_rejected():
    with pytest.raises(UnsupportedJobVersion):
        migrate({"schema_version": True})


def test_string_version_rejected():
    with pytest.raises(UnsupportedJobVersion):
        migrate({"schema_version": "1"})


def test_v1_to_v2_is_pure_and_sets_new_fields():
    v1 = {
        "schema_version": 1,
        "machine": {"post": "grbl", "safe_z": 5},
        "operations": [
            {"type": "profile", "feed": 1500},
            {"type": "pocket", "feed": 900},
            {"type": "drill"},
        ],
    }
    before = json.dumps(v1, sort_keys=True)
    v2 = MIGRATIONS[1](v1)
    assert json.dumps(v1, sort_keys=True) == before  # input untouched
    assert v2["schema_version"] == 2
    assert v2["machine"]["spindle"] == "cw"
    assert v2["operations"][0]["plunge_feed"] == 500
    assert v2["operations"][1]["plunge_feed"] == 300
    assert "plunge_feed" not in v2["operations"][2]


def test_v2_to_v3_makes_pocket_direction_explicit():
    v2 = {
        "schema_version": 2,
        "operations": [{"type": "pocket", "feed": 900}, {"type": "profile", "direction": "x"}],
    }
    before = json.dumps(v2, sort_keys=True)
    v3 = MIGRATIONS[2](v2)
    assert json.dumps(v2, sort_keys=True) == before
    assert v3["schema_version"] == 3
    assert v3["operations"][0]["direction"] == "climb"
    assert "keep_tool_down" not in v3["operations"][0]  # optional, safe default (retract)
    assert v3["operations"][1]["direction"] == "x"  # profiles untouched


def test_v1_migrates_all_the_way_to_current():
    data = migrate({"schema_version": 1, "operations": [{"type": "pocket", "feed": 900}]})
    assert data["schema_version"] == CURRENT_VERSION
    assert data["operations"][0]["direction"] == "climb"


def test_malformed_operations_do_not_crash_migration():
    data = migrate({"schema_version": 1, "operations": ["junk", {"type": "profile"}]})
    assert data["schema_version"] == CURRENT_VERSION


def test_invalid_job_file_is_a_typed_error(tmp_path):
    path = tmp_path / "job.json"
    path.write_text("{not json")
    with pytest.raises(JobFileError):
        load_job(path)
    path.write_text(json.dumps({"schema_version": CURRENT_VERSION}))
    with pytest.raises(JobFileError):
        load_job(path)


def test_pockets_cannot_have_tabs(tmp_path):
    """Tabs belong to profiles only; a pocket frees nothing, so there is nothing to hold."""
    jobs = Path(__file__).parent.parent / "corpus" / "jobs" / "v3"
    data = json.loads((jobs / "messy_2_lower.json").read_text())
    assert data["operations"][0]["type"] == "pocket"
    data["operations"][0]["tabs"] = {"count": 4, "width": 8, "height": 3}
    path = tmp_path / "job.json"
    path.write_text(json.dumps(data))
    with pytest.raises(JobFileError, match="tabs"):
        load_job(path)
