# Copies

Each [part](../operations/parts.md) can be cut several times. Every extra time is an
**instance** of the part: the same operations, cut again at another spot. Click the part in
the Browser and make a grid under **Instances** in its properties, or use `chipload copies`.
With several parts, give `--part`.

```sh
chipload copies part.chipload --grid 3x2 --gap 10     # 3 columns, 2 rows, 10 mm between cuts
chipload copies part.chipload --grid 3x2 --pitch 120,80
chipload copies part.chipload --order by-instance     # every operation on one instance, then the next
chipload copies part.chipload --off                   # no instances: the part is cut once
chipload copies part.chipload --part 2 --grid 2x1 --gap 10   # with several parts
```

- **A grid makes instances:** the part is the grid's first cell, and the other cells
  become its instances, numbered row by row from 2. Making a grid again replaces them.
  `--pitch` and `--gap` go with `--grid`.
- **Pitch:** the distance from cell to cell. *Gap* works the pitch out from the part's
  real cut, tool included, so the cuts of neighbours stay that far apart. A negative pitch
  grows the grid left or down.
- **Order:**
  - **by operation** (the default): each operation on every instance, then the next, so
    tool changes happen once;
  - **by instance:** every operation on one instance, then the next. The instances are cut
    in their listed order, a grid's row by row, back and forth. This was called *by copy*,
    and *by part* before Chipload 0.17; `--order by-copy` and `--order by-part` still
    work.
- **Checked:**
  - every instance is planned and checked like the part;
  - instances whose cuts would overlap each other, or another part, are refused;
  - instances have to fit the stock and the machine like anything else.
- **Limits:** up to 100 columns or rows, and 1 000 instances.
- **Display:** the canvas draws the instances faded: each part's instances of the shapes it
  cuts. The setup sheet and the playback say which instance is being cut.

Instances only move for now. They aren't nested: to fit different parts on one sheet, give
each its own part and grid, or arrange them in CAD.

Projects saved before Chipload 0.38 kept a grid of copies; they open with that grid as
instances, cut in the same order, so the G-code stays the same.
