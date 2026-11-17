# Parts

The operations of a project are grouped into **parts**. A part is a name, the operations
that cut it, its [instances](../drawings/instances.md), and an on/off switch. Most projects have
one part. Use several when one sheet holds different things, for example 4 lids and 2
boxes.

- **What a part cuts:** its operations select shapes, DXFs and layers as usual. A part
  owns no geometry, so one DXF with a layer per part can feed several parts.
- **Order:** part by part. Each part is cut with all its instances before the next one
  starts, and within a part its operations run in order. Operations are numbered across
  the whole project (1, 2, 3 …), as the G-code comments and the report number them.
- **Tool changes:** a tool used in two parts is changed back for the second part. Two
  parts that each use T1 and then T2 change tool three times. The report counts the
  changes.
- **Off:** a part that is off is left out with all its operations.
- **The active part:** new operations go into it. Adding a part makes the new one active.
- **Seeing one part:** with two parts or more, Generate, the automatic update, the
  simulation and playback show only the active part, so a change to part 2 doesn't plan
  part 1 again. The status bar says "part 2 of 2 · cycle …". **All parts** on the canvas
  bar shows every part.
- **Statistics:** the last section of a part's properties gives its time, lengths and
  lifts, its operations' summed with all of its instances
  ([Statistics of a part](../checking/time.md#statistics-of-a-part)).
- **Export writes every part** that is on. When only the active part is shown, Export
  plans the whole project first, then writes it.
- **One part's G-code:** right-click a part in the Browser and choose **Export G-code of
  this part…**. You get the program the project gives with every other part switched off:
  that part with all its instances, the same stock, work zero and machine, checked as every
  program is. It works for a part that is off too. The file name suggested is the
  project's and the part's (`box-lid.nc`), and the setup sheet, when one is written, is of
  that part and names it. The project isn't changed. On the command line, `--part ID` on
  `run`, `sheet` and `plan` does the same, and can be given more than once.
- **Stock:** all parts share the project's stock and work zero. Parts cut from a different
  material belong in another project.
- **Safety:**
  - Instances of a part must not cut into each other.
  - An instance must not cut into another part. The parts as drawn may overlap, for example an
    engraving over a cut-out.
  - Overlaps are refused.
- **In the G-code:** with two parts or more, a comment marks where each part starts:
  `(Part 2: box, cut 2 times)`.

```sh
chipload part list part.chipload                        # id, name, instances, operations
chipload part add part.chipload --name box              # an empty part, now active
chipload op part part.chipload --op 4 --part 2          # move operation 4 into part 2
chipload part instance add part.chipload --part 2   # part 2 placed again (see Instances)
chipload part set part.chipload 2 --name lid --off      # rename, turn off (--on)
chipload part activate part.chipload 1
chipload run part.chipload -o lid.nc --part 2           # only part 2's G-code
chipload part remove part.chipload 2 --with-operations  # without the flag: refused while it has operations
```

In the app, the [Browser](../window.md#the-browser) lists the parts under their
[setup](../concepts/setups.md), with their operations. Click a part to see its instances in
Properties. Right-click it to activate, export, place again (an instance), move or delete it, or right-click its setup
for a new one. Drag an operation onto a part to move it there.

![Two parts in the Browser: part B is active, cut twice, and its properties are
open](../images/parts.png)

## A part's stock

A part can say what it is cut from: a material from your [library](../library/materials.md),
a thickness, or both. Set it in the part's properties under **Stock**, or with `part set
--material` and `--thickness`. Left as *Any board*, as it starts, it asks for nothing.

- **It picks the board:** a part whose board isn't that stock moves, with every instance of
  it, to the first board that is. With none, Chipload makes one like the board it leaves
  (its size or fit, its work zero, Z zero and overcut), with the new thickness or material,
  and the Browser shows the two [setups](../concepts/setups.md). The status bar says where
  it went: "Made Setup 2 (6 mm) for Bottom". One undo puts it back.
- **Through cuts follow:** a cut *through the stock* goes through the new board.
- **Back again:** *Any board* (or `--stock none`) takes it back to the first board; a board
  Chipload made and nothing is cut from any more goes with it. A board you renamed or
  changed is yours, and stays.
- **By hand:** a part moved by hand onto a board of another stock stays there; the report
  says so: "Bottom is to be cut from 6 mm, but this board is 18 mm". A two-sided board or
  one in pieces never moves a part.

A DXF with a side and a bottom, the bottom from 6 mm plywood, the rest from 18 mm:

```sh
chipload import box.dxf -o box.chipload                 # one board, 18 mm
chipload part set box.chipload 2 --thickness 6mm        # "Made Setup 2 (6 mm) for Bottom"
chipload run box.chipload -o box.nc                     # box-S1.nc and box-S2.nc, a sheet each
chipload part set box.chipload 2 --stock none           # back on the 18 mm board
```

## Mirrored parts

A part can be cut **mirrored**, left to right or top to bottom, while the drawing stays as
you drew it. Set **Mirror** in the part's properties, or `chipload part set --mirror`. Use it
for left- and right-hand pieces from one drawing, the plug of an inlay, a stamp or branding
iron whose text must read backwards, or a sign cut from the back of clear acrylic.

- **About its own centre:** the part is mirrored about the centre line of the shapes its
  operations cut, so it stays where it is on the board and a board that fits its parts
  still fits. To cut a mirrored twin somewhere else, mirror a copy of its shapes in the
  drawing instead (`shape mirror`).
- **Settings mean what they say:** inside stays inside, climb stays climb. Chipload plans
  the part as drawn with the cutting direction turned, then mirrors it, so a climb cut
  is still a climb cut on the mirrored part. Tabs placed by hand, dogbone corners and ring
  starts stay on their feature.
- **On the canvas:** the toolpath and the simulation show the part as it is cut, and its
  shapes are drawn there dashed and faded. You still edit the shapes as drawn; the start
  and tab markers stay on them. The Browser marks the part *mirrored*.
- **What isn't mirrored:** a surface pass and dowel holes belong to the board, not the part.
- **Instances:** each instance is the mirrored part, placed.
- **Text:** a mirrored part's text reads backwards on the board, and the report says so.
  On the [back of a board](../concepts/setups.md) the two mirrors cancel and it reads right.
- **The setup sheet** says which parts are cut mirrored.

```sh
chipload part set part.chipload 1 --mirror left-right   # also top-bottom; none: as drawn
```
