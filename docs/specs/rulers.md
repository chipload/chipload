# Spec: rulers on the canvas

Status: **agreed and built 2026-10-31.** The user: "I want a ruler so I can see the scale of
axis directly." Asked which coordinates the numbers show, the user chose drawing
coordinates (recommended); the rest as below.

## What

- A ruler along the canvas's **top** (X) and **left** (Y) edges, 18 points thick, in **top
  view** only (none in 3D or a turned view, where a ruler would be wrong). In Edit and
  Simulate alike.
- **Drawing coordinates** (the user's choice): 0 at the drawing origin, as the shapes'
  *Position* fields and the grid. Alternative not taken: the work zero's (program)
  coordinates; the status bar already shows those beside the drawing's.
- In the **display units**: millimetres, or inches when the app shows inches; the unit in
  the corner where the rulers meet.
- **Ticks on the grid's lines** (`draw::grid_step`, `grid_major_every`), so ruler and grid
  agree at every zoom: a short tick on each line, a longer one half way between the
  stronger lines (in mm every fifth), a full one with its number on each stronger line (in
  mm every tenth; in inches every inch, or every foot when zoomed far out). Numbers without
  trailing zeros ("150", "0.5"); the left ruler's read upwards.
- **The cursor** is marked on both rulers in the accent colour.
- **On by default**; *View > Show rulers* turns them off, remembered in the app settings
  (`show_rulers`, an optional field, on when missing: no version change).
- **Fit** (Home, the view fitted to the drawing) leaves the rulers' strips free, so the
  drawing doesn't start under them.
- The rulers are painted over the canvas and take no clicks of their own.

## Tests

- `draw::ruler_ticks`: on the grid, numbered on its stronger lines, half ticks, inches.
- Settings: a file without `show_rulers` shows them.
- Window: the choice is remembered; drawn in top view and left out in 3D.
