# Placing the drawing

The imported drawing can be moved, turned in 90° steps and scaled on the stock, without
changing the DXF. Job settings > **Drawing position**, or `chipload place`:

| | App | Command line |
|---|---|---|
| Move by | *move by X Y* | `--by 10,-5` |
| Put at an offset from where the DXF has it | | `--to 10,-5` |
| Align with the stock | *on the stock*: bottom left (with a margin) or centre | `--align bottom-left --margin 10`, `--align center` |
| Turn about its centre | *turn*: 90°, 180°, 270° | `--rotate 90` |
| Scale about its centre | *scale by*, or *size W H* | `--scale 2`, `--width 100`, `--height 50` |
| Back to as drawn | | `--reset` |

- **Order:** a turn and a scale come before an alignment, so `--rotate 90 --align
  bottom-left` turns the part and then puts it in the corner.
- **Saved as values:** the placement is kept in the project, and [updating from a new
  DXF](update.md) puts the new drawing in the same place.
- **What follows:** operations follow the drawing, and their start points move with it.
- **Arcs:** scaling keeps arcs exact.
- **Where it is now:** `chipload place part.chipload` with no option prints the placement
  and size.

Shapes drawn in Chipload aren't part of the imported drawing and stay where they are. With
[several DXFs](several.md), each is placed on its own: `--dxf ID` says which.

To have the G-code's zero somewhere else, without moving anything, set the [work
zero](../concepts/stock.md#work-zero) instead.
