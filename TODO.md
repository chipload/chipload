# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

(nothing)

## Next

- **Adaptive as spirals and trochoidal channels on the medial axis**
  (docs/specs/adaptive-spiral.md, approved). Stage 1 (round pockets) is on main; stage 2
  (wide areas by spiral, corners and branches by loops, lifts) is on the branch
  `adaptive-stage2-wip`, paused 2026-10-12 for the user's test run. Replaces the fronts
  (adaptive-fronts.md) and the trochoidal strategy. Then rest machining with a smaller
  tool, its own spec.

## Done

- **Rounded corners on the clearing rings of a pocket** (docs/specs/pocket-round-corners.md):
  every ring but the wall ring gets its corners rounded (radius min(k·s, D), less where that
  would leave material), for smoother motion; per pocket, on for new ones (schema 14).
- **Clearer left sidebar** (docs/specs/sidebar-sections.md): the Job row on top, then
  Operations (with + Profile / + Pocket / + V-carve for the selection) and, below a
  draggable split, Shapes (selection, shape editor, layers with counts).
- **Material left lost behind a bay of the tool's reach** (bug found 2026-10-12 by the
  material-left property test): the offset library returned a loop that closes while
  growing as an open polyline, which was dropped; it is now closed (ends within 0.001 mm)
  or an error. Display only, toolpaths were not affected.
- **Import/export of the tool library and machine profiles**
  (docs/specs/library-import-export.md): files in the library's own formats; import adds
  new entries and keeps yours unless asked to replace (`chipload tool export/import`,
  `chipload machine export/import`, GUI Import…/Export… in the library windows).
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

- **Ring corner radius per corner:** a ring now has one radius, set by its sharpest corner;
  rounding each corner by its own angle would let acute shapes (triangles) keep larger
  radii at their other corners (follow-up to docs/specs/pocket-round-corners.md).
- **A list of every shape** in the Shapes section of the sidebar, drawn and imported, to
  click and select (docs/specs/sidebar-sections.md decision 5, user 2026-10-12).
- **Several DXFs in one project:** import more than one drawing into a project, each with
  its own source and placement (user request 2026-10-11).
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
