import pytest

from gncam.core.errors import UnsupportedJobVersion
from gncam.core.job.loader import migrate
from gncam.core.job.models import CURRENT_VERSION


def test_missing_version_rejected():
    with pytest.raises(UnsupportedJobVersion):
        migrate({})


def test_newer_version_rejected():
    with pytest.raises(UnsupportedJobVersion):
        migrate({"schema_version": CURRENT_VERSION + 1})


def test_current_version_passes_through():
    data = {"schema_version": CURRENT_VERSION}
    assert migrate(data) == data
