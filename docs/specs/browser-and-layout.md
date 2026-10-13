# The Browser and a more professional layout

Status: approved 2026-10-13. Supersedes the left panel of sidebar-sections.md and the panel
part of drawing-sources.md §4 once approved.

## 1. Why

The left panel has grown by addition: a *Job settings* button, a Drawing section with
*Select* and *⋯* buttons on every row, a selection read-out, *Copy as selector*, a row of
*+ Profile …* buttons, and operation cards with ↑ ↓ *Duplicate* *Delete* under them. It
works, but it reads as a form, not as a CAD program. The user asked for a left panel called
**Browser** that looks like the Browser in SolidCraft (the Fusion 360 style tree), and for a
more professional layout overall.

We take the *look and behaviour* of that style. No code is taken from SolidCraft or any
other program; the tree, icons and styling are written here.

## 2. What the Browser looks like

```
BROWSER
▾ ◫ two_dxfs                          ← the project (name of the file)
  ▸ ⚙ Job settings                    ← stock, machine, posts; click: properties
  ▸ ⌀ Tools (2)                       ← T1 endmill Ø6, T2 V-bit 60°
  ▾ ▱ Drawing (3)
    ▾ 👁 ▤ simple_part      7         ← a DXF; eye hides it; count weak, right-aligned
        👁 ≡ CUT_OUTSIDE   1
        👁 ≡ HOLES         6
    ▸ ⊘ ▤ part B            7         ← hidden: struck eye, name dimmed
    ▸ 👁 ▣ logo.png         4
    ▸ 👁 ◯ Drawn shapes     2
  ▾ ⛭ Operations (4)
      ✓ ⊓ 1 Holes          T1  ●      ← included tick, kind icon, number, name, tool, problem dot
      ✓ ⊓ 2 Outline        T1
      ✓ ⊓ 3 Outline B      T1
      ─ tool change → T2 ─
      ✓ ∨ 4 Logo           T2
```

- **Rows** are flat, one line, about 22 px high, with no buttons in them. Hovering a row
  shades it and a selected row gets the accent fill across the whole width.
- **Indent guides:** a thin vertical line under each open folder, as in SolidCraft.
- **Icons** are small line drawings that we paint ourselves with egui's painter, one per
  kind of row: project, settings, tools, DXF, layer, picture, drawn shape, and one per
  operation kind. Drawing them ourselves avoids an icon font, which would raise the font
  licence question from 2026-10-09.
- **Counts** sit right-aligned in weak text. A problem shows as a red dot at the right end
  of its row, with the problem as hover text.
- **Captions** ("BROWSER", "PROPERTIES") are small capitals in weak text, as SolidCraft
  does it, instead of today's shaded header bars.

## 3. What the rows do

| Action | Today | Browser |
|---|---|---|
| Select a DXF, layer or picture | *Select* button | click the row (Ctrl+click adds) |
| Place, update, remove a DXF | *⋯* menu | right-click menu; *Place…* also on double-click |
| Edit or remove a picture | *⋯* menu | right-click menu; double-click edits |
| Hide a group or layer | checkbox | eye icon |
| Open the job settings | *⚙ Job settings* button | click *Job settings* (or Stock / Machine under it) |
| Open an operation | click its card | click its row |
| Turn an operation off | checkbox on the card | tick at the row's left |
| Reorder operations | ≡ handle, ↑ ↓ | drag the row; Alt+↑ / Alt+↓ |
| Duplicate, delete, add rest pocket | buttons under the list | right-click menu, Ctrl+D, Delete |
| New operation | *+ Profile* … buttons | toolbar (§5) and right-click on *Operations* |

The canvas and the Browser stay in step. Clicking a contour selects the row of its layer or
picture without expanding anything, and selecting an operation outlines what it cuts, as
today.

## 4. Properties on the right

The right panel gets the caption **PROPERTIES** and a title line naming what it shows,
for example "Profile · 2 Outline" or "Job settings".

