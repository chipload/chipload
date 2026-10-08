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

Revisit when adding G2/G3 output (smaller files, smoother finish).
