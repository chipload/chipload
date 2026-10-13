# Importing a DXF

*File > Import DXF…* (Ctrl+I), or `chipload import part.dxf -o part.chipload`, makes a new
project from a DXF. *File > Open…* also opens a `.dxf` directly. `chipload inspect part.dxf`
shows what Chipload sees in a DXF without making a project.

## What is read

- **Read:** lines, arcs, circles, polylines with arcs (bulges), splines and ellipses
  (turned into arcs within 0.01 mm), and blocks inserted with a move, turn or scale.
- **Not supported:** DWG files. Save as DXF in your CAD program.
- **Layers** are kept, and you can select by them: right-click a shape on the canvas
  selects its whole layer. In DXF, layer names don't care about upper or lower case.
- **Joining:** pieces whose ends meet (within 0.001 mm) are joined into **contours**:
  - **closed**, for profiles, pockets and V-carving;
  - **open**, for follow path; profiles and pockets skip them with a note.
- **Outer and hole:** each closed contour is an *outer* (outline) or a *hole* (inside
  another), which sets the default side of a new profile.
- **The project keeps the geometry itself.** The DXF isn't needed afterwards, unless the
  drawing changes ([Updating from a new DXF](update.md)).

## The import report

Chipload reads the drawing as drawn: as little repair as possible, and nothing dropped
without saying so. The report under the canvas (and from `inspect`) lists:

| Issue | Meaning |
|---|---|
| `duplicate` | a line or arc drawn twice on top of itself; one is kept |
| `zero_length` | a line or arc of no length; left out |
| `nonzero_z` | the drawing has Z values (3D); they are ignored, the drawing is read flat |
| `entity_skipped` | an entity Chipload doesn't read (text, dimensions, hatches, …), or one with a coordinate beyond 1 km or not a number, which only a broken file has |
| `block_depth` | blocks nested too deep; the inner ones are skipped |
| `open_contour` | a contour that doesn't close: profiles and pockets skip it |
| `branch_point` | three or more lines meet at a point, so the contour's path is ambiguous |
| `invalid_contour` | a closed contour that crosses or folds back on itself: it can't be cut until fixed |
| `units_chosen` | the DXF didn't say its units; this records your choice |

**Crossing contours are fine.** Two shapes that cross, such as a pocket cutting through a
part's outline to make an opening, are separate shapes, each cut on its own. Chipload never
merges them.

**Centre lines** and other open lines on a cutting layer don't stop a profile or pocket on
that layer: they're skipped, with a note.

## Units

If the DXF doesn't say whether it's in mm or inches, Chipload asks (command line:
`--units mm` or `--units inch`). See [Units and coordinates](../concepts/units.md).

## New project defaults

The stock just covers the drawing, 10 mm thick:
- its bottom-left corner sits at the drawing's origin when the drawing lies right of and
  above the origin;
- otherwise it is centred on the origin.

The machine and tool come from your default [machine profile](../library/machines.md) and
[tool](../library/tools.md) if you have them. Otherwise you get GRBL, a safe Z of 5 mm, and
a 6 mm endmill at 18 000 rpm.