The selection read-out and the drawn-shape editor move out of the left panel into this
panel. They show whenever shapes are selected and no operation or settings row is. That
includes "N shapes selected", the circle, rectangle or polyline fields, and *Copy as
selector*.

Today's forms stay as they are, inside collapsing sections. Their label column gets one
fixed width, so the values line up from section to section.

## 5. Header: top bar and ribbon

Today's header is a menu bar (File, Edit, Job, Settings, View, Draw, Help) over a row of
text buttons. Following SolidCraft, it becomes two bands.

**Top bar**, one slim line in a darker fill:

```
◆ Chipload  📂 💾 ↶ ↷        [ two_dxfs.chipload ● ]                         ⚙  ?   v0.16.1
```

- From the left: the logo and name, then quick icons for Open, Save, Undo and Redo.
- In the middle, the project name on a tab. A dot on the tab means unsaved changes.
- On the right: Settings, Help and the version.

**Ribbon**, groups of icon buttons. The caption under each group is a drop-down that holds
that group's whole menu, so nothing in today's menus is lost:

```
 FILE ▾          DRAW ▾          OPERATION ▾                                 VIEW ▾               
[New][Open]     [○][▭][⌇]       [Profile][Pocket][V-carve][Follow][Holes]   [Show…]              [▶ Generate][⬇ Export]
 [Save][+DXF]    [Image]
```

- **FILE ▾** holds today's File menu. It also takes the project-wide items of the
  Edit and Job menus.
- **DRAW ▾** holds today's Draw menu, plus adding a picture.
- **OPERATION ▾** has the new-operation items, plus rest pocket and duplicate.
- **VIEW ▾** holds today's View menu, with Show toolpath, Show grid and the theme. The
  Fit, Top, 3D and Simulation buttons move onto the canvas (§6b).
- Help and Settings move to the top bar's icons.

Each button is an icon over a short word. The icons are painted the same way as the
Browser's. *Generate* stays the one accent-coloured button, at the right end, with
*Export G-code* beside it.

The window keeps the system's own title bar. Drawing our own, as SolidCraft does on
Windows, would bring per-platform quirks: dragging, snapping and Wayland. That can come
later if wanted.

## 6. Overall look

- **Panels against the canvas:** the side panels get a fill a step lighter than the
  canvas in the dark theme, and a step darker in the light one, with a 1 px border where
  they meet it. There are no gaps between panels.
- **Spacing:** one grid of 4 and 8 points. Fewer separators, with whitespace and the
  captions dividing sections instead.
- **Status bar:** the hint stays on the left. The right side gets the units (mm), the
  post-processor and the cycle time once generated, in weak text.
- **Fonts and colours:** both stay as they are, with WCAG AA contrast still checked in
  style.rs. The theme setting keeps light and dark.

## 6b. The canvas

Today the canvas has a flat dark fill and short axis arrows at work zero. A snap grid that
follows the zoom (`draw::grid_step`) exists, but it isn't drawn. Following SolidCraft:

- **A drawn grid** on the work plane (Z 0) in the top view and in 3D, where it lies flat
  and turns with the view, as SolidCraft's does.
  - **Minor lines** fall exactly on the snap grid, so a click lands on a visible line.
  - **Major lines** come every 10 minor steps. The spacing follows the zoom, and lines
    fade out instead of crowding when they get closer than a few pixels.
  - The **current step** shows in the status bar: "grid 1 mm".
- **Axis lines through work zero:** X in red and Y in green, running the whole width of
  the grid. In 3D, Z is a short blue line. These replace today's short arrows.
- **Background:** a soft vertical gradient. It is light grey in the light theme and slate
  in the dark one, and follows the app theme; today the canvas is dark in both. The
  stock, drawing and toolpath colours are checked for contrast against both backgrounds.
- **Navigation bar:** a small floating bar at the canvas's bottom centre with Fit, Top,
  3D, Simulation and Grid on or off. These are today's toolbar view buttons, moved where
  the eye is.
