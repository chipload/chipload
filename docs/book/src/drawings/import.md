# Importing a DXF

*File > Import DXF, SVG or STL…* (Ctrl+I), or `chipload import part.dxf -o part.chipload`, makes a new
project from a DXF. *File > Open…* also opens a `.dxf` directly. An STL file picked here starts a project with that [3D model](models.md). `chipload inspect part.dxf`
shows what Chipload sees in a DXF without making a project.

**Dropping files** on the window:
- **A `.chipload` project** opens (after asking about unsaved changes).
- **DXFs, SVGs and pictures** are added to the open project: each DXF or [SVG](svg.md)
  as [another drawing](several.md), each picture through its [image](images.md) window,
  one after the other. With no project open, the first starts one.
- **The hint** while you drag says which will happen.

## What is read

- **Read:** lines, arcs, circles, polylines with arcs (bulges), splines and ellipses
  (turned into arcs within 0.01 mm), and blocks inserted with a move, turn or scale.
- **Not supported:** DWG files. Save as DXF in your CAD program.
- **Layers** are kept, and you can select by them: right-click a shape on the canvas
  selects its whole layer. In DXF, layer names don't care about upper or lower case.
- **Closed shapes are read as drawn:** a closed polyline, a circle or a closed spline is one
  closed contour, even where other shapes touch it or share its edges. A pocket drawn
  against the part's outline leaves both whole.
- **Joining:** loose lines and arcs whose ends meet (within 0.001 mm) are joined into
  **contours**, only with others on the same layer:
  - **closed**, for profiles, pockets and V-carving;
  - **open**, for follow path; profiles and pockets skip them with a note.
- **Outer and hole:** each closed contour is an *outer* (outline) or a *hole* (inside
  another), which sets the side of a new profile (and an outline with its holes gets two
  profiles).
- **The project keeps the geometry itself.** The DXF isn't needed afterwards, unless the
  drawing changes ([Updating from a new DXF](update.md)).

## The import report

Chipload reads the drawing as drawn: as little repair as possible, and nothing dropped
without saying so. The report under the canvas (and from `inspect`) lists:

| Issue | Meaning |
|---|---|
| `duplicate` | a line, arc or shape drawn twice on top of itself on the same layer; one is kept. On different layers both are kept |
| `zero_length` | a line or arc of no length; left out |
| `nonzero_z` | the drawing has Z values (3D); they are ignored, the drawing is read flat |
| `entity_skipped` | an entity Chipload doesn't read (text, dimensions, hatches, …), or one with a coordinate beyond 1 km or not a number, which only a broken file has |
| `block_depth` | blocks nested too deep; the inner ones are skipped |
| `open_contour` | a contour that doesn't close: profiles and pockets skip it |
| `branch_point` | three or more loose lines on one layer meet at a point, so the contour's path is ambiguous |
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

## The new project dialog

Before the project is made, a dialog asks what it starts with. It opens for *File > New
project* (Ctrl+N), *Import* and a drawing dropped on an empty window:

- **Material,** from your [materials](../library/materials.md). A material with a thickness
  of its own fills in the thickness.
- **Thickness** of the board: depths are measured from its top, and cuts through it follow
  it.
- **Board:** *Fit the drawing* with a margin (it follows the drawing as it changes), or a
  **Size**. Either way it is centred on the drawing.
- **Z zero:** on the top of the board or on the spoilboard.
- **Machine,** from your [machine profiles](../library/machines.md), with its post, G-code
  units and tool changes shown under it.
- **Tools:** tick the [library](../library/tools.md) tools the project uses; the search
  narrows a long list. Each keeps its library number; when two have the same, the second
  takes the next free one, and the T column shows it.

**Enter** creates the project, **Escape** cancels. Everything is remembered for the next
project, which is often the same, so New project, Enter is enough.

The first time, your library's defaults are offered: the default machine, material and tool,
the material's thickness (else the default thickness, else 10 mm), the board fitted with a
10 mm margin, and Z zero on top. Without a library you get GRBL, a safe Z of 5 mm, a
travel of 800 × 800 × 100 mm (set your machine's own under **Machine**, so the checker
keeps every program inside it), and a centre-cutting 6 mm endmill at 18 000 rpm. New
pockets go down round a helix, and new follow paths and chamfers ramp in, whatever the
tool; only a V-bit plunges. Choose *Plunge* in an operation's form for a straight
plunge.

On the command line, `chipload new` and `chipload import` take the same choices and
otherwise use your library's defaults:

```
chipload import part.dxf -o part.chipload --material Plywood --thickness 18 \
    --fit 10 --z-zero top --machine "Shapeoko 4" --tools "6 mm end mill,V 60"
chipload new -o sign.chipload --size 600x300 --tools ""      # no tools yet
```
