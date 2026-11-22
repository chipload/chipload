# Spec: scale by reference, and stretch in X and Y

Status: **approved 2026-11-22** (the user: "go ahead"; §8 as recommended; see *As
built*, §9). The fourth item of step 2 of the drawing review
(TODO, *Next* 1; docs/specs/drawing-improvements.md).

Builds on drawn-shapes-box.md (the box round selected drawn shapes), box-handles.md (its
handles), dxf-whole-on-canvas.md (a whole DXF on the canvas), drawing-scale.md (a DXF's
uniform scale), drawing-input.md (typed values, snaps) and splines.md §5.

## 1. What is missing today

- **Scale by reference.** Scaling takes a factor (`shape scale --factor`, `place --scale`)
  or a corner handle dragged by eye. A drawing that comes in at the wrong size gets fixed
  by measuring, dividing and typing: a traced picture, a DXF without units, a sketch
  drawn roughly. What the user knows is a length on the drawing: "these two holes are
  100 mm apart" or "this side is 450 mm".
- **Stretch.** Nothing makes shapes wider without making them taller. A logo is fitted
  into a space of another shape, or a part drawn 400 mm long is wanted 500 mm long.

## 2. Scale by reference

**The idea:** two points on the drawing and the length their distance should have. The
factor is new length ÷ current length, and the first point stays where it is.

**In the window:** *Scale* in the MODIFY group and the Modify menu, beside *Mirror*:
1. Select what to scale: drawn shapes, a text, a picture, a group, or a whole DXF. That
   is anything whose box the corner handles scale today, and it scales the way they do.
2. Start *Scale*.
3. Click the **first point**. It snaps, as every point does, and it is the point that
   stays.
4. Click the **second point**. The status bar shows the distance between them and a
   **New length** field, focused, holding that distance.
5. Type the new length (arithmetic and units work, as in every length field) and press
   Enter. **Or** click a third point: the distance from the first point to it is the new
   length (decision 2), so a length can be taken from another shape.
- **Preview:** after the second click, the shapes follow the cursor's distance from the
  first point, so a third click can also be done by eye.
- **Esc** goes back one step; Esc at the first step ends the tool.
- The two points need not lie on the selection: measuring a length on a shape that isn't
  selected, and scaling the selection by it, is allowed.

**On the command line:**

```
chipload shape scale P 3 4 5 --reference "10,10 110,12" --length 120
chipload place P --dxf 2 --reference "0,0 254,0" --length 100
```

- `--reference "X1,Y1 X2,Y2"` with `--length L` instead of `--factor`; the first point
  stays. They don't combine with `--factor`, `--about`, `--width`, `--height` or
  `--scale`.
- Texts and pictures already take `--height` and `--width`; they get no reference option.

**Refused, with the reason:** two points closer together than the chaining tolerance; a
new length of 0 or less; a resulting scale outside the range a DXF's placement allows
(drawing-scale.md decision 5).

## 3. Stretch in X and Y

**The idea:** a different factor along X and along Y, about a point that stays: the box's
opposite side when a handle is dragged, the box's middle otherwise.

**What a shape becomes** (decision 3: the exact stretched shape, within the arc
tolerance):
- **Lines and straight polyline sides:** their points move; exact.
- **A rectangle square to the axes** (turned 0°, 90°, 180° or 270°): its length and width
  change; it stays a rectangle.
- **Splines:** their points move, and the spline through them is drawn again
  (splines.md §5, decision 4). It is not exactly the stretched curve, but it is smooth and
  goes through the stretched points, which is what the points are for.
