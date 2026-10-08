"""Rust and Python must use the same tolerance values (CLAUDE.md: named constants)."""

import re
from pathlib import Path

from chipload.core import tolerances

RUST = Path(__file__).parent.parent.parent / "crates" / "chipload-core" / "src" / "tolerances.rs"


def test_tolerances_match():
    rust = {
        name: float(value)
        for name, value in re.findall(r"pub const (\w+): f64 = ([0-9.e+-]+);", RUST.read_text())
    }
    python = {name: getattr(tolerances, name) for name in dir(tolerances) if name.isupper()}
    assert rust == python
