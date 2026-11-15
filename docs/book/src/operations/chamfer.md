# Chamfer

**Chamfer** breaks the top edge of closed shapes with a V-bit: the edge of a box side, a
tray, a shelf or a sign, or the rim of a hole or a pocket. You give the width you want at
the surface; the depth follows from the bit's angle.

Select the shapes, then **Chamfer** (ribbon, OPERATION, the small button under Styles), or
press **Q** over the canvas, then **C**.

## How it cuts

The bit's flank runs on the line and its tip stays in the waste:

- the tool's centre runs half the bit's flat tip outside the line, on the waste side (on the
  line itself with a pointed bit);
- it goes down as deep as the width over the tangent of half the bit's angle: with a 90°
  bit, as deep as the width (a 45° chamfer); with a 60° bit, 1.73 × the width (a steeper
  one).

So the chamfer comes out as wide as you typed, and a flat-tipped bit leaves no step on the
part. The other flank grooves the waste, which the cut-out or the pocket takes away.

- **The part's outside corners** are turned round as a cone, as a chamfer bit would.
- **Its inside corners** meet in a mitre.
- **Open shapes** are skipped, with a note: use a [follow path](path.md) for them.

**Order:** a new chamfer goes at the end of its part, like any new operation, after the
cut-out. That works: the tabs or the onion skin hold the part, the cut-out's slot is on the
waste side, and a chamfer stays far above the tabs. Drag it before the cut-out if you'd
rather have the part held by the board while it is chamfered; with the V-bit between two
end mill operations, that costs a tool change.

## Settings

| Setting | |
|---|---|
| **Tool** | a V-bit with its angle; any other tool is refused |
| **Side** | where the waste is: *outside* the shape (a part's outline) or *inside* it (a hole's or a pocket's rim). A new chamfer takes it from the shapes, as a new profile does: inside for holes, outside for outlines, two chamfers when both are selected |
| **Width** | at the surface, from the line into the part; 1 mm for a new one. At most half the bit's cone (its diameter less its tip, halved), or the shank would rub: 6.25 mm with a 12.7 mm bit with a 0.2 mm tip |
| **Depth** | shown beneath: "2 mm with the 90° bit" |
| **Start depth** | where the edge is: 0 for the stock top, more for the edge of a step an earlier operation cut |
| **Step down** | off: one pass. On: equal passes no deeper than this, the last at the full depth |
| **Direction** | climb or conventional |
| **Feed**, **Plunge feed** | mm/min; from the V-bit's cutting data |
| **Entry** | *plunge* (a V-bit goes down on its point), or *ramp* along the path |

The setup sheet says, for example, "Chamfer 2 mm wide, outside, climb", with its depths.

[New sign…](../drawings/signs.md) adds a chamfer for its edge.

## Command line

```
chipload op add part.chipload chamfer --shapes 1,2 --width 2
chipload op add part.chipload chamfer --layer OUTLINE --side outside --tool 3
chipload set part.chipload operations.4.width=3
```

`--width` is the width at the surface (1 mm if left out); `--side` sets the waste side in
place of the one the shapes tell. A chamfer takes no `--depth` or `--through`.

## In the file

`"type": "chamfer"` in the [project file](../reference/project-file.md#operation). Lengths
are in mm, feeds in mm/min. A project with a chamfer opens in Chipload 0.37.0 and later.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick on the operation's row in the Browser |
| `part` | the [part](parts.md) it belongs to |
| `select` | the geometry: `{"layer": "NAME"}` or `{"shapes": [ids]}` |
| `tool` | Tool: the project tool's id, a V-bit |
| `side` | Side: `outside` or `inside` |
| `width` | Width |
| `start_depth` | Start depth |
| `step_down` | Step down; `null` for one pass |
| `direction` | Direction: `climb` or `conventional` |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
| `entry` | Entry: plunge or ramp |
