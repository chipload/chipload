# Spec: DXF export and construction geometry (drawing phase 5)

Status: **draft 2026-10-28, for review** (decisions in §9).

Phase 5 of docs/specs/drawing-roadmap.md (§8), brought forward on 2026-10-28: the user put
designs (docs/specs/designs.md) and parameters (docs/specs/parameters.md) on hold and asked
for this next. Two parts: **A**, getting the drawing out as a DXF; **B**, construction
geometry, lines that help drawing and are never cut.

# Part A: DXF export

## A1. Why

Once a part is drawn (or edited) in Chipload, the drawing is wanted elsewhere: a laser
cutter's software, a quote request to a CNC shop, a CAD program, a colleague. Today the
only way out is G-code.

## A2. What is written

- **Every shape the canvas shows,** where it shows it (each drawing file's placement
  applied): drawn shapes, a DXF's or SVG's shapes, traced pictures, and text letters as
  their outlines. Hidden groups and layers (`drawing.hidden`) are left out unless
  `--hidden` (decision 2).
- **On their layers,** with their names. A name a DXF can't hold (`< > / \ " : ; ? * | = '`
  and control characters) has those characters replaced by `_`; two layers that then
  clash get `_2`, `_3`. The export's report lists each rename.
- **As the simplest entities that are exact:**
  - a circle → `CIRCLE`;
  - everything else → `LWPOLYLINE`, closed or open as the shape is, with arcs as bulges
    (an arc past a half circle in two halves);
  - an imported path made of several runs that don't join → one `LWPOLYLINE` per run.
  Rectangles, polygons and slots are polylines (DXF has no such entities); their corner
  radii are bulges, so arcs stay true arcs.
- **Not written:** toolpaths (that's the G-code), the stock, copies of the job's grid,
  operations, dimensions; construction geometry unless `--construction` (part B).

## A3. Units and coordinates (decision 3)

- **Millimetres by default,** with `$INSUNITS = 4` (mm) in the header, so the file says
  its units and Chipload's own importer (and any other) reads it without asking.
- `--units inch` writes inches (`$INSUNITS = 1`), every coordinate divided by 25.4.
- **Coordinates are the drawing's:** the origin is the drawing origin, as on the canvas
  and in the G-code's work coordinates when work zero is the drawing origin. Y up, as DXF.

## A4. The file itself (decision 4)

- **AutoCAD R2000 format (AC1015), ASCII,** the oldest version with `LWPOLYLINE` and
  `$INSUNITS`; read by every CAD, laser and CAM program we know of, and by Chipload.
- **Written through the `dxf` crate** already used for reading (ADR 0005), with everything
  the crate would fill from the clock or at random set to fixed values: creation and update
  times (fixed at 2000-01-01, the same whatever the time zone), and the fingerprint and
  version GUIDs (derived from the drawing's content). Handles are given in order.
- **Deterministic:** the same project gives a byte-identical DXF on every machine and run
  (CLAUDE.md), tested by writing twice, under two time zones.
- *Fallback, if the crate can't be made deterministic:* a small writer of our own for the
  same subset (header with `$INSUNITS`, layer table, the two entities), written from the
  DXF reference.

## A5. Checked (as G-code is)

Before the file is written, it is read back with Chipload's own importer. Every exported
shape must come back as the same geometry (each point within the chaining tolerance, each
arc's centre and radius within the arc tolerance) on the same layer, or nothing is written
and the error says which shape didn't. A round trip is the export's definition of
correct.

## A6. Command line and window

```
chipload dxf export P -o part.dxf [--units inch] [--hidden] [--construction]
                                  [--layer NAME …] [--selection IDS]
```

- `--layer` (repeatable) writes those layers only; `--selection` those shapes only.
- The report: entities written by type, layers, renames, and the file.
- **Window:** *File > Export drawing as DXF…* (and the Export button's menu): the file
  name, then the options (inch, hidden, construction, only the selection when there is
  one). The status bar says what was written.

# Part B: construction geometry

## B1. What it is

Lines (and circles, and any drawn shape) that help drawing: a centre line to mirror about,
a circle to place holes on, a guide at 45°. They show and snap like any shape, but are
**never cut**: no contour, no selection by layer, no toolpath, not in a pocket's
boundary.

## B2. How a shape becomes one (decision 5)

- **A flag on a drawn shape:** `"construction": true`. Any drawn kind can be one.
- **In the window:** the status bar of every drawing tool has a *Construction* toggle
  (key **K**) for the shapes it draws next; a drawn shape's Properties has the same
  checkbox; the right-click menu toggles it for the selection.
- **On the command line:** `chipload shape add … --construction`, `chipload shape set P
  ID --construction true|false`.
- **Imported shapes** can't be construction (they are what the file says); explode first.

## B3. How it behaves

- **Shown** dashed and thinner, in a muted colour, on its layer.
- **Snaps** like any shape: ends, middles, centres, intersections with other shapes,
  perpendicular, tangent, nearest.
- **Never cut:** left out of contours before chaining, so it can't join a real shape's
  ends, can't be selected by an operation (by layer, by drawing, or by id: refused with a
  message), and never reaches planning or the checker. An operation that selected a shape
  that is then made construction is flagged, as when a shape is deleted.
- **Editing operations** (offset, fillet, trim, extend, booleans, arrays) work on it; their
  results are construction too. Trim and extend use construction lines as cutting edges,
  which is often what they are for.
- **Exported** only with `--construction`, on a layer of their own (`<layer>_construction`)
  so the receiver can tell them apart.

## B4. File format (decision 6)

`construction` on a drawn shape, written only when true. An older Chipload reading it
would cut the guide lines, so the schema is bumped (**32**) and the migration from 31
changes nothing (no older file has the flag); the frozen v31 samples and their goldens are
unchanged.

# Tests

- Export: each kind (circle, rectangle with radius, polygon, slot, polyline with arcs, a
  DXF's paths, an SVG's, a text's letters, a traced picture) round-trips through the
  importer; layer renames; inch export; hidden and construction options; a byte-identical
  second export, also under another time zone. Every corpus project is exported and read
  back (a sweep, as for exploding).
- Construction: not in contours, not selectable, flagged when an operation used it; snaps;
  trims against it; the schema 32 migration on frozen v31 samples; goldens unchanged.
- CLI and window: export with each option; the construction toggle, key and checkbox.

# §9. Decisions

1. **Both parts in one phase,** export first (A), then construction (B), one commit each.
   *Recommended: yes.*
2. **Export what the canvas shows,** placed, layers kept, hidden things left out unless
   asked. *Recommended: yes*: "what you see is what you get"; the receiver gets the part
   where it is on the stock.
3. **Millimetres with `$INSUNITS` by default, inches as an option.** *Recommended: yes*:
   a DXF that says its units is never misread.
4. **R2000 ASCII through the `dxf` crate,** with clock and random fields fixed, a writer of
   our own only if that fails. *Recommended: yes*: no new dependency, and the most widely
   read format with polylines and units.
5. **Construction as a flag on drawn shapes,** rather than a layer setting. *Recommended:
   yes*: a guide line usually sits on the layer of the thing it helps draw; a layer
   setting would make users invent layers for guides.
6. **Schema 32 for the flag,** so an older Chipload refuses the file rather than cutting
   the guides. *Recommended: yes* (if parameters come first later, they take the next
   number instead).
