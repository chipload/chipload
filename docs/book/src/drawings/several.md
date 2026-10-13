# Several drawings

A project can hold several DXFs, for example two parts cut from one sheet, besides its
[images](images.md) and the shapes you draw. Each DXF has its own place, turn and scale, and
is updated from its own file.

```
chipload dxf add part.chipload lid.dxf --name lid     # a second DXF, where its file has it
chipload dxf list part.chipload                       # id, name, file, placement, size
chipload place part.chipload --dxf 2 --by 250,0       # move, turn or scale that one
chipload update part.chipload --from lid.dxf --dxf 2  # a new version of that one
chipload dxf remove part.chipload 2                   # it and its shapes
```

In the app: *File > Add DXF to project…*, or drop the DXF on the window.

## The Drawing panel

The **Drawing** panel, under the operations, lists what the project's shapes come from:

- **Each DXF**, with its layers inside.
- **Each image.**
- **The shapes drawn in Chipload**, by kind: rectangles, circles, polylines.

![The Drawing panel: two DXFs with their layers, the second hidden, an image, and the
first DXF's placement window](../images/drawing.png)

- **Select** selects the whole group, or one layer of one DXF. An operation added from a
  DXF's layer cuts only that DXF's layer (its selector names the DXF).
- **⋯** has the rest:
  - **For a DXF:** *Place…* (move, turn, scale: [Placing the drawing](placement.md)),
    *Update from a DXF…* or *from a path…*, and *Remove*.
  - **For an image:** *Edit…* and *Remove*.
- **The checkbox hides** a group or a layer on the canvas: it isn't drawn and can't be
  clicked. **Hidden shapes are still cut.** An operation that cuts some says so on its card
  ("cuts 3 hidden contours"), and its toolpath still shows. To stop cutting them, turn the
  operation off.
- **Saved:** what is hidden is saved in the project (`drawing.hidden`). Changing it is a
  step you can undo.

*File > Update drawing from DXF…* works while the project has one DXF. With several, update
each from its ⋯ menu.

- **Where it goes:** a new DXF lands where its file has it, often on top of the first.
  [Place](placement.md) it afterwards.
- **Which one:** with several DXFs, `place` and `update` need `--dxf ID`. With one, they
  work as before.
- **Name:** the file's name without its extension, unless you give `--name`. A second copy
  of the same file is named "part 2".
- **Removing** a DXF removes its shapes. The report names the operations that used them;
  they can't run until you give them geometry again.

## The same layer in two DXFs

An operation that selects a layer cuts that layer in every DXF: both parts' "HOLES". When a
layer is in more than one, the report says so: *layer "HOLES" is in lid and part; it cuts
all of them*. To cut only one part's layer, limit the selector to its DXF:

```json
"select": {"layer": "HOLES", "dxf": 2}
```

A selector's `dxf` (or `image`, for a picture's traced shapes) keeps it to that source's
shapes. Without one, it means every source, as it always did.

The [setup sheet](../checking/sheet.md) lists every DXF and image with where it was
put, when there is more than one.
