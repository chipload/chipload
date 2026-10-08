# Spec: v0 profile pipeline (DXF → profile toolpath → GRBL G-code)

Status: **approved 2026-10-08**

The first end-to-end version. Scope is deliberately small: closed-contour profiling only,
straight G1 output, one tool per program, GRBL post. Everything here is reachable from the CLI.

Out of scope for v0 (each gets its own spec later): pocket, drill, tabs, lead-ins, ramps,
G2/G3 output, open-path cutting, tool changes, multiple post-processors, UI.

## 1. Job schema v2

Decisions already made (2026-10-08):

| Field | v2 | Migration from v1 |
|---|---|---|
| `machine.spindle` | required, `"cw"` or `"ccw"` (emitted as M3 / M4) | set to `"cw"` |
| `ProfileOp.plunge_feed` | required, mm/min, > 0 | `feed / 3` |
| `PocketOp.plunge_feed` | required, mm/min, > 0 | `feed / 3` |
| `machine.travel` | optional `{x, y, z}` in mm, default `null` | not touched (optional) |
| `Selector` | must have at least one of `layer` / `handles` | not touched; an empty v1 selector now fails validation |

No v1 file has ever produced G-code, so the migration preserves no existing machining behaviour.
The frozen `tests/corpus/jobs/v1/example_job.json` gets a test: migrates to v2, generates G-code that
matches a golden file.

All feeds are mm/min. All lengths are mm. `depth` is measured down from the stock top.

## 2. DXF import

Input: path + the job's `source.units`. Output: `ImportResult(contours, report)`.

### Units
- `$INSUNITS` = 4 (mm) or 1 (inch): used. If it disagrees with `source.units` → `DxfImportError`. Never pick one silently.
- `$INSUNITS` = 0 or missing: use `source.units` (which the job requires) and record "units taken from job" in the report.
- Any other `$INSUNITS` value (cm, m, feet, ...) → `DxfImportError` in v0.
- All geometry is converted to mm on import.

### Entities
| Entity | Handling |
|---|---|
| LINE | Line |
| ARC, CIRCLE | Arc (circle = full 2π arc) |
| LWPOLYLINE, 2D POLYLINE | Lines + bulge arcs (`ezdxf.math.bulge_to_arc`); the closed flag adds the closing segment |
| SPLINE, ELLIPSE | flattened to Lines within `ARC_APPROX_TOL_MM`; each one is counted in the report |
| INSERT | exploded recursively via `virtual_entities()` with the transform applied; children keep the INSERT's handle for selection; skipped children are reported |
| everything else (TEXT, HATCH, DIMENSION, POINT, 3DFACE, ...) | ignored, **counted per type in the report** |

- **OCS/extrusion:** arcs, circles and polylines with extrusion ≠ (0,0,1) (e.g. mirrored arcs with (0,0,−1)) are converted to WCS. If the entity's plane isn't parallel to XY → reported and ignored.
- **Z:** XY is kept and Z dropped. Any nonzero Z is reported (count and max |Z|).
- Layer names match **case-insensitively** (AutoCAD treats them that way).

### Cleanup (all reported, nothing silently dropped)
- Zero-length primitives (length < `CHAIN_TOL_MM`) are removed.
- Exact duplicates (same primitive either direction, endpoints within `CHAIN_TOL_MM`) are removed.
- Partial overlaps are **not** detected in v0. They produce branching nodes (see below) and are reported there.

### Chaining
- Endpoints within `CHAIN_TOL_MM` are snapped to one node. Clustering is deterministic: primitives are processed in file order.
- Nodes with exactly 2 connections are chained. A node with ≠ 2 connections ends a chain.
- **Closed contour** = a chain that returns to its start with every node of degree 2.
- Everything else is an **open contour**, reported with its endpoints (so gaps are easy to find).
- A closed contour that self-intersects (`shapely.is_valid_reason`) is reported and marked unusable.

### Nesting
For each closed contour, nesting depth = number of other closed contours that strictly contain it.
Even depth = outer boundary, odd = hole. Used for reporting and cut ordering.

### Report
`ImportReport`: units used and their source, per-type counts (imported / flattened / ignored),
removed duplicates and zero-length segments, nonzero-Z count, open contours (layer + endpoints),
invalid contours, and overall bounding box. Both `inspect` and `run` print it.

