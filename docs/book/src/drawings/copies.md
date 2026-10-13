# Copies

Each [part](../operations/parts.md) can be cut several times, in a grid of its own: click
the part in the Browser and set **Copies** in its properties, or use `chipload copies`.
With several parts, give `--part`.

```sh
chipload copies part.chipload --grid 3x2 --gap 10     # 3 columns, 2 rows, 10 mm between cuts
chipload copies part.chipload --grid 3x2 --pitch 120,80
chipload copies part.chipload --order by-copy         # every operation on one copy, then the next
chipload copies part.chipload --off                   # one copy again
chipload copies part.chipload --part 2 --grid 2x1 --gap 10   # with several parts
```

- **Pitch:** the distance from copy to copy. *Gap* works the pitch out from the part's real
  cut, tool included, so the cuts of neighbours stay that far apart. A negative pitch
  grows the grid left or down.
- **Order:**
  - **by operation** (the default): each operation on every copy, then the next, so tool
    changes happen once;
  - **by copy:** every operation on one copy, then the next (called *by part* before
    Chipload 0.17; `--order by-part` still works).
- **Checked:**
  - every copy is planned and checked like the original;
  - copies whose cuts would overlap each other, or another part, are refused;
  - copies have to fit the stock and the machine like anything else.
- **Limits:** up to 100 columns or rows, and 1 000 copies.
- **Display:** the canvas draws the copies faded: each part's copies of the shapes it cuts. The setup sheet and the playback say
  which copy is being cut.

Copies don't turn, and they aren't nested. To fit different parts on one sheet, give each its
own part and grid, or arrange them in CAD.
