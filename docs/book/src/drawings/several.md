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

## In the Browser

The **Drawing** folder of the [Browser](../window.md#the-browser) lists what the project's
shapes come from:

- **Each DXF**, with its layers inside.
- **Each image.**
- **The shapes drawn in Chipload**, by kind: rectangles, circles, polylines.

![The Browser: two DXFs with their layers, the second hidden, an image, and the
first DXF's position and scale in Properties](../images/drawing.png)

- **A click** selects the whole group, or one layer of one DXF; Ctrl+click adds. An
  operation added from it cuts only that (its selector names the DXF or image). A DXF or
  picture also shows its properties: position and scale, or size and place.
- **Right-click** has the rest:
  - **For a DXF:** *Place…* (its properties: [Placing the drawing](placement.md)),
    *Update from a DXF…* or *from a path…*, and *Remove*.
  - **For an image:** *Trace settings…* (also a double-click) and *Remove*.
- **The eye hides** a group or a layer on the canvas: it isn't drawn and can't be
  clicked. **Hidden shapes are still cut.** An operation that cuts some says so on its row
  ("3 hidden · T1"), and its toolpath still shows. To stop cutting them, untick the
  operation.
- **Saved:** what is hidden is saved in the project (`drawing.hidden`). Changing it is a
  step you can undo.

*File > Update drawing from DXF…* works while the project has one DXF. With several, update
each from its row's right-click menu.

- **Where it goes:** a new DXF lands where its file has it, often on top of the first.
  [Place](placement.md) it afterwards.
- **Which one:** with several DXFs, `place` and `update` need `--dxf ID`. With one, they
  work as before.
- **Name:** the file's name without its extension, unless you give `--name`. A second copy
  of the same file is named "part 2".
- **Removing** a DXF removes its shapes. The report names the operations that used them;
  they can't run until you give them geometry again.

## What an operation selects

An operation selects a **DXF or image as a whole**, a **layer** within one, or **shapes**:

```text
"select": {"dxf": 2}                       // all of part B, whatever its layers
"select": {"dxf": 2, "layer": "HOLES"}     // part B's holes
"select": {"image": 1}                     // a traced picture
"select": {"image": 1, "exclude": [863]}   // the picture but one of its shapes
"select": {"shapes": [12, 14]}             // these shapes
```

A DXF or an image selected as a whole, or one of its layers, follows the DXF when it is
updated and the picture when it is traced again. In the app:
- **A click** on a DXF or image in the Browser picks the whole DXF or image.
- **A click** on a layer in the Browser, or right-click *Select layer* on the canvas, picks
  that layer of that DXF only.
- **Leaving some out:** with a whole DXF or picture selected, **Ctrl+click** a contour on the
  canvas to leave it out, and again to put it back. The selection then reads "image 1, 1
  left out", and the operation's selector gets `exclude`.

When a picture is traced again (a new size, place or setting), its shapes get new numbers.
Left-out shapes are found again by where they sit in the picture, and the report says so.
One that can't be found (the tracing changed too much) gives a problem, and nothing is
generated until you pick the shapes to leave out again. Ctrl+Z undoes the retrace.

**Adding a DXF never changes what an existing operation cuts.** An operation that
selected a layer by its name alone, made while the project had one DXF, keeps cutting that
DXF when the new one has the same layer. The report says so: *Operations 1, 2 keep cutting
part only*.

A selector with a layer and no DXF (written by hand, or by Chipload before 0.16) means that
layer in every DXF. When a layer is in more than one, the report says so: *layer "HOLES" is
in lid and part; it cuts all of them*.

The [setup sheet](../checking/sheet.md) lists every DXF and image with where it was
put, when there is more than one.
