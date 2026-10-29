# Spec: the work zero as a thing in the view

Status: **approved 2026-10-29** ("ok go ahead": all five as recommended) and **built** the
same day. A work zero per part was raised and left for *setups* (a board with its own stock
and zero), a later spec.

Follows docs/specs/work-zero.md (the model, the translation at output, the header line: all
unchanged) and docs/specs/job-settings-layout.md (the Zero section).

## 1. Why

The user (2026-10-29): "could we show the work zero as a marker in the main view? it could
show up in the browser, is selectable and can be dragged freely. In the properties panel
there could be shortcut buttons, move to corner of stock."

Today the zero is a list of seven words in Job settings > Zero. The view draws the X/Y axes
there, but they can't be picked. With the stock always centred on the outline, the zero is
what ties the drawing to the board at the machine. It should be seen and moved where it is
shown, as a picture or a text is.

The zero is only a shift applied when the G-code and setup sheet are written
(work-zero.md §3): moving it moves no geometry, no stock and no toolpath. Only the numbers
in the program change. The checker shifts the stock itself, so it still judges the program.

## 2. The Browser

- A row **Work zero** right under *Job settings*, with the axes icon and its place as a
  weak note: "stock bottom-left", "drawing origin", "X 25 Y −10".
- Clicking it selects the zero (`Focus::WorkZero`): the Properties panel shows it, and the
  marker in the view is highlighted.

## 3. The view

- **The marker** is today's axes at the zero, plus a round handle at its foot, drawn above
  the drawing and the stock.
- **Picking:** a click on the handle selects the zero. The handle wins over a shape under
  it, as the picture and text handles do.
- **Dragging:** only once it is selected, so a drag meant for a shape never moves the zero.
  It snaps as the cursor does (snap points, grid; Alt: free), and also to the stock's
  corners and centre and the drawing origin.
  - Dropped on a stock corner or the centre, it becomes that corner
    (`{"at": "stock", "corner": …}`): it follows the stock when its size changes.
  - On the drawing origin, it becomes the drawing origin.
  - Anywhere else, a point (`{"at": "point", …}`).
  - While dragging, a label at the cursor says which: "stock bottom-left", "X 25.0 Y −10.0".
  - One drag is one undo step. Escape while dragging puts it back.
- 2D view only. The 3D view keeps drawing the axes.
- **The drawing origin** (added 2026-10-29, the user after a test cut): once the zero is
  elsewhere, the drawing origin shows as ghost axes, grey and thinner, so it can still be
  found. They can't be picked.

## 4. Properties

```
⊕ Work zero
  X Y zero    [stock picture: ● corners, ● centre]   [Drawing origin]
              stock bottom-left
  X Y         [25.0] [−10.0]       (drawing coordinates; typing makes it a point)
  Z zero      [stock top ▾]
              The G-code's X0 Y0 Z0. Moving it moves no cut, only the program's numbers.
```

- **Shortcut buttons:** a small drawing of the stock, the board's proportions, with five
  dots to click: the four corners and the centre. The current one is filled. Beside it a
  *Drawing origin* button.
- **X Y:** where the zero is, in drawing coordinates. Typing a value makes it a point.
- **Z zero** moves here too: zero is one thing, X, Y and Z.
- **Job settings:** the *Zero* section goes. Its summary line moves above the sections,
  as a link that selects the work zero. One place to set it, two places it shows.
- The picture's row is called *On the stock*, with its hover saying the zero stays at that
  corner when the stock changes size.

## 5. Command line

CLI first (CLAUDE.md rule 5). `set machine.work_zero=…` already works; a short command
makes it plain:

```
chipload zero part.chipload --stock bottom-left     # also centre, bottom-right, top-left, top-right
chipload zero part.chipload --drawing-origin
chipload zero part.chipload --at 25,-10              # a point, any length unit
chipload zero part.chipload --z bottom               # Z zero: top or bottom
chipload zero part.chipload                          # prints where it is (--json too)
```

## 6. Tests

- Browser row, selecting it from the row and from the view.
- A drag snaps: onto a stock corner → `stock` with that corner; onto the drawing origin →
  `drawing_origin`; elsewhere → `point`, with Alt free. One undo step; Escape restores it.
- A drag that starts on a shape with the zero not selected moves the shape, not the zero.
- The shortcut buttons set each choice. Typed X Y make a point.
- `zero` command: each flag, the printout, `--json`.
- The G-code of a project with its zero dragged onto a corner equals the one with the
  corner chosen in the list (same file).
- Layout: the six languages, a narrow panel; pictures.

- **Machine profiles:** a new profile (window or `machine add`) starts with the same zero
  as a new project. A profile's zero is still what new projects take, so an existing
  profile with the drawing origin keeps giving that until it is changed.

## 7. Decisions (all as recommended, 2026-10-29)

1. **Default for new projects:** **stock bottom-left (recommended)**, or keep the drawing
   origin (the answer of 2026-10-28). The stock is always centred on the outline now, and a
   board corner is what the operator can find. Existing projects keep their zero.
2. **Dragging only once selected (recommended)**, or straight away. Straight away is quicker,
   but the marker often sits on a shape's corner, where a drag was meant for the shape.
3. **Z zero moves to the work zero's properties, and the Zero section leaves Job settings
   (recommended)**, or the Zero section stays as well.
4. **A `zero` command (recommended)**, or only `set machine.work_zero=…`.
5. **The old stock positions** (stock at the drawing origin, corner or centre) **stay
   loadable and hidden, as now (recommended)**. A migration that removes them would have to
   move the whole drawing and turn the zero into a point to keep old programs identical.
