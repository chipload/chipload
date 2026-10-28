# Stock and work zero

Job settings > **Stock** describes the material on the machine, and Job settings >
**Machine** says where the G-code's zero is.

## Stock

| Setting | |
|---|---|
| **Fit the drawing** | on (new projects): the stock is the drawing's outline plus the **margin** on every side, centred on it, and follows the drawing as it changes. Off: a **fixed size** you type, for a board you want to use; it starts at the size the stock had. |
| **Size X × Y** | the board's size: worked out while it fits the drawing, typed otherwise |
| **Drawing** | the size of the drawing's outline (what is cut; construction geometry left out) |
| **Material** | your own feeds per tool for what you cut ([Materials](../library/materials.md)). Choosing one sets the thickness and the operations' feeds |
| **Thickness** | mm. Depths count down from the stock top, so set this to the real board. |
| **Max depth below stock** | how far a cut may go below the stock bottom, into the spoilboard. Within it a cut gets a warning; deeper is refused. New projects: 0.5 mm. |
| **Stock position** | where the stock lies: **centred on the drawing** (sticking out equally all round, following the drawing; new projects), or its bottom-left corner or its centre at the drawing's origin |
| **Z zero** | where Z0 is: the stock **top** (depths are negative Z) or its **bottom**, the spoilboard (the top is at Z = thickness) |

For many jobs the stock is mostly a picture for the simulation: you cut from whatever board
is at hand. What you need to know then is how big a board must be. **After Generate, the
report gives the cut's outline**, the area the tool cuts with its radius included, for
example `Cut outline: 144.068 × 111.622 mm`: the smallest board that holds the job. The
setup sheet has it too.

If the drawing reaches past a fixed stock (after an update, or a move), the report says so.
A stock centred on the drawing can't be used to align the drawing (it would follow it):
choose a position at the drawing's origin first.

## Work zero

**Work zero X Y** (under Machine) is the point of the drawing that becomes X0 Y0 in the
G-code:
- **the drawing's origin**, the default;
- **a stock corner or the stock centre**;
- **any point**, in drawing coordinates.

Only the G-code moves; the drawing and the canvas stay where they are. The canvas draws the
X and Y axes at the work zero.

Every program says where its zero is in a comment at the top, e.g. `(Work zero: stock center
= drawing X150.000 Y110.000, Z0 = stock top)`, and so does the setup sheet. Set the
machine's zero exactly there.

## Safe Z and travel

Under Machine:
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