- **Orientation cube** in the top right corner, in 3D only. It shows TOP, FRONT and RIGHT
  faces that turn with the view, and clicking a face turns the view to it. In the top
  view it is a small XY axis sign instead.
- *View > Show grid* and the key **G** turn the grid off. The choice is remembered in the
  app settings, as an optional field, so no settings schema bump.

## 7. Decisions

1. **Name and structure:** the left panel becomes **Browser**: one tree with the project
   as its root and Job settings, Tools, Drawing and Operations as folders.
   *Recommended.*
   - Alternative: keep two stacked sections, Drawing and Operations, and restyle them as
     trees.
2. **Operations inside the Browser:** they are rows in the same tree.
   *Recommended.*
   - Alternative: a separate panel, as Fusion's CAM workspace keeps setups.
   - With the operations inside, the draggable split between Drawing and Operations goes
     away. Each folder opens and closes on its own instead, and the whole tree scrolls.
3. **Row actions:** right-click menus and keyboard shortcuts replace the *Select*, *⋯*,
   ↑ ↓, *Duplicate* and *Delete* buttons. *Recommended.*
   - The risk is that menus are less discoverable. The guide and the hover text say "right-click
     for more".
4. **Selection and shape editor:** both move to the Properties panel (§4).
   *Recommended.*
5. **New-operation buttons:** they move to the ribbon (§5), with a right-click on
   *Operations* as a second way in. *Recommended.*
6. **Header:** the menu bar and toolbar give way to the top bar and ribbon of §5, with
   the menus as drop-down captions over each group. *Recommended.*
   - Alternative: keep a classic File / Edit / … menu bar above the ribbon. That costs one
     more line of height.
7. **Window frame:** keep the system's title bar. *Recommended.*
   - A custom frame, as SolidCraft has, can be decided later.
8. **Icons:** we paint them ourselves with egui's painter.
   *Recommended.*
   - Alternative: an icon font, such as a Material Symbols or Lucide build. Both are
     Apache-2.0 or ISC, so allowed, but each adds a dependency and a font file to the
     binary.
9. **Turning an operation off:** a tick at the row's left. The eye is kept for "hide on the
   canvas" only, so a hidden shape is never mistaken for one that isn't cut, and an operation
   that's off is never mistaken for one that's only hidden. *Recommended.*
10. **Canvas grid:** drawn, with its minor lines on the snap grid, plus axis lines
   through work zero (§6b). *Recommended.*
11. **Canvas background:** it follows the app theme, with a light grey gradient in the
   light theme. *Recommended.*
   - Alternative: keep the canvas dark in both themes, as most CAM programs do.
12. **Navigation bar and orientation cube** on the canvas, with the view buttons moving
   there from the header (§6b). *Recommended.*
13. **Order of work:** four commits, each with its guide pages and screenshots updated.
   1. The Browser (§2, §3).
   2. Properties (§4).
   3. The header and overall look (§5, §6).
   4. The canvas (§6b).

   The first one alone already answers the request. *Recommended.*

## 8. What stays the same

- No change to the project file, the pipeline or the G-code. This is GUI only, so no
  schema bump and no golden changes.
- Every action in today's menus stays reachable: from the ribbon's drop-down captions or
  the top bar. Keyboard shortcuts stay as they are.
- The GUI tests keep driving the same actions. Tests that click *Select* buttons or cards
  move to the new rows.

## 9. Tests

- The tree lists every DXF, layer, picture and drawn kind, and every operation, in project
  order. The counts match.
- Clicking a row gives the same selection as today's *Select*. Ctrl+click adds.
- Hidden rows stay hidden when saved and loaded again (`drawing.hidden`, as now).
- The right-click actions do what the old buttons did, through the same functions.
- Drag reordering and Alt+↑/↓ give the same order as ↑ ↓ did.
- The screenshots (`packaging/docs/screenshots.sh`) are renewed and looked at in both themes.
