# Placing the drawing

The imported drawing can be moved, turned by any angle and scaled on the stock, without
changing the DXF. In the app, click the DXF in the [Browser](../window.md#the-browser): its
**Position, rotation and scale** are in Properties.

| Row | What it is | A change |
|---|---|---|
| **Position** X Y | the lower-left corner of the drawing's box, mm | moves the drawing so its corner is there. *Bottom left* puts it 10 mm in from the stock's corner, *Centre* on the stock's centre. |
| **Rotation** | degrees counter-clockwise from the DXF as drawn, any angle | turns it about its centre; ⟲ 90 and ⟳ 90 turn it a quarter |
| **Scale** | the factor from the DXF as drawn | scales it about its centre |
| **Size** | the box's width and height | |

Each row has a reset button: the position back to where the DXF has it, rotation 0, scale 1.
A change applies when you finish the edit (Enter, Tab or releasing a drag), as one undo
step.

On the command line, `chipload place` (for a picture, `--image ID` with `--at`, `--rotate`
and `--width`):

| | Command line |
|---|---|
| Put the box's lower-left corner at a point | `--at 10,20` |
| Move by | `--by 10,-5` |
| Put at an offset from where the DXF has it | `--to 10,-5` |
| Align with the stock | `--align bottom-left --margin 10`, `--align center` |
| Turn about its centre, on top of any turn | `--rotate 30` (any angle; negative: clockwise) |
| Scale about its centre | `--scale 2`, `--width 100`, `--height 50` |
| Back to as drawn | `--reset` |

- **Order:** a turn and a scale come before an alignment, so `--rotate 90 --align
  bottom-left` turns the part and then puts it in the corner.
- **Saved as values:** the placement is kept in the project, and [updating from a new
  DXF](update.md) puts the new drawing in the same place.
- **What follows:** operations follow the drawing, and their start points move with it.
- **Arcs:** scaling and turning keep arcs exact. Turns by 90°, 180° and 270° are exact
  to the bit; other angles are as exact as the computer's arithmetic.
- **Where it is now:** `chipload place part.chipload` with no option prints the placement
  and size.

Shapes drawn in Chipload aren't part of the imported drawing and stay where they are. With
[several DXFs](several.md), each is placed on its own: `--dxf ID` says which.

To have the G-code's zero somewhere else, without moving anything, set the [work
zero](../concepts/stock.md#work-zero) instead.
