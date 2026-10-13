# V-carve

A V-carve engraves closed shapes, such as lettering (converted to outlines in CAD), logos
and inlay outlines, with a V-bit:
- the bit follows each shape's centre line;
- it goes as deep as makes the cut exactly as wide as the shape at the surface.

Narrow strokes come out shallow and wide ones deep, and sharp outside corners such as serif
tips stay sharp. Select the closed contours, then **V-carve** (ribbon, OPERATION). A logo or sketch that is
only a picture can be [traced into shapes](../drawings/images.md) first.

## The V-bit

The tool must be a **V-bit** with its **angle** set: the included angle, e.g. 60° or 90°.
**Tip** is the flat tip's diameter, 0 for a pointed bit. Chipload never guesses the angle;
a V-bit without one is refused.

## Settings

| Setting | |
|---|---|
| **Start depth** | where the carved surface is: 0 for the stock top, more to carve into a floor an earlier pocket cut |
| **Max depth** | the deepest the tip may go. Strokes that need more are carved to it and come out narrower than drawn, with a warning. The bit's own size is a limit too: it can't go deeper than its cutting edge reaches. |
| **Step down** | optional: carve in passes, each only where the last stopped short |
| **Feed**, **Plunge feed** | moves steeper than 45° use the plunge feed |

Regions work as in pockets: an outline minus the selected contours inside it (the inside of
an O stays standing). Open contours are skipped.

## Limits

- **A flat tip:** it can't reach into the last bit of a sharp corner, and strokes narrower
  than the tip are left out, with a warning.
- **No floor clearing:** the floor is the V the bit leaves; wide areas aren't cleared flat.
  For that, pocket them first with an endmill and carve on the pocket floor with the start
  depth.
- **Text:** text from the DXF must be converted to outlines (curves) in CAD first. Chipload
  doesn't read TEXT entities or fonts.

Every V-carve move is checked so the cone never reaches past the outline at the surface.

## In the file

`"type": "vcarve"` in the [project file](../reference/project-file.md#operation). Lengths are
in mm, feeds in mm/min, angles in radians.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick on the operation's row in the Browser |
| `select` | the geometry: `{"layer": "NAME"}` or `{"shapes": [ids]}` |
| `tool` | Tool: the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | Max depth |
| `step_down` | Step down, or null for one pass |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
