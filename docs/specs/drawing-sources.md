# Spec: several drawings and pictures in one project, a Drawing panel, hiding

Status: **approved** (2026-10-13, all recommendations taken). Built: all (as built: §10).

The user's request:
- Import several DXFs and pictures into one project, each with its own position and scale.
- Drag and drop them into the app.
- The Shapes list gets groups (each DXF, each picture, drawn rectangles, circles, polylines)
  for easy selection, and groups can be hidden.
- A DXF's placement and a picture's tracing are edited from that list, which may get a new
  name.
- The simulation can hide the toolpath.

## 1. Today

- **One DXF:** a project holds at most one DXF (`drawing.source`), with one placement
  (move, 90° turns, scale). Its shapes are `path` shapes with a DXF handle. *Update drawing*
  replaces them, matching shapes by handle.
- **Placement is edited under Job settings,** not where the shapes are.
- **Pictures** can already be several (`drawing.images`), each with X, Y and a width.
  They are edited from Job settings (*Edit…*).
- **The Shapes panel** shows the selection, the selected shape's editor, and a flat list
  of layers with *Select*.
- **Dropping a file:** a DXF starts a new project (after the unsaved-changes question), a
  `.chipload` opens, and anything else is refused.
- **Nothing can be hidden** except the pictures (*View > Show images*).

## 2. Several DXFs (schema 21)

**Model:**
- `drawing.source` becomes `drawing.dxfs`: a list of DXFs.
- Each DXF has what `source` has now (file, SHA-256, units, what was imported, the import
  report, the placement), plus an `id` and a `name`. The name defaults to the file's name
  without its extension.
- Each imported shape names its DXF (`"dxf": id`), as traced shapes name their image.
  DXF handles are only unique within one file, so *update* matches shapes by (DXF, handle).

**Migration v20→v21:**
- A project's `source`, if any, becomes DXF 1, and every shape with a DXF handle gets
  `"dxf": 1`.
- Nothing moves and no id changes, so the goldens don't change.

**Per DXF:**
- **Place:** move, turn and scale apply to that DXF's shapes only. The placement already
  is per drawing; it just moves into each DXF.
- **Update from a newer file:** as now, for that DXF only.
- **Remove:** removes the DXF and its shapes. Operations that selected them get problems,
  as when an image is removed.

**Where a new DXF goes:** at its own origin, as the first one does now. The stock-fit
note says when it reaches beyond the stock. It is then moved in the Drawing panel (§4).
- **Considered:** placing it beside what is already there automatically. That's a guess
  about intent, and it's out of scope (nesting, ADR 0017).

**Layers with the same name in two DXFs** (both have "0", or "Cut"): an operation that
selects layer "Cut" would silently cut both parts.
- **Fix:** a selector gets an optional `dxf` (or `image`), which limits it to that source's
  shapes: `{"layer": "Cut", "dxf": 2}`.
- **In the app,** *Select* on a layer inside a DXF's group fills it in.
- **Older selectors** (no `dxf`) still mean every source, so nothing changes for projects
  with one DXF.
- **A problem (warning)** appears when a layer-only selector matches shapes in more than
  one source: "layer "Cut" is in part-a.dxf and part-b.dxf: limit the operation to one of
  them".

**Pictures** keep X, Y and width. They also get a scale, shown as the width; no new
field, because a picture has no natural size (image-trace.md decision 7).

**Command line (rule 5, before the app):**
- `chipload dxf add PROJECT FILE [--units mm|inch] [--name N]`
- `chipload dxf list PROJECT`
- `chipload dxf place PROJECT ID [--dx --dy --rotate --scale --align …]`
- `chipload dxf update PROJECT ID --from FILE`
- `chipload dxf remove PROJECT ID`
- **The existing commands** `import`, `update` and `place` keep working:
  - with one DXF, as now;
  - with several, `update` and `place` need `--dxf ID`.

## 3. Dropping files

- **With a project open:**
  - A dropped DXF is **added** to the project (§2). It asks for units if the file has none.
  - A dropped picture opens the **image window** to import it.
  - Several files at once are added one after the other. Pictures open their windows in
    turn.
- **With no project open:** a DXF starts a new project, as now. A picture starts a new
  project with its image window, as *Import image…* does.
- **A `.chipload`** opens, as now (with the unsaved-changes question).
- **The hint while dragging** says which of these will happen.

## 4. The Drawing panel (was Shapes)

The panel is renamed **Drawing**: it holds the sources as well as the shapes.

It's a tree of groups:

