# CLAUDE.md

Project: **Chipload**, a cross-platform 2.5D CAM application.
Input: DXF (and DWG via conversion). Output: G-code for CNC routers, mills, plasma, laser.
This is open-source software (**GPL-3.0-or-later**, copyright Gustav Näslund) that drives physical machines. Correctness and safety outrank speed and cleverness.

**Migration in progress (ADR 0004):** the app is being rewritten in Rust (`crates/`, egui UI). The Python implementation (`src/chipload/`) is the **reference** until the Rust version passes the full conformance suite; new features go into the specs and Python first unless agreed otherwise. See "Rust" below.

## Architecture rules (non-negotiable)

1. `chipload.core` is **headless**. It must never import Qt, PySide6, or anything from `chipload.ui`. A test enforces this (`tests/test_architecture.py`).
2. Dependency direction: `ui -> cli/app services -> core`. Never the reverse.
3. Every feature must be usable from the CLI before it gets a UI.
4. The UI is thin: it builds a Job, calls core, displays results. No geometry or toolpath logic in `ui/`.
5. Post-processors are **data-driven** (Jinja2 templates + a small config), never hardcoded per machine.

## Conventions

- **Units:** internal geometry and toolpaths are always **millimetres**. Convert only at the edges (DXF import via `$INSUNITS`, UI display, post-processor output). Never guess units silently: if the DXF has no units, require an explicit choice and record it in the job.
- **Coordinates:** right-handed, Y up, angles in **radians** internally, counter-clockwise positive.
- **Tolerances:** all in `core/tolerances.py` (default chaining/joining tolerance 0.001 mm, arc approximation tolerance 0.01 mm). Never write a bare epsilon like `1e-6` in algorithm code. Import the named constant.
- **Floats:** float64 everywhere. Clipper works on integers, so scale through one shared helper (`core/geometry/clipper_scale.py`) and nowhere else.
- **Types:** full type hints, `mypy --strict` on `core/`. Prefer frozen dataclasses for geometry and job models.
- **Determinism:** same input + same job file must produce byte-identical G-code. No timestamps in output unless a template asks for them, no set/dict-order dependence.
- **Style:** `ruff` for lint and format. Small functions, no clever one-liners in geometry code.

## Persisted formats and schema versioning

Anything saved to disk and read back later (job files, tool libraries, machine profiles, post-processor configs, app settings) is a **persisted format**. Users keep these files for years, so each one is versioned from the first commit.

- Every persisted file has a top-level integer `schema_version`. Also record `written_by` (e.g. `"chipload 1.4.2"`) to help debug customer files.
- Models are Pydantic classes in `core/job/`. Each format has a `CURRENT_VERSION` constant and a `MIGRATIONS` dict mapping `N -> function(data: dict) -> dict` that upgrades version N to N+1.
- Loading always goes through one function per format (e.g. `load_job`). It must:
  - **Older version:** apply migrations step by step to the current version, then validate.
  - **Current version:** validate and load.
  - **Missing or newer-than-known version:** raise a typed error (`UnsupportedJobVersion`) and refuse to load. Never guess.
- Validation code only ever targets the latest format. Old formats are handled solely by migrations.
- **Bump the version** whenever a change would make older code misread a file: rename, removal, new required field, or changed meaning/units of a value. Adding an optional field with a safe default does not need a bump.
- **Every schema change ships in the same commit as:**
  1. the version bump,
  2. the migration function,
  3. a frozen sample file of the old version in `tests/corpus/jobs/v<N>/`,
  4. a test that the sample migrates and still produces the expected golden G-code.
- Never edit or delete a frozen old-version sample. Add a new one instead.
- Migrations must be pure functions (no I/O, no clock) and must not silently change machining behaviour. If a migration cannot preserve behaviour exactly, stop and ask.
- Saving always writes the current version. Never write an older format.
- Treat missing units, sides, directions, or depths in an old file as a migration decision to surface, not something to default quietly.

## Commands

```
uv sync --all-extras          # install deps incl. dev tools and PySide6 (or: pip install -e ".[dev,ui]")
                             # plain `uv sync` removes pytest/ruff/mypy; pytest-qt needs PySide6
pytest                       # all tests
pytest tests/unit -q         # fast loop
pytest tests/golden          # G-code regression
pytest --update-golden       # ONLY when a change is intended; review the diff
ruff check . && ruff format --check .
mypy src/chipload/core
chipload run part.dxf --job job.json -o out.nc  # headless end to end
chipload inspect part.dxf                       # dump entities, units, issues
QT_QPA_PLATFORM=offscreen pytest tests/ui       # GUI smoke tests

cargo test --workspace                           # Rust tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo deny check licenses bans sources            # GPL-3.0 compatibility of every crate
cargo run -p chipload-cli -- check out.nc --job job.json   # Rust checker on any G-code
```

## Rust

