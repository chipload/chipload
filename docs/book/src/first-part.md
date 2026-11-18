# Your first part

This walks through one part, from nothing to G-code: a plate of 160 × 100 mm with rounded
corners and four holes, drawn in Chipload and cut out with profiles. The same steps follow
in the app and on the command line. Starting from a DXF or an SVG instead differs only in
the first steps ([below](#starting-from-a-drawing-file)).

## In the app

1. **A new project:** *File > New project* (Ctrl+N).
   - The [new project dialog](drawings/import.md#the-new-project-dialog) asks for the
     material, the board's thickness and size, Z zero, the machine and the tools. Set the
     real thickness, 12 mm here: depths are measured from the board's top. Leave the board
     on *Fit the drawing*: it grows round what you draw. Enter creates the project.
2. **Draw the plate:** press **R** (or the rectangle in the ribbon's DRAW group) and click
   two opposite corners on the canvas, anywhere.
   - The rectangle stays selected, and its values are under *Selection* in the
     [Properties](window.md#properties) panel on the right. Type the exact ones: centre
     0, 0, length 160, width 100, corner radius 8.
   - Clicks land on the [grid](window.md#the-canvas) and [snap](drawings/shapes.md#snapping)
     to what is drawn, so values usually start out round.
3. **Draw the holes:** press **C**, then type `-65,-35` and Enter for the centre, and `4`
   and Enter for the radius. A typed value opens beside the cursor as you type
   ([Exact values](drawings/shapes.md#exact-values)).
   - **Enter** starts the circle again: `65,-35`, `65,35` and `-65,35`, each with radius 4.
   - Esc ends the tool. Every drawn shape can be changed later: drag its handles, or type
     its values under *Selection*. [Drawing shapes](drawings/shapes.md) has the other
     tools: lines and arcs, polygons, slots, offsets, fillets, arrays.
4. **Check the stock and the machine:** **Stock** in the [Browser](window.md#the-browser)
   on the left is the board you chose; **Machine** shows the machine.
   [Work zero](concepts/stock.md#work-zero) is at the stock's bottom-left corner (the *Work
   zero* row; drag it in the view to move it). Without a [tool library](library/tools.md)
   and [machine profiles](library/machines.md), a project has the GRBL post, a safe height
   of 5 mm and a 6 mm endmill at 18 000 rpm.
5. **Select what to cut:** drag a box from left to right round the whole plate: it selects
   every shape wholly inside it.
   - A click selects one shape, Ctrl+click adds one; right-click selects a whole layer.
   - Right-click a shape for **Select similar** (every circle of its diameter, or every
     shape of its size on its layer) or **Select inside** (the closed shapes within an
     outline: a part's pockets and holes). With Ctrl held they add to the selection. On
     the command line, `chipload shape similar plate.chipload 2` and `chipload shape inside
     plate.chipload 1` print the shape ids, for `op add … --shapes`.
6. **Add an operation:** **Profile**, in the ribbon's OPERATION group.
   - Quicker: press **S** over the canvas, then **P**. The quick menu opens at the mouse
     with a key for each kind (P profile, K pocket, H holes, V V-carve, F follow path, C
     chamfer, L flute, E pattern, R prism, M moulding) and a digit for each of your
     [styles](library/styles.md). A kind the selection can't take is greyed out, and its
     hover says why (holes need a circle, a pocket a closed shape). Escape closes it.
   - **Side:** an outline is cut *outside* and holes *inside*. Selected together, as here,
     they become two profiles, the holes first, so the plate is still held while they are
     cut.
   - **End depth:** for a cut-out, the stock thickness plus a little, e.g. 12.5 mm into a
     spoilboard. More than the allowed overcut (0.5 mm by default, in the setup's Stock) is
     refused.
   - **Step down:** how deep each pass goes.
   - **Tabs:** they keep the part from coming loose on the last pass. The outline's
     profile has them; change them under *Tabs*.
7. **Generate:** F5, or leave *View > Update toolpath automatically* on.
   - The toolpath shows on the canvas: arrows give the cutting direction, rapids are
     dashed, and each operation's number sits at its first plunge.
   - Problems show in red on the field they're about.
   - Warnings appear in the report.

   ![A generated toolpath: the Browser on the left, the canvas, the job settings on the right and the report below](images/toolpath.png)
8. **Look at it:** **3D** shows the toolpath at its real depths. **Simulate** (F6) shows
   the stock as cut, and the slider under the canvas plays the program back; F6 again goes
   back to editing.
9. **Export:** *File > Export G-code* (Ctrl+E).
   - You can only export once the [checker](#the-checker) has passed the program.
   - The first time, it asks where to write. After that, Ctrl+E writes the same file again
     without asking, as Ctrl+S saves, so a changed depth is one key away from the machine.
     The project remembers its file for next time. *File > Export G-code as…*
     (Ctrl+Shift+E) always asks.
   - Next to the G-code comes a setup sheet (`plate-sheet.html`) for the operator: stock,
     zero, tools in order, operations and times.
10. **Save the project:** Ctrl+S, as `plate.chipload`.

## On the command line

```sh
chipload new -o plate.chipload --thickness 12 --size 200x150    # an empty project
chipload shape add plate.chipload rectangle --center 0,0 --length 160 --width 100 \
  --corner-radius 8                                              # shape 1
for at in -65,-35 65,-35 65,35 -65,35; do
  chipload shape add plate.chipload circle --center $at --radius 4   # shapes 2 to 5
done
chipload op add plate.chipload profile --shapes 1 2 3 4 5 --through
chipload plan plate.chipload                       # warnings, cycle time, chip load
chipload run plate.chipload -o plate.nc            # checked G-code and plate-sheet.html
```

- `new` takes the rest from your library's defaults, as the dialog does; `--fit 10` in
  place of `--size` fits the board round the drawing.
- `op add` makes the same two profiles the app does, the holes first, with the defaults
  the app gives (`--through`: through the board, into the spoilboard by the allowed
  overcut). Every value can be changed afterwards with `set` (see
  [Scripting](scripting.md)).
- The app makes the same project. Open `plate.chipload` in it to see and change it.

## Starting from a drawing file

A part drawn in a CAD program comes in as a DXF or an SVG, and the rest is the same:

- **In the app:** *File > Import DXF, SVG or STL…* (Ctrl+I) in place of *New project*: the same dialog
  opens, and the drawing becomes part of the project. If a DXF doesn't say whether it's
  in mm or inches, Chipload asks; it never guesses. The report under the canvas lists
  what was imported, and anything skipped or repaired: duplicate lines, gaps it closed,
  splines turned into arcs.
- **Selecting:** a single click on the canvas takes the whole drawing file; double-click
  takes one outline. A drawing file or one of its layers can be clicked in the Browser.
- **On the command line:** `chipload import plate.dxf -o plate.chipload` (add `--units mm`
  if the DXF doesn't say), then `chipload inspect plate.chipload` for the contours with
  their numbers and layers, and `op add plate.chipload profile --layer CUT` for every
  shape on a layer.
- The project keeps the drawing itself, so the file isn't needed again unless it changes;
  then *File > Update drawing from DXF or SVG…* takes the new version and keeps your operations
  ([Updating](drawings/update.md)). Imported and drawn shapes mix freely in one project.

## The checker

Before Chipload writes a program, an independent checker reads the G-code back and refuses
it if it finds, among other things:
- a rapid move (G0) down into the stock, or sideways while the tool is below the safe
  height;
- a cut deeper than the stock bottom plus the allowed overcut;
- a move beyond the machine's travel, when the travel is set;
- an arc whose ends don't lie on its circle;
- a cutting move with no feed, or with the spindle off;
- other units than the machine's, or relative coordinates;
- a tool change with the spindle running or below the safe height.

The checker is separate code that shares nothing with the part of Chipload that makes the
toolpath, so a bug in one is likely caught by the other. `chipload check part.nc --job
part.chipload` runs it on any G-code, also from other programs.

## Before you cut

- **Work zero:** the setup sheet says where X0 Y0 and Z0 are. Set them on the machine
  exactly there.
- **Clamps:** look at the 3D view or the simulation for clamps in the toolpath.
  Chipload doesn't know where your clamps are.
- **Air cut:** run the program once above the stock.
- **What holds the part:** in the last pass, tabs or an onion skin should keep it in
  place.