```
Drawing
 ▾ 👁 part-a.dxf            12 shapes   [Select] [Place…] [Update…] [Remove]
     👁 Cut                  8 shapes   [Select]
     👁 Engrave              4 shapes   [Select]
 ▸ 👁 part-b.dxf             5 shapes   …
 ▾ 👁 logo.png (image 1)     4 shapes   [Select] [Edit…] [Remove]
 ▸ 👁 Drawn: rectangles      2
 ▸ 👁 Drawn: circles         3
 ▸ 👁 Drawn: polylines       1
 Selection: 3 shapes on Cut …   (the selected shape's editor, Copy as selector, as now)
```

**Groups:**
- each DXF, with its layers inside;
- each image (its layer);
- the drawn shapes by kind: rectangles, circles, polylines;
- shapes made with `shape add` or copy/paste are drawn shapes too.

**Row actions:**
- **Select:** selects the whole group, or a layer of it (§2: the operation then names the
  source).
- **Place… (a DXF):** opens the placement (move, turns, scale, align) that is under Job
  settings today. It moves out of Job settings into the panel.
- **Update… / Remove** for a DXF.
- **Edit… / Remove** for an image, moved from Job settings.

**Hiding (👁):**
- A hidden group or layer isn't drawn on the canvas and can't be clicked or box-selected.
- **It is still cut:** hiding is for seeing, not for machining. An operation's own toggle
  turns cutting off.
