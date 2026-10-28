# Drawing shapes

Chipload draws the parts it cuts, with or without a DXF:

- **Circle** (key **C**): the centre, then a point on it.
- **Rectangle** (**R**): two opposite corners. It keeps its centre, length and width, a
  rotation, and a corner radius (0: sharp corners).
- **Polyline** (**L**): points joined by straight or arc segments, open or closed. Click
  the first point again to close it; Enter or a double-click finishes it open.
- **Arc** (**A**): where it starts, where it ends, then a point it passes through (or type
  its radius: it bows to the cursor's side). It becomes an open polyline of one or two arc
  segments.
- **Slot** (**S**): the centres of its two ends, then its width (a click, or type it). It
  is a rectangle whose corner radius is half its width; it shows as *Slot*, and stays one
  when you change its width.
- **Polygon** (**N**): the centre, then a corner. Its number of sides is on the status bar
  while you draw (6 to start with, then the last you chose). Its form has the radius to
  the corners and the size **across flats**, either of which sets the other (a hexagon
  nut's spanner size), and the angle of its first corner. `chipload shape add P polygon
  --center 50,30 --sides 6 --across-flats 13`.

Pick the tool in the ribbon's DRAW group (the small buttons; hover over one for its name
and key) or by its key, then click on the canvas. With no
tool active, **Enter** starts the last one again. Clicks snap to the drawing (below), else
to the [grid](../window.md#the-canvas), whose step the status bar gives ("grid 1 mm").

## Snapping

Within 8 pixels of the cursor, a click or a drag lands exactly on a point of the drawing,
marked with a small diamond and named on the status bar ("intersection of #3 and #4"):

- **End** of a line or arc, **intersection** where two cross or touch, **centre** of an arc
  or circle, **quadrant** (an arc's points at 0°, 90°, 180° and 270°), **middle** of a line
  or arc.
- While drawing a polyline or measuring, from the last point: **perpendicular** to a line
  or arc, and **tangent** to an arc.
- **On** a line or arc: the nearest point of it, when none of the others is near.

The nearest of them wins; at the same spot, they count in the order above. Hold **Alt** to
turn snapping off.

## Exact values

While a tool waits for a point, type a number: an entry opens beside the cursor. **Enter**
uses it, **Esc** closes it.

| Type | For |
|---|---|
| `120,40` | the point X 120, Y 40 |
| `@30,-10` | 30 to the right of the last point and 10 down |
| `30` | 30 from the last point, towards the cursor |
| `30<45` | 30 from the last point at 45° (counter-clockwise from the X axis) |

Lengths are in the units the app shows, unless you name them: `1/2in`, `12mm`. Where the
decimal point is a comma, separate X and Y with a semicolon: `12,5;30`. For a circle, a
plain length after the centre is its radius. The preview shows where the value lands before
you press Enter; a value that can't be used turns red and the status bar says why.

![Drawing a polyline: 40<120 typed, the preview at 120° and the length and angle beside the
cursor](../images/typing.png)

Beside the cursor, the size being drawn: the line's length and angle, a circle's radius, a
rectangle's length × width. Hold **Shift** to keep the line from the last point at a
multiple of 15° (0° and 90° for straight lines), its length on the grid.

## Editing operations

The ribbon's MODIFY group (and the *Modify* menu) holds tools that make new shapes from
drawn ones. Shapes from a DXF, a picture or a text can't be changed by them (explode a
DXF first, [Explode](several.md)).

- **Offset** (**O**): select the shapes (or click one with the tool), then move the cursor
  to the side you want, outside or inside a closed shape, left or right of an open one:
  the preview follows at the cursor's distance on the grid (Alt: off the grid). Click, or
  type the distance and press Enter. The status bar has the corners, *round* (the exact
  offset, arcs round the corners) or *sharp* (the sides extended until they meet, as a
  frame), and a *Count* for several rings, each the distance further. The shapes stay; the
  new ones go on the same layer and are selected. A circle, a rectangle or slot, and a
  polygon inward or with sharp corners keep their kind; anything else becomes a polyline
  with true arcs. On the command line: `chipload shape offset P 1 2 --distance 3
  [--side inside] [--corners sharp] [--count 2]`.
- **Fillet** and **Chamfer**: select one drawn shape (or click it with the tool), then
  click a corner (it is marked when the cursor is near it) to round it with an arc of the
  *Radius* on the status bar, or cut it straight across, the *Distance* back along each
  side. Shift+click does every corner. A corner is a point between two straight sides; a
  radius too large for the sides is refused with the most that fits (two corners on one
  side share it). A rectangle rounded all round keeps its kind (its corner radius); any
  other shape becomes a polyline and keeps its number, so operations still cut it.
  `chipload shape fillet P 3 --radius 2 [--corner 0]`, `chipload shape chamfer P 3
  --distance 1`.
- **Trim** (**X**), **Extend** and **Break** work on the drawn shape under the cursor; no
  need to select it first. Trim shows the piece it will remove before you click, Extend
  the shape grown to the next one ahead (click near the end to extend), Break a mark where
  it will split. Each click is one step; Esc ends the tool. On the command line:
  - `chipload shape trim P ID --at X,Y` removes the piece of a drawn shape at that point,
    between the nearest places where shown shapes cross it: an open shape cut in its
    middle becomes two, a closed one opens, a circle becomes an arc.
  - `chipload shape extend P ID --end start|end` lengthens a straight end along itself
    to the next shape ahead.
  - `chipload shape break P ID --at X,Y` splits an open shape there, or opens a closed
    one.
  - The shape keeps its id for the first piece, so operations follow it.
- **Union**, **Subtract** and **Intersect** combine closed drawn shapes, with true arcs
  where they have arcs: a union is everything any of them covers, a subtraction the first
  shape with the others cut out of it, an intersection what they all cover. The result
  takes the first shape's number, so operations that cut it follow; the other shapes go,
  and operations that cut those are flagged until you choose their geometry again. A
  result that is one of the shapes unchanged (a hole cut where a circle already lies, a
  shape a union swallows whole) stays as drawn, a circle still a circle; anything else
  becomes a polyline. A ring of shapes united gives the outline and the hole inside as
  two shapes. Open shapes are refused, and so is a result with nothing left. In the
  window: select the shapes, then *Union* or *Intersect* acts at once (the shape drawn
  first keeps its number). For *Subtract*, select the shapes to cut out, choose
  *Subtract*, then click the shape to cut them from: the preview shows the result under
  the cursor. On the command line: `chipload shape union P 1 2 3`, `chipload shape subtract P 1 4 5` (4
  and 5 cut out of 1), `chipload shape intersect P 1 2`.
- **Arrays** add copies of drawn shapes in a grid or round a centre; the originals stay
  and the copies go on their layers with new numbers. A grid counts the originals in its
  columns and rows and spaces them centre to centre (negative goes left or down). A polar
  array counts the originals too: over 360° the copies spread evenly round, over a
  smaller angle the first and last sit at its ends; each copy turns with its place unless
  you say not to. This adds shapes to the drawing; the project's
  [grid of copies](copies.md) is different, it repeats the whole part with
  its operations. In the window: select the shapes, choose *Array* or *Polar array*,
  and set the copies under *Selection*; for a polar array click the centre on the canvas
  (it snaps as drawing does). The preview shows the copies; *Create* adds them. On the
  command line: `chipload shape array P 2 3 --columns 4 --rows 2
  --dx 30 --dy 20`, `chipload shape array-polar P 5 --count 6 --center 100,80
  [--angle-deg 180] [--no-rotate]`.
- **Align** and **Distribute** sit under *Selection* when drawn shapes are selected.
  Align lines up each shape's box (its left or right side, top or bottom, or middle) with
  the box round them all, or, with *To the stock*, with the stock's (then one shape is
  enough). Distribute, with three shapes or more, evens out the gaps between their boxes
  across or up and down; the outermost two stay put. Shapes move whole and keep their
  numbers. On the command line: `chipload shape align P 2 3 --to left|centre|right|top|
  middle|bottom [--stock]`, `chipload shape distribute P 2 3 4 --horizontal` (or
  `--vertical`).
- New shapes on a layer that an operation selects by its name are cut by that operation
  too, as a shape drawn there is; the status bar names those operations.

## Editing

- **Moving with the mouse:** drag a drawn shape to move it; if it is selected, the whole
  selection moves with it. The point you grab snaps as drawing does, so a corner can be
  dropped exactly on another shape's corner or a crossing; Shift keeps the move to 15°
  steps, Alt turns snapping off. Select several with a box first (drag on empty canvas,
  [the canvas](../window.md#the-canvas)). Shapes from a DXF, a picture or a text move as a
  whole with their own handles.
- **Handles:** drag a selected shape's handles to change it. A drag snaps as drawing does;
  hold Alt to turn snapping off.
- **Exact values:** under *Selection* in the [Properties](../window.md#properties) panel, a
  selected shape has its exact numbers and its layer.
- **Lengths on the canvas:** a selected drawn shape shows its lengths beside it: each
  straight side of a polyline, a rectangle's length and width, a circle's or polygon's
  radius (a side too short on screen to hold its label shows none until you zoom in).
  Click one, type the new length and press Enter: a polyline side keeps its start and its
  end moves along it, a rectangle keeps its centre.
- **Rounded corners:** a rectangle's *Corner radius* rounds all four corners with true arcs
  (G2/G3 in the program). It can be at most half the shorter side: half of it rounds the
  short ends fully, into a slot. A larger radius is refused rather than shrunk, and a corner
  handle won't drag the rectangle smaller than its corners allow; lower the radius first.
  The reset button makes the corners sharp again.
- **Move, turn and size:** select a polyline, or several drawn shapes (Ctrl+click), and a
  dashed box shows round them: drag the square in the middle to move them, the round handle
  beyond the right side to turn them (in steps of 15°; hold Alt for 0.1°), a corner to
  scale them evenly, the opposite corner staying put. One circle or rectangle keeps its own
  handles instead. The shapes keep their ids, so the operations that cut them follow, and so
  do the operations' start points, [corner choices](../operations/profile.md#corner-by-corner)
  and tabs placed by hand on them (also with `shape move`, `turn` and `scale`, and when you
  change one shape through its own handles or form: a corner's choice stays with that
  corner of a rectangle you move or resize).
- **Position and Turn:** a selected polyline's form, and the *Selection* section when several
  drawn shapes are selected, have *Position* (the lower-left corner of the box round them:
  type a value to move them there) and *Turn*: ⟲ 90, ⟳ 90, or type an angle and press
  Enter. A polyline keeps no angle of its own; turning moves its points.
- **In the Browser:** under *Drawing*, the *Circles*, *Rectangles*, *Polylines* and
  *Polygons* groups list their shapes, in a folder per layer when there are several. Click a row to select the
  shape (Ctrl+click adds or removes); right-click to rename, copy or delete it. A group
  with more than 20 shapes starts closed.
- **Names:** give a drawn shape a *Name* in its form (or *Rename…* in the Browser); the
  Browser shows it, with its number beside it. Empty: it's shown by its number. Copies
  start without a name.
- **Copy and paste:** Ctrl+C and Ctrl+V. *Draw > Paste* puts the copy 10 mm right and up;
  Ctrl+V over the canvas pastes at the cursor.
- **Mirror:** *Draw > Mirror left–right* and *Mirror up–down* flip shapes about the middle
  of their box.
- **Delete:** Del. Operations that cut the shape are flagged.
- **Undo:** Ctrl+Z.

Imported shapes can't be edited: they change only with the DXF, unless you
[explode it](explode.md). Lines whose ends almost meet can be joined into one shape with
*Draw > Join ends…* (same page).

## On the command line

```sh
chipload shape add part.chipload circle --center 40,30 --radius 5 --layer HOLES   # prints its id
chipload shape add part.chipload rectangle --center 80,30 --length 40 --width 10 --angle-deg 15
chipload shape set part.chipload 9 --corner-radius 3  # round the rectangle's corners
chipload shape add part.chipload polyline --points "0,0 50,0 50,20" --bulges "0 1" --open
chipload shape set part.chipload 8 --radius 6         # change a value or the layer
chipload shape set part.chipload 8 --name "Lid"       # its name in the Browser ("" takes it away)
chipload shape copy part.chipload 8 9 --dx 25 --dy 0  # copies, with new ids
chipload shape mirror part.chipload 8 9 --axis x      # flip in place; ids kept
chipload shape move part.chipload 8 9 --dx 10 --dy -5
chipload shape turn part.chipload 8 9 --angle-deg 30  # about the middle of their box (or --about X,Y)
chipload shape scale part.chipload 8 --factor 1.5
chipload shape remove part.chipload 8
chipload shape join part.chipload --gap 0.05   # join ends that almost meet
```

Bulges are as in DXF: tan(sweep / 4), 0 straight, 1 a counter-clockwise half circle, −1
clockwise.
