"""Writing generated programs to disk."""

from __future__ import annotations

import os
from pathlib import Path


def write_text_atomically(path: str | Path, text: str) -> None:
    """Write via a temporary file and rename, so a machine never picks up a half-written
    program. ASCII with LF line endings on every platform, for byte-identical output."""
    target = Path(path)
    tmp = target.with_name(target.name + ".tmp")
    tmp.write_text(text, encoding="ascii", newline="\n")
    os.replace(tmp, target)
