# 0002: Shapely offsets and G1-only output for v0

Status: accepted (2026-10-08)

Profile offsets use Shapely `buffer` (GEOS) with round joins. All curves are flattened to
chords within `ARC_APPROX_TOL_MM`, so v0 emits only G0/G1.

- Simple and well tested; arc-preserving offsets would be a lot of new geometry code.
- GEOS rounds the chord count per round join, so a chord can span 1.5x the nominal angle.
  `GEOS_JOIN_CHORD_FACTOR` compensates, and every offset is verified after the fact
  (distance to contour, side), so a GEOS surprise becomes a `GeometryError`, not a gouge.
- No `pyclipper` yet, so `core/geometry/clipper_scale.py` doesn't exist yet. If Clipper is
  added, integer scaling must still go through that single helper.
- Golden G-code depends on the GEOS version; `uv.lock` pins it.

- Inward offsets near collapse: GEOS's negative `buffer` can wrongly return empty (a convex
  polygon with inscribed radius 22.44 mm gave an empty `buffer(-21)`; 40 of 10 000 random
  cases). Offsetting a closed ring as a whole has the same problem. `erode_polygons` instead
  subtracts the union of per-segment round-capped buffers, which had no wrong result in the
  same cases, at about 20x the cost (milliseconds). Pockets use it; profiles still use
  `buffer`, where the failure can only cause a false "tool does not fit" refusal.

Revisit when adding G2/G3 output (smaller files, smoother finish).
