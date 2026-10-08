"""Post-processor config: a persisted, versioned format (TOML).

Machine-specific output lives in the Jinja2 template the config names; the config only
holds small settings. See docs/specs/v0-profile-pipeline.md section 5.
"""

from __future__ import annotations

import re
import tomllib
from importlib import resources
from pathlib import Path
from typing import Any

from pydantic import BaseModel, ConfigDict, Field, ValidationError, field_validator

from chipload.core.errors import PostConfigError, UnsupportedPostConfigVersion
from chipload.core.job.versioning import Migration, migrate_to_current

CURRENT_VERSION = 1

# Map N -> function upgrading version N data to N+1. Pure functions only.
MIGRATIONS: dict[int, Migration] = {}

_SAFE_NAME = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_.-]*$")


class PostConfig(BaseModel):
    model_config = ConfigDict(frozen=True, extra="forbid")

    schema_version: int = CURRENT_VERSION
    written_by: str | None = None
    name: str
    template: str  # file name in chipload/core/post/templates
    decimals: int = Field(default=3, ge=0, le=6)
    file_extension: str = ".nc"
    spindle_dwell_s: float = Field(ge=0)  # required: a silent 0 s default could cut at low rpm

    @field_validator("template")
    @classmethod
    def _plain_file_name(cls, value: str) -> str:
        if not _SAFE_NAME.match(value):
            raise ValueError(f"template must be a plain file name, got {value!r}")
        return value


def parse_post_config(data: dict[str, Any]) -> PostConfig:
    migrated = migrate_to_current(data, CURRENT_VERSION, MIGRATIONS, UnsupportedPostConfigVersion)
    try:
        return PostConfig.model_validate(migrated)
    except ValidationError as exc:
        raise PostConfigError(f"Invalid post-processor config: {exc}") from exc


def load_post_config(path: str | Path) -> PostConfig:
    try:
        data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise PostConfigError(f"Cannot read post-processor config {path}: {exc}") from exc
    return parse_post_config(data)


def load_builtin_post(name: str) -> PostConfig:
    """Load a post shipped with Chipload, e.g. "grbl"."""
    if not _SAFE_NAME.match(name):
        raise PostConfigError(f"Invalid post-processor name {name!r}")
    resource = resources.files("chipload.core.post").joinpath("posts", f"{name}.toml")
    if not resource.is_file():
        raise PostConfigError(f"Unknown post-processor {name!r}")
    try:
        data = tomllib.loads(resource.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        raise PostConfigError(f"Built-in post-processor {name!r} is broken: {exc}") from exc
    return parse_post_config(data)
