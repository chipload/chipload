# Materials

A material keeps **your own** feeds, step down and stepover for each tool you use with it:
the numbers that work on your machine. Choose a material for an operation, or for the whole
project, and the operations cut with them.

Chipload has no built-in materials and doesn't calculate feeds. Catalogue values are made
for rigid industrial machines; a hobby router wants far less. As an example, a Shapeoko
cutting wood with a 6 mm 2-flute bit runs about 1500 mm/min with a 2–2.5 mm step down. Start
from what you know works on yours.

A material is about feeds only: choosing one never changes the stock or any depth.

## What a material holds

| | |
|---|---|
| **Name** | "Plywood", "Oak", "MDF", or "Plywood careful": unique in your library |
| **Values per tool** | for each tool of your [tool library](tools.md): feed, plunge feed, step down and stepover |

A material only needs values for the tools you use with it.

## Making one

The quickest way is from a job that cut well:
1. Make a material under *This computer > Materials > New material*, and choose it in an
   operation's **Material** list, under its tool.
2. Cut, and adjust the operation's feeds or step down until they're right.
3. Its **Material** row then says *Changed from Plywood*: click **Save to Plywood**. The next
   operation in plywood starts with these values.

You can also type them in: *This computer > Materials*, click the material, *Add a tool…*,
then fill in the values and **Save**.

The tool must be in your tool library: the values belong to that library tool, so every
project that uses it finds them. A tool only in the project is saved to the library first
(*Save to library…* on the tool).

## Choosing a material

**For one operation:** its **Material** list, under *Tool*. The operation takes the
material's values for its tool: feed and plunge feed, step down, and stepover for pockets (a
surface keeps its own wide stepover). A material without values for the tool is listed as
"Plywood (no values for this tool)"; choosing it changes nothing yet, and *Save to Plywood*
fills it from the operation. *None* leaves the operation's feeds as they are.

**For every operation:** the setup's *Material*, or *Use for all operations* on a
material. As one step (one Undo takes it back) every operation takes the material and its
values; an operation whose tool has no values there keeps its own, and the report says so.
Operations you add later start with this material.

**For new projects:** *Make default* on a material (`chipload material default Plywood`,
`--none` to clear it). A new project, empty or imported, starts with it, as it starts with
the default machine and tool; the report says so.

**Changing an operation's tool** takes its material's values for the new tool, when the
material has some.

Each operation's material row says where its values stand: *From Plywood*, *Changed from
Plywood* (with *Use Plywood's* and *Save to Plywood*), or *Plywood has no values for this
tool* (with *Save to Plywood*). Editing a feed by hand keeps the material; the row then says
*Changed from Plywood*.

The project keeps only each material's name (`material` on each operation, and on the
stock for the operations added later). The values are already in the operations, so the
project cuts the same on a computer without that material.

## On the command line

```
chipload material add Plywood
chipload material set Plywood --tool "6 mm 2-flute" --feed 1500 --plunge-feed 500 --step-down 2.5 --stepover 0.4
chipload material list                         # every material with its values per tool
chipload material show Plywood
chipload material default Plywood              # new projects start with it (--none: no default)
chipload material use part.chipload Plywood    # every operation, and those added later (--none: no material)
chipload material use part.chipload Plywood --op 2   # operation 2 alone
chipload material save part.chipload --op 2    # keep operation 2's values in its material (--material NAME)
chipload material set Plywood --rename "Birch ply"
chipload material remove Plywood --tool "6 mm 2-flute"   # one tool's values (without --tool: the material)
chipload material export materials-backup.json # (--name NAME, repeatable)
chipload material import materials-backup.json [--replace]
```

`chipload op tool` on an operation with a material takes the material's values for the new
tool too.

Removing a tool from the library (`tool remove`, or *Delete* on the tool) removes its
values from every material. Materials live in `materials.json` in the settings folder,
beside the tool library; see the [Materials file](../reference/materials-file.md).
