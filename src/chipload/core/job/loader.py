"""Load jobs: migrate older versions, refuse unknown or newer ones."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from chipload.core.errors import UnsupportedJobVersion
from chipload.core.job.models import CURRENT_VERSION, Job
from chipload.core.job.versioning import Migration, migrate_to_current

# Map N -> function upgrading version N data to N+1. Pure functions only.
# Each must set data["schema_version"] = N + 1.
MIGRATIONS: dict[int, Migration] = {}


def migrate(data: dict[str, Any]) -> dict[str, Any]:
    return migrate_to_current(data, CURRENT_VERSION, MIGRATIONS, UnsupportedJobVersion)


def load_job(path: str | Path) -> Job:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    return Job.model_validate(migrate(data))
