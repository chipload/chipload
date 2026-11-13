# Spec: a relief made lightly: turning and cutting a model live, the Relief wizard

Status: **approved 2026-11-13** ("Go ahead": decisions as recommended, 6 (a)); steps 1-3 built. It builds on docs/specs/stl-relief.md (the model,
the relief, the drop cutter), whose rules for planning and checking stay as they are.

The user tried a 57 MB STL from Printables and found it "quite heavy to work with", and
found reliefs hard to discover. Their asks (2026-11-13): a light model to turn in real
time while choosing which side faces up; the back of the model removed, then the rest
scaled; "a very visual workflow" that is "very lightweight and fast"; Relief perhaps a
wizard, and Inlay too. Decided in conversation: quarter turns now and a free tilt later; a
cut level now and a slanted cut only when a model needs one.

## 1. Why

**Heavy.** A model's STL file is kept in the project as base64 text (about 76 MB for a
57 MB file). The window copies the project for every undo step and every background plan,
so every edit copies that text again, and every change of the model reads the STL again
from it: decoding, parsing, turning, footprint. Nothing of that changes what is cut; it is
only slow.

**Blind.** *Up* (six faces) and *Rotation* are chosen in the properties form, and the
model is seen only as its outline and a faint top surface on the board. A model that
lies wrong shows that only after the change is made, and each change is slow.

**The back.** A full-round model (a figure, a head) is carved from its top down to its
lowest point, so its depth goes into the far side, which the tool cannot reach anyway, and
the visible front comes out flat. The usual wish is the front half, scaled to the depth
the board allows.

**Scattered.** A relief takes four separate steps today: the model, the Relief operation,
its roughing pass, and a pocket or profile round it.

## 2. Fast first: no copies, one read (no change in output)

- **Shared data.** The STL's bytes are held once, shared by every copy of the project
  (`Arc`), and decoded from base64 only when the project is loaded or the file added. The
  file format stays the same (base64 in `data`); only memory changes.
- **One read per file.** The read and turned mesh is kept, keyed by the file's SHA-256 and
  *Up*, by whoever asks for it: the window keeps it in its own state, the command line for
  one run. It is passed into the pipeline's functions as a value; no global state.
- **In the background.** Adding a model reads it on a background thread; the window says
  "Reading dome.stl…" and stays usable. Planning already runs in the background.
- Byte-identical G-code: every golden stays as it is.

## 3. The light copy

On reading, Chipload also makes a **light copy** of the mesh for showing it: at most about
50 000 triangles, whatever the file. Method: **vertex clustering** (J. Rossignac and
P. Borrel, "Multi-resolution 3D approximations for rendering complex scenes", 1993): a
grid over the model's box, each cell's corners merged into their mean, triangles whose
corners fall into fewer than three cells dropped. The cell is the smallest that gives at
most the limit. A file smaller than the limit is its own light copy.

The light copy is used **only for seeing**: the choosing window (§4), the 3D view's model
and its live relief preview. The outline on the board, the planned toolpath, its check,
the simulation and the G-code always come from the full model. What is cut is never the
light copy.

It lives in `chipload-mesh` (core only, no I/O), with tests: the limit holds, the box
stays within one cell of the original's, a closed model stays without holes larger than a
cell, the same file gives the same copy.

## 4. The choosing window: *Turn and cut…*

A window with two views side by side and the controls below:

- **Left: the model in 3D**, the light copy shaded, turning with the mouse as the 3D view
  does. Under it the board as a flat plate, so it is plain which side faces up. The cut
  level (§5) is a see-through plane; what lies below it is drawn grey and faint.
- **Right: the relief it gives**, seen from above, shaded as the 3D view shades a model:
  the light copy turned, cut and scaled, its top heights on a grid of about 200 cells along
  the longer side. Redrawn after each change.
- **Controls:**
  - **Face up:** six buttons (the faces, as *Up* today) and two quarter-turn buttons
    (turn on the board by 90°, as *Rotation*).
  - **Cut level:** a slider and a number, from the top down (§5).
  - **Width** and **Depth**, with *Keep proportions*, as in the properties.
  - The relief's size on the board in words: "120 × 84 mm, 6 mm deep".
- **OK** applies it all as one undo step: the model's values change and its outline is
  drawn again from the full model. **Cancel** changes nothing.

Speed targets, on the user's 57 MB file on an ordinary laptop: turning the view at least
30 frames a second; the relief preview redrawn within 0.1 s of a change; the window open
within 1 s once the file is read.

It opens from the model's properties (*Turn and cut…*), as step 2 of the Relief wizard
(§6), and by itself when an STL is dropped on the window or added by the ribbon's
*3D model* button.

Drawn with egui's own painter, the shading worked out on the CPU (decision 5); only if
that is not smooth enough on the user's file does it move to the graphics card.

## 5. The cut level

