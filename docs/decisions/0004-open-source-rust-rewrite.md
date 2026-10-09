# 0004: Open source under GPL-3.0-or-later, rewrite in Rust with egui

Status: accepted (2026-10-09)

## Decision

- Chipload becomes open source under **GPL-3.0-or-later**, copyright Gustav Näslund.
  Contributions under the same license with a DCO sign-off, no CLA (so no dual licensing).
- The application is rewritten in **Rust**, with an **egui** GUI, in the same repository
  (`crates/`), side by side with the Python implementation until feature parity.

## Why

- GPL keeps derivatives open and lets the project use GPL libraries (e.g. LibreDWG).
- Rust: one small native binary per platform (no Python runtime to package), strong typing
  for safety-critical geometry, and good libraries for arc-aware offsetting.
- egui: an immediate-mode canvas suits a toolpath viewer, it's pure Rust, and the same code
  can target the browser (WebAssembly).
- Same repo: both implementations share `tests/corpus/` and `docs/specs/`, so conformance
  testing needs no syncing.

## How (migration order)

1. The Python version, the specs and the corpus are the reference.
2. Port in this order, each step passing the shared corpus: job models + migrations,
   independent G-code checker, DXF import, profile, tabs, pocket, post-processor + CLI, GUI.
3. Until Rust output replaces the golden files, toolpaths are compared to Python's
   geometrically (same order and Z, XY within tolerance), because the offset library differs.
4. Python is removed once Rust passes everything and the GUI is feature-equal.

## Consequences

- Dependency rules change from "no GPL" to "GPL-3.0-compatible only" (CLAUDE.md, `deny.toml`).
- Two implementations exist for a while; new features need agreement on where they go first.
- Each crate choice beyond the basics (offsetting, DXF parsing) gets its own ADR after checking
  license and maintenance.
- No code of conduct or security contact address yet (decision 2026-10-09). Revisit before
  the repository goes public.
- Libraries: `dxf` for reading DXF (ADR 0005), `cavalier_contours` for offsetting and polygon
  queries (ADR 0006).
