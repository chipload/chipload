# 0009: Python reference removed; Rust is the implementation

Status: accepted (2026-10-09)

The migration of ADR 0004 is complete. The Python implementation (`src/chipload`, its tests,
`pyproject.toml`, `uv.lock`, `tools/make_corpus.py`) is removed. The last commit containing it
is tagged **`python-final`**; check it out to compare against the old behaviour or to rerun the
corpus generator.

## What made it safe
- Every Python test that protected behaviour has a Rust counterpart: job loading, migration
  and saving, DXF import (units, bulges, mirrored arcs, inserts, splines, repairs, nesting),
  selection, profiles, tabs, pockets, posts, the checker, the CLI, the GUI actions, and the
  architecture rules (now checked on `cargo metadata`, indirect dependencies included).
- Before removal, the conformance suite compared the two implementations: identical import
  structure, toolpath order, direction, depths, feeds and pocket structure; geometry within
  3 × `ARC_APPROX_TOL_MM`; byte-identical G-code from the same toolpath; identical CLI text.
- The golden G-code was regenerated from Rust in a separate, reviewed commit; every file
  passed both independent checkers (Python and Rust).

## Consequences
- Specs in `docs/specs/` describe behaviour; their notes on GEOS/Shapely workarounds are
  history from the Python version and marked as such.
- The post-processor files moved to `crates/chipload-post/posts/` and `templates/`.
- Known behaviour differences kept from the port, now the only behaviour: fit-point-only
  splines are refused (ADR 0005); curved rings start at their true lowest point (v0 spec
  §4); the uncut-material count ignores sliver-thin leftover (pocket spec §3).
- Job files are unchanged: same JSON schema and versions, all frozen samples still load.
