# Spec: two-sided work made easier: the turn axis, warnings, a wizard, pins placed again

Status: **approved 2026-11-10** ("Go ahead": decisions 1-8 as recommended); all three steps built (step 3 changed by decision 10).
The user asked for suggestions ("the workflow
with adding dowels for two sided … is not so intuitive"), then approved the suggested order
and asked for this spec. It builds on docs/specs/two-sided.md (§3 the link, §6 dowel holes),
whose rules for the holes, the checker and the sheet stay as they are.

## 1. Why

Today, in the window, two-sided work with dowel pins takes four separate steps:

1. right-click a setup and choose *Add the back side*;
2. draw the back's shapes;
3. find *Add dowel holes* in the back setup's form, below *Turn over* and *Put back by*;
4. set the front's work zero to a pin yourself.

What goes wrong:

- **Hidden.** The button is only in the back setup's form. The front's form doesn't mention
  dowels. The only reminder that they are missing is a line on the setup sheet, seen after
  generating.
- **No choices in the window.** The window always adds 8 mm pins 8 mm into the spoilboard
  and leaves the work zero where it is. The command line has `--diameter`, `--depth` and
  `--zero`, and the guide recommends the zero at the pin.
- **The order matters, and nothing says so.** The holes are placed beyond the shapes there
  are when you click. Added too early, they are refused, or they collide with the shapes
  drawn later.
- **The holes don't move afterwards.** They are plain circles. Change the board, its margin
  or a part, and they stay put. You find out when *Generate* is refused with "into the
  dowel pin…".
- **"No room" is a dead end.** The message names the margin needed, but you have to find
  the stock form, change it, and come back.
- **You can't see the turn axis,** so you can't see where the pins go or whether the shapes
  clear them.

## 2. Step 1: the turn axis on the canvas

While the active setup has a back, or is one, the canvas draws the **turn axis**:

- a dashed line across the board, on the board's centre line, vertical for *left to right*
  and horizontal for *top to bottom*;
- with a small turn arrow and the words *Turn axis* at one end, outside the board.

It is drawn in the stock outline's colour, with longer dashes, under the shapes. The dowel
circles are ordinary drawn shapes and show as they do today. *View > Show turn axis* turns
it off and on (on by default, for the session, as *Show legend*).

The axis is only drawn. It isn't a shape, isn't selected and isn't snapped to (decision 1).

## 3. Step 1: warnings where you are

A back put back by **pins**, whose front has **no dowel holes** (none, or all of them off),
gets a warning in three places. A warning, not a problem: it doesn't stop *Generate*, as you
may fit pins some other way.

- **The Browser:** ⚠ after the back setup's name, with the reason on hover.
- **The setup's form:** the *Dowel holes* row says *None yet* in the warning colour, beside
  the button.
- **The report after generating:** one line, "Setup 1 back is put back by dowel pins, and
  Setup 1 has no dowel holes for them: add them, or put it back by a fence corner". The same
  text goes on the sheet, as today (`sheet-turn-no-dowels`).

**The front's form** gets a row, **Back side**:
- *None*, with **Add the back side**;
- or the back's name as a link, and its dowel holes: *Operation 3* (a link to it), or
  *None yet* with **Add dowel holes**.

**In the back's form:** *Operation 3* becomes a link. A **Remove** button beside it removes
the operation and its circles, as one undo step.

## 4. Step 2: the wizard, *Two-sided…*

**WIZARDS > Two-sided…** (a large ribbon button, after *Sign*), and *Add the back side…*
when you right-click a setup, open one dialog. It makes the back side, with its part and its
dowel holes, as one undo step. Opened on a setup that already has a back, it changes that back
and its pins instead (decision 4).

| Row | Values | Default |
|---|---|---|
| **Front** | the setups with no back yet, or the back's front | the active setup |
| **Turn over** | *Left to right*, *Top to bottom* | left to right, or the back's |
| **Put back by** | *Dowel pins*, *A fence corner* | pins |
| **Pin diameter** | length | 8 mm |
| **Into the spoilboard** | length | 8 mm |
| **Bored with** | the project's drills and end mills, then the library's (+), as in *New sign…* | a drill of the pin's diameter, else the smallest end mill that can bore it (1.2 × its diameter at most the pin's) |
| **Work zero at the first pin** | check | on |

