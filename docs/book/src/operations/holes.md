# Holes

**Holes** cuts every selected circle, either drilled with a drill or bored with a helix
with an endmill. Select the circles (a whole layer is fine: other shapes are skipped with a
note), then **+ Holes**. Circles drawn in Chipload count, and so do imported circles,
including ones drawn as two half arcs.

The tool decides how:

| Tool | Method | Which circles |
|---|---|---|
| Drill | drilled at the centre | the drill's diameter, within ±0.1 mm |
| Endmill | helical bore at the wall | larger than the tool |
| V-bit | refused | |

## Drilling

- The drill feeds straight down at the hole's centre.
- **Pecks** (optional): it drills that many mm, rapids up to clear the chips, rapids back
  down to 0.5 mm above the bottom, and drills on.
- **End depth** is the depth of the drill's **point**. The full diameter ends a little
  higher, about 0.3 × the diameter for a 118° point.
- **Plain moves:** the holes are written as plain G0/G1 moves, not canned cycles, so they
  run the same on every controller and the checker reads them. The checker allows exactly
  one kind of rapid below the stock top: straight back into the hole just drilled.

## Helical boring

- **The path:** the endmill goes down a helix at the hole's wall, at most *step per turn*
  per turn and never steeper than the ramp angle. Then it cuts one level circle at the
  bottom, for a flat floor and a clean wall.
- **Size:** the hole comes out at its drawn size.
- **Any endmill:** the helix never plunges, so a tool that isn't centre-cutting is fine.
- **Too small:** a hole must be larger than the tool, by at least a tenth of the tool's
  diameter. Smaller holes are skipped, with a note.
- **Larger holes:** more than twice the tool's diameter, the helix cuts a ring and leaves a
  core.
  - In a **through hole**, the core comes loose at the last turn, with a note. Hold it down,
    or use a pocket.
  - A **blind hole** that would leave a core standing is refused: use a pocket.

## Order

Holes are cut nearest first, from where the tool is. Where the Holes operation sits in the
list decides whether holes come before or after the profiles.

## In the file

`"type": "holes"` in the [project file](../reference/project-file.md#operation). Lengths are
in mm, feeds in mm/min, angles in radians.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick in the operation list |
| `select` | the geometry: `{"layer": "NAME"}` or `{"shapes": [ids]}` |
| `tool` | Tool: the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | End depth; for a drill, the depth of its point |
| `feed` | Feed: the bore's helix and circle |
| `plunge_feed` | Drill feed, or the bore's descent to the start depth |
| `method` | `{"type": "drill", "peck": … or null}` or `{"type": "bore", "step_down": …, "ramp_angle": …, "direction": …}` |
