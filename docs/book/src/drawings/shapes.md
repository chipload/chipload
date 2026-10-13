# Drawing shapes

Chipload draws three kinds of shape, for the odd hole, slot or outline that isn't in the
DXF, or a project without one:

- **Circle:** centre and radius.
- **Rectangle:** centre, length and width, and a rotation.
- **Polyline:** points joined by straight or arc segments, open or closed.

**Circle**, **Rectangle** or **Polyline** in the ribbon's DRAW group, then click on the
canvas. Clicks snap to the drawing's ends, centres and middles, else to the
[grid](../window.md#the-canvas), whose step the status bar gives ("grid 1 mm"). Anything more
is CAD's job: draw it there and import the DXF.

## Editing

- **Handles:** drag a selected shape's handles to change it. A drag snaps to end points,
  centres and middles; hold Alt to turn snapping off.
- **Exact values:** under *Selection* in the [Properties](../window.md#properties) panel, a
  selected shape has its exact numbers and its layer.
- **Copy and paste:** Ctrl+C and Ctrl+V. *Draw > Paste* puts the copy 10 mm right and up;
  Ctrl+V over the canvas pastes at the cursor.
- **Mirror:** *Draw > Mirror left–right* and *Mirror up–down* flip shapes about the middle
  of their box.
- **Delete:** Del. Operations that cut the shape are flagged.
- **Undo:** Ctrl+Z.

Imported shapes can't be edited: they change only with the DXF.

## On the command line

```sh
chipload shape add part.chipload circle --center 40,30 --radius 5 --layer HOLES   # prints its id
chipload shape add part.chipload rectangle --center 80,30 --length 40 --width 10 --angle-deg 15
chipload shape add part.chipload polyline --points "0,0 50,0 50,20" --bulges "0 1" --open
chipload shape set part.chipload 8 --radius 6         # change a value or the layer
chipload shape copy part.chipload 8 9 --dx 25 --dy 0  # copies, with new ids
chipload shape mirror part.chipload 8 9 --axis x      # flip in place; ids kept
chipload shape delete part.chipload 8
```

Bulges are as in DXF: tan(sweep / 4), 0 straight, 1 a counter-clockwise half circle, −1
clockwise.
