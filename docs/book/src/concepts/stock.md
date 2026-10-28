# Stock and work zero

Job settings has four sections, in the order a job is set up: **Stock** (the board),
**Zero** (where the G-code's zero is), **Material** (the feeds new operations take) and
**Machine** (set once, so it starts closed; its summary line gives the profile, post, safe
Z and spindle).

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

The **Zero** section's summary line says where the zeros are, e.g. "X0 Y0 drawing origin ·
Z0 stock top", so you can read it with the section closed.

**Work zero X Y** is the point of the drawing that becomes X0 Y0 in the G-code:
- **the drawing's origin**, the default;
- **a stock corner or the stock centre**;
- **any point**, in drawing coordinates.

**Z zero** is where Z0 is: the stock **top** (depths are negative Z) or its **bottom**, the
spoilboard (the top is at Z = thickness).

Only the G-code moves; the drawing and the canvas stay where they are. The canvas draws the
X and Y axes at the work zero.

Every program says where its zero is in a comment at the top, e.g. `(Work zero: stock center
= drawing X150.000 Y110.000, Z0 = stock top)`, and so does the setup sheet. Set the
machine's zero exactly there.

## Safe Z and travel

Under Machine:
- **Max depth below stock:** how far a cut may go below the stock bottom, into the
  spoilboard. Within it a cut gets a warning; deeper is refused. New projects: 0.5 mm.
- **Safe Z:** the height for every rapid move sideways, above the stock top. Make it clear
  of your clamps. Before a plunge the tool rapids down to 1 mm above the stock top, then
  feeds; it never rapids below the stock top.
- **Travel X Y Z:** the machine's travel. When set, the checker refuses a program that
  moves farther than the machine can.
- **Max feed:** the fastest feed the machine takes. Feeds above it are refused, and the
  cycle time uses it for rapid moves.
- **G-code units:** mm (`G21`) or inch (`G20`), as your controller expects. Only the program
  changes; see [Units and coordinates](units.md). Command line: `chipload set part.chipload
  machine.gcode_units=inch`, or `--gcode-units inch` on `machine add` and `machine set`.
