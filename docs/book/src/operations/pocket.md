# Pocket

A pocket clears everything inside closed contours, down to the end depth, leaving islands
standing. Select the contours, then **Pocket** (ribbon, OPERATION).

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
  ring last at each depth. Round islands, each island's rings are cut one after another,
  out to the island, before the next island's, so the tool doesn't cross the pocket
  between rings. Where an island is close to the wall (less than a tool diameter), its
  rings and the wall's take turns there, so no ring meets more than a stepover of material.
- **Stepover:** the distance between rings, as a fraction of the tool diameter (0.4 by
  default).
- **Rounded inner rings:** every ring but the wall ring has rounded corners (up to two
  stepovers in radius), for smoother, faster motion. Where rounding would leave material,
  the corners are rounded less.
- **Spiral:** each ring steps out onto the next over its last stretch instead of closing,
  so the rings of a lobe are one continuous spiral with no stop and no sideways move into
  material. Round an island, its rings spiral out towards it the same way. The
  stepped-out stretch is checked against the walls and islands like a ring; where it
  wouldn't fit (against a sharp corner, or onto a ring with a lead or a chosen start) the
  ring closes and is linked.
- **Links:** where a ring closes, the tool feeds across to the next at depth when the move
  stays clear of the walls and is at most one tool diameter, which it checks; otherwise it
  retracts. There is nothing to set: a checked link is always better than a retract.

What a round tool leaves is normal: inside corners keep the tool's radius, and slots
narrower than the tool stay uncut. The [simulation](../checking/simulation.md) shows it.
Chipload also warns about two things:
- **Uncut material:** the rings don't pass close enough to some material (a stepover too
  large for the shape). Lower the stepover.
- **A narrow part:** the tool doesn't fit through a narrow part of the pocket. The pocket
  is cleared as separate areas, and material stays in between; a [rest
  pass](rest.md) with a smaller tool can clear it.

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

A pocket can have rest passes: smaller tools that clear only what its tool left, shown under
it in the Browser. See [Rest machining](rest.md).

## In the file

`"type": "pocket"` in the [project file](../reference/project-file.md#operation). Lengths are
in mm, feeds in mm/min, angles in radians.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick on the operation's row in the Browser |
| `part` | the [part](parts.md) it belongs to |
| `select` | the geometry: `{"layer": "NAME"}` or `{"shapes": [ids]}` |
| `tool` | Tool: the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | End depth |
| `direction` | Direction |
| `clearance` | Oversize |
| `step_down` | Step down |
| `stepover` | Stepover, a fraction of the tool diameter |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
| `entry` | Entry: plunge, ramp, or `{"type": "helix", "radius": …, "ramp_angle": …}` |
| `starts` | start points |
| `lead` | Lead |
| `corners` | Corners (dogbones, T-bones) |
| `rest` | Rest after: `{"after_tool": id}` or null |
| `rest_passes` | Its rest passes, each `{tool, feed, plunge_feed, step_down, stepover, entry, enabled}`; left out when there are none |

Projects from before Chipload 0.14 that used the removed *trochoidal* or *adaptive*
clearing are cut as ring pockets. Projects from before 0.18 that had the tool retracting
between rings or sharp inner rings now get links and rounded rings: the same rings, cut
faster.
