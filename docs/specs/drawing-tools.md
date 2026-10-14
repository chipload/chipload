# Spec: drawing circles, rectangles and polylines

Status: **approved 2026-10-10** (Q1–Q6 in §8: all as recommended).

Guide: docs/book/src/drawings/shapes.md.

User request (2026-10-10): draw rectangles, polylines and circles directly in the app, and
adjust their defining values in the UI after creating them:
- circle: centre point and radius;
- rectangle: centre point, length, width and rotation angle;
- polyline: a list of x, y points.

Builds on projects (docs/specs/project-file.md), which already store shapes with ids that are
never reused and keep drawn shapes when the drawing is updated from a DXF.

Out of scope: arcs and bulges in polylines, rounded rectangle corners, editing or deleting
imported shapes, dragging handles on the canvas, snapping to existing geometry, constraints,
text.

## 1. Format: job schema version 5

Drawn shapes store their **defining values**, not segments, so they stay editable. Segments
are derived from them on every load, as contours already are.

```json
{"id": 7, "layer": "0", "kind": "circle", "center": [50.0, 30.0], "radius": 12.5}
{"id": 8, "layer": "0", "kind": "rectangle", "center": [0.0, 0.0],
 "length": 80.0, "width": 40.0, "angle": 0.5235987755982988}
{"id": 9, "layer": "0", "kind": "polyline", "closed": true,
 "points": [[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]]}
```

- `path` shapes (imported) are unchanged: `closed`, `segments`, `dxf_handle`, `dxf_type`.
- Millimetres and radians in the file, as everywhere. The UI and CLI show the rectangle angle
  in degrees.
- **Version bump to 5.** A v4 Chipload would refuse the new kinds anyway ("unknown variant"),
  but the bump makes it say the clear thing ("made by a newer Chipload"). Migration v4 → v5
  is the identity: v4 files only hold `path` shapes. Frozen v4 samples exist already
  (`tests/corpus/jobs/v4/`); they must migrate and give their golden G-code unchanged.
