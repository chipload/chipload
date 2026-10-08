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


def test_gcode_checker_is_independent_of_the_generator():
    """The checker must not reuse generator code paths, or it can't catch their bugs."""
    path = CORE / "sim" / "gcode_check.py"
    tree = ast.parse(path.read_text(encoding="utf-8"))
    imported = [
        node.module for node in ast.walk(tree) if isinstance(node, ast.ImportFrom) and node.module
    ]
    allowed = ("__future__", "chipload.core.tolerances", "math", "re", "dataclasses", "typing")
    assert all(name.startswith(allowed) for name in imported), imported
