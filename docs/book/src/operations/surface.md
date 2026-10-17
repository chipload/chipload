# Surface

**Surface** takes the whole stock top down flat with a wide endmill. Use it to flatten
your spoilboard, or to face a board before cutting it. It needs no selection: click
**Surface** (ribbon, OPERATION), or right-click a part, *New operation ▸ Surface*. On the
command line, `chipload op add-surface part.chipload`.

A new surface goes **first** in the active part, since it faces the stock before anything
else is cut. It takes the project's widest endmill and 0.5 mm off the top in one pass.

## How it cuts

- **Back and forth** along the stock's longer side, in straight passes one stepover apart.
  Half the passes climb, half are conventional: usual for a shallow facing pass, and about
  half the time of cutting one way only.
- **Past the ends:** each pass starts and ends beside the stock, the tool's edge just
  clear of it, so the tool **goes down in the air**, never into the stock. No ramp or helix
  is needed. Between passes it moves across beside the stock, without lifting.
- **Past the sides:** the first and last passes reach past the stock's long sides by a
  quarter of the tool, so the edges come out clean even when the stock is a little larger
  than entered.
- **Depths:** with a step down smaller than the end depth, each depth comes back the other
  way, going down beside the stock where the one before ended.
- **Once:** the stock is faced once, even when the part has [copies](../drawings/copies.md).

The setup sheet says how far the tool runs past the stock's ends (its radius): keep clamps
and fences clear of that.

## Settings

- **End depth:** how much comes off, from the stock top. At most the stock thickness.
- **Step down:** the deepest each pass goes. Usually the whole depth in one pass.
- **Stepover:** how far each pass moves across, as a fraction of the tool diameter, from
  0.1 to 0.9. A new surface takes 0.75.
- **Feed** and **plunge feed:** the plunge feed is for the moves down beside the stock.

The other operations' depths don't change: they still count from the stock top as
entered. Facing 0.5 mm off means their first 0.5 mm cuts air, which is safe. A surface
placed after other operations of its part gets a warning: it would face what they already
cut.

## Flattening the spoilboard

1. Make a project whose **stock** is the spoilboard's area, Z zero at its top
   ([Stock and work zero](../concepts/stock.md)).
2. Copy your surfacing bit into the project ([Tool library](../library/tools.md)).
3. Add a Surface, with 0.3 to 0.5 mm off: enough to take out the dips.
4. Export, zero Z on the spoilboard, and run it.

## In the file

`"type": "surface"` in the [project file](../reference/project-file.md#operation). Lengths
are in mm, feeds in mm/min.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick on the operation's row in the Browser |
| `part` | the [part](parts.md) it belongs to |
| `tool` | Tool: an endmill, the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | End depth: how much comes off |
| `step_down` | Step down |
| `stepover` | Stepover, a fraction of the tool diameter (0.1 to 0.9) |
| `feed` | Feed |
| `plunge_feed` | Plunge feed: down beside the stock |
