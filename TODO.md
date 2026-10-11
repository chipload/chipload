# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

- **GUI pass:** make the GUI a little more attractive and intuitive (proposal approved
  2026-10-11, all eight points as recommended).
  - [x] The report under the canvas: result line, coloured warnings that select their
    operation, a Drawing section.
  - [x] Forms in sections with summaries; operations as cards.
  - [x] A toolbar row (Generate stands out) and a start screen.
  - [x] A canvas legend, an out-of-date notice, View > Theme, a calmer style.
  - [x] Remember the theme between sessions (settings schema 2, decided 2026-10-11).

## Done

- **V-carving, step 1** (docs/specs/v-carve.md): a V-bit follows the centre line of closed
  shapes, as deep as makes the cut as wide as the shape (schema 10, V-bit angle and tip,
  tools.json 2). Next steps, each with its own spec: text typed in the app, tracing
  black-and-white pictures, clearing a flat floor with a second tool.
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

- **Trochoidal vs adaptive:** decide whether trochoidal should get bigger loops that cover
  the pocket's width, or whether adaptive already does what's wanted.
