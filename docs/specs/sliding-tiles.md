# Spec: sliding tiles, a board longer than the machine

Status: **parked 2026-11-15** (the user meant puzzle pieces, docs/specs/pieces.md; this idea kept for later). Follows the TODO idea "a job larger than the bed
cut in tiles, the board slid between them, with the setup sheet saying how far; probably
built on setups" (docs/specs/setups.md, two-sided.md).

## 1. Why

A sign 2 m long, a bench top, a long shelf with fluting, on a machine that reaches 800 mm.
The board fits under the gantry but the tool can't reach all of it. Today that means
cutting the drawing apart by hand into projects, working out each zero on paper, and
hoping the pieces meet. That's where a 2 m sign gets a step in the middle.

With **tiling**, a setup is cut in **tiles** along one axis. Between tiles the operator
slides the board by an exact distance, the **slide**, and puts it back in place on two
pins. The machine's zero never changes, and Chipload splits the cuts between the tiles.

## 2. How it works, for the user

1. Draw the whole part as it is, full length, in one drawing.
2. In the setup's form, turn on **Tiling**: *along X* or *along Y*. Chipload proposes the
   slide from the machine's travel, and shows the tile boundaries on the board in the view:
   "3 tiles, slide 700 mm".
3. Click **Add tile pins**. This draws pin holes in the board's margins, one pair for each
   tile, and adds a holes operation that drills them through the board into the
   spoilboard.
4. Generate and export: one program for each tile (`sign-tile1.nc`, `sign-tile2.nc`, …), each
   with its setup sheet steps.
5. Run tile 1. It cuts its share and drills the first two pairs of pin holes.
6. For each further tile, do what the sheet says: lift the board off the pins, slide it
   700 mm to the left, drop the pins through the next pair of holes into the spoilboard,
   clamp it, and run the tile. **Don't touch the zero.**

## 3. The pins (decision 2)

The registration is the same idea as the two-sided dowels (two-sided.md §6), only it slides
instead of turning.

- The pins sit in the board's two **margins along the slide axis**: one pin in each margin
  strip, at the same position along the axis. A pair at the same position fixes the board
  along the axis and stops it turning.
- The holes go at positions **A + j·d** along the axis, where *d* is the slide and *j*
  runs from 0 to N−1 (N tiles). They're drawn circles on a layer "tile pins", drilled by one
  holes operation marked `dowels: true`, first in the setup's first part (after a surface
  pass, as the dowels are).
- **Tile 1** drills pairs 0 and 1. Pair 0 goes into the spoilboard at machine position A,
  and pair 1 lies one slide further, at A + d, still within reach. Its spoilboard hole
  isn't needed.
- Before **tile k** (k ≥ 2), the board slides by *d* toward the start of the axis. Pair
  k−1, drilled by the tile before, then lies over the spoilboard holes at A, and the pins go
  through it. Tile k drills pair k at machine position A + d, back into the spoilboard hole
  tile 1 made.
- No tile drills where a pin stands. This follows from the rule in §5: each hole is cut by
  the first tile that reaches it.
- **Pin diameter and depth:** the dowels' defaults, 8 mm, and 8 mm into the spoilboard, with
  a drill of that size or an end mill bore.
- **Where along the axis:** pair 0 sits a pin radius plus 1 mm in from the board's start,
  and the slide is chosen so that pair 1 is still in tile 1's reach (§4).
