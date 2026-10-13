# Pocket

A pocket clears everything inside closed contours, down to the end depth, leaving islands
standing. Select the contours, then **+ Pocket**.

## What gets cleared

- **Each selected outline is its own pocket.** Two that cross are cleared separately and
  never merged: the second cuts air where they overlap.
- **Islands:** a selected contour wholly inside a pocket is left standing. A contour inside
  an island is a pocket again.
- **Not selected, not counted:** contours that aren't selected don't matter, even inside a
  pocket. A circle you'll drill later doesn't become an island.
- **Same shape twice:** two selected contours that are the same shape are refused, since
  one can't be both outline and island.

## Rings

The pocket is cleared in **rings**:
- **Order:** rings are offsets of the walls, cut from the centre outwards, with the wall
  ring last at each depth.
- **Stepover:** the distance between rings, as a fraction of the tool diameter (0.4 by
  default).
- **Round the inner rings** (on by default): every ring but the wall ring gets rounded
  corners, for smoother, faster motion. Where rounding would leave material, the corners
  are rounded less.
- **Keep the tool down where safe:** between rings, the tool feeds across at depth instead
  of retracting, when the move stays clear of the walls and is at most one tool diameter.

What a round tool leaves is normal: inside corners keep the tool's radius, and slots
narrower than the tool stay uncut. The [simulation](../checking/simulation.md) shows it.
Chipload also warns about two things:
- **Uncut material:** the rings don't pass close enough to some material (a stepover too
  large for the shape). Lower the stepover.
- **A narrow part:** the tool doesn't fit through a narrow part of the pocket. The pocket
  is cleared as separate areas, and material stays in between; a [rest
  pocket](rest.md) with a smaller tool can clear it.

## Entry

- **Plunge:** straight down at the innermost ring. It needs a centre-cutting endmill.
- **Ramp:** down along the first ring, out and back, never steeper than the ramp angle.
- **Helix:** down round a small circle at the pocket's widest point, as helical G2/G3.
  - The radius is a quarter of the tool diameter by default, and must be less than the
    tool radius. Where there's no room, it shrinks.
  - With no room for even a tenth of the tool's diameter, the pocket ramps in, with a note.
  - New pockets use a helix when the tool is marked not centre-cutting.

## Start point and lead

As on [profiles](profile.md#start-point): drag the start marker of each wall ring, and add a
lead from the cleared side onto the wall.

## Dogbones

**Corners** puts dogbones or T-bones into the pocket wall's inside corners, as on
[profiles](profile.md#dogbones-and-t-bones). Not with [rest machining](rest.md).

## Rest machining

A pocket can clear only what a larger tool left: see [Rest machining](rest.md).

## In the file

`"type": "pocket"` in the [project file](../reference/project-file.md#operation). Lengths are
in mm, feeds in mm/min, angles in radians.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick in the operation list |
| `select` | the geometry: `{"layer": "NAME"}` or `{"shapes": [ids]}` |
| `tool` | Tool: the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | End depth |
| `direction` | Direction |
| `clearance` | Clearance |
| `step_down` | Step down |
| `stepover` | Stepover, a fraction of the tool diameter |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
| `keep_tool_down` | keep the tool down where safe |
| `round_corners` | round the inner rings |
| `strategy` | `{"type": "rings"}` |
| `entry` | Entry: plunge, ramp, or `{"type": "helix", "radius": …, "ramp_angle": …}` |
| `starts` | start points |
| `lead` | Lead |
| `corners` | Corners (dogbones, T-bones) |
| `rest` | Rest after: `{"after_tool": id}` or null |

Projects from before Chipload 0.14 may use the *trochoidal* or *adaptive* strategy, which
were removed. They load, but generating asks you to switch the pocket to rings (**Use
rings** in the app).
