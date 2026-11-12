# Spec: relief carving from an STL model (drop cutter)

Status: **approved 2026-11-12** ("Go ahead!": decisions 1-10 as recommended); step 1 built. The user asked: "we start on the next features, stl
3d model engraving. drop cutter i think it is called", then "Yes go ahead" for this spec and
its ADR (docs/decisions/0031-relief-carving-from-stl.md, which takes this in as an exception to
ADR 0017).

## 1. What it is for

A maker downloads or models an STL file and carves it into a board **from the top**. Typical
jobs:
- a sign with raised, rounded lettering;
- a decorative panel, a rosette or a coin;
- a portrait or a landscape relief;
- a 2.5D part with sloped walls.

What "correct" means:
- every point the tool goes to touches the model, or lies above it, never below, within
  the tolerance (§7);
- the cut stays inside the model's area on the board;
- nothing below the relief's depth is cut;
- the independent checker passes it as any program.

## 2. The model in the project

**Reading.** STL comes in two forms.
- **Binary:** an 80-byte header, a u32 triangle count, then 50 bytes per triangle (normal,
  three corners, a 2-byte attribute).
- **Text:** `solid … facet normal … outer loop vertex … endloop endfacet … endsolid`.

A binary file may also begin with "solid". It is read as binary when its length is exactly
84 + 50 × count, otherwise as text. The stored normals are ignored: each face's direction is
worked out from its corners.

The new core crate `chipload-mesh` reads the bytes (ADR 0031). It does no I/O, like
`chipload-image`.

**Hostile input,** as for every file Chipload reads:
- at most 5 million triangles;
- a size that doesn't match the count is refused;
- NaN or infinite numbers are refused;
- triangles with no area are dropped and counted;
- the reader runs under `catch_unwind`;
- a report says how many triangles there are, how many were dropped, and the size.

**Units.** STL has none. Most files are in mm, some in inches. Following the units rule, the
size is never guessed silently. On import the model is shown with its size in mm ("120 × 80 ×
14 mm as mm") and placed **by the width you give** (§3), so its own unit only matters for
keeping proportions. The record keeps `units: mm | inch` as a reference, chosen on import
and mm by default, and the import report says which was taken (decision 2).

**In the project** (`drawing.models`, like `drawing.images`), each model has:
- `id`, `name`, `file`, `sha256`;
- `data`: the STL itself, base64. The project is self-contained, as for pictures; over
  20 MB of STL the import warns that the project gets large (decision 3);
- `up`: which of the model's axes points up: `z` (default), `y`, `x`, or one of them
  reversed (`-z` …). Many files are modelled Y-up;
- the placement (§3);
- `layer`: the layer its outline goes on (§4).

**Updating** the model works as for a DXF: `model update --from new.stl` keeps the
placement, and the sha256 says when the file has changed.

## 3. Placing it on the board

The model is seen from above, its `up` axis pointing up. Its footprint is placed like a
picture:
- **x, y:** the lower-left corner of its footprint on the board;
- **width:** sets the scale in X and Y together. The height on the board follows;
- **rotation:** about Z, any angle, as pictures and DXFs turn;
- **depth:** how deep the relief goes. The model's highest point sits at the stock top,
  or at a start depth, and its lowest point at the depth (decision 4). The depth scales Z
  on its own, so a 14 mm tall model can be carved 4 mm deep, the usual way reliefs are
  cut. *Keep proportions* sets the depth from the width.
- **mirror:** for the back of a two-sided board, which takes care of this as for any part.

The placed model is the mesh scaled and moved, in the drawing's coordinates, Z from the
stock top down.

## 4. What is cut: the area

By default the cut covers the model's **footprint outline**: the outline of its shadow
seen from above, grown by nothing. Chipload works out that outline as a closed shape and
puts it on the model's layer, as a traced picture's shapes go on its layer.
- Where the model has no surface inside the outline, as in a hole through it, the tool goes
  to the relief's depth (the floor).
- The outline can be selected for a profile, for example to cut the relief out of the
  board, or for a pocket that lowers the background first.

**Optional boundary:** the relief operation can select closed shapes instead, as a pocket
does. It then cuts only inside them; the model's footprint outline is the default
selection (decision 5).

## 5. The operation: Relief

A new operation kind, **Relief**, in the Operation menu and ribbon. It cuts one model:

| Field | Meaning | Default |
|---|---|---|
| model | which model | the selected or only one |
| select | the area (§4) | the model's outline |
| tool | the finishing tool | a ball nose, else the smallest end mill |
| stepover | between raster lines, mm or % of the tool | 10 % of the tool (ball nose) |
| angle | the raster's direction, degrees | 0 (along X) |
| direction | one way or back and forth | back and forth |
| feed, plunge feed, spindle | as other operations, from the tool's cutting data | |
| start depth | the model's top below the stock top | 0 |
| depth | the relief's depth (the model's §3 depth) | from the model |
| **roughing** | optional, like a V-carve's floor pass: tool, step down, stepover, allowance | off; on when the relief is deeper than one step down |