- **Circles, arcs, slots, arc sides of polylines, polygons and turned rectangles:** a
  stretched arc is part of an ellipse, which isn't a line or an arc. The shape becomes a
  **polyline** of lines and arcs fitted to the stretched curve within `ARC_APPROX_TOL_MM`
  (core's `fit_path`, as splines and SVG curves are fitted). It keeps its id, so the
  operations that cut it follow, and the report says which shapes became polylines.
- **Corner radii** (rectangles, polylines, polygons) **stay radii** (decision 4): the
  corner points move and each corner is rounded again with the same radius. A rounded
  corner is usually there for a tool or a look, not as a part of the outline to stretch.
  A radius that no longer fits its corner is refused, naming the corner, as when it is
  typed.
- **Stretch 1 along both axes** changes nothing; stretch equal along both is a uniform
  scale and goes that way, so circles stay circles.

**What stretches** (decision 5): drawn shapes. A text, a picture or a whole DXF is not
stretched: their placements have one scale. The refusal says what to do instead: a DXF
can be exploded into drawn shapes (`dxf explode`), and a text's letters made into shapes.
A selection mixing drawn shapes with these is refused the same way.

**In the window:**
- **Side handles:** the box round selected drawn shapes gets a handle in the middle of
  each side. Dragging one stretches the shapes one way, the opposite side staying put.
  The corner handles still scale evenly. The box's turn handle and the side handles both
  stay usable when the box is turned: the stretch then follows the box's own sides.
- **Properties:** the selection's *Width* and *Height* with a **Keep proportions** chain,
  on by default. On, a new width scales evenly. Off, it stretches one way.
- **Readout** while dragging a side handle: the new width or height and its factor.

**On the command line:**

```
chipload shape stretch P 3 4 --x 1.25            # 25 % wider, about the box's middle
chipload shape stretch P 3 4 --width 500 --about 0,0
chipload shape stretch P 3 4 --x 1.1 --y 0.9
```

- `--x FX` and `--y FY` (factors, > 0, default 1) or `--width W` and `--height H` (mm);
  one of each axis at most. `--about X,Y` is the point that stays; default the middle of
  the box round the shapes.
- Prints the shapes that became polylines.

## 4. Correctness

- **Scale by reference:** the distance between the two points after scaling equals the
  new length (within the chaining tolerance), and the first point hasn't moved.
- **Stretch, exact kinds:** every point of a line, a polyline with straight sides and a
  square rectangle lands at its stretched place.
- **Stretch, fitted kinds:** every point of the fitted polyline lies within
  `ARC_APPROX_TOL_MM` of the true stretched curve, and the curve's points within the same
  of the polyline (both ways, so nothing is cut short). A property test on random circles,
  arcs and polylines with arcs, with random factors from 0.2 to 5.
- **Radii:** after a stretch each corner radius is what it was.
- **Ids:** every shape keeps its id; the operations that cut it still select it.

## 5. Tests

- **Pipeline:** the correctness checks above; refusals (points too close, length 0, a
  text in the selection, a radius that no longer fits); stretch 1 by 1 changes nothing;
  equal factors keep circles.
- **CLI:** `shape scale --reference --length`, `place --reference --length`, `shape stretch`
  with factors, sizes and `--about`; conflicting options refused.
- **Window:** the Scale tool's clicks with a typed length and with a third click; Esc steps
  back; a side handle stretching one way; the Properties chain on and off.
- **Golden:** none needed. G-code from a stretched shape is G-code from a polyline, which
  is covered. The existing goldens stay unchanged.

## 6. Format

No change. A stretched shape is stored as what it becomes (a polyline, a rectangle with a
new length, a spline with moved points).

## 7. Order of work

1. Scale by reference: the pipeline (one function for drawn shapes, and `place` for a
   whole DXF), the command line, the window's tool, the guide, the seven languages.
2. Stretch: the pipeline (exact kinds, fitted kinds, radii), `shape stretch`, the side
   handles, the Properties chain, the guide (drawings/shapes.md; splines' note), the seven
   languages.

## 8. Decisions (recommendations first)

1. **Scale by reference is a tool of its own, *Scale*, in MODIFY,** working on whatever
   the box's corner handles scale. *Alternative:* a step inside the box's corner drag
   (hard to find, and a drag can't take typed values well).
2. **The new length is typed or taken from a third click.** The click covers "make this as
   long as that". *Alternative:* typed only.
3. **A stretch gives the exact stretched shape:** curves that stop being arcs become
   polylines fitted within the arc tolerance. *Alternatives:* (a) refuse to stretch
   circles and arcs, which leaves most real outlines unstretchable; (b) keep circles as
   circles with their centres moved (what some CAD programs call stretching a part longer),
   which is a different tool. That could come later as *Stretch points*, if missed.
4. **Corner radii stay radii,** each corner rounded again with its radius. *Alternative:*
   stretch them too, into fitted elliptical corners, which loses the radii a user typed.
5. **Only drawn shapes stretch;** a DXF, a text or a picture is refused with the way to go
   (explode, letters to shapes). *Alternative:* a stretch in a DXF's placement (a schema
   change, and arcs that need fitting on every update).
6. **No default key** for *Scale*: the free letters (D, H, J, W, Y) don't suggest it, and
   S opens the quick menu. It is in the ribbon and the Modify menu, as Mirror's
   neighbours are. *Alternative:* W.

## 9. As built

- **Step 1:** `chipload_pipeline::scaling::{reference_factor, scale_by_reference}`;
  `shape scale --reference --length`, `place --reference --length` (a DXF, as a
  `Move::About`). The window's *Scale* (MODIFY, the Modify menu) takes the selected drawn
  shapes directly, one circle too (it has no box), else what the box's corners scale.
  After the second click the new length is typed in the entry beside the cursor, as for
  Offset and typed points, rather than in a status bar field: the same place every typed
  value goes. Stretch's geometry (`chipload_job::edit::stretch`, with a property test),
  `chipload_pipeline::scaling::stretch_shapes` and `shape stretch` came in the same
  commit; the window's side handles and *Keep proportions* follow in step 2.
