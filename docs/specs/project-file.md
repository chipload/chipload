# Spec: project files with embedded drawings

Status: **approved 2026-10-09** (answers to Q1–Q2 folded in)

Guide: docs/book/src/drawings/update.md.

**Superseded in part (2026-10-10):** jobs from before projects (schema 1-3) are no longer
read, so the external-drawing mode, `handles` selectors, *Embed drawing* and
`chipload import --into` (§1 `external`, §3, D3, D5) were removed in schema v6. Projects
(v4 on) are unchanged.

Today a job file points at a DXF (`source.file`, optionally locked by `source.sha256`), and
every run re-imports it. This spec makes the job file a **project**: it holds the drawing's
geometry itself, so the DXF is only needed to import (and later to update) it. It also gives
geometry a home for the drawing tools planned later (circles, rectangles, polylines).

Decisions (2026-10-09):
- D1. Project files are JSON with the extension **`.chipload`**.
- D2. The project stores the **imported shapes**, not the original DXF.
- D3. A reference to an external DXF remains only so that **old (v3) job files open**; new
  projects always embed their drawing. *Update drawing from DXF* covers re-exported drawings.
- D4. Import-time notes are kept in the project (Q1).
- D5. No legacy command: a project whose drawing is still external can be opened and
  embedded, not run (Q2; the software is unreleased, so there are no users' old jobs to
  carry). Everything that generates G-code works on embedded drawings only.

Out of scope: the drawing tools themselves (a later spec adds shape kinds and editing),
multiple drawings per project, tool libraries.

## 1. Format: job schema version 4

A project is the job format at `schema_version: 4`; there is one versioned format, loaded by
`load_job`, so every older job still loads (migrated) and every frozen sample keeps working.

```json
{
  "schema_version": 4,
  "written_by": "chipload 0.2.0",
  "drawing": {
    "shapes": [
      {"id": 1, "layer": "lower", "dxf_handle": "50", "kind": "path", "closed": true,
       "segments": [
         {"line": {"start": [0.0, 0.0], "end": [100.0, 0.0]}},
         {"arc": {"center": [100.0, 5.0], "radius": 5.0, "start_angle": -1.5707963267948966,
                  "sweep": 3.141592653589793}}
       ]}
    ],
    "next_shape_id": 2,
    "source": {"file": "messy_2.dxf", "sha256": "…", "units": "mm", "units_from": "dxf",
               "imported": {"CIRCLE": 5, "LWPOLYLINE": 17}, "flattened": {}, "ignored": {},
               "issues": [{"severity": "warning", "code": "nonzero_z", "message": "…"}]},
    "external": null
  },
  "stock": {}, "machine": {}, "tools": [], "operations": []
}
```

- **Geometry is always millimetres**, angles radians (CCW positive), as everywhere else.
  Units are resolved once, at import; `drawing.source.units` only records what the DXF was in.
- **Shapes.** v4 has one kind, `path`: an ordered list of `line` / `arc` segments (the core
  primitives, stored exactly; f64 values round-trip exactly through JSON) plus a `closed`
  flag. One shape per imported DXF entity (a block INSERT gives one shape per expanded
  entity). Later specs may add kinds (e.g. `circle`, `rectangle`); an older Chipload refuses a
  file with an unknown kind, which is the safe outcome.
- **IDs.** Every shape has a positive integer `id`, unique in the project and **never
  reused**: `next_shape_id` only grows, so an operation can't silently end up pointing at a
  different shape after a deletion.
- **`dxf_handle`** (optional): the handle of the DXF entity the shape came from (the INSERT's
  handle for block content), for *Update drawing from DXF*. Drawn shapes have none.
- **`dxf_type`** (optional): the DXF entity type it came from (`LINE`, `LWPOLYLINE`, …), so
  messages read as they do for a DXF import.
- **`source`** (optional): provenance of the last import: file name, its sha256, its units and
  where they came from (`units_from`: `dxf` or `user`), the entity counts (`imported`,
  `flattened`, `ignored`), and the import-time issues as `{severity, code, message}` (skipped
  entities, nonzero Z, fit-point splines, …), which can't be recomputed from the shapes.
  No timestamp (determinism).
- **`external`** (legacy only): `{"file", "sha256", "units"}` for a migrated v3 job whose
  drawing isn't embedded yet; then `shapes` is empty. New projects never use it.
- **Selectors** gain `{"shapes": [1, 2]}`: the operation cuts the contours made of exactly
  those shapes (a contour partly selected is an error, as today with handles). `layer`
  selectors stay. `handles` selectors are valid only in a project with an `external` drawing.

