# Spec: a relief made lightly: turning and cutting a model live, the Relief wizard

Status: **draft 2026-11-13**, for review. It builds on docs/specs/stl-relief.md (the model,
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

It opens from the model's properties (*Turn and cut…*), as step 1 of the Relief wizard
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

On the ribbon's WIZARDS group and in *Wizards > Relief…*. Built like *New sign…*: it makes
ordinary things, and afterwards they are edited as any others.

1. **The model:** an STL file to pick, or one already in the project; then the choosing
   window of §4, inside the wizard.
2. **Tools:** the finishing tool (ball nose or tapered ball nose, from the project and the
   library, as Inlay… offers them) and, ticked by default when the depth is more than the
   finishing tool's diameter, a roughing end mill with its allowance (default 0.5 mm).
3. **Round it:** one of
   - **nothing:** the relief alone, within its outline;
   - **cut out:** a profile outside the outline, through the board, with tabs, after the
     relief;
   - **background lowered:** a pocket from a rectangle *margin* mm (default 10) round the
     model's box down to the relief's floor, with the outline as its island, before the
     relief; the board fitted round it when the stock follows the drawing.

   A drawing of the board shows the result as the options change, as the two-sided
   wizard's does.

It makes, as one undo step and in a part of its own: the model, its relief with the
roughing pass, and the pocket or profile; the operations in cutting order.

Command line: `relief add PROJECT FILE.stl --width W --depth D --keep K --up U
--tool T [--rough-tool R] [--around none|cut-out|background] [--margin M]`, the same
choices as the wizard (rule 5: the command line first).

## 7. Inlay to WIZARDS

*Inlay…* already asks questions and makes a pocket, a plug's setup and floor passes in
one step: it is a wizard. It leaves OPERATION (and the *Operation* menu) for WIZARDS (and
the *Wizards* menu). One place, not two (the "no worse options" rule).

On the ribbon, WIZARDS then holds Sign, Two-sided, Inlay and Relief. To keep the names at
1280 px, Sign and Two-sided stay large and Inlay and Relief are small buttons one above
the other, as Surface and Style are now (decision 6). The layout test guards it.

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
5. §6: the Relief wizard, command line first; §7 Inlay moved. Guide in each step.

## 10. Decisions

1. **Turning:** quarter turns now (six faces up, four ways on the board), which *Up* and
   *Rotation* already hold; a free tilt later. *Decided 2026-11-13.*
2. **The back:** a cut level from the top, as a share of the height; a slanted cut only
   when a model needs it. *Decided 2026-11-13.*
3. **Order:** turn, then cut, then size; width and depth are the kept part's, and a typed
   depth stays when the cut changes. *Recommended.*
4. **Where the choosing window opens:** the model's properties, the wizard's step 1, and
   by itself on adding an STL (drop or ribbon button). *Recommended.*
5. **Drawing:** egui's painter with CPU shading first. *Recommended.*
6. **WIZARDS on the ribbon:** Sign and Two-sided large, Inlay and Relief small.
   *Recommended;* the alternative is all four small, which loses every name in the group.
7. **The cut level as a share** (0.5 = the top half), not in mm: it keeps its meaning when
   the model is scaled. The window shows the mm as well. *Recommended.*
8. **The light copy's limit:** 50 000 triangles, a constant, no setting. *Recommended.*
