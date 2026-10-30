# Stock and work zero

The board is its [setup](setups.md)'s: click the setup in the Browser for its properties.
They have two sections, **Stock** (the board) and **Material** (the feeds new operations
take), and above them a line that says where the zero is, e.g. "Work zero: X0 Y0 stock bottom
left · Z0 stock top"; click it to set the zero. The machine, shared by every setup, is under
**Machine**.

## Stock

| Setting | |
|---|---|
| **Thickness** | first, as every job needs it. Depths count down from the stock top, so set this to the real board. |
| **Size: Outline + margin** | new projects: the stock is the drawing's outline (its size in brackets) plus the **margin** on every side, centred on it, and follows the drawing as it changes. |
| **Size: Fixed size, centred** | a size you type, for a board you want to use, centred on the drawing's outline; it starts at the size the stock had. The drawing's size is shown under it, to check the board against. |

A project from before 28 October 2026 may have its stock's corner or centre at the drawing's
origin. It keeps that place, and a list under the size shows it, until you choose
*centred on the drawing* there. On the command line, `stock.origin` still takes
`bottom-left` and `center`.

The **Material** section chooses your own feeds per tool for what you cut
([Materials](../library/materials.md)): every operation, and those added later, takes it and
its values. It never changes the stock.

For many jobs the stock is mostly a picture for the simulation: you cut from whatever board
is at hand. What you need to know then is how big a board must be. **After Generate, the
report gives the cut's outline**, the area the tool cuts with its radius included, for
example `Cut outline: 144.068 × 111.622 mm`: the smallest board that holds the job. The
setup sheet has it too.

If the drawing reaches past a fixed stock (after an update, or a move), the report says so.
A stock centred on the drawing can't be used to align the drawing (it would follow it):
choose a position at the drawing's origin first.

## Work zero

**Work zero** is the point that becomes X0 Y0 Z0 in the G-code. It has its own row in the
[Browser](../window.md#the-browser), under its setup, which says where it is ("stock bottom
left"), and its own place in the view: the foot of the red and green axes.

Only the G-code moves; the drawing, the stock and the toolpath stay where they are. So moving
the zero never moves a cut, only the numbers in the program. The setup sheet and the
program's first lines say where it is.

**In the view:** click the round handle at the foot of the axes to select the zero, then drag
it. It snaps to the stock's corners and centre and to the drawing origin, then as the pointer
snaps when drawing (points of the drawing, the grid); hold **Alt** to place it freely. A label
at the pointer says where it goes. **Esc** while dragging leaves it where it was, and one drag
is one step for Undo. Until the zero is selected, a drag on its handle does what it does
anywhere else, so a drag meant for a shape under it never moves it. Once the zero is away
from the drawing origin, the origin stays in view as grey ghost axes, so you can still see
where the drawing's own X0 Y0 is.

**Its properties:**
- **On the stock:** a small drawing of the board with a dot at each corner and the centre.
  Click one: the zero stays at that corner when the stock changes size.
- **Drawing origin:** where the drawing has its own 0,0.
- **Position:** where the zero is, in drawing coordinates. Type a value to make it a point of
  its own.
- **Z zero:** the stock **top** (depths are negative Z) or its **bottom**, the spoilboard (the
  top is at Z = thickness).

New projects start with the zero at the stock's **bottom-left corner** and Z0 on its top: the
corner you touch off on. A [machine profile](../library/machines.md) can give new projects
another zero. Projects from before 29 October 2026 keep theirs, often the drawing origin.

On the command line:

```
chipload zero part.chipload                       # where it is
chipload zero part.chipload --stock bottom-left   # also bottom-right, top-left, top-right, centre
chipload zero part.chipload --drawing-origin
chipload zero part.chipload --at 25,-10           # a point; any length unit
chipload zero part.chipload --z bottom            # Z0 on the spoilboard
```