## 2. Loading a project
Shapes → primitives → the same chaining, duplicate/zero-length removal, open/closed
classification, nesting and issue report as a DXF import today, run on every load. The file
holds the drawing **as drawn**; repairs are never baked into it. The import report shown for a
project is `source.issues` followed by the recomputed ones (duplicates, zero-length, open,
invalid), in the same order as a DXF import.

Contours are numbered as today (#1, #2, … in chaining order); those numbers are for display
only, never stored in selectors.

**Determinism:** importing a DXF into a project and running it gives byte-identical G-code to
running the v3 job on the DXF: shapes keep the import order and exact primitives. The golden
files must not change.

## 3. Migration v3 → v4 (pure, no I/O)
- `source.{file, sha256, units}` → `drawing.external`; `drawing.shapes` = [], `next_shape_id`
  = 1. Selectors are unchanged (`handles` stay valid in external mode).
- A project in external mode can't be run or planned (D5): the error says to embed it.
- **Embed drawing** (GUI action, CLI `chipload import --into`): the app imports the external
  DXF (I/O happens here, not in the migration), creates the shapes, converts each `handles`
  selector to the `shapes` with those `dxf_handle`s, clears `external`, and records `source`.
  The result is reported operation by operation; nothing is converted silently.
- Frozen v3 samples already exist in `tests/corpus/jobs/v3/`; new v4 samples go into
  `tests/corpus/jobs/v4/`.

## 4. Update drawing from DXF
For a part changed in CAD and exported again. Replaces the **imported** shapes; shapes drawn
in Chipload are kept.
- The new DXF must be in the same units as recorded in `source.units`; otherwise the user is
  asked (never guessed).
- Old imported shapes are matched to new ones by **(dxf_handle, n-th shape with that
  handle)**. A match keeps its id, so operations using it keep working with the new geometry.
  Unmatched new shapes get new ids. Unmatched old shapes are removed.
- An operation that selected a removed shape is **flagged, never fixed silently**: it can't
  run until its geometry is chosen again. Layer selectors simply pick up the new geometry.
- The report lists shapes kept unchanged, changed, added and removed, and every operation
  whose geometry changed or was lost, so the toolpath can be reviewed before cutting.

## 5. Command line (before the GUI, CLAUDE.md rule 5)
- `chipload import part.dxf [--units mm|inch] -o part.chipload`: a new project with the same
  defaults as the GUI's New job (docs/specs/job-editor.md §6).
- `chipload import --into old.json [--dxf part.dxf] -o part.chipload`: embed a legacy job's
  drawing (§3).
- `chipload update part.chipload --from part.dxf [-o out.chipload]`: §4, prints the report.
- `chipload run part.chipload -o out.nc`, `chipload plan part.chipload` and
  `chipload inspect part.chipload` (inspect also still takes a `.dxf`). There is no form that
  runs a job against a separate DXF (D5).

## 6. GUI
- **Open…** (Ctrl+O) opens `.chipload` projects and legacy `.json` jobs. **Import DXF…**
  creates a new project from a drawing (replaces "Open drawing" + "New job").
- **Save** writes `.chipload`. A legacy job shows a banner *"Drawing not embedded — Embed
  now"*; saving it as `.chipload` embeds it first.
- **File → Update drawing from DXF…** (§4), shown with its report.
- The canvas, selection and operations work as now; selections become `shapes` selectors
  (layer selections stay layers). The sha256 lock and *Re-lock* only apply to legacy jobs.

## 7. Tests
- Round trip: import → save → load gives equal shapes and ids; JSON floats exact.
- Determinism: every corpus job embedded as a project produces its golden G-code unchanged.
- Migration: every frozen v1–v3 sample loads as v4 (external mode); embedding it with its
  DXF converts handles to shapes and gives its golden G-code unchanged. The golden test runs
  embedded v4 samples of the same jobs (`tests/corpus/jobs/v4/`).
- IDs: never reused after deletions; unknown shape kinds and selectors refused.
- Update: unchanged DXF → nothing changes; moved geometry with the same handles → ids kept,
  operations follow; deleted entity → operation flagged; drawn shapes untouched; different
  units → refused without an explicit choice.
- CLI: import, update, run and inspect on projects; legacy run unchanged.

## Decisions on the questions (2026-10-09)
- Q1: keep import-time notes (`source.issues`, counts).
- Q2: no legacy command; external-mode projects must be embedded before they run.
