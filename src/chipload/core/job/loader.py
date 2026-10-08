"""Load jobs: migrate older versions, refuse unknown or newer ones."""
from __future__ import annotations

import json
from collections.abc import Callable
from pathlib import Path
from typing import Any

from chipload.core.errors import UnsupportedJobVersion
from chipload.core.job.models import CURRENT_VERSION, Job

# Map N -> function upgrading version N data to N+1. Pure functions only.
# Each must set data["schema_version"] = N + 1.
MIGRATIONS: dict[int, Callable[[dict[str, Any]], dict[str, Any]]] = {}


def migrate(data: dict[str, Any]) -> dict[str, Any]:
    version = data.get("schema_version")
    if not isinstance(version, int) or version < 1 or version > CURRENT_VERSION:
        raise UnsupportedJobVersion(version, CURRENT_VERSION)
    while version < CURRENT_VERSION:
        data = MIGRATIONS[version](data)
        version = data["schema_version"]
    return data


def load_job(path: str | Path) -> Job:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    return Job.model_validate(migrate(data))
