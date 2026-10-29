# Spec: the stock around the drawing, and the cut's outline

Status: **decided and built 2026-10-28** by the user during the router shakedown
(2026-10-28).

## Why

The user, on the router: "The stock should be positioned centered above the drawing so to
get equal amount of stickout. Right now the stock starts at origo, which is not what you
want, often the drawing starts from the origo and the stock must cover it." And: "instead of
giving the stock size in the setup, we could calculate the project outline and recommended
minimum stock size … For me the stock is mainly a visual aide when simulating, in practice I
take whatever piece of material I have laying around. Also the total outline of the cut is
a nice number to have … of course, one could have a fixed stock that they want to utilize,
so that option should still be there."

## Decisions (the user, 2026-10-28)

1. A stock **centred on the drawing**, following it: a third stock position, beside "corner
   at the drawing origin" and "centred on the drawing origin".
2. A new project's stock **fits the drawing**: its outline plus **10 mm** each side.
3. **A fixed size stays possible:** typed, for a board the user wants to use.
4. The **work zero stays the drawing origin** by default.
5. The **cut's outline** is reported: the area the tool cuts, its radius included.

## As built

- **File (schema 33):** `stock.origin` takes `"drawing"`; `stock.fit` is the margin in mm,
  or null for a fixed size (`x`, `y`). `fit` needs `origin: "drawing"` (else a problem).
  Migration 32 → 33 changes only the version: older projects keep their stock as it was,
  every golden G-code is unchanged.
- **The drawing's outline** (`Drawing::extents`): every shape but construction geometry,
  arcs included. An empty drawing centres the stock on the origin, at `x` × `y`.
- **One computation:** `Job::stock_size` and `Job::stock_bounds` give the stock everyone
  uses: the checker (rapids through the stock, travel), planning, the simulation, the setup
  sheet, the canvas, work zero at a stock corner.
- **New projects** from a DXF or SVG: centred on the drawing, fitting it with 10 mm. A
  project started empty fits too (the user after the first cuts, 2026-10-28: fitting should
  be the default): 300 × 200 mm centred on the origin until the first shape, then the
  drawing plus 10 mm (1/2" when lengths are shown in inches). Saved projects keep what
  they have.
- **Window:** Job settings > Stock starts with a **Fit the drawing** box and the margin
  beside it (the user, 2026-10-28: a button at the end of the size row was pushed off a
  narrow panel). Off, the size it had stays, fixed, and the size row takes typed X and Y.
  Then the drawing's size, and the new position in the list. Choosing a position at the
  origin keeps the size and makes it fixed.
- **Align to the stock** (`place --align`, the window's *Align*) is refused for a stock
  centred on the drawing: it would follow the drawing.
- **The cut's outline** is part of the estimate (`Estimate::outline`, from the planned
  moves with each tool's radius): a line after the estimated time in `run`, `plan`, the
  window's report and the setup sheet. The frozen English messages and the setup-sheet
  goldens gain this line; nothing else changes in them.
