# Edit and Simulate

Status: agreed 2026-10-30. The user found box selection unavailable after the simulation was
shown, and proposed "a distinct simulation switch that shows the simulation and box
selection is inactive, and a drawing/operation mode where the simulation is hidden"; then
asked whether a change needs a new toolpath at all ("right now it generates if I draw a new
shape, that shouldn't be necessary"); then "your suggestions are good, go ahead and build".

## 1. Why

The canvas did two jobs with one mouse: a drawing board (select, box-select, draw, drag
handles, tabs and starts) and a viewer of the result (the shaded simulation, playback, 3D,
hiding the drawing). They clash: hiding the drawing to see the cut turned a drag into a pan,
silently. And every committed edit made a new toolpath and simulation, also a shape that
nothing cuts.

## 2. Two modes

- **Edit** (projects open in it): the drawing and the toolpath's lines, no shaded
  simulation. Selecting, box selection, the drawing and editing tools, shape and box
  handles, the work zero's handle, tabs and start points, the right-click menu: all as
  before. The drawing is always shown.
- **Simulate:** the shaded simulation, the playback bar, the drawing's and toolpath's
  toggles. A left drag pans in top view and orbits in 3D; it never selects or moves
  anything. A click on an operation's number on the toolpath opens it; nothing else on the
  canvas reacts to a click. Measuring works.
- **The switch:** **Edit | Simulate**, first in the view bar: two halves in an accent
  outline, the mode's half filled; View > *Simulate*; and **F6** (beside F5, Generate). In
  Simulate an accent frame runs round the canvas. The canvas's hint line says which mode it
  is (decision 10).
- **Properties stay editable in both** (decision 4): in Simulate a changed depth or step
  gives a new toolpath and simulation, as now.
- **Back to Edit by itself** when a drawing or editing tool starts, text is placed, or
  something is selected in the Browser (decision 5).
- **Generate doesn't switch modes** (decision 6). Entering Simulate with no toolpath
  generates one.
- **3D in both** (decision 7): the 3D view shows the toolpath's lines in Edit, the shaded
  stock in Simulate; drawing tools work in top view as before.

## 3. The simulation is computed in Simulate only

When Simulate is entered, and after each new toolpath while in it. Leaving Simulate keeps
the last one hidden: back in Simulate without changes, it shows at once. A new toolpath
made in Edit drops it (it no longer matches).

## 4. The toolpath is still made automatically in Edit

With *Update toolpath automatically* on, as before: its lines, tabs, starts, the problems
and the cycle time are what Edit is for (decision 1).

## 5. Only edits that can change the cut make a new toolpath

A run is made from: the project without its shapes; the contours each operation and rest
pass that is cut selects, with their numbers and geometry (planning reads nothing else of
the drawing); each setup's board and work zero; and whether the drawing reaches outside the
stock (a note). `chipload_pipeline::cut_inputs` gives them. After an edit, or an undo or
redo, the window compares them with what the toolpath shown (or the run under way) was made
from:

- **The same:** the toolpath stays, nothing is generated, no simulation is redone. Drawing a
  shape no operation selects, on a board that doesn't fit the drawing, inside the stock;
  construction geometry; moving or deleting such a shape.
- **Different:** as before, the toolpath is out of date and made again. A shape on a layer
  an operation cuts; any change to operations, tools, machine, setups, parts or the
  drawing's files, pictures and texts; a shape that changes a fitting board's size (a zero
  on its corner moves every coordinate); a shape off the stock.

A run finishing after such an edit is kept too, not dropped. Exporting always regenerates
first when anything that can change the cut changed (as before).

## 6. Not now

- Remembering the mode between runs or per project.
- Opening the simulation in a window of its own.

## 7. Tests

- The window: a shape no operation selects keeps the toolpath (also through undo and redo);
  a shape an operation selects, one off the stock, and one that grows a fitting board make a
  new one.
- Modes: projects open in Edit with no simulation computed; Simulate computes it and a drag
  pans there (no selection box); F6 switches; a drawing tool or a Browser selection goes back
  to Edit; leaving and re-entering Simulate without changes keeps the simulation; a box
  selection works in Edit after the simulation was shown (the user's report).
- The guide's pictures that showed the simulation by default are made in Simulate.

## 8. Decisions

Agreed 2026-10-30:

1. **The toolpath stays automatic in Edit**; only the simulation waits for Simulate.
2. **Edit and Simulate, one switch** (the view bar, View menu, F6).
3. **In Simulate the canvas is a viewer:** drags pan or orbit, clicks only open an
   operation from its number.
4. **Properties stay editable in Simulate.**
5. **Starting a drawing or editing tool, or a Browser selection, returns to Edit.**
6. **Projects open in Edit; Generate doesn't switch.**
7. **The 3D view in both modes** (settled while building: it shows the toolpath's ramps in
   Edit too).
8. **Only edits that can change the cut regenerate** (§5); exporting regenerates first when
   needed.
9. **F6** for the switch (settled while building: S is already *Draw slot*).

Agreed 2026-10-31 (the user: "maybe the simulation mode button should be more distinctive,
now it does not stand out"):

10. **An Edit | Simulate switch and a frame.** The *Simulate* button looked like the bar's
    toggles beside it. Now the mode is a switch of its own, first in the bar: *Edit* (a
    pencil) and *Simulate* in an outline of the accent colour (Generate's), the mode's half
    filled; each half sets its mode. In Simulate a 3-point frame of the accent colour runs
    round the canvas. Chosen over only filling the Simulate button in the accent colour.
