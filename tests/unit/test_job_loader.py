import json

import pytest

from chipload.core.errors import JobFileError, UnsupportedJobVersion
from chipload.core.job.loader import load_job, migrate
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


def test_v1_migration_is_pure_and_sets_new_fields():
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
    v2 = migrate(v1)
    assert json.dumps(v1, sort_keys=True) == before  # input untouched
    assert v2["schema_version"] == 2
    assert v2["machine"]["spindle"] == "cw"
    assert v2["operations"][0]["plunge_feed"] == 500
    assert v2["operations"][1]["plunge_feed"] == 300
    assert "plunge_feed" not in v2["operations"][2]


def test_malformed_v1_operations_do_not_crash_migration():
    v2 = migrate({"schema_version": 1, "operations": ["junk", {"type": "profile"}]})
    assert v2["schema_version"] == 2


def test_invalid_job_file_is_a_typed_error(tmp_path):
    path = tmp_path / "job.json"
    path.write_text("{not json")
    with pytest.raises(JobFileError):
        load_job(path)
    path.write_text(json.dumps({"schema_version": CURRENT_VERSION}))
    with pytest.raises(JobFileError):
        load_job(path)
