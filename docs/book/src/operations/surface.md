# Surface

**Surface** takes a top down flat with a wide endmill. It comes in two kinds:

- **The whole board** (a board surface): flatten your spoilboard, or face a board before
  cutting it. It needs no selection. Right-click the setup, *Face the board…*; on the
  command line, `chipload op add-surface part.chipload` (`--setup ID` for another setup). It goes **first** in its setup,
  since it faces the stock before anything else is cut, and is cut **once**, whatever parts
  and [instances](../drawings/instances.md) the setup has.
- **A part's area** (a part surface): select the part's outline (closed shapes) and add a
  Surface, as you would a pocket; on the command line, `chipload op add part.chipload
  surface --shapes 1 --margin 3`. It faces each selected shape grown by a **margin**, and
  is cut under every instance of the part, moving and turning with it. Other parts' areas
  are left alone.

A new surface takes the project's widest endmill and 0.5 mm off the top in one pass; a part
surface's margin starts at the tool's radius.

## How the whole board is cut

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
- **Once:** the stock is faced once, even when the part has [instances](../drawings/instances.md).

The setup sheet says how far the tool runs past the stock's ends (its radius): keep clamps
and fences clear of that.

## How a part's area is cut

- **The area:** each selected closed shape grown by the margin; areas that overlap are
  joined, and a shape inside another is faced over, not left standing.
- **Rings, as a pocket:** the area is cleared in rings one stepover apart, the tool going
  down each depth by a helix, never plunging. Its edge reaches the grown outline.
- **Under each instance:** cut for the part and each of its instances, where they lie.

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
| `select` | the shapes a part surface faces; none for the whole board |
| `margin` | Margin: how far a part surface's area reaches past its shapes, mm |
| `tool` | Tool: an endmill, the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | End depth: how much comes off |
| `step_down` | Step down |
| `stepover` | Stepover, a fraction of the tool diameter (0.1 to 0.9) |
| `feed` | Feed |
| `plunge_feed` | Plunge feed: down beside the stock |
