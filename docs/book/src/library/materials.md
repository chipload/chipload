# Materials

A material is a name, "Plywood", "Oak", "MDF" or "Plywood careful", that you choose for an
operation or for a whole setup. Its numbers are **your own**: each tool keeps them in its
[cutting data](tools.md#cutting-data), one row per material. Choose a material, and the
operations cut with their tools' rows for it.

Chipload has no built-in materials and doesn't calculate feeds. Catalogue values are made
for rigid industrial machines; a hobby router wants far less. As an example, a Shapeoko
cutting wood with a 6 mm 2-flute bit runs about 1500 mm/min with a 2–2.5 mm step down. Start
from what you know works on yours.

A material is about feeds only: choosing one never changes the stock or any depth.

## Making one

*This computer > Materials > New material*, then name it. Its page lists the tools with
values for it; click one to open its cutting data. To give it values:
- **From a job that cut well:** choose the material in an operation's **Material** list,
  under its tool. Cut, and adjust the operation's feeds or step down until they're right.
  Its **Material** row then says *Changed from Plywood*: click **Save to Plywood**. The values
  the operation uses go into its tool's Plywood row (a profile its slot values, a pocket
  both); the row's others stay. A tool without a Plywood row gets one.
- **By hand:** open the tool (*This computer > Tools*), and under **Cutting data**, *Add a
  material…*.

The tool must be in your tool library: the values belong to that library tool, so every
project that uses it finds them. A tool only in the project is saved to the library first
(*Save to library…* on the tool).

## Choosing a material

**For one operation:** its **Material** list, under *Tool*. The operation takes its tool's
row for the material, else its row for *Any material*: the values it uses (a profile the
slot values, a pocket both, a surface the clearing ones and its own wide stepover). A
material the tool has neither row for is listed as "Plywood (no values for this tool)";
choosing it changes nothing yet, and *Save to Plywood* fills it from the operation. *None*
leaves the operation's feeds as they are.

**For every operation:** the setup's *Material*, or *Use for all operations* on a
material. As one step (one Undo takes it back) every operation takes the material and its
values; an operation whose tool has no values there keeps its own, and the report says so.
Each project tool also takes its row's spindle speed. Operations you add later start with
this material.

**For new projects:** *Make default* on a material (`chipload material default Plywood`,
`--none` to clear it). A new project, empty or imported, starts with it, as it starts with
the default machine and tool; the report says so.

**The board's thickness for new projects:** on *This computer > Materials*, tick *New
projects: stock thickness* and type it (`chipload material thickness 18`, `--none` for the
built-in 10 mm). It is kept in `materials.json` beside the default material.

**Changing an operation's tool** takes the new tool's values for its material, when it has
some.

Each operation's material row says where its values stand: *From Plywood*, *Changed from
Plywood* (with *Use Plywood's* and *Save to Plywood*), or *Plywood has no values for this
tool* (with *Save to Plywood*). Editing a feed by hand keeps the material; the row then says
*Changed from Plywood*. When the tool's Plywood row is for another spindle speed than the
tool runs at, the row says that too.

The project keeps only each material's name (`material` on each operation, and on the
stock for the operations added later). The values are already in the operations, so the
project cuts the same on a computer without that material.

## On the command line

```
chipload material add Plywood
chipload tool cutting "6 mm 2-flute" --material Plywood --plunge-feed 500 \
  --slot-feed 1500 --slot-step-down 2.5 --feed 1800 --step-down 5 --stepover 0.4
chipload material list                         # every material with its tools' rows
chipload material show Plywood
chipload material default Plywood              # new projects start with it (--none: no default)
chipload material thickness 18                 # and with stock 18 mm thick (--none: 10 mm)
chipload material use part.chipload Plywood    # every operation, and those added later (--none: no material)
chipload material use part.chipload Plywood --op 2   # operation 2 alone
chipload material save part.chipload --op 2    # operation 2's values into its tool's row (--material NAME)
chipload material set Plywood --rename "Birch ply"
chipload material remove Plywood               # and every tool's row for it
chipload material export materials-backup.json # (--name NAME, repeatable)
chipload material import materials-backup.json [--replace]
```

`chipload op tool` on an operation with a material takes the material's values for the new
tool too.

Removing a material removes every tool's row for it. Materials, names only, live in
`materials.json` in the settings folder, beside the tool library; see the [Materials
file](../reference/materials-file.md). The rows live in `tools.json`.