**Finishing:**
- Parallel lines at the angle, one stepover apart, across the area.
- Along each line the drop cutter (§6) gives the tool's lowest Z without cutting the model,
  at sample points 1/10 of the stepover apart or 0.2 mm, whichever is smaller. Neighbouring
  points on one straight line within the arc tolerance (0.01 mm) are joined.
- Lines are joined into one path back and forth, with a lift only where the area breaks.
- Each point is kept at or above the floor.

**Roughing,** when on (decision 6):
- The roughing tool clears the material in levels, one step down at a time.
- At each level the raster lines run at `max(level, drop-cutter Z + allowance)`, using the
  roughing tool's own drop cutter, and only where there is still material above the line.
- A flat end mill does it. It reuses the drop cutter, so it needs no new clearing method.
- Clearing each level as a pocket (offset rings) comes later if the raster proves slow.

## 6. The drop cutter

For a point (x, y) and a tool shape, the drop cutter finds the lowest Z of the tool's tip at
which the tool touches the placed mesh and nowhere goes into it. It does this for every
triangle under the tool's footprint and takes the highest result. For each triangle there
are three tests:
1. **Corners:** the tool's height over each corner within its radius;
2. **Face:** the tool touching the plane of the triangle inside the triangle;
3. **Edges:** the tool touching each edge segment.

Each test is worked out in closed form for each tool shape:
- **flat end mill:** a cylinder;
- **ball nose:** a sphere on a cylinder;
- **V-bit:** a cone with a flat tip;
- **tapered ball nose:** a sphere joined to a cone. Its result is the higher of the
  sphere's and the cone's, each over the radii it covers.

**Speed:**
- The triangles go into a uniform grid by their XY bounds, so each point tests only nearby
  triangles.
- The lines are worked out in parallel, then joined in line order, so the result is
  byte-identical every time.
- `rayon` (MIT/Apache-2.0) would be the one new dependency. Otherwise `std::thread::scope`
  splits the lines into chunks, with no new dependency (decision 8).

**Sources:** the method is the standard inverse tool offset or drop cutter of three-axis
CAM. It is described in textbooks on sculptured surface machining, for example B. K. Choi
and R. B. Jerard, *Sculptured Surface Machining* (Kluwer, 1998); the exact reference is to
be checked when the algorithm section is written. The closed forms are written here from
geometry, never from another program's source (ADR 0031).

## 7. Verifying and checking

**Verifying the path,** as every offset is verified:
- After a path is made, an independent test samples it. At each sampled tool position, no
  mesh corner lies inside the tool, and no point sampled on the faces does either, within
  the tolerance.
- That test is written apart from the drop cutter: a brute-force distance test,
  slower but simple. It runs on a sample of positions, so it stays quick.
- A failure is an error. The path is never handed out unchecked.

**The checker:** unchanged in principle. 3D moves are G1 X Y Z, which it already handles:
- limits;
- no rapid below safe height over the stock;
- feed not zero;
- no cut below the max depth below stock.

The relief's depth must be at most the stock's thickness; the checker refuses the program
otherwise, as it does for any cut.

**The simulation:**
- The height map cuts with a sphere profile for ball noses, besides the flat and cone
  profiles it has now.
- A tapered ball nose is the sphere and the cone together.
- The 3D view shows the relief.

## 8. Tools

A new tool type, **ball nose** (`ballnose`), stored like the others:
- `diameter`: the cutting diameter;
- optional `angle`: the taper, as a V-bit's, for a tapered ball nose;
- optional `tip_diameter`: the ball's diameter, for a tapered ball nose.

A plain ball nose has neither `angle` nor `tip_diameter`. The ball's diameter is then the
diameter.

A ball nose cuts reliefs and follow paths only (decision 7). The other operations refuse it
with a problem, because their depths and floors assume a flat bottom.

## 9. Command line, then the window

```
chipload model add sign.chipload rose.stl --width 120 --depth 6 --at 20,20 [--up y] [--units inch]
chipload model set sign.chipload 1 --rotate 15 --depth 4
chipload model list sign.chipload          # size, triangles, placement
chipload model remove sign.chipload 1
chipload op add sign.chipload relief --model 1 --tool 4 --stepover 8% --angle 45
chipload op set ... roughing (as the V-carve floor pass is set)
chipload tool add "Ball 3" --type ballnose --diameter 3
chipload tool add "Taper 1/32" --type ballnose --diameter 6.35 --angle 3.6 --tip 0.8
```

**The window** (a later step, as rule 5 says):
- *File > Add 3D model…*;
- *Models* in the Browser with their placement in Properties, and handles on the canvas as
  for a picture;
- the model drawn faintly shaded in the 3D view;
- *Relief* in the ribbon's OPERATION group and the Operation menu.

## 10. Schema

*Settled while building (§14): no bump; the draft said:* **Version 37:**
- the tool type `ballnose`;
- `drawing.models`;
- the operation kind `relief`.

An older Chipload must refuse these files, not misread them, hence the bump. The migration
36 → 37 changes nothing, since no older file has any of them. A frozen v36 sample already
exists; a test shows that it migrates to the same G-code.