A new value on the model, `keep`: the share of the turned model's height kept, measured
from its top, more than 0 and at most 1. Optional, 1 when missing (the whole model, as
today), so no schema bump; an older Chipload refuses a file that has it, as with other new
fields.

- **What is kept:** the part of the turned model above the level
  `top − keep × height`. Below it nothing: the triangles are clipped at that plane, not
  dropped whole, so the front half of a head keeps its full outline.
- **Depth:** the depth is the kept part's, from its top to the cut. *Keep proportions*
  makes it from the kept part's proportion; a depth typed stays when the cut changes
  (decision 3).
- **Where the cut part has no surface** (the cut went through the model), the relief goes
  to its floor there, as anywhere the model has no surface.
- **The outline** is the kept part's shadow from above.
- Command line: `model set --keep 0.5` (or `--keep 50%`); `get/set
  drawing.models.0.keep`.

**Slanted cut:** not now. Turned a quarter, any model's cut is level with one of its own
axes; a slanted cut waits for a model that needs it, and comes with the free tilt.

## 6. The Relief wizard: *Relief…*

On the ribbon's WIZARDS group and in *Wizards > Relief…*. Built like *New sign…* and
*Two-sided…*: it makes ordinary things, and afterwards they are edited as any others.

**The order follows the questions a user answers in their head**, each step needing only
what came before it: *what* is carved, *how it lies*, *how big and where*, *what is round
it*, *with what tools*, then a last look. The tools come late on purpose: whether a
roughing pass is needed depends on the depth (step 3), and whether an end mill is needed
at all depends on what is round it (step 4).

A **picture of the board** stays on the right in every step from step 3 on, drawn from
the light copy: the board, the relief's shading, its outline, and from step 4 the pocket
or cut-out. Each step's change shows there at once. *Back* keeps every answer; *Next* is
greyed out, with the reason beside it, until the step is complete.

1. **Model.** An STL file to pick (*Choose file…*, or dropped on the wizard), or a model
   already in the project from a list. Reading a large file shows "Reading…" here, and
   *Next* waits for it.
2. **Turn and cut.** The choosing window of §4, inside the wizard: the model in 3D and
   the relief it gives; *Face up* (six faces), the quarter turns, and the *Cut level*.
   Size isn't asked here: the preview fits its box.
3. **Size and place.** Width and depth with *Keep proportions* (a new relief starts half
   the board wide and at most half its thickness deep, as an added model does now), and
   where it lies: centred on the board by default, or at X, Y. The board's own size and
   thickness are shown, and can be changed here, since they bound both; a depth beyond the
   board's thickness is refused with the reason.
4. **Round it.** One of
   - **nothing:** the relief alone, within its outline;
   - **background lowered:** a pocket from a rectangle *margin* mm (default 10) round
     the model's box down to the relief's floor, with the outline as its island, cut
     before the relief; with rounded corners (*corner radius*, default 0);
   - **cut out:** a profile outside the outline, through the board, with tabs, cut after
     the relief.

   The board grows to hold the margin when it follows the drawing.
5. **Tools.** Only the tools the choices need, each from the project and the library as
   Inlay… offers them:
   - **finishing:** a ball nose or tapered ball nose, always;
   - **roughing:** an end mill, ticked by default when the depth is more than the
     finishing tool's diameter (as the deep-relief warning says), with its allowance
     (default 0.5 mm);
   - **pocket or cut-out:** an end mill, only with step 4's background or cut-out; the
     roughing end mill by default, so one tool change does both.
6. **Check and make.** In words, in cutting order: what will be made, the tools and the
   tool changes, the depth, and the time the plan estimates from the light copy (marked
   as an estimate). *Make* adds, as one undo step and in a part of its own: the model, the
   pocket (if any), its relief with the roughing pass, and the profile (if any).
   Afterwards the relief's operation is selected and the 3D view shows the board.

Command line: `relief add PROJECT FILE.stl --width W [--depth D] [--keep K] [--up U]
[--rotation A] [--at X,Y] --tool T [--rough-tool R] [--allowance A]
[--around none|background|cut-out] [--margin M] [--end-mill E]`: the same answers in the
same order (rule 5: the command line first), so a script and the wizard give the same
project.

## 7. Inlay to WIZARDS, both large

*Inlay…* already asks questions and makes a pocket, a plug's setup and floor passes in
one step: it is a wizard. It leaves OPERATION (and the *Operation* menu) for WIZARDS (and
the *Wizards* menu). One place, not two (the "no worse options" rule).

On the ribbon, WIZARDS holds Sign, Two-sided, Inlay and Relief, all four **large** buttons
with their names (the user, 2026-11-13).

**Room.** Measured 2026-11-13 in English: at 1280 px the ribbon ends about 20 px before
*Generate*; two more large buttons take about 90 px, so about 70 px must come from
elsewhere, or every button drops its name. Inlay leaving OPERATION frees nothing: its small
button shares a column with Surface and Style, which stays. Where the room comes from is
decision 6. The layout test guards it in every language.

## 8. Not now

