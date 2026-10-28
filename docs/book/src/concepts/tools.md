# Tools and tool changes

## The project's tools

**Tools** in the [Browser](../window.md#the-browser) lists the tools the project cuts with;
click it to see them all in Properties, or click one tool. Each has an id (T1, T2, …),
which is the tool number in the G-code, and:

| Setting | |
|---|---|
| **Type** | endmill, drill or V-bit |
| **Diameter** | mm |
| **Spindle speed** | rpm. Required: a program can't start the spindle without it. |
| **Centre cutting** | whether the endmill can plunge straight down. Profiles spiral down with any endmill; when it can't plunge, pockets go down round a helix and sloped tabs may be no steeper than the ramp angle. |
| **Angle**, **Tip** | V-bits: the included angle and the flat tip's diameter |

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

**Tool changes** (Job settings > Machine) says how a job with several tools is written:
- **One file per tool:** `part-1-T1.nc`, `part-2-T3.nc`, … Change the tool by hand
  between files, set Z zero with the new tool, and keep X and Y zero. Every post can do
  this.
- **One file with tool changes** (`T2 M6`): for a sender or controller that handles the
  change. Only posts that support it offer it (GRBL, whose sender must do the change).

The command line can override it for one run: `chipload run part.chipload -o part.nc
--tool-change program`. The setup sheet lists the tools in the order they're used.
