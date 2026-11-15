# Flute

**Flute** cuts a decorative groove along open shapes that dives in gradually at its start,
runs at full depth, and comes up gradually at its end. Flutes are cut down the face of
columns, table legs and pilasters, across panels, as the rays of a sunburst, or along a
frame. With a ball nose or a V-bit, each end narrows to a point.

Draw the flutes as lines, arcs or open polylines, select them, then press **S** over the
canvas and **L**, or choose **Operation > New Flute**, or *New operation* in a part's
right-click menu in the Browser.

## How it cuts

The tool's centre follows each shape on the line, as a [follow path](path.md) does. Along
the shape, the depth goes:

1. from the **start depth** at the shape's first end,
2. down over the **start ramp** to the **depth**,
3. at the depth through the middle,
4. up over the **end ramp** to the start depth at the shape's last end.

A ramp of 0 makes a square end: the flute is at full depth right at that end. Use it for a
flute that runs off the edge of the board.

- **First and last end** are the ends the shape was drawn from and to. Most flutes have the
  same ramp at both ends, so this rarely matters. A shape drawn the other way round swaps
  its ramps.
- **Ramps longer than the shape** (both together): both are shortened in the same
  proportion, so the flute reaches its depth at one point only. A note names the shape.
- **Closed shapes** are skipped with a note. A closed groove is a [follow path](path.md).

**Passes:** deeper than the step down, the flute is cut in equal passes. Each pass follows
the same ramps, cut off at its own depth, so the ramps keep their slope and the tool never
has more than one step of material under it. The tool goes back and forth along the
shape, with no retract between passes; at a square end it goes straight down one step at
the plunge feed.

## Settings

| Setting | |
|---|---|
| **Tool** | a ball nose (a round-bottomed flute), a V-bit (a V-groove, no deeper than its cone) or an end mill (a flat-bottomed groove). A drill is refused |
| **Start depth** | where the flute's ends are: 0 for the stock top, more for a flute in the floor of a pocket |
| **Depth** | through the middle. The width at the top is shown beneath: "6 mm wide with the 6 mm ball nose" |
| **Step down** | off: one pass. On: equal passes no deeper than this, the last at the depth |
| **Start ramp**, **End ramp** | how far along the shape, from each end, the flute reaches its depth, mm; 20 mm for a new flute. 0: a square end |
| **Ramp** | *straight*: an even taper. *Curved*: steepest at the surface, easing into the floor, so no line shows where the ramp meets the flat |
| **Feed**, **Plunge feed** | mm/min. The feed is used along the whole shape, ramps included; the plunge feed only going straight down |

A new flute takes the project's first ball nose (else its first V-bit), as deep as the ball's
radius for a half-round flute (3 mm with another tool, no deeper than a V-bit's cone).

The report says how wide the flute is at the top: "the flute is 6 mm wide at the top". The
setup sheet says, for example, "Flute, ramps 20 / 20 mm, straight ramps", with its depths.

## Command line

```
chipload op add part.chipload flute --shapes 4,5,6 --depth 4 --ramp 20
chipload op add part.chipload flute --layer FLUTES --ramp-start 0 --ramp-end 30 --ramp-shape curved
chipload set part.chipload operations.2.ramp_end=0
```

`--ramp` sets both ends; `--ramp-start` and `--ramp-end` one each; `--ramp-shape` is
`straight` or `curved`. A flute takes no `--through`.

## In the file

`"type": "flute"` in the [project file](../reference/project-file.md#operation). Lengths
are in mm, feeds in mm/min. A project with a flute opens in Chipload 0.37.0 and later.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick on the operation's row in the Browser |
| `part` | the [part](parts.md) it belongs to |
| `select` | the geometry: `{"layer": "NAME"}` or `{"shapes": [ids]}` |
| `tool` | Tool: the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | Depth |
| `step_down` | Step down; `null` for one pass |
| `ramp_start` | Start ramp |
| `ramp_end` | End ramp |
| `ramp_shape` | Ramp: `straight` or `curved` |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
