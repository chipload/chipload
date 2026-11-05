# Spec: two-sided work, turning the board over

Status: **draft 2026-11-04, for review.** Decisions 1, 3, 6, 8 and 13 are the user's
(2026-11-04), and 5 (2026-11-05): the back drawn as seen from the front, dowel pins with a corner as the
fallback, sides that meet give at most a note, and the view stays the front's while
generation mirrors, and the back is planned with the cutting direction flipped rather than
with mirrored geometry. The way setups link (§3) follows the user's question "is it in the
properties panel for setup 2 we say that it is the backside of setup 1?". Follows
docs/specs/setups.md §9.

## 1. Why

Parts machined on both faces: a box side with a groove inside and a chamfer outside, a sign
with a pocket for a hanger on its back, a cutting board with juice grooves on one face and
feet recesses on the other. Today that means two projects, with the drawing mirrored by hand
and the zero worked out on paper, which is where two-sided work goes wrong. The two sides
must line up to a fraction of a millimetre, and the operator must turn the board the
right way.

A **back setup** is a setup that is the same board as another setup, turned over. Chipload
mirrors its cuts, keeps the board and zero in step with the front, and tells the operator
how to turn the board.

## 2. How it works, for the user

1. Draw everything in one drawing, **as seen from the front**. Back features go on their
   own layers, where they lie when you look through the board from the front (an x-ray
   view). Front and back then line up because they are the same coordinates.
2. Right-click the front setup and choose **Add the back side**. That makes "Setup 1 back",
   linked to setup 1, with one empty part for the back's operations.
3. Click **Add dowel holes** in the back setup's form. That adds two holes at the start of
   setup 1, on the line the board turns about.
4. Add the back's operations to the back setup as usual, picking the back layers in the
   front view.
5. Run setup 1: the front's cuts and the dowel holes, through the board into the
   spoilboard.
6. Turn the board over as the setup sheet says, put a pin through each hole into the
   spoilboard, and run setup 2.

## 3. The link (the setup's form)

A setup's form gets a **Board** choice:

- *its own board*, as today;
- *back of: Setup N ▾*, offered for every other setup that is not itself a back and has no
  back yet.

With *back of*, one more row, **Turn over**:

- *left to right*: about the board's vertical centre line, so X is mirrored;
- *top to bottom*: about its horizontal centre line, so Y is mirrored.

The quick way is **Add the back side** on the right-click menu of a setup without a back:
a new setup named "<name> back", linked, with *left to right*, one empty part, and made
active.

What the link does:

- **The board is shared, not copied.** The back setup has no stock of its own. Its Stock
  item shows the front's board read-only ("12 mm, Setup 1's board"), and a click on it opens
  the front's Stock. Change the thickness in setup 1 and the back follows.
- **The work zero follows the front's, mirrored** (§5). It can't be set separately: a zero
  of its own would let the sides disagree.
- **In the Browser** the back setup sits right after its front, named with its turn:
  "Setup 1 back  ⟲ left to right". Its parts and operations are listed and edited as usual.

Rules:

- **One back per front.** A second setup can't be *back of* the same setup.
- **No back of a back.** The back of the back is the front again.
- **Removing the front** unlinks its back first: the back gets a copy of the board and the
  zero as they were, with a note in the Report. Choosing *its own board* in the back's form
  does the same.
- **Turning a setup off** works as today. The link stays.

## 4. Mirroring (decision 8)

The view stays the front's. Shapes are drawn, picked and edited in front coordinates, and
the back's toolpath preview and markers are shown there too, lying on the shapes they cut,
as if seen through the board.

**A back setup is planned in front coordinates with the cutting sense flipped, and the
finished program is mirrored** (the user, 2026-11-05).

- **Why it works:** mirroring a path reverses its sense of rotation, but the spindle keeps
  turning the same way. A path planned as conventional in front coordinates, once mirrored,
  cuts climb on the back face. Seen from the drawing, the back is cut as if the spindle
  turned the other way.
