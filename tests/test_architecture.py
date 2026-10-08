"""core must stay headless: no Qt, no UI imports."""

import ast
from pathlib import Path

CORE = Path(__file__).parent.parent / "src" / "chipload" / "core"
FORBIDDEN = ("PySide6", "PyQt5", "PyQt6", "chipload.ui", "chipload.cli")


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
    allowed = (
        "__future__",
        "chipload.core.tolerances",
        "chipload.core.job.models",  # job data only, to derive the limits
        "math",
        "re",
        "dataclasses",
        "typing",
    )
    assert all(name.startswith(allowed) for name in imported), imported


def _crate_dependencies(crate: str) -> set[str]:
    import tomllib

    manifest = Path(__file__).parent.parent / "crates" / crate / "Cargo.toml"
    data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    return set(data.get("dependencies", {}))


def test_rust_checker_is_independent_of_the_generator():
    deps = _crate_dependencies("chipload-check")
    assert not deps & {"chipload-toolpath", "chipload-post", "chipload-dxf"}, deps


def test_rust_core_is_headless():
    deps = _crate_dependencies("chipload-core")
    assert not any(d.startswith(("egui", "eframe", "chipload-")) for d in deps), deps


def test_only_the_gui_crate_uses_egui():
    crates = Path(__file__).parent.parent / "crates"
    for crate in sorted(p.name for p in crates.iterdir() if (p / "Cargo.toml").exists()):
        if crate == "chipload-gui":
            continue
        deps = _crate_dependencies(crate)
        assert not any(d.startswith(("egui", "eframe")) for d in deps), (crate, deps)