- **The operation card** says when the operation selects hidden shapes ("selects 4 hidden
  shapes"), so nothing is cut out of sight by surprise.
- **Hiding is saved in the project** (`drawing.hidden`: a list of group and layer keys), so
  it stays when the project is reopened. It is an optional field, and old files read as
  nothing hidden.
- **Per shape:** no. Groups and layers are what you'd hide: a part, its engraving layer,
  the construction circles.

**Job settings** keeps the stock, machine and tools. Its *Drawing* and *Image* rows move to
the panel.

## 5. The simulation: hide the toolpath

- **A toggle:** *View > Show toolpath*, and a checkbox on the canvas legend. It hides the
  toolpath lines over the simulated stock, in the top view and in 3D.
- **The playback marker** (the tool's position) stays visible.
- **Without a simulation,** the same toggle hides the toolpath on the drawing.
- **It is an app setting** (remembered, like *Show images*), not saved in the project.

## 6. Order of work

1. **Schema 21:** `dxfs`, `dxf` on shapes, `dxf`/`image` on selectors, `hidden`, and the
   migration. The `dxf …` commands, with `update` and `place` per DXF, and the problem for
   a layer in several sources.
2. **Dropping files** (§3).
3. **The Drawing panel:**
   - the groups with *Select*;
   - placement, update and remove for DXFs, and image editing moved into it;
   - hiding.
4. **Hide the toolpath** in the simulation (§5). This is small and independent, so it can
   come first.

Each step comes with its tests and guide pages, as usual.

## 7. Edge cases

- **The same DXF file added twice** (two copies of a part): allowed. They are two sources,
  each placed on its own. The *copies* grid stays the way to repeat a whole job.
- **An operation selecting shapes of a removed DXF:** problems, as for removed shapes
  today.
- **Undo:** adding, placing, updating and removing a DXF are each one undo step.
- **Hidden shapes selected by an operation:** see §4. They are still drawn when the
  toolpath is shown, so the cut is never invisible.
- **Box selection** ignores hidden groups.
- **The setup sheet** lists every DXF and image with its placement.

## 8. Tests

- **Migration:** v20 samples (with and without a DXF) migrate to v21 and give the same
  G-code. Every existing golden is unchanged.
- **Two DXFs:**
  - two DXFs with a clashing layer: the problem, then the `dxf` selector cuts only one;
  - updating one leaves the other's shapes and ids alone;
  - removing one gives problems only for the operations that used it.
- **A new golden:** two DXFs placed apart, one scaled, plus an image.
- **The app:**
  - drop handling for each case of §3;
  - the panel's groups and *Select*;
  - hiding: not drawn, not clickable, still cut, and the operation card's note;
  - the toolpath toggle.
- **Guide:** a new page *Drawings and pictures*, combining placement, update and the
  panel, plus screenshots of the panel.

## 9. Decisions for review

1. **Several DXFs as separate sources, each with its own placement** (schema 21).
   **Recommended.** The alternative, merging every DXF into one drawing with one placement,
   is simpler but can't place parts separately, which is the point of the request.
2. **A new DXF goes at its own origin,** to be moved in the panel. **Recommended:** no
   guessing. Alternative: put it to the right of what is there.
3. **Selectors may name a source** (`dxf`, `image`), and a layer-only selector that
   matches several sources is a warning. **Recommended:** otherwise adding a second part
   can silently change what an existing operation cuts. Alternatives:
   - layers stay global, with no warning;
   - each DXF's layers are renamed on import ("part-b/Cut"). That changes names users
     know, and the matching on update gets harder.
4. **Dropping onto an open project adds** DXFs and pictures. Only a `.chipload` replaces
   the project. With no project open, a dropped file starts one. **Recommended.**
   Alternative: ask each time (add, or new project).
5. **Rename Shapes → Drawing,** a tree of sources and drawn-shape kinds, with layers
   inside DXFs. **Recommended.** Alternatives: "Geometry", or "Layers".
6. **Placement and image editing move from Job settings into the Drawing panel.**
   **Recommended:** edit them where they are. Job settings keeps stock, machine and tools.
7. **Hiding is for seeing only:** hidden shapes are still cut, the operation card says so,
   and the toolpath still shows. It is saved in the project, for groups and layers, not
   single shapes. **Recommended.** Alternative: hidden means not cut. That's surprising,
   and it duplicates the operation's own on/off.
8. **Hide the toolpath** as an app setting, in the View menu and on the legend.
   **Recommended.**
9. **Pictures keep their width as their size** (no separate scale field). **Recommended**
   (image-trace.md decision 7).
10. **Order:** §6, starting with the toolpath toggle, since it is small. **Recommended.**

## 10. As built

- **§5, hide the toolpath** (2026-10-13): *View > Show toolpath* and a checkbox at the top of
  the open legend. It hides the toolpath, its plunges, tabs and ring starts (which can't be
  hovered or dragged while hidden); playback's tool still shows. Like *Show images*, it is
  set for the session, not remembered: the app has no remembered view settings yet.
- **§2, several DXFs** (2026-10-13, schema 21):
  - **Commands:** `chipload dxf add|list|remove`, and `--dxf ID` on the existing `place` and
    `update`, instead of also `dxf place` and `dxf update`, which would have repeated them.
    `update` on a project with no DXF adds one.
  - **Selectors:** a segment knows where its shape came from (`chipload_core::chain::ShapeFrom`),
    so a selector's `dxf` or `image` is matched in planning, and its messages say "layer
    'HOLES' of DXF 2".
  - **The note** for a layer in several sources is a planning note (the report, `run`, the
    setup sheet), not a project problem: cutting all of them may be what was meant.
  - **Shape order:** an updated DXF's shapes take its old shapes' place in the list, so
    the cut order doesn't change. Two DXFs on top of each other change each other's
    nesting (a hole inside the other part's outline), and so the order: the test places
    them apart, as parts are.
  - **The setup sheet** lists the DXFs and images with their placements when there are two
    or more; single-drawing sheets are unchanged.
  - **Found on the way:** a tool or operation name with a character outside ASCII ("60°
    V-bit") in a program split per tool stopped the G-code from being written. Fixed on its
    own: G-code comments are made ASCII ("60deg V-bit", "Lador").
- **§3, dropping files** (2026-10-13): dropped DXFs and pictures go into a queue that is
  brought in one at a time: a DXF without units asks, a picture opens its window, and the
  rest wait. *File > Add DXF to project…* (and *from path…*) do the same for one DXF;
  *Update drawing* is off with several DXFs until the Drawing panel picks which.
- **§4, the Drawing panel** (2026-10-13):
  - **Row actions:** each row has *Select*, and a **⋯** menu with the rest (*Place…*,
    *Update from a DXF…* / *from a path…*, *Remove*; *Edit…* / *Remove* for an image). Four
    buttons don't fit the sidebar.
  - **Groups:** DXF groups open on their layers. *Select* on a layer under a DXF is a layer
    selection that names its source, and so does an imported image's layer (the
    operation's selector gets `dxf` or `image`).
  - **Hidden** is a contour whose every shape is hidden: it is left out of the canvas's
    scene, so it is neither drawn nor picked. There is no box selection in the app, so §7's
    line about it has nothing to do. The operation card says "cuts N hidden contours".
  - **Job settings** keep one line about the drawing (how many shapes, DXFs and images),
    pointing at the panel.
- **Selecting a source as a whole** (2026-10-13, after the user asked whether depending on
  layers is practical, prompted by the clash note): a selector may name only a DXF or an
  image (`{"dxf": 2}`, `{"image": 1}`); a layer narrows within it. The panel's group *Select*
  and an imported image give that; right-click *Select layer* gives the layer of the picked
  contour's source. **Adding a DXF pins** an operation that selected a layer of one DXF by
  name alone to that DXF, when the new DXF has the layer too, so adding a part never
  changes an existing operation. The clash note remains only for selectors written by hand
  or before. Images are selected as a whole, not by their layer.