- **Room across the axis:** each pin is centred in its margin. If a margin is narrower than
  the pin plus 2 mm, the holes are made anyway, with a warning naming the room there is and
  the room needed (the dowels' rule since 2026-11-12). On a board fitted round the drawing,
  the fit counts the pin circles, so the board grows round them.
- **The checker knows the pins:**
  - The pin depth is allowed inside the pin circles, as it is inside dowel circles.
  - In tile k ≥ 2, the pins stand at pair k−1. A cut below the stock's top whose tool comes
    within one is refused. The checker works out which pair that is from the project (the
    tile pin circles, the slide and the tile number), never from the toolpath.

## 4. Tiles, slide and reach (decision 3)

- **Axis:** X or Y, one axis only. The board slides toward the axis's start: to the left
  along X, toward the front along Y.
- **Slide d:** a length. Proposed from the machine's travel along the axis:
  d = travel − 2 × (pin diameter + 2 mm), rounded down to 10 mm. It can be typed shorter.
  Without a travel in the machine profile, the slide must be typed.
- **Windows:** tile k's window is [s + (k−1)·d, s + k·d) along the axis, where *s* is the
  board's start.
- **Reach:** each tile reaches a little beyond its window, by *e* on each side, where
  e = (travel − d) / 2 (0 without a travel). Each program moves no more than the travel
  along the axis, which the checker already checks.
- **Number of tiles:** the fewest that reach every cut. The count is shown ("3 tiles"), and
  is never stored.
- **The zero:** the setup's work zero is the zero of tile 1. Tile k's numbers are the
  drawing's, shifted back by (k−1)·d along the axis, so the same machine zero is right for
  every tile. The sheet says to set the zero once, before tile 1, and to leave it.
- **Z:** the same board, so the same Z zero. A surface pass runs in each tile, over that
  tile's share.

## 5. Splitting the cuts (decision 4)

The setup is planned once, in drawing coordinates, as it is today. The plan is then split
into tiles by a step after planning (`chipload-toolpath`, `tiling.rs`). A **piece** is a
run of moves from leaving the safe height to coming back to it: one cut chain, one drill
cycle, one helical bore.

1. **A piece goes whole to the first tile that reaches all of it.** Small things stay
   whole: holes, bores, a letter, a flute. They aren't cut in two because they lie across a
   boundary. The second pair of pin holes goes to tile 1, as §3 needs.
2. **A piece no tile reaches whole is split at the window boundaries.** Each share goes to
   its window's tile. Lines are cut where they cross a boundary, and arcs too (helical arcs
   with their Z in proportion), so arcs stay arcs.
3. **At a split** the share ends with a retract to the safe height, and the next share
   starts with a rapid to above the board, then a plunge at the plunge feed down to where
   the cut continues.
   - In a profile cut in laps, or a pocket cut in levels, the earlier lap at that spot was
     cut in the same tile. So the plunge enters at most one step down of fresh material.
   - In a V-carve, a chamfer or any 3D move, it can be deeper. The operation then gets a
     note: "split across tiles 1 and 2: the cutter plunges into the cut at the boundary".
4. **The order inside a tile** is the plan's order: parts, operations, tool groups and
   copies as today. Each tile is cut whole, all its tools, before the board slides.
5. **Both tiles cut up to the boundary with the tool's centre.** That overlaps by the
   tool's width, so nothing is left standing between tiles if the pins put the board back
   true.

The split is verified like any other output: each tile's program goes through the checker
on its own, with the setup's stock shifted as the board is, the pins standing where they
stand, and the travel.

## 6. Files, report and setup sheet

- **Files:** `out-tile1.nc`, `out-tile2.nc`, … With several tools in files:
  `out-tile1-1-T1.nc`. With several setups: `out-S2-tile1.nc`. With one tile (tiling on,
  but everything reaches), the names stay as without tiling.
- `run --tile K` writes one tile. `plan`, `simulate` and `sheet` take `--tile K` too.
  Without it, `plan` gives the time per tile and the sum, and `simulate` the whole board
  after every tile.
- **The setup sheet:** one sheet for the setup, with a section per tile:
  - **Tile 1:** "Clamp the board with its start at …; set the zero; run `out-tile1.nc`.
    It drills the first pin holes."
  - **Tile k:** "Unclamp. Slide the board 700.0 mm to the left (−X). Put the 8 mm pins
    through the second pair of holes into the spoilboard. Clamp. Don't touch the zero.
    Run `out-tile2.nc`."
  - The board overhangs the machine as it slides: "support the board's free end level with
    the spoilboard".
  - A picture of the board with the windows and the pairs numbered.
- **The report:** the time per tile and in all; a note for each split operation.

## 7. The window

- **The setup's form:** a **Tiling** section, folded until it's turned on:
  - *Tiling: off / along X / along Y*;
  - *Slide*, a length, with the proposal shown greyed when it isn't typed;
  - the result: "3 tiles, each reaching 760 mm";
  - **Add tile pins** (or **Update tile pins** after the slide or the board changes, which
    redraws the circles; its operation is kept).
