"""Load and save jobs. Loading migrates older versions and refuses unknown or newer ones;
saving always writes the current version."""

from __future__ import annotations

import copy
import json
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path
from typing import Any

from pydantic import ValidationError

from chipload.core.errors import JobFileError, UnsupportedJobVersion
from chipload.core.io.output import write_text_atomically
from chipload.core.job.models import CURRENT_VERSION, Job
from chipload.core.job.versioning import Migration, migrate_to_current

# Map N -> function upgrading version N data to N+1. Pure functions only.
# Each must set data["schema_version"] = N + 1.
MIGRATIONS: dict[int, Migration] = {}


def _v1_to_v2(data: dict[str, Any]) -> dict[str, Any]:
    """v2 adds spindle direction and plunge feed.

    v1 never produced G-code, so there is no machining behaviour to preserve. Values chosen
    by decision 2026-10-08: spindle "cw" (M3), plunge_feed = feed / 3.
    """
    upgraded = copy.deepcopy(data)
    machine = upgraded.get("machine")
    if isinstance(machine, dict):
        machine["spindle"] = "cw"
    operations = upgraded.get("operations")
    for op in operations if isinstance(operations, list) else []:
        # Malformed entries are left alone; validation after migration reports them.
        if not isinstance(op, dict) or op.get("type") not in ("profile", "pocket"):
            continue
        feed = op.get("feed")
        if isinstance(feed, int | float) and not isinstance(feed, bool):
            op["plunge_feed"] = feed / 3
    upgraded["schema_version"] = 2
    return upgraded


def _v2_to_v3(data: dict[str, Any]) -> dict[str, Any]:
    """v3 makes the pocket cutting direction explicit.

    Pocket generation did not exist in v2, so no machining behaviour is preserved or changed.
    Decision 2026-10-08: existing pocket ops become "climb". The new optional fields
    (keep_tool_down, center_cutting) keep their safe defaults.
    """
    upgraded = copy.deepcopy(data)
    operations = upgraded.get("operations")
    for op in operations if isinstance(operations, list) else []:
        if isinstance(op, dict) and op.get("type") == "pocket":
            op["direction"] = "climb"
    upgraded["schema_version"] = 3
    return upgraded


MIGRATIONS[1] = _v1_to_v2
MIGRATIONS[2] = _v2_to_v3


def migrate(data: dict[str, Any]) -> dict[str, Any]:
    return migrate_to_current(data, CURRENT_VERSION, MIGRATIONS, UnsupportedJobVersion)


def load_job(path: str | Path) -> Job:
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise JobFileError(f"Cannot read job file {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise JobFileError(f"Job file {path} must contain a JSON object")
    try:
        return Job.model_validate(migrate(data))
    except ValidationError as exc:
        raise JobFileError(f"Invalid job file {path}: {exc}") from exc


def dump_job(job: Job) -> str:
    """Serialise as the current schema version.

    Every field is written, defaults included, so a default that changes in a later version
    can't silently change what an existing file means. ASCII-only JSON, stable field order.
    """
    current = job.model_copy(
        update={"schema_version": CURRENT_VERSION, "written_by": f"chipload {_app_version()}"}
    )
    return json.dumps(current.model_dump(mode="json"), indent=2) + "\n"


def save_job(job: Job, path: str | Path) -> None:
    write_text_atomically(path, dump_job(job))


def _app_version() -> str:
    try:
        return version("chipload")
    except PackageNotFoundError:
        return "unknown"
