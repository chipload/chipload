# Spec: drawing phase 1, precise input and the basic tools

Status: **approved 2026-10-27** with the roadmap (docs/specs/drawing-roadmap.md, phase 1;
the user: "sounds good, go ahead"). Details settled while building are marked
*Implemented:*.

Guide: docs/book/src/drawings/shapes.md.

Builds on drawing-tools.md (tools, grid), drawing-follow-ups.md (handles, snapping, bulges)
and drawn-shapes-box.md (the box). Built in six steps, one commit each, in this order.

## 1. Step 1: the `chipload-draw` crate and more snaps

A new crate, `chipload-draw` (ADR 0023), depending only on `chipload-core`. No I/O, builds
for wasm32. Snapping moves into it from `chipload-gui/src/draw.rs`; the grid (which needs
the display units) stays in the GUI.

**Snap kinds**, in tie-break order:

| Kind | Where | Needs |
|---|---|---|
| End | each segment's start and end | |
| Intersection | where two segments cross or touch (lines and arcs, any contours, the same contour too) | |
| Centre | each arc's and circle's centre | |
| Quadrant | the points of an arc at 0°, 90°, 180°, 270° that lie on it | |
| Middle | each segment's middle | |
| Perpendicular | the foot of the perpendicular from the last point onto a segment, if it lies on it | a last point |
| Tangent | the points of an arc where a line from the last point touches it, if they lie on it | a last point |
| Nearest | the point of a segment nearest the cursor | |

- **Choosing** (as today): of every candidate within 8 px of the cursor, the nearest on
  screen wins; ties by kind in the order above, then the lowest contour id. **Nearest** is
  only taken when no other kind is within reach, so it never hides an end point. Then the
  grid; Alt for free.
- **Cost:** candidates are found per frame for the segments whose box, grown by the reach,
  holds the cursor; intersections only between those. A traced picture of thousands of
  segments costs only what is under the cursor.
- **The last point** is the previous click of the tool being used (a polyline's last
  point, a circle's centre is *not* one: perpendicular and tangent need a line).
- **Status bar names** as now: "intersection of #3 and #4", "quadrant of #2",
  "perpendicular to #4", "tangent to #2", "on #5".
- **Tolerance:** two segments touch when they come within `CHAIN_TOL_MM`; equal candidates
  (two ends at one point) are one.

## 2. Step 2: the drawing command: keys, typed values, angle steps

**Keys** (shortcut table, Drawing group; not while a text field has focus): **L** polyline,
**C** circle, **R** rectangle, **A** arc, **S** slot, **N** polygon. **Enter** with no tool
active starts the last tool used again. Esc and Backspace as now.

**Typed values.** While a tool waits for a point, typing a digit, `-`, `.` or `@` opens a
small entry beside the cursor, holding that character. Enter applies it, Esc closes it.

| Typed | Means |
|---|---|
| `x,y` | the point (drawing coordinates, as the forms show) |
| `@dx,dy` | the point that far from the last point |
| `d` | the point at distance d from the last point, towards the cursor |
| `d<a` | the point at distance d from the last point, at angle a (degrees, counter-clockwise from +X) |

- Lengths take a unit (`1/2in`, `12mm`); a plain number is in the display units, as in the
  forms (`parse_length`). Angles are degrees.
- Where the decimal comma is used: `x;y` separates with a semicolon, and then a comma is a
  decimal point (`12,5;30`).
- **Per tool**, at the step after the first: the circle takes `d` as its radius (from the
  centre); the arc's third step takes `d` as its radius (§4); the slot's third step `d` as
  its width; the polygon's second step `d` as its corner radius. A point typed at the first
  step must be `x,y`.
- Something that doesn't parse leaves the entry open, in red, with the reason on the status
  bar. The parser is a pure function with tests (in the GUI's `draw.rs`: it is input, not
  geometry).

**Angle steps.** Holding **Shift** while drawing holds the line from the last point to a
multiple of 15°: the cursor is projected onto that direction and its length goes to the
grid. Snapping is off while Shift is held (Alt still frees the length).

**Readout.** Beside the cursor while drawing: the line's length and angle (polyline, arc,
slot), the radius (circle, polygon), length × width (rectangle). The status bar keeps the
fuller text.

