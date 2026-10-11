# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

(nothing)

## Next

- **Adaptive clearing that divides the area smarter** (user request 2026-10-11, with a
  screenshot): on a rectangle with two round holes the adaptive path wastes time. Short
  zig-zag cuts comb along every wall, there are long link moves across the part, re-entries
  cluster in corners, and one big spiral sits in the middle. Idea: split the pocket by its
  segment Voronoi diagram / medial axis (boostvoronoi, ADR 0015) into channels and open
  areas, and clear each along its centre line with steady engagement, in an order that
  keeps links short. Needs a spec first: what "efficient" is measured as (cut time, air
  time, link length, re-entries), and checks that it stays gouge-free and leaves nothing.

## Done

- **A tool per operation** (docs/specs/tool-per-operation.md): each operation cuts with its
  own tool, in tool groups. Output is one checked file per tool, or one file with `T M6`
  for senders that handle the change (machine setting, project schema 12, machines.json 2).
  CLI `op tool`, `run --tool-change`, `check --tool`; GUI *Tools* section and an operation
  *Tool* box.

- **Scaling the imported drawing** (docs/specs/drawing-scale.md): uniformly, about its
  centre, by a factor or to a width or height; updates re-approximate curves at the new size
  (schema 11, `chipload place --scale/--width/--height`, GUI *Size* rows).
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

- **Several DXFs in one project:** import more than one drawing into a project, each with
  its own source and placement (user request 2026-10-11).
- **Import SVG** as well as DXF, working the same way: units, contours, the import report,
  update from a new version, placement (user request 2026-10-11).
- **Import/export the tool library and machine profiles**, e.g. when moving to a new
  computer or for a backup (user request 2026-10-11).
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
