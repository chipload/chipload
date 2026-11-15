# Your first part

This walks through one part, from a DXF to G-code: a plate with holes, cut out with an
outside profile. The same steps follow in the app and on the command line.

## In the app

1. **Import the drawing:** *File > Import DXF…* (Ctrl+I).
   - If the DXF doesn't say whether it's in mm or inches, Chipload asks. It never guesses.
   - The [new project dialog](drawings/import.md#the-new-project-dialog) asks for the
     material, the board's thickness and size, Z zero, the machine and the tools. Set the
     real thickness: depths are measured from the board's top. Enter creates the project.
   - The report under the canvas lists what was imported, and anything skipped or
     repaired: duplicate lines, gaps it closed, splines turned into arcs.
2. **Check the setup and the machine:** the project opens on **Setup 1** under *Setups* in
   the [Browser](window.md#the-browser) on the left, with the board you chose; **Machine**
   shows the machine. [Work zero](concepts/stock.md#work-zero) is at the stock's bottom-left
   corner (the *Work zero* row under the setup; drag it in the view to move it). Without a
   [tool library](library/tools.md) and [machine profiles](library/machines.md), a project
   has the GRBL post, a safe height of 5 mm and a 6 mm endmill at 18 000 rpm.
3. **Select what to cut:**
   - double-click the outline on the canvas (a single click takes the whole DXF);
   - Ctrl+click adds more;
   - right-click selects a whole layer;
   - right-click a shape for **Select similar** (every circle of its diameter, on any
     layer, or every shape of its size on its layer) or **Select inside** (the closed
     shapes within an outline: a part's pockets and holes). With Ctrl held they add to the
     selection. On the command line, `chipload shape similar part.chipload 5` and
     `chipload shape inside part.chipload 1` print the shape ids, for `op add … --shapes`;
   - or click a DXF or one of its layers in the Browser.
4. **Add an operation:** **Profile**, in the ribbon's OPERATION group.
   - Quicker: press **S** over the canvas, then **P**. The quick menu opens at the mouse with a
     key for each kind (P profile, K pocket, H holes, V V-carve, F follow path, C chamfer, L flute) and a digit
     for each of your [styles](library/styles.md). A kind the selection can't take is
     greyed out, and its hover says why (holes need a circle, a pocket a closed shape).
     Escape closes it.
   - **Side:** Chipload picks *outside* for an outline and *inside* for holes, and asks
     when a selection mixes both.
   - **End depth:** for a cut-out, the stock thickness plus a little, e.g. 10.3 mm into a
     spoilboard. More than the allowed overcut (0.5 mm by default, in the setup's Stock) is
     refused.
   - **Step down:** how deep each pass goes.
   - **Tabs:** they keep the part from coming loose on the last pass. Add them under
     *Tabs*.
5. **Generate:** F5, or leave *View > Update toolpath automatically* on.
   - The toolpath shows on the canvas: arrows give the cutting direction, rapids are
     dashed, and each operation's number sits at its first plunge.
   - Problems show in red on the field they're about.
   - Warnings appear in the report.

   ![A generated toolpath: the Browser on the left, the canvas, the job settings on the right and the report below](images/toolpath.png)
6. **Look at it:** **3D** shows the toolpath at its real depths. **Simulate** (F6) shows
   the stock as cut, and the slider under the canvas plays the program back; F6 again goes
   back to editing.
7. **Export:** *File > Export G-code* (Ctrl+E).
   - You can only export once the [checker](#the-checker) has passed the program.
   - The first time, it asks where to write. After that, Ctrl+E writes the same file again
     without asking, as Ctrl+S saves, so a changed depth is one key away from the machine.
     The project remembers its file for next time. *File > Export G-code as…*
     (Ctrl+Shift+E) always asks.
   - Next to the G-code comes a setup sheet (`part-sheet.html`) for the operator: stock,
     zero, tools in order, operations and times.
8. **Save the project:** Ctrl+S. A project (`.chipload`) keeps the drawing itself, so the DXF
   isn't needed again unless it changes.

## On the command line

```sh
chipload import plate.dxf -o plate.chipload        # add --units mm if the DXF doesn't say
chipload inspect plate.chipload                    # contours with their numbers and layers
chipload set plate.chipload stock.thickness=12 \
  'operations.0={"type": "profile", "name": "Outline", "part": 1,
    "select": {"layer": "CUT_OUTSIDE"}, "side": "outside", "direction": "climb", "tool": 1, "start_depth": 0,
    "end_depth": 12.3, "clearance": 0, "step_down": 3, "feed": 1500, "plunge_feed": 400,
    "ramp_angle": 0.0524,
    "tabs": {"count": 4, "width": 6, "height": 2, "flank_angle": 1.5707963267948966}}'
chipload plan plate.chipload                       # warnings, cycle time, chip load
chipload run plate.chipload -o plate.nc            # checked G-code and plate-sheet.html
```

- `set` adds the operation (see [Scripting](scripting.md)); `part` is the project's one
  part, which a new project has as part 1.
- `select` picks the outline by its layer, as `inspect` lists it. To pick single shapes
  instead, use `{"shapes": [ids]}`: `inspect --json` gives each contour's shape ids.
- Angles in project files are in radians: 0.0524 is the 3° spiral ramp every profile goes
  down at, and 1.5707963267948966 is 90°, a straight tab.
- The app makes the same project. Open `plate.chipload` in it to see and change the
  operation.

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