- **The view:** the window boundaries drawn as dashed lines across the board, numbered
  "1 | 2 | 3", and each tile's reach drawn faintly. The toolpath is the whole plan.
- **Export** writes every tile of the active setup, each with its sheet section.
- **Simulate:** the whole board, all tiles in order, as it will be in the end.

## 8. Command line and file

- `chipload setup tile part.chipload [--setup N] --axis x [--slide 700]`, and
  `--off`.
- `chipload setup tile-pins part.chipload [--setup N] [--diameter 8] [--depth 8]` draws
  the pin circles and adds or updates their operation.
- `run|plan|sheet|simulate --tile K`.
- **The file:** a setup gets `tiling`:

  ```json
  "tiling": { "axis": "x", "slide": 700.0 }
  ```

  It's optional (null: no tiling). `slide` is null when it's proposed from the travel,
  and is then worked out each time. **Schema 37**, a bump (decision 5). The migration from
  36 is a no-op that sets `tiling: null`. It comes with a frozen v36 sample, and a tiled
  golden (a 1200 mm board at slide 500: an outline split across three tiles, holes kept
  whole, the pins).
- **Validation:**
  - the slide must be greater than the pin diameter plus 2 mm;
  - without a travel, it must be given;
  - tiling on a setup that is a back, or has a back, is refused for now (§9);
  - the pin circles must be on the slide's pitch (within the chaining tolerance), and in
    pairs at the same position along the axis.

## 9. Not in this step

- **Tiling in both axes** (a grid of tiles). That's two slides with pins in both directions.
  Rare on a router whose gantry already spans the board's width.
- **Tiling with two-sided work.** The back's pins and the tiles' pins would have to agree.
- **Registration without pins** (a fence and pencil marks, or re-zeroing on a cut mark). It's
  less true, and easy to get wrong in the middle of a long job. Pins first, a fallback only
  if it's asked for.
- **Ramping into a split** instead of plunging (§5.3). A later improvement, if the plunge
  marks show.
- **Choosing boundaries by hand,** to move a split away from a feature.

## 10. Decisions

1. **Tiling is a setting of a setup,** not N setups made by hand or by a wizard. It's one
   board, one set of parts and operations, and the tiles split only the programs.
   **Recommended:** nothing to keep in step between copies; turning it off undoes it.
   Alternative: a tile is a setup, with the parts copied into each; that's flexible, but it's
   N times the operations to edit.
2. **Registration by pins in the margins, on the slide's pitch,** drilled as the tiles go
   (§3). The zero is set once. **Recommended:** the same tools and checker rules as the
   two-sided dowels, and no measuring between tiles. Alternative: re-zero at each tile on
   a mark the tile before cut. That needs no pins, but each re-zero is a chance to be off.
3. **One axis; slide and reach from the machine's travel** (§4), with the slide typed when
   there's no travel. **Recommended.**
4. **Pieces stay whole where one tile reaches them, and are split at the boundaries only
   when none does** (§5). A split ends with a retract and starts with a plunge, with a note
   on 3D moves. **Recommended:** fewest splits, and the pin holes fall into place.
   Alternative: always split at the boundaries; simpler, but holes and letters get cut in
   two.
5. **Schema 37 for `tiling`, a bump,** though the field is optional. **Recommended:** an
   older Chipload that read past it would cut the whole board in one program, which the
   machine can't reach. With the bump it refuses the file clearly. Alternative: no bump,
   relying on the setup's strict fields to make an older version refuse it with a less
   clear message.
6. **Order of work** (each step a commit):
   1. the model, schema 37 with its migration, validation, `setup tile`;
   2. splitting (`tiling.rs`) with property tests: the shares joined give back the plan, and
      every share lies within its tile's reach;
   3. running per tile: files, `--tile`, the checker's shifted stock and standing pins,
      the report;
   4. tile pins: `setup tile-pins`, the checker's depth allowance;
   5. the setup sheet, the simulation, the tiled golden;
   6. the window: the Tiling section, the boundaries in the view, export;
   7. the guide page (*Setups ▸ Tiling*), schemas, translations.
   **Recommended.**
