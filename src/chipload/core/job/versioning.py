"""Shared schema-version handling for every persisted format (see CLAUDE.md)."""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

from chipload.core.errors import UnsupportedSchemaVersion

# A migration upgrades version N data to N + 1. Pure: no I/O, no clock.
Migration = Callable[[dict[str, Any]], dict[str, Any]]


def migrate_to_current(
    data: dict[str, Any],
    current: int,
    migrations: Mapping[int, Migration],
    error: type[UnsupportedSchemaVersion],
) -> dict[str, Any]:
    """Apply migrations step by step. Missing, malformed or newer versions are refused."""
    version = data.get("schema_version")
    # bool is a subclass of int; `"schema_version": true` must not load as version 1.
    if isinstance(version, bool) or not isinstance(version, int):
        raise error(version, current)
    if version < 1 or version > current:
        raise error(version, current)
    while version < current:
        data = migrations[version](data)
        if data.get("schema_version") != version + 1:
            raise AssertionError(f"Migration from v{version} did not set schema_version")
        version += 1
    return data
