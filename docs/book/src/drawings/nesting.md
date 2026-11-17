# Nesting

**Nesting** places every [instance](instances.md) of a setup's parts on its board, as close
together as they fit, so a sheet holds as many parts as it can. Every part that is on takes
part, the part itself (instance 1) included.

Chipload nests what the tools really cut: each instance's toolpath with the width of its
tool, tabs and ramps included, not only the drawing. Between two cuts it leaves the **gap**,
a web of board that keeps the sheet stiff and the tabs holding; at the board's edge it
leaves the **margin**, room for screws or clamps. Only the instances' places change: the
drawing stays where it is, and every operation follows its instances. The result is
planned and checked as any project is before it is kept.

On the command line:

```sh
chipload nest part.chipload                         # the active setup, on its board
chipload nest part.chipload --count 2=4             # part 2 cut four times: instances added first
chipload nest part.chipload --turns 2=none          # part 2 never turned (wood grain); others quarter turns
chipload nest part.chipload --gap 5 --margin 15     # more room between parts and at the edge
chipload nest part.chipload --dry-run               # say what would go where, write nothing
chipload part set part.chipload 2 --turns half      # part 2 turned 180° at most, kept for the next nest
```

- **How it fills:** the largest parts first, each as far toward the start of the board's
  longer side as it fits, then as low across it as it fits, so what is left of the board is
  one strip across its far end. Each part is tried at every turn it allows.
- **Turns:** `none`, `half` (0° or 180°), `quarter` (the default) or `every-15`, for every
  part (`--turns quarter`) or one (`--turns 2=none`). They are kept in the project.
- **Counts:** `--count ID=N` cuts part ID N times: instances are added (not mirrored), or
  the highest-numbered ones removed. The part itself always stays.
- **Mirrored instances** stay mirrored; nesting turns and moves them, never mirrors them.
- **What doesn't fit** is switched off, kept where it was, and named ("Part 2, instance 4
  didn't fit: switched off"). Make the board larger, allow turns, or move the part to a
  setup of its own, switch it on and nest again.
- **The board** must have a size of its own. A board that follows the drawing (fitted to it
  or centred on it) is given one with `--board 1220x610`: it becomes that size, its corner
  at the drawing's origin.
- **The machine's travel**, when it is set in the [machine](../library/machines.md), limits
  how much of a larger board is used, measured from the side where work zero is.
- **The gap and margin** (3 mm and 10 mm until you choose) are kept with the setup, so the
  next nest uses them.
- **Not yet:** a setup cut in [pieces](../concepts/pieces.md) or a two-sided setup can't be
  nested, and small parts don't go inside another part's holes.

Everything `chipload nest` says is also in `--json`: what went where (`placed`, each part,
instance, angle in radians and move in mm), what didn't fit (`unplaced`), how far along the
board it reaches (`used`, `length`, `used_pct`), and whether the board was given a size.
