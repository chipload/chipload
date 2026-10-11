# 0015: boostvoronoi for the V-carve centre line

Status: accepted (2026-10-11). Spec: docs/specs/v-carve.md.

## Context
V-carving follows the medial axis (centre line) of closed shapes. The medial axis is the part
of the Voronoi diagram of the outline's segments that lies inside the shape. That diagram is
hard to compute robustly: segments meet at shared ends, run parallel or nearly touch, and
floating-point predicates get those cases wrong.

## Decision
Use the `boostvoronoi` crate (0.12.1, **BSL-1.0**, allowed since 2026-10-09; its dependencies
are MIT or Apache-2.0) in `chipload-toolpath`. It is a Rust port of Boost.Polygon's segment
Voronoi, with exact predicates on integer input.

- The outline is split into chords (arcs within a quarter of `VCARVE_TOL_MM`) and snapped to
  an integer grid of `VORONOI_GRID_MM` (0.0001 mm, so ±214 m fits an `i32`).
- The diagram only says **where** the tool goes. How wide the cut is at each point is the
  exact distance to the real outline (arcs, not chords). Every move is verified against that
  distance before G-code is made, as offsets are (ADR 0006).
- The library runs under `catch_unwind`; a panic or an error becomes a geometry error naming
  the contours.

## Consequences
- No hand-written Voronoi code to get right. The library needs segments that don't cross
  except at their ends; contours the import marks invalid are already refused before they
  get there.
- Like `cavalier_contours` (ADR 0012), it has `debug_assert!`s that only run in debug
  builds. So far only self-crossing input has tripped them; if valid input does, the same
  choice as ADR 0012 applies.
- If the library ever fails on real drawings, our own implementation can replace it behind
  the same function (`vcarve::medial_axis`); the verification and the tests stay.
