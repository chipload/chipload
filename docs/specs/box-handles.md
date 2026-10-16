# Spec: handles for pictures and texts

Status: built 2026-10-17 (asked for directly by the user).

## 1. Why

Drawn shapes have handles on the canvas (drawing-follow-ups.md §1). Pictures and texts could
only be placed by typing numbers into Properties (direct-placement.md, text.md). A drag is
quicker for rough placement; the fields stay for exact values.

## 2. The handles

When a whole picture or text is selected (clicking one of its shapes selects it whole), in
top view, while the project can be drawn on, its box shows handles. The box is the one
Properties' *Position* talks about: the picture, or the box round the letters, before the
turn, turned about its centre.

- **Move:** a square at the box's centre. Dragging it moves the box; the centre snaps as the
  cursor does (snap points, grid; Alt: free).
- **Turn:** a round handle a fixed distance on screen beyond the middle of the box's right
  edge (its own +X side), as a rectangle's. The angle is the direction from the centre to the
  cursor, in steps of 15°; with Alt in steps of 0.1°.
- **Size:** a square at each of the four corners. Dragging one scales the box evenly (the
  picture's or letters' proportions stay); the opposite corner stays where it is. The scale
  is the cursor's distance along the box's diagonal. A picture's width or a text's capital
  height is rounded to 0.1 mm (Alt: not rounded). It can't go below 1 mm.

The box's outline is drawn dashed while one of them shows.

## 3. While dragging

Tracing a picture again can take a while, and every change remakes the toolpath, so a drag
changes nothing until it ends: the box's outline follows the cursor, and on release the
picture or text is set to the new values exactly as its Properties would set them (one undo
step, the report says what was made). A text's new box after a size drag is its letters'
box at the new height; with extra letter spacing it can differ a little from the dragged
box.

Esc during a drag isn't needed: dragging the handle back, or Undo, restores it.

## 4. Where the code is

The box, its handles and what a drag gives are pure functions in
`chipload_pipeline::box_handles` (`PlacedBox`, `BoxHandle`, `box_handles`, `drag_box`), with
`image_box`/`image_in_box` and `text_box`/`text_in_box` to go between them and a
picture or text. The GUI only finds the selected picture or text, shows the handles, and
applies the result through its existing setters.

## 5. Tests

- Moving puts the centre at the cursor; the size and turn stay.
- Turning gives the cursor's direction in 15° steps, and 0.1° steps when free.
- A corner drag scales evenly and keeps the opposite corner, also when the box is turned.
- A size below 1 mm is refused.
- A text with its centre as anchor keeps it as anchor; its position becomes the new centre.
- GUI: dragging a selected text's move handle and letting go moves its letters, as one
  undo step.
