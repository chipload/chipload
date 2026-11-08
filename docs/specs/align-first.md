# Spec: Align keeps the shape picked first in place

Status: built (2026-11-08; decision by the user: "first picked should not move. even when
doing multiple aligns after another using the same selection"). Changes
docs/specs/drawing-editing.md §7; builds on arrange-whole.md.

## 1. Why

The user's report: pick shape 1, then shape 2, *Align left*: shape 2 moves to shape 1's
left side. Then, with the same selection, *Align centre*: shape 1 moves to shape 2's
centre. Align lined up with the box round all the selected shapes, so which shape moved
depended on their sizes and places. A second align could move the shape the first one
had lined the others up with. The window kept the selection as a set of ids, so the order
of picking was lost anyway.

## 2. The reference

`align_shapes(job, ids, to, with)`, `with` one of:

- **First:** the unit (arrange-whole.md) holding the first id named stays put; every other
  unit lines its box side or middle up with that unit's box. Two units or more.
- **All:** the box round them all, as before. Two units or more.
- **Stock:** the stock's box, as before. One unit is enough.

The command line: `chipload shape align P IDS --to …` lines up with the first id named;
`--stock` with the stock. Distribute is unchanged.

## 3. The window: which shapes came first

The app notes, each frame, the shapes Align would take and compares them with the last
frame's:

- **A selection made anew** (none of its shapes were selected before: a click, a click
  on a group, a box drag on nothing selected) is first as a whole.
- **Shapes added to it** (Ctrl+click, a box drag adding) come after; the first stay first.
- **When some of the first are taken out of it**, none is first.
- **Nothing selected:** none.

Align lines up with **First** when the shapes that came first are all in one unit (a shape,
the pieces of one contour, or a group picked whole), and puts them first. Otherwise (a box
drag that picked several things at once, or the first taken out) with **All**. The stock
box wins over both. Aligning keeps the shapes' ids and the selection, so the next align
keeps the same first. Hovering over *Align* tells which applies.

## 4. Tests

- `chipload-pipeline` tests/arrange.rs: left then centre with First, the first stays
  exactly; named the other way round the other stays; All moves both to the box's middle.
- `chipload-gui` tests/drawing.rs: S3 clicked, S2 added (the lower id): left then centre,
  S3 never moves; a selection made all at once lines up with the box round them all.