The pin rows show only with *Dowel pins*.

**The picture** draws the board to scale:
- its outline;
- the area both sides cut, shaded (the room dowel holes keep clear of, two-sided.md §6);
- the turn axis, dashed;
- the two pins, as circles;
- the work zero marker, at the first pin when that is checked.

It is worked out as the holes are: from the plans of both sides. It is made again when a
value changes, kept by a key as the sign's preview is, so the dialog never waits each frame.

**No room.** When the margin can't hold the pins, the picture shows where they would go, in
the warning colour, and the message names the room needed:
- **A fitted board** (fit margin): a button, **Widen the margin to 16 mm**, sets the margin in
  the dialog. It changes the board only when you click *Make*, and says so: "The board will
  be 6 mm longer: saw it to the new size" (decision 5);
- **A board of its own size:** the message names the length the board needs. Change it in
  its form; the dialog follows when you come back.

**The tool.** A library tool chosen is copied into the project when you click *Make*, as in
the sign and inlay dialogs (app/wizard_tools.rs). The holes operation cuts with it. A drill
drills; an end mill bores, as holes choose today.

**Make** (*Make the back side*, or *Change the back side* for an existing back):
1. adds the back setup with its part, as *Add the back side* does (or changes its turn and
   how it is put back);
2. adds the dowel holes, as *Add dowel holes* does, with these values;
3. sets the front's work zero to the first pin, if checked;
4. makes the back the active setup and selects it in the Browser.

Then a report line says what was made. The back's shapes are drawn afterwards, as today.
If the back's cuts later come too close to a pin, *Place again* (§5) moves the holes, as
long as the front isn't cut yet.

With *A fence corner*, it makes the back and no holes, and the sheet says what it says today.

## 5. Step 3: pins placed again, on request

*Changed 2026-11-10, decision 10 (the user, asked while building):* the pins **never move
by themselves**. The draft had them follow every change, as frames do. Two things spoke
against it. Following means planning both sides' toolpaths at every edit, which can stall
the window for seconds on a V-carve-heavy project. And once the front is cut, its holes are
drilled: moving them, and the work zero with them, would leave the project disagreeing with
the board on the machine. A frame is drawing; a pin is a hole.

So the dowel circles stay where they are made, as plain circles, until you ask:

- **Place again** in the back setup's form, beside *Remove*: the two circles move to where
  the holes would go now, clear of both sides' cuts as they are now, centred in the margin
  on the turn axis. The operation keeps its tool, depth and feeds. A front work zero at the
  first pin moves with it; a zero elsewhere stays. One undo step. Its hover says: only
  before the front is cut.
- **When Generate is refused** because the back cuts into a pin, or warns that a front cut
  breaks into a hole, the report adds a line, **Place the dowel holes again**, that does
  the same.
- **No room:** refused with the room needed, as *Add dowel holes* is; the wizard (§4) can
  widen a fitted board's margin.
- `setup dowels --place` on the command line.

Nothing new is saved: no record of the pins, no schema change.

## 6. Command line

```
chipload setup add sign.chipload --back-of 1 --turn left-right --pins 8 --pin-depth 8 --zero
chipload setup add sign.chipload --back-of 1 --register corner
chipload setup dowels sign.chipload --setup 2 --diameter 6 --depth 10 --zero --tool 3
chipload setup dowels sign.chipload --setup 2 --place      # place them again, where they go now
chipload setup dowels sign.chipload --setup 2 --remove     # the operation and its circles
```

- `setup add --back-of … --pins D` does in one command what the wizard does: the back setup
  and its dowel holes. `--pins` without a value takes 8 mm.
- `setup dowels` gains `--tool`, `--place` (§5) and `--remove`.
- `setup list` shows each back's pins: diameter, depth, placed or by hand.
- `--json` gives the same.

