# Nesting

**Nesting** places every [instance](instances.md) of a setup's parts on its board, as close
together as they fit, so a sheet holds as many parts as it can. Every part that is on takes
part, instance 1 included.

Chipload nests what the tools really cut: each instance's toolpath with the width of its
tool, tabs and ramps included, not only the drawing. Between two cuts it leaves the **gap**,
a web of board that keeps the sheet stiff and the tabs holding; at the board's edge it
leaves the **margin**, room for screws or clamps. Only the instances' places change: the
drawing stays where it is, and every operation follows its instances. The result is
planned and checked as any project is before it is kept.

In the window:
- **Right-click the setup** in the Browser and choose *Nest…*, or *Wizards > Nest…* for the
  active setup. The window shows the board's size, the gap and the margin, and a row per
  part with how many times it is cut and how it may turn.
- **The preview** is worked out as soon as the window opens, and again with *Preview*
  after a change: the instances show on the board where they would go, outlined in the
  selection colour, the board dashed; what didn't fit is parked to the right of the board
  in amber and named in the window. Nothing changes until you choose *Apply*.
- **Apply** places them as one undo step (Ctrl+Z puts everything back) and lists what was
  switched off in the Report. *Apply* before a preview nests and applies in one go.
- **Nest again**, in the setup's right-click menu once it has been nested, nests it again
  with the same gap and margin and each part's turns, the instances there are now, without
  the window: after you change a part, or moved instances by hand.

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
  the highest-numbered ones that are on removed; at least one stays.
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
- **The setup sheet** says the setup was nested: how many instances, the gap and margin,
  and how far along the board the cuts reach, so you know what is left of the sheet.
- **Not yet:** a setup cut in [pieces](../concepts/pieces.md) or a two-sided setup can't be
  nested, and small parts don't go inside another part's holes.

Everything `chipload nest` says is also in `--json`: what went where (`placed`, each part,
instance, angle in radians and move in mm), what didn't fit (`unplaced`), how far along the
board it reaches (`used`, `length`, `used_pct`), and whether the board was given a size.