- Workspace in `crates/`, one crate per layer. The architecture rules above become crate dependencies: `chipload-core` has no I/O and no GUI; `chipload-check` must never depend on `chipload-toolpath` or `chipload-post` (a pytest in `tests/test_architecture.py` enforces it); only `chipload-gui` may depend on egui.
- Same conventions as Python: millimetres, radians, f64, named tolerances (`chipload_core::tolerances`), typed errors (`thiserror`), no `unwrap()`/`expect()` in library code, `#![forbid(unsafe_code)]`.
- Determinism: no `HashMap`/`HashSet` iteration in anything that reaches output (use `BTreeMap`/`IndexMap`); explicit number formatting.
- Job files: the same JSON schema and versions as Python; Rust must load every frozen sample in `tests/corpus/jobs/`.
- Conformance: the shared corpus (`tests/corpus/`) and the specs in `docs/specs/` are the contract. Toolpaths are compared to the Python output geometrically (same order, same Z, XY within tolerance) until the Rust output replaces the golden files.

Run the relevant tests after every change. Do not report a task done until they pass.

## Workflow expectations

- Work in **small steps**, one feature or fix per commit, with a clear message.
- For any new toolpath operation or non-trivial algorithm: write a short spec in `docs/specs/<name>.md` first (inputs, outputs, edge cases, what "correct" means), get it reviewed, then write tests, then implement.
- Prefer tests first for geometry. Add a Hypothesis property test where an invariant exists (e.g. offset contour never crosses the original, pocket stays inside boundary).
- **Verify APIs against the installed library version** (`pip show`, read the source or `help()`), don't rely on memory. This matters for ezdxf, Shapely, and Clipper bindings.
- If something is ambiguous (units, offset side, climb vs conventional, machine behaviour), **stop and ask** instead of choosing silently.
- Do not add dependencies without stating the license and why it's needed. Every dependency must be compatible with **GPL-3.0-or-later**. Allowed: MIT, BSD-2/3-Clause, ISC, Zlib, Apache-2.0, MPL-2.0, LGPL-2.1-or-later/3.0, GPL-2.0-or-later, GPL-3.0(-or-later), Unicode-3.0. **Forbidden:** GPL-2.0-only (incompatible), proprietary, unknown or no license. **Needs explicit approval:** AGPL-3.0 (adds network obligations) and anything not listed. For Rust, `deny.toml` encodes this list.
- Keep `docs/decisions/` updated with a short ADR for any significant architectural or library decision.

## Safety rules for toolpath and G-code code

These bugs break tools, stock, and machines. Treat them as high-risk:

- Offset side (inside/outside), climb vs conventional, and spindle direction must be explicit in the job model and covered by tests.
- Never emit a rapid (G0) move below the safe height while stock lies in the path. Rapid-through-material is tested by `sim/gcode_check.py`.
- Every generated program must pass the independent checker in `core/sim/gcode_check.py` (stays inside machine limits, arc endpoints consistent with centre/radius, no feed of zero, no unit mismatch, retract before XY travel). The checker must **not** reuse the generator's code paths.
- Arc output (G2/G3) must be validated: start/end radius mismatch beyond tolerance is an error.
- Changing post-processor output or any golden file requires an explicit note in the commit message explaining why.
- Tabs, lead-ins, ramps, and multi-pass depths must be tested at boundaries (zero, equal to stock thickness, tool larger than feature).

## DXF handling notes

- Use `ezdxf`. Treat all input as hostile and messy: duplicate entities, gaps, overlaps, zero-length segments, tiny arcs, splines, ellipses, bulged polylines, blocks/inserts with transforms, mixed layers, 3D-coordinates with nonzero Z.
- Pipeline: read -> flatten blocks/transforms -> convert to Line/Arc primitives (approximate splines and ellipses within tolerance) -> remove duplicates/zero-length -> chain into contours with tolerance -> classify closed/open -> nesting (outer vs holes) -> report issues.
- Never silently drop geometry. Anything ignored or repaired is reported in an `ImportReport` that the CLI and UI both show.
- DWG: convert to DXF through the ODA File Converter wrapper in `core/io/dwg_convert.py`. Do not bundle or redistribute it (proprietary). Being GPL-3.0, the project may now use LibreDWG (GPL-3.0-or-later) instead; that's a separate decision with its own ADR.

## Test corpus

- `tests/corpus/dxf/` holds real and synthetic DXFs; `tests/corpus/expected/` holds expected import reports and golden G-code.
- `tests/corpus/jobs/v<N>/` holds frozen job files for every past schema version (see schema versioning above).
- When a user-reported file breaks something, add it (anonymised) to the corpus **with a failing test first**, then fix.

## Licensing and packaging

- The project is GPL-3.0-or-later (`LICENSE`). New Rust source files start with `// SPDX-License-Identifier: GPL-3.0-or-later`. Contributions are accepted under the same license with a DCO sign-off (`CONTRIBUTING.md`).
- Packaging scripts live in `packaging/` (Nuitka/PyInstaller, signing, notarization). Don't hardcode secrets or certificate paths. Read them from environment variables.

## Things to avoid

- No global mutable state in `core`.
- No `print` in library code; use `logging`.
- No broad `except Exception` that hides geometry errors. Raise typed errors from `core/errors.py`.
- No network calls from core.
- Don't "fix" a failing golden test by regenerating it without understanding the diff.
- Don't change a persisted format without a version bump and migration. Don't add a new persisted format without `schema_version`.
