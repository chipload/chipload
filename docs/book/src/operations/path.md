# Follow path

**Follow path** puts the tool's centre exactly on the selected lines, open or closed, down
to the end depth in passes. Use it for engraving, V-grooves, score and fold lines, or a
single cut through thin material.

Select the shapes, then **Follow** (ribbon, OPERATION). A picture of handwriting or line art can be
traced into [centre lines](../drawings/images.md#centre-lines) for it.
- **Open lines** are cut back and forth, pass by pass.
- **Closed shapes** go round, as a profile on the line does.
- **Order:** the paths are cut nearest first.

It cuts only what it selects. Profiles and pockets keep skipping open lines, so a centre
line on a cutting layer is never cut by them. A follow path that selects a whole layer does
cut that layer's centre lines.

| Setting | |
|---|---|
| **Start depth**, **End depth**, **Step down** | as on every operation |
| **Entry** | *plunge*, or *ramp* along the path. A pointed V-bit counts as centre-cutting. |
| **Feed**, **Plunge feed** | mm/min |

There is no side, clearance, tabs, lead or start point: the tool is on the line.

## In the file

`"type": "path"` in the [project file](../reference/project-file.md#operation). Lengths are
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
| `step_down` | Step down |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
| `entry` | Entry: plunge or ramp |