- **Planning:** the planner takes a **cutting sense**, the spindle's turn as seen in the
  drawing's coordinates. It is the machine's spindle for a front setup and the opposite for
  a back. Every choice of travel direction already goes through one function,
  `cut_clockwise(side, direction, spindle)`: profiles, pocket walls, islands and links,
  follow-path rings, helical bores. They all take the cutting sense, never
  `machine.spindle` directly. The G-code's M3 or M4 still comes from the machine's
  spindle, so planning and output can't be confused.
- **Then the program is mirrored** about the turn axis before the post: X (left to right)
  or Y (top to bottom) negated about the board's centre line, and G2 and G3 swapped, next
  to the shift into work coordinates. The checker judges the mirrored G-code as it judges
  any program.
- **The preview** is the planned toolpath itself, in front coordinates, lying on the shapes
  it cuts. Nothing needs mirroring back for the view.

So the operation's settings mean what they say on the face being cut:

- **inside stays inside,** **outside stays outside;**
- **climb stays climb,** **conventional stays conventional;**
- **a ring's start point,** tabs placed by hand and dogbone corners stay on their feature
  without being moved, because they are stored in front coordinates and planning stays
  there;
- **tabs along X or Y** keep their axis.

Choices made in the front frame, like a ring starting at its lowest point or the order of
the cuts, come out mirrored on the back: the highest point, the far side first. That's
harmless: the path is as valid, and the checker sees the real G-code.

Mirroring the finished toolpath *without* flipping the cutting sense would turn every climb
cut into a conventional one without a word. This is the part of the feature that can break
tools, so it gets explicit tests. For each operation kind with a direction (profile, pocket,
follow path, helical bore), climb and conventional, a back setup's G-code is compared move
by move with a front setup cutting the same shapes drawn mirrored by hand. A test also
checks that the planner reads no `machine.spindle` outside the cutting sense.

**Text and pictures.** A text drawn reading correctly in the front view reads mirrored on
the back. When a back setup's operation carves a text that isn't mirrored, the Report says
so as a note: "Operation 5 (Name) on the back carves text that reads right from the front,
so it reads mirrored on the back face. Mirror the text if it should read right there." It
is a note, not a warning: a mirrored text can be intended (a stamp, a mould). See
decision 9.

**The simulation** shows both sides of the one board (§10).

## 5. The zero and the depths

The turn axis is always the **board's centre line**, so the board maps onto itself and the
checker's stock is the same rectangle for both sides.

- **XY zero:** the front's zero mirrored about the axis.
  - A stock corner becomes the corner across the axis: bottom-left becomes bottom-right
    when turned left to right, top-left when turned top to bottom. The board's centre stays
    the centre.
  - The drawing origin and a point are mirrored as points.
  - The sheet says where to zero on the turned board.
- **The recommended zero for two-sided work is a dowel hole:** the same physical pin for
  both sides. *Add dowel holes* offers to set the front's zero to the first pin's centre
  (a point zero), and the back's follows: the pin is on the axis, so it mirrors onto itself.
- **Z zero:** the front's choice, on the face now up. *Stock top* is the back face, and the
  back's depths count from it. *Stock bottom* is the spoilboard, as before.
- **Thickness after surfacing:** a surface operation in the front setup leaves the board
  thinner. The back setup's thickness is the board's minus the front's deepest surfacing.
  Its Stock row shows both ("12 mm board, 11.5 mm after surfacing"), and the checker and
  the `through` depths use the thinner one. Otherwise a through cut on the back would go
  further into the spoilboard than the checker believes.

## 6. Dowel holes (decision 3)

**Add dowel holes**, in the back setup's form, or `setup dowels` on the command line:

- draws two circles of the pin diameter as drawn shapes, on a layer "dowels" (made if
  missing), on the turn axis, one at each end, centred in the board's margin beyond the
  parts' outline;
- adds a holes operation "Dowel holes" first in the front setup's first part. It cuts
  through the board and to the **pin depth** into the spoilboard, drilled with a drill of
  the pin diameter, or bored with the end mill when there is none (as holes choose today);
- the pin diameter defaults to 6 mm, and the pin depth to 8 mm below the stock.

**The pin depth is allowed only inside the dowel circles.** Every other cut stays within the
setup's *max depth below stock*. The checker gets the dowel circles from the project, as it
gets the stock, never from the toolpath, so it stays independent. A move deeper than the
max depth below stock is refused unless it lies inside a dowel circle (grown by the
position tolerance), and there it may go to the pin depth.