- **Validation on load** (latest format only): radius, length and width finite and > 0;
  angle finite; centre and points finite; a closed polyline has ≥ 3 points, an open one
  ≥ 2; consecutive points must be more than `CHAIN_TOL_MM` apart. Anything else is a
  `JobError` naming the shape id. Self-intersecting or folded polylines are not refused
  here: they become invalid contours like an imported drawing would (red, can't be cut).

## 2. Shapes to segments (chipload-job or -dxf, pure)

- **Circle**: one arc, `start_angle` 0 and sweep 2π, exactly as an imported DXF `CIRCLE`.
  So it starts at (cx + r, cy) and behaves identically in every operation.
- **Rectangle**: four lines, counter-clockwise. `length` runs along the local X axis and
  `width` along local Y; `angle` rotates the rectangle about its centre, CCW positive. It
  starts at the corner at (−length/2, −width/2) in local coordinates.
- **Polyline**: one line per pair of consecutive points, plus the closing line if `closed`.
- Drawn shapes go through exactly the same chaining, classification, nesting and issue
  report as imported ones. A closed drawn shape is its own contour. An open polyline can join
  imported open geometry it meets end to end, as two DXF entities would.
- Same input, same segments (determinism). Rotation uses `sin_cos` on the stored radians;
  nothing is rounded.

## 3. Ids, editing and operations

- A new shape takes `next_shape_id`, which then grows (ids are never reused).
- **Editing** a drawn shape keeps its id, so operations that select it follow the new
  geometry. The toolpath becomes out of date (existing "press F5" behaviour).
- **Deleting** a drawn shape: operations that selected it are **flagged, not fixed**. They
  can't run until their geometry is chosen again, as after *Update drawing from DXF*.
- **Imported shapes stay read-only** in this step: no editing, no deleting. *Update from
  DXF* keeps drawn shapes (project-file.md §4).
- `drawing.source` (the import provenance) is untouched by drawing.

## 4. Layer

Every drawn shape has a layer, editable like its other values; any name, including a new
one. Default for a new shape: the layer of the current canvas selection if it is all on one
layer, else `0`. Layer selectors pick drawn shapes up like imported ones.

## 5. Command line (first, CLAUDE.md rule 5)

```
chipload shape add part.chipload circle --center 50,30 --radius 12.5 [--layer L] [-o out]
chipload shape add part.chipload rectangle --center 0,0 --length 80 --width 40 [--angle-deg 30]
chipload shape add part.chipload polyline --points "0,0 40,0 20,30" [--open]
chipload shape set part.chipload 7 --radius 15          # any defining value, or --layer
chipload shape remove part.chipload 7
chipload inspect part.chipload                          # lists shapes with kind and values
```

Each writes the project in place (atomically) or to `-o`, and prints the shape id and any
operations it flagged. `set` on an imported shape is refused with a clear message.

## 6. GUI

- **Draw** menu and toolbar: Circle, Rectangle, Polyline. Esc cancels.
  - Circle: click the centre, then a point on the circle.
  - Rectangle: click two opposite corners (angle 0); set the angle afterwards.
  - Polyline: click points; Enter or double-click finishes it open; clicking the first point
    closes it. Backspace removes the last point.
  - While drawing, a preview follows the cursor and the status bar shows the values.
  - Clicks are rounded to a grid that depends on the zoom (1, 0.5, 0.1 mm…), shown in the
    status bar, so values start out round. They can be typed exactly afterwards.
- Clicking a drawn shape selects it as today; when the selection is **one drawn shape**, the
  **Selection** section (left panel) shows its editor. (Implementation note 2026-10-10: not
  the right-hand properties panel as first written, because that panel holds the operation
  being edited, whose *Use selection* needs canvas selections, drawn shapes included.)
  - circle: centre X, Y, radius;
  - rectangle: centre X, Y, length, width, angle (°);
  - polyline: a table of X, Y rows with insert and delete per row, and a *closed* checkbox;
  - layer, plus a **Delete shape** button.
  - Invalid values (radius 0, too few points) are shown in red and not applied.
- Every edit is one undo step (existing undo/redo).
- Imported shapes show their kind, layer and DXF handle, read-only.

## 7. Tests

- **Format:** round trip of each kind (exact floats); validation errors for each invalid
  value; v4 samples migrate and give unchanged golden G-code; a v5 file is refused by the
  v4 loader path as "newer version".
- **Segments:** circle = the same primitive an imported CIRCLE gives; rectangle corners for
  angle 0, 90° and 30°, CCW; open and closed polylines.
- **Machining:** a project with a drawn rectangle and a drawn circle hole, profiled outside
  and inside, passes the checker; a new golden file (with a note in the commit message, as
  CLAUDE.md requires for new golden output).
- **Ids:** a deleted shape's id is never handed out again; deleting a selected shape flags
  the operation.
- **CLI:** add, set, delete, inspect; `set` on an imported shape refused.
- **GUI (app level):** create each kind through the same functions the clicks call, edit
  values, undo/redo, delete; grid rounding at a few zoom levels.

## 8. Decisions (approved 2026-10-10)

1. **Storage** (§1): store defining values, derive segments on load, schema v5 with an
   identity migration. **Yes.**
2. **Rectangle convention** (§2): length along X and width along Y at angle 0; angle CCW,
   in degrees in the UI and CLI and radians in the file. **Yes.**
3. **Polylines** (§1, §2): straight segments only, with a *closed* flag. Open polylines are
   allowed, although no operation cuts open contours yet. **Yes**; arcs later
   if you need them.
4. **Layer default** (§4): the selection's layer if there's one, else `0`.
   **Yes.**
5. **Creating by clicks** (§6), rounded to a zoom-dependent grid, then exact values in the
   panel. No dragging handles on the canvas in this step. **Yes**; handles can
   come later.
6. **Imported shapes read-only** (§3) in this step; deleting a drawn shape flags the
   operations that used it. **Yes.**
