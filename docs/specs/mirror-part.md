# Spec: mirrored parts

Status: **approved 2026-11-09**, all decisions as recommended (the user: "Go ahead"). Built
2026-11-09 on branch `signs` as described, with these details settled while building:
- the form's **Mirror** is a drop-down (as Turn over is for a back setup), not three
  buttons with pictures;
- the mirrored outlines are drawn dashed and faded; the markers the user moves (ring
  starts, tab centres, unreached corners) stay on the shapes as drawn, where they are
  stored;
- the planner mirrors each step's moves, arcs, tabs and pocket pieces about the copy's
  centre line, and plans from the tool's real position mirrored back;
- a back setup's text note is left out for a mirrored part (the two mirrors cancel), and
  a mirrored part's note is said once per part and text;
- golden `v36_mirrored_part`; the direction tests share their helpers with two-sided
  (`crates/chipload-pipeline/tests/common`).

Reviewed again the same day (the user: "rethink if the mirror strategy is sound"). Planning
as drawn with the sense turned, then mirroring, holds: the checker judges the real G-code,
and everything that compares positions across the board (parts and copies kept apart, the
board's fit, the simulation, dowel placement) works on the mirrored moves. Two things did
not hold, fixed with a test each:
- **The box** counted only operations that are on, so switching the outline off to cut a
  pocket again moved the mirror line and the pocket. It now counts every mirrored
  operation of the part, on or off: it changes only when the part's shapes change.
- **Dogbone legs** are checked against what every operation cuts (dogbones.md §6). Other
  parts were seen where they are drawn, not where they lie relative to the mirrored part.
  Their contours are now mirrored into the part's planning frame (`Contour::mirrored`).
Known limits, kept: the start, tab and corner markers stay on the shapes as drawn (where
they are stored) while the toolpath is drawn where it is cut; moving the part's shapes
moves its mirror line with them, as moving them moves an unmirrored part.

Builds on docs/specs/parts.md (a part: operations cut together, with copies),
docs/specs/two-sided.md §4 (planning with the cutting sense flipped, then mirroring the
program: branch `two-sided`) and docs/specs/copies-grid.md.

## 1. Why

The user (2026-11-09): "I also want a general mirror part functionality." Uses:

- the plug of an inlay, cut mirrored so it reads right when turned face down
  (docs/specs/vcarve-inlay.md);
- left- and right-hand pieces from one drawing (a pair of brackets, the two sides of a box
  with a groove);
- stamps, branding irons and moulds, whose text must read backwards;
- a sign cut from the back of clear acrylic, to be read through it.

The drawing already has *Mirror* for shapes (`shape mirror`), which changes the drawing. A
mirrored part leaves the drawing as drawn and **cuts** it mirrored: one drawing, both
hands, and an edit to the artwork reaches both.

## 2. What it does

A part gets a **mirror**: none (the default), **left to right** (X negated) or **top to
bottom** (Y negated), about the centre line of the part's **box**: the bounding box of the
shapes its operations select, on or off (a surface pass and dowel holes aside). Mirrored about its own centre, the part keeps its box,
so it stays where it is on the board, and a board that fits its parts fits it unchanged.

Every operation of the part is cut mirrored. Its settings mean what they say on the
mirrored part:

- **inside stays inside,** **outside stays outside;**
- **climb stays climb,** **conventional stays conventional;**
- tabs placed by hand, dogbone corners and a ring's start point stay on their feature;
- tabs along X or Y keep their axis;
- depths, feeds and order of operations are untouched.

## 3. How (decision 2)

As a back setup is (two-sided.md §4): **the part is planned in drawing coordinates with the
cutting sense flipped, and its finished toolpath is mirrored** about the part's centre line
(X or Y negated, G2 and G3 swapped), before the copies are shifted into place and before the
post. The planner already takes the cutting sense rather than the machine's spindle
(`cut_clockwise(side, direction, sense)`), so nothing new chooses a direction.

Mirroring the finished toolpath *without* flipping the sense would turn every climb cut into
a conventional one without a word. This is the part that can break tools, so it gets the
same explicit tests as the back setup: for every operation kind with a direction (profile,
pocket, follow path, helical bore), climb and conventional, a mirrored part's G-code is
compared move by move with an unmirrored part cutting the same shapes drawn mirrored by
hand.

With copies, each copy is the mirrored part, shifted. In a back setup, the part's mirror
and the board's turn both apply: the sense flips twice, which the tests cover too.

The checker judges the mirrored G-code as it judges any program.

## 4. Text

A mirrored part carves its texts reading backwards. That is often the point (a plug, a
stamp), so it is a note, not a warning, on the part: "Part 2 (Plug) is mirrored: its text
reads backwards on the board, and right when turned face down."

## 5. In the window

- The part's form gets **Mirror**: none, left to right, top to bottom (a row of three
  buttons with small pictures). The Browser's part row shows a small mirror mark.
- **The preview shows the part as cut** (decision 3): the toolpath mirrored on the board,
  and the part's shapes drawn mirrored there as dimmed outlines; the shapes as drawn stay
  where they are, to edit. The simulation is the program, so it is mirrored as cut.
- The setup sheet names the part as mirrored.

## 6. Command line and file

    chipload part set sign.chipload 2 --mirror left-right    # also top-bottom, none

```json
"parts": [ { "id": 2, "name": "Plug", "enabled": true, "copies": null, "setup": 2,
             "mirror": "left_right" } ]
```

`mirror` is optional: missing means none, and it is left out when none. No schema bump:
older Chipload refuses a part with an unknown field (`deny_unknown_fields`) and reads every
other file as before.

## 7. Tests

- The sense test of §3, per operation kind, climb and conventional, also with copies and
  in a back setup.
- The part keeps its box: the mirrored toolpath's extents equal the unmirrored ones.
- A board that fits its parts is the same with the mirror on.
- Golden: a small mirrored part (a profile with tabs by hand and a pocket), its program.
- CLI: `part set --mirror`, `get`/`set parts.0.mirror`.

## 8. Out of scope

- Mirroring about any other line than the part's centre lines (a part moved as well as
  mirrored: mirror its shapes in the drawing, as today).
- Turning a part by an angle (DXFs and pictures turn already; a part doesn't).

## 9. Decisions

1. **A part setting, in place** (§2): the part's operations cut mirrored about its own
   box's centre line, the drawing untouched. *Recommended.* Alternative: a command that
   copies the shapes mirrored and the operations with them (two things to keep in step
   afterwards).
2. **Plan with the sense flipped, then mirror the toolpath** (§3), the two-sided
   mechanism. *Recommended*: one way of mirroring in the code, already tested move by move.
   That needs the `two-sided` branch in main first (decision 4).
3. **The preview shows the part as cut**, its shapes as dimmed mirrored outlines (§5).
   *Recommended*: on the plug's own board, what is cut is what should be seen. (A back
   setup shows the front's coordinates because its whole board is turned; a part isn't.)
4. **Order:** merge `two-sided` into main, then rebase `signs` on it; build the mirrored part
   first, the inlay on top. *Recommended.* Alternative: build mirroring again on `signs`
   (two ways of doing the same risky thing).
