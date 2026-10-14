# The window

## The header

At the top, a slim **top bar**:
- **Left:** Open, Save, Undo and Redo.
- **Middle:** the project's name on a tab. A dot on it means unsaved changes.
  Closing, opening or starting a project with unsaved changes asks first, in the middle of
  the window: **Save** (then go on), **Don't save** or **Cancel**.
- **Right:** the **colour theme** (a click goes System → Light → Dark), **? Help** (user
  guide, keyboard shortcuts, about) and the version.

Under it, the **ribbon** groups the buttons:

| Group | Buttons | Its caption's menu also has |
|---|---|---|
| FILE | New, Open, Import (a DXF as a new project), Save | open recent, save as, update drawing, export, the *…from path* items, undo, redo, quit |
| DRAW | Circle, Rectangle, Polyline, Image, Add DXF | copy, paste, mirror and delete drawn shapes; copy as selector |
| OPERATION | Profile, Pocket, V-carve, Follow, Holes (for the selected shapes) | Generate |
| LIBRARY | Tools, Machines: open them under *This computer* in the Browser | the same |
| VIEW | Measure | fit, top and 3D view, Z scale, what is shown, legend, update automatically, colour theme |

**Generate** (F5) and **Export** sit at the right end.

Each group's caption (**FILE ▾** …) opens its whole menu. This guide writes those menus as
*File > …*, *Draw > …* and *View > …*. *Help > …* is the question mark in the top bar.

The **status bar** at the bottom gives hints on the left. On the right it shows the
cursor's position, the number of copies, the cycle time once generated, the
post-processor and the units.

## The Browser

The **Browser** on the left holds your library and the whole project as a tree:

- **This computer**, at the top and folded at the start: what is kept on this computer for
  every project. It is there also with no project open.
  - **Preferences:** the [setup sheet](checking/sheet.md), updating the toolpath by
    itself, and the colour theme.
  - **Tools** and **Machines:** your [tool library](library/tools.md) and [machine
    profiles](library/machines.md).
  - **Post-processors:** the built-in ones and [yours](posts.md#your-own-post); click one
    for what it writes, and *Use in this project*.
- **The project**, named after its file.
  - **Job settings:** stock, machine, post-processor and tools, shown on the right when
    clicked. A red dot means one of them has a problem.
  - **Tools:** the project's tools, each with the library tool it came from at the right
    ("← 6 mm 2F", "(changed)" when the values differ). See [Tool
    library](library/tools.md#project-tools-and-the-library).
  - **Drawing:** what the shapes come from. Each DXF has its layers under it; then come
    each picture, and the shapes drawn in Chipload by kind (circles, rectangles,
    polylines). See [Several drawings](drawings/several.md).
  - **Parts**, each with its operations under it, in cutting order. A line marks each
    tool change. See [Parts](operations/parts.md).

![The Browser: two DXFs with their layers, the second hidden, a picture, and four
operations; the first DXF's placement window is open](images/drawing.png)

**Rows:**
- **Click** a row to select it.
  - A DXF, picture or layer selects its shapes for a new operation, and Ctrl+click adds
    to the selection.
  - An operation, a part, a tool, *Tools*, *Job settings*, a DXF (or one of its layers)
    or a picture shows its settings on the right, in [Properties](#properties).
- **Double-click** a picture for its trace settings, or a part to make it the active part.
- **Right-click** a row for the rest:
  - **A DXF:** *Place…* (its properties), *Update from a DXF…* and *Remove*.
  - **A picture:** *Trace settings…* and *Remove*.
  - **An operation:** *Move up* and *Move down* (also Alt+↑ and Alt+↓), *Duplicate*, *Add
    rest pocket* and *Delete*.
  - **The Drawing folder:** *Add DXF…* and *Import image…*.
  - **A part:** *Activate*, *New operation* (for the selected shapes), *Move up*, *Move
    down* and *Delete part* (with its operations).
  - **The Parts folder:** *New part*.
  - **A library tool:** *Add to project*, *Make default*, *Delete*. **A library machine:**
    *Use in this project*, *Make default*, *Delete*. **This computer > Tools or Machines:** *New*,
    *Import…*, *Export…*.
- **The triangle** opens and closes a folder, and so does a click on a plain folder like
  *Tools*.

**The eye** hides a DXF, layer, picture or kind of drawn shape on the canvas: it isn't
drawn and can't be clicked. **Hidden shapes are still cut.**

**The tick** in front of an operation or a part turns it on or off. One that is off isn't
in the program, and its row is dimmed; a part that is off takes its operations with it.

**The active part** is in bold, marked *active*. New operations from the ribbon go into
it, whatever is open. Its name is also in the status bar.

**On an operation's row:**
- **Right end:** its tool, and how many hidden contours it cuts ("3 hidden · T1").
- **A red dot** marks a problem.
- **Hover** the row for its kind, its geometry, its depths, its time once generated and
  its first problem.
- **Drag** the row onto another to move it there, or onto a part to move it to the end of
  that part.

**Drag a library tool** onto the project's *Tools* to copy it into the project.

## The canvas

The canvas shows the stock, the drawing and the toolpath on a soft gradient. It follows the
colour theme: dark, or light grey.

- **The grid** lies on the stock top, in top view and in 3D.
  - Its fine lines are where clicks snap while drawing. A stronger line comes every ten.
  - It follows the zoom, and the status bar gives its step ("grid 5 mm").
  - **G** or *View > Show grid* hides it, and Chipload remembers that.
- **The axes:** a red line along X and a green one along Y run through the program's zero.
  Short thick strokes point the positive way, and in 3D a blue one points up Z.
- **The navigation bar** floats at the bottom, each button an icon over its name:
  - how you look: **Fit** (F), **Top** (T) and **3D**;
  - what is shown: **Stock** (the [simulation](checking/simulation.md)), **Toolpath**,
    **Images** and **Grid** (G).
- **The corner:**
  - In 3D, the top right corner has a cube that turns with the view. Click a face (TOP,
    FRONT, RIGHT, BACK, LEFT) to look straight at it.
  - In top view, the corner has an X–Y sign instead.
- **The legend** (bottom left) explains the colours.

![The canvas in 3D: the simulated stock on the grid, the cube in the corner and the
navigation bar](images/simulation-3d.png)

## Properties

The **Properties** panel on the right shows what is open, under a title:
- **Job settings:** the stock, the machine and the post.
- **Preferences:** the settings folder, whether a [setup sheet](checking/sheet.md) is
  written with the G-code, updating the toolpath by itself, and the colour theme.
- **A post-processor:** its name, files, extension, decimals, arcs and tool changes.
- **Tools:** every tool, with *Add from library…* and *New tool*; **a tool** alone, with
  how it stands to the library tool it came from, *Save to library* and *Remove tool*.
- **A library tool or machine:** its values, with *Save* and *Revert* while it has changes.
- **A DXF:** its file, units and size, its [position and scale](drawings/placement.md),
  *Update from a DXF…* and *Remove*.
- **A picture:** its width, lower-left corner and layer. A change applies when you finish
  the edit, and traces the picture again. *Trace settings…* opens its
  [window](drawings/images.md).
- **A part:** its name, on or off, and its [copies](drawings/copies.md).
- **An operation:** its settings, under its kind and number ("Pocket · Operation 1").

While shapes are selected, a **Selection** section comes first. It holds:
- what is selected, by contour number;
- a drawn shape's exact values, to edit;
- *Copy as selector*, which copies the selection as JSON for a project file.

Its triangle folds it away, so the operation's settings come up.

![Properties: a drawn polyline selected, with its points, above operation 1's settings](images/properties.png)
