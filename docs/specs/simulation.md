# Spec: material-removal simulation

Status: **approved 2026-10-13**, all decisions as recommended (§10).

User request (2026-10-13): commercial-CAM features 1–5 and 7; this is 6 (surfacing, 5, is
skipped for now). Today the canvas shows the toolpath as lines and the material left as
shaded areas worked out from the geometry. Neither shows the part as it will come off the
machine: the floor a V-carve leaves, where passes overlap, a tab's real shape, a corner a
pocket missed. A **simulation** cuts a model of the stock with the program's moves and the
tool's shape. It shows the result in the top view and in 3D, and gives a few numbers.

Display and report only: nothing here changes a toolpath, the G-code or the project file.
It isn't a safety check either. The independent checker stays the check, and the
simulation never makes a program pass or fail (decision 7).

## 1. The model: a height map

- The stock is a grid of square **cells** over its X–Y rectangle. Each cell holds one
  height: the top of the material there, starting at the stock top. Cutting can only lower
  a cell. Overhangs and undercuts can't be shown, and a 2.5D program makes none.
- **Cell size** (decision 2): the smallest of 0.1, 0.2, 0.25, 0.5, 1 or 2 mm that keeps the
  grid at 4 million cells or fewer. That gives 0.25 mm for a 600 × 400 mm sheet, and 0.1 mm
  for anything up to 200 × 200 mm. The size is shown next to the result ("cells
  0.25 mm"). Features smaller than a cell blur.
- Heights are `f32` in mm in program coordinates (Z0 at the stock top or bottom, as the
  job says), so 4 million cells are 16 MB.

## 2. Cutting

The planned program's moves are used in order (the same `Program` that is posted), with
every copy and every tool group. Rapids cut nothing: the checker already refuses rapids
into material. Feeds cut. Arcs are still chords at this stage (`merge_arcs` runs at
posting), within the arc tolerance.

For each feed move from `A` to `B` (Z linear along it), every cell whose centre `P` the tool
can reach is lowered to the lowest point of the tool over `P` during the move:

| Tool | Shape (`d`: distance from the tool axis to `P`) | Lowest over `P` |
|---|---|---|
| Endmill | flat bottom, radius `r` | `min z(t)` over the `t` where `d(t) ≤ r` |
| V-bit | cone of the included angle, flat tip of `tip / 2`; above the cone a cylinder of the tool diameter | `min over t of z(t) + max(0, d(t) − tip/2) · cot(angle / 2)`, for `d(t) ≤ r` |
| Drill | a 118° point (decision 3) on a cylinder of the drill's diameter | as the V-bit, with a pointed tip |

- An endmill's lowest point is at an end of the stretch of `t` where the tool covers `P`
  (Z is linear in `t`). That stretch comes from a quadratic, so no sampling is needed.
- For cones the minimum over `t` of a convex function is found by solving where its slope
  is zero, clamped to the covered stretch. Exactly, not sampled.
- Only the cutting part is modelled: no flute length, shank or holder. A cut deeper than
  the flutes isn't caught (tool data has no flute length).
- Moves are cut in program order. Each cell only ever goes down, so the final map doesn't
  depend on the order. A later playback (§9) will.

**Speed** (decision 4): moves are cut in strips of at most a few tool diameters, each over
the cells within reach only. A 4-million-cell sheet cut by a 20 000-move program should
take about a second on one core. It runs in the background after generation, as the
preview does (docs/specs/background-generation.md). It is cancelled when the program
changes, and its result is dropped if the project changed meanwhile.

## 3. Where it lives

- `chipload_pipeline::simulate` (decision 5): `simulate(job, plan) -> Simulation` with the
  grid (origin, cell size, size, heights) and the numbers in §5. It reads only the planned
  program, the tools and the stock. It doesn't depend on the toolpath crate's geometry,
  only on its `Program` and `Move`.
- No GUI code in it, no new dependency for the model. The CLI image (§6) uses the `png`
  crate, already in the dependency tree through eframe (MIT OR Apache-2.0); it becomes a
  direct dependency of the pipeline.

## 4. Showing it

- **Top view:** with "Simulation" on (View menu, and a toggle by the 3D button), the stock
  is drawn as an image from the height map:
  - the colour runs from the stock top (the canvas's stock colour) to the deepest cut
    (dark);
  - hill shading from a light at the upper left makes walls and steps stand out;
  - cells cut below the stock bottom (into the spoilboard) are tinted the warning colour.

  The drawing, the selection and the toolpath lines stay on top. The material-left shading
  is hidden while the simulation shows, because the simulation shows the same thing in more
  detail.
- **3D view:** the height map as a shaded surface. egui draws coloured triangle meshes
  without a 3D engine, so it fits the existing orbit camera (docs/specs/3d-view.md).
  - The surface is drawn from a coarser grid of at most 250 000 cells: each cell of it
    takes the lowest height of the cells it covers, so narrow cuts stay visible.
  - Triangles are drawn far to near (orthographic, so one sort per view change).
  - Walls are drawn as vertical faces between cells of different heights.
  - The Z scale (×1 to ×10) applies.
- **While it runs:** "Simulating…" next to the toggle. Then its numbers (§5) appear in the
  report panel.

## 5. Numbers

In the report panel, and from the CLI:

- the volume removed, in cm³;
- the deepest point cut, and how far below the stock bottom it goes, if at all;
- the cell size;
- the time the simulation took (only in the GUI's debug line, not in the report, so the
  report stays the same on every run).

## 6. CLI

`chipload simulate part.chipload -o part-sim.png` plans the project as `run` does (without
posting). It writes the top-view image, one pixel per cell, shaded as in §4, and prints
the numbers in §5. `--cell 0.5` sets the cell size (at least 0.05 mm, at most 10 mm), and
`--no-image` only prints the numbers. Same input, same image, byte for byte: no
timestamps, fixed PNG settings.

## 7. Tests

- **Unit (pipeline):**
  - an endmill plunge: a round hole of the tool's radius at its depth, nothing outside
    `r + cell`;
  - a straight slot: its width and floor;
  - a ramp: the floor slopes as the move does;
  - a V-bit groove: its width at the top matches the follow-path note
    (docs/specs/follow-path.md §5) within a cell;
  - a drill: a cone at the bottom;
  - a rapid cuts nothing;
  - cells only go down;
  - a rectangular pocket's volume: within one cell's width all round its walls, times its
    depth.
- **Exactness:** for random single moves, the analytic minimum agrees with a sampled
  reference (1000 samples along the move) within 1 µm, for each tool shape.
- **Cell size:** the 4-million-cell rule at its boundaries.
- **Determinism:** two runs give identical heights, and the CLI image is byte-identical.
- **Golden:** the CLI's numbers for three corpus projects (a profile with tabs, a pocket, a
  V-carve), as text files beside the other goldens, `expected/sim/`. No golden PNGs (binary
  diffs aren't reviewable). A hash of the heights per project instead, in the same text
  file.
- **GUI:** the toggle shows the image; the 3D mesh is built (vertex count), and isn't
  built when the simulation is off. A screenshot check of both views.

## 8. Edge cases

- **No program yet,** or the program failed: no simulation, and the toggle is disabled with
  a reason.
- **A huge stock with tiny cells:** the 4-million-cell rule picks the cell size. A
  `--cell` that would need more than 16 million cells is refused in the CLI.
- **Moves outside the stock:** the parts outside the grid are ignored. The checker already
  reports them.
- **Several tools:** each move is cut with the tool of its tool group.
- **Copies:** every copy is cut. The grid covers the whole stock.

## 9. Not in this step

- **Playback:** a slider that simulates up to a move, with the tool drawn where it is. It
  needs snapshots to go backwards, so it gets its own spec, as does the playback the TODO
  already lists.
- **Comparing with the design:** marking material cut that should stay (gouges) or left
  that should go. The geometric material-left display does the second today.
- **Holders and flute length.**
- **Ball and bull-nose endmills** (not tool types yet).

## 10. Decisions

1. **A height map (2.5D), shown as an image in top view and a shaded mesh in 3D.**
   **Recommended**: it is all a 2.5D program can make, it is small, and egui can draw both
   without a 3D engine or new dependency. Alternative: a voxel or mesh-boolean model,
   which could show undercuts but needs a real renderer.
2. **Automatic cell size from a 4-million-cell budget** (0.1–2 mm steps), shown with the
   result; `--cell` in the CLI. **Recommended**. A GUI setting can follow if the automatic
   one proves too coarse.
3. **A drill is modelled with a 118° point**: tool data has no point angle for drills, and
   118° is the usual one. **Recommended**, with a note in the result when the program
   uses a drill.
4. **Computed in the background after each generation, final result only.**
   **Recommended**. Playback is a later step with its own spec.
5. **In `chipload-pipeline`** (a `simulate` module), not a new crate and not in the
   checker. **Recommended**: it reads the planned program, as the cycle time estimate
   does.
6. **CLI `simulate` with a PNG through the `png` crate,** already in the dependency tree
   (MIT OR Apache-2.0). **Recommended**: usable from the CLI before the GUI (CLAUDE.md
   rule 5), and the image is useful for a quick look. Alternative: a PGM/PPM file with no
   dependency, which few viewers open.
7. **Display only:** it never fails a program or adds warnings to the check.
   **Recommended**: the independent checker is the safety gate. A design comparison
   (§9) could add warnings later, with its own spec.
