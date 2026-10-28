# Materials

A material keeps **your own** feeds, step down and stepover for each tool you use with it:
the numbers that work on your machine. Choose the material for a project, and its
operations cut with them.

Chipload has no built-in materials and doesn't calculate feeds. Catalogue values are made
for rigid industrial machines; a hobby router wants far less. As an example, a Shapeoko
cutting wood with a 6 mm 2-flute bit runs about 1500 mm/min with a 2–2.5 mm step down. Start
from what you know works on yours.

## What a material holds

| | |
|---|---|
| **Name** | "Plywood", "Oak", "MDF": unique in your library |
| **Thickness** | optional: the board it usually comes as. Choosing the material sets the stock to it |
| **Values per tool** | for each tool of your [tool library](tools.md): feed, plunge feed, step down and stepover |

A material only needs values for the tools you use with it.

## Making one

The quickest way is from a job that cut well:
1. In *Job settings > Stock*, choose the **Material** (make one first under *This computer >
   Materials > New material*).
2. Cut, and adjust an operation's feeds or step down until they're right.
3. The operation's **Material** row then says *Changed from Plywood*: click **Save to
   Plywood**. The next project in plywood starts with these values.

You can also type them in: *This computer > Materials*, click the material, *Add a tool…*,
then fill in the values and **Save**.

The tool must be in your tool library: the values belong to that library tool, so every
project that uses it finds them. A tool only in the project is saved to the library first
(*Save to library…* on the tool).

## Choosing a material for a project

*Job settings > Stock > Material*, or *Use in project* on a material. As one step (one
Undo takes it back):
- the stock takes the material's **thickness**, if it has one;
- a cut that went **through** the old board still goes through the new one, as far below
  it as before;
- every operation whose tool has values in the material takes them: feed and plunge feed,
  step down, and stepover for pockets (a surface keeps its own wide stepover);
- an operation whose tool has no values there keeps its own, and the report says so.

Operations you add later take the material's values for their tool. Changing an
operation's tool keeps its feeds; its Material row then offers **Use Plywood's** values.

Each operation's **Material** row says where its values stand: *From Plywood*, *Changed
from Plywood* (with *Use Plywood's* and *Save to Plywood*), or *Plywood has no values for
this tool* (with *Save to Plywood*).

The project keeps only the material's name. Its values are already in the operations, so
the project cuts the same on a computer without that material.

## On the command line

```
chipload material add Plywood --thickness 18
chipload material set Plywood --tool "6 mm 2-flute" --feed 1500 --plunge-feed 500 --step-down 2.5 --stepover 0.4
chipload material list                         # every material with its values per tool
chipload material show Plywood
chipload material use part.chipload Plywood    # choose it for a project (--none: no material)
chipload material save part.chipload --op 2    # keep operation 2's values in the project's material (--material NAME)
chipload material set Plywood --rename "Birch ply" --no-thickness
chipload material remove Plywood --tool "6 mm 2-flute"   # one tool's values (without --tool: the material)
chipload material export materials-backup.json # (--name NAME, repeatable)
chipload material import materials-backup.json [--replace]
```

Removing a tool from the library (`tool remove`, or *Delete* on the tool) removes its
values from every material. Materials live in `materials.json` in the settings folder,
beside the tool library; see the [Materials file](../reference/materials-file.md).
