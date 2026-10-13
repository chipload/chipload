# Copies

A project can cut its part several times, in a grid: Job settings > **Copies**, or
`chipload copies`.

```sh
chipload copies part.chipload --grid 3x2 --gap 10     # 3 columns, 2 rows, 10 mm between cuts
chipload copies part.chipload --grid 3x2 --pitch 120,80
chipload copies part.chipload --order by-part         # every operation on one copy, then the next
chipload copies part.chipload --off                   # one part again
```

- **Pitch:** the distance from copy to copy. *Gap* works the pitch out from the cut's real
  extent, tool included, so the cuts of neighbours stay that far apart. A negative pitch
  grows the grid left or down.
- **Order:**
  - **by operation** (the default): each operation on every copy, then the next, so tool
    changes happen once;
  - **by part:** every operation on one copy, then the next.
- **Checked:**
  - every copy is planned and checked like the original;
  - copies whose cuts would overlap are refused;
  - copies have to fit the stock and the machine like anything else.
- **Limits:** up to 100 columns or rows, and 1 000 copies.
- **Display:** the canvas draws the copies faded. The setup sheet and the playback say
  which copy is being cut.

Copies don't turn, and they aren't nested. To fit different parts on one sheet, use a nester
or arrange them in CAD.
