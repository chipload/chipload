# 0006: Offsetting and polygon queries with `cavalier_contours`

Status: accepted (2026-10-09)

The Rust port uses `cavalier_contours` 0.9.0 (jbuckmccready/cavalier_contours, MIT OR
Apache-2.0; actively released in 2026) for polyline geometry. It replaces Shapely/GEOS from
the Python version (ADR 0002).

## Why
- Polylines with **bulges** (arcs) are first-class: offsets keep true arcs, which gives G2/G3
  output later instead of G1 chords (smaller programs, smoother finish).
- Parallel offset of polylines and of shapes with islands (`Shape::parallel_offset`), which is
  what profiles and pockets need.
- Polygon queries we need anyway: `scan_for_self_intersect`, `winding_number`, `contains`,
  `area`, `extents`, boolean operations.
- One geometry dependency instead of two (no `geo`).

## Conditions
- Everything stays behind `chipload-core`/`chipload-toolpath` APIs so the library could be
  swapped.
- Every offset is still verified after the fact (distance to the contour, correct side), as in
  the Python version: library bugs must become errors, never gouges.
- The property tests (no gouge, pocket coverage, tabs) are ported and run against it.
- Toolpaths will differ slightly from Shapely's, so profile/pocket conformance with the Python
  reference is geometric, and new golden files are reviewed before they replace the old ones.
