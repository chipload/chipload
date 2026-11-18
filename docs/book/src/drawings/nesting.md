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
  active setup. The window shows the board's size, the gap and the margin, the goal, and a
  row per part with how many times it is cut and how it may turn.
- **The goal** says what the nest makes as small as it can, once it has placed as much as
  it can:
  - **Compact** (the default): the smallest box round the parts, a block in the lower-left
    corner. Good when the rest of the board is waste.
  - **Keep the top free**: the parts as low as they go, across the whole width, so a strip
    across the top is left to saw off and keep.
  - **Keep the right side free**: the parts as far left as they go, up the whole height,
    so a strip down the right is left.
- **The preview** is worked out as soon as the window opens, and again with *Preview*
  after a change: the instances show on the board where they would go, outlined in the
  selection colour, the board dashed; what didn't fit is parked to the right of the board
  in amber and named in the window. Nothing changes until you choose *Apply*.
- **Apply** places them as one undo step (Ctrl+Z puts everything back) and lists what was
  switched off in the Report. *Apply* before a preview nests and applies in one go.
- **Nest again**, in the setup's right-click menu once it has been nested, nests it again
  with the same gap and margin and each part's turns, the instances there are now, without
  the window: after you change a part, or moved instances by hand. It searches on from the
  board as it is and never makes it worse: when it finds nothing better, nothing moves and
  the status says "No better nest found; the board is as it was." Press it again after a
  better nest and it searches on from there.

On the command line:

```sh
chipload nest part.chipload                         # the active setup, on its board
chipload nest part.chipload --count 2=4             # part 2 cut four times: instances added first
chipload nest part.chipload --turns 2=none          # part 2 never turned (wood grain); others quarter turns
chipload nest part.chipload --gap 5 --margin 15     # more room between parts and at the edge
chipload nest part.chipload --goal low              # keep the top of the board free (also compact, left)
chipload nest part.chipload --dry-run               # say what would go where, write nothing
chipload part set part.chipload 2 --turns half      # part 2 turned 180° at most, kept for the next nest
```

- **How it fills:** from the board's lower-left corner, by the goal: for *compact* each part
  where the box round everything placed grows least, for the others as low (or as far left)
  as it fits. It tries many orders of the parts and turns of each, for a moment (the window
  shows how far it has got), and keeps the layout that places the most and best meets the
  goal. The same project always gives the same layout. Nesting a setup that was nested before searches on from the board as it is
  and keeps it unless it finds better.
- **Turns:** `none`, `half` (0° or 180°), `quarter` (the default) or `every-15`, for every
  part (`--turns quarter`) or one (`--turns 2=none`). They are kept in the project.
- **Counts:** `--count ID=N` cuts part ID N times: instances are added (not mirrored), or
  the highest-numbered ones that are on removed; at least one stays.
- **Mirrored instances** stay mirrored; nesting turns and moves them, never mirrors them.
- **What doesn't fit** is switched off, kept where it was, and named ("Part 2, instance 4
  didn't fit: switched off"). Make the board larger, allow turns, or let it go onto more
  boards.
- **Onto more boards** (*What doesn't fit: Onto more boards* in the window, `--overflow
  new-boards` on the command line), as a slicer adds plates: what doesn't fit goes to the
  next [setup](../concepts/setups.md) with the same stock and a size of its own, else to a
  new one made like this board (its size, work zero, gap and margin), nested there in
  turn, and so on until everything is placed. The preview shows each board in a row to the
  right; the Report says "2 more on Setup 2, a new board like the first". Instances keep
  their numbers. A part larger than an empty board is still switched off. A part whose
  [stock](../operations/parts.md#a-parts-stock) isn't this board's goes to a board of its
  own stock instead.
- **The board** must have a size of its own. A board that follows the drawing (fitted to it
  or centred on it) is given one with `--board 1220x610`: it becomes that size, its corner
  at the drawing's origin.
- **The machine's travel**, when it is set in the [machine](../library/machines.md), limits
  how much of a larger board is used, measured from the side where work zero is.
  A board that follows the drawing starts in the window at the travel; with a board
  surface, less the surface tool's diameter along the board's longer side, since facing
  runs from beside the board to beside it. A board whose surface would run past the travel
  is refused before nesting, naming the surface.
- **The gap, margin and goal** (3 mm, 10 mm and compact until you choose) are kept with the
  setup, so the next nest uses them.
- **The result** says how many were placed and the box they take: "8 of 8 placed, in
  742 × 380 mm of the board's 1200 × 590 mm (40 %)", the board less its margin.
- **The setup sheet** says the setup was nested: how many instances, the gap and margin,
  and the box the cuts take from the board's lower-left corner, so you know what is left of
  the sheet.
- **Not yet:** a setup cut in [pieces](../concepts/pieces.md) or a two-sided setup can't be
  nested, and small parts don't go inside another part's holes.

Everything `chipload nest` says is also in `--json`: what went where (`placed`, each part,
instance, angle in radians and move in mm), what didn't fit (`unplaced`), the goal
(`goal`), the box round what was placed and the usable board (`extent`, `usable`, X and Y
in mm, and `used_pct`, the box's share), whether the board was given a size, and whether a
nest again found nothing better (`unchanged`).
