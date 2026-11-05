# Setups

A **setup** is one board clamped once on the machine: its stock, its work zero, and the
parts cut from it. Most projects have one and never need to think about it. A project needs
more when its parts come from more than one board, or from one board clamped more than once:

- a box with sides from 12 mm plywood and a bottom from 6 mm;
- a big sheet whose parts lie far apart, each group with its own zero nearby.

Each setup is a **program of its own**, run on its own, with its own setup sheet: the
operator clamps the board, sets the zero and runs that program, then the next. Chipload never
puts two boards in one program, because re-measuring in the middle of a program is where
crashes come from.

## What a setup holds

- **Its board:** size, thickness, fit, Z zero, max overcut and material, as
  [Stock and work zero](stock.md) describes. A board that fits the drawing fits **its own
  parts**: the shapes its operations cut.
- **Its work zero.**
- **Its parts**, each with its operations and copies ([Parts](../operations/parts.md)).

The drawing, the machine, the tools and the materials are the project's, shared by every
setup: you draw all parts in one drawing, a layer per part, and each setup's board is placed
around its own.

## Running

With one setup the files are named as always (`part.nc`, `part-1-T1.nc`, …). With more, each
file names its setup first: `part-S1.nc`, `part-S2-1-T1.nc`, and each setup's sheet beside
them (`part-S2-sheet.html`). `run` writes every setup that is on; `--setup N` only one.
`plan`, `sheet` and `simulate` work on the active setup, or the one `--setup N` names.

## In the window

The Browser shows **Setups**, always, each with its **Stock**, its **Work zero** and its
parts beneath it. Click a setup for its name, whether it is cut and its parts; its board and
zero are the two items beneath it. Right-click *Setups* or a setup for **Add a setup**, another board: it starts like the
active one, with a part of its own, and both become active. Right-click a setup for **New
part** in it.

With two or more, the setup you click is the **active setup**: it is shown in full colour,
generated, simulated and exported, while the other boards and their shapes are ghosts, faint
and out of the way. Right-click a part for **Move to setup ▸**. The machine, shared by every
setup, is under **Machine**.

Right-click a setup for **Export G-code of this setup…**: it becomes the active setup, is
planned if it isn't shown already, and its program and sheet are written. **Export G-code**
writes the active setup's program, named `-S1`, `-S2` as `run` names them,
with its own sheet. Make the next setup active and export it too. A setup whose parts still
have operations can't be removed: move them or delete them first.

![Two setups: the 6 mm bottom active, the 12 mm sides a ghost](../images/setups.png)

## On the command line

```sh
chipload setup add box.chipload --name "6 mm bottom" --thickness 6   # with a part of its own
chipload setup part box.chipload 2 --to 2        # part 2 is cut from setup 2's board
chipload setup list box.chipload
chipload setup set box.chipload 2 --name "Bottom" --off
chipload setup activate box.chipload 1
chipload setup remove box.chipload 2 --with-parts
chipload set box.chipload setups.1.stock.thickness=6     # a board's values, by path
chipload run box.chipload -o box.nc --setup 2
```

In paths, a setup's board is `setups.N.stock…` and its zero `setups.N.work_zero`, counting
from 0. The older `stock.…` and `machine.work_zero` still work and mean the active setup's.

## The back side (experimental)

A setup can be the **back of another**: the same board turned over, for parts cut on both
faces. It shares its front's board and work zero: change the thickness of the front, and the
back follows. A board that fits fits the shapes of both sides.

```sh
chipload setup add sign.chipload --back-of 1 --turn left-right   # "Setup 1 back", with a part
chipload setup set sign.chipload 2 --turn top-bottom --register corner
chipload setup set sign.chipload 2 --own-board    # a board of its own again, as it is now
```

`--turn` is how the board is turned over: **left-right** about its vertical centre line, or
**top-bottom** about its horizontal one. `--register` is how it is put back in place:
**pins** (dowel pins on the turn axis, the default) or **corner** (pushed into the same
corner of a fence). A board has one back, and a back can't be turned over again. Removing
the front gives its back a board of its own.

You draw the back as seen **from the front**, looking through the board: the back's
shapes lie where they are on the board, and the view, the toolpath and the markers stay the
front's. The back's program is mirrored when it is made, about the board's centre line, so
it cuts those shapes on the turned board. Climb stays climb and inside stays inside on the
back face.

The back's board is as thick as the front leaves it: a surfacing pass on the front makes the
back that much thinner, and its cuts through the stock follow.

Its work zero is the front's, on the board as it lies after turning: a stock corner is the
corner of the turned board in that place, so push the board against the same stops.

This is being built (docs/specs/two-sided.md): the dowel holes, the setup sheet's words for
turning the board, the window and a simulation of both sides are still to come.

