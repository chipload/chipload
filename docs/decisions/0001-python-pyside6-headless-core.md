# 0001: Python + PySide6 with a headless core

Status: accepted

Python core (no Qt imports), PySide6 UI, ezdxf for DXF, Shapely/Clipper for offsetting.
Rationale: developer productivity, reviewability of AI-written code, testable headless engine.
Revisit performance-critical parts (C++/Rust via pybind11/PyO3) only after profiling.
