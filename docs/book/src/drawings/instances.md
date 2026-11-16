# Instances

Each [part](../operations/parts.md) can be cut several times. Every extra time is an
**instance** of the part: the same shapes and operations, cut again somewhere else, moved,
turned and perhaps mirrored. Instances are linked to the part: change the part, and every
instance follows.

In the window:
- **Right-click the part** in the Browser: *Add instance* places it again beside what it
  cuts, 10 mm away; *Add mirrored instance* does the same, mirrored left to right.
- **On the canvas**, each instance is drawn faded where it is cut, with its number (`#2`);
  one that is off is dashed. Click one to select it, drag it to move it, and drag its
  round handle to turn it about its middle. Turning snaps to quarter turns within 3°; hold
  Alt to turn freely. Escape during a drag puts it back; each drag is one undo step.
- **In the Browser**, each instance has a row under its part, with its tick. Right-click
  it to *Mirror* it, *Turn 90°* or *Remove* it; Delete removes the selected one.
- **Its properties** set whether it is cut, its mirror, its rotation (any angle) and how
  far it is moved.
- **A grid:** the part's properties make one under **Instances**.

On the command line, `chipload part instance` places them one by one. With several parts,
give `--part`.

```sh
chipload part instance add part.chipload                  # beside the part, 10 mm from its cut
chipload part instance add part.chipload --angle 90 --mirror left-right
chipload part instance add part.chipload --at 300,0       # moved 300 mm right of the part
chipload part instance list part.chipload                 # their numbers, places and turns
chipload part instance set part.chipload 3 --angle 45     # turn instance 3 (also --at, --mirror, --off, --on)
chipload part instance remove part.chipload 3             # or --all: the part is cut once
chipload part instance array part.chipload --grid 3x2 --gap 10   # a grid in place of them
chipload part set part.chipload 1 --order by-instance     # every operation on one instance, then the next
```

- **Numbers:** the part itself is instance 1 ([placed](#the-part-itself) or where drawn);
  the others are numbered from 2 and keep their numbers when one is removed.
- **Place:** an instance is mirrored first (`left-right` or `top-bottom`), then turned
  (`--angle`, degrees counter-clockwise), both about the middle of the part's box, then
  moved by `--at` from where the part is drawn. A new instance without `--at` goes
  beside everything the part cuts, 10 mm away, its bottom level with the part's.
- **Climb stays climb:** a mirrored instance is cut in the same direction as the part,
  not as a plain mirror image would be. Tabs, starts, tabs along X or Y and a pattern's
  lines turn with the instance, so every instance comes out the same.
- **A grid:** `part instance array` (or the **Instances** section) makes the part the
  grid's first cell and the other cells its instances, numbered row by row from 2, in
  place of the instances it had. *Pitch* is the distance from cell to cell; *gap* works
  the pitch out from the part's real cut, tool included. A negative pitch grows the grid
  left or down. `chipload copies` is the same command under its old name, kept for
  scripts: `copies --grid 3x2 --gap 10`, `copies --order by-instance`, and `copies --off`
  removes every instance.
- **Order:**
  - **by operation** (the default): each operation on every instance, then the next, so
    tool changes happen once;
  - **by instance:** every operation on one instance, then the next. The instances are cut
    in their listed order, a grid's row by row, back and forth. This was called *by copy*,
    and *by part* before Chipload 0.17; `--order by-copy` and `--order by-part` still
    work.
- **Checked:**
  - every instance is planned and checked like the part;
  - instances whose cuts would overlap each other, or another part, are refused when the
    project is run, and the commands warn as soon as you place one there;
  - instances have to fit the stock and the machine like anything else.
- **Limits:** up to 100 columns or rows in a grid, and 1 000 instances.
- **Display:** the canvas and the setup sheet draw the instances faded where they are cut.
  The setup sheet and the playback say which instance is being cut, and the sheet says
  where a part placed elsewhere than drawn is cut.

## The part itself

The part itself, instance 1, can be placed too: cut moved and turned like an instance,
while its shapes stay where they are drawn. Its shapes are what you edit; every instance,
the first included, is a placement of them. This lets you lay parts out on a sheet without
moving the drawing, for instance when several parts come from one DXF.

In the window:
- **Right-click the part** in the Browser and choose *Place…*: its form opens as
  **Instance 1**, with its rotation and how far it is moved. *Reset placement* puts it back
  where drawn.
- **On the canvas**, once placed, it is a ghost `#1` where it is cut, picked, dragged and
  turned like any instance. Its shapes, where they are drawn, get a dashed box saying
  "not cut here"; edit them there, and every instance follows.
- **In the Browser**, a part with instances or a placement lists itself first, as
  Instance 1; its tick is the part's.
- **The Compass** (Q) works on a selected instance, the first included: drag an arrow to
  move it along, the ring to turn it, the centre to put it on a point, or type a value.
  It moves and turns instances only; copies and arrays are for shapes.

```sh
chipload part place part.chipload --at 300,0 --angle 90   # the part cut 300 mm right, turned
chipload part place part.chipload --reset                 # back where drawn
chipload part instance set part.chipload 1 --at 350,0     # the same, as instance 1
```

- **Turned and moved only:** the part's own mirror (`part set --mirror`) mirrors its
  shapes, so every instance; instance 1 has no mirror of its own, can't be switched off
  alone (switch the part off) and can't be removed.
- **Its instances stay** where they were: they are placed from the part as drawn, not from
  instance 1.
- **The board:** a board that fits the drawing holds the part where it is cut, not its
  shapes where they are drawn.
- **Dowel holes** belong to the board: they are cut where drawn, once, never placed or
  repeated per instance.
- **Checked** like an instance: a placed part may not cut into another part. Parts cut
  where they are drawn may still overlap each other, as before.
- In a setup cut in [pieces](../concepts/pieces.md) the part can't be placed yet.

Instances aren't nested automatically yet: to fit different parts on one sheet, place each
part and its instances, or arrange them in CAD.

Projects saved before Chipload 0.38 kept a grid of copies; they open with that grid as
instances, cut in the same order, so the G-code stays the same.