- A free tilt (any angle about any axis): the model's turn then becomes three angles, with
  a schema bump and a migration from *Up*.
- A slanted cut (§5).
- Drawing the model on the graphics card (§4).
- A relief from a picture (brightness as height, inflated shapes): agreed for after this.

## 9. Order of building

1. §2: shared data, one read, reading in the background. No change in output.
2. §3: the light copy, in `chipload-mesh`, with tests; measured on a large file.
3. §5: the cut level, command line first, golden test of a cut dome.
4. §4: the choosing window.
5. §7: Inlay moved, the ribbon's room (decision 6).
6. §6: the Relief wizard, command line first. Guide in each step.

## 10. Decisions

1. **Turning:** quarter turns now (six faces up, four ways on the board), which *Up* and
   *Rotation* already hold; a free tilt later. *Decided 2026-11-13.*
2. **The back:** a cut level from the top, as a share of the height; a slanted cut only
   when a model needs it. *Decided 2026-11-13.*
3. **Order:** turn, then cut, then size; width and depth are the kept part's, and a typed
   depth stays when the cut changes. *Recommended.*
4. **Where the choosing window opens:** the model's properties, the wizard's step 2, and
   by itself on adding an STL (drop or ribbon button). *Recommended.*
5. **Drawing:** egui's painter with CPU shading first. *Recommended.*
6. **Room on the ribbon for Inlay and Relief, both large** (the user: large icons). About
   70 px are needed at 1280 px. Options:
   - (a) *Measure* moves from the ribbon's VIEW group to the navigation bar under the
     canvas, beside fit, top and 3D, where the other viewing tools are (about 55 px:
     the button and the group's separator); and LIBRARY's *Tools* and *Machines* become
     one *Library* button that opens *This computer* in the Browser (about 50 px).
   - (b) The window opens 1360 wide instead of 1280 (still fits a 1366 px screen; a
     laptop at 150 % scaling is 1280 wide and shows icons alone either way).
   - (c) Accept icons alone at 1280 in English.
   *Recommended: (a);* both moves put things where they belong, and *Library* in the menu
   bar keeps both lists one click away.
7. **The cut level as a share** (0.5 = the top half), not in mm: it keeps its meaning when
   the model is scaled. The window shows the mm as well. *Recommended.*
8. **The light copy's limit:** 50 000 triangles, a constant, no setting. *Recommended.*
9. **The wizard's order:** model, turn and cut, size and place, round it, tools, check and
   make (§6), the board's picture beside every step from size on. *Recommended.*
10. **Where a relief lands:** centred on the board, in a part of its own. *Recommended.*

## 11. Settled while building

- **Step 1 (§2), 2026-11-13.** The model's `data` is a `ModelData`: the base64 text in an
  `Arc`, written to the file as the same string. The read mesh is remembered *on that
  shared value* (`ModelData::remembered`, worked out once for the text), not in a cache
  passed around: every copy of the project that shares the text shares the read, and
  nothing is global. The pipeline's adding is split: `read_model` (read, check, encode,
  parse once) and `add_model` (place it); the window runs `read_model` on a worker thread
  ("Reading head.stl…"), and dropped files wait behind it. Measured on a generated 57 MB
  STL (1.14 million triangles), release build: copying the project 45 ms → 6 µs; a change
  of the model in the window 1.13 s → 0.26 s; its placed mesh 0.49 s → 0.15 s. Still on
  each change: the outline from the full mesh (by design) and the faint 3D surface, which
  step 2 moves to the light copy. Pictures keep their data as plain text: small files.
- **Step 2 (§3), 2026-11-13.** `Mesh::light(most)` in `chipload-mesh`, `LIGHT_TRIANGLES`
  50 000: the grid's cube count along the longest side starts at √(most/2) and is
  corrected by √(most/count) until the copy is within the limit and at least 70 % full
  (at most 8 tries). Lookups by `HashMap`, the output in the triangles' own order, so the
  same file gives the same copy. It is made with the read (§2), on the worker thread, and
  remembered with the full mesh; placed by the full mesh's box, so the two lie alike. On
  the 57 MB file: 47 709 triangles in 0.35 s; the faint 3D surface 210 ms → 10 ms. Used
  for the 3D view's model now, and the choosing window (§4) next.
- **Step 3 (§5), 2026-11-13.** `Mesh::above(level)` in `chipload-mesh` clips each
  triangle against the level (Sutherland–Hodgman against one plane, a fan for the quad);
  the cut corners lie exactly on it. The toolpath's `turned_mesh` cuts the turned model,
  so placement, outline, drop cutter and check all see the kept part; the light copy is
  cut at the same level. `keep` is written always (`serde(default)` 1 for files from
  before it). The model's properties take it as *Keep from top* in % until the choosing
  window comes; the command line as `--keep 0.5` or `50%` on `model add` and `model set`.
  Golden `v36_relief_dome_cut` (the dome's top half, 5 mm deep).