## 3. Step 3: box selection and moving with the mouse

In top view, with no tool active, a **left drag that starts on empty canvas** (not on a
handle, marker or contour) draws a selection box:

- **Left to right** (solid box): selects the contours wholly inside it.
- **Right to left** (dashed box): also those it crosses or touches.
- Without keys it replaces the selection; with **Shift** it adds; with **Ctrl** it toggles
  each contour, as Ctrl+click does.
- Only contours shown on the canvas (hidden groups and a hidden drawing are out).
- *Implemented:* a picture's or text's contours count only when the box catches nothing
  else and replaces the selection: it then selects the whole picture or text, as a click
  does (their shapes get new ids when traced or lettered again, so operations take them
  whole).
- The test (inside, crossing) is a pure function in `chipload-draw` on the segments, arcs
  exact (by their bounds and their crossings with the box's sides).

**Moving with the mouse** (the user, 2026-10-27: "box select and move shapes with the
mouse"): a left drag that **starts on a drawn shape** moves shapes instead of drawing a box.

- On a selected drawn shape: the whole selection moves (when it is all drawn shapes).
- On a drawn shape that isn't selected: it becomes the selection (Shift: is added to it),
  then moves.
- The point grabbed follows the cursor and **snaps** like a handle (snap points of the
  moved shapes left out; Alt free; Shift holds the move to 15° steps). *Implemented:*
  typing a value during the drag was left out: the mouse button is held down meanwhile,
  which makes typing awkward. An exact move is *Position* in Properties or `shape move`.
- While dragging, the outlines follow; on release the shapes are moved as one undo step,
  keeping their ids (`shape move`, drawn-shapes-box.md §2), so operations follow.
- A drag starting on an imported contour (a DXF's, a picture's, a text's letters) draws a
  box: those move as a whole with their own box handles, as now.
- Handles still win over the shape under them; a click without a drag selects as now.

**Panning moves** to the right or middle button, or **Space + left drag** (touchpads), as
in the 3D view. Scrolling zooms as now. The shortcut window and the guide say so.

## 4. Step 4: arc and slot tools

Both make existing kinds; no format change.

**Arc (A):** start point, end point, then a point the arc passes through.
- Becomes an **open polyline**: one segment with its bulge, or two (split at the arc's
  middle) when the arc is more than a half circle, since |bulge| ≤ 1.
- Third step typed as `d`: the radius; the arc bends towards the cursor's side and is the
  shorter of the two arcs (at least the chord's half: a smaller radius is refused).
- Refused: start and end the same point; the three points on one line ("on a line: use the
  polyline tool").

**Slot (S):** the centre of one end, the centre of the other end, then the width (a point:
twice its distance from the centre line; typed: the width).
- Becomes a **rectangle**: centre the middle of the two ends, length the distance between
  them plus the width, width the width, angle the direction from the first end to the
  second, corner radius half the width.
- **A slot stays a slot:** a rectangle whose corner radius is half its width (within
  `CHAIN_TOL_MM`) is shown as "Slot" in its form and the Browser, and changing its width
  (form, corner handle, CLI `shape set --width`) keeps the radius at half the new width.
  (A rectangle's length is never shorter than its width when drawn as a slot; if the user
  makes it so, the radius follows the shorter side.)
- Refused: both ends at one point; width zero.

## 5. Step 5: polygons (schema 30)

A new drawn kind, so the number of sides stays editable:

```json
{"id": 12, "layer": "0", "kind": "polygon", "center": [50.0, 30.0], "radius": 20.0,
 "sides": 6, "angle": 0.0}
```

- **radius** to the corners; **angle** of the first corner from +X, CCW, radians in the
  file. **sides** a whole number, 3 to 1000. Validation: finite, radius > `CHAIN_TOL_MM`.
- **Segments:** `sides` lines, counter-clockwise, starting at the first corner.
- **Schema 30:** older Chiploads would refuse the kind as unknown; the bump makes them say
  "made by a newer Chipload". Migration 29 → 30 is the identity. Frozen v29 sample added;
  it migrates and gives its golden G-code unchanged.
- **Drawing (N):** the centre, then a corner (radius and angle). The number of sides is a
  field on the status bar while the tool is active (default 6, remembered for the session).
- **Form:** centre X, Y; sides; radius (corners); **across flats** (2 · r · cos(π / n),
  editable, changes the radius); angle (°).
- **Handles:** centre (moves it); the first corner (radius and angle). The box, mirror,
  copy, paste, Browser group *Polygons*, names, corners for dogbones: as for rectangles.
- **CLI:** `chipload shape add P polygon --center 50,30 --radius 20 --sides 6
  [--angle-deg 0]` (or `--across-flats 34.64` in place of `--radius`); `shape set` the
  same values.
- *Implemented:* the drag handles are the centre and the first corner (`Handle::Center`,
  `Handle::Radius`); a single selected polygon has no dashed box, as a circle and a
  rectangle have none. Mirroring left-right gives its first corner the angle π − angle.
  The golden `v30_polygon` (a hexagon island in the rounded rectangle) is new; no other
  G-code changed. The shape tools sit in the ribbon as small icon buttons, three to a row
  (the ribbon had no room for three more large ones).

## 6. Step 6: lengths on the canvas

With one drawn shape selected (top view): labels at its segments' middles (straight
polyline segments: length; rectangles: length and width; circles and polygons: radius).
**Clicking a label** opens the entry there with the value; Enter sets it:

- a polyline segment: its end point moves along the segment so it gets that length (the
  start stays; the rest of the polyline is unchanged);
- a rectangle's length or width: about its centre, as the form does;
- a circle's or polygon's radius.

Same validation as the form; one undo step.

*Implemented:* the labels stand off their side, outward (to the right of a
counter-clockwise loop, to the left of an open polyline), clear of the side's middle
handle, and a side too short on screen for its label shows none. The rules are
`chipload_job::edit::dimensions` and `with_dimension`, beside the handles.

## 7. Tests

- **Snaps** (`chipload-draw`): line × line, line × arc (two, one tangent, none), arc × arc;
  intersections only on the segments, not their extensions; perpendicular and tangent from a
  point, on and off the segment; quadrants only within an arc's sweep; the choosing rules
  (nearest, tie order, Nearest last); only segments near the cursor are looked at.
- **Typed values:** every form of §2 in mm and inches, with units, the semicolon form,
  refusals; Shift steps at 0°, 15°, 345°, with grid lengths.
- **Moving:** a drag on a selected shape moves the whole selection; on an unselected one,
  only it; snapped grab point; one undo step; operations keep their shapes.
- **Box selection:** inside versus crossing for lines, an arc whose ends are outside but
  bulge in, a circle round the box (crossing: no, it doesn't touch; inside: no).
- **Arc and slot:** the geometry from clicks and typed values; more than a half circle
  splits; refusals; the slot keeps its radius when its width changes.
- **Polygon:** round trip, validation, v29 → v30, segments (CCW, first corner), across
  flats, handles, mirror, the box; a pocket of a hexagon passes the checker.
- Golden G-code unchanged; the frozen v29 sample's golden unchanged.
- GUI: the app-level tests drive the same functions the clicks and keys call.

## 8. Decisions (settled with the roadmap, 2026-10-27)

1. Snap kinds and their order as §1.
2. One entry with a small grammar (§2) rather than separate length and angle fields; it
   is quicker to type and to test.
3. Box selection and moving take the left drag in top view (moving when the drag starts on
   a drawn shape, the user's addition the same day); panning moves to right, middle or
   Space + drag, as in the 3D view.
4. The arc tool is start, end, through point (plus a typed radius); arcs by centre come
   with trim in phase 2 (a circle, trimmed).
5. A slot is a rectangle that stays a slot (§4); a polygon is its own kind (§5).
