# Units and coordinates

- **Millimetres inside.** Chipload works in mm throughout: the project and the toolpath.
  A drawing in inches is converted once, on import.
- **G-code in mm or inches.** *G-code units* (Job settings > Machine) says what the program
  is written in: **mm** (`G21`, the default) or **inch** (`G20`), as your controller is set.
  In inches every coordinate, arc centre and feed (in/min) is converted as the program is
  written, with 4 decimals, and the header says so. Everything else, including what the app
  shows, stays in mm. The checker reads the program in its own units and refuses one in
  other units than the machine's, or one that changes units part way.
- **A DXF's units:** Chipload reads them from the DXF's own setting (`$INSUNITS`). A DXF
  that doesn't say is never guessed at: the app asks for mm or inch, and the command line
  needs `--units mm` or `--units inch`. The choice is kept in the project, so updating
  from a new version of the DXF reads it the same way.
- **Coordinates:** X to the right, Y up, as in the drawing. Angles count counter-clockwise.
  The app shows them in degrees, and project files store them in radians (90° is
  1.5707963267948966).
- **Absolute moves:** the G-code uses absolute coordinates (`G90`), in its units, from the
  [work zero](stock.md#work-zero).

The canvas shows the cursor's position in drawing coordinates and in work coordinates.
*View > Measure* (M) gives the distance, ΔX, ΔY and the angle between two clicks.
