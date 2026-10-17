# Units and coordinates

- **Millimetres inside.** Chipload works in mm throughout: the project and the toolpath.
  A drawing in inches is converted once, on import.
- **G-code in mm or inches.** *G-code units* (Job settings > Machine) says what the program
  is written in: **mm** (`G21`, the default) or **inch** (`G20`), as your controller is set.
  In inches every coordinate, arc centre and feed (in/min) is converted as the program is
  written, with 4 decimals, and the header says so. The project itself stays in mm. The checker reads the program in its own units and refuses one in
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

## mm or inches on screen

*Units* (Preferences, or the VIEW menu) says what the app shows and takes lengths in:
**mm**, or **inch**. It is a setting of this computer, not of the project: a project is
always stored in mm, so it opens the same for someone who works in the other units.

With inch:
- every length field shows inches with 4 decimals (`0.2500 in`) and takes inches. You can
  type a fraction, `1/4` or `1 1/2`, or another unit: `6mm` is turned into inches;
- feeds are in in/min and the chip load in in/tooth;
- the grid steps are inch fractions (1/32", 1/16", 1/8", 1/4", …) with a stronger line
  every inch, and clicks snap to them;
- the cursor position, *Measure*, the playback line and the [setup sheet](../checking/sheet.md)
  are in inches;
- a new project starts from inch values: stock 24" × 12" × 3/4", safe Z 0.2", 0.02" below
  the stock allowed, and a 1/4" endmill.

In mm, fields also take a length in inches: type `1/4in` or `0.25"`.

**On the command line** a plain number is mm, as always, so scripts don't change. A length
can carry its unit: `--height 0.5in`, `--diameter 1/4in`, `--at 1in,2in`, `--width 1 1/2in`
(quoted), `--travel 30in,30in,4in`; feeds `--max-feed 200in/min`. `chipload run
--sheet-units inch` and `chipload sheet --units inch` write the setup sheet in inches.
`--json`, `get`, `set`, the [project file](../reference/project-file.md) and the
[MCP tools](../assistants.md) are always in mm.

Messages follow the display units too: the report's notes, the checker's warnings, the
problems on fields and in the Problems strip, and the setup sheet's *Watch out* list ("cuts
0.0118 in below the stock"). On the command line they stay in mm, as its other output. The
G-code's units are a separate setting of the machine (above): you can show
inches and send mm G-code, or the other way round.
