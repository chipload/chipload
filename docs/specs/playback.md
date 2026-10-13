# Spec: toolpath playback slider

Status: **approved 2026-10-13**, all decisions as recommended (§9).
Built 2026-10-13. As built:
- §4, measured (release, a 600 × 400 mm sheet at 0.25 mm cells, about 10 000 moves): the
  simulation with its history takes 0.6 s; the history is 670 000 entries (5 MB, against
  the 40 MB estimate); heights plus shading per frame take 5.7 ms at the 600 × 400 display
  grid. The ignored test `playback_frame_timing` prints these.
- §2: the slider's total is the moves' time, without the spindle's start-up dwell the
  cycle time estimate adds (so 1 min 33 s against the estimate's 1 min 36 s).
- §2: moves that take no time (the first rapids, before every axis is known) count as
  done at time 0.
- §1: the slider is a strip under the canvas, from the first generation on, with the play
  button, the speed (×10 at start), the marks (operation numbers where the operation
  changes; "T2" at tool changes) and the readout under it. Keys work whenever no text
  field has the keyboard.
- §3: the playback's stock uses its own texture; at the end the full-resolution layer is
  shown again unchanged.
- §5: `chipload simulate --after N` cuts only the first N moves (N up to the number of
  moves; more is an error).

User request (2026-10-13): a slider like PrusaSlicer's, to drag the program's progress back
and forth and see the toolpath, and the stock as cut so far, at any point. It builds on the
simulation (docs/specs/simulation.md, which left playback for its own spec) and replaces the
"toolpath playback" item in the TODO.

Display only: nothing here changes a toolpath, the G-code or the project file.

## 1. What the user sees

- A **slider** across the bottom of the canvas, shown once a toolpath has been generated.
  It runs from the program's start to its end. It starts at the end, where the canvas
  looks exactly as today.
- On the slider, **marks** where each operation starts (its number) and where the tool
  changes ("T2").
- Dragging it, the canvas shows:
  - the moves done so far as today; the moves still to come faint (or hidden; decision 4);
  - the **tool** at the current point: a circle of its diameter in top view, a cylinder
    outline (V-bit: cone) in 3D;
  - with the simulation on, the **stock as cut up to that point**, in top view and 3D.
- Next to the slider: the elapsed time and the total ("12 min 30 s of 41 min"), the
  operation ("Operation 3 (Outline), copy 2"), and the tool's X, Y, Z and the feed
  ("X 120.400 Y 35.000 Z −6.000, F 1800, cutting").
- **Keys** while the canvas has focus: ← / → one move back or on, Shift+← / → one
  operation, Home / End the start / end. A ▶ button plays at a chosen speed (×1, ×10, ×100
  the estimated time) and stops at the end or when the slider is touched.

## 2. Position: by time (decision 1)

The slider's position is **time**, from the cycle time estimate's per-move times
(docs/specs/cycle-time.md): a long cut takes more of the slider than a short rapid, as the
machine would spend. Within a move the tool's point is interpolated, so dragging is smooth
even over one long move. The simulation's stock follows **whole moves**: a move counts as
cut once the slider is past its end (decision 3).

## 3. The stock at any point: cell histories (decision 2)

Cutting only ever lowers a cell, so going back can't be done by cutting. Instead, while the
simulation runs (docs/specs/simulation.md §2) it records, for each cell of the **display
grid** (the coarse grid the 3D surface uses, at most 250 000 cells), every time the cell is
lowered: the move's index and the new height. A cell's entries are in move order and its
heights only go down.

- The height of a cell after move `k`: its last entry at or before `k`, or the stock top if
  none. A binary search per cell; both directions cost the same.
- The display grid's cell takes the lowest of the full grid's cells it covers, as now. The
  history is recorded on the display grid directly (a full cell lowering a display cell to
  a new low is an entry), so it isn't a second simulation.
- **Size:** a cell is lowered about once per depth pass that reaches it: typically under 20
  entries. At 8 bytes an entry (index `u32`, height `f32`), 250 000 cells × 20 is 40 MB.
  The history is capped at 64 million entries (512 MB is too much for a display aid): past
  that, playback shows the toolpath without the stock, with a note.
- **At the end** of the slider the full-resolution image (simulation §4) is shown as today;
  anywhere else the display grid, which is coarser on a large sheet. Dragging back to the
  end brings the full image back.

## 4. Drawing fast enough

Per frame while dragging (each cost an estimate for 250 000 cells, release build):

- heights at the slider's move: 250 000 binary searches, about 2–5 ms;
- the shading (simulation §4) of the display grid, about 2–4 ms;
- the texture's upload: 1 MB, under 1 ms; the 3D surface is rebuilt every frame already;
- only when the slider's **move** changes; between moves only the tool marker moves.

The first build checks these numbers with a timing test on the largest corpus project (and
a 600 × 400 mm sheet). If they are over 16 ms, the display grid while dragging becomes
smaller (decision 5).

## 5. Where it lives

- `chipload_pipeline::simulate` gains the history: `simulate_with_history(…)` returns the
  `Simulation` and a `History { grid (origin, cell, columns, rows), starts: Vec<u32>,
  entries: Vec<(u32, f32)> }` (per cell its run of entries, as in compressed sparse rows),
  with `heights_after(k) -> Vec<f32>` and `Simulation`-like shading for a grid of heights.
- `chipload_pipeline::estimate` exposes the per-move times (already computed inside).
- The GUI holds the history with the simulation and the slider's state (position, playing);
  no geometry in the GUI.
- The CLI: `chipload simulate --after 1234` (a move index) writes the stock after that move,
  for tests and for scripts. (Time on the CLI is not needed.)

## 6. G-code line (decision 6)

The slider shows the move and operation, not the G-code line. Arcs are merged when the post
writes them (one G2 for many moves) and every post's template decides its own header lines,
so a line number needs a mapping from the post; it can come later with a G-code panel that
highlights the current line, as PrusaSlicer does.

## 7. Edge cases

- **No toolpath, or the toolpath is out of date:** no slider (out of date: it stays, faded,
  until the next generation, like the toolpath).
- **Simulation off:** the slider still works for the toolpath and the tool marker; the stock
  isn't shown.
- **A new generation** while the slider is not at the end: the slider keeps its time if the
  new program is at least as long, else goes to the end.
- **Several tool groups:** the tool marker changes with the tool; a mark at each change.
- **Copies:** the operation label says which copy.

## 8. Tests

- **History (pipeline):** for random programs and every `k` in a sample of positions,
  `heights_after(k)` equals a fresh simulation of moves `0..=k` on the display grid, cell for
  cell. At the last move it equals the display grid of the full simulation. Entries per
  cell are in move order and their heights go down. The cap gives the note.
- **Time ↔ move:** the slider's time maps to the move whose span contains it; times are
  cumulative from the estimate; a zero-length move (a feed of no length) never traps the
  slider.
- **CLI:** `simulate --after k` writes the same numbers as the history's heights after `k`.
- **GUI:** keys move by a move, by an operation, to the start and end; the marks are at the
  operations' start times; the slider at the end shows exactly today's canvas; play stops
  at the end.
- **Timing:** the per-frame work (§4) on a 600 × 400 sheet in release, printed (not
  asserted, machines differ) by an ignored test run before release.
- Screenshot checks of the top view and 3D at a middle position.

## 9. Decisions

1. **The slider runs by time**, from the estimate's per-move times. **Recommended**: like
   the machine, long cuts take more of it. Alternative: by move index (each move the same
   width, so a long cut is a hair and a helix is wide).
2. **Per-cell histories on the display grid**, built with the simulation, to show the
   stock at any point in both directions. **Recommended**: one pass, then every position
   costs the same. Alternatives: snapshots every so many moves (much more memory at the
   full grid), or re-simulating from the start when going back (slow on large programs).
3. **The stock follows whole moves; the tool marker is interpolated.** **Recommended**:
   cutting part of a move per frame would need the cut from that move again.
4. **Moves still to come drawn faint** (not hidden), so the whole program stays readable.
   **Recommended**. A "hide the rest" choice in the View menu if wanted later.
5. **Display grid while dragging: the 3D surface's (≤ 250 000 cells), full image at the
   end.** **Recommended**; made smaller only if the timing test says so.
6. **No G-code line number yet.** **Recommended**: it needs a move-to-line mapping from
   each post, best done with a G-code panel later.
7. **Play button with ×1, ×10, ×100 speeds.** **Recommended**: a CNC run is long, and
   watching it go by helps check the order of operations. Alternative: dragging only.
