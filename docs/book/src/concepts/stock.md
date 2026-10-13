# Stock and work zero

Job settings > **Stock** describes the material on the machine, and Job settings >
**Machine** says where the G-code's zero is.

## Stock

| Setting | |
|---|---|
| **Size X × Y** | the board's size, mm |
| **Thickness** | mm. Depths count down from the stock top, so set this to the real board. |
| **Max depth below stock** | how far a cut may go below the stock bottom, into the spoilboard. Within it a cut gets a warning; deeper is refused. New projects: 0.5 mm. |
| **Stock position** | where the stock lies around the drawing's origin: its bottom-left corner or its centre there |
| **Z zero** | where Z0 is: the stock **top** (depths are negative Z) or its **bottom**, the spoilboard (the top is at Z = thickness) |

A new project from a DXF gets stock that just covers the drawing. If the drawing reaches
past the stock (after an update, or a move), the report says so.

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
  of your clamps.
- **Travel X Y Z:** the machine's travel. When set, the checker refuses a program that
  moves farther than the machine can.
- **Max feed:** the fastest feed the machine takes. Feeds above it are refused, and the
  cycle time uses it for rapid moves.
