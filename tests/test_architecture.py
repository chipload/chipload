"""core must stay headless: no Qt, no UI imports."""

import ast
from pathlib import Path

CORE = Path(__file__).parent.parent / "src" / "chipload" / "core"
FORBIDDEN = ("PySide6", "PyQt5", "PyQt6", "chipload.ui", "chipload.cli", "chipload.licensing")


def test_core_has_no_ui_imports():
    offenders = []
    for path in CORE.rglob("*.py"):
        tree = ast.parse(path.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            names = []
            if isinstance(node, ast.Import):
                names = [a.name for a in node.names]
            elif isinstance(node, ast.ImportFrom) and node.module:
                names = [node.module]
            for name in names:
                if name.startswith(FORBIDDEN):
                    offenders.append(f"{path.relative_to(CORE)}: {name}")
    assert not offenders, offenders
