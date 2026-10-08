"""Shared fixtures for comparing the Rust port with the Python reference (ADR 0004)."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent.parent


def _cargo() -> str | None:
    found = shutil.which("cargo")
    if found:
        return found
    home = Path(os.path.expanduser("~/.cargo/bin/cargo"))
    return str(home) if home.exists() else None


@pytest.fixture(scope="session")
def rust_cli() -> Path:
    """Build the Rust CLI once per test session; skip if no Rust toolchain is installed."""
    cargo = _cargo()
    if cargo is None:
        pytest.skip("Rust toolchain not installed")
    subprocess.run([cargo, "build", "-q", "-p", "chipload-cli"], cwd=ROOT, check=True)
    return ROOT / "target" / "debug" / ("chipload.exe" if os.name == "nt" else "chipload")
