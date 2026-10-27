# Spec: drawing phase 2, editing operations

Status: **draft 2026-10-27**, for review (decisions in §10).

Guide: docs/book/src/drawings/shapes.md (a new *Editing operations* section).

Phase 2 of docs/specs/drawing-roadmap.md (approved 2026-10-27): offset, fillet and
chamfer, trim, extend and break, union, subtract and intersect, arrays, align and
distribute. Builds on phase 1 (drawing-input.md): the command pattern, typed values,
snapping, box selection.

## 1. Common rules

- **On the command line first** (rule 5): each is a `chipload shape …` command that writes
  the project, then a GUI tool.
- **Drawn shapes only.** Imported shapes (a DXF's, a picture's, a text's) can't be changed
  in place; *Explode* makes a DXF's shapes drawn first. They can still be **cutting edges**
  for trim and extend, and **snap targets**.
- **No format change.** Results are the existing kinds: circle, rectangle, polygon,
  polyline (with arcs). Where an operation can't keep a kind, the shape becomes a polyline
  and keeps its id (the GUI says so once: "the rectangle becomes a polyline").
- **Ids:** a shape changed in place keeps its id, so operations follow it. A shape split in
  two keeps its id for the first part; the others get new ids. Shapes removed (union,
  subtract, intersect) flag the operations that used them, as deleting does. *Approved:*
  the result of a union, subtraction or intersection takes the first selected shape's id.
- **New shapes go on the source shape's layer** (decision 1 on layer-selecting operations).
- **Geometry in `chipload-draw`** (ADR 0023), on core's lines and arcs; the pipeline maps
  shapes to it and applies results (ids, flagged operations, points that follow). Every
  result is checked before it is kept (§2, §5); a panic inside `cavalier_contours` becomes
  an error.
- **One undo step** per operation; results are selected afterwards.

## 2. Offset

A shape at a distance, inside or outside (a closed shape) or left or right (an open one).
The input is kept.

```
chipload shape offset P IDS --distance 3 [--inside] [--corners round|sharp] [--count 1]
```

- **Kinds kept where exact:** a circle stays a circle (radius ± d); a rectangle offset
  outward with round corners stays a rectangle (corner radius + d), inward a rectangle
  (corner radius max(r − d, 0); the sharp case is exact too); a slot stays a slot.
  Everything else becomes a closed or open polyline with true arcs (`cavalier_contours`
  `parallel_offset`, ADR 0006).
- **Corners** (outward, or a polyline's convex corners): *round* (an arc about the
  corner: the exact offset) or *sharp* (the two sides extended until they meet, as a frame
  round a part). Sharp applies between two straight sides; where an arc meets a corner it
  stays round. Decision 2.
- **`--count N`:** N offsets, each `distance` further out (or in): rings round an outline.
- **A shape that vanishes** (inward further than it is wide) is refused with the reason;
  one that splits (a dumbbell inward) gives several shapes.
- **Checked:** every result lies at the distance from its source (sampled along it, within
  `ARC_APPROX_TOL_MM`), on the asked side, and doesn't cross it, as toolpath offsets are.
- **GUI (O):** select shapes, or click one; move the cursor to the side wanted; the
  preview follows at the cursor's distance on the grid; click, or type the distance. The
  corners choice and the count are fields on the status bar while the tool is active, as
  the polygon's sides are.

## 3. Fillet and chamfer

Round or cut the corners of a drawn shape.

```
chipload shape fillet P ID --radius 5 [--corner K]
chipload shape chamfer P ID --distance 3 [--corner K]
```

- **Corners:** between two straight segments of a polyline or polygon (`--corner K`: the
  vertex K, from 0; without it every such corner). A corner between a line and an arc, or
  two arcs, is left as it is in this phase (decision 3).
- **Fillet:** the arc of `radius` tangent to both sides, its ends replacing the corner
  (the polyline gains a vertex and a bulge). **Chamfer:** a straight cut, `distance` back
  along each side.
- **Too large:** a radius or distance that needs more of a side than it has (shared with
  the next corner's when all are done) is refused with the most that fits.
- **Kinds:** a rectangle with all four corners filleted keeps its kind (the corner radius);
  one corner, or a chamfer, makes it a polyline. A polygon becomes a polyline.
- **GUI:** select a shape, then the tool; hover a corner (it highlights), click, type the
  radius (the last one used is the default). Holding Shift while clicking does every
  corner.

## 4. Trim, extend and break

```
chipload shape trim P ID --at X,Y       # the piece of ID under X,Y, between crossings, goes
chipload shape extend P ID --end start|end   # an open polyline's end line grows to the next crossing
chipload shape break P ID --at X,Y      # split at the point
```

- **Cutting edges:** every contour shown (drawn and imported), the shape's own other
  segments included. Crossings come from `chipload_draw::intersect`.
- **Trim:** the piece between the two crossings nearest `--at` along the shape (or to the
  shape's end) is removed. An open polyline trimmed in its middle becomes two (the first
  keeps the id); a closed shape trimmed becomes one open polyline (a circle becomes an
  arc). A piece with no crossing at all is refused ("nothing crosses it: delete it
  instead").
- **Extend:** only a straight end segment, along its line, to the nearest crossing ahead
  within 10 000 mm; refused when nothing is ahead.
- **Break:** an open polyline becomes two at the point; a closed one opens there. The
  point is the nearest on the shape to `--at`.
- **GUI:** Trim (X): click the pieces to remove, one after the other, each a step; Esc
  ends. Extend: click near the end to extend. Break: click the point.

## 5. Union, subtract, intersect

Closed drawn shapes combined (`cavalier_contours` boolean, true arcs).

```
chipload shape union P IDS
chipload shape subtract P BASE IDS      # IDS cut out of BASE
chipload shape intersect P IDS
```

- **Result:** closed polylines (a circle or rectangle stays one when nothing touched it).
  The first result shape takes the first input's id (approved); other pieces get new ids;
  the inputs are removed and the operations using them flagged, except where they keep the
  id.
- **Holes:** a union that encloses a hole (a ring of shapes) gives the outline and the
  hole as two shapes; nesting already treats the hole as a hole.
- **Refused:** an open shape among the inputs; a result that is empty (subtracting
  everything, shapes that don't meet for intersect).
- **Checked:** every result is closed and doesn't cross itself; areas add up (a union's
  area is at least the largest input's and at most their sum; an intersection's at most the
  smallest input's; a subtraction's at most the base's).
- **GUI:** select the shapes, then *Union*, *Subtract* (the shape selected first is the
  base, as in *the first selected shape keeps its id*) or *Intersect*.

## 6. Arrays

Copies of shapes in a grid or round a centre; the originals are kept and the copies get
new ids, on the originals' layers.

```
chipload shape array P IDS --columns 4 --rows 2 --dx 30 --dy 20
chipload shape array-polar P IDS --count 6 --center X,Y [--angle-deg 360] [--no-rotate]
```

- **Grid:** `columns × rows` including the original, `dx` and `dy` apart (centre to
  centre); negative values go left or down.
- **Polar:** `count` including the original, spread over `angle-deg` (360: evenly round;
  less: the first and last at the ends of the sweep), turned with their position unless
  `--no-rotate`.
- Different from the job's grid of copies (copies-grid.md), which copies a whole part with
  its operations; an array adds shapes to the drawing.
- **GUI:** select, then the tool: a small form under *Selection* with the values and a
  preview of the copies on the canvas; *Create* adds them.

## 7. Align and distribute

```
chipload shape align P IDS --to left|centre|right|top|middle|bottom [--stock]
chipload shape distribute P IDS --horizontal|--vertical
```

- **Align:** each shape's box moves so its side or middle lines up with the selection's
  box (or, with `--stock`, the stock's). Shapes move as a whole (`transform_shapes`), ids
  kept.
- **Distribute:** three shapes or more, the gaps between their boxes made even, the first
  and last staying put.
- **GUI:** buttons in the *Selection* section when two or more drawn shapes are selected
  (align to the stock with one shape too).

## 8. GUI placement

The ribbon gets a **MODIFY** group of small icon buttons, three to a row, as the shape
tools (named with their key on hover): Offset, Fillet, Chamfer, Trim, Extend, Break,
Union, Subtract, Intersect, Array, Polar array, Align. The same items in the Draw menu and
in the canvas's right-click menu on a drawn shape. Keys (decision 4): **O** offset, **X**
trim; the rest have none.

## 9. Tests

- `chipload-draw`, for each operation: the edges named above (offset that vanishes or
  splits; fillet too large, at a 179° and a 1° corner; trim with no crossing, at an end,
  of a circle; extend with nothing ahead; boolean of touching, nested and disjoint shapes;
  polar array of 1 and of 360° with no double at the end).
- `proptest` invariants: an offset lies at its distance and never crosses its source; a
  fillet's arc is tangent to both sides; a union contains every input's points; a trim
  never adds length.
- Pipeline: ids kept and flagged as §1; points that follow (start points, corners, tabs)
  move with a shape changed in place.
- CLI: each command, its refusals.
- Golden G-code unchanged; one new golden: a part drawn and edited only with these
  commands (an outline offset with sharp corners, filleted, with a polar array of holes
  subtracted), pocketed and profiled.

## 10. Decisions

1. **New shapes' layer and layer-selecting operations.** An operation that selects a
   layer (not shapes) cuts every contour on it, so an offset or an array copy on that
   layer would be cut too, as a shape drawn there is today. (a) Same layer, and the status
   bar and CLI name the operations that will now cut it; (b) same layer, and those
   operations are pinned to their current shapes, as adding a DXF pins them; (c) a layer
   of their own. *Recommended: (a)*: drawing on a layer means "cut this", and an offset
   outline or a row of holes is usually meant to be cut; the notice makes it visible, and
   undo is one step.
2. **Offset corners:** both *round* and *sharp* (a real trade-off: round is the true
   offset, sharp the frame round a part), default round. *Recommended: yes.*
3. **Fillets between a line and an arc** (or two arcs) left for later. *Recommended:
   yes*: straight corners are what rectangles, polygons and most outlines have.
4. **Keys:** only O (offset) and X (trim); F, T, M, G and P stay the view's. *Recommended:
   yes.*
5. **Order of building:** offset; fillet and chamfer; trim, extend, break; booleans;
   arrays; align and distribute; one commit each. *Recommended: yes.*
