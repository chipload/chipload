import pytest

from chipload.core.errors import UnsupportedJobVersion
from chipload.core.job.loader import migrate
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
