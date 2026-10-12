# Spec: small items of the improvement pass

Status: approved (2026-10-12; decisions 1–6 as recommended, chip load warning A). The small items fitted into the general improvement
pass (TODO.md, approved 2026-10-12): a chip load readout, drag and drop, a measure tool, copy,
paste and mirror of drawn shapes, and Help → Keyboard shortcuts. Each lands as its own
commit, in this order: shortcuts, drag and drop, measure, copy/paste/mirror, chip load.

## 1. Help → Keyboard shortcuts

A window listing every shortcut and mouse action, grouped as File, Edit, Generate, View,
Canvas and Drawing (for example "Ctrl+S Save", "F5 Generate", "Middle drag: pan", "Shift +
drag a segment's middle: bend it"). The shortcut list comes from one table in the GUI, the
same one the key handling and the menu labels use, so the window can't fall out of date. A
test checks that every shortcut the app handles is listed. F1 opens it.

## 2. Drag and drop

Dropping a file on the window:

- a `.chipload` project opens it, as File → Open does;
- a `.dxf` starts a new project from it, as Import DXF (Ctrl+I) does, including the units
  question for a drawing without units;
- the unsaved-changes question comes first, as for Open and Import.

Anything else, or more than one file, gets a status line ("Drop one .chipload or .dxf
file") and nothing else happens. While a file is dragged over the window, the canvas shows
what dropping it will do ("Open project", "New project from this DXF").

Not here: replacing the open project's drawing with a dropped DXF (that's File → Update
drawing). Under WSL, dropping from Windows Explorer may not reach the app; that is WSLg's
limit, not something the app can fix.

## 3. Measure tool

View → Measure (M), top view only:

- Click two points; each snaps to the drawing's ends, centres and middles as drawing does.
- A dimension line between them shows the distance, ΔX, ΔY and the angle
  (`L 26.926  ΔX 25.000  ΔY 10.000  21.8°`), in mm.
- The next click starts a new measurement. Esc, or choosing another tool, ends it.
- Nothing goes into the project and no undo step is made.
- It works with or without a project; without one there is nothing to snap to.

## 4. Copy, paste and mirror drawn shapes

Edit menu, with the selection of drawn shapes:

- **Copy (Ctrl+C)** keeps the selected drawn shapes in the app.
- **Paste (Ctrl+V)** adds them as new shapes (new ids, same layers). The group is centred on
  the cursor when the cursor is over the canvas, else moved 10 mm right and 10 mm up from
  where it was copied. The pasted shapes are then selected. One undo step.
- **Mirror left–right / Mirror up–down** flips the selected drawn shapes about the vertical
  or horizontal line through the centre of their bounding box. The shapes keep their ids,
  so operations that cut them follow (as when a shape is edited). One undo step.
- **How each shape mirrors:**
  - a circle: its centre;
  - a rectangle: its centre, and the angle becomes −angle (a rectangle turned half a
    turn is the same rectangle, so this holds for both directions);
  - a polyline: its points, with the bulges negated so arcs bend the mirrored way.
- **Imported shapes** can't be copied or mirrored in this step. The menu items say so on
  hover. The drawing as a whole can already be moved, turned and scaled (placement).
- **CLI first (rule 5):**
  - `chipload shape copy PROJECT ID... --dx 10 --dy 10` prints the new ids;
  - `chipload shape mirror PROJECT ID... --axis x|y` mirrors about the selection's centre
    (`--axis x` flips left–right).
  - Both go through new pipeline functions (`copy_shapes`, `mirror_shapes`) beside
    `add_shape`, with the same checks.
- Copy uses the app's own clipboard, not the system's. Pasting into another Chipload window
  can come later.

## 5. Chip load readout

How thick a chip each cutting edge takes: chip load = feed / (rpm × flutes), in mm per tooth.
It's shown:

- **In the operation form, next to Feed:** `0.042 mm/tooth`. When the tool has no flutes or
  rpm set it reads "chip load: set the tool's flutes" (or rpm).
- **On the setup sheet:** in the operation's feed column.
- **In the CLI:** `chipload plan`, one line per operation, and `chip_load_mm` per operation
  in `plan --json`.
- It is computed in the pipeline (`chip_load(job, op)`), so the GUI and CLI agree.

The plunge feed isn't given a chip load: the tool cuts with its tip when going down.

### When to warn: your decision

A good chip load depends on the material, the tool's diameter and coating, and the machine's
stiffness. There's no single safe range. The options:

- **A. No warning, just the number** (*recommended* for this step). Shown plainly, with a
  hover text explaining it and the usual ballpark ("wood and plastics with 3–12 mm tools:
  often 0.05–0.25 mm/tooth; aluminium: often 0.02–0.08"). It can't be wrong, and it already
  helps.
- **B. A rough amber warning by tool diameter**, regardless of material:
  - below 0.005 × D per tooth, the tool rubs and heats up (0.03 mm for a 6 mm tool);
  - above 0.05 × D per tooth, it risks breaking (0.3 mm for a 6 mm tool);
  - using the warning machinery planned for the live problems (amber at the field).

  Simple, but it would warn wrongly for some materials (for example metals at the low
  end).
- **C. A range per tool in the tool library** (min and max chip load fields): warnings only
  where the user has set them. Most accurate, but it needs a schema change to projects and
  the library, so it's a later step of its own.

## 6. Decisions to review

1. **Shortcuts:** in a window from Help (F1), built from one table that the menus and key
   handling also use. *Recommended.*
2. **Drag and drop:** a project opens; a DXF starts a new project, as Import does; one file
   at a time. *Recommended.* The alternative, a DXF dropped on an open project replacing its
   drawing, is too easy to do by accident.
3. **Measure:** two snapped clicks, nothing saved, top view only, key M. *Recommended.*
4. **Copy, paste and mirror** for drawn shapes only, with paste centred on the cursor, mirror
   in place about the selection's centre, and CLI commands first. *Recommended.* Copying
   imported shapes (converting them to drawn polylines) can come later.
5. **Chip load:** shown in the form, on the sheet and in the CLI. *Recommended.*
6. **Chip load warning:** A, B or C (§5). *Recommended: A now, C later.*

## 7. Tests

- **Shortcuts:** every handled shortcut appears in the window's table, and no two entries
  share a key combination.
- **Drag and drop:** a dropped `.chipload` opens and a dropped `.dxf` imports (through the
  same functions as the menus); unsaved changes ask first; other or several files change
  nothing.
- **Measure:** two clicks give the right distance, deltas and angle; snapping is used; the
  project and the undo stack are unchanged.
- **Copy, paste and mirror:**
  - pipeline: copied shapes get new ids, the same layers and moved values; mirroring twice
    gives the original back; a mirrored polyline's arcs bend the other way (the contour
    matches a mirrored copy of the original's points); imported shapes are refused;
  - CLI commands;
  - GUI: paste at the cursor and undo; operations follow a mirrored shape.
- **Chip load:** the formula; missing flutes or rpm give no number; `plan` and
  `plan --json` carry it; the form shows it.

## 8. As built (2026-10-12)

- **Shortcuts:** `app/shortcuts.rs`: one table (`BINDINGS`) that the key handling, menu
  labels and hover texts read; `OTHERS` lists canvas and drawing keys and mouse actions.
- **Drag and drop:** `app/dropping.rs`; `.json` (older jobs) opens too, as in File → Open.
- **Measure:** `app/measure.rs`; it also ends when you start drawing or switch to 3D.
- **Copy, paste and mirror:**
  - Unlike §4, the copied shapes also go onto the **system clipboard**, as JSON
    (`{"chipload_shapes": 1, "shapes": [...]}`, each shape as in a project file). egui only
    reports Ctrl+V when the system clipboard holds text, and Ctrl+C reaches it as a copy
    event, not a key press. So pasting into another Chipload window works already. Text that
    isn't Chipload shapes pastes nothing. The Edit menu's Paste uses the last copy.
  - This also fixes Ctrl+Shift+C (copy as selector), which arrived as a copy event and so
    never reached its shortcut.
  - Pipeline: `copy_shapes`, `paste_shapes`, `mirror_shapes`, `shapes_to_copy`,
    `shapes_bounds`, `clipboard_text`, `shapes_from_clipboard`; `chipload_job::edit::{translated,
    mirrored, Mirror}`.
  - The box round shapes is exact: new `Primitive::bounds` in core (arcs included), so a
    pasted circle is centred on the cursor exactly.