## 3. Selection

A contour belongs to an operation if all its source entities match the selector:
- `layer`: entity layer equals the selector layer (case-insensitive).
- `handles`: entity handle (or its root INSERT's handle) is in the list.
- If both are given, both must match.

Errors (`SelectorError`, job aborts):
- The selector matches nothing.
- A contour is only partly matched (e.g. its entities are on two layers). That's ambiguous, so we refuse.
- The selector matches an invalid contour.
- The selector matches only open contours.

Open contours that match are **skipped and reported** (decision 2026-10-08): real drawings put
centrelines on the same layer as the geometry to cut, and v0 profiles closed contours only.
Each skipped contour is listed in the run output with its endpoints.

## 4. Profile toolpath

Per selected closed contour. r = tool radius.

### Offset side
`side` is relative to **the selected loop itself**, not to "the part". To cut a hole, select the
hole loop with `side: inside`.
- `outside`: `buffer(+r, join_style=round)`, then the exterior ring(s) only.
- `inside`: `buffer(−r, join_style=round)`, then each resulting polygon's exterior ring.
- `on`: the contour itself.

Arc flattening, for both input arcs and buffer corners: segment angle θ is chosen so the sagitta
r·(1 − cos θ/2) ≤ `ARC_APPROX_TOL_MM`. The chords lie inside the true arc, so outside profiles of
curves can be up to 0.01 mm undersize. That's accepted for v0.

Edge cases:
- **Tool larger than feature** (inside offset is empty) → `GeometryError` naming the contour. Never skipped silently.
- **Inside offset splits into several pieces** (a narrow neck): all pieces are cut, and the report warns that material remains in the neck.
- **Offset result invalid or empty for outside/on** → `GeometryError`.

### Cut direction
Assumes a standard right-hand cutter.

| side | spindle cw, climb | spindle cw, conventional |
|---|---|---|
| outside | clockwise | counter-clockwise |
| inside | counter-clockwise | clockwise |
| on | clockwise (treated as outside) | counter-clockwise |

With `ccw` spindle every entry flips. `on` has material on both sides, so climb/conventional
don't really apply; by decision (2026-10-08) it is treated as outside.

### Start point
The ring vertex with the smallest y, ties broken by smallest x. Deterministic, no lead-in in v0, so it leaves a small witness mark.

### Depth passes
- n = ceil(depth / step_down), with depth/step_down rounded down by `DEPTH_TOL_MM` first so 18/6 gives 3 passes, not 4.
- Equal passes: pass i is cut at depth · i / n (i = 1..n). The last pass is exactly `depth`.
- `depth > stock.thickness` → error in v0 (decision 2026-10-08). An explicit optional overcut field comes later.

### Z levels
- `z_zero: top` → stock top Z = 0, cut Z = −depth.
- `z_zero: bottom` → stock top Z = thickness, cut Z = thickness − depth.
- `safe_z` must be above the stock top (> 0 or > thickness). Otherwise it's an error.

### Move sequence (per contour)
1. G0 Z safe_z (if not already there)
2. G0 X Y to the start point
3. For each pass: G1 Z(pass) at `plunge_feed`, then G1 around the ring back to the start at `feed`.
   The cutter stays down between passes on the same contour because the start and end points are the same.
4. G0 Z safe_z

### Order
- Operations run in job order.
- Within an operation: deepest nesting first (holes before their outer boundaries, so a part isn't
  freed before its holes are cut). Within the same depth, greedy nearest-neighbour from the previous end point,
  with ties broken by start point (y, then x). Starts at (0,0).
- All passes of one contour are finished before the next contour starts.

### Other generation-time errors
- More than one tool used across operations → error (GRBL has no tool changer; tool changes come later).
- Tool has no `rpm` → error.
- A feed above `machine.max_feed` (when set) → error, never clamped silently.
- Operation types `pocket` and `drill` → "not implemented in this version" error.

## 5. Post-processor

`core/post/` turns an abstract toolpath (list of Rapid/Feed moves in mm, plus the header data) into text.

- **Post config** is a persisted format: TOML (stdlib `tomllib`, no new dependency) with
  `schema_version = 1`, a `CURRENT_VERSION`, `MIGRATIONS`, and `load_post_config` like jobs.
  Fields: `name`, `template`, `decimals` (default 3), `file_extension`, `spindle_dwell_s`.
- Shipped post: `core/post/posts/grbl.toml` + `templates/grbl.nc.j2`.
- The Python side does only generic things: modal compression (omit axes and F that haven't changed),
  fixed-decimal formatting, and normalising −0.000 to 0.000. Everything machine-specific is in the template.
- GRBL output: `G21 G90 G17 G94` header, `M3`/`M4 S<rpm>`, `G4 P<dwell>` if dwell > 0,
  the moves, `G0 Z<safe>`, `M5`, `M30`. No timestamps or file names, so output is byte-identical across runs.
- `spindle_dwell_s` in grbl.toml is **3 s** (decision 2026-10-08), since router spindles need time to reach speed.

## 6. Independent G-code checker (`core/sim/gcode_check.py`)

It parses the G-code **text** with its own tokenizer and modal state machine. It imports nothing from
`toolpath/` or `post/`. Limits come from a small `CheckLimits` dataclass that the checker builds from the Job itself.

Errors (any error → `run` writes no file):
- Unknown G/M word (whitelist: G0 G1 G2 G3 G4 G17 G20 G21 G90 G94, M3 M4 M5 M30).
- Motion before `G21`, or `G20` anywhere (unit mismatch).
- `G91` (relative mode isn't produced, so it's treated as an error).
- Motion before `G90` (absolute mode must be explicit).
- Code after `M30`, unparseable text, unbalanced comments, a word repeated on one line,
  or two motion commands on one line.
- G1/G2/G3 with no feed set, or F ≤ 0.
- Cutting motion while the spindle is off.
- G0 that changes X/Y while Z is below `safe_z` (retract before XY travel).
- G0 ending below the stock top (rapid into material).
- Any Z below the stock bottom.
- G2/G3: |start radius − end radius| > `ARC_APPROX_TOL_MM`. These aren't emitted in v0, but the check exists.
- Position outside `machine.travel`, when it's set.

Warnings: `machine.travel` not set (limits not checked); cutting XY outside the stock bounds
expanded by the tool radius; no `M30` at the end.

Travel is checked as the span the program covers on each axis, because the checker only sees
work coordinates and doesn't know where work zero sits on the machine.

## 7. CLI

- `chipload inspect part.dxf [--units mm|inch]`: prints the ImportReport and the contours (id, layer,
  closed/open, nesting depth, bbox). Exit 0, or 1 if the import failed.
- `chipload run part.dxf --job job.json -o out.nc`: load (and migrate) the job, import the DXF,
  select, generate, post, check. The output file is written **only** if everything passes. Exit 0 on success,
  1 on any error (message names the operation or contour), 2 on usage errors.
  The DXF on the command line is what gets used. If `source.sha256` is set and doesn't match → error.

## 8. Tests

- **Unit:** units/`$INSUNITS` combinations; bulge arcs; mirrored-extrusion arc; INSERT with
  rotation and scale; duplicates and zero-length removal; gap → open contour; branching node; nesting.
- **Profile:** passes at the boundaries (depth = step_down, depth = thickness, depth not a multiple, depth > thickness → error);
  tool larger than hole → error; narrow neck split; direction table (signed area of the emitted ring) for all
  side × direction × spindle combinations.
- **Hypothesis:** for random simple polygons, every outside-path vertex is at distance r ± tol from the
  contour and outside it; every inside-path vertex is at distance ≥ r − tol and inside it.
- **Checker:** one hand-written bad program per rule, each flagged; every golden program passes.
- **Golden:** synthetic DXFs in `tests/corpus/dxf/` (made by a committed script in `tools/`) →
  `tests/corpus/expected/*.nc`; `pytest --update-golden` regenerates them.
- **Migration:** the v1 sample migrates and produces the expected golden G-code.
- **CLI:** `run` end to end on a corpus file; with a failing checker, no output file is written.

## 9. Known limitations of v0
G1 only (larger files); no tabs, so small parts can break free on the last pass; plunges straight down
on the contour; one tool; GRBL only; golden files depend on the GEOS version (pinned via `uv.lock`).
