# Profile

A profile cuts along a closed contour:
- **outside** it, to cut a part out;
- **inside** it, to cut a hole;
- **on** the line.

Select the contours, then **Profile** (ribbon, OPERATION). See [Side and direction](../concepts/side-direction.md)
and [Depths and oversize](../concepts/depths.md) for what every profile shares.

Open contours (centre lines, construction lines) in the selection are skipped with a note.
To cut along an open line, use [Follow path](path.md).

## Entry: spiral ramp or straight plunge

In the operation's **Cut** settings, right after the step down, **Entry** is the same
choice every operation has:

- **Spiral ramp**: the tool goes down while moving along the contour, never
  steeper than the **ramp angle** (3° by default, at most 30°), and never more than one
  step down per lap. On long contours the step down decides how fast it goes down; on
  small ones the angle does. A spiral-ramped profile is **one forward spiral**:
  - each pass's laps descend evenly along the contour;
  - one level lap at the bottom flattens the floor;
  - the tool rises out along the cut path, never back and forth.

  It's the best way down: no plunge into the part. Tools that aren't centre-cutting need
  it, and new profiles start with it when the tool is marked so.
- **Straight plunge**: the tool goes straight down at the start of each pass. It
  needs a centre-cutting endmill. Only then does the **lead-in and lead-out** show: the
  plunge goes down in the waste, so it leaves no mark on the part.

In the file, `entry` is `{"type": "ramp", "ramp_angle": …}` for the spiral ramp and
`{"type": "plunge"}` for a straight plunge.

## Holding the part

**Holding** chooses how a part that is cut out stays in place: *none*, *tabs*, *onion
skin*, or *onion skin and tabs*. Only the chosen kind's settings show.

- **Tabs** keep the part attached at a few points until you cut it free.
- **An onion skin** holds it all round during the roughing; the last lap then cuts
  through the skin, and the part is free during that lap.
- **Both:** the skin holds it all round during the roughing, and the tabs keep it from
  moving while the last lap cuts through the skin.

## Tabs

Tabs are bridges of material left at the bottom of the cut, so a part cut out doesn't come
loose on the last pass.

| Setting | |
|---|---|
| **count** | tabs per contour, spread evenly along it |
| **width** | the bridge left on the part, along its edge (the flat top for sloped tabs) |
| **height** | the bridge's thickness, up from the bottom of the cut |
| **flank angle** | 90° for straight tabs (the tool goes straight up and down); less for sloped ones. A ramped profile gets 30° by default and follows the slope, which is gentler on the tool. |

- **Placement:** tabs slide off corners to straight stretches, by up to a quarter of their
  spacing.
- **Spacing:** they keep a tool diameter from each other and from the start of the contour.
- **Shown** highlighted on the canvas.

## Start point

Each contour's cut starts at a point Chipload picks. To move it, select the operation, then
**drag the start marker** along the contour on the canvas; right-click the marker to reset
it. The tabs move with the start. In the file, `starts` holds the points.

## Lead-in and lead-out

**Lead** adds a quarter-circle arc from the waste onto the contour and off it again, so the
plunge mark isn't on the part's wall. It is offered with straight plunges.
- **Radius:** at most 10 × the tool diameter. Chipload shrinks it where there's no room,
  with a note.
- **Spiral-ramped profiles:** a project that has both keeps them: one lead-in at the start
  depth, and no lead-out (the tool rises out along the cut). The lead's row shows until it
  is unticked.
- **Plunged profiles:** a lead on every pass.
- **On the line:** not available, since there is no waste side.

## Finishing pass

**Finishing pass** with a *stock* value:
- the roughing laps leave that much on the wall;
- one light lap at the wall at full depth then takes it off.

The wall comes out clean: the tool isn't loaded on that lap, so it doesn't deflect, and
there are no step marks from the passes. The stock must be less than the tool radius.

## Onion skin

**Onion skin** with a *thickness*:
- the passes stop that far above the end depth;
- one last lap cuts through the skin.

Small parts stay held until the very end, on a vacuum table or instead of tabs. The skin is
at most one step down, and thinner than the tab height when there are tabs (*onion skin and
tabs* under Holding).

## Dogbones and T-bones

A round tool leaves a fillet of its radius in every inside corner, so a square tenon won't
go into the mortise. **Corners** reaches into them:
- **dogbones:** the tool goes into the corner along its bisector;
- **T-bones:** the tool goes in along the longer wall, so the notch is in the shorter wall,
  out of sight.

*up to* sets the sharpest corner angle that gets one (100° by default, so 90° corners
drawn slightly off count). A corner the tool can't reach without touching another contour
is left, with a note.

## In the file

`"type": "profile"` in the [project file](../reference/project-file.md#operation). Lengths are
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
| `side` | Side |
| `direction` | Direction |
| `clearance` | Oversize |
| `step_down` | Step down |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
| `entry` | Entry: `{"type": "plunge"}` or `{"type": "ramp", "ramp_angle": …}` |
| `tabs` | Tabs: `count`, `width`, `height`, `flank_angle`, or null |
| `starts` | start points, `[[x, y], …]` in drawing coordinates |
| `lead` | Lead: `{"radius": …}` or null |
| `finish` | Finishing pass: `{"stock": …}` or null |
| `skin` | Onion skin: `{"thickness": …}` or null |
| `corners` | Corners: `{"type": "dogbone"` or `"tbone", "max_angle": …}`, or null |