If the margin beyond the parts' outline on the axis is narrower than the pin diameter plus
2 mm, the button says so and offers to widen the board's fit margin.

The circles and the operation are ordinary: they can be moved along the axis, resized or
removed. Only their role is marked (the operation's `dowels: true`), so the setup sheet can
name them and the checker can allow their depth. A dowel circle moved off the axis is a
problem: on the turned board it would no longer meet its hole.

**The fallback without pins: a corner.** A back setup's form has **Register by: pins ▾ /
a corner**. With *a corner*, no holes are needed. The sheet says to push the turned board
into the same corner of the fence, and warns that the board's real size must match the
stock's, because any error moves the back by up to twice that error.

## 7. Sides that meet (decision 6)

Where a front cut and a back cut overlap in XY and their depths together reach the board's
thickness, the cut goes through. That can be by design: a window, a slot cut from both
sides, a hole made by two pockets. So it is **at most a note** in the Report, never a
warning or an error: "Operations 2 (front) and 6 (back) meet: together they cut through the
board." It's worked out from the operations' outlines and depths, not from the simulation.

## 8. The setup sheet

The back setup's sheet starts with how to turn the board, in words and a small top-view
picture of the board with the axis, the pins and an arrow:

- "Turn the board over left to right, about the line through the pins: the left edge goes
  to the right. The face that was down is now up."
- With pins: "Put a 6 mm pin through each hole into the spoilboard."
- With a corner: "Push the board into the same corner of the fence as for setup 1. Its
  size must be exactly 300 × 200 mm."
- "Zero: X Y at pin 1, Z at the top of the back face."

The front's sheet says after its last operation: "Leave the board in place until the dowel
holes are drilled. Setup 2 cuts its back."

## 9. Command line and file

- `chipload setup add part.chipload --back-of 1 [--turn left-right|top-bottom]`
- `chipload setup set part.chipload 2 --back-of 1 --turn top-bottom`, or `--own-board` to
  unlink
- `chipload setup dowels part.chipload --setup 2 [--diameter 6] [--depth 8] [--zero]`
  (`--zero` sets the front's zero to pin 1)
- `chipload setup set part.chipload 2 --register corner`
- `setup list` shows the link: "2  Setup 1 back  back of 1, left to right, pins".

**Schema 36.** A setup is either its own board or a back:

```json
{ "id": 2, "name": "Setup 1 back", "enabled": true,
  "back_of": { "setup": 1, "turn": "left_right", "register": "pins" } }
```

A back setup has `back_of` and no `stock` or `work_zero`. A holes operation may have
`dowels: true` and a `pin_depth`. The field is optional, but the version **is bumped**: an
older Chipload reading a back setup would cut it unmirrored, which ruins the part. The
migration 35 → 36 changes nothing, since no older file has a back. There's a frozen v35
sample and the usual golden test.

Validation:

- `back_of.setup` names another setup that is not itself a back;
- no two backs name the same setup;
- a back has no stock and no zero;
- dowel circles lie on the axis (within the chaining tolerance).

## 10. The simulation, both sides (the user, 2026-11-05: "visualization is important")

The point of simulating two-sided work is to see the finished part: the front's cuts and
the back's meeting in one board, before any wood is cut.

### 10.1 The model: a floor beside each height

Today the simulation is a height map: one number per cell, the top of the material, which
cuts only ever lower. Material cut from below has nowhere to go.

With 2.5D cuts from the top and from the bottom, a cell's material is always **one
unbroken column**. The front removes from the top down, the back from the bottom up, and
neither can leave a void in the middle. So two numbers per cell hold it exactly, with no
voxels:

- `heights`, as today: the top of the material, program Z of the front;
- `floors`, new: the bottom of the material, in the same Z. It starts at the stock bottom
  everywhere;
- a cell where the floor reaches the height is **cut through**: empty.

**The back's moves in front coordinates.** The back is planned in front coordinates (§4),
so its toolpath needs no un-mirroring for the simulation. Only its Z is turned over. With
the board's front top at `top` and its bottom at `bottom`, a back move at depth `d` below
the back face reaches up to `bottom + d`. A move raises the floor of every cell under the
tool to the tool's highest point there. The tool's shape (flat end, V-bit cone, drill
point) is the same as for the front, pointing up.

**Surfacing:** the front's surfacing lowers the height; the back's thickness follows
(§5). The floor isn't touched.

**Order:**
- Simulating the **front setup** is as today. Its floors stay at the stock bottom, unless
  through cuts reach it: then those cells are cut through.
- Simulating the **back setup** starts from the front's final map, heights and floors, and
  runs the back's moves on it. The front is simulated first, without playback, and its
  map is cached until the front changes.
- The **playback** of the back shows the back's moves only, on top of the front's result.
  Its history gets a side per entry: a height lowered or a floor raised.

**Numbers:** removed volume counts both sides. *Deepest* stays the front's. A new *through*
count gives the area cut through, in cm². *Material left* (rest of pockets) works on each
side's own surface.

Memory: one more `f32` per cell, at most 250 000 cells, so 1 MB more.

### 10.2 Drawing it

Today the 3D view meshes only the top surface, a sheet of quads from cell centre to cell
centre, drawn back to front (painter's order) in an egui mesh. A two-sided board needs a
solid:

- **the top surface** over cells with material, as today;
- **the bottom surface** from the floors, seen when looking from below;
- **walls** where a cell with material meets a cut-through cell or the board's edge, from
  its floor to its height. Today's sheet has no edge walls, so this also gives a one-sided
  board its edges;
- **cut-through cells left out**, so a hole is a hole and the spoilboard shows through.

The drawing stays an egui mesh in painter's order, because there is no depth buffer. The
app renders through glow and the screenshot tests through wgpu, and a mesh works in both;
a custom paint callback with a depth buffer would need writing twice. On a grid of columns
seen through an orthographic camera, drawing cells far to near is correct, as in tile-based
isometric drawing. Within each cell, the faces turned away go first: the bottom when
looking from above, the top from below. The quads are drawn as blocks per cell, not from
centre to centre, so walls meet them cleanly. The surface keeps its smooth shading from
the heights. If the painter's order shows faults (thin walls between deep pockets), a depth
buffer is the fallback, and a decision for then.

**Looking from below:** the tilt today stops just short of side-on. Two ways to see the
back:

- **Turn the board over**, a button by the view's orientation controls, shown when the
  project has a back setup. It turns the 3D view about the setup's turn axis, so you see
  the back face up, as the operator will after turning it. Press it again to see the front.
  It is the same board, seen from the other side, with everything drawn on it: shapes,
  toolpath and markers, mirrored with the view.
- **The tilt goes past side-on**, down to straight up, so a drag can look underneath.

**Recommended: both.** The button is the quick, exact view. The free tilt is for looking at
where the sides meet. Activating the back setup turns the board over once, so its
toolpath is seen on the face it cuts. Activating the front turns it back.

**The top view** (2D, a shaded picture) shows the face up: the front, or the back after
*Turn the board over*. Cut-through cells are shown as holes, in the background colour, on
either face.

**The command line:** `chipload simulate part.chipload --setup 2 -o sim.png` simulates
through setup 2, front first. `--face back` draws the picture of the back face; the
default is the face that setup cuts. The numbers in `--json` get `through_cm2`.

### 10.3 What is checked

- A back cut reaching up to meet a front pocket gives a through cell exactly where the two
  overlap. This is tested: a 12 mm board, a 7 mm front pocket and a 7 mm back pocket
  overlapping by 20 mm, so the through area is the overlap.
- A back setup simulated alone, without its front, is the old behaviour turned over:
  floors raised, heights untouched.
- A one-sided project gives the same pictures and numbers as today, except for the new
  edge walls, which are shown in the renewed screenshots.

## 11. Not in this step

- **More than two sides,** for example edges machined standing up. That needs a fourth
  axis or a vise setup, and is out of scope (ADR 0017).
- **Pins on another line than the centre line.** It would let the pins sit where the board
  has room, but the board would then no longer map onto itself.

## 12. Decisions

1. **The back is drawn as seen from the front (x-ray), and Chipload mirrors it.** *The user,
   2026-11-04: as recommended.* Alternative: draw the back mirrored by hand, which is how
   the sides come to disagree.
2. **A back setup is linked in its own form, *Board: back of Setup N*, with a quick *Add the
   back side* on the front's menu; the board is shared, the zero follows.** **Recommended**
   (§3). Alternative: a "two-sided" switch on one setup holding both sides' parts: fewer
   items, but a setup would then be two programs, against setups.md decision 3.
3. **Dowel pins on the turn axis, a corner as the fallback.** *The user, 2026-11-04: as
   recommended.* Pins on the axis map onto themselves, so the stock's real size doesn't
   matter.
4. **The turn axis is always the board's centre line.** **Recommended:** the board maps
   onto itself, so the checker, the fit and the corners need nothing new. Alternative: any
   line (§11).
5. **A back setup is planned in front coordinates with the cutting sense flipped, then its
   program is mirrored, so climb and conventional keep their meaning on the back face.**
   *The user, 2026-11-05: "invert the cut direction for mirrored operation … then we don't
   need to modify the geometry."* (§4) Better than the first draft's mirroring the contours
   before planning: hand-placed tabs, starts and corners stay put, and the preview needs no
   mirror. Rejected: mirroring the toolpath without the flip, which silently turns climb
   into conventional.
6. **Sides that meet are at most a note.** *The user, 2026-11-04: cutting pockets that meet
   can be by design.* (§7)
7. **The pin depth is allowed only inside the dowel circles, and the checker learns them
   from the project.** **Recommended** (§6). Alternative: raise the setup's max depth below
   stock to the pin depth. That's simpler, but then every cut could go 8 mm into the
   spoilboard unchecked.
8. **The view stays the front's; the toolpath is mirrored on generation.** *The user,
   2026-11-04: "that is how the geometry is drawn. We see the result in the simulation."*
   (§4)
9. **Text on the back that isn't mirrored gives a note, and nothing is mirrored for the
   user.** **Recommended:** a mirrored text may be meant. Alternative: mirror texts on back
   layers automatically. That's convenient, but it breaks decision 1's rule that the back
   is drawn as it lies.
10. **The back's thickness is the board's minus the front's surfacing.** **Recommended**
    (§5). Alternative: a note asking the user to set it: one more number to get wrong.
11. **Schema 36, with a bump although the field is optional.** **Recommended** (§9): old
    code must refuse a back setup rather than cut it unmirrored.
12. **Order of work,** each step a commit:
    1. the model, schema 36, validation, `setup add --back-of`, `setup set`;
    2. planning: the cutting sense, the mirrored program, zero, thickness after surfacing,
       with the climb/conventional tests of §4 and a two-sided golden (a sign with a back
       pocket);
    3. dowel holes: `setup dowels`, the checker's allowance inside the circles, the
       problems;
    4. the notes (text, sides that meet) and the setup sheet's turning words and picture;
    5. the window: the Board and Turn over rows, *Add the back side*, *Add dowel holes*, the
       read-only Stock;
    6. the simulation's floors, the back run on the front's map, through cells, the
       numbers, `simulate --face` (§10.1);
    7. the solid in the 3D view: walls, bottom surface, holes, *Turn the board over* and
       the tilt past side-on (§10.2);
    8. the guide page, translations, schemas.
    **Recommended.** Built on a local branch and not pushed until the user has tried it and
    likes it (the user, 2026-11-05: "an experiment … we must not push this before we like
    it").
13. **The simulation shows both sides in one board,** with a floor per cell beside each
    height (§10.1). *The user, 2026-11-05: in the first version.* Alternative: simulate
    each side alone, which can't show where they meet.
14. **The 3D view draws a solid in painter's order in an egui mesh,** with walls, a bottom
    surface and holes (§10.2). **Recommended:** it works in the glow app and in the wgpu
    screenshot tests alike. Alternative: a depth buffer in a custom paint callback, kept as
    the fallback if the order shows faults.
15. **Looking at the back: a *Turn the board over* button and a tilt past side-on, both,**
    and activating the back setup turns the board over (§10.2). **Recommended.**
    Alternative: only one of them.