## 7. Guide and sheet

- *Setups → The back side* starts with *Two-sided…*, then the details; *Dowel holes* says
  that the holes stay where they are made, and how *Place again* moves them.
- *The window*'s ribbon table: WIZARDS gets *Two-sided*.
- The sheet: no change.

## 8. Not now

- Pins off the turn axis, or more than two (two-sided.md keeps them on the axis).
- Pins for a fence corner. A corner needs no holes.
- Faint pins on the canvas before any are added (it would need both sides' plans every
  frame; the wizard's picture shows them).

## 9. Order of building

1. **Step 1:** the turn axis (§2) and the warnings, the front's *Back side* row, the links
   and *Remove* (§3).
2. **Step 2:** the wizard (§4), with `setup add --pins` and `setup dowels --tool` (§6).
3. **Step 3:** pins placed again on request (§5), with `--place`.

One commit each, with tests, the guide and TODO.md.

## 10. Decisions

1. **The turn axis is only drawn** (§2): not a shape, not selected or snapped to, with a
   *View* toggle, on by default. *Recommended.* Alternative: a snap target too, to place
   things on it. Moving a dowel circle already keeps to the axis.
2. **Missing dowel holes are a warning, not a problem** (§3): it never stops *Generate*.
   *Recommended:* pins may be fitted some other way. Alternative: a problem that stops
   generating until holes are added or the back uses a corner.
3. **The front's form shows its back side and dowel holes** (§3), with *Add the back side*
   and *Add dowel holes* there too. *Recommended.*
4. **One dialog makes and changes a back side** (§4): *Two-sided…* in WIZARDS, and *Add the
   back side…* on a setup's right-click, which today adds a back with no questions.
   *Recommended:* the right-click item asks the same questions. Alternative: keep the
   quick right-click item and add the wizard beside it.
5. **The margin is widened only by a button you click** (§4), applied when you click *Make*
   and saying the board grows. *Recommended,* as two-sided.md §6 says: never silently.
6. ~~A pin moved by hand stays where you put it~~: moot after decision 10, as nothing
   moves by itself.
7. ~~Pins that follow live on the back's `back_of`~~: moot after decision 10; nothing new
   is saved.
8. **The wizard defaults the work zero to the first pin** (§4). *Recommended:* front and back
   then share it exactly (two-sided.md §6); today the window never sets it. Alternative:
   off, as `setup dowels` defaults today. That stays off on the command line unless
   `--zero` is given.
9. **The order of building** (§9): the axis and warnings, then the wizard, then pins that
   follow. *Approved by the user, 2026-11-10.*
10. **Pins are placed again on request, never by themselves** (§5). *Chosen by the user,
    2026-11-10*, of: on request (recommended); following every change with a quick
    estimate; following with planned toolpaths; no step 3. Following was slow and unsafe
    once the front is cut.

## 11. Settled while building

Step 1, 2026-11-10:
- `setup dowels --remove` came with step 1, as the window's *Remove* did (the command line
  first); `setup list` says *dowel holes: operation N* or *no dowel holes (warning)* for a
  back with pins, and `--json` gives `dowels` and `missing_dowels`.
- The axis label is *⟲ Turn axis*, past the board's top (left to right) or right end (top
  to bottom).

Step 2, 2026-11-10:
- **The tool by default** is a drill of the pins' size, else the **largest** end mill that
  bores them (the draft said the smallest: the largest bores quickest, and both fit), else
  *Chosen for the holes* (the holes operation's own choice). `setup dowels` without
  `--tool` takes the same (`dowels::pin_tool`), so the window and the command line agree.
- **Add dowel holes** (in a back's or a front's form) opens the wizard too, so the window
  always asks for the size, depth, tool and zero. The one-click versions stay only for
  tests.
- **Changing the turn** of a back with dowel holes takes them out first: on the new axis
  they would be refused. With *A fence corner* and the same turn, holes there are kept.
- The picture draws the drawing's shapes faintly too, so you see what the board holds.

