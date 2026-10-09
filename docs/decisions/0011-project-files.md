# 0011: Projects embed their drawing (job schema v4, `.chipload`)

Status: accepted (2026-10-09). Spec: docs/specs/project-file.md.

A job file used to point at a DXF that was re-imported on every run. It now holds the
drawing itself: one shape per imported entity (lines and arcs in mm, exact f64), with stable
ids that operations select. The DXF is only needed to import a drawing or to update it after
a change in CAD.

## Why
- One self-contained file reproduces the G-code; no lost or changed DXF, no sha256 lock.
- Shape ids are owned by Chipload and survive re-exports (matched by DXF handle on update).
- Built-in drawing tools (planned) need a place for geometry.

## How
- Job schema version 4: `drawing.{shapes, next_shape_id, source, external}`; selectors gain
  `shapes`. Ids are never reused.
- The project stores shapes as drawn; chaining, repairs and the report are recomputed on
  load by the same code that imports a DXF (`import_dxf` itself goes DXF -> shapes ->
  contours), so a project and its DXF give identical G-code. The golden files did not change.
- Import-time facts that can't be recomputed (units and where they came from, entity counts,
  skipped entities, nonzero Z, …) are kept in `drawing.source`.
- The v3 -> v4 migration is pure, so an older job becomes a project with an `external`
  drawing reference. It opens, but runs only after *Embed drawing* /
  `chipload import --into` converts it (handle selectors -> shapes). There is no command
  that runs a job against a separate DXF any more (the software is unreleased).
- `serde_json` is built with `float_roundtrip`: its default float parser is not exact in the
  last bit (30 % of random f64 values changed in a test), and a project must reload exactly
  the geometry it saved.
