# Parts

The operations of a project are grouped into **parts**. A part is a name, the operations
that cut it, its [copies](../drawings/copies.md), and an on/off switch. Most projects have
one part. Use several when one sheet holds different things, for example 4 lids and 2
boxes.

- **What a part cuts:** its operations select shapes, DXFs and layers as usual. A part
  owns no geometry, so one DXF with a layer per part can feed several parts.
- **Order:** part by part. Each part is cut with all its copies before the next one
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
- **Export writes every part** that is on. When only the active part is shown, Export
  plans the whole project first, then writes it.
- **One part's G-code:** right-click a part in the Browser and choose **Export G-code of
  this part…**. You get the program the project gives with every other part switched off:
  that part with all its copies, the same stock, work zero and machine, checked as every
  program is. It works for a part that is off too. The file name suggested is the
  project's and the part's (`box-lid.nc`), and the setup sheet, when one is written, is of
  that part and names it. The project isn't changed. On the command line, `--part ID` on
  `run`, `sheet` and `plan` does the same, and can be given more than once.
- **Stock:** all parts share the project's stock and work zero. Parts cut from a different
  material belong in another project.
- **Safety:**
  - Copies of a part must not cut into each other.
  - A copy must not cut into another part. The parts as drawn may overlap, for example an
    engraving over a cut-out.
  - Overlaps are refused.
- **In the G-code:** with two parts or more, a comment marks where each part starts:
  `(Part 2: box, 2 copies)`.

```sh
chipload part list part.chipload                        # id, name, copies, operations
chipload part add part.chipload --name box              # an empty part, now active
chipload op part part.chipload --op 4 --part 2          # move operation 4 into part 2
chipload copies part.chipload --part 2 --grid 2x1 --gap 10
chipload part set part.chipload 2 --name lid --off      # rename, turn off (--on)
chipload part activate part.chipload 1
chipload run part.chipload -o lid.nc --part 2           # only part 2's G-code
chipload part remove part.chipload 2 --with-operations  # without the flag: refused while it has operations
```

In the app, the [Browser](../window.md#the-browser) lists the parts with their operations.
Click a part to see its copies in Properties. Right-click it to activate, export, move or
delete it,
or right-click *Parts* for a new one. Drag an operation onto a part to move it there.

![Two parts in the Browser: part B is active, cut twice, and its properties are
open](../images/parts.png)