## 11. Edge cases and tests

- **A model not closed** (holes in its surface): the drop cutter falls through a gap to the
  floor. The import report counts open edges, and a note says it.
- **A vertical wall, or a face pointing down:** the edge and corner tests hold the tool
  off it. A face pointing down is never reached from above, and doesn't matter.
- **A tool wider than a feature:** the tool rides over it. The simulation shows what is left.
- **Depth 0, depth equal to the stock thickness, a model larger than the stock:** depth 0
  is a problem; depth equal to the stock is allowed and the checker is the judge; a model
  outside the stock gets the usual "outside the stock" note.
- **Property tests (proptest):**
  - random small meshes and tool positions: the drop cutter's Z against a brute-force
    search on a fine grid of tool heights, within tolerance;
  - the tool never below the floor;
  - one tool shape's result is never above a larger version of the same shape at the same
    point (a monotonic check).
- **Golden:** a small relief (a dome and a stepped boss) in each tool shape, its G-code
  frozen, and a simulation golden.
- **Fuzz** the STL reader as the DXF and SVG readers are.

## 12. Order of building

1. **The reader and the model:** `chipload-mesh`, `drawing.models`, schema 37, `model
   add/set/list/remove`, the footprint outline on its layer.
2. **The drop cutter and finishing:** the ball nose tool, the relief operation (finishing
   only), the path verification, the simulation's sphere profile. Golden and proptests.
3. **Roughing:** levels with the raster.
4. **The window:** the model in the Browser and the 3D view, the forms, the ribbon button.
5. **The guide:** a page *3D reliefs*, the tool type, the reference.

One commit each, with tests, the guide and TODO.md.

## 13. Decisions

1. **Take relief carving in** as an exception to ADR 0017 (ADR 0031): top-down, 3 axes, STL
   only. *Recommended.* Alternative: keep it out; users use another program for reliefs.
2. **Units:** the model is placed by the width you give. The file's unit only labels the
   size on import (mm by default, `--units inch` to read it so). *Recommended:* the width is
   what you want on the board anyway. Alternative: require the unit on every import, as for
   a DXF without units.
3. **The STL is embedded** in the project, base64, as pictures are, with a warning over
   20 MB. *Recommended:* the project stays one file that opens anywhere. Alternative: refer
   to the file by path, which is smaller but breaks when the file moves.
4. **The depth scales Z on its own** (*Keep proportions* optional). The model's top is at the
   start depth. *Recommended:* reliefs are nearly always flattened. Alternative: uniform
   scale only.
5. **The area defaults to the model's footprint outline,** drawn on the model's layer, and
   can be any selected closed shapes. *Recommended:* the outline also serves to cut the
   relief out. Alternative: the footprint's bounding rectangle, which is simpler but cuts
   the floor round an irregular model.
6. **Roughing by levels with the raster** in the first version, using the drop cutter with
   an allowance. *Recommended:* no new clearing method. Alternative: Z-level pockets from the
   height map's contours, which are fewer moves but much more work. That stays for later if
   needed.
7. **A ball nose cuts only reliefs and follow paths.** *Recommended:* the other operations
   assume a flat bottom for their depths and floors. Alternative: allow it everywhere, with
   a note.
8. **Parallel lines with `std::thread::scope`,** no new dependency. *Recommended:* it is enough
   for one loop over lines, and keeps the dependency list as it is. Alternative: `rayon`
   (MIT/Apache-2.0), tidier when more loops come.
9. **The tapered ball nose comes in the first version,** with the plain ball nose, the flat
   end mill and the V-bit. *Recommended:* it is the usual tool for detailed hobby reliefs.
   Alternative: the plain ball nose first, the tapered one later.
10. **The order of building** (§12): the reader and the model first, then finishing, then
    roughing, then the window. *Recommended:* each step can be tried from the command line.

## 14. Settled while building

Step 1, 2026-11-12:
- **No schema bump** (§10 changed). `drawing.models` is left out of the file when empty,
  as texts, groups and frames are, and an older Chipload refuses a project that has one
  (an unknown field) instead of misreading it. The same holds for the ball nose and the
  relief operation in step 2: an unknown tool type or operation kind is refused on
  loading. Projects without models open in 0.35 as before.
- **The footprint outline** is the placed model's shadow on a grid of at most 1000 cells
  along its longer side (and cells of at least 0.05 mm), traced with the picture tracer
  (`chipload-image`), lines and arcs within half a cell. A cell is covered when its centre
  lies under a triangle or a corner falls in it. The outline is a `path` shape with
  `model` as its origin, drawn again on every change, never exploded.
- **Placing** (`models::placed_mesh`): the mesh turned up, scaled by width over its own
  width, its footprint's lower-left corner at (x, y), turned about the footprint's
  centre, its top at Z 0 and its lowest point at −depth.
- `--units` sets only the size read on import (mm by default); `--proportional` puts
  the depth back in proportion.
- The window shows the outline as a shape; the model's own place in the Browser, its form
  and handles come with step 4.
