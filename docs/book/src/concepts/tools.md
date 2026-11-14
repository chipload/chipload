# Tools and tool changes

## The project's tools

**Tools** in the [Browser](../window.md#the-browser) lists the tools the project cuts with;
click it to see them all in Properties, or click one tool. Each has an id (T1, T2, …),
which is the tool number in the G-code, and:

| Setting | |
|---|---|
| **Type** | endmill, drill, V-bit or ball nose |
| **Diameter** | mm |
| **Spindle speed** | rpm. Required: a program can't start the spindle without it. |
| **Centre cutting** | whether the endmill can plunge straight down. Profiles spiral down with any endmill. New pockets go down round a helix and new follow paths ramp unless it is marked centre-cutting; when it can't plunge, sloped tabs may be no steeper than the ramp angle. |
| **Corner radius** | end mills: the radius of rounded corners, a *bull nose*. Not set: square corners. Less than half the diameter; a tool rounded that far is a ball nose |
| **Angle**, **Tip** | V-bits: the included angle and the flat tip's diameter. Tapered ball noses: the taper's included angle and the ball's diameter at the tip; a plain ball nose has neither |

Each operation names its tool: pick it in the operation's form, or on the command line
`chipload op tool part.chipload --op 2 --tool 3` (the tool must be in the project). Changing
the tool keeps the feeds, unless the operation has a [material](../library/materials.md)
with values for the new tool; otherwise check them. Under the tool, each operation's
**Material** list (`material` in the project file) chooses where its feeds come from.
Tools usually come from your [tool
library](../library/tools.md):
the project keeps its own copy, so it cuts the same later.

## Tool changes

Operations run in the list's order. A run of operations next to each other with the same
tool is a **tool group**, and between groups the tool changes. The Browser draws a
line at each change.

**Tool changes** (Machine) says how a job with several tools is written:
- **One file per tool:** `part-1-T1.nc`, `part-2-T3.nc`, … Change the tool by hand
  between files, set Z zero with the new tool, and keep X and Y zero. Every post can do
  this.
- **One file with tool changes** (`T2 M6`): for a sender or controller that handles the
  change. Only posts that support it offer it (GRBL, whose sender must do the change).

The command line can override it for one run: `chipload run part.chipload -o part.nc
--tool-change program`. The setup sheet lists the tools in the order they're used.

A **ball nose** carves [3D reliefs](../drawings/models.md) and can follow a path; the
other operations need a flat bottom and refuse it. `chipload tool add "Ball 3" --type
ballnose --diameter 3` adds a plain one, and `--angle 7.2 --tip 1` a tapered one.

A **bull nose** is an end mill with round corners: a flat bottom, then a quarter round of
its **corner radius** up to the side. Its corners are stronger than square ones, and it
cuts wherever an end mill does:

- **Reliefs** are cut with its true shape: it roughs with smaller steps on slopes than a
  square end mill, smooths them, and finishes wide reliefs fast, as its flat leaves no
  ridges on flat ground and its corners leave a ball nose's soft ones on slopes.
- **Profiles, pockets and the other operations** cut an end mill's path; the corners leave
  a fillet where the floor meets a wall, which the simulation shows. Two warnings watch
  for what the corners change: a cut through the board that goes less than the corner
  radius below it leaves a fillet along the bottom edge that can hold the part, and a
  pocket's or surface's stepover wider than the flat bottom leaves ridges on the floor.

`chipload tool add "Bull 6" --diameter 6 --corner-radius 1` adds one; in the window, tick
*Corner radius* under the end mill's diameter. Lists show it as *Ø6 mm R1 mm*.
