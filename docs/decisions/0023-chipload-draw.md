# 0023: Drawing geometry in its own crate, `chipload-draw`

Status: accepted (2026-10-27). Specs: docs/specs/drawing-roadmap.md, drawing-input.md.

## Context
Chipload is to draw the parts it cuts without a CAD program (ADR 0017 as amended
2026-10-27). That brings geometry that is neither a toolpath nor an import: snapping to
intersections and tangents, selecting with a box, and later trim, fillet, offset and
booleans of drawn shapes. Snapping lived in `chipload-gui/src/draw.rs`, which the
architecture rules keep free of geometry.

## Decision
- **A new crate, `chipload-draw`,** beside `chipload-text` and `chipload-image`: drawing
  geometry on `chipload-core`'s primitives. It depends only on `chipload-core` (and through
  it `cavalier_contours`, for offsets and booleans later). No I/O, no project model; it
  builds for wasm32.
- `chipload-pipeline::shapes` maps project shapes to and from it and applies the results
  to the project (ids, flagged operations), as it does for move, turn and scale.
- The GUI keeps the interaction only: the tool's state, the typed entry, previews, prompts
  and the display grid.

## Consequences
- The architecture test lists the crate with the libraries that must not reach the GUI,
  and the wasm32 check and CI build it.
- Snapping is tested without the GUI, and can grow (intersections, tangents) without the
  GUI growing.
