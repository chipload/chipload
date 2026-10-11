# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

(nothing)

## Done

- **Trochoidal vs adaptive:** dropped 2026-10-11; adaptive covers it, trochoidal stays as
  it is.
- **GUI pass** (approved 2026-10-11): report under the canvas, forms in sections, operations
  as cards, a toolbar row and start screen, a canvas legend, View > Theme (remembered).
- **V-carving, step 1** (docs/specs/v-carve.md): a V-bit follows the centre line of closed
  shapes, as deep as makes the cut as wide as the shape (schema 10, V-bit angle and tip,
  tools.json 2). The next steps are under Later.
- **Placing the imported drawing** (docs/specs/drawing-placement.md): move it by an offset or
  align it with the stock, kept through updates from the DXF (schema 8, `chipload place`).
- **Turning the imported drawing in 90° steps** (docs/specs/drawing-rotation.md): about its
  centre, exact (schema 9, `chipload place --rotate`, GUI ⟲ ⟳).
- **Start points, lead-in/out, helical entry** (docs/specs/helix-and-leads.md), in 0.9.0.
- **Offset retry** (docs/specs/offset-retry.md): a pocket or profile whose offsets hit a
  numerical coincidence is retried with offsets moved by 1e-6 mm instead of refused.

## Later

- **A tool per operation** instead of one tool per project: each operation picks its own
  tool, with a tool change between them (user request 2026-10-11).
- **Several DXFs in one project:** import more than one drawing into a project, each with
  its own source and placement (user request 2026-10-11).
- **Scale the imported drawing**, as well as move and turn it (user request 2026-10-11).
- **Import SVG** as well as DXF, working the same way: units, contours, the import report,
  update from a new version, placement (user request 2026-10-11).
- **Voronoi elsewhere:** investigate whether the segment Voronoi diagram (boostvoronoi, ADR
  0015) can make other parts faster or more robust: pocket clearing and adaptive, material
  left, narrow-part checks, distance queries (user request 2026-10-11).
- **V-carve: text typed in the app** (docs/specs/v-carve.md, step 2): a text shape with a
  font, size and position, its outlines V-carved. Needs a font decision (bundle an OFL font,
  or use the system's fonts) and a schema bump. Not prioritised (2026-10-11).
- **V-carve: tracing a black-and-white picture** (step 3): outlines from a bitmap, with the
  pixel size in mm chosen explicitly. Not prioritised.
- **V-carve: a flat floor** for strokes wider than the bit carves: the V-bit cuts the walls,
  an endmill clears the floor. Needs a tool per operation. Not prioritised.
